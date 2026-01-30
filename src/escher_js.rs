use anyhow::{anyhow, Result};
use std::collections::{HashMap, HashSet};

use crate::escher_bridge::{EscherSpec, EscherSpecMeta};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Term {
    Var(String),
    Component { name: String, args: Vec<Term> },
    If {
        cond: Box<Term>,
        then_branch: Box<Term>,
        else_branch: Box<Term>,
    },
}

impl Term {
    fn show(&self) -> String {
        match self {
            Term::Var(name) => format!("@{}", name),
            Term::Component { name, args } => {
                let args_text = args.iter().map(|t| t.show()).collect::<Vec<_>>().join(", ");
                format!("{}({})", name, args_text)
            }
            Term::If {
                cond,
                then_branch,
                else_branch,
            } => format!(
                "if {} then {} else {}",
                cond.show(),
                then_branch.show(),
                else_branch.show()
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompileContext {
    pub gamma: HashMap<String, String>,
    pub sigma: HashMap<String, String>,
    pub sigma_ptr_vars: HashSet<String>,
    pub function_name: String,
    pub receiver_arg_index: Option<usize>,
}

pub fn parse_rendered_term(rendered: &str) -> Result<Term> {
    let body = match rendered.split_once('=') {
        Some((_, rhs)) => rhs,
        None => rendered,
    };
    let trimmed = body.trim();
    let unwrapped = strip_wrapping_braces(trimmed);
    parse_term_text(unwrapped.trim())
}

fn strip_wrapping_braces(input: &str) -> String {
    let mut current = input.trim();
    loop {
        let bytes = current.as_bytes();
        if bytes.len() >= 2 && bytes[0] == b'{' && bytes[bytes.len() - 1] == b'}' {
            current = current[1..bytes.len() - 1].trim();
            continue;
        }
        break;
    }
    current.to_string()
}

pub fn translate_rendered_method(
    rendered: &str,
    params_js: &[String],
    ctx: &CompileContext,
) -> Result<String> {
    let term = parse_rendered_term(rendered)?;
    compile_method(params_js, &term, ctx)
}

pub fn build_context_from_spec(
    function_name: &str,
    spec: &EscherSpec,
    meta: &EscherSpecMeta,
) -> Result<(CompileContext, Vec<String>)> {
    let list_field_count = meta.value_fields.len() + meta.pointer_fields.len();
    if meta.arg_count + list_field_count != spec.input_types.len() {
        return Err(anyhow!(
            "spec/meta mismatch for {}: args {} + fields {} != input_types {}",
            function_name,
            meta.arg_count,
            list_field_count,
            spec.input_types.len()
        ));
    }

    let mut used = HashSet::new();
    let mut gamma = HashMap::new();
    let mut params_js = Vec::new();
    for idx in 0..meta.arg_count {
        let raw = meta.arg_names.get(idx).map(String::as_str).unwrap_or("");
        let mut base = if raw.is_empty() {
            format!("arg{}", idx)
        } else {
            let cleaned = sanitize_ident(raw);
            if cleaned.chars().all(|c| c == '_') {
                format!("arg{}", idx)
            } else {
                cleaned
            }
        };
        if base == "this" {
            base = format!("arg{}_this", idx);
        }
        let name = unique_ident(&base, &mut used);
        let var_name = format!("x{}", idx);
        if meta.receiver_arg_index == Some(idx) {
            gamma.insert(var_name, "this".to_string());
        } else {
            gamma.insert(var_name, name.clone());
            params_js.push(name);
        }
    }

    let mut sigma = HashMap::new();
    let mut sigma_ptr_vars = HashSet::new();
    let value_start = meta.arg_count;
    for (idx, field) in meta.value_fields.iter().enumerate() {
        sigma.insert(format!("x{}", value_start + idx), field.clone());
    }
    let ptr_start = value_start + meta.value_fields.len();
    for (idx, field) in meta.pointer_fields.iter().enumerate() {
        let var_name = format!("x{}", ptr_start + idx);
        sigma.insert(var_name.clone(), field.clone());
        sigma_ptr_vars.insert(var_name);
    }

    Ok((
        CompileContext {
            gamma,
            sigma,
            sigma_ptr_vars,
            function_name: function_name.to_string(),
            receiver_arg_index: meta.receiver_arg_index,
        },
        params_js,
    ))
}

pub fn compile_method(
    params_js: &[String],
    body: &Term,
    ctx: &CompileContext,
) -> Result<String> {
    let js_name = js_function_name(&ctx.function_name);
    let params = params_js.join(", ");
    let compiled = compile_statement(body, ctx, true)?;
    Ok(format!("{}({}) {{ {} }}", js_name, params, compiled))
}

pub fn compile_term(term: &Term, ctx: &CompileContext) -> Result<String> {
    match term {
        Term::Var(name) => ctx
            .gamma
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow!("unknown variable '{}'", name)),
        Term::If {
            cond,
            then_branch,
            else_branch,
        } => {
            let cond_js = compile_term(cond, ctx)?;
            let then_js = compile_term(then_branch, ctx)?;
            let else_js = compile_term(else_branch, ctx)?;
            Ok(format!(
                "(({}) ? ({}) : ({}))",
                cond_js, then_js, else_js
            ))
        }
        Term::Component { name, args } => {
            if name == &ctx.function_name {
                return compile_recursive_call(name, args, ctx);
            }
            compile_component(name, args, ctx)
        }
    }
}

fn compile_statement(term: &Term, ctx: &CompileContext, emit_return: bool) -> Result<String> {
    match term {
        Term::If {
            cond,
            then_branch,
            else_branch,
        } => {
            let cond_js = compile_term(cond, ctx)?;
            let then_js = compile_statement(then_branch, ctx, emit_return)?;
            let else_js = compile_statement(else_branch, ctx, emit_return)?;
            Ok(format!(
                "if ({}) {{ {} }} else {{ {} }}",
                cond_js, then_js, else_js
            ))
        }
        _ => {
            let expr = compile_term(term, ctx)?;
            if emit_return {
                Ok(format!("return {};", expr))
            } else {
                Ok(format!("{};", expr))
            }
        }
    }
}

fn compile_component(name: &str, args: &[Term], ctx: &CompileContext) -> Result<String> {
    match name {
        "and" => compile_binary_expr(name, args, ctx, "_a && _b"),
        "or" => compile_binary_expr(name, args, ctx, "_a || _b"),
        "not" => compile_unary_expr(name, args, ctx, "!_a"),
        "equal" => compile_binary_expr(name, args, ctx, "_a === _b"),
        "eq_ptr" => compile_binary_expr(name, args, ctx, "_a === _b"),
        "dec" => compile_unary_expr(name, args, ctx, "_a - 1"),
        "inc" => compile_unary_expr(name, args, ctx, "_a + 1"),
        "isNonNeg" => compile_unary_expr(name, args, ctx, "_a >= 0"),
        "zero" => compile_nullary(name, args, "0"),
        "T" => compile_nullary(name, args, "true"),
        "F" => compile_nullary(name, args, "false"),
        "null_ptr" => compile_nullary(name, args, "null"),
        "is_null" => compile_unary_expr(name, args, ctx, "_a === null"),
        "index_ptr" => compile_index_ptr(args, ctx),
        "is_null_at_ptr" => compile_is_null_at_ptr(args, ctx),
        _ => Err(anyhow!(
            "unknown component '{}' in term {}",
            name,
            Term::Component {
                name: name.to_string(),
                args: args.to_vec(),
            }
            .show()
        )),
    }
}

fn compile_recursive_call(name: &str, args: &[Term], ctx: &CompileContext) -> Result<String> {
    let mut compiled_args = Vec::new();
    let mut receiver_expr: Option<String> = None;
    for (idx, term) in args.iter().enumerate() {
        if is_sigma_var(term, ctx) {
            continue;
        }
        let compiled = compile_term(term, ctx)?;
        if ctx.receiver_arg_index == Some(idx) {
            receiver_expr = Some(compiled);
        } else {
            compiled_args.push(compiled);
        }
    }
    let js_name = js_function_name(name);
    if let Some(receiver) = receiver_expr {
        let callee = format_field_access(&receiver, &js_name);
        Ok(format!("{}({})", callee, compiled_args.join(", ")))
    } else {
        Ok(format!("{}({})", js_name, compiled_args.join(", ")))
    }
}

fn compile_index_ptr(args: &[Term], ctx: &CompileContext) -> Result<String> {
    ensure_arity("index_ptr", args, 2)?;
    if sigma_field(&args[0], ctx).is_some() {
        return Err(anyhow!(
            "index_ptr expects Ptr first, List second (sigma should be on second arg)"
        ));
    }
    if let Some(field) = sigma_field(&args[1], ctx) {
        let ptr_expr = compile_term(&args[0], ctx)?;
        return Ok(format_field_access(&ptr_expr, field));
    }
    Err(anyhow!(
        "index_ptr requires list field (sigma) on second arg"
    ))
}

fn compile_is_null_at_ptr(args: &[Term], ctx: &CompileContext) -> Result<String> {
    ensure_arity("is_null_at_ptr", args, 2)?;
    if sigma_field(&args[0], ctx).is_some() {
        return Err(anyhow!(
            "is_null_at_ptr expects Ptr first, List second (sigma should be on second arg)"
        ));
    }
    if let Some(field) = sigma_field(&args[1], ctx) {
        if !is_sigma_ptr_var(&args[1], ctx) {
            return Err(anyhow!(
                "is_null_at_ptr requires Ptr field for {:?}",
                args[1]
            ));
        }
        let ptr_expr = compile_term(&args[0], ctx)?;
        return Ok(format!("({} === null)", format_field_access(&ptr_expr, field)));
    }
    Err(anyhow!(
        "is_null_at_ptr requires list field (sigma) on second arg"
    ))
}

fn is_sigma_var(term: &Term, ctx: &CompileContext) -> bool {
    matches!(term, Term::Var(name) if ctx.sigma.contains_key(name))
}

fn is_sigma_ptr_var(term: &Term, ctx: &CompileContext) -> bool {
    matches!(term, Term::Var(name) if ctx.sigma_ptr_vars.contains(name))
}

fn sigma_field<'a>(term: &Term, ctx: &'a CompileContext) -> Option<&'a str> {
    match term {
        Term::Var(name) => ctx.sigma.get(name).map(String::as_str),
        _ => None,
    }
}

fn compile_unary_expr(
    name: &str,
    args: &[Term],
    ctx: &CompileContext,
    expr: &str,
) -> Result<String> {
    ensure_arity(name, args, 1)?;
    let value = compile_term(&args[0], ctx)?;
    let replaced = expr.replace("_a", &format!("({})", value));
    Ok(format!("({})", replaced))
}

fn compile_binary_expr(
    name: &str,
    args: &[Term],
    ctx: &CompileContext,
    expr: &str,
) -> Result<String> {
    ensure_arity(name, args, 2)?;
    let lhs = compile_term(&args[0], ctx)?;
    let rhs = compile_term(&args[1], ctx)?;
    let replaced = expr
        .replace("_a", &format!("({})", lhs))
        .replace("_b", &format!("({})", rhs));
    Ok(format!("({})", replaced))
}

fn compile_nullary(name: &str, args: &[Term], replacement: &str) -> Result<String> {
    ensure_arity(name, args, 0)?;
    Ok(replacement.to_string())
}

fn ensure_arity(name: &str, args: &[Term], expected: usize) -> Result<()> {
    if args.len() != expected {
        Err(anyhow!(
            "component '{}' expects {} args, got {}",
            name,
            expected,
            args.len()
        ))
    } else {
        Ok(())
    }
}

fn unique_ident(base: &str, used: &mut HashSet<String>) -> String {
    if used.insert(base.to_string()) {
        return base.to_string();
    }
    let mut suffix = 1;
    loop {
        let candidate = format!("{}_{}", base, suffix);
        if used.insert(candidate.clone()) {
            return candidate;
        }
        suffix += 1;
    }
}

fn sanitize_ident(raw: &str) -> String {
    let mut out = String::new();
    for (idx, ch) in raw.chars().enumerate() {
        let valid = if idx == 0 {
            is_js_ident_start(ch)
        } else {
            is_js_ident_continue(ch)
        };
        if valid {
            out.push(ch);
        } else if idx == 0 && ch.is_ascii_alphanumeric() {
            out.push('_');
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push('_');
    }
    out
}

fn js_function_name(raw: &str) -> String {
    let cleaned = sanitize_ident(raw);
    if cleaned.chars().all(|c| c == '_') {
        "fn".to_string()
    } else {
        cleaned
    }
}

fn format_field_access(base_expr: &str, field: &str) -> String {
    let base = format!("({})", base_expr);
    if is_valid_js_ident(field) {
        format!("{}.{}", base, field)
    } else {
        format!("{}[{}]", base, js_string_literal(field))
    }
}

fn is_valid_js_ident(raw: &str) -> bool {
    let mut chars = raw.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !is_js_ident_start(first) {
        return false;
    }
    chars.all(is_js_ident_continue)
}

fn js_string_literal(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 2);
    out.push('"');
    for ch in raw.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn is_js_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_' || ch == '$'
}

fn is_js_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Ident(String),
    If,
    Then,
    Else,
    At,
    LParen,
    RParen,
    Comma,
}

fn parse_term_text(input: &str) -> Result<Term> {
    let tokens = tokenize(input)?;
    let mut parser = Parser::new(tokens);
    let term = parser.parse_term()?;
    if parser.has_remaining() {
        return Err(anyhow!("unexpected trailing tokens"));
    }
    Ok(term)
}

fn tokenize(input: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.peek().copied() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }
        match ch {
            '@' => {
                chars.next();
                tokens.push(Token::At);
            }
            '(' => {
                chars.next();
                tokens.push(Token::LParen);
            }
            ')' => {
                chars.next();
                tokens.push(Token::RParen);
            }
            ',' => {
                chars.next();
                tokens.push(Token::Comma);
            }
            _ if is_ident_start(ch) => {
                let mut ident = String::new();
                while let Some(c) = chars.peek().copied() {
                    if is_ident_continue(c) {
                        ident.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                match ident.as_str() {
                    "if" => tokens.push(Token::If),
                    "then" => tokens.push(Token::Then),
                    "else" => tokens.push(Token::Else),
                    _ => tokens.push(Token::Ident(ident)),
                }
            }
            _ => {
                return Err(anyhow!("unexpected character '{}'", ch));
            }
        }
    }
    Ok(tokens)
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn has_remaining(&self) -> bool {
        self.pos < self.tokens.len()
    }

    fn parse_term(&mut self) -> Result<Term> {
        if self.consume_token(&Token::If) {
            let cond = self.parse_term()?;
            self.expect_token(&Token::Then)?;
            let then_branch = self.parse_term()?;
            self.expect_token(&Token::Else)?;
            let else_branch = self.parse_term()?;
            return Ok(Term::If {
                cond: Box::new(cond),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
            });
        }
        self.parse_comp_or_var()
    }

    fn parse_comp_or_var(&mut self) -> Result<Term> {
        if self.consume_token(&Token::At) {
            let name = self.expect_ident()?;
            return Ok(Term::Var(name));
        }
        let name = self.expect_ident()?;
        if self.consume_token(&Token::LParen) {
            let mut args = Vec::new();
            if !self.consume_token(&Token::RParen) {
                loop {
                    args.push(self.parse_term()?);
                    if self.consume_token(&Token::Comma) {
                        continue;
                    }
                    self.expect_token(&Token::RParen)?;
                    break;
                }
            }
            Ok(Term::Component { name, args })
        } else {
            Ok(Term::Var(name))
        }
    }

    fn consume_token(&mut self, expected: &Token) -> bool {
        if self.tokens.get(self.pos) == Some(expected) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect_token(&mut self, expected: &Token) -> Result<()> {
        if self.consume_token(expected) {
            Ok(())
        } else {
            Err(anyhow!("expected token {:?}", expected))
        }
    }

    fn expect_ident(&mut self) -> Result<String> {
        match self.tokens.get(self.pos) {
            Some(Token::Ident(name)) => {
                self.pos += 1;
                Ok(name.clone())
            }
            other => Err(anyhow!("expected identifier, got {:?}", other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::escher_bridge::{EscherSpec, EscherSpecMeta};
    use std::collections::{HashMap, HashSet};

    fn normalize_lines(input: &str) -> Vec<String> {
        input.lines().map(|line| line.trim_end().to_string()).collect()
    }

    fn assert_js_eq(actual: &str, expected: &str) {
        assert_eq!(normalize_lines(actual), normalize_lines(expected));
    }

    #[test]
    fn compile_recursive_call_without_methodization() {
        let term_src =
            "if is_null(@x1) then @x1 else lastPtr(index_ptr(@x1, @x2), @x2)";
        let term = parse_term_text(term_src).expect("parse term");
        let mut gamma = HashMap::new();
        gamma.insert("x0".to_string(), "arg0".to_string());
        gamma.insert("x1".to_string(), "arg1".to_string());
        let mut sigma = HashMap::new();
        sigma.insert("x2".to_string(), "next".to_string());
        let ctx = CompileContext {
            gamma,
            sigma,
            sigma_ptr_vars: HashSet::new(),
            function_name: "lastPtr".to_string(),
            receiver_arg_index: None,
        };
        let params_js = vec!["arg0".to_string(), "arg1".to_string()];
        let js = compile_method(&params_js, &term, &ctx).expect("compile");
        let expected = "lastPtr(arg0, arg1) { if (((arg1) === null)) { return arg1; } else { return lastPtr((arg1).next); } }";
        assert_js_eq(&js, expected);
    }

    #[test]
    fn compile_equal_uses_strict_equality() {
        let term_src = "equal(zero(), zero())";
        let term = parse_term_text(term_src).expect("parse term");
        let ctx = CompileContext {
            gamma: HashMap::new(),
            sigma: HashMap::new(),
            sigma_ptr_vars: HashSet::new(),
            function_name: "eq".to_string(),
            receiver_arg_index: None,
        };
        let js = compile_term(&term, &ctx).expect("compile");
        let expected = "((0) === (0))";
        assert_eq!(js, expected);
    }

    #[test]
    fn compile_index_ptr_checks_bounds() {
        let term_src = "index_ptr(@x0, @x1)";
        let term = parse_term_text(term_src).expect("parse term");
        let mut gamma = HashMap::new();
        gamma.insert("x0".to_string(), "ptr".to_string());
        gamma.insert("x1".to_string(), "ptrs_next".to_string());
        let ctx = CompileContext {
            gamma,
            sigma: HashMap::new(),
            sigma_ptr_vars: HashSet::new(),
            function_name: "idx".to_string(),
            receiver_arg_index: None,
        };
        let err = compile_term(&term, &ctx).expect_err("compile error");
        assert_eq!(
            err.to_string(),
            "index_ptr requires list field (sigma) on second arg"
        );
    }

    #[test]
    fn compile_index_ptr_uses_field_access_with_sigma() {
        let term_src = "index_ptr(@x0, @x1)";
        let term = parse_term_text(term_src).expect("parse term");
        let mut gamma = HashMap::new();
        gamma.insert("x0".to_string(), "this".to_string());
        let mut sigma = HashMap::new();
        sigma.insert("x1".to_string(), "next".to_string());
        let mut sigma_ptr_vars = HashSet::new();
        sigma_ptr_vars.insert("x1".to_string());
        let ctx = CompileContext {
            gamma,
            sigma,
            sigma_ptr_vars,
            function_name: "last_ptr".to_string(),
            receiver_arg_index: Some(0),
        };
        let js = compile_term(&term, &ctx).expect("compile");
        assert_eq!(js, "(this).next");
    }

    #[test]
    fn compile_recursive_call_with_receiver_and_sigma() {
        let term_src = "if is_null_at_ptr(@x0, @x1) then @x0 else last_ptr(index_ptr(@x0, @x1), @x1)";
        let term = parse_term_text(term_src).expect("parse term");
        let mut gamma = HashMap::new();
        gamma.insert("x0".to_string(), "this".to_string());
        let mut sigma = HashMap::new();
        sigma.insert("x1".to_string(), "next".to_string());
        let mut sigma_ptr_vars = HashSet::new();
        sigma_ptr_vars.insert("x1".to_string());
        let ctx = CompileContext {
            gamma,
            sigma,
            sigma_ptr_vars,
            function_name: "last_ptr".to_string(),
            receiver_arg_index: Some(0),
        };
        let js = compile_method(&[], &term, &ctx).expect("compile");
        let expected =
            "last_ptr() { if (((this).next === null)) { return this; } else { return ((this).next).last_ptr(); } }";
        assert_js_eq(&js, expected);
    }

    #[test]
    fn build_context_from_spec_maps_gamma_sigma() {
        let spec = EscherSpec {
            name: "last_ptr".to_string(),
            input_types: vec![
                "Ptr".to_string(),
                "Ptr".to_string(),
                "List[Int]".to_string(),
                "List[Ptr]".to_string(),
                "List[Ptr]".to_string(),
            ],
            return_type: "Ptr".to_string(),
            examples: Vec::new(),
        };
        let meta = EscherSpecMeta {
            arg_count: 2,
            arg_names: vec!["this".to_string(), "this".to_string()],
            value_fields: vec!["val".to_string()],
            pointer_fields: vec!["next".to_string(), "prev".to_string()],
            receiver_arg_index: Some(0),
        };

        let (ctx, params_js) =
            build_context_from_spec("last_ptr", &spec, &meta).expect("build context");

        assert_eq!(ctx.gamma.get("x0").map(String::as_str), Some("this"));
        assert_eq!(
            ctx.gamma.get("x1").map(String::as_str),
            Some("arg1_this")
        );
        assert_eq!(params_js, vec!["arg1_this".to_string()]);
        assert_eq!(ctx.sigma.get("x2").map(String::as_str), Some("val"));
        assert_eq!(ctx.sigma.get("x3").map(String::as_str), Some("next"));
        assert_eq!(ctx.sigma.get("x4").map(String::as_str), Some("prev"));
        assert!(ctx.sigma_ptr_vars.contains("x3"));
        assert!(ctx.sigma_ptr_vars.contains("x4"));
    }

    #[test]
    fn translate_rendered_method_with_direct_context() {
        let rendered = "last_ptr(x0, x1) = if is_null_at_ptr(@x0, @x1) then @x0 else last_ptr(index_ptr(@x0, @x1), @x1)";
        let mut gamma = HashMap::new();
        gamma.insert("x0".to_string(), "this".to_string());
        let mut sigma = HashMap::new();
        sigma.insert("x1".to_string(), "next".to_string());
        let mut sigma_ptr_vars = HashSet::new();
        sigma_ptr_vars.insert("x1".to_string());
        let ctx = CompileContext {
            gamma,
            sigma,
            sigma_ptr_vars,
            function_name: "last_ptr".to_string(),
            receiver_arg_index: Some(0),
        };

        let js = translate_rendered_method(rendered, &[], &ctx).expect("compile");

        let expected =
            "last_ptr() { if (((this).next === null)) { return this; } else { return ((this).next).last_ptr(); } }";
        assert_js_eq(&js, expected);
    }

    #[test]
    fn translate_find() {
        let rendered = "find(x0, x1, x2, x3) = if equal(x1, index_ptr(x0, x2)) then index_ptr(x0, x2) else if is_null_at_ptr(x0, x3) then null_ptr() else find(index_ptr(x0, x3), x1, x2, x3)";
        let mut gamma = HashMap::new();
        gamma.insert("x0".to_string(), "this".to_string());
        gamma.insert("x1".to_string(), "target".to_string());
        let mut sigma = HashMap::new();
        sigma.insert("x2".to_string(), "val".to_string());
        sigma.insert("x3".to_string(), "next".to_string());
        let mut sigma_ptr_vars = HashSet::new();
        sigma_ptr_vars.insert("x3".to_string());
        let ctx = CompileContext {
            gamma,
            sigma,
            sigma_ptr_vars,
            function_name: "find".to_string(),
            receiver_arg_index: Some(0),
        };
        let js = translate_rendered_method(rendered, &["target".to_string()], &ctx)
            .expect("compile");
        let expected = "find(target) { if (((target) === ((this).val))) { return (this).val; } else { if (((this).next === null)) { return null; } else { return ((this).next).find(target); } } }";
        assert_js_eq(&js, expected);
    }

}

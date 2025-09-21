//! AST 定義 + Display 実装

use std::collections::HashMap;
use std::fmt::{self, Display};

/// プレースホルダー ID （`PH0`, `PH1`, ...）
pub type Placeholder = String;

/* ---------- Expressions ---------- */
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(i64),
    Str(String),
    Var(String),
    New(String), // new <Class>()
    This,
    Lhs(Box<Lhs>),
    Hole(Placeholder),                       // ホール表現
    Literal(String),                         // 新しいバリアント
    MethodCall(Box<Lhs>, String, Vec<Expr>), // obj.method(args)
}

/* ---------- L-value ---------- */
#[derive(Debug, Clone, PartialEq)]
pub enum Lhs {
    Var(String),
    ObjAccess(Box<Lhs>, String), // obj.prop ...
    Hole(Placeholder),           // ホール表現
    This,                        // New variant for 'this' keyword
}

/* ---------- Statements ---------- */
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    VarDecl { name: String, expr: Expr },
    Assign { lhs: Lhs, expr: Expr },
    Expr(Expr),
    Hole(Placeholder), // Added
}

/* ---------- Program ---------- */
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub stmts: Vec<Stmt>,
}

/* ---------- Display impls (pretty-printer) ---------- */
impl Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Expr::*;
        match self {
            Num(n) => write!(f, "{n}"),
            Str(s) => write!(f, "{s:?}"), // Strings are quoted
            Var(v) => write!(f, "{v}"),
            New(cls) => write!(f, "new {cls}()"),
            This => write!(f, "this"),
            Lhs(lhs) => write!(f, "{lhs}"),
            Hole(ph) => write!(f, "{ph}"),
            Literal(lit) => write!(f, "{lit}"), // Literals (e.g. from JSON) might not need quotes here if they are already strings
            MethodCall(obj, method, args) => {
                let args_str = args
                    .iter()
                    .map(|a| format!("{}", a))
                    .collect::<Vec<String>>()
                    .join(", ");
                write!(f, "{}.{}({})", obj, method, args_str)
            }
        }
    }
}

impl Display for Lhs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Lhs::*;
        match self {
            Var(v) => write!(f, "{v}"),
            ObjAccess(obj, p) => write!(f, "{obj}.{p}"),
            Hole(ph) => write!(f, "{ph}"),
            This => write!(f, "this"),
        }
    }
}

impl Display for Stmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Stmt::*;
        match self {
            VarDecl { name, expr } => write!(f, "var {} = {};", name, expr),
            Assign { lhs, expr } => write!(f, "{} = {};", lhs, expr),
            Expr(expr) => write!(f, "{};", expr), // Assuming expressions as statements end with a semicolon
            Hole(ph) => write!(f, "{};", ph),     // Placeholder statement
        }
    }
}

impl Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for stmt in &self.stmts {
            writeln!(f, "{}", stmt)?;
        }
        Ok(())
    }
}

pub fn synchronize_and_hole_stmt(
    template_stmt: &mut Stmt,
    target_stmt: &Stmt,
    next_hole_id: &mut usize,
    hole_map: &mut HashMap<String, Vec<String>>,
) {
    use crate::{add_to_hole, update_hole_value};

    match (template_stmt.clone(), target_stmt) {
        (
            Stmt::VarDecl {
                name: name1,
                expr: mut expr1,
            },
            Stmt::VarDecl {
                name: name2,
                expr: expr2,
            },
        ) => {
            if name1 != *name2 {
                let stmt1_str = format!(
                    "{}",
                    Stmt::VarDecl {
                        name: name1.clone(),
                        expr: expr1.clone()
                    }
                );
                let stmt2_str = format!(
                    "{}",
                    Stmt::VarDecl {
                        name: name2.clone(),
                        expr: expr2.clone()
                    }
                );
                let ph_name = add_to_hole(
                    hole_map,
                    next_hole_id,
                    "var_decl_stmt",
                    stmt1_str,
                    stmt2_str,
                );
                *template_stmt = Stmt::Hole(ph_name);
                return;
            }
            synchronize_and_hole_expr(&mut expr1, expr2, next_hole_id, hole_map);
            *template_stmt = Stmt::VarDecl {
                name: name1,
                expr: expr1,
            };
        }
        (
            Stmt::Assign {
                lhs: mut lhs1,
                expr: mut expr1,
            },
            Stmt::Assign {
                lhs: lhs2,
                expr: expr2,
            },
        ) => {
            synchronize_and_hole_lhs(&mut lhs1, lhs2, next_hole_id, hole_map);
            synchronize_and_hole_expr(&mut expr1, expr2, next_hole_id, hole_map);
            *template_stmt = Stmt::Assign {
                lhs: lhs1,
                expr: expr1,
            };
        }
        (Stmt::Expr(mut e1), Stmt::Expr(e2)) => {
            synchronize_and_hole_expr(&mut e1, e2, next_hole_id, hole_map);
            *template_stmt = Stmt::Expr(e1);
        }
        (s1, s2) => {
            if let Stmt::Hole(ph_name) = &s1 {
                let mut found_key = None;
                for key_in_map in hole_map.keys() {
                    if key_in_map.starts_with(ph_name) && key_in_map.contains("(") {
                        found_key = Some(key_in_map.clone());
                        break;
                    }
                }
                if let Some(actual_hole_key) = found_key {
                    update_hole_value(hole_map, &actual_hole_key, format!("{}", s2));
                } else {
                    let s1_str = format!("{}", s1);
                    let s2_str = format!("{}", s2);
                    let new_ph_name =
                        add_to_hole(hole_map, next_hole_id, "stmt_mismatch", s1_str, s2_str);
                    *template_stmt = Stmt::Hole(new_ph_name);
                }
            } else if s1.to_string() != s2.to_string() {
                let s1_str = format!("{}", s1.clone());
                let s2_str = format!("{}", s2);
                let ph_name = add_to_hole(hole_map, next_hole_id, "stmt_value", s1_str, s2_str);
                *template_stmt = Stmt::Hole(ph_name);
            }
        }
    }
}

pub fn synchronize_and_hole_expr(
    template_expr: &mut Expr,
    target_expr: &Expr,
    next_hole_id: &mut usize,
    hole_map: &mut HashMap<String, Vec<String>>,
) {
    use crate::{add_to_hole, update_hole_value};

    match (template_expr.clone(), target_expr) {
        (Expr::Num(n1), Expr::Num(n2)) => {
            if n1 != *n2 {
                let ph_name = add_to_hole(
                    hole_map,
                    next_hole_id,
                    "num_value",
                    n1.to_string(),
                    n2.to_string(),
                );
                *template_expr = Expr::Hole(ph_name);
            }
        }
        (Expr::Str(s1), Expr::Str(s2)) => {
            if s1 != *s2 {
                let ph_name =
                    add_to_hole(hole_map, next_hole_id, "str_value", s1.clone(), s2.clone());
                *template_expr = Expr::Hole(ph_name);
            }
        }
        (Expr::Var(v1), Expr::Var(v2)) => {
            if v1 != *v2 {
                let ph_name =
                    add_to_hole(hole_map, next_hole_id, "var_name", v1.clone(), v2.clone());
                *template_expr = Expr::Hole(ph_name);
            }
        }
        (Expr::New(c1), Expr::New(c2)) => {
            if c1 != *c2 {
                let ph_name =
                    add_to_hole(hole_map, next_hole_id, "class_name", c1.clone(), c2.clone());
                *template_expr = Expr::Hole(ph_name);
            }
        }
        (Expr::Lhs(mut lhs1), Expr::Lhs(lhs2)) => {
            synchronize_and_hole_lhs(&mut lhs1, lhs2, next_hole_id, hole_map);
            *template_expr = Expr::Lhs(lhs1);
        }
        (Expr::MethodCall(obj1, method1, args1), Expr::MethodCall(obj2, method2, args2)) => {
            let mut mut_obj1 = obj1.clone();
            synchronize_and_hole_lhs(&mut mut_obj1, obj2, next_hole_id, hole_map);
            let new_obj1 = mut_obj1;

            if method1 != *method2 {
                let expr1_str = format!(
                    "{}",
                    Expr::MethodCall(obj1.clone(), method1.clone(), args1.clone())
                );
                let expr2_str = format!(
                    "{}",
                    Expr::MethodCall(obj2.clone(), method2.clone(), args2.clone())
                );
                let ph_name_call = add_to_hole(
                    hole_map,
                    next_hole_id,
                    "method_call_expr",
                    expr1_str,
                    expr2_str,
                );
                *template_expr = Expr::Hole(ph_name_call);
                return;
            }
            if args1.len() != args2.len() {
                let expr1_str = format!(
                    "{}",
                    Expr::MethodCall(obj1.clone(), method1.clone(), args1.clone())
                );
                let expr2_str = format!(
                    "{}",
                    Expr::MethodCall(obj2.clone(), method2.clone(), args2.clone())
                );
                let ph_name_call = add_to_hole(
                    hole_map,
                    next_hole_id,
                    "method_call_expr_args_len_diff",
                    expr1_str,
                    expr2_str,
                );
                *template_expr = Expr::Hole(ph_name_call);
                return;
            }

            let mut new_args1 = Vec::new();
            for (arg1, arg2) in args1.iter().zip(args2.iter()) {
                let mut mut_arg1 = arg1.clone();
                synchronize_and_hole_expr(&mut mut_arg1, arg2, next_hole_id, hole_map);
                new_args1.push(mut_arg1);
            }
            *template_expr = Expr::MethodCall(new_obj1, method1, new_args1);
        }
        (e1, e2) => {
            if let Expr::Hole(ph_name) = &e1 {
                let mut found_key = None;
                for key_in_map in hole_map.keys() {
                    if key_in_map.starts_with(ph_name) && key_in_map.contains("(") {
                        found_key = Some(key_in_map.clone());
                        break;
                    }
                }
                if let Some(actual_hole_key) = found_key {
                    update_hole_value(hole_map, &actual_hole_key, format!("{}", e2));
                } else {
                    let e1_str = format!("{}", e1);
                    let e2_str = format!("{}", e2);
                    let new_ph_name =
                        add_to_hole(hole_map, next_hole_id, "expr_mismatch", e1_str, e2_str);
                    *template_expr = Expr::Hole(new_ph_name);
                }
            } else if e1.to_string() != e2.to_string() {
                let e1_str = format!("{}", e1.clone());
                let e2_str = format!("{}", e2);
                let ph_name = add_to_hole(hole_map, next_hole_id, "expr_value", e1_str, e2_str);
                *template_expr = Expr::Hole(ph_name);
            }
        }
    }
}

pub fn synchronize_and_hole_lhs(
    template_lhs: &mut Lhs,
    target_lhs: &Lhs,
    next_hole_id: &mut usize,
    hole_map: &mut HashMap<String, Vec<String>>,
) {
    use crate::{add_to_hole, update_hole_value};

    match (template_lhs.clone(), target_lhs) {
        (Lhs::Var(v1), Lhs::Var(v2)) => {
            if v1 != *v2 {
                let ph_name = add_to_hole(hole_map, next_hole_id, "lhs_var_name", v1, v2.clone());
                *template_lhs = Lhs::Hole(ph_name);
            }
        }
        (Lhs::ObjAccess(obj1, prop1), Lhs::ObjAccess(obj2, prop2)) => {
            let mut mut_obj1 = obj1.clone();
            synchronize_and_hole_lhs(&mut mut_obj1, obj2, next_hole_id, hole_map);
            let new_obj1 = mut_obj1;

            if prop1 != *prop2 {
                let lhs1_str = format!("{}", Lhs::ObjAccess(obj1.clone(), prop1.clone()));
                let lhs2_str = format!("{}", Lhs::ObjAccess(obj2.clone(), prop2.clone()));
                let ph_name = add_to_hole(
                    hole_map,
                    next_hole_id,
                    "lhs_obj_access_prop_diff",
                    lhs1_str,
                    lhs2_str,
                );
                *template_lhs = Lhs::Hole(ph_name);
            } else {
                *template_lhs = Lhs::ObjAccess(new_obj1, prop1.clone());
            }
        }
        (Lhs::This, Lhs::This) => { /* Match, do nothing */ }
        (l1, l2) => {
            if let Lhs::Hole(ph_name) = &l1 {
                let mut found_key = None;
                for key_in_map in hole_map.keys() {
                    if key_in_map.starts_with(ph_name) && key_in_map.contains("(") {
                        found_key = Some(key_in_map.clone());
                        break;
                    }
                }
                if let Some(actual_hole_key) = found_key {
                    update_hole_value(hole_map, &actual_hole_key, format!("{}", l2));
                } else {
                    let l1_str = format!("{}", l1);
                    let l2_str = format!("{}", l2);
                    let new_ph_name =
                        add_to_hole(hole_map, next_hole_id, "lhs_mismatch", l1_str, l2_str);
                    *template_lhs = Lhs::Hole(new_ph_name);
                }
            } else if l1.to_string() != l2.to_string() {
                let l1_str = format!("{}", l1.clone());
                let l2_str = format!("{}", l2);
                let ph_name = add_to_hole(hole_map, next_hole_id, "lhs_value", l1_str, l2_str);
                *template_lhs = Lhs::Hole(ph_name);
            }
        }
    }
}

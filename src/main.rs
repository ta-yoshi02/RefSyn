mod ast;
mod env; // env モジュールを宣言
mod parser;
mod server;

use serde::{Deserialize, Serialize};
use crate::parser::parse_operations;
use crate::env::MemoEnv; // MemoEnv をインポート
use std::collections::HashMap;
use warp::http::StatusCode;
use crate::ast::{Stmt, Placeholder, Expr, Lhs, Program}; // Program を追加

#[derive(Deserialize, Debug)]
pub struct MethodCallOperation { // New struct for individual method call operations
    #[serde(rename = "callLabel")]
    call_label: String,
    #[serde(rename = "contextSensitiveID")]
    context_sensitive_id: String,
    #[serde(rename = "receiverObject")]
    receiver_object: String, // This is the ID of the receiver object
    #[serde(rename = "methodName")]
    method_name: String,
    operations: Vec<serde_json::Value>,
}

#[derive(Deserialize, Debug)]
pub struct SynthesisRequest { // Modified to accept a list of MethodCallOperation
    method_calls: Vec<MethodCallOperation>, // New field
}

#[derive(Debug, Serialize, Deserialize)] // Added Debug and Deserialize
pub struct SynthesisResponse {
    pub common_pattern: Option<String>, // Made public for direct access if needed
    pub hole_information: Option<HashMap<String, Vec<String>>>, // Made public
    pub code: Vec<String>, // Changed to Vec<String> and made public
    pub individual_codes: Vec<String>, // メソッド呼び出しごとのコード
}

fn format_statement(stmt: &ast::Stmt) -> String {
    format!("{}", stmt)
}

fn add_to_hole(
    hole_map: &mut HashMap<String, Vec<String>>, // Changed type
    next_hole_id: &mut usize,
    category: &str,
    value1_str: String,
    value2_str: String,
) -> Placeholder {
    let hole_key = format!("Hole{} ({})", *next_hole_id, category);
    let placeholder_name = format!("Hole{}", *next_hole_id);
    *next_hole_id += 1;
    hole_map.insert(hole_key, vec![value1_str, value2_str]);
    placeholder_name // Return the base name for use in AST::Hole
}

// Helper function to update an existing hole's values
fn update_hole_value(
    hole_map: &mut HashMap<String, Vec<String>>, // Changed type
    placeholder_key: &str, // Changed to take the full key
    new_value_str: String,
) {
    if let Some(values) = hole_map.get_mut(placeholder_key) {
        values.push(new_value_str);
    } else {
        eprintln!("Warning: Attempted to update non-existent hole with key: {}", placeholder_key);
    }
}

fn synchronize_and_hole_expr(
    template_expr: &mut ast::Expr,
    target_expr: &ast::Expr,
    next_hole_id: &mut usize,
    hole_map: &mut HashMap<String, Vec<String>>, // Changed type
) {
    match (template_expr.clone(), target_expr) {
        (ast::Expr::Num(n1), ast::Expr::Num(n2)) => {
            if n1 != *n2 {
                let ph_name = add_to_hole(hole_map, next_hole_id, "num_value", n1.to_string(), n2.to_string());
                *template_expr = ast::Expr::Hole(ph_name);
            }
        }
        (ast::Expr::Str(s1), ast::Expr::Str(s2)) => {
            if s1 != *s2 {
                let ph_name = add_to_hole(hole_map, next_hole_id, "str_value", s1.clone(), s2.clone());
                *template_expr = ast::Expr::Hole(ph_name);
            }
        }
        (ast::Expr::Var(v1), ast::Expr::Var(v2)) => {
            if v1 != *v2 {
                let ph_name = add_to_hole(hole_map, next_hole_id, "var_name", v1.clone(), v2.clone());
                *template_expr = ast::Expr::Hole(ph_name);
            }
        }
        (ast::Expr::New(c1), ast::Expr::New(c2)) => {
            if c1 != *c2 {
                let ph_name = add_to_hole(hole_map, next_hole_id, "class_name", c1.clone(), c2.clone());
                *template_expr = ast::Expr::Hole(ph_name);
            }
        }
        (ast::Expr::Lhs(mut lhs1), ast::Expr::Lhs(lhs2)) => {
            synchronize_and_hole_lhs(&mut lhs1, lhs2, next_hole_id, hole_map);
            *template_expr = ast::Expr::Lhs(lhs1);
        }
        (ast::Expr::MethodCall(obj1, method1, args1), ast::Expr::MethodCall(obj2, method2, args2)) => {
            let mut mut_obj1 = obj1.clone();
            synchronize_and_hole_lhs(&mut mut_obj1, obj2, next_hole_id, hole_map);
            let new_obj1 = mut_obj1;

            if method1 != *method2 {
                let expr1_str = format!("{}", ast::Expr::MethodCall(obj1.clone(), method1.clone(), args1.clone()));
                let expr2_str = format!("{}", ast::Expr::MethodCall(obj2.clone(), method2.clone(), args2.clone()));
                let ph_name_call = add_to_hole(hole_map, next_hole_id, "method_call_expr", expr1_str, expr2_str);
                *template_expr = ast::Expr::Hole(ph_name_call);
                return;
            }
            if args1.len() != args2.len() {
                let expr1_str = format!("{}", ast::Expr::MethodCall(obj1.clone(), method1.clone(), args1.clone()));
                let expr2_str = format!("{}", ast::Expr::MethodCall(obj2.clone(), method2.clone(), args2.clone()));
                let ph_name_call = add_to_hole(hole_map, next_hole_id, "method_call_expr_args_len_diff", expr1_str, expr2_str);
                *template_expr = ast::Expr::Hole(ph_name_call);
                return;
            }

            let mut new_args1 = Vec::new();
            for (arg1, arg2) in args1.iter().zip(args2.iter()) {
                let mut mut_arg1 = arg1.clone();
                synchronize_and_hole_expr(&mut mut_arg1, arg2, next_hole_id, hole_map);
                new_args1.push(mut_arg1);
            }
            *template_expr = ast::Expr::MethodCall(new_obj1, method1, new_args1);
        }
        (e1, e2) => {
            if let ast::Expr::Hole(ph_name) = &e1 {
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
                    let new_ph_name = add_to_hole(hole_map, next_hole_id, "expr_mismatch", e1_str, e2_str);
                    *template_expr = ast::Expr::Hole(new_ph_name);
                }
            } else if e1.to_string() != e2.to_string() {
                let e1_str = format!("{}", e1.clone());
                let e2_str = format!("{}", e2);
                let ph_name = add_to_hole(hole_map, next_hole_id, "expr_value", e1_str, e2_str);
                *template_expr = ast::Expr::Hole(ph_name);
            }
        }
    }
}

fn synchronize_and_hole_lhs(
    template_lhs: &mut ast::Lhs,
    target_lhs: &ast::Lhs,
    next_hole_id: &mut usize,
    hole_map: &mut HashMap<String, Vec<String>>, // Changed type
) {
    match (template_lhs.clone(), target_lhs) {
        (ast::Lhs::Var(v1), ast::Lhs::Var(v2)) => {
            if v1 != *v2 {
                let ph_name = add_to_hole(hole_map, next_hole_id, "lhs_var_name", v1, v2.clone());
                *template_lhs = ast::Lhs::Hole(ph_name);
            }
        }
        (ast::Lhs::ObjAccess(obj1, prop1), ast::Lhs::ObjAccess(obj2, prop2)) => {
            let mut mut_obj1 = obj1.clone();
            synchronize_and_hole_lhs(&mut mut_obj1, obj2, next_hole_id, hole_map);
            let new_obj1 = mut_obj1;

            if prop1 != *prop2 {
                let lhs1_str = format!("{}", ast::Lhs::ObjAccess(obj1.clone(), prop1.clone()));
                let lhs2_str = format!("{}", ast::Lhs::ObjAccess(obj2.clone(), prop2.clone()));
                let ph_name = add_to_hole(hole_map, next_hole_id, "lhs_obj_access_prop_diff", lhs1_str, lhs2_str);
                *template_lhs = ast::Lhs::Hole(ph_name);
            } else {
                *template_lhs = ast::Lhs::ObjAccess(new_obj1, prop1.clone());
            }
        }
        (ast::Lhs::This, ast::Lhs::This) => { /* Match, do nothing */ }
        (l1, l2) => {
            if let ast::Lhs::Hole(ph_name) = &l1 {
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
                    let new_ph_name = add_to_hole(hole_map, next_hole_id, "lhs_mismatch", l1_str, l2_str);
                    *template_lhs = ast::Lhs::Hole(new_ph_name);
                }
            } else if l1.to_string() != l2.to_string() {
                let l1_str = format!("{}", l1.clone());
                let l2_str = format!("{}", l2);
                let ph_name = add_to_hole(hole_map, next_hole_id, "lhs_value", l1_str, l2_str);
                *template_lhs = ast::Lhs::Hole(ph_name);
            }
        }
    }
}

fn synchronize_and_hole_stmt(
    template_stmt: &mut ast::Stmt,
    target_stmt: &ast::Stmt,
    next_hole_id: &mut usize,
    hole_map: &mut HashMap<String, Vec<String>>, // Changed type
) {
    match (template_stmt.clone(), target_stmt) {
        (
            Stmt::VarDecl { name: name1, expr: mut expr1 },
            Stmt::VarDecl { name: name2, expr: expr2 },
        ) => {
            if name1 != *name2 {
                let stmt1_str = format!("{}", Stmt::VarDecl { name: name1.clone(), expr: expr1.clone() });
                let stmt2_str = format!("{}", Stmt::VarDecl { name: name2.clone(), expr: expr2.clone() });
                let ph_name = add_to_hole(hole_map, next_hole_id, "var_decl_stmt", stmt1_str, stmt2_str);
                *template_stmt = Stmt::Hole(ph_name);
                return;
            }
            synchronize_and_hole_expr(&mut expr1, expr2, next_hole_id, hole_map);
            *template_stmt = Stmt::VarDecl { name: name1, expr: expr1 };
        }
        (
            Stmt::Assign { lhs: mut lhs1, expr: mut expr1 },
            Stmt::Assign { lhs: lhs2, expr: expr2 },
        ) => {
            synchronize_and_hole_lhs(&mut lhs1, lhs2, next_hole_id, hole_map);
            synchronize_and_hole_expr(&mut expr1, expr2, next_hole_id, hole_map);
            *template_stmt = Stmt::Assign { lhs: lhs1, expr: expr1 };
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
                    let new_ph_name = add_to_hole(hole_map, next_hole_id, "stmt_mismatch", s1_str, s2_str);
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

fn find_common_pattern_and_holes(
    programs: &[ast::Program],
    memo_envs: &[MemoEnv], // Add MemoEnv slice as parameter
) -> (Option<ast::Program>, HashMap<String, Vec<String>>) {
    if programs.is_empty() {
        return (None, HashMap::new());
    }
    if programs.len() == 1 {
        return (Some(programs[0].clone()), HashMap::new());
    }

    let mut hole_map: HashMap<String, Vec<String>> = HashMap::new(); // Changed type
    let mut next_hole_id = 1; // Changed from 0 to 1 to start holes from Hole1

    let overall_min_stmts = programs.iter().map(|p| p.stmts.len()).min().unwrap_or(0);

    let mut template_program = programs[0].clone();
    template_program.stmts.truncate(overall_min_stmts);

    for i in 1..programs.len() {
        let target_program = &programs[i];
        for stmt_idx in 0..overall_min_stmts {
            if stmt_idx < template_program.stmts.len() && stmt_idx < target_program.stmts.len() {
                synchronize_and_hole_stmt(
                    &mut template_program.stmts[stmt_idx],
                    &target_program.stmts[stmt_idx],
                    &mut next_hole_id,
                    &mut hole_map,
                );
            } else {
                break; 
            }
        }
    }

    (Some(template_program), hole_map)
}

pub async fn handle_synthesis(req: SynthesisRequest) -> Result<impl warp::Reply, warp::Rejection> {
    println!("Received synthesis request: {:?}", req);

    let mut parsed_programs = Vec::new();
    let mut all_memo_envs_for_pattern_finding = Vec::new(); // Store MemoEnv for each call

    if req.method_calls.is_empty() {
        println!("No method calls provided in the request.");
        let response = SynthesisResponse {
            common_pattern: Some("No method calls provided.".to_string()),
            hole_information: None,
            code: vec![],
            individual_codes: vec![],
        };
        return Ok(warp::reply::with_status(warp::reply::json(&response), StatusCode::OK));
    }

    for (index, method_call_op) in req.method_calls.iter().enumerate() {
        println!(
            "Processing method call: {}, Receiver: {}, Method: {}",
            method_call_op.call_label,
            method_call_op.receiver_object,
            method_call_op.method_name
        );

        let mut memo_env_for_call = MemoEnv::new();
        // Set 'this' for the current method call context
        memo_env_for_call.add_special_mapping(method_call_op.receiver_object.clone(), "this".to_string());
        
        // Use a unique scope ID for each method call, e.g., based on index or callLabel
        let scope_id = format!("method_call_{}_{}", index, method_call_op.call_label);
        memo_env_for_call.start_method_call_scope(&scope_id);

        match parse_operations(
            &method_call_op.operations,
            Some(&method_call_op.receiver_object), // Pass receiver_object as current_receiver_id
            &mut memo_env_for_call,
        ) {
            Ok(program) => {
                println!(
                    "Parsed AST for {}: \n{}",
                    method_call_op.call_label,
                    program
                );
                parsed_programs.push(program);
                all_memo_envs_for_pattern_finding.push(memo_env_for_call.clone()); // Clone and store
            }
            Err(e) => {
                eprintln!(
                    "Error parsing operations for {}: {}",
                    method_call_op.call_label,
                    e
                );
                // Consider how to handle partial failures. For now, return an error for the whole request.
                let response = SynthesisResponse {
                    common_pattern: None,
                    hole_information: None,
                    code: vec![format!(
                        "Error parsing operations for {}: {}",
                        method_call_op.call_label,
                        e
                    )],
                    individual_codes: vec![],
                };
                return Ok(warp::reply::with_status(
                    warp::reply::json(&response),
                    StatusCode::BAD_REQUEST,
                ));
            }
        }
    }

    if parsed_programs.is_empty() {
        println!("No ASTs were parsed successfully from any method call.");
        let response = SynthesisResponse {
            common_pattern: Some("No operations to synthesize or error during parsing.".to_string()),
            hole_information: None,
            code: vec![],
            individual_codes: vec![],
        };
        return Ok(warp::reply::with_status(warp::reply::json(&response), StatusCode::OK));
    }

    // 4. AST のリストから共通パターンとホールを抽出
    let (common_pattern_ast_option, hole_map) = find_common_pattern_and_holes(&parsed_programs, &all_memo_envs_for_pattern_finding);
    
    let common_pattern_ast = match common_pattern_ast_option {
        Some(ast) => {
            println!("Common Pattern AST: \n{}", ast);
            ast
        }
        None => {
            println!("No common pattern could be determined.");
            Program { stmts: vec![] }
        }
    };
    
    println!("Initial Hole Map: {:?}", hole_map);

    // 5. レスポンスを生成
    let common_pattern_str = format!("{}", common_pattern_ast);
    let individual_codes_str: Vec<String> = parsed_programs
        .iter()
        .map(|p| format!("{}", p))
        .collect();

    let formatted_hole_info = if !hole_map.is_empty() {
        let mut info = HashMap::new();
        for (hole_id_with_cat, values) in &hole_map { 
            info.insert(hole_id_with_cat.clone(), values.clone());
        }
        Some(info)
    } else {
        None
    };
    println!("Formatted Hole Info: {:?}", formatted_hole_info);

    let response = SynthesisResponse {
        common_pattern: Some(common_pattern_str),
        hole_information: formatted_hole_info,
        code: individual_codes_str.clone(),
        individual_codes: individual_codes_str,
    };
    println!("Final Response: {:?}", response);

    Ok(warp::reply::with_status(warp::reply::json(&response), StatusCode::OK))
}

#[tokio::main]
async fn main() {
    println!("Starting RefSyn server...");
    server::run_server().await;
}

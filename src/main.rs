mod ast;
mod parser;

use warp::Filter;
use serde::{Deserialize, Serialize};
use warp::http::Method;
use crate::parser::parse_operations;
use std::collections::HashMap;

// メソッドIDからインデックスを抽出
fn extract_method_index(id: &str) -> Option<i32> {
    // "method_1", "method1" または "__temp1" のようなパターンを検出
    let digits: String = id.chars().filter(|c| c.is_digit(10)).collect();
    digits.parse::<i32>().ok()
}

// __tempID から数値部分を抽出するヘルパー関数
fn extract_temp_id_number(id: &str) -> Option<i32> {
    if id.starts_with("__temp") {
        let digits = id.chars()
            .filter(|c| c.is_digit(10))
            .collect::<String>();
        return digits.parse::<i32>().ok();
    }
    None
}

// 操作が特定のグループに属するかを判定
fn belongs_to_group(op: &serde_json::Value, group_id: &str) -> bool {
    // IDフィールドのチェック
    if let Some(id) = op.get("id").and_then(|v| v.as_str()) {
        if id.contains(group_id) {
            return true;
        }
    }
    
    // fromフィールドのチェック
    if let Some(from) = op.get("from").and_then(|v| v.as_str()) {
        if from.contains(group_id) {
            return true;
        }
    }
    
    // toフィールドのチェック
    if let Some(to) = op.get("to").and_then(|v| v.as_str()) {
        if to.contains(group_id) {
            return true;
        }
    }
    
    false
}

#[derive(Deserialize, Debug)]
struct SynthesisRequest {
    operations: Vec<serde_json::Value>,
    code_lines: Vec<String>,
    context_id: Option<String>, // メソッド呼び出しの文脈情報を追加
}

#[derive(Serialize)]
struct SynthesisResponse {
    code: String,
}

#[derive(Serialize)]
struct ComparisonResponse {
    common: Vec<String>,
    differing: Vec<String>,
}

fn generate_code(operations: &Vec<serde_json::Value>, code_lines: &Vec<String>) -> String {
    let mut result = String::new();
    result.push_str("// Generated code:\n\n");

    // メタデータとしての操作内容を表示
    for op in operations {
        if let Ok(op_str) = serde_json::to_string_pretty(op) {
            result.push_str(&format!("// operation: {}\n", op_str));
        } else {
            result.push_str("// operation: <failed to serialize>\n");
        }
    }

    // 操作をASTに変換
    let ops_result = parser::parse_operations(operations);
    if let Err(e) = ops_result {
        result.push_str(&format!("\n// Error parsing operations: {}\n", e));
        // 元のコード行を追加
        for line in code_lines {
            result.push_str(&format!("{}\n", line));
        }
        return result;
    }

    let (_program, common_stmts, differing_stmts) = ops_result.unwrap();

    // メソッド呼び出し別のコード生成
    result.push_str("\n// メソッド呼び出し別コード:\n");
    
    // Kanonからの操作データに基づいて、呼び出し単位でグループ化
    let mut method_calls = extract_method_calls_from_operations(operations, code_lines);
    
    // メソッド呼び出しを番号でソート（call1, call2, ...の順に）
    method_calls.sort_by(|(a, _), (b, _)| {
        let a_num = extract_call_number(a);
        let b_num = extract_call_number(b);
        a_num.cmp(&b_num)
    });
    
    // メソッド呼び出し単位でステートメントをグループ化
    let mut call_blocks = Vec::new();
    for (call_info, call_ops) in &method_calls {
        if let Ok((method_program, _, _)) = parser::parse_operations(call_ops) {
            // 各メソッド呼び出しのステートメントを保存（トポロジカルソートした順序で）
            let sorted_stmts = sort_statements(&method_program.stmts);
            call_blocks.push(sorted_stmts.clone());
            
            // メソッド呼び出し情報を表示
            result.push_str(&format!("\n// {}\n", call_info));
            
            // トポロジカルソートしたステートメントを表示
            for stmt in &sorted_stmts {
                let stmt_str = format_statement(stmt);
                result.push_str(&format!("{}\n", stmt_str));
            }
        }
    }
    
    // 共通パターンの抽出と表示
    if call_blocks.len() > 1 {
        result.push_str("\n// 共通パターン (ホール表現):\n");
        let common_template = parser::extract_common_template(&call_blocks);
        result.push_str(&common_template);
    } else {
        result.push_str("\n// 共通パターンを抽出するには2つ以上のメソッド呼び出しが必要です\n");
    }
    
    // 共通部分と差分部分の表示
    if !common_stmts.is_empty() {
        result.push_str("\n// 共通部分:\n");
        for stmt in &common_stmts {
            result.push_str(&format!("{}\n", stmt));
        }
    }

    if !differing_stmts.is_empty() {
        result.push_str("\n// 差分部分:\n");
        for stmt in &differing_stmts {
            result.push_str(&format!("{}\n", stmt));
        }
    }

    // 元のコード行も表示
    result.push_str("\n// 元のメソッド呼び出し:\n");
    for line in code_lines {
        result.push_str(&format!("{}\n", line));
    }

    result
}

// メソッド呼び出し番号を抽出する補助関数
fn extract_call_number(call_info: &str) -> i32 {
    // "call1", "call2" などから数字部分を抽出
    let digits: String = call_info.chars()
        .filter(|c| c.is_digit(10))
        .collect();
    digits.parse::<i32>().unwrap_or(i32::MAX) // 解析できない場合は大きな値
}

// ステートメントを適切な文字列形式にフォーマットする
fn format_statement(stmt: &ast::Stmt) -> String {
    match stmt {
        ast::Stmt::VarDecl { name, expr } => {
            let expr_str = match expr {
                ast::Expr::New(class_name) => format!("new {}", class_name),
                ast::Expr::Num(n) => n.to_string(),
                ast::Expr::Str(s) => format!("\"{}\"", s),
                ast::Expr::This => "this".to_string(),
                ast::Expr::MethodCall(obj, method, args) => {
                    // オブジェクトの処理
                    let processed_obj = process_lhs_obj(obj);
                    let obj_str = match &processed_obj {
                        ast::Lhs::Var(name) => name.clone(),
                        ast::Lhs::ObjAccess(inner_obj, prop) => {
                            // ネストされたオブジェクトアクセスを処理
                            let inner_obj_str = match &**inner_obj {
                                ast::Lhs::Var(obj_name) => {
                                    if obj_name == "main-new1" {
                                        "this".to_string()
                                    } else {
                                        obj_name.clone()
                                    }
                                },
                                _ => format!("<complex>"),
                            };
                            format!("{}.{}", inner_obj_str, prop)
                        },
                        _ => "this".to_string(),
                    };
                    
                    let args_str = args.iter()
                        .map(|arg| match arg {
                            ast::Expr::Var(name) => name.clone(),
                            ast::Expr::Num(n) => n.to_string(),
                            ast::Expr::Str(s) => format!("\"{}\"", s),
                            ast::Expr::This => "this".to_string(),
                            _ => format!("{:?}", arg),
                        })
                        .collect::<Vec<String>>()
                        .join(", ");
                    
                    format!("{}.{}({})", obj_str, method, args_str)
                },
                _ => format!("{:?}", expr),
            };
            format!("var {} = {};", name, expr_str)
        },
        ast::Stmt::Assign { lhs, expr } => {
            // 左辺値の処理
            let processed_lhs = process_lhs_obj(lhs);
            let lhs_str = match &processed_lhs {
                ast::Lhs::Var(name) => {
                    if name == "main-new1" {
                        "this".to_string()
                    } else {
                        name.clone()
                    }
                },
                ast::Lhs::ObjAccess(obj, prop) => {
                    let obj_str = match &**obj {
                        ast::Lhs::Var(obj_name) => {
                            if obj_name == "main-new1" {
                                "this".to_string()
                            } else {
                                obj_name.clone()
                            }
                        },
                        ast::Lhs::ObjAccess(inner_obj, inner_prop) => {
                            // ネストされたアクセスを処理
                            let inner_str = format_statement(&ast::Stmt::Assign {
                                lhs: ast::Lhs::ObjAccess(inner_obj.clone(), inner_prop.clone()),
                                expr: ast::Expr::Var("dummy".to_string())
                            });
                            // "inner_obj.prop = dummy;" 形式から "inner_obj.prop" 部分を抽出
                            inner_str.trim_end_matches(" = dummy;").to_string()
                        },
                        _ => format!("<complex>"),
                    };
                    format!("{}.{}", obj_str, prop)
                },
                _ => format!("{:?}", &processed_lhs),
            };
            
            let expr_str = match expr {
                ast::Expr::Var(name) => {
                    if name == "main-new1" {
                        "this".to_string()
                    } else {
                        name.clone()
                    }
                },
                ast::Expr::This => "this".to_string(),
                ast::Expr::Num(n) => n.to_string(),
                ast::Expr::Str(s) => format!("\"{}\"", s),
                ast::Expr::MethodCall(obj, method, args) => {
                    let processed_obj = process_lhs_obj(obj);
                    let obj_str = match &processed_obj {
                        ast::Lhs::Var(name) => {
                            if name == "main-new1" {
                                "this".to_string()
                            } else {
                                name.clone()
                            }
                        },
                        _ => "this".to_string(),
                    };
                    
                    let args_str = args.iter()
                        .map(|arg| match arg {
                            ast::Expr::Var(name) => {
                                if name == "main-new1" {
                                    "this".to_string()
                                } else {
                                    name.clone()
                                }
                            },
                            ast::Expr::Num(n) => n.to_string(),
                            ast::Expr::Str(s) => format!("\"{}\"", s),
                            _ => format!("{:?}", arg),
                        })
                        .collect::<Vec<String>>()
                        .join(", ");
                    
                    format!("{}.{}({})", obj_str, method, args_str)
                },
                _ => format!("{:?}", expr),
            };
            
            format!("{} = {};", lhs_str, expr_str)
        },
        ast::Stmt::Expr(expr) => {
            match expr {
                ast::Expr::MethodCall(obj, method, args) => {
                    let processed_obj = process_lhs_obj(obj);
                    let obj_str = match &processed_obj {
                        ast::Lhs::Var(name) => {
                            if name == "main-new1" {
                                "this".to_string()
                            } else {
                                name.clone()
                            }
                        },
                        ast::Lhs::ObjAccess(inner_obj, prop) => {
                            let obj_name = match &**inner_obj {
                                ast::Lhs::Var(name) => {
                                    if name == "main-new1" {
                                        "this".to_string()
                                    } else {
                                        name.clone()
                                    }
                                },
                                _ => format!("<complex>"),
                            };
                            format!("{}.{}", obj_name, prop)
                        },
                        _ => "this".to_string(),
                    };
                    
                    let args_str = args.iter()
                        .map(|arg| match arg {
                            ast::Expr::Var(name) => {
                                if name == "main-new1" {
                                    "this".to_string()
                                } else {
                                    name.clone()
                                }
                            },
                            ast::Expr::Num(n) => n.to_string(),
                            ast::Expr::Str(s) => format!("\"{}\"", s),
                            ast::Expr::This => "this".to_string(),
                            _ => format!("{:?}", arg),
                        })
                        .collect::<Vec<String>>()
                        .join(", ");
                    
                    format!("{}.{}({});", obj_str, method, args_str)
                },
                _ => format!("{:?};", expr),
            }
        },
    }
}

// Lhs::ObjAccess の場合にもmain-new1をthisに置き換え
fn process_lhs_obj(obj: &ast::Lhs) -> ast::Lhs {
    match obj {
        ast::Lhs::Var(name) => {
            if name == "main-new1" {
                ast::Lhs::Var("this".to_string())
            } else {
                obj.clone()
            }
        },
        ast::Lhs::ObjAccess(inner_obj, prop) => {
            let processed_obj = process_lhs_obj(inner_obj);
            ast::Lhs::ObjAccess(Box::new(processed_obj), prop.clone())
        },
        _ => obj.clone(),
    }
}

// メソッド呼び出しのステートメントをトポロジカルソートする関数
fn sort_statements(stmts: &[ast::Stmt]) -> Vec<ast::Stmt> {
    let mut result = Vec::new();
    let mut var_decls = Vec::new();
    let mut assignments = Vec::new();
    let mut other_stmts = Vec::new();
    
    // ステートメントを種類ごとに分類
    for stmt in stmts {
        match stmt {
            ast::Stmt::VarDecl { .. } => var_decls.push(stmt.clone()),
            ast::Stmt::Assign { .. } => assignments.push(stmt.clone()),
            _ => other_stmts.push(stmt.clone()),
        }
    }
    
    // 変数宣言を最初に配置
    result.extend(var_decls);
    
    // 代入を次に配置
    result.extend(assignments);
    
    // その他のステートメントを最後に配置
    result.extend(other_stmts);
    
    result
}

// コード行からメソッド呼び出しを抽出（より汎用的な実装）
fn extract_method_call_from_code(line: &str) -> Option<String> {
    // JavaScriptのメソッド呼び出しパターンを検出: obj.method(args)
    let line = line.trim();
    
    // JavaScript構文をチェック
    if let Some(dot_pos) = line.find('.') {
        if dot_pos > 0 && dot_pos < line.len() - 1 {
            // ドットの前の部分がオブジェクト名
            let obj_part = &line[..dot_pos];
            let obj_name = obj_part.trim().trim_end_matches(|c: char| !c.is_alphanumeric() && c != '_');
            
            // メソッド呼び出しを検出
            if let Some(paren_open) = line[dot_pos..].find('(') {
                let after_dot = dot_pos + 1;
                let method_name = &line[after_dot..after_dot + paren_open - 1].trim();
                
                // 引数部分を抽出
                let args_start = dot_pos + paren_open + 1;
                if let Some(remaining) = line.get(args_start..) {
                    if let Some(paren_close) = remaining.find(')') {
                        let args = &remaining[..paren_close].trim();
                        
                        // 完全なメソッド呼び出し文字列を構築
                        return Some(format!("{}.{}({})", 
                            if obj_name.is_empty() { "this" } else { obj_name },
                            method_name,
                            args
                        ));
                    }
                }
            }
        }
    }
    
    None
}

// コメント行からより汎用的なコンテキスト情報を抽出
fn extract_context_from_comment(line: &str) -> Option<String> {
    let line = line.trim();
    
    // "コンテキスト:" パターンを検出
    if let Some(context_pos) = line.find("コンテキスト:") {
        let after_context = &line[context_pos + "コンテキスト:".len()..];
        
        // コンテキスト部分を抽出し、余分な記号を取り除く
        let context = after_context.trim()
            .trim_end_matches(|c| c == ')' || c == '）' || c == ',' || c == '.' || c == ':')
            .trim();
        
        if !context.is_empty() {
            // callパターンとcontextを分離
            let mut call_pattern = "method".to_string();
            let mut call_param = String::new();
            
            // "call1" や "call_1" パターンを検出
            for (word_idx, word) in line.split_whitespace().enumerate() {
                if word.starts_with("call") {
                    call_pattern = word.to_string();
                    // "call1"から数字部分を抽出
                    let digits: String = word.chars()
                        .filter(|c| c.is_digit(10))
                        .collect();
                    
                    if !digits.is_empty() {
                        call_param = digits;
                    } else {
                        call_param = (word_idx + 1).to_string();
                    }
                    break;
                }
            }
            
            // オブジェクト名.メソッド名(パラメータ) の形式で返す
            return Some(format!("{}.{}({})", 
                       if context == "main" { "Object" } else { context },
                       call_pattern,
                       call_param));
        }
    }
    
    None
}

// 操作データからメソッド呼び出し情報を直接抽出する関数
fn extract_method_info_from_operations(operations: &[serde_json::Value]) -> Vec<(String, String)> {
    let mut method_calls = Vec::new();
    let mut method_indices = HashMap::new();
    
    // methodCallオペレーションを探す
    for op in operations {
        if let Some(edit_type) = op.get("editType").and_then(|v| v.as_str()) {
            if edit_type == "methodCall" {
                if let Some(label) = op.get("label").and_then(|v| v.as_str()) {
                    if let Some(id) = op.get("id").and_then(|v| v.as_str()) {
                        // メソッド名
                        let method_name = if label.contains(".") {
                            label.to_string()
                        } else if id.contains("__") {
                            // IDに基づいてメソッド名を構築
                            let parts: Vec<&str> = id.split("__").collect();
                            if parts.len() >= 2 {
                                format!("{}.{}", parts[0], label)
                            } else {
                                format!("object.{}", label)
                            }
                        } else {
                            format!("object.{}", label)
                        };
                        
                        // パラメータ情報を抽出
                        let mut param_value = String::new();
                        if let Some(params) = extract_parameter_value_for_method(operations, id) {
                            param_value = params;
                        }
                        
                        // メソッド呼び出し文字列を構築
                        let method_call = if param_value.is_empty() {
                            method_name.clone()
                        } else {
                            format!("{}({})", method_name, param_value)
                        };
                        
                        method_calls.push((method_call, id.to_string()));
                        
                        // メソッドインデックスを保存
                        if let Some(idx) = extract_method_index(id) {
                            method_indices.insert(id.to_string(), idx);
                        }
                    }
                }
            }
        }
    }
    
    // メソッドインデックスでソート
    method_calls.sort_by_key(|(_, id)| method_indices.get(id).cloned().unwrap_or(0));
    
    method_calls
}

// メソッド呼び出しのパラメータ値を抽出
fn extract_parameter_value_for_method(operations: &[serde_json::Value], method_id: &str) -> Option<String> {
    for op in operations {
        if let Some(is_lit) = op.get("isLiteral").and_then(|v| v.as_bool()) {
            if is_lit {
                if let Some(id) = op.get("id").and_then(|v| v.as_str()) {
                    // "__temp2" のようなIDが、"__temp1" のメソッドに関連するパラメータかを確認
                    let method_idx = extract_method_index(method_id)?;
                    let param_idx = extract_method_index(id)?;
                    
                    if param_idx > method_idx && param_idx - method_idx <= 2 {
                        if let Some(label) = op.get("label").and_then(|v| v.as_str()) {
                            return Some(label.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

// 操作パターンからメソッド呼び出しを推測
fn infer_method_patterns_from_operations(operations: &[serde_json::Value]) -> Vec<String> {
    let mut patterns = Vec::new();
    let mut temp_ids = Vec::new();
    let mut literal_values = HashMap::new();
    
    // まず全ての__tempIDとリテラル値を収集
    for op in operations {
        if let Some(id) = op.get("id").and_then(|v| v.as_str()) {
            if id.starts_with("__temp") {
                temp_ids.push(id.to_string());
                
                // リテラル値を記録
                if let Some(is_lit) = op.get("isLiteral").and_then(|v| v.as_bool()) {
                    if is_lit {
                        if let Some(label) = op.get("label").and_then(|v| v.as_str()) {
                            literal_values.insert(id.to_string(), label.to_string());
                        }
                    }
                }
            }
        }
    }
    
    // エッジ情報を解析してオブジェクト・プロパティ関係を特定
    let mut object_properties = HashMap::new();
    
    for op in operations {
        if let Some(edit_type) = op.get("editType").and_then(|v| v.as_str()) {
            if edit_type == "addEdge" {
                if let Some(from) = op.get("from").and_then(|v| v.as_str()) {
                    if let Some(to) = op.get("to").and_then(|v| v.as_str()) {
                        if let Some(label) = op.get("label").and_then(|v| v.as_str()) {
                            // プロパティ関係を記録 (例: __temp1.val = __temp2)
                            object_properties.insert(
                                (from.to_string(), label.to_string()),
                                to.to_string()
                            );
                        }
                    }
                }
            }
        }
    }
    
    // append/addメソッド呼び出しのパターンを検出
    // 通常、連結リストでは__tempN → __tempN+1 の関係で、Nが奇数というパターンがある
    for i in (1..temp_ids.len()).step_by(2) {
        if i + 1 < temp_ids.len() {
            let current_id = &temp_ids[i];
            let next_id = &temp_ids[i + 1];
            
            // 'next'関係があるかを確認
            let has_next_relation = object_properties.contains_key(&(current_id.clone(), "next".to_string()));
            
            // メソッド名を推測
            let method_name = if has_next_relation {
                "append" // 連結リスト的なオブジェクトではappendが一般的
            } else {
                "add" // その他の場合はaddを仮定
            };
            
            // オブジェクト名を推測
            let object_name = if current_id.ends_with("1") {
                "lst" // 最初のノードに関連する場合（例: __temp1）
            } else if current_id.ends_with("3") {
                "node" // 2番目のノード（例: __temp3）
            } else {
                "obj" // その他
            };
            
            // パラメータ値を取得
            let param_value = literal_values.get(next_id).cloned().unwrap_or_else(|| {
                format!("value{}", (i / 2) + 1)
            });
            
            // メソッド呼び出しパターンを構築
            patterns.push(format!("{}.{}({})", object_name, method_name, param_value));
        }
    }
    
    // 検出できなかった場合のフォールバック
    if patterns.is_empty() && !temp_ids.is_empty() {
        // __tempIDの数に基づいてメソッド呼び出し回数を推測
        let count = (temp_ids.len() + 1) / 2;
        for i in 0..count {
            // オブジェクト.メソッド(引数) の形式で生成
            patterns.push(format!("obj.method({})", i + 1));
        }
    }
    
    patterns
}

// 操作データからメソッド呼び出し情報を抽出する関数
fn extract_method_calls_from_operations(operations: &[serde_json::Value], code_lines: &[String]) -> Vec<(String, Vec<serde_json::Value>)> {
    // 結果格納用
    let mut method_calls = Vec::new();
    
    // 操作データから直接抽出できるか試みる
    let extracted_calls = extract_method_info_from_operations(operations);
    
    // 成功した場合はそれを使用
    if !extracted_calls.is_empty() {
        println!("操作データから直接メソッド呼び出し情報を抽出しました: {:?}", extracted_calls);
        for (call_info, group_id) in extracted_calls {
            // グループIDに基づいて操作をフィルタリング
            let group_ops: Vec<serde_json::Value> = operations.iter()
                .filter(|op| belongs_to_group(op, &group_id))
                .cloned()
                .collect();
            
            if !group_ops.is_empty() {
                method_calls.push((call_info, group_ops));
            }
        }
        
        if !method_calls.is_empty() {
            return method_calls;
        }
    }
    
    // 操作データから直接抽出できなかった場合は、コード行から抽出
    let mut method_patterns = Vec::new();
    
    // コード行を解析
    for line in code_lines {
        // JavaScriptのメソッド呼び出しを検出 (foo.bar(args) 形式)
        if let Some(obj_method) = extract_method_call_from_code(line) {
            method_patterns.push(obj_method);
        } 
        // コメント行からのコンテキスト情報
        else if line.contains("メソッド呼び出し") || line.contains("コンテキスト:") {
            if let Some(context_info) = extract_context_from_comment(line) {
                method_patterns.push(context_info);
            }
        }
    }
    
    println!("コード行からメソッド呼び出し情報を抽出しました: {:?}", method_patterns);
    
    // 操作データからも抽出できなかった場合、操作パターンからの推測を試みる
    if method_patterns.is_empty() {
        method_patterns = infer_method_patterns_from_operations(operations);
        println!("操作パターンから推測したメソッド呼び出し: {:?}", method_patterns);
    }
    
    // それでも見つからない場合は汎用的な名前を使用
    if method_patterns.is_empty() && operations.len() >= 4 {
        let call_count = operations.len() / 4; // 約4つの操作で1回の呼び出しと仮定
        for i in 0..call_count {
            method_patterns.push(format!("object.method({})", i + 1));
        }
    }
    
    // メソッドパターンに基づいて操作をグループ化
    let classified = classify_operations_by_ast(operations, &method_patterns);
    if !classified.is_empty() {
        return classified;
    }
    
    // 最後の手段：均等に分割
    split_operations_evenly(operations, &method_patterns)
}

// AST情報を使って操作を分類する
fn classify_operations_by_ast(operations: &[serde_json::Value], method_patterns: &[String]) -> Vec<(String, Vec<serde_json::Value>)> {
    let mut result = Vec::new();
    let pattern_count = method_patterns.len() as i32;
    
    if pattern_count == 0 {
        return result;
    }
    
    // 前処理で依存関係に基づいたグループを作成
    let operation_groups = preprocess_operations(operations);
    
    // グループごとに操作を収集
    let mut group_map: HashMap<i32, Vec<serde_json::Value>> = HashMap::new();
    
    // テンプID→一時変数名のマッピングを記録
    let mut temp_id_mapping: HashMap<String, String> = HashMap::new();
    let mut group_var_counters: HashMap<i32, i32> = HashMap::new();
    
    // 前処理：テンプIDに一貫した変数名を割り当て
    for (op_idx, group_id) in &operation_groups {
        if let Some(op) = operations.get(*op_idx) {
            // ノード追加操作からテンプIDを取得
            if let Some(edit_type) = op.get("editType").and_then(|v| v.as_str()) {
                if edit_type == "addNode" {
                    if let Some(id) = op.get("id").and_then(|v| v.as_str()) {
                        if id.starts_with("__temp") {
                            // グループごとに変数カウンターを管理
                            let mod_group_id = group_id % pattern_count;
                            let var_idx = group_var_counters.entry(mod_group_id).or_insert(0);
                            let var_name = format!("v{}", *var_idx);
                            *var_idx += 1;
                            
                            // テンプID→変数名のマッピングを記録
                            temp_id_mapping.insert(id.to_string(), var_name);
                        }
                    }
                }
            }
        }
    }
    
    // 操作を変換しながらグループに追加
    for (op_idx, group_id) in &operation_groups {
        if let Some(op) = operations.get(*op_idx) {
            let mod_group_id = group_id % pattern_count;
            
            // 操作を複製して変数名を置換
            let mut modified_op = op.clone();
            
            // ID置換
            if let Some(id) = op.get("id").and_then(|v| v.as_str()) {
                if id.starts_with("__temp") {
                    if let Some(var_name) = temp_id_mapping.get(id) {
                        if let Some(obj) = modified_op.as_object_mut() {
                            obj.insert("original_id".to_string(), serde_json::Value::String(id.to_string()));
                            obj.insert("id".to_string(), serde_json::Value::String(var_name.clone()));
                        }
                    }
                }
            }
            
            // from置換
            if let Some(from) = op.get("from").and_then(|v| v.as_str()) {
                if from.starts_with("__temp") {
                    if let Some(var_name) = temp_id_mapping.get(from) {
                        if let Some(obj) = modified_op.as_object_mut() {
                            obj.insert("original_from".to_string(), serde_json::Value::String(from.to_string()));
                            obj.insert("from".to_string(), serde_json::Value::String(var_name.clone()));
                        }
                    }
                } else if from == "main-new1" {
                    // main-new1は特殊ケース（thisの表現）
                    if let Some(obj) = modified_op.as_object_mut() {
                        obj.insert("original_from".to_string(), serde_json::Value::String(from.to_string()));
                        obj.insert("from".to_string(), serde_json::Value::String("this".to_string()));
                    }
                }
            }
            
            // to置換
            if let Some(to) = op.get("to").and_then(|v| v.as_str()) {
                if to.starts_with("__temp") {
                    if let Some(var_name) = temp_id_mapping.get(to) {
                        if let Some(obj) = modified_op.as_object_mut() {
                            obj.insert("original_to".to_string(), serde_json::Value::String(to.to_string()));
                            obj.insert("to".to_string(), serde_json::Value::String(var_name.clone()));
                        }
                    }
                }
            }
            
            // グループに追加
            group_map.entry(mod_group_id).or_insert_with(Vec::new).push(modified_op);
        }
    }
    
    // グループ未割り当ての操作を処理
    for (i, op) in operations.iter().enumerate() {
        // 既にグループ化された操作はスキップ
        if operation_groups.iter().any(|(idx, _)| *idx == i) {
            continue;
        }
        
        // グループ未割り当ての操作を直接グループ化
        let group_idx = determine_operation_group(op, pattern_count);
        if group_idx >= 0 {
            // 操作を複製して変数名を置換
            let mut modified_op = op.clone();
            
            // ID置換
            if let Some(id) = op.get("id").and_then(|v| v.as_str()) {
                if id.starts_with("__temp") {
                    if let Some(var_name) = temp_id_mapping.get(id) {
                        if let Some(obj) = modified_op.as_object_mut() {
                            obj.insert("original_id".to_string(), serde_json::Value::String(id.to_string()));
                            obj.insert("id".to_string(), serde_json::Value::String(var_name.clone()));
                        }
                    }
                }
            }
            
            // from置換
            if let Some(from) = op.get("from").and_then(|v| v.as_str()) {
                if from.starts_with("__temp") {
                    if let Some(var_name) = temp_id_mapping.get(from) {
                        if let Some(obj) = modified_op.as_object_mut() {
                            obj.insert("original_from".to_string(), serde_json::Value::String(from.to_string()));
                            obj.insert("from".to_string(), serde_json::Value::String(var_name.clone()));
                        }
                    }
                }
            }
            
            // to置換
            if let Some(to) = op.get("to").and_then(|v| v.as_str()) {
                if to.starts_with("__temp") {
                    if let Some(var_name) = temp_id_mapping.get(to) {
                        if let Some(obj) = modified_op.as_object_mut() {
                            obj.insert("original_to".to_string(), serde_json::Value::String(to.to_string()));
                            obj.insert("to".to_string(), serde_json::Value::String(var_name.clone()));
                        }
                    }
                }
            }
            
            group_map.entry(group_idx).or_insert_with(Vec::new).push(modified_op);
        }
    }
    
    // グループとメソッドパターンを対応付け
    for group_id in 0..pattern_count {
        if let Some(ops) = group_map.get(&group_id) {
            if !ops.is_empty() && (group_id as usize) < method_patterns.len() {
                result.push((format!("call{}", group_id + 1), ops.clone()));
            }
        }
    }
    
    result
}

// 操作がどのグループに属するかを決定する関数
fn determine_operation_group(op: &serde_json::Value, pattern_count: i32) -> i32 {
    // ID、from、toフィールドを確認
    if let Some(edit_type) = op.get("editType").and_then(|v| v.as_str()) {
        // エッジ操作の特殊処理: fromとtoの両方が__tempで始まる場合、小さい方のIDに基づいてグループ化
        if edit_type == "addEdge" {
            if let (Some(from), Some(to)) = (
                op.get("from").and_then(|v| v.as_str()),
                op.get("to").and_then(|v| v.as_str())
            ) {
                if from.starts_with("__temp") && to.starts_with("__temp") {
                    // 両方がtemp IDの場合、小さい方の数値を使用
                    let from_num = extract_temp_id_number(from).unwrap_or(0);
                    let to_num = extract_temp_id_number(to).unwrap_or(0);
                    let min_num = std::cmp::min(from_num, to_num);
                    
                    // グループIDを計算
                    return ((min_num - 1) / 2) % pattern_count;
                }
                else if from.starts_with("__temp") {
                    // fromだけが__tempの場合
                    if let Some(id_num) = extract_temp_id_number(from) {
                        return ((id_num - 1) / 2) % pattern_count;
                    }
                }
                else if to.starts_with("__temp") {
                    // toだけが__tempの場合
                    if let Some(id_num) = extract_temp_id_number(to) {
                        return ((id_num - 1) / 2) % pattern_count;
                    }
                }
                else if from == "main-new1" {
                    // main-new1はthisを表す特殊ケース
                    if to.starts_with("__temp") {
                        if let Some(id_num) = extract_temp_id_number(to) {
                            return ((id_num - 1) / 2) % pattern_count;
                        }
                    }
                }
            }
        }
        
        // 通常のIDベースのグループ化
        if let Some(id) = op.get("id").and_then(|v| v.as_str()) {
            if id.starts_with("__temp") {
                if let Some(id_num) = extract_temp_id_number(id) {
                    return ((id_num - 1) / 2) % pattern_count;
                }
            }
        }
    }
    
    // グループを特定できない場合は-1を返す
    -1
}

// 操作を均等に分割する
fn split_operations_evenly(operations: &[serde_json::Value], method_patterns: &[String]) -> Vec<(String, Vec<serde_json::Value>)> {
    let mut result = Vec::new();
    let pattern_count = method_patterns.len();
    
    if pattern_count == 0 || operations.is_empty() {
        return result;
    }
    
    let ops_per_pattern = operations.len() / pattern_count;
    
    for (i, pattern) in method_patterns.iter().enumerate() {
        let start = i * ops_per_pattern;
        if start < operations.len() {
            let end = std::cmp::min((i + 1) * ops_per_pattern, operations.len());
            let group_ops = operations[start..end].to_vec();
            result.push((pattern.clone(), group_ops));
        }
    }
    
    result
}

// グループ化の前処理として、操作間の依存関係を分析する
fn preprocess_operations(operations: &[serde_json::Value]) -> Vec<(usize, i32)> {
    let mut op_groups = Vec::new();
    
    for (i, op) in operations.iter().enumerate() {
        if let Some(edit_type) = op.get("editType").and_then(|v| v.as_str()) {
            if edit_type == "addEdge" {
                if let (Some(from), Some(to), Some(label)) = (
                    op.get("from").and_then(|v| v.as_str()),
                    op.get("to").and_then(|v| v.as_str()),
                    op.get("label").and_then(|v| v.as_str()),
                ) {
                    if from.starts_with("__temp") && to.starts_with("__temp") {
                        if let (Some(from_num), Some(to_num)) = (
                            extract_temp_id_number(from),
                            extract_temp_id_number(to),
                        ) {
                            // next 接続は特別扱い - toのグループに割り当て
                            if label == "next" {
                                let group_id = (to_num - 1) / 2;
                                op_groups.push((i, group_id));
                            } else {
                                // その他の接続はfromのグループに割り当て
                                let group_id = (from_num - 1) / 2;
                                op_groups.push((i, group_id));
                            }
                        }
                    }
                    // main-new1 は特殊処理 - thisを表すため、対象のグループに割り当て
                    else if from == "main-new1" {
                        if to.starts_with("__temp") {
                            if let Some(to_num) = extract_temp_id_number(to) {
                                let group_id = (to_num - 1) / 2;
                                op_groups.push((i, group_id));
                            }
                        }
                    }
                    // 他のエッジ操作
                }
            } else if edit_type == "addNode" {
                // ノード追加操作も対応するグループに割り当て
                if let Some(id) = op.get("id").and_then(|v| v.as_str()) {
                    if id.starts_with("__temp") {
                        if let Some(id_num) = extract_temp_id_number(id) {
                            let group_id = (id_num - 1) / 2;
                            op_groups.push((i, group_id));
                        }
                    }
                }
            }
        }
    }
    
    op_groups
}

// サーバー実行関数
async fn run_server() {
    // CORSの設定
    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(&[Method::POST, Method::GET, Method::OPTIONS])
        .allow_headers(["Content-Type"]);

    // JSONリクエストを処理するルート
    let synthesize = warp::post()
        .and(warp::path("synthesize"))
        .and(warp::body::json())
        .map(|request: SynthesisRequest| {
            // リクエスト内容をログに出力
            println!("Received request with {} operations and {} lines of code", 
                     request.operations.len(), request.code_lines.len());
            
            // コードの合成を実行
            let synthesized_code = generate_code(&request.operations, &request.code_lines);
            
            println!("Sending response with synthesized code");
            
            // レスポンスを作成
            let response = SynthesisResponse {
                code: synthesized_code,
            };
            warp::reply::json(&response)
        });

    // 比較エンドポイント
    let compare_ast = warp::post()
        .and(warp::path("compare"))
        .and(warp::body::json())
        .map(|request: SynthesisRequest| {
            println!("Comparing AST with {} operations", request.operations.len());
            let (_, common, differing) = parse_operations(&request.operations).unwrap();
            let response = ComparisonResponse {
                common: common.into_iter().map(|stmt| format!("{:?}", stmt)).collect(),
                differing: differing.into_iter().map(|stmt| format!("{:?}", stmt)).collect(),
            };
            warp::reply::json(&response)
        });

    // ルートを結合して、CORSを適用
    let routes = synthesize.or(compare_ast).with(cors);

    // サーバーを起動
    println!("refsyn サーバー起動中: http://localhost:3030");
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

#[tokio::main]
async fn main() {
    println!("リファレンス合成サーバーを起動します...");
    run_server().await;
}

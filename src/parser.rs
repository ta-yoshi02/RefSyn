//! Kanon operations JSON → AST(Program)

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use crate::ast::{Expr, Lhs, Program, Stmt};

// ステートメントテンプレートの列挙型
#[derive(Clone)]
pub enum StmtTemplate {
    Original(Stmt),
    VarDecl {
        name_hole: String,
        expr_hole: String,
    },
    Assign {
        lhs_hole: String,
        expr_hole: String,
    },
}

/// 差分情報を保持するための構造体
#[derive(Clone, Debug)]
pub struct HoleInfo {
    pub name: String,        // ホールの名前 (例: "Hole1")
    pub position: String,    // 位置情報 (例: "var_name", "var_value", "lhs", "expr")
    pub values: Vec<String>, // 各ブロックでの実際の値
}

/// ============ Kanon Operation ============
/// 受信 JSON をそのまま Deserialize して使う
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    pub edit_type: String,

    // どの editType でも出現し得るキーを全部 optional で持つ
    pub id: Option<String>,
    pub label: Option<String>,
    pub is_literal: Option<bool>,
    pub r#type: Option<String>,

    pub from: Option<String>,
    pub to: Option<String>,
    pub old_to: Option<String>,
    pub new_to: Option<String>,

    // AST比較用のフィールド
    pub ast_node: Option<String>, // ASTノード情報
}

// Value から Operation に変換する関数
fn get_operation(val: &Value) -> Option<Operation> {
    serde_json::from_value(val.clone()).ok()
}

/// Kanon の operations 配列を AST へ
pub fn parse_operations(ops: &[Value]) -> anyhow::Result<(Program, Vec<Stmt>, Vec<Stmt>)> {
    // Kanon で生成された temp id → 変数名 or Lhs のマップ
    let mut node_map: HashMap<String, String> = HashMap::new();
    let mut stmts = Vec::<Stmt>::new();
    let mut var_counter = 0usize;

    // メソッド呼び出し別の変数マップを管理
    let mut method_node_maps: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut method_var_counters: HashMap<String, usize> = HashMap::new();
    
    // 最後に使用されたオブジェクトの参照をメソッド別に追跡する
    let mut last_object_refs: HashMap<String, String> = HashMap::new();

    // helper closure - メソッド別の変数名生成
    let mut fresh = |method_name: Option<&String>| {
        if let Some(method) = method_name {
            let counter = method_var_counters.entry(method.clone()).or_insert(0);
            let name = format!("v{}", *counter);
            *counter += 1;
            name
        } else {
            let name = format!("v{var_counter}");
            var_counter += 1;
            name
        }
    };

    let mut common_stmts = Vec::new();
    let mut differing_stmts = Vec::new();

    // メソッド呼び出し情報を追跡
    let mut current_method: Option<String> = None;
    let mut method_statements: HashMap<String, Vec<Stmt>> = HashMap::new();
    let mut current_method_params: Vec<Expr> = Vec::new();
    let mut method_obj: Option<String> = None;

    // ノードとエッジの操作をグループ化して解析
    let mut i = 0;
    while i < ops.len() {
        // get_operationを直接呼び出し、Valueを直接処理
        if let Some(op) = get_operation(&ops[i]) {
            match op.edit_type.as_str() {
                "methodCall" => {
                    if let Some(method_name) = op.label {
                        // メソッド呼び出しの開始
                        current_method = Some(method_name.clone());
                        
                        // 新しいメソッド呼び出しの場合、ノードマップを初期化
                        if !method_node_maps.contains_key(&method_name) {
                            method_node_maps.insert(method_name.clone(), HashMap::new());
                        }
                        
                        // メソッド呼び出しのオブジェクトを特定
                        if let Some(obj_id) = op.from {
                            method_obj = Some(obj_id.clone());
                            
                            // このオブジェクトを最後に使用されたオブジェクトとして記録
                            last_object_refs.insert(method_name.clone(), obj_id.clone());
                        }
                        
                        // 引数があれば処理
                        if let Some(arg_id) = op.to {
                            if let Some(arg_val) = op.id {
                                current_method_params.push(parse_literal(&arg_val));
                            } else {
                                // 引数としてオブジェクト参照が渡されている場合
                                let method_map = method_node_maps.get_mut(&method_name).unwrap();
                                if method_map.contains_key(&arg_id) {
                                    let var_name = method_map.get(&arg_id).unwrap().clone();
                                    current_method_params.push(Expr::Var(var_name));
                                } else if node_map.contains_key(&arg_id) {
                                    // グローバルマップから変数名を検索
                                    let var_name = node_map.get(&arg_id).unwrap().clone();
                                    method_map.insert(arg_id.clone(), var_name.clone());
                                    current_method_params.push(Expr::Var(var_name));
                                } else {
                                    // まだマップに存在しない場合は新しい変数名を生成
                                    let var_name = fresh(Some(&method_name));
                                    method_map.insert(arg_id.clone(), var_name.clone());
                                    current_method_params.push(Expr::Var(var_name));
                                }
                            }
                        }

                        // メソッド呼び出しステートメントを生成
                        if let Some(obj) = &method_obj {
                            let obj_lhs = if obj == "main-new1" {
                                Lhs::Var("this".into())
                            } else {
                                let method_map = method_node_maps.get(&method_name).unwrap();
                                if method_map.contains_key(obj) {
                                    Lhs::Var(method_map.get(obj).unwrap().clone())
                                } else if node_map.contains_key(obj) {
                                    // グローバルマップから変数名を検索
                                    let var_name = node_map.get(obj).unwrap().clone();
                                    Lhs::Var(var_name)
                                } else {
                                    let var_name = fresh(Some(&method_name));
                                    let method_map = method_node_maps.get_mut(&method_name).unwrap();
                                    method_map.insert(obj.clone(), var_name.clone());
                                    Lhs::Var(var_name)
                                }
                            };

                            let method_call = Expr::MethodCall(
                                Box::new(obj_lhs.clone()),
                                method_name.clone(),
                                current_method_params.clone()
                            );
                            
                            stmts.push(Stmt::Expr(method_call));
                            
                            // メソッド呼び出しのステートメントを追跡
                            if !method_statements.contains_key(&method_name) {
                                method_statements.insert(method_name.clone(), Vec::new());
                            }
                        }
                        
                        // パラメータリストをリセット
                        current_method_params = Vec::new();
                    }
                }
                
                "addNode" => {
                    let id = op.id.expect("addNode.id");
                    let label = op.label.expect("addNode.label");
                    let is_lit = op.is_literal.unwrap_or(false);

                    // 現在のメソッド呼び出しのコンテキストに応じて処理
                    if let Some(method_name) = &current_method {
                        let method_map = method_node_maps.get_mut(method_name).unwrap();
                        
                        if is_lit {
                            // リテラルノードはメソッド引数または内部変数として処理
                            let var_name = method_map.entry(id.clone()).or_insert_with(|| fresh(Some(method_name))).clone();
                            
                            // リテラル値を解析
                            let expr = parse_literal(&label);
                            
                            // 変数宣言を生成
                            let var_decl = Stmt::VarDecl { name: var_name.clone(), expr };
                            stmts.push(var_decl.clone());
                            
                            // メソッド固有のステートメントリストに追加
                            if let Some(method_stmts) = method_statements.get_mut(method_name) {
                                method_stmts.push(var_decl);
                            }
                        } else {
                            // オブジェクトノードはメソッド内で生成されるオブジェクト
                            let var_name = method_map.entry(id.clone()).or_insert_with(|| fresh(Some(method_name))).clone();
                            
                            // 新しいオブジェクト生成
                            let expr = Expr::New(label);
                            
                            // 変数宣言を生成
                            let var_decl = Stmt::VarDecl { name: var_name.clone(), expr };
                            stmts.push(var_decl.clone());
                            
                            // メソッド固有のステートメントリストに追加
                            if let Some(method_stmts) = method_statements.get_mut(method_name) {
                                method_stmts.push(var_decl);
                            }
                            
                            // このノードを最後に生成されたオブジェクトとして記録
                            last_object_refs.insert(format!("{}_last", method_name), id.clone());
                        }
                    } else {
                        // メソッド呼び出し外の通常の処理
                        let var_name = node_map.entry(id.clone()).or_insert_with(|| fresh(None)).clone();

                        let expr = if is_lit {
                            parse_literal(&label)
                        } else {
                            Expr::New(label) // label がクラス名
                        };

                        stmts.push(Stmt::VarDecl { name: var_name, expr });
                    }
                }

                "addEdge" => {
                    let from = op.from.expect("addEdge.from");
                    let to = op.to.expect("addEdge.to");
                    let prop = op.label.expect("addEdge.label");

                    // メソッド呼び出しのコンテキストに応じて処理
                    if let Some(method_name) = &current_method {
                        let method_map = method_node_maps.get_mut(method_name).unwrap();
                        
                        // thisかどうか判断
                        let lhs_obj = if from == "main-new1" {
                            Lhs::Var("this".into())
                        } else if prop == "next" && from == last_object_refs.get(&format!("{}_last", method_name)).cloned().unwrap_or_default() {
                            // 連結リストパターン: 最後に生成されたオブジェクトの次のノードを設定
                            // ネストされたオブジェクトアクセスを使用（this.next など）
                            if let Some(last_obj_id) = last_object_refs.get(method_name) {
                                if last_obj_id == "main-new1" {
                                    // thisを最初のノードとして処理
                                    Lhs::Var("this".into())
                                } else if method_map.contains_key(last_obj_id) {
                                    Lhs::ObjAccess(
                                        Box::new(Lhs::Var(method_map.get(last_obj_id).unwrap().clone())),
                                        "next".to_string()
                                    )
                                } else {
                                    // fromが知られていない場合、新しい変数を生成
                                    let var_name = fresh(Some(method_name));
                                    method_map.insert(from.clone(), var_name.clone());
                                    Lhs::Var(var_name)
                                }
                            } else {
                                // fromが知られていない場合、新しい変数を生成
                                let var_name = fresh(Some(method_name));
                                method_map.insert(from.clone(), var_name.clone());
                                Lhs::Var(var_name)
                            }
                        } else {
                            // 通常のケース
                            // fromが知られていない場合、新しい変数を生成
                            if !method_map.contains_key(&from) {
                                let var_name = fresh(Some(method_name));
                                method_map.insert(from.clone(), var_name);
                            }
                            let v = method_map.get(&from).unwrap();
                            Lhs::Var(v.clone())
                        };

                        let lhs = Lhs::ObjAccess(Box::new(lhs_obj), prop.clone());

                        let rhs_expr = if to == "main-new1" {
                            Expr::This
                        } else {
                            // toが知られていない場合、新しい変数を生成
                            if !method_map.contains_key(&to) {
                                let var_name = fresh(Some(method_name));
                                method_map.insert(to.clone(), var_name);
                            }
                            let v = method_map.get(&to).unwrap();
                            Expr::Var(v.clone())
                        };

                        let assign = Stmt::Assign { lhs, expr: rhs_expr };
                        stmts.push(assign.clone());
                        
                        // メソッド固有のステートメントリストに追加
                        if let Some(method_stmts) = method_statements.get_mut(method_name) {
                            method_stmts.push(assign);
                        }
                        
                        // 最後の参照オブジェクトを更新
                        if prop == "next" {
                            last_object_refs.insert(format!("{}_last", method_name), to.clone());
                        }
                    } else {
                        // メソッド呼び出し外の通常の処理
                        let lhs_obj = if from == "main-new1" {
                            Lhs::Var("this".into())
                        } else {
                            if !node_map.contains_key(&from) {
                                node_map.insert(from.clone(), fresh(None));
                            }
                            let v = node_map.get(&from).unwrap();
                            Lhs::Var(v.clone())
                        };

                        let lhs = Lhs::ObjAccess(Box::new(lhs_obj), prop.clone());

                        let rhs_expr = if to == "main-new1" {
                            Expr::This
                        } else {
                            if !node_map.contains_key(&to) {
                                node_map.insert(to.clone(), fresh(None));
                            }
                            let v = node_map.get(&to).unwrap();
                            Expr::Var(v.clone())
                        };

                        stmts.push(Stmt::Assign { lhs, expr: rhs_expr });
                    }
                }

                "compare" => {
                    // AST比較ロジック
                    let ast_node = op.ast_node.expect("compare.ast_node");
                    if node_map.contains_key(&ast_node) {
                        common_stmts.push(Stmt::Expr(Expr::Literal(ast_node.clone())));
                    } else {
                        differing_stmts.push(Stmt::Expr(Expr::Literal(ast_node)));
                    }
                }

                // ひとまず other editType は無視
                _ => {}
            }
        }
        i += 1;
    }

    // メソッド内の処理を適切に構成
    for (method_name, method_stmts) in &method_statements {
        if !method_stmts.is_empty() {
            println!("Method '{}' has {} statements", method_name, method_stmts.len());
        }
    }

    Ok((Program { stmts }, common_stmts, differing_stmts))
}

// リテラル文字列を適切な式に変換するヘルパー関数
fn parse_literal(literal: &str) -> Expr {
    if let Ok(num) = literal.parse::<f64>() {
        // f64からi64に変換してExpr::Numを作成
        Expr::Num(num as i64)
    } else if literal == "true" || literal == "false" {
        // Boolバリアントが無いのでStringとして扱う
        Expr::Str(literal.to_string())
    } else if literal == "null" || literal == "undefined" {
        // Nullバリアントが無いのでStringとして扱う
        Expr::Str(literal.to_string())
    } else {
        // デフォルトは文字列
        Expr::Str(literal.to_string())
    }
}

// ASTブロック間の共通部分と差分部分を抽出
pub fn compare_ast_blocks(blocks: &[Vec<Stmt>]) -> String {
    if blocks.is_empty() || blocks.len() < 2 {
        return String::new();
    }

    let mut template_code = String::new();
    let mut holes = Vec::<HoleInfo>::new();
    let mut hole_counter = 1;

    // 最初のブロックを基準に比較
    let first_block = &blocks[0];
    
    for (stmt_idx, stmt) in first_block.iter().enumerate() {
        // 他のすべてのブロックにこの位置のステートメントが存在するか確認
        let mut all_blocks_have_stmt = true;
        for block in blocks.iter().skip(1) {
            if stmt_idx >= block.len() {
                all_blocks_have_stmt = false;
                break;
            }
        }
        
        if !all_blocks_have_stmt {
            continue;
        }
        
        // 現在の位置のすべてのステートメントを収集
        let stmts_at_position: Vec<&Stmt> = blocks.iter()
            .filter_map(|block| block.get(stmt_idx))
            .collect();
        
        // ステートメントの種類が同じか確認
        let mut same_kind = true;
        let base_kind = stmt_kind(stmt);
        for s in stmts_at_position.iter().skip(1) {
            if stmt_kind(s) != base_kind {
                same_kind = false;
                break;
            }
        }
        
        if !same_kind {
            // 異なる種類のステートメントが混在する場合
            template_code.push_str(&format!("// ステートメント構造が異なります (位置: {})\n", stmt_idx));
            continue;
        }
        
        // 同じ種類のステートメントを比較して共通部分と差分を特定
        match stmt {
            Stmt::VarDecl { name, expr } => {
                // 変数名とその値を比較
                let mut name_values: Vec<String> = vec![name.clone()];
                let mut expr_values: Vec<String> = vec![expr_to_str(expr)];
                
                for other_stmt in stmts_at_position.iter().skip(1) {
                    if let Stmt::VarDecl { name: n, expr: e } = other_stmt {
                        name_values.push(n.clone());
                        expr_values.push(expr_to_str(e));
                    }
                }
                
                // 変数名が異なるかチェック
                let name_differs = name_values.iter().any(|n| n != &name_values[0]);
                let name_hole = if name_differs {
                    let hole_name = format!("Hole{}", hole_counter);
                    holes.push(HoleInfo {
                        name: hole_name.clone(),
                        position: "var_name".to_string(),
                        values: name_values.clone(),
                    });
                    hole_counter += 1;
                    hole_name
                } else {
                    name_values[0].clone()
                };
                
                // 値が異なるかチェックする変数を削除（常にホール化するため）
                let expr_type = match expr {
                    Expr::New(cls) => format!("new {}", cls),
                    Expr::Num(_) | Expr::Str(_) => {
                        // リテラル値は常に違いを検出する対象として扱う
                        let hole_name = format!("Hole{}", hole_counter);
                        holes.push(HoleInfo {
                            name: hole_name.clone(),
                            position: "var_value".to_string(),
                            values: expr_values.clone(),
                        });
                        hole_counter += 1;
                        hole_name
                    },
                    _ => format!("{:?}", expr),
                };
                
                template_code.push_str(&format!("var {} = {};\n", name_hole, expr_type));
            },
            
            Stmt::Assign { lhs, expr } => {
                // 左辺と右辺を比較
                let lhs_str = lhs_to_str(lhs);
                let mut lhs_values: Vec<String> = vec![lhs_str.clone()];
                let mut expr_values: Vec<String> = vec![expr_to_str(expr)];
                
                for other_stmt in stmts_at_position.iter().skip(1) {
                    if let Stmt::Assign { lhs: l, expr: e } = other_stmt {
                        lhs_values.push(lhs_to_str(l));
                        expr_values.push(expr_to_str(e));
                    }
                }
                
                // 左辺が異なるかチェック
                let lhs_differs = lhs_values.iter().any(|l| l != &lhs_values[0]);
                let lhs_template = if lhs_differs {
                    let hole_name = format!("Hole{}", hole_counter);
                    holes.push(HoleInfo {
                        name: hole_name.clone(),
                        position: "lhs".to_string(),
                        values: lhs_values.clone(),
                    });
                    hole_counter += 1;
                    hole_name
                } else {
                    format_lhs(lhs)
                };
                
                // 右辺が異なるかチェック
                let expr_differs = expr_values.iter().any(|e| e != &expr_values[0]);
                let expr_template = if expr_differs {
                    let hole_name = format!("Hole{}", hole_counter);
                    holes.push(HoleInfo {
                        name: hole_name.clone(),
                        position: "expr".to_string(),
                        values: expr_values.clone(),
                    });
                    hole_counter += 1;
                    hole_name
                } else {
                    format_expr(expr)
                };
                
                template_code.push_str(&format!("{} = {};\n", lhs_template, expr_template));
            },
            
            _ => {
                // その他のステートメントはそのまま出力
                template_code.push_str(&format!("{:?};\n", stmt));
            }
        }
    }
    
    // ホール情報を出力
    if !holes.is_empty() {
        template_code.push_str("\n// ホール情報:\n");
        for hole in &holes {
            template_code.push_str(&format!("// {} ({}): {:?}\n", 
                                          hole.name, hole.position, hole.values));
        }
    }
    
    template_code
}

// ステートメントの種類を返す補助関数
fn stmt_kind(stmt: &Stmt) -> &'static str {
    match stmt {
        Stmt::VarDecl { .. } => "VarDecl",
        Stmt::Assign { .. } => "Assign",
        Stmt::Expr(_) => "Expr",
    }
}

// 左辺値を整形する補助関数
fn format_lhs(lhs: &Lhs) -> String {
    match lhs {
        Lhs::Var(name) => name.clone(),
        Lhs::ObjAccess(obj, prop) => {
            match &**obj {
                Lhs::Var(name) => format!("{}.{}", name, prop),
                Lhs::ObjAccess(inner_obj, inner_prop) => {
                    // ネストされたオブジェクトアクセスをサポート
                    let inner_fmt = format_lhs(&Lhs::ObjAccess(inner_obj.clone(), inner_prop.clone()));
                    format!("{}.{}", inner_fmt, prop)
                },
                _ => format!("<complex>.{}", prop),
            }
        },
        _ => format!("{:?}", lhs),
    }
}

// 式を整形する補助関数
fn format_expr(expr: &Expr) -> String {
    match expr {
        Expr::Var(name) => name.clone(),
        Expr::Num(n) => n.to_string(),
        Expr::Str(s) => format!("\"{}\"", s),
        Expr::New(cls) => format!("new {}", cls),
        Expr::This => "this".to_string(),
        Expr::Literal(lit) => lit.clone(),
        _ => format!("{:?}", expr),
    }
}

// 共通テンプレート抽出のためのパブリックインターフェース
pub fn extract_common_template(blocks: &[Vec<Stmt>]) -> String {
    compare_ast_blocks(blocks)
}

// 簡易的な補助関数
fn expr_to_str(e: &Expr) -> String {
    match e {
        Expr::Var(v) => v.clone(),
        Expr::Num(n) => n.to_string(),
        Expr::Str(s) => s.clone(),
        Expr::New(n) => format!("new_{}", n),
        Expr::This => "this".into(),
        Expr::Literal(l) => l.clone(),
        _ => "expr_unknown".into(),
    }
}

fn lhs_to_str(lhs: &Lhs) -> String {
    match lhs {
        Lhs::Var(v) => v.clone(),
        Lhs::ObjAccess(obj, prop) => {
            match &**obj {
                Lhs::Var(ov) => format!("{}.{}", ov, prop),
                Lhs::ObjAccess(inner_obj, inner_prop) => {
                    // ネストされたオブジェクトアクセスをサポート
                    let inner_fmt = lhs_to_str(&Lhs::ObjAccess(inner_obj.clone(), inner_prop.clone()));
                    format!("{}.{}", inner_fmt, prop)
                },
                _ => format!("<complex>.{}", prop),
            }
        },
        _ => "lhs_unknown".into(),
    }
}

//! Kanon operations JSON → AST(Program)

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use crate::ast::{Expr, Lhs, Program, Stmt};

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
}

/// Kanon の operations 配列を AST へ
pub fn parse_operations(ops: &[Value]) -> anyhow::Result<Program> {
    let ops: Vec<Operation> = ops.iter().map(|v| serde_json::from_value(v.clone())).collect::<Result<_, _>>()?;

    // Kanon で生成された temp id → 変数名 or Lhs のマップ
    let mut node_map: HashMap<String, String> = HashMap::new();
    let mut stmts = Vec::<Stmt>::new();
    let mut var_counter = 0usize;

    // helper closure
    let mut fresh = || {
        let name = format!("v{var_counter}");
        var_counter += 1;
        name
    };

    for op in ops {
        match op.edit_type.as_str() {
            "addNode" => {
                let id     = op.id.expect("addNode.id");
                let label  = op.label.expect("addNode.label");
                let is_lit = op.is_literal.unwrap_or(false);

                let var_name = node_map.entry(id.clone()).or_insert_with(|| fresh()).clone();

                let expr = if is_lit {
                    // Number or string? ざっくり判定
                    if let Ok(n) = label.parse::<i64>() {
                        Expr::Num(n)
                    } else {
                        Expr::Str(label)
                    }
                } else {
                    Expr::New(label) // label がクラス名
                };

                stmts.push(Stmt::VarDecl { name: var_name, expr });
            }

            "addEdge" => {
                let from = op.from.unwrap();
                let to   = op.to.unwrap();
                let prop = op.label.unwrap();

                // "main-new1" は this に対応させる
                let lhs_obj = if from == "main-new1" {
                    Lhs::Var("this".into())
                } else {
                    let v = node_map.get(&from)
                        .unwrap_or_else(|| panic!("unknown `from` id {from}"));
                    Lhs::Var(v.clone())
                };

                let lhs = Lhs::ObjAccess(Box::new(lhs_obj), prop);

                let rhs_expr = if to == "main-new1" {
                    Expr::This
                } else {
                    let v = node_map.get(&to)
                        .unwrap_or_else(|| panic!("unknown `to` id {to}"));
                    Expr::Var(v.clone())
                };

                stmts.push(Stmt::Assign { lhs, expr: rhs_expr });
            }

            // ひとまず other editType は無視
            _ => {}
        }
    }

    Ok(Program { stmts })
}

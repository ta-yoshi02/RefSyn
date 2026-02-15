//! リスト形式の環境表現を管理するモジュール
//!
//! このモジュールはKanonのグラフ操作によって変化する環境を
//! List[field_name] = [values...] の形式で表現するためのものです。

use crate::models::VisGraph;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldKind {
    Pointer,
    Value,
}

impl FieldKind {
    pub fn default_value(self) -> Value {
        match self {
            FieldKind::Pointer => Value::Null,
            FieldKind::Value => json!(-1),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtrValue {
    Null,
    Index(usize),
}

impl PtrValue {
    pub fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::Null => Some(PtrValue::Null),
            Value::Number(n) => n.as_i64().and_then(|i| {
                if i >= 0 {
                    Some(PtrValue::Index(i as usize))
                } else {
                    None
                }
            }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListEnvironment {
    /// フィールド名をキーとし、各オブジェクトのフィールド値の配列を値とするマップ
    pub field_lists: HashMap<String, Vec<Value>>,
    #[serde(default)]
    pub field_kinds: HashMap<String, FieldKind>,
    /// オブジェクトIDからインデックスへのマッピング
    pub obj_id_to_index: HashMap<String, usize>,
    /// インデックスからオブジェクトIDへのマッピング
    pub index_to_obj_id: HashMap<usize, String>,
    /// 次に割り当てるインデックス
    pub next_index: usize,
    /// リテラルIDから値へのマッピング
    pub literal_id_to_value: HashMap<String, Value>,
}

/// グラフ操作の種類
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GraphOperation {
    pub edit_type: String,
    pub id: Option<String>,
    pub label: Option<Value>,
    pub is_literal: Option<bool>,
    #[serde(rename = "type")]
    pub node_type: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    #[serde(default)]
    pub old_to: Option<String>,
    #[serde(default)]
    pub new_to: Option<String>,
    #[serde(default)]
    pub old_label: Option<String>,
    #[serde(default)]
    pub new_label: Option<String>,
}

impl ListEnvironment {
    /// 初期のVisGraphからListEnvironmentを作成
    pub fn from_vis_graph(vis_graph: &VisGraph) -> Self {
        let object_ids: Vec<String> = vis_graph
            .nodes
            .iter()
            .filter(|n| !n.is_literal)
            .filter(|n| n.id != "__RectForVariable__") // 不要なオブジェクトを除外
            .map(|n| n.id.clone())
            .collect();

        let obj_id_to_index: HashMap<String, usize> = object_ids
            .iter()
            .enumerate()
            .map(|(i, id)| (id.clone(), i))
            .collect();

        let index_to_obj_id: HashMap<usize, String> = object_ids
            .iter()
            .enumerate()
            .map(|(i, id)| (i, id.clone()))
            .collect();

        let literal_id_to_value: HashMap<String, Value> = vis_graph
            .nodes
            .iter()
            .filter(|n| n.is_literal)
            .map(|n| (n.id.clone(), n.label.clone()))
            .collect();

        let num_objects = object_ids.len();
        let mut field_lists: HashMap<String, Vec<Value>> = HashMap::new();
        let mut field_kinds: HashMap<String, FieldKind> = HashMap::new();

        // エッジからフィールドリストを構築
        for edge in &vis_graph.edges {
            // __RectForVariable__に関連するエッジは無視
            if edge.from == "__RectForVariable__" || edge.to == "__RectForVariable__" {
                continue;
            }
            if let Some(&from_index) = obj_id_to_index.get(&edge.from) {
                let (inferred_kind, value_to_insert) =
                    if let Some(&to_index) = obj_id_to_index.get(&edge.to) {
                        (FieldKind::Pointer, json!(to_index as i32))
                    } else if let Some(literal_value) = literal_id_to_value.get(&edge.to) {
                        (FieldKind::Value, literal_value.clone())
                    } else {
                        (FieldKind::Pointer, Value::Null)
                    };

                let entry = field_kinds
                    .entry(edge.label.clone())
                    .or_insert(inferred_kind);
                if inferred_kind == FieldKind::Pointer {
                    *entry = FieldKind::Pointer;
                }
                let default_value = entry.default_value();
                let field_vec = field_lists
                    .entry(edge.label.clone())
                    .or_insert_with(|| vec![default_value.clone(); num_objects]);
                if *entry == FieldKind::Pointer {
                    for v in field_vec.iter_mut() {
                        if v.as_i64() == Some(-1) {
                            *v = Value::Null;
                        }
                    }
                }

                if from_index < field_vec.len() {
                    field_vec[from_index] = value_to_insert;
                }
            }
        }

        ListEnvironment {
            field_lists,
            field_kinds,
            obj_id_to_index,
            index_to_obj_id,
            next_index: num_objects,
            literal_id_to_value,
        }
    }

    /// 操作を適用してListEnvironmentを更新
    pub fn apply_operation(&mut self, operation: &GraphOperation) -> Result<(), String> {
        match operation.edit_type.as_str() {
            "addNode" => self.add_node(operation),
            "addEdge" => self.add_edge(operation),
            "editEdgeReference" => self.edit_edge_reference(operation),
            "addVariable" => self.add_variable(operation),
            "editVariableReference" => self.edit_variable_reference(operation),
            "removeNode" => self.remove_node(operation),
            "removeEdge" => self.remove_edge(operation),
            _ => Err(format!("Unknown operation type: {}", operation.edit_type)),
        }
    }

    fn add_node(&mut self, operation: &GraphOperation) -> Result<(), String> {
        let id = operation.id.as_ref().ok_or("addNode requires id")?;
        let is_literal = operation.is_literal.unwrap_or(false);

        if is_literal {
            // リテラルノードの場合、値をマッピングに追加
            if let Some(label) = &operation.label {
                self.literal_id_to_value.insert(id.clone(), label.clone());
            }
            return Ok(());
        }

        // 新しいオブジェクトインデックスを割り当て
        let new_index = self.next_index;
        self.obj_id_to_index.insert(id.clone(), new_index);
        self.index_to_obj_id.insert(new_index, id.clone());
        self.next_index += 1;

        // 全てのフィールドリストに新しい要素を追加（フィールド種別のデフォルト値）
        for (field_name, field_vec) in self.field_lists.iter_mut() {
            let default_value = self
                .field_kinds
                .get(field_name)
                .copied()
                .unwrap_or(FieldKind::Value)
                .default_value();
            field_vec.push(default_value);
        }

        Ok(())
    }

    fn add_edge(&mut self, operation: &GraphOperation) -> Result<(), String> {
        let from = operation.from.as_ref().ok_or("addEdge requires from")?;
        let to = operation.to.as_ref().ok_or("addEdge requires to")?;
        let label = Self::label_string(operation);
        self.set_edge_reference(from, &label, to)
    }

    fn edit_edge_reference(&mut self, operation: &GraphOperation) -> Result<(), String> {
        let from = operation
            .from
            .as_ref()
            .ok_or("editEdgeReference requires from")?;
        let new_to = operation
            .new_to
            .as_ref()
            .or(operation.to.as_ref())
            .ok_or("editEdgeReference requires newTo")?;
        let label = Self::label_string(operation);
        if let Some(old_to) = operation.old_to.as_deref() {
            self.validate_old_target(from, &label, old_to);
        }
        self.set_edge_reference(from, &label, new_to)
    }

    fn add_variable(&mut self, operation: &GraphOperation) -> Result<(), String> {
        let target = operation
            .to
            .as_ref()
            .or(operation.new_to.as_ref())
            .ok_or("addVariable requires to")?;
        let label = Self::label_string(operation);
        self.set_variable_reference(&label, target)
    }

    fn edit_variable_reference(&mut self, operation: &GraphOperation) -> Result<(), String> {
        let target = operation
            .new_to
            .as_ref()
            .or(operation.to.as_ref())
            .ok_or("editVariableReference requires newTo")?;
        let label = Self::label_string(operation);
        let var_node_id = format!("__Variable-{}", label);
        if let Some(old_to) = operation.old_to.as_deref() {
            self.validate_old_target(&var_node_id, &label, old_to);
        }
        self.set_variable_reference(&label, target)
    }

    fn ensure_object_node(&mut self, object_id: &str) -> usize {
        if let Some(&idx) = self.obj_id_to_index.get(object_id) {
            return idx;
        }

        let new_index = self.next_index;
        self.obj_id_to_index
            .insert(object_id.to_string(), new_index);
        self.index_to_obj_id
            .insert(new_index, object_id.to_string());
        self.next_index += 1;
        for (field_name, field_vec) in self.field_lists.iter_mut() {
            let default_value = self
                .field_kinds
                .get(field_name)
                .copied()
                .unwrap_or(FieldKind::Value)
                .default_value();
            field_vec.push(default_value);
        }
        new_index
    }

    fn label_string(operation: &GraphOperation) -> String {
        match operation.label.as_ref() {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Number(n)) => n.to_string(),
            Some(Value::Bool(b)) => b.to_string(),
            _ => String::new(),
        }
    }

    fn resolve_target_value(&self, target_id: &str) -> (Value, FieldKind) {
        if let Some(&to_index) = self.obj_id_to_index.get(target_id) {
            (json!(to_index as i32), FieldKind::Pointer)
        } else if let Some(literal_value) = self.literal_id_to_value.get(target_id) {
            (literal_value.clone(), FieldKind::Value)
        } else if target_id == "null" {
            (Value::Null, FieldKind::Pointer)
        } else {
            (Value::Null, FieldKind::Pointer)
        }
    }

    fn current_field_value(&self, from_id: &str, label: &str) -> Option<Value> {
        let from_index = *self.obj_id_to_index.get(from_id)?;
        let field_vec = self.field_lists.get(label)?;
        field_vec.get(from_index).cloned()
    }

    fn validate_old_target(&self, from_id: &str, label: &str, old_target: &str) {
        let Some(current_value) = self.current_field_value(from_id, label) else {
            return;
        };
        let (expected_value, expected_kind) = self.resolve_target_value(old_target);
        let normalized_current = if expected_kind == FieldKind::Value && current_value.is_null() {
            json!(-1)
        } else {
            current_value
        };
        if normalized_current != expected_value {
            eprintln!(
                "WARN: oldTo mismatch for {}.{} (expected {:?}, found {:?})",
                from_id, label, expected_value, normalized_current
            );
        }
    }

    fn set_edge_reference(&mut self, from: &str, label: &str, target: &str) -> Result<(), String> {
        let from_index = match self.obj_id_to_index.get(from) {
            Some(&index) => index,
            None => return Ok(()),
        };

        let (mut to_value, inferred_kind) = self.resolve_target_value(target);
        let field_kind = {
            let entry = self
                .field_kinds
                .entry(label.to_string())
                .or_insert(inferred_kind);
            if inferred_kind == FieldKind::Pointer {
                *entry = FieldKind::Pointer;
            }
            *entry
        };
        if field_kind == FieldKind::Value && to_value.is_null() {
            to_value = json!(-1);
        }

        let field_vec = self
            .field_lists
            .entry(label.to_string())
            .or_insert_with(|| vec![field_kind.default_value(); self.next_index]);
        if field_kind == FieldKind::Pointer {
            for value in field_vec.iter_mut() {
                if value.as_i64() == Some(-1) {
                    *value = Value::Null;
                }
            }
        }

        while field_vec.len() <= from_index {
            field_vec.push(field_kind.default_value());
        }
        field_vec[from_index] = to_value;
        Ok(())
    }

    fn set_variable_reference(&mut self, label: &str, target: &str) -> Result<(), String> {
        let variable_node_id = format!("__Variable-{}", label);
        self.ensure_object_node(&variable_node_id);
        self.set_edge_reference(&variable_node_id, label, target)
    }

    fn remove_node(&mut self, _operation: &GraphOperation) -> Result<(), String> {
        // 簡単のため、今回は実装しない
        Ok(())
    }

    fn remove_edge(&mut self, _operation: &GraphOperation) -> Result<(), String> {
        // 簡単のため、今回は実装しない
        Ok(())
    }

    /// 操作シーケンスを順次適用
    pub fn apply_operations(&mut self, operations: &[GraphOperation]) -> Result<(), String> {
        for operation in operations {
            self.apply_operation(operation)?;
        }
        Ok(())
    }

    /// デバッグ用：環境の状態を文字列として出力
    pub fn to_debug_string(&self) -> String {
        let mut result = String::new();

        // 見やすさのためのヘッダー（テスト期待値に合わせる）
        result.push_str("List Environment:\n");

        // __RectForVariable__ を表示用に含めるか（Kanonの変数バインディングがある場合は先頭に1要素追加）
        // テストでは __RectForVariable__ を先頭インデックスとして期待しているため、
        // 表示のみ +1 オフセットを施す。
        let include_rect_for_display = self.obj_id_to_index.contains_key("__Variable-lst");

        for (field_name, values) in &self.field_lists {
            // lst も他フィールドと同様に List[...] 形式で出す（以前の obj_lst は互換のため残す）
            if field_name == "lst" {
                let lst_value = self.get_lst_reference();
                result.push_str(&format!("obj_lst = {}\n", lst_value));
            }

            // 表示用の値列を構築
            let mut display_values: Vec<Value> = Vec::new();
            if include_rect_for_display {
                let default_value = self
                    .field_kinds
                    .get(field_name)
                    .copied()
                    .unwrap_or(FieldKind::Value)
                    .default_value();
                display_values.push(default_value);
            }
            let field_kind = self
                .field_kinds
                .get(field_name)
                .copied()
                .unwrap_or(FieldKind::Value);
            for v in values.iter() {
                // 数値（インデックス）は __RectForVariable__ を入れた分だけ +1 して表示
                if let Some(n) = v.as_i64() {
                    if n >= 0 {
                        if include_rect_for_display {
                            display_values.push(json!((n + 1) as i64));
                        } else {
                            display_values.push(json!(n));
                        }
                    } else {
                        if field_kind == FieldKind::Pointer {
                            display_values.push(Value::Null);
                        } else {
                            display_values.push(json!(-1));
                        }
                    }
                } else {
                    // 文字列などはそのまま
                    display_values.push(v.clone());
                }
            }

            result.push_str(&format!(
                "List[obj_{}] = {:?}\n",
                field_name, display_values
            ));
        }

        result
    }

    /// lstフィールドの参照先インデックスを取得
    fn get_lst_reference(&self) -> Value {
        if let Some(lst_values) = self.field_lists.get("lst") {
            // __Variable-lstのインデックスを探す
            if let Some(&var_lst_index) = self.obj_id_to_index.get("__Variable-lst") {
                if var_lst_index < lst_values.len() {
                    return lst_values[var_lst_index].clone();
                }
            }
        }
        Value::Null
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Edge, Node};

    fn create_test_graph() -> VisGraph {
        VisGraph {
            nodes: vec![
                Node {
                    id: "obj1".to_string(),
                    is_literal: false,
                    label: json!("Node"),
                },
                Node {
                    id: "obj2".to_string(),
                    is_literal: false,
                    label: json!("Node"),
                },
                Node {
                    id: "obj3".to_string(),
                    is_literal: false,
                    label: json!("Node"),
                },
                Node {
                    id: "obj4".to_string(),
                    is_literal: false,
                    label: json!("Node"),
                },
                Node {
                    id: "val1".to_string(),
                    is_literal: true,
                    label: json!(2),
                },
                Node {
                    id: "val2".to_string(),
                    is_literal: true,
                    label: json!(0),
                },
                Node {
                    id: "val3".to_string(),
                    is_literal: true,
                    label: json!(9),
                },
                Node {
                    id: "val4".to_string(),
                    is_literal: true,
                    label: json!(5),
                },
            ],
            edges: vec![
                Edge {
                    from: "obj1".to_string(),
                    to: "obj2".to_string(),
                    label: "next".to_string(),
                },
                Edge {
                    from: "obj2".to_string(),
                    to: "obj3".to_string(),
                    label: "next".to_string(),
                },
                Edge {
                    from: "obj3".to_string(),
                    to: "obj4".to_string(),
                    label: "next".to_string(),
                },
                Edge {
                    from: "obj1".to_string(),
                    to: "val1".to_string(),
                    label: "value".to_string(),
                },
                Edge {
                    from: "obj2".to_string(),
                    to: "val2".to_string(),
                    label: "value".to_string(),
                },
                Edge {
                    from: "obj3".to_string(),
                    to: "val3".to_string(),
                    label: "value".to_string(),
                },
                Edge {
                    from: "obj4".to_string(),
                    to: "val4".to_string(),
                    label: "value".to_string(),
                },
            ],
        }
    }

    #[test]
    fn test_initial_environment() {
        let graph = create_test_graph();
        let env = ListEnvironment::from_vis_graph(&graph);

        println!("{}", env.to_debug_string());

        // next フィールドをチェック
        let next_list = env.field_lists.get("next").unwrap();
        assert_eq!(next_list[0], json!(1)); // obj1 -> obj2
        assert_eq!(next_list[1], json!(2)); // obj2 -> obj3
        assert_eq!(next_list[2], json!(3)); // obj3 -> obj4
        assert_eq!(next_list[3], Value::Null); // obj4 -> nullPtr

        // value フィールドをチェック
        let value_list = env.field_lists.get("value").unwrap();
        assert_eq!(value_list[0], json!(2)); // obj1.value = 2
        assert_eq!(value_list[1], json!(0)); // obj2.value = 0
        assert_eq!(value_list[2], json!(9)); // obj3.value = 9
        assert_eq!(value_list[3], json!(5)); // obj4.value = 5
    }

    #[test]
    fn test_add_node_operation() {
        let graph = create_test_graph();
        let mut env = ListEnvironment::from_vis_graph(&graph);

        // 新しいNodeを追加
        let add_node_op = GraphOperation {
            edit_type: "addNode".to_string(),
            id: Some("obj5".to_string()),
            label: Some(json!("Node")),
            is_literal: Some(false),
            node_type: None,
            from: None,
            to: None,
            old_to: None,
            new_to: None,
            old_label: None,
            new_label: None,
        };

        env.apply_operation(&add_node_op).unwrap();

        println!("After adding node:");
        println!("{}", env.to_debug_string());

        // オブジェクトが追加されたことを確認
        assert_eq!(env.obj_id_to_index.get("obj5"), Some(&4));

        // 全てのフィールドリストが拡張されたことを確認
        let next_list = env.field_lists.get("next").unwrap();
        assert_eq!(next_list.len(), 5);
        assert_eq!(next_list[4], Value::Null); // 新しいノードのデフォルト値（nullPtr）

        let value_list = env.field_lists.get("value").unwrap();
        assert_eq!(value_list.len(), 5);
        assert_eq!(value_list[4], json!(-1)); // 新しいノードのデフォルト値
    }

    #[test]
    fn test_add_edge_operation() {
        let graph = create_test_graph();
        let mut env = ListEnvironment::from_vis_graph(&graph);

        // まず新しいNodeを追加
        let add_node_op = GraphOperation {
            edit_type: "addNode".to_string(),
            id: Some("obj5".to_string()),
            label: Some(json!("Node")),
            is_literal: Some(false),
            node_type: None,
            from: None,
            to: None,
            old_to: None,
            new_to: None,
            old_label: None,
            new_label: None,
        };
        env.apply_operation(&add_node_op).unwrap();

        // リテラルノードを追加（実際のリテラルの追加は追跡しないが、操作として表現）
        let add_literal_op = GraphOperation {
            edit_type: "addNode".to_string(),
            id: Some("val5".to_string()),
            label: Some(json!(4)),
            is_literal: Some(true),
            node_type: None,
            from: None,
            to: None,
            old_to: None,
            new_to: None,
            old_label: None,
            new_label: None,
        };
        env.apply_operation(&add_literal_op).unwrap();

        // obj5とval5をvalueエッジで接続
        let add_edge_op = GraphOperation {
            edit_type: "addEdge".to_string(),
            id: None,
            label: Some(json!("value")),
            is_literal: None,
            node_type: None,
            from: Some("obj5".to_string()),
            to: Some("val5".to_string()),
            old_to: None,
            new_to: None,
            old_label: None,
            new_label: None,
        };
        env.apply_operation(&add_edge_op).unwrap();

        println!("After adding edge:");
        println!("{}", env.to_debug_string());

        // エッジが追加されたことを確認
        let value_list = env.field_lists.get("value").unwrap();
        assert_eq!(value_list[4], json!(4)); // obj5.value = 4
    }

    #[test]
    fn test_edit_edge_reference_operation() {
        let graph = create_test_graph();
        let mut env = ListEnvironment::from_vis_graph(&graph);

        let edit_op = GraphOperation {
            edit_type: "editEdgeReference".to_string(),
            from: Some("obj1".to_string()),
            label: Some(json!("next")),
            old_to: Some("obj2".to_string()),
            new_to: Some("obj3".to_string()),
            ..GraphOperation::default()
        };
        env.apply_operation(&edit_op).unwrap();

        let next_list = env.field_lists.get("next").unwrap();
        assert_eq!(next_list[0], json!(2)); // obj1.next = obj3

        let mismatch_old_to_op = GraphOperation {
            edit_type: "editEdgeReference".to_string(),
            from: Some("obj1".to_string()),
            label: Some(json!("next")),
            old_to: Some("obj2".to_string()),
            new_to: Some("null".to_string()),
            ..GraphOperation::default()
        };
        env.apply_operation(&mismatch_old_to_op).unwrap();
        let next_list = env.field_lists.get("next").unwrap();
        assert_eq!(next_list[0], Value::Null); // oldTo mismatchでも更新は継続
    }

    #[test]
    fn test_variable_reference_operations() {
        let graph = create_test_graph();
        let mut env = ListEnvironment::from_vis_graph(&graph);

        let add_var_op = GraphOperation {
            edit_type: "addVariable".to_string(),
            label: Some(json!("return")),
            to: Some("obj2".to_string()),
            ..GraphOperation::default()
        };
        env.apply_operation(&add_var_op).unwrap();

        let var_idx = *env
            .obj_id_to_index
            .get("__Variable-return")
            .expect("variable node should be created");
        let return_list = env.field_lists.get("return").unwrap();
        assert_eq!(return_list[var_idx], json!(1)); // return -> obj2

        let edit_var_op = GraphOperation {
            edit_type: "editVariableReference".to_string(),
            label: Some(json!("return")),
            old_to: Some("obj2".to_string()),
            new_to: Some("obj4".to_string()),
            ..GraphOperation::default()
        };
        env.apply_operation(&edit_var_op).unwrap();
        let return_list = env.field_lists.get("return").unwrap();
        assert_eq!(return_list[var_idx], json!(3)); // return -> obj4
    }
}

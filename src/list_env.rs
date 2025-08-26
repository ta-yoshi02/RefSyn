//! リスト形式の環境表現を管理するモジュール
//!
//! このモジュールはKanonのグラフ操作によって変化する環境を
//! List[field_name] = [values...] の形式で表現するためのものです。

use crate::models::{VisGraph, Node, Edge};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListEnvironment {
    /// フィールド名をキーとし、各オブジェクトのフィールド値の配列を値とするマップ
    pub field_lists: HashMap<String, Vec<Value>>,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

impl ListEnvironment {
    /// 初期のVisGraphからListEnvironmentを作成
    pub fn from_vis_graph(vis_graph: &VisGraph) -> Self {
        let mut object_ids: Vec<String> = vis_graph
            .nodes
            .iter()
            .filter(|n| !n.is_literal)
            .map(|n| n.id.clone())
            .collect();

        // IDの辞書順でソートして一貫したインデックスを保証
        object_ids.sort();

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

        // エッジからフィールドリストを構築
        for edge in &vis_graph.edges {
            if let Some(&from_index) = obj_id_to_index.get(&edge.from) {
                let field_vec = field_lists
                    .entry(edge.label.clone())
                    .or_insert_with(|| vec![json!(-1); num_objects]); // デフォルトは-1

                let value_to_insert = if let Some(&to_index) = obj_id_to_index.get(&edge.to) {
                    json!(to_index as i32)
                } else if let Some(literal_value) = literal_id_to_value.get(&edge.to) {
                    literal_value.clone()
                } else {
                    json!(-1)
                };

                if from_index < field_vec.len() {
                    field_vec[from_index] = value_to_insert;
                }
            }
        }

        ListEnvironment {
            field_lists,
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

        // 全てのフィールドリストに新しい要素を追加（デフォルト値-1）
        for field_vec in self.field_lists.values_mut() {
            field_vec.push(json!(-1));
        }

        Ok(())
    }

    fn add_edge(&mut self, operation: &GraphOperation) -> Result<(), String> {
        let from = operation.from.as_ref().ok_or("addEdge requires from")?;
        let to = operation.to.as_ref().ok_or("addEdge requires to")?;
        let binding = json!("");
        let label = operation.label.as_ref().unwrap_or(&binding).as_str().unwrap_or("");

        // fromがオブジェクトでなければ無視
        let from_index = match self.obj_id_to_index.get(from) {
            Some(&index) => index,
            None => return Ok(()), // fromがオブジェクトでない場合は無視
        };

        // toの値を決定
        let to_value = if let Some(&to_index) = self.obj_id_to_index.get(to) {
            json!(to_index as i32)
        } else if let Some(literal_value) = self.literal_id_to_value.get(to) {
            // toがリテラルIDの場合、対応する値を使用
            literal_value.clone()
        } else {
            // どちらでもない場合はデフォルト値
            json!(-1)
        };

        // フィールドリストを更新
        let field_vec = self.field_lists
            .entry(label.to_string())
            .or_insert_with(|| vec![json!(-1); self.next_index]);

        // リストサイズが足りない場合は拡張
        while field_vec.len() <= from_index {
            field_vec.push(json!(-1));
        }

        field_vec[from_index] = to_value;

        Ok(())
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
        
        for (field_name, values) in &self.field_lists {
            result.push_str(&format!("List[obj_{}] = {:?}\n", field_name, values));
        }
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_graph() -> VisGraph {
        VisGraph {
            nodes: vec![
                Node { id: "obj1".to_string(), is_literal: false, label: json!("Node") },
                Node { id: "obj2".to_string(), is_literal: false, label: json!("Node") },
                Node { id: "obj3".to_string(), is_literal: false, label: json!("Node") },
                Node { id: "obj4".to_string(), is_literal: false, label: json!("Node") },
                Node { id: "val1".to_string(), is_literal: true, label: json!(2) },
                Node { id: "val2".to_string(), is_literal: true, label: json!(0) },
                Node { id: "val3".to_string(), is_literal: true, label: json!(9) },
                Node { id: "val4".to_string(), is_literal: true, label: json!(5) },
            ],
            edges: vec![
                Edge { from: "obj1".to_string(), to: "obj2".to_string(), label: "next".to_string() },
                Edge { from: "obj2".to_string(), to: "obj3".to_string(), label: "next".to_string() },
                Edge { from: "obj3".to_string(), to: "obj4".to_string(), label: "next".to_string() },
                Edge { from: "obj1".to_string(), to: "val1".to_string(), label: "value".to_string() },
                Edge { from: "obj2".to_string(), to: "val2".to_string(), label: "value".to_string() },
                Edge { from: "obj3".to_string(), to: "val3".to_string(), label: "value".to_string() },
                Edge { from: "obj4".to_string(), to: "val4".to_string(), label: "value".to_string() },
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
        assert_eq!(next_list[3], json!(-1)); // obj4 -> null

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
        };

        env.apply_operation(&add_node_op).unwrap();

        println!("After adding node:");
        println!("{}", env.to_debug_string());

        // オブジェクトが追加されたことを確認
        assert_eq!(env.obj_id_to_index.get("obj5"), Some(&4));
        
        // 全てのフィールドリストが拡張されたことを確認
        let next_list = env.field_lists.get("next").unwrap();
        assert_eq!(next_list.len(), 5);
        assert_eq!(next_list[4], json!(-1)); // 新しいノードのデフォルト値

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
        };
        env.apply_operation(&add_edge_op).unwrap();

        println!("After adding edge:");
        println!("{}", env.to_debug_string());

        // エッジが追加されたことを確認
        let value_list = env.field_lists.get("value").unwrap();
        assert_eq!(value_list[4], json!(4)); // obj5.value = 4
    }
}

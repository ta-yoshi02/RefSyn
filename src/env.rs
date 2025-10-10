use crate::models::VisGraph;
use serde_json::json;
use std::collections::HashMap;

/// visGraphオブジェクトを受け取り、フィールド名をキーとした値のリストに変換する。
///
/// # Arguments
/// * `vis_graph` - Kanonから渡されるグラフ表現。
///
/// # Returns
/// エッジのラベル名をキーとし、各オブジェクトのフィールド値を格納した配列を値とするHashMap。
///   - 参照先がオブジェクトの場合: そのオブジェクトのインデックス（usize）
///   - 参照先がリテラルの場合: そのリテラル値（serde_json::Value）
///   - 参照がない場合: null
pub fn translate_graph_to_field_lists(
    vis_graph: &VisGraph,
) -> HashMap<String, Vec<serde_json::Value>> {
    // フェーズ1: オブジェクトのインデックス化とリテラルのマッピング
    let mut object_ids: Vec<&String> = vis_graph
        .nodes
        .iter()
        .filter(|n| !n.is_literal)
        .map(|n| &n.id)
        .collect();

    // オブジェクトの順序をIDの辞書順で固定し、一貫したインデックスを保証する
    object_ids.sort();

    let obj_id_to_index: HashMap<&String, usize> = object_ids
        .iter()
        .enumerate()
        .map(|(i, &id)| (id, i))
        .collect();

    let literal_id_to_value: HashMap<&String, &serde_json::Value> = vis_graph
        .nodes
        .iter()
        .filter(|n| n.is_literal)
        .map(|n| (&n.id, &n.label))
        .collect();

    let num_objects = object_ids.len();
    let mut field_lists: HashMap<String, Vec<serde_json::Value>> = HashMap::new();

    // フェーズ2: フィールドリストの生成
    for edge in &vis_graph.edges {
        // エッジの始点がオブジェクトでなければスキップ
        if let Some(&from_index) = obj_id_to_index.get(&edge.from) {
            // このエッジラベルに対応するVecを確保（なければデフォルト値で初期化）
            let field_vec = field_lists
                .entry(edge.label.clone())
                .or_insert_with(|| vec![json!(null); num_objects]);

            // 終点のノードがオブジェクトかリテラルかを判断し、格納する値を決める
            let value_to_insert = if let Some(&to_index) = obj_id_to_index.get(&edge.to) {
                // 他のオブジェクトへの参照: インデックスを格納
                json!(to_index)
            } else if let Some(literal_value) = literal_id_to_value.get(&edge.to) {
                // リテラルへの参照: リテラル値を格納
                (*literal_value).clone()
            } else {
                // 該当なし（不正なエッジなど）
                json!(null)
            };

            // from_indexに対応する場所に値を格納
            if from_index < field_vec.len() {
                field_vec[from_index] = value_to_insert;
            }
        }
    }

    field_lists
}

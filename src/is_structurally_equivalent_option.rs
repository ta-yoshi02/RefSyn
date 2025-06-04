use std::collections::HashMap;

/// Option<OpId>型の2つの値が構造的に同等かを判定するヘルパー関数
pub fn is_structurally_equivalent_option(
    id_a: &Option<String>, 
    id_b: &Option<String>, 
    id_mapping: &HashMap<String, String>
) -> bool {
    match (id_a, id_b) {
        (Some(a), Some(b)) => {
            // 両方Someの場合は文字列値で比較
            if a == b {
                return true;
            }
            
            // IDマッピングで対応している場合は同等とみなす
            if let Some(mapped_id) = id_mapping.get(a) {
                if mapped_id == b {
                    return true;
                }
            }
            
            // 逆方向の確認
            if let Some(mapped_id) = id_mapping.get(b) {
                if mapped_id == a {
                    return true;
                }
            }
            
            false
        },
        (None, None) => {
            // 両方Noneの場合は同等
            true
        },
        _ => {
            // 一方がSomeで他方がNoneの場合は異なる
            false
        }
    }
}

//! 環境管理用モジュール
//!
//! このモジュールは、コード生成時の変数スコープと共有を管理するための環境システムを提供します。
//! 主に以下の3つのコンポーネントから構成されています：
//!
//! 1. `VarEnv` - メソッド呼び出し内の局所的な変数環境を管理します
//! 2. `MemoEnv` - プログラムのコンテクストを管理します
//! 3. `EnvManager` - 上記2つのコンポーネントを統合し、コンテキストに応じた環境アクセスを提供します
//!
//! このモジュールは特に異なるメソッド呼び出し間で変数を適切に共有し、
//! 正確なコード生成を可能にする役割を担っています。

use crate::models::VisGraph;
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct MemoEnv {
    id_to_name: HashMap<String, String>,
    name_to_id: HashMap<String, String>,
    next_var_id: u32,
    property_of: HashMap<String, (String, String)>, // obj_id -> (owner_id, property_name)
    current_method_call: Option<String>,            // 現在処理中のメソッド呼び出しID
    method_call_vars: HashMap<String, Vec<String>>, // メソッド呼び出しIDごとの変数ID一覧
    external_refs: HashMap<String, String>,         // 変数IDの外部参照パス (例: obj_0 -> "this.next")
    method_receivers: HashMap<String, String>,      // メソッド呼び出しID -> レシーバーオブジェクトID
    current_receiver: Option<String>,               // 現在のメソッド呼び出しのレシーバーID
}

impl MemoEnv {
    pub fn new() -> Self {
        Default::default()
    }

    /// 指定されたIDと名前の特別なマッピングを追加します。主に "this" のようなキーワードに使用します。
    pub fn add_special_mapping(&mut self, id: String, name: String) {
        self.id_to_name.insert(id.clone(), name.clone());
        self.name_to_id.insert(name, id);
    }

    /// IDに対応する名前を取得します。
    pub fn get_name_by_id(&self, id: &str) -> Option<&String> {
        self.id_to_name.get(id)
    }

    /// 名前から対応するIDを取得します。
    pub fn get_id_by_name(&self, name: &str) -> Option<&String> {
        self.name_to_id.get(name)
    }

    /// テスト用のIDと名前のマッピングを追加します。
    pub fn add_name_id_mapping(&mut self, id: String, name: String) {
        self.id_to_name.insert(id.clone(), name.clone());
        self.name_to_id.insert(name, id);
    }

    /// プロパティの割り当てを登録します。
    /// `value_id` が `owner_id` の `property_name` プロパティであることを記録します。
    pub fn register_property_assignment(
        &mut self,
        owner_id: String,
        property_name: String,
        value_id: String,
    ) {
        self.property_of.insert(value_id, (owner_id, property_name));
    }

    /// 指定されたIDがどのオブジェクトのプロパティであるかを取得します。
    /// (owner_id, property_name) のタプルを返します。
    pub fn get_property_access_for_id(&self, id: &str) -> Option<&(String, String)> {
        self.property_of.get(id)
    }

    /// IDに対応する変数名を解決または生成します。
    /// 既にマッピングが存在する場合はその名前を返します。
    /// 存在しない場合は、新しい一意な変数名を生成してマッピングし、その名前を返します。
    pub fn resolve_or_create_var_name_for_id(&mut self, id: &str, is_receiver: bool) -> String {
        // 既存のマッピングがある場合はそれを使用
        if let Some(name) = self.id_to_name.get(id) {
            return name.clone();
        }

        // 外部参照パスがある場合はそれを返す（優先）
        if let Some(external_path) = self.get_external_reference_path(id) {
            return external_path;
        }

        // レシーバーとして明示的に指定された場合のみ "this" を返す
        if is_receiver {
            // 現在のメソッド呼び出しコンテキストでのレシーバーとして登録
            self.register_method_receiver(id);
            return "this".to_string();
        }

        // 通常の変数として新しい名前を生成
        let new_name = format!("obj_{}", self.next_var_id);
        self.next_var_id += 1;
        self.id_to_name.insert(id.to_string(), new_name.clone());
        self.name_to_id.insert(new_name.clone(), id.to_string());
        new_name
    }

    /// 指定されたIDのアクセスパスを構築します。
    /// 外部参照、プロパティ参照チェーンを辿り、"this.prop1.prop2" や "obj_N" といった形式の文字列を構築します。
    pub fn get_access_path(&self, id: &str) -> Option<String> {
        // 1. 外部参照が直接登録されていればそれを返す
        if let Some(path) = self.external_refs.get(id) {
            return Some(path.clone());
        }

        // 2. 現在のレシーバーまたはthisとしてマッピングされているかチェック
        if self.is_current_receiver(id) {
            return Some("this".to_string());
        }
        
        if let Some(name) = self.id_to_name.get(id) {
            if name == "this" {
                return Some("this".to_string());
            }
        }

        // 3. プロパティチェーンを辿って "this" からのパスを構築
        let mut current_id_for_prop_chain = id.to_string();
        let mut path_components = Vec::new();
        let mut visited_ids_for_prop_chain = std::collections::HashSet::new(); // ループ防止

        while visited_ids_for_prop_chain.insert(current_id_for_prop_chain.clone()) {
            if let Some((owner_id, prop_name)) = self.property_of.get(&current_id_for_prop_chain) {
                path_components.push(prop_name.clone());
                
                // 所有者が現在のレシーバーまたは "this" ならパスを構築して返す
                if self.is_current_receiver(owner_id) {
                    path_components.reverse();
                    return Some(format!("this.{}", path_components.join(".")));
                }
                
                if let Some(owner_name) = self.id_to_name.get(owner_id) {
                    if owner_name == "this" {
                        path_components.reverse();
                        return Some(format!("this.{}", path_components.join(".")));
                    }
                }
                current_id_for_prop_chain = owner_id.clone(); // 所有者を辿る
            } else {
                break; // これ以上プロパティチェーンを辿れない
            }
        }
        
        // 4. ローカル変数名を返す（thisでない場合）
        if let Some(name) = self.id_to_name.get(id) {
            if name != "this" {
                return Some(name.clone());
            }
        }

        // 5. それでも見つからなければアクセスパスは不明
        None
    }

    /// メソッド呼び出しのスコープを開始します。
    /// これによりメソッド呼び出し内の変数を追跡できるようになります。
    pub fn start_method_call_scope(&mut self, method_call_id: &str) {
        self.current_method_call = Some(method_call_id.to_string());
        self.method_call_vars.insert(method_call_id.to_string(), Vec::new());
        // 新しいメソッド呼び出しスコープで変数カウンターをリセット
        self.next_var_id = 0;
    }

    /// メソッド呼び出しのスコープを開始し、レシーバーを設定します。
    pub fn start_method_call_scope_with_receiver(&mut self, method_call_id: &str, receiver_id: &str) {
        self.start_method_call_scope(method_call_id);
        self.set_current_receiver(receiver_id);
    }

    /// 現在のメソッド呼び出しスコープを終了します。
    pub fn end_method_call_scope(&mut self) {
        self.current_method_call = None;
        self.current_receiver = None; // レシーバーコンテキストもクリア
    }

    /// 変数IDを現在のメソッド呼び出しスコープに登録します。
    pub fn register_var_in_current_scope(&mut self, var_id: &str) {
        if let Some(method_call_id) = &self.current_method_call {
            if let Some(vars) = self.method_call_vars.get_mut(method_call_id) {
                if !vars.contains(&var_id.to_string()) {
                    vars.push(var_id.to_string());
                }
            }
        }
    }

    /// 変数の外部参照パスを取得または設定します。
    /// 例：obj_0 -> "this.next" のようにメソッド呼び出し後に変数がどのように参照されるかを記録します。
    pub fn register_external_reference(&mut self, var_id: &str, ref_path: &str) {
        self.external_refs.insert(var_id.to_string(), ref_path.to_string());
    }

    /// 変数の外部参照パスを取得します。
    pub fn get_external_reference_path(&self, var_id: &str) -> Option<String> {
        // 直接登録されている外部参照があればそれを返す
        if let Some(path) = self.external_refs.get(var_id) {
            return Some(path.clone());
        }
        
        // プロパティチェーンから外部参照パスを構築
        // 例：obj_0が"this.next"のプロパティとして登録されている場合など
        if let Some((owner_id, prop_name)) = self.property_of.get(var_id) {
            if let Some(owner_name) = self.get_name_by_id(owner_id) {
                if owner_name == "this" {
                    // 外部参照パスを返す
                    return Some(format!("this.{}" , prop_name));
                }
            }
        }
        
        None
    }

    /// 他のMemoEnvの状態をこの環境に統合します。
    /// これはグローバル状態管理に使用されます。
    pub fn merge_from(&mut self, other: &MemoEnv) {
        // プロパティ割り当てを統合
        for (var_id, (owner_id, prop_name)) in &other.property_of {
            self.property_of.insert(var_id.clone(), (owner_id.clone(), prop_name.clone()));
        }
        
        // 外部参照を統合
        for (var_id, ref_path) in &other.external_refs {
            self.external_refs.insert(var_id.clone(), ref_path.clone());
        }
        
        // ID-名前マッピングを統合（重複がある場合は既存を保持）
        for (id, name) in &other.id_to_name {
            if !self.id_to_name.contains_key(id) {
                self.id_to_name.insert(id.clone(), name.clone());
                self.name_to_id.insert(name.clone(), id.clone());
            }
        }
    }
    
    /// グローバルな外部参照を設定します。
    /// 前のメソッド呼び出しの結果を次のメソッド呼び出しで参照するために使用されます。
    pub fn setup_cross_scope_references(&mut self, global_env: &MemoEnv) {
        // プロパティ割り当てから外部参照パスを推定
        for (var_id, (owner_id, prop_name)) in &global_env.property_of {
            if let Some(owner_name) = global_env.get_name_by_id(owner_id) {
                if owner_name == "this" {
                    let ref_path = format!("this.{}" , prop_name);
                    self.register_external_reference(var_id, &ref_path);
                } else if let Some(owner_ref_path) = global_env.get_external_reference_path(owner_id) {
                    let ref_path = format!("{}.{}" , owner_ref_path, prop_name);
                    self.register_external_reference(var_id, &ref_path);
                }
            }
        }
    }

    /// メソッド呼び出しのレシーバーを登録します。
    /// レシーバーオブジェクトIDを現在のメソッド呼び出しコンテキストに関連付けます。
    pub fn register_method_receiver(&mut self, receiver_id: &str) {
        self.current_receiver = Some(receiver_id.to_string());
        if let Some(method_call_id) = &self.current_method_call {
            self.method_receivers.insert(method_call_id.clone(), receiver_id.to_string());
        }
        // レシーバーを "this" として登録
        self.add_special_mapping(receiver_id.to_string(), "this".to_string());
    }

    /// 現在のメソッド呼び出しコンテキストでレシーバーIDを設定します。
    pub fn set_current_receiver(&mut self, receiver_id: &str) {
        self.current_receiver = Some(receiver_id.to_string());
        // レシーバーを "this" として登録
        self.add_special_mapping(receiver_id.to_string(), "this".to_string());
    }

    /// 指定されたIDが現在のレシーバーかどうかを判定します。
    pub fn is_current_receiver(&self, id: &str) -> bool {
        self.current_receiver.as_ref().map_or(false, |receiver| receiver == id)
    }

    /// 指定されたメソッド呼び出しのレシーバーIDを取得します。
    pub fn get_method_receiver(&self, method_call_id: &str) -> Option<&String> {
        self.method_receivers.get(method_call_id)
    }

    /// 現在のレシーバーIDを取得します。
    pub fn get_current_receiver(&self) -> Option<&String> {
        self.current_receiver.as_ref()
    }
}

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
//! 環境管理用モジュール
//!
//! このモジュールは、コード生成時の変数スコープと共有を管理するための環境システムを提供します。
//! 主に以下の3つのコンポーネントから構成されています：
//!
//! 1. `VarEnv` - メソッド呼び出し内の局所的な変数環境を管理します
//! 2. `MemoEnv` - メソッド間で共有される重要な変数と関係性を管理します
//! 3. `EnvManager` - 上記2つのコンポーネントを統合し、コンテキストに応じた環境アクセスを提供します
//!
//! このモジュールは特に異なるメソッド呼び出し間で変数を適切に共有し、
//! 正確なコード生成を可能にする役割を担っています。

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
    /// receiver_object_id に一致する場合は特別な名前 (例: "this") を返します。
    pub fn resolve_or_create_var_name_for_id(&mut self, id: &str, is_receiver: bool) -> String {
        if is_receiver {
            // receiver_object_id に一致する場合は "this" を返す
            return "this".to_string();
        }

        // main-new* パターンのIDを自動的に "this" として認識
        if id.starts_with("main-new") {
            if !self.id_to_name.contains_key(id) {
                self.add_special_mapping(id.to_string(), "this".to_string());
            }
            return "this".to_string();
        }

        // 外部参照パスがある場合はそれを返す（優先）
        if let Some(external_path) = self.get_external_reference_path(id) {
            return external_path;
        }

        if let Some(name) = self.id_to_name.get(id) {
            return name.clone();
        }

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

        // 2. IDが "this" 自身を指す場合 (main-new* パターンなど、特殊マッピングで "this" になっている場合も含む)
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
                
                // 所有者が "this" (特殊マッピング含む) ならパスを構築して返す
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
        
        // 4. 上記で見つからなければ、IDに紐づくローカル名 (obj_Xなど) を返す (存在すれば)
        //    これは、メソッドスコープ内で完結し、外部参照もthisからのプロパティでもない場合。
        //    ただし、この関数は「アクセスパス」を求めるものなので、ローカル名を返すのが適切かは検討の余地あり。
        //    現状の resolve_or_create_var_name_for_id がローカル名を生成するので、ここではNoneを返す方が一貫性があるかもしれない。
        //    一旦、get_name_by_idで取得できる名前を返すようにしてみる。
        //    ただし、それが "this" でないことを確認する（ステップ2で処理済みのため）。
        if let Some(name) = self.id_to_name.get(id) {
            if name != "this" { // "this" は既に処理済み
                 // ここで `obj_N` のような名前が返ることを期待。
                 // ただし、これが本当に「アクセスパス」と言えるかは文脈による。
                 // 例えば、`let obj_0 = new Foo();` の `obj_0` はこの段階では `obj_0`。
                 // `this.bar = obj_0;` となった後、`obj_0` の外部参照は `this.bar` になるべき。
                 // `external_refs` やプロパティチェーンで解決できない場合のフォールバックとして機能する。
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

    /// 現在のメソッド呼び出しスコープを終了します。
    pub fn end_method_call_scope(&mut self) {
        self.current_method_call = None;
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
                    return Some(format!("this.{}", prop_name));
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
                    let ref_path = format!("this.{}", prop_name);
                    self.register_external_reference(var_id, &ref_path);
                } else if let Some(owner_ref_path) = global_env.get_external_reference_path(owner_id) {
                    let ref_path = format!("{}.{}", owner_ref_path, prop_name);
                    self.register_external_reference(var_id, &ref_path);
                }
            }
        }
    }
}

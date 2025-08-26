# GEMINI.md: Kanon-to-Escher-Scala Interpreter Specification

## 1. プロジェクト概要：Kanon-to-Escher-Scala Interpreter

### 1.1. 目的

本プロジェクトの目的は、対話的プログラム合成フレームワーク**RefSyn**の一部として、ライブプログラミング環境**Kanon**が生成する可視化グラフ情報（JavaScriptオブジェクト環境）を、プログラム合成エンジン**Escher-Scala**が解釈可能な形式的なJSON仕様に変換するインタープリターを実装することです。

これにより、ユーザーがGUI上で行った直感的なデータ構造の操作から、主問題とそれに付随する複数の部分問題の仕様を自動抽出し、複雑なプログラムの合成を可能にします。

### 1.2. 設計思想 (Core Tenets)

* **動的フィールド検出:** `next`や`val`といった特定のフィールド名をハードコーディングせず、Kanonのグラフ構造から動的に値フィールドとポインタフィールドを特定します。
* **型互換性:** `null`や`undefined`、リストの終端を一意の整数値（センチネル値として **-1**）で表現し、Escher-Scalaの`List[Int]`型との互換性を確保します。
* **ローカルスコープ:** （v2.0更新）環境内に単一のデータ構造のみを想定し、インデックス解決を各テストケース内で完結させます。これにより、実装が簡素化され、各テストケースの独立性が保たれます。

---

## 2. 実装アーキテクチャ

インタープリターは、以下の3つの主要ステップで構成されます。

### ステップ1：グラフ構造の解析 (Analyze) 🌳

Kanon環境をスキャンし、対象となるデータ構造の特性を把握します。

1.  **ルートオブジェクトの特定:** 変数宣言を手がかりに、トラバーサルの開始点となる単一のルートオブジェクト（例：`list`）を特定します。
2.  **フィールド名の動的スキャン:** ルートオブジェクトからグラフを辿り、値フィールド（例：`val`）とポインタフィールド（例：`next`, `prev`）を動的に洗い出します。

### ステップ2：データ構造の変換 (Translate) 🔄

解析されたデータ構造を、値リストとポインタインデックスリストのセットに変換します。この処理は**テストケースごと**に独立して行われます。

1.  **ローカルインデックスの構築:** トラバーサルを開始する**前**に、対象データ構造内の全オブジェクトを一度スキャンし、そのテストケース内でのみ有効な`id_to_index`マップを作成します。
2.  **トラバーサルの実行:** ルートオブジェクトから幅優先探索（BFS）などを実行し、オブジェクトの訪問順を決定論的に定めます。
3.  **リストセットの構築:** トラバーサル中に、以下のリスト群を生成します。
    * **`vs` (Value List):** 値フィールドの値を格納するリスト。 (`List[Int]`)
    * **`ns_<field>` (Next-node Index List):** 検出された各ポインタフィールドについて、参照先のノードインデックスを格納するリスト。 (`List[Int]`)
4.  **値の正規化:**
    * `Int`型の値はそのまま格納します。
    * `null`、`undefined`、およびリストの終端（どこも指していないポインタ）は、センチネル値 **-1** として表現します。

### ステップ3：JSON仕様への整形 (Format) 📝

すべてのリストセットを、Escher-Scalaが要求する単一のJSONオブジェクトにまとめ上げます。

1.  **`input`配列の構築:** 各データ構造から得られたリストセット（例：`vs`, `ns_next`, `ns_prev`, ...）と、関数に渡される単純な引数（例：`Int`）を、一つの`input`配列にまとめます。
2.  **`inputTypes`の動的生成:** 構築したリストセットと引数の型情報に基づき、`["Int", "List[Int]", "List[Int]", ...]`のような型配列を動的に生成します。
3.  **`examples`配列の生成:** 複数のKanon環境（テストケース）を処理し、それぞれを`examples`配列内の1つの要素として整形します。

---

## 3. 詳細実装ガイド

### 3.1. データ構造 (Rust)

```rust
use std::collections::{HashMap, VecDeque, HashSet};
use serde_json::{json, Value};

// Kanon環境の仮定義
type ObjectId = String;
type VariableName = String;
type FieldName = String;
struct KanonObject {
    fields: HashMap<FieldName, Value>, // 値フィールドとポインタフィールドを両方含む
}
type KanonEnv = HashMap<ObjectId, KanonObject>;
type RootObject = (VariableName, ObjectId);

// 1つのテストケースを表す
struct TestCase {
    pub env: KanonEnv,
    pub root: RootObject,
    pub arguments: Vec<Value>,
    pub output: Value,
}

// 1つの独立したデータ構造の解析結果
struct AnalyzedStructure {
    pub root_object_id: ObjectId,
    pub value_fields: Vec<FieldName>,
    pub pointer_fields: Vec<FieldName>,
}
```

### 3.2. 実装ステップ (Rust)

#### **ステップ1 & 2：解析と変換を統合した関数**

```rust
// fn translate_test_case(test_case: &TestCase) -> Value

// 1. フィールド名を動的にスキャン
let structure = analyze_structure(&test_case.root, &test_case.env);

// 2. ローカルなID->インデックスのマッピングを作成
let mut id_to_index = HashMap::new();
let mut q = VecDeque::new();
q.push_back(structure.root_object_id.clone());
let mut visited_ids_for_indexing = HashSet::new();
let mut index_counter = 0;
while let Some(id) = q.pop_front() {
    if !visited_ids_for_indexing.insert(id.clone()) { continue; }
    id_to_index.insert(id.clone(), index_counter);
    index_counter += 1;
    let node = test_case.env.get(&id).unwrap();
    for pf in &structure.pointer_fields {
        if let Some(next_id) = node.fields.get(pf).and_then(|v| v.as_str()) {
            q.push_back(next_id.to_string());
        }
    }
}

// 3. リストセットを構築
let mut lists: HashMap<String, Vec<Value>> = HashMap::new();
// ... (translate_single_structure のロジックをここに展開) ...
// トラバーサルを実行し、id_to_indexを使ってvsとns_*を構築する

// 4. `input`配列を構築
let mut inputs = test_case.arguments.clone();
// 決定論的な順序でリストを追加 (フィールド名をソート)
let mut sorted_value_fields = structure.value_fields.clone();
sorted_value_fields.sort();
for vf in sorted_value_fields {
    inputs.push(json!(lists.get(&format!("vs_{}", vf)).unwrap()));
}
let mut sorted_pointer_fields = structure.pointer_fields.clone();
sorted_pointer_fields.sort();
for pf in sorted_pointer_fields {
    inputs.push(json!(lists.get(&format!("ns_{}", pf)).unwrap()));
}

// return json!({ "input": inputs, "output": test_case.output });
```

#### **ステップ3：JSON仕様への整形 (Format)**

```rust
// fn format_as_escher_json(all_test_cases: Vec<TestCase>) -> String

let mut examples = Vec::new();
for test_case in &all_test_cases {
    let example = translate_test_case(test_case);
    examples.push(example);
}

// inputTypesは、最初のテストケースから決定論的に生成
let first_case_analysis = analyze_structure(&all_test_cases[0].root, &all_test_cases[0].env);
let mut input_types = vec!["Int"]; // 引数の型
let mut sorted_value_fields = first_case_analysis.value_fields;
sorted_value_fields.sort();
for _ in sorted_value_fields {
    input_types.push("List[Int]".to_string());
}
let mut sorted_pointer_fields = first_case_analysis.pointer_fields;
sorted_pointer_fields.sort();
for _ in sorted_pointer_fields {
    input_types.push("List[Int]".to_string());
}

let escher_spec = json!({
  "name": "append-g", // 生成したい関数名
  "inputTypes": input_types,
  "returnType": "Int", // 生成したい関数の返り値の型
  "examples": examples
});

// return serde_json::to_string_pretty(&escher_spec).unwrap();
```

---

## 4. Example Walkthrough (Rich Version, Local Indexing)

**シナリオ:** `append`関数を合成する過程で、**「リストの最後のノードのインデックスを返す」**ヘルパー関数`append-g`の仕様を生成するケースを考えます。

**入力:** 3つの異なるKanon環境（テストケース）

1.  **Case 1:**
    * `var list = new Node(2);` (`id1`)
    * 引数: `arg = 0`
    * 期待される出力 (`output`): `0` (リストの最後のノードは`list`自身なので、そのローカルインデックスは`0`)
2.  **Case 2:**
    * `var list = new Node(1); list.next = new Node(2);` (`id2`, `id3`)
    * 引数: `arg = 5`
    * 期待される出力 (`output`): `1` (最後のノード`id3`のローカルインデックスは`1`)
3.  **Case 3:**
    * `var list = new Node(10); list.next = new Node(20); list.next.next = new Node(30);` (`id4`, `id5`, `id6`)
    * 引数: `arg = 99`
    * 期待される出力 (`output`): `2` (最後のノード`id6`のローカルインデックスは`2`)

**実行フロー (各ケースで独立):**

* **Case 1の処理:**
    * ローカルインデックス: `{"id1": 0}`
    * `vs = [2]`
    * `ns_next = [-1]`
    * `input = [0, [2], [-1]]` (引数, vs, ns_next)
    * `output = 0`
* **Case 2の処理:**
    * ローカルインデックス: `{"id2": 0, "id3": 1}`
    * `vs = [1, 2]`
    * `ns_next = [1, -1]`
    * `input = [5, [1, 2], [1, -1]]`
    * `output = 1`
* **Case 3の処理:**
    * ローカルインデックス: `{"id4": 0, "id5": 1, "id6": 2}`
    * `vs = [10, 20, 30]`
    * `ns_next = [1, 2, -1]`
    * `input = [99, [10, 20, 30], [1, 2, -1]]`
    * `output = 2`

**最終的なJSON:**

```json
[
  {
    "name": "append-g",
    "inputTypes": ["Int", "List[Int]", "List[Int]"],
    "returnType": "Int",
    "examples": [
      { "input": [0, [2], [-1]], "output": 0 },
      { "input": [5, [1, 2], [1, -1]], "output": 1 },
      { "input": [99, [10, 20, 30], [1, 2, -1]], "output": 2 }
    ]
  }
]

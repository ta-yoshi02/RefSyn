# Kanon → Escher JSON ブリッジ（default: escher-ts, fallback: Scala）

このドキュメントは、Kanon の操作トレース（オペレーション列）から Escher 用 JSON を生成する手順を説明します。既定の出力は `escher-ts` の native task JSON で、必要なら legacy の Escher-Scala spec も生成できます。

1) 同型統合（unification）で共通部分と差分を特定
2) 差分境界でのローカル List/Int 環境スナップショットを構築
3) Escher 互換の JSON 例（examples）を出力

最後に、生成したファイルを `scripts/run_escher.js` 経由で合成器へ渡します。

## 実装済みの機能

- モジュール: `src/escher_bridge.rs`
- `EscherCase`: `ListEnvironment` と関数引数（`arg_names` を含む）、期待出力をまとめたケース（`receiver_arg_index` で `this` の位置を指定）
  - `build_escher_spec(name, return_type, cases, field_tables) -> EscherSpec`:
    - フィールド（値フィールド/ポインタフィールド）を動的に検出
    - 変数ノードから BFS によるローカルインデックス化
    - nullPtr は JSON `null`、`Int` の欠損はセンチネル `-1`
    - 複数の `examples` を持つ単一の legacy Escher 仕様を生成
  - `derive_spec_meta(&cases) -> EscherSpecMeta` で native task 生成に必要なメタ情報を導出
  - `build_escher_task_spec(&spec, &meta) -> EscherTaskSpec` で escher-ts native task を生成
  - `tasks_to_json(&[EscherTaskSpec]) -> String` で native task 配列 JSON を生成
  - `write_spec_to_file(path, content)` でファイル出力

- 環境ビルダー: `src/list_env.rs`
  - `ListEnvironment::from_vis_graph(&VisGraph)` で初期リストを構築
  - `ListEnvironment::apply_operation(&GraphOperation)` で Kanon の編集操作を適用
  - ポインタはインデックス、値は整数として保持（数値文字列はブリッジ側で数値化をサポート）

- 統合ユーティリティ: `src/lib.rs`
  - `pub fn analyze_operations_with_unification(vis_graph, operations_a, operations_b)`
    - `unification_result`（`common_a`, `diff_a` 等）を含む `UnificationAnalysisResult` を返却
  - 備考: `common_a` の各操作は ID（例: `op_3` → 3番目）から元の JSON 操作列へ対応付け可能

## エンドツーエンドの流れ

各テストケースの入力:
- `VisGraph`（Kanon の環境: ノード/エッジ）
- 比較する操作列 2 本（A と B、JSON 配列）
- 関数引数（`__Variable-<name>` を引数とみなし、参照先がオブジェクトなら `Ptr`、リテラルなら `Int`）
- 期待される出力（例: `Int`）

手順:

1. 操作列の統合で共通/差分を特定
   - A と B を統合用の表現に変換し、統合を計算
   - 使用関数: `refsyn::analyze_operations_with_unification(&vis_graph, &ops_a, &ops_b)`

2. 差分境界での環境を構築
   - `ListEnvironment::from_vis_graph(&vis_graph)` から開始
   - `analysis.unification_result.common_a` から ID（`op_0`, `op_1`, …）の末尾番号を抽出し昇順にソート
   - 各番号 i について:
     - 元の JSON A[i] を `list_env::GraphOperation` にデコード（serde）
     - `env.apply_operation(&graph_op)` で適用
   - 得られた `env` が「最初の差分直前」のスナップショット

3. Escher ケースを作成
   - `env`, `vis_graph`, `arguments`, `arg_names`, `arg_types`, `receiver_arg_index`, `output` を `EscherCase` に包む
   - Kanon 側の複数テストを複数ケースとして作り、`build_escher_spec` にまとめて渡せる

4. native task JSON を出力
- `build_escher_spec("<func-name>", "<return-type>", &cases, None)` → `EscherSpec`
- `derive_spec_meta(&cases)` と `build_escher_task_spec(&spec, &meta)` で escher-ts task へ変換
- `tasks_to_json(&tasks)` で文字列化
- 任意のパスへ保存（例: `target/escher/ts/tests.json`）:
  - `write_spec_to_file(path, &json_text)`
- その後、`scripts/run_escher.js` が既定 backend (`ESCHER_BACKEND=ts`) で実行する

## サンプルコード（Rust）

前提（各テストごとに準備済み）:
- `vis_graph: refsyn::models::VisGraph`
- `ops_a: Vec<serde_json::Value>`
- `ops_b: Vec<serde_json::Value>`
- `arguments: Vec<serde_json::Value>`（例: `[json!(0)]`）
- `arg_types: Option<Vec<String>>`（例: `Some(vec!["Ptr".to_string()])`。省略時は `Int` 扱い）
- `receiver_arg_index: Option<usize>`（例: `Some(0)`）
- `expected_output: serde_json::Value`（例: `json!(2)`）

```rust
use refsyn::escher_bridge::{
    build_escher_spec, build_escher_task_spec, derive_spec_meta, tasks_to_json, write_spec_to_file,
    EscherCase,
};
use refsyn::list_env::{ListEnvironment, GraphOperation};
use serde_json::json;

// 1) 統合（unification）
let analysis = refsyn::analyze_operations_with_unification(&vis_graph, &ops_a, &ops_b)?;

// 2) 共通部分の境界で環境を構築（A 側の共通操作を元の順序で適用）
let mut env = ListEnvironment::from_vis_graph(&vis_graph);
let mut common_indices: Vec<usize> = analysis
    .unification_result
    .common_a
    .iter()
    .filter_map(|op| op.id.strip_prefix("op_")?.parse::<usize>().ok())
    .collect();
common_indices.sort_unstable();

for idx in common_indices {
    let op_json = ops_a[idx].clone();
    let graph_op: GraphOperation = serde_json::from_value(op_json)?;
    env.apply_operation(&graph_op)?;
}

// 3) Escher ケースに包む
let case = EscherCase {
    env,
    vis_graph: vis_graph.clone(),
    arguments: vec![json!(0)],          // 受け取り側のポインタ
    arg_names: vec!["this".to_string()],
    arg_types: Some(vec!["Ptr".to_string()]),
    receiver_arg_index: Some(0),
    output: json!(2),                   // 期待出力
};

// 複数ケースを作成してまとめて渡すことも可能
let meta = derive_spec_meta(std::slice::from_ref(&case))?;
let spec = build_escher_spec("append-g", "Int", &[case], None)?;
let task = build_escher_task_spec(&spec, &meta)?;
let json_text = tasks_to_json(&[task])?;

// 4) 既定 backend (escher-ts) が読むファイルとして書き出す
write_spec_to_file("target/escher/ts/tests.json", &json_text)?;
```

## フィールド検出とインデックス化の詳細

- 動的フィールド検出:
  - 値フィールド: リテラルノードへ向かうエッジが1つ以上あるラベル
  - ポインタフィールド: 非リテラルオブジェクトへ向かうエッジが1つ以上あるラベル
  - 特殊ノード（`__RectForVariable__`, `__Variable-*`）は分類から除外

- BFS のルート検出:
  - ID が `__Variable-this` の変数ノードがあれば優先
  - それ以外は `__Variable-<name>` の変数ノードを探索
  - その変数インデックスの `<name>` フィールドがオブジェクトを指すなら、そのオブジェクトを BFS ルートに採用
  - BFS はポインタフィールドのみを辿り、ローカルインデックス順を決める

- 正規化:
  - スカラー値: `Int` として格納（数値 JSON または数値文字列）
  - `Int` の欠損/未定義: `-1`
  - ポインタ: 元のインデックスを BFS ローカルインデックスへ再マップ、終端は `null`

- 引数順序:
  - `this` を先頭にし、残りは変数名の昇順（`__Variable-<name>` の `<name>`）
  - 参照先がオブジェクトなら `Ptr`、リテラルなら `Int`

## 複数のテストケース

複数の `EscherCase` を `build_escher_spec` に渡すと、ブリッジは次を行います:
- 最初のケースから `inputTypes` を導出（引数型は `arg_types` を優先、未指定なら `Int`）
- 検出したフィールド集合が全ケースで一致するか検査
- `examples` に全ケースを含めた単一の関数仕様を生成

## Tips / 注意点

- Kanon のリテラルが文字列（例: `"2"`）でも、ブリッジは値リストとして数値へパースします。
- ルートの変数ノード（例: `__Variable-lst` → ラベル `lst`）が存在することを確認してください。ない場合はルート検出に失敗します。
- `Ptr` 引数が複数ある場合は `receiver_arg_index` を指定して受け取り位置を明示してください。
- `method_calls[].arguments`（および任意の `argumentTypes` / `argumentNames`）を渡すと、呼び出し引数が Escher 入力引数へそのまま反映されます。
  これにより `append(26)` と `append(10)` のような差は、環境差分ではなくメソッド引数として表現できます。
- Unification を使わず位置ベースのフォールバックが必要な場合は、
  - `operation_analyzer::analyze_operations_with_environments` を利用できます。差分ごとに `environment_before` を返すので、そのまま `EscherCase` に変換可能です。
- `lib.rs` 内部の一部ヘルパは出力整形上、エッジ表現が文字列になる箇所があります。ブリッジ用途では、本ドキュメントの `ListEnvironment` API を推奨します。

## 配置場所（ソース案内）

- `src/escher_bridge.rs`: ブリッジ実装（spec JSON 生成）
- `src/list_env.rs`: リストベース環境表現と操作適用
- `src/lib.rs`: 公開 API（`analyze_operations_with_unification`）
- `scripts/run_escher.js`: backend 切り替え用ランナー（既定: `ts`）
- `scripts/run_escher_scala.js`: legacy Scala fallback
- `external/escher-ts`: native escher-ts 実装

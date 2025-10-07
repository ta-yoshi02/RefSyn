# RefSyn Test Suite Overview

本書は、このリポジトリに含まれる全テスト（Rustのユニット/統合テスト、ならびに Escher-Scala 側で消費する `tests.json` 仕様）を横断的に説明します。各テストが何を検証しているか、期待される結果は何か、どのファイルにあるかを要点でまとめています。

- 実行方法: `cargo test`
- 範囲: Rust 側のユニット/統合テスト全般 + Escher-Scala の `tests.json` 仕様（Scala ランタイムが読み込む例群）

---

## Escher-Scala の JSON 仕様

- 目的: Escher-Scala の合成器が読み込む入出力仕様サンプル集。`inputTypes`・`returnType` と `examples` を通して合成タスクを定義。
- 位置: `Escher-Scala/src/main/resources/escher/tests.json`
- 主な仕様（関数名の例）:
  - `append`/`append-f`/`append-g`、`prepend-arg`/`prepend-g`、`removeLast-f`/`removeLast-g`、`removeFirst-f`/`removeFirst-g`、`concat-f`/`concat-g`、`removeAt-f` など
  - 各 `examples` は `input` と `output` のペア。`List[Int]` は整数配列で表し、ポインタ終端は `-1` をセンチネルとして使用
- テスト性質: Rust 側では直接は実行されず、Scala 実行時に読み込まれる仕様データ（サンプル群）
  - 参照: `Escher-Scala/src/main/resources/escher/tests.json:1`

---

## Rust: ソース内ユニットテスト

- `src/list_env.rs:311`
  - 対象: `ListEnvironment`
  - 目的: VisGraph からの初期環境構築、`addNode`/`addEdge` の適用、`field_lists`/インデックスの整合性
  - 主なテスト:
    - `test_initial_environment`: `next`/`value` フィールドが正しく構成される
    - `test_add_node_operation`: 新規ノード追加で全フィールド配列が拡張される
    - `test_add_edge_operation`: オブジェクト/リテラル両参照の `addEdge` 反映を検証

- `src/escher_bridge.rs:338`
  - 対象: `build_escher_spec`
  - 目的: フィールドの動的解析、BFS ローカルインデクシング、`examples` 生成、JSON 整形
  - 主なテスト:
    - `test_build_spec_simple_list`: 生成JSONに名前/型情報/`examples` が含まれること

- `src/program_analyzer.rs:118`
  - 対象: `ProgramAnalysis::analyze_program`
  - 目的: JavaScript テキストから `object_declarations` と `method_calls` を抽出
  - 主なテスト:
    - `test_program_analysis`: 変数宣言→ID 付与、メソッド呼び出し→レシーバID/メソッド名の対応を検証

---

## Rust: 統合/機能テスト（tests/*.rs）

- `tests/integration_tests.rs:16`
  - 対象: 操作列→共通パターン（関数本体）合成
  - 目的: リスト操作（append/prepend/remove/insertAfter 等）から AST 合成できるかを検証
  - 主なテスト:
    - `test_append_method_synthesis`: Node 作成/`val`/`next` 代入/ホール抽出を検証
    - `test_prepend_method_synthesis`: 先頭挿入の合成を検証（新規 Node 生成、返り値扱いなど）
    - `test_remove_last_method_synthesis`: 削除系操作の合成可否・基本構造
    - `test_remove_first_method_synthesis`: 参照編集中心のケース（合成の可否を寛容に扱う）
    - `test_insert_after_method_synthesis`: 新規 Node 作成と `next` 参照の再配線の存在
    - `test_with_actual_json_files`: `tests/data/operations.json` を読み、パターン抽出の健全性を確認
    - `test_multiple_method_differences`: append/prepend がいずれも合成可能であること

- `tests/unify_ops_tests.rs:83`
  - 対象: `unify_operation_graphs`
  - 目的: 2つの操作グラフのユニフィケーション（構造同型 + 属性一致）で `common_*`/`diff_*` に正しく仕分けできるか
  - 主なテスト: `test_unify_append`/`test_unify_add2`/`test_unify_prepend`/`test_unify_insert_after`/`test_unify_concat`/`test_unify_set`
    - 期待: 各 `expected_common_*`/`expected_diff_*` と結果が一致

- `tests/isomorphism_tests.rs:84`
  - 対象: `unify_isomorphic_graphs`（最大共通部分写像ベース）
  - 目的: 操作グラフ同士の同型性判定 + 属性整合で `common/diff` を正しく返す
  - 主なテスト: append/add2/prepend/insert_after/concat/set の各ケース
    - 期待: 期待ベクトルと完全一致（不一致や非同型時は panic させる実装）

- `tests/graph_isomorphism_tests.rs:23`
  - 対象: `match_graphs_with_isomorphism` および `match_graphs_with_flexible_structure`
  - 目的: 厳密/柔軟な同型判定の双方で、多様な操作列が同一構造かを検証
  - 主なテスト: append/prepend/removeLast/removeFirst/removeAt/insertAfter/concat/set/add2Nodes など（差分ケースも含む）

- `tests/max_common_subgraph_tests.rs:49`
  - 対象: `maximum_common_subgraph`
  - 目的: 2グラフの最大共通部分写像（対応付け）と差分の抽出
  - 主なテスト:
    - `test_max_common_subgraph_mapping_and_diff`: 対応関係と差分集合が期待どおり
    - `test_max_common_subgraph_difference`: メソッド間（例: prepend vs insertAfter）の相違が表現される

- `tests/program_analysis_integration_tests.rs:31`
  - 対象: `ProgramAnalysis` + 動的環境構築
  - 目的: JS プログラム解析→`this` マッピング/レシーバ解決、解析堅牢性
  - 主なテスト: 単一/複数レシーバ、現実的なリンクリスト例、空/不正プログラムの取扱い

- `tests/receiver_context_tests.rs:8`
  - 対象: `MemoEnv`（レシーバ/スコープ/アクセスパス）と `parser`
  - 目的: レシーバを `this` として扱う文脈管理、アクセスパス解決、スコープ分離
  - 主なテスト: レシーバ管理/動的 `this`/プロパティ経路/複数スコープ/パーサ連携/エラーハンドリング

- `tests/operation_analysis_tests.rs:3`
  - 対象: `operation_analyzer` 系
  - 目的: 差分点抽出/可視化/同一列の評価
  - 主なテスト: 基本/詳細/同一列（差分0）

- `tests/comprehensive_tests.rs:47`
  - 対象: 合成パイプライン全体（IR 化/正規順序/パターン抽出）
  - 目的: tex（`analysys.tex`）復元に基づく網羅テスト
  - 主なテスト:
    - `test_append_comprehensive`: Node 作成・`val`/`next` 代入・ホール抽出
    - `test_prepend_comprehensive`: 先頭挿入・返り値扱い（ログ検査中心）
    - `test_insert_after_comprehensive`: 再配線の回数/存在確認
    - `test_ir_conversion_comprehensive`: IR 変換と種類別件数、正規順序のサイズ
    - `test_pattern_matching_detailed`: 近縁操作列から共通パターン抽出
    - `test_edge_cases`: 空/不正/単一操作ケースの堅牢性
    - `test_synthesis_statistics`: 実行ヘルプの出力（統計表示）

- `tests/method_specific_tests.rs:37`
  - 対象: メソッド別（removeVal/removeAt/concat/set/swap/clear/reverse 等）合成
  - 目的: 各操作の本質的な構造（`next`/`val` の代入、参照編集、性能など）に着目
  - 主なテスト: 参照再配線の有無、期待件数（例: swap で 2 回以上の `val` 変更）、大規模操作（100件）を 10 秒以内で処理

- `tests/improved_example_tests.rs:9`
  - 対象: append の厳密検証
  - 目的: `analysys.tex` の期待構造（Node 作成 → `val` 設定 → `next` 接続、ホール化）と抽出パターンの整合性
  - 主なテスト:
    - `test_append_method_correctness`: 0/3 の値差異がホール化されること

- `tests/synthesis_api_tests.rs:61`（非同期テストを含む）
  - 対象: HTTP ハンドラ `handle_synthesis`、および Kanon 由来データの適用
  - 目的: ペイロード→合成→HTTP 200 と期待 JSON を返すこと、`ListEnvironment` の状態遷移
  - 主なテスト:
    - `test_synthesis_from_kanon_payload`（tokio）: 200 OK、`individual_codes` 件数・内容、`common_pattern`/`holes` の期待
    - `test_kanon_operations_applied`: VisGraph 初期化→操作適用→`val`/`next`/インデクスの検証

- 付随データ
  - `tests/data/operations.json:1`: 実データ例。複数のメソッド呼び出しと操作ログを含み、統合テストで読み込まれる場合あり

---

## 期待される挙動の共通パターン

- ノード追加系（append/prepend/insertAfter など）
  - 新規 Node 生成（`new Node()` 相当）
  - `val` プロパティ設定（値差異はホール化）
  - `next` の接続/再配線
- 参照編集/削除系（remove/removeAt/removeLast/removeFirst/clear/reverse）
  - `next` の `undefined`/`null`/再配線、`deleteNode` 相当の取り扱い
  - 複雑ケースは合成 `None` を許容するテストも含む（堅牢性重視）
- グラフ同型/最大共通部分
  - 構造（ノード種別/エッジ種別/変数 op）と属性（ラベル/ID）の一致で `common`/`diff` を厳密検証
- 動的スコープ/レシーバ（`this`）
  - `ProgramAnalysis` と `MemoEnv` によりレシーバ ID を `this` に動的割付
  - アクセスパスの解決（例: `this.value`, `this.next`）

---

## 実行メモ

- すべての Rust テスト実行: `cargo test`
- HTTP/非同期テストを含むため、ネットワークは使わずローカルで完結
- Escher-Scala の `tests.json` は Scala ランタイムが参照（Rust テストでは読み込まない）

---

## 参考

- 仕様・ブリッジ:
  - `docs/REFSYN_TO_ESCHER.md`
  - `docs/SYSTEM_FLOW_AND_COMMANDS.md`
- ブリッジ実装（BFS ローカルインデクシング/動的フィールド検出）:
  - `src/escher_bridge.rs`
- 環境表現と操作適用:
  - `src/list_env.rs`
- レシーバコンテキスト/スコープ:
  - `src/env.rs`

以上。詳細が必要なテストや、追加の観点（カバレッジ/実行時間/失敗時の典型ログなど）があればお知らせください。必要に応じて追補します。

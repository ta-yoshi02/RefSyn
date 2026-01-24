# RefSyn → Escher-Scala 実行フローとコマンド集

このドキュメントは、RefSyn で「unification → 差分特定 → List/Int 環境へ整形 → Escher-Scala 用 JSON 生成」までを行い、Escher-Scala で合成を動かすための最短手順をまとめたものです。

## システムの流れ（概要）
- 共通/差分抽出: 2 本の操作列（Kanon の操作ログ）を unification し、共通部分と差分部分を抽出
- 環境構築: 共通部分までの操作を初期グラフに適用し、差分直前の List 環境（`List[Int]`/`List[Ptr]`/`Int`/`Ptr` セット）を構築
- JSON 整形: 動的に検出したフィールドで BFS ローカルインデクシングを行い、`tests.json`（Escher 仕様）を生成
- 合成実行: Escher-Scala が `tests.json` を読み込み、合成を実行

実装の要点:
- 動的フィールド検出（値/ポインタ）: 特定のフィールド名をハードコーディングしない
- ルート検出: `__Variable-this` を優先し、なければ `__Variable-<name>` → `<name>` エッジをルートとみなし、ポインタフィールドのみで BFS
- 引数生成: `__Variable-<name>` の参照先を引数化し、`this` を先頭に並べる
- nullPtr は JSON `null`、`Int` の欠損は `-1`

関連ファイル:
- `src/escher_bridge.rs`（Escher 用 JSON 生成）
- `src/list_env.rs`（List 環境構築/更新）
- `src/lib.rs:596` 付近（unification エントリ関数）
- サンプル: `examples/generate_escher_json.rs`

## 前提条件
- Rust ツールチェーン（`cargo`）
- Java 8 + sbt（Escher-Scala 実行用）

## クイックスタート（JSON 生成まで）
- リポジトリ直下へ移動
  - `cd /Users/taku/workspace/kenkyu/proj-ref/refsyn`
- サンプルの JSON 生成（`tests.json` を自動出力）
  - `cargo run --example generate_escher_json`
- 出力確認
  - `ls -la Escher-Scala/src/main/resources/escher/tests.json`
  - `sed -n '1,120p' Escher-Scala/src/main/resources/escher/tests.json`

この例は内部で以下を実行します：
- `analyze_operations_with_unification` により共通部分を抽出
- 共通操作を初期 `VisGraph` に順に適用し、差分直前の `ListEnvironment` を構築
- `build_escher_spec` で `input`/`inputTypes`/`examples` を組み立て
- `specs_to_json` で複数仕様を配列化し、`tests.json` へ書き込み

## Escher-Scala の実行（手動）
- Java 8 の環境変数（macOS 例）
  - `export JAVA_HOME=$(/usr/libexec/java_home -v1.8)`
  - `export PATH=$JAVA_HOME/bin:$PATH`
- 実行
  - `cd Escher-Scala`
  - `sbt "runMain escher.Test"`

Escher-Scala は `src/main/resources/escher/tests.json` を読み込みます。合成が成功すると `--- Synthesis succeeded ---` が表示されます。

## 入力を差し替える（カスタマイズ）
- 編集ファイル: `examples/generate_escher_json.rs`
  - `vis_graph`: Kanon の環境（ノード/エッジ）
  - `ops_a`, `ops_b`: 比較する 2 本の操作列（JSON オブジェクト配列）
  - `arguments`/`arg_types`/`receiver_arg_index`/`output`: 各ケースの引数・型・受け取り位置・期待出力
- `build_escher_spec("<関数名>", "<戻り値型>", &cases, None)` で `EscherSpec` を生成
- 複数仕様は `Vec<EscherSpec>` にまとめて `specs_to_json` → `write_spec_to_file`
- 再生成
  - `cargo run --example generate_escher_json`

## よくある注意点
- ルート検出には `__Variable-<name>` → `<name>` エッジが必要（`__Variable-this` があれば優先）
- 引数順は `this` を先頭にし、残りは変数名の昇順
- ケース間で検出されたフィールド集合が一致している必要あり（ラベル揺れに注意）
- 値ノードが文字列数値（例: `"2"`）でも `Int` として解釈される
- ポインタ終端/未接続は `null`（nullPtr）

## 参考ドキュメント
- `docs/REFSYN_TO_ESCHER.md`: 仕様詳細とコード断片
- `architecture.md`: フローの背景と全体像

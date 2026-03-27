# RefSyn → Escher 実行フローとコマンド集

このドキュメントは、RefSyn で「unification → 差分特定 → List/Int 環境へ整形 → Escher JSON 生成」までを行い、既定 backend の `escher-ts` で合成を動かすための最短手順をまとめたものです。

## システムの流れ（概要）
- 共通/差分抽出: 2 本の操作列（Kanon の操作ログ）を unification し、共通部分と差分部分を抽出
- 環境構築: 共通部分までの操作を初期グラフに適用し、差分直前の List 環境（`List[Int]`/`List[Ptr]`/`Int`/`Ptr` セット）を構築
- JSON 整形: 動的に検出したフィールドで BFS ローカルインデクシングを行い、`tests.json`（Escher 仕様）を生成
- 合成実行: `scripts/run_escher.js` が backend を選んで合成を実行

実装の要点:
- 動的フィールド検出（値/ポインタ）: 特定のフィールド名をハードコーディングしない
- ルート検出: `__Variable-this` を優先し、なければ `__Variable-<name>` → `<name>` エッジをルートとみなし、ポインタフィールドのみで BFS
- 引数生成: `__Variable-<name>` の参照先を引数化し、`this` を先頭に並べる
- nullPtr は JSON `null`、`Int` の欠損は `-1`

関連ファイル:
- `src/escher_bridge.rs`（legacy spec / native task JSON 生成）
- `src/list_env.rs`（List 環境構築/更新）
- `src/lib.rs:596` 付近（unification エントリ関数）
- サンプル: `examples/generate_escher_json.rs`
- backend: `external/escher-ts`, `scripts/run_escher.js`, `scripts/run_escher_scala.js`

## 前提条件
- Rust ツールチェーン（`cargo`）
- Node.js 20 以上
- `pnpm`（`external/escher-ts` の build 用）
- Java 8 + sbt は `ESCHER_BACKEND=scala` fallback を使う時だけ必要

## クイックスタート（JSON 生成まで）
- リポジトリ直下へ移動
  - `cd /Users/taku/workspace/kenkyu/proj-ref/refsyn`
- escher-ts をビルド
  - `cd external/escher-ts && pnpm install --frozen-lockfile && pnpm build && cd ../..`
- サンプルの JSON 生成（`tests.json` を自動出力）
  - `cargo run --example generate_escher_json`
- 出力確認
  - `ls -la target/escher/ts/tests.json`
  - `sed -n '1,120p' target/escher/ts/tests.json`

この例は内部で以下を実行します：
- `analyze_operations_with_unification` により共通部分を抽出
- 共通操作を初期 `VisGraph` に順に適用し、差分直前の `ListEnvironment` を構築
- `build_escher_spec` で legacy spec を組み立て、`build_escher_task_spec` で native task に変換
- `tasks_to_json` で task 配列を文字列化し、`target/escher` へ書き込み

## Escher の実行（手動）
- 既定の TS backend
  - `node scripts/run_escher.js --file target/escher/ts/tests.json --quiet`
- Scala fallback
  - `export ESCHER_BACKEND=scala`
  - `node scripts/run_escher.js --file target/escher/scala/tests.json --quiet`

TS backend は `external/escher-ts/dist/refsyn.js` を使います。Scala fallback は `scripts/run_escher_scala.js` 経由で `Escher-Scala/target/scala-2.12/escher-scala-opt.js` を呼びます。

## 入力を差し替える（カスタマイズ）
- 編集ファイル: `examples/generate_escher_json.rs`
  - `vis_graph`: Kanon の環境（ノード/エッジ）
  - `ops_a`, `ops_b`: 比較する 2 本の操作列（JSON オブジェクト配列）
  - `arguments`/`arg_types`/`receiver_arg_index`/`output`: 各ケースの引数・型・受け取り位置・期待出力
- `build_escher_spec("<関数名>", "<戻り値型>", &cases, None)` で legacy `EscherSpec` を生成
- `derive_spec_meta` と `build_escher_task_spec` を通して native task 化
- `tasks_to_json` → `write_spec_to_file`
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

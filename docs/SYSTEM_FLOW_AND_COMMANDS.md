# RefSyn → Escher 実行フローとコマンド集

現実装を起動するための手順であり、論文の仕様を定義する文書ではない。
修正の優先順位は [残作業](PAPER_ALIGNMENT_WORKLIST.md) を参照する。

RefSynの合成経路の概要と、手元で動かすためのコマンドをまとめる。モジュール構成と境界は `architecture.md` を参照。

## システムの流れ（概要）
- 記録の読込み：Kanonが送る型付き呼出し前状態（`validation.version = 3`）と編集列を読む（`src/demonstration.rs`）
- 正規化：編集をMOLDの書込みにし、実演ごとの操作グラフを作る（`src/operation_graph.rs`）
- 対応付け：第1実演から各実演への全体同型を求める（`src/alignment.rs`）
- 雛形：穴付き雛形と文列、本体メソッドのJSを作る（`src/template.rs`）
- PBE問題：穴ごとの入力環境をescher-ts task JSONにする（`src/pbe_problem.rs`、型とJSONは `src/escher_bridge.rs`）
- 合成と翻訳：`scripts/run_escher.js` が `runtime/refsyn-escher-adapter.mjs` 経由で各穴を合成し、補助メソッドのJSにする
- 検証：`runtime/whole-method-validation.mjs` が各実演を再生し、通った場合だけ適用用コードを返す

## 前提条件
- Rust ツールチェーン（`cargo`）
- Node.js（CIで検証している版と最低対応版は [docs/TESTS.md](TESTS.md#web-tests-from-a-clean-checkout) を参照）
- `pnpm`（`external/escher-ts` の build 用）
- `npm`（`web/` デモの install/test/build 用）

## Web デモのセットアップ
- `web/` は独立した npm パッケージとして管理する
- クリーンなcheckoutからテストを通す手順（サブモジュール、escher-ts、WASMターゲット、wasm-bindgen-cli、WASMビルド）は [docs/TESTS.md](TESTS.md#web-tests-from-a-clean-checkout) を参照
- 本番 build：`cd web && npm run build`

## クイックスタート
- escher-ts をビルド
  - `cd external/escher-ts && pnpm install --frozen-lockfile && pnpm build && cd ../..`
- nativeサーバを起動（`http://127.0.0.1:3030/synthesize`）
  - `cargo run`
- 保存したリクエストを合成する
  - `cargo run --example run_payload -- <payload.json>`
- 生成された task JSON は `target/escher/ts/` に保存され、手動でも実行できる
  - `node scripts/run_escher.js --file target/escher/ts/<name>.json --quiet`

## よくある注意点
- リクエストには、Kanonが記録した型付き呼出し前状態と各クラスの `new C()` の状態が必要。旧形式（表示グラフだけ）のpayloadは合成しない
- 全実演は同じクラスの同じメソッドを対象とし、操作グラフが互いに同型である必要がある
- PBEで扱えるのは、単一クラスの参照（nullを含む）、整数、真偽値（#28のマージ後、`af76b2e` 以降）。文字列と複数クラスは合成しない
- `runtime/refsyn-escher-adapter.mjs` の走査helperのうち、`last_ptr`、`penultimateRef`、`findByValueRef` は、同じobjectへの再訪を検出して失敗する。`nthNextRef` は再訪を検出せず、指定した歩数で止まって値を返す（escher-tsの評価と同じ）

## 参考ドキュメント
- [文書の入口](README.md)：現在使う資料と隔離した履歴
- [残作業](PAPER_ALIGNMENT_WORKLIST.md)：論文本文との照合と完了条件
- [実装構成](../architecture.md)：全体像とモジュールの責務

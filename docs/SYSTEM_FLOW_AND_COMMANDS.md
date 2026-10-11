# RefSyn → Escher 実行フローとコマンド集

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
- Node.js 20 以上
- `pnpm`（`external/escher-ts` の build 用）
- `npm`（`web/` デモの install/test/build 用）

## Web デモのセットアップ
- `web/` は独立した npm パッケージとして管理する
- 依存 install：`cd web && npm install`
- テスト実行：`cd web && npm test`
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
- escher-tsで表現できるのは整数と参照（nullを含む）。文字列や真偽値を出力する穴は合成しない
- `runtime/refsyn-escher-adapter.mjs` が生成するポインタ走査 helper は acyclic なリストを前提にしており、循環リスト入力は未サポート

## 参考ドキュメント
- `docs/REFSYN_TO_ESCHER.md`：穴の入力環境とtask JSONの符号化
- `docs/KANON_CODEX_VERIFICATION.md`：Kanon UI を操作して `set test` から合成まで検証する時の手順
- `architecture.md`：全体像とモジュールの責務

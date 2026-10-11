# Architecture Overview

RefSynは、Kanonで記録した実演から、論文（MOLD）3〜4章の手順でメソッドを合成する。
実演を操作グラフに正規化し、全体同型で対応付けて穴付き雛形を作り、各穴をescher-tsのPBEで埋め、全実演での再生に通ったコードだけをKanonへ返す。

## Runtime Split

- **Rust core (`synthesize_core`)**：`SynthesisRequest` から、合成した本体メソッド（`composed_method_code`）、穴ごとのescher-ts task JSON、補助メソッドの期待出力（`hole_checks`）を返す。HTTP、ファイル出力、Nodeプロセス、環境変数は扱わない。
- **Native server adapter (`handle_synthesis`)**：task JSONを保存し、escher-tsを子プロセスで実行して補助メソッドを得たうえで、全体検証（`validation.rs` → `scripts/validate_method.mjs`）を通ったときだけ適用用コードを返す。
- **Browser worker runtime**：`synthesize_browser`（wasm）でRust coreを呼び、同じ `runtime/refsyn-escher-adapter.mjs` で合成し、Workerで同じ検証器を実行する。

## 処理の流れとモジュール

```text
Kanon（型付き呼出し前状態・new C()の状態・編集列）
  → demonstration    記録の読込み（IDはKanonの記録どおり）
  → operation_graph  編集をMOLDの書込みへ正規化（deleteEdgeは共有null頂点への書込み）
  → alignment        第1実演から各実演への全体同型（論文4.2）
  → template         穴付き雛形、穴の順序、文列（生成→穴→書込み→return）とJSへの変換
  → pbe_problem      穴ごとの入力環境（呼出し前状態＋生成物＋先行穴）とescher-ts task
  → escher-ts        各穴の式 → runtime adapterでJSの補助メソッドへ翻訳
  → validation       各実演の呼出し前状態から実行し、MOLDの意味で作った期待状態と照合
  → Kanon            検証済みの変更だけをソースへ適用
```

`pipeline.rs` はescher-ts実行前までの組立てを、`lib.rs` は公開型とcore／native／wasmの入口を受け持つ。

| モジュール | 入力 → 出力 | 失敗条件 |
| --- | --- | --- |
| `demonstration` | `SynthesisRequest`（`validation.version = 3`）→ `Recording`（クラスと `new C()` の状態、実演ごとのヒープ、引数、編集） | 型付き状態がない、不明なID・クラス・参照、未対応の編集 |
| `operation_graph` | 実演 → 頂点宣言列、書込み列、返り値の頂点 | 記録された旧対象と再生中の状態の不一致、値fieldへのdeleteEdge、辺があるfieldへのaddEdge、範囲外のconstructorを持つクラスの生成 |
| `alignment` | 操作グラフ列 → 頂点の全単射の列 | 非同型（型・クラス・既存／新規・辺の向き・フィールド・重複数のいずれかが保存されない） |
| `template` | グラフと全単射 → 生成文、穴（型と実演ごとの出力）、書込み、返り値 | — |
| `pbe_problem` | 記録と雛形 → 穴ごとのtask、`hole_checks` | escher-tsが表現できない値（文字列、真偽値、整数以外、複数クラス、未定義のpointer field） |

設計判断と受入条件の詳細は `docs/PAPER_ALIGNED_REQUIREMENTS.md`、境界のデータと検証の契約は `docs/SYNTHESIS_TEMPLATE_CONTRACT.md` にある。

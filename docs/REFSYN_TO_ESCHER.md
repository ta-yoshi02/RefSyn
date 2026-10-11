# 穴の入力環境と escher-ts task JSON

RefSynは雛形の穴ごとに一つのPBE問題を作り、escher-tsのtask JSONにする（`src/pbe_problem.rs`）。
構造体とJSONの出力、escher-tsの起動は `src/escher_bridge.rs` が受け持つ。

## Runtime boundaries

- Rust core の入口は `synthesize_core(SynthesisRequest, SynthesisCoreOptions)`。task JSON の生成までを担当し、HTTP や subprocess は扱わない。
- native server (`handle_synthesis`) は core の返した JSON を保存し、`scripts/run_escher.js` 経由で `runtime/refsyn-escher-adapter.mjs` を実行する。
- browser runtime は `synthesize_browser` wasm export を worker 内から呼び、返された task JSON を同じ `runtime/refsyn-escher-adapter.mjs` へ渡す。

## 穴の入力環境（論文3.2節）

穴 `k` の第 `i` 実演の入力は、次を合わせた環境である。

1. 第 `i` 実演の呼出し前状態（Kanonが記録した型付きsnapshot）と、仮引数への実引数の束縛
2. 雛形が生成する全オブジェクト。フィールドはKanonが記録した `new C()` 直後の状態
3. 穴 `0..k-1` の出力例

書込み文はどの穴の入力にも反映しない。穴 `k` 自身と後続の穴の値も含めない。
先行穴の値には、操作列と頂点対応から得た出力例 `y_j` を束縛する。PBEの成功条件は全実演で `eval(e_j, E_j) = y_j` なので、後続穴の入力例を作るのに先行穴の合成完了を待つ必要はなく、全穴の問題を同時に作る。
生成メソッドの実行時には先行式の評価結果が束縛される。全体検証で各補助メソッドの実際の戻り値が `y_j` と一致することを確かめ（`hole_checks`）、一致しなければ採用しない。

## task JSON の構成

- `name`：補助メソッド名（`<method>_h<k>`）。`refsynMeta.jsMethodName` にも同じ名前を入れる
- `classes`：受信物のクラス一つ。各フィールドは整数フィールドなら `Ref[Int]`、参照フィールドなら `Ref[Object[C]]`
- `signature.args`：メソッドの仮引数、生成物の局所変数（`created<j>`）、先行穴の局所変数（`hole<j>`）の順。整数は `Int`、参照は `Ref[Object[C]]`
- `signature.autoExpandClassSignature`：`thisRef`、`nodeHeap`、フィールドごとのヒープ
- `examples`：各実演について `[thisRef, nodeHeap, フィールドヒープ（フィールド名順）..., 引数..., 生成物..., 先行穴...]` と出力
- `components`：整数と参照の有無に応じたライブラリ部品

## 値の符号化

| 値 | 符号化 |
| --- | --- |
| オブジェクト | ヒープ上の番号（呼出し前状態の順、続けて生成物）を `{"ref": n}` で表す |
| null | `{"ref": -1}`。escher-tsはnullを参照としてだけ表せるため、nullしか現れないフィールドは参照フィールドとする |
| 整数 | JSONの数値 |
| undefined、不在（整数フィールド） | `{"error": true}`。読み出しても有効な値にならない |

文字列、真偽値、整数以外の数、複数クラスのヒープ、undefinedや不在になりうる参照フィールドは、置き換えずに失敗とする。

## 配置場所

- `src/pbe_problem.rs`：穴ごとの環境とtaskの構築
- `src/escher_bridge.rs`：task構造体、JSON出力、部品の選択、escher-tsの実行
- `scripts/run_escher.js`：escher-ts 実行用ランナー
- `runtime/refsyn-escher-adapter.mjs`：合成とJSへの翻訳
- `external/escher-ts`：escher-ts 実装

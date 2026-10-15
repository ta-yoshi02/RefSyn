# 論文との適合確認表

確認日：2026-10-14。
RefSynの基準：main `88927bcc99003d4f4836f9e518056ae64871675c`（#32のマージ後）。固定するKanonは `60fa5c8`、escher-tsは `4e7c4bc`。
論文の基準：`949ffd2cfffd63ea39c45e03fdefcdaa831c1f63`（`/Users/taku/workspace/paint2026-yoshio`）。
本書は、論文の各規則について、実装、テスト、実操作の証拠、制限を対応付けた確認表である。形式的な適合証明ではない。
論文本文の修正案は `docs/drafts/2026-10-14-paper-text-draft.md` にあり、本文は編集していない。

## 仕様の参照範囲

`ja/main_ja.tex` が読み込む本文のうち、1〜5章を手法と実装の仕様、6章（`evaluation.tex`）を評価条件の照合対象とする。
`main_ja.tex` はappendixを読み込まない。コメントアウトされた旧説明は根拠にしない。
今回読んだ範囲：`method.tex` 146〜260行、`operation-graphs.tex` の構文（19〜53行）と雛形・コード生成（130〜180行）、`implementation.tex` 150〜188行、`evaluation.tex` 全文。

## 証拠の所在

| 証拠 | 場所 | 状態 |
| --- | --- | --- |
| 主要5ケースのKanonでの新規記録（`b42d9ca`）、予備実験と本測定、baselineとの問題同一性の検査 | `docs/evaluation_cases/reruns/2026-10-14-fresh-recording/` | main（#33でマージ、`aaf69d9`） |
| `88927bc` での再確認（task・部品・探索設定・backendの一致、5ケースでのC1・C5、`this.val = 0` のinsert再記録） | `docs/evaluation_cases/reruns/2026-10-14-recheck-88927bc/` | #34（本書と同じPR） |
| #29のC1・C5の検査ログ | `docs/evaluation_cases/reruns/2026-10-14-pr29-checks/` | main |
| `af76b2e` のinsert記録（別セッション） | `docs/evaluation_cases/reruns/2026-10-14-recording/` | main。`b42d9ca` 以降の結果ではない。記録中のサーバ（PID 90770）は、本表の記録を取ったセッションが利用者の承認を得て停止した（`2026-10-14-fresh-recording/logs/FOREIGN_REQUESTS.md`） |
| ユーザーが実演したinsertの画面・terminalの取得と、その独立確認（別セッション、`af76b2e`） | `docs/evaluation_cases/reruns/2026-10-14-insert-live/`、`2026-10-14-insert-live-recheck/` | main。本表の根拠には使っていない |

## 適合確認表

### A. 論文の規則

| # | 論文の規則（箇所） | 実装 | テスト | 実操作の証拠 | 制限・差 |
| --- | --- | --- | --- | --- | --- |
| A1 | 操作グラフ：頂点 `Node(C)`・`addNode(C)`・定数・`null`、書込み `editEdge(m.f,n)` の列（`operation-graphs.tex` 19〜38行） | `src/operation_graph.rs`、`src/demonstration.rs` | `edits_become_writes_in_order_with_duplicates`、`delete_edges_write_one_shared_null_vertex`、`inconsistent_or_unsupported_edits_fail`（`tests/paper_pipeline_tests.rs`） | 5ケースの新規記録：全呼出しの操作列が受理され、合成まで進んだ | Kanonの記録形式（`addEdge`、`editEdgeReference`、`deleteEdge`）を、旧参照先と呼出し前状態の整合を確かめてから `editEdge` に正規化する。この整合検査は論文の規則ではない（B2） |
| A2 | 全体同型による対応付け。既存物・生成物・定数の型・`null` を区別し、辺の向き・field・重複を保つ（`operation-graphs.tex` 72〜81行） | `src/alignment.rs`、`Vertex::corresponds` | `insert_demonstrations_correspond_as_in_the_paper`、`correspondence_preserves_kind_class_direction_and_multiplicity` | 5ケース：異なる入力順の実演（insertの第3実演、appendの第2実演など）が対応した | 同型が複数あれば一つを採る（論文の記述どおり）。ユーザが選ぶ機構はない |
| A3 | 頂点ごとの穴、同じ頂点の出現で共有（`operation-graphs.tex` 130〜145行） | `src/template.rs` | `insert_template_has_seven_statements_in_the_paper_order`、`creations_precede_holes_shared_vertices_bind_once_and_writes_keep_order_and_multiplicity` | 5ケースの生成メソッド（各 `response.json`） | 穴の束縛順は第1実演の辺列での初出順。論文の例の `<value>`、`<prev>`、`<next>` の順と同じになるのは辺列がその順のときだけ。今回のinsert記録では後続（`i+1`個先）が `hole1`、更新対象が `hole2` になった |
| A4 | コード生成：生成文 → 穴の束縛文 → 代入文、各穴を1回評価（`operation-graphs.tex` 165〜180行） | `Template::statements`、`render_method` | A3と同じ | 5ケースの反映後ソース（`source_after.js`） | なし |
| A5 | 後続の穴の入力に先行穴の結果を加える（`method.tex` 194〜201行） | `src/pbe_problem.rs` は先行穴の**出力例**を入力に加え、`src/pipeline.rs` は合成前に全taskを作る | C1〜C5（下の表） | 下の表 | 論文の文言（「評価結果」）と実装（「出力例」）が異なる。一致の条件と検査範囲を本文に書く案を作成済み |
| A6 | 生成は `new C()`（`operation-graphs.tex` 171行） | 操作グラフ、雛形、Kanonの生成直後snapshot、全体検証 | `creating_a_class_outside_the_supported_constructors_fails`、#32の各テスト | `this.val = 0` を持つinsertのKanon再記録（`2026-10-14-recheck-88927bc/kanon_insert_val0_b9ad0a8/`）：passed 3/3、反映後の再実行で3呼出しとも `testPassed` | 対応するのは、引数なしの `new C()` が例外を出さず、基本型・`null`・`undefined` のデータプロパティだけを設定するクラス（B3） |

### B. 実装上の拡張・制限・採用条件（論文本文にないもの）

| # | 内容 | 種類 | 実装 | テスト | 証拠・制限 |
| --- | --- | --- | --- | --- | --- |
| B1 | 返り値：特殊変数 `return` への参照設定を末尾の `return` 文にする | 拡張（論文は脚注で言及） | `OperationGraph::ret`、`Template` | `http_validates_demonstrated_return_values`（`tests/whole_method_validation_tests.rs`） | prependの新規記録で passed 2/2、再実行で `testPassed` |
| B2 | 記録の整合検査：`addEdge` は空・`null`・`undefined` のfield、または**この実演で生成し、まだ書き込んでいないfield**（`new C()` の既定値を上書き）にだけ許す。`editEdgeReference`・`deleteEdge` は記録された旧参照先と状態の一致を要求 | 採用条件（記録の整合性） | `src/operation_graph.rs`（`unwritten_created`）、`runtime/whole-method-validation.mjs` | `add_edge_replaces_an_unwritten_constructor_default_of_a_created_object`、`creation_and_write_history_do_not_carry_over_to_the_next_demonstration`、検証器の同名テスト | #32で既定値の上書きを許可した。修正前は `this.val = 0` のクラスを合成前に拒否していた（`2026-10-14-fresh-recording/insert_general/attempt1_constructor_val0/`）。同じ要求は `88927bc` で passed 3/3 |
| B3 | `new C()` の条件：引数なしで実行し、データプロパティは基本型・`null`・`undefined` だけ | 実装制限 | Kanon `captureInstance`、`AddObject` | `creating_a_class_outside_the_supported_constructors_fails` | 仮引数の宣言は拒否理由にならない（引数なしの実行で `undefined` になる） |
| B4 | 型：整数・真偽値・単一クラスの参照（`null` を含む）。文字列、整数以外の数、複数クラスは拒否 | 実装制限 | `field_kinds`、`task_type`（`src/pbe_problem.rs`） | `an_unrepresentable_or_mixed_kind_input_state_is_rejected_by_the_pbe_encoding`、`tests/boolean_values_tests.rs` | fieldの型は**呼出し前状態と `new C()` の状態だけ**から決め、書込み値の型は見ない。入力状態の型が揃っていれば、既定値と異なる型の書込みも通る（`false`→数値、`0`→真偽値、`0`→参照。#32のレビューで再現し、`88927bc` で再実行して passed 3/3）。入力状態での型の混在（例：呼出し前が数値で既定値が `false`）と、表現できない値（`""`、`0.5`）は拒否 |
| B5 | 生成メソッドの採用前検査 | 採用条件（論文本文に未記載） | `runtime/whole-method-validation.mjs`、`src/validation.rs`、`web/src/lib/browser-runner.ts` | `runtime/whole-method-validation.test.mjs`（35件）、`tests/whole_method_validation_tests.rs` | 5ケースとも passed（全実演）。範囲は C の表の「採用時」の行 |
| B6 | Kanonの入力：引数は整数・`null`・参照だけ。リテラルは数値か文字列として記録 | 実装制限（Kanon） | `normalizeCallArgument`、`literalInputType`（Kanon `src/js/testize.js`） | — | 真偽値の引数や真偽値リテラルの実演は画面から記録できない。真偽値の対応を主張する場合は明記する |

### C. 出力例を束縛する方式の成立条件（A5）

| 条件 | 内容 | 根拠 | 確認の状態（`88927bc`） |
| --- | --- | --- | --- |
| C1 | 解は各実演で出力例を返す | AscendRecの例検査。`web/src/lib/binding-conditions.test.ts` が、WASMの生成したtaskに対し返った式をescher-tsで評価する | CI：paper_insertと真偽値3fixture。主要5ケースでも1回確認（CI外、`2026-10-14-recheck-88927bc/consistency/c1_c5_five_cases.txt`） |
| C2 | 穴kの入力は呼出し前状態、引数、生成直後の生成物、先行穴の出力例だけ | `pbe_problem::build` | `hole_inputs_add_created_objects_and_earlier_holes_but_no_writes`。生成物の既定値が入力に残ることは `hole_inputs_keep_the_constructor_default_of_a_created_object` |
| C3 | 翻訳した補助関数も実際の実行で出力例を返す | 採用前検査が、各補助関数がちょうど1回呼ばれ、戻り値が出力例と `Object.is` で等しいことを照合する | 採用のたびに全実演で検査。5ケースとも passed |
| C4 | 実行順が符号化と同じ（全生成 → 穴 → 書込み） | `Template::statements` | A3・A4のテスト |
| C5 | 補助関数は後続の穴が読む状態を変えない | 下の三段階 | 下の三段階 |

C5（純粋性）の検査範囲：

| 段階 | 確かめること | 対象 | 範囲外 |
| --- | --- | --- | --- |
| 構成上 | 部品は読取り・判定・算術のみ。翻訳（`compileComponent`）はfield読取り、局所変数、算術・比較・条件式、自己再帰の固定された形だけを出力し、それ以外は翻訳エラーで採用しない | すべての合成結果 | 部品実装の純粋性の形式的証明はない |
| 採用時 | 実行後の全オブジェクト・戻り値・生成物の一致、大域・標準プロトタイプ・クラスとプロトタイプの不変、C3 | 合成のたびに全実演 | 補助関数の呼出し間の状態は比べない。元に戻した書込みや、後で上書きされる書込みは検出しない。実演にない入力 |
| テスト時 | Proxyで入力から到達できるobjectへのset・defineProperty・deleteProperty・setPrototypeOf・preventExtensionsを捕捉。元に戻した書込みと上書きされる書込みも検出 | CI：4fixture。主要5ケースは1回（CI外） | 実演にない入力。大域、プロトタイプ、static、局所変数 |

### D. 評価（6章）

| 項目 | 論文（`evaluation.tex`） | 実装側の記録 | 状態 |
| --- | --- | --- | --- |
| 対象と実演数・操作数 | 5メソッド、実演数3/3/2/3/2、操作数2/4/5/5/1 | 新規記録で同じ（`2026-10-14-fresh-recording/stage_summary.json`） | 一致 |
| MOLDのコスト上限 | 14（48行） | 本測定は32 | **不一致**。両手法32・15秒に揃える本文案を作成済み |
| baselineのコスト上限 | 32、15秒（68行） | 32、15秒 | 一致 |
| 反復と成功の定義 | 記載なし | 10回、10回すべてで解けたものを成功 | 本文案で追記 |
| 実装の版 | 「評価中は実装を固定」（50行）、SHAの記載なし | 測定は `b42d9ca`。`88927bc` でtask・部品・探索設定・escher-tsが同一 | 本文案でSHAを追記 |
| 環境 | Darwin 25.6.0 | Darwin 27.0.0、Rust 1.93.0、Node 22.17.0 | 本文案で更新 |
| 結果（○×） | baseline：setAt・prepend・popBackが○、append・insertが× / MOLD：全て○ | 本測定で同じ（10/10と0/10） | 一致 |
| field名 | `Obj`・`f`・`g` を `Node`・`val`・`next` に読み替える注（86〜87行） | 新規記録は `Node`・`val`・`next`。baselineのtaskは旧記録の `f`/`g` で、問題の同一性を `check_baseline_spec.mjs` で確認 | 本文案で注を差し替え |
| 測定区間 | 記載なし | escher-tsの処理（解析・準備・探索）。翻訳・RefSynの解析・全体検証を含まない | 時間を載せる場合に注記（本文案） |

## 残作業（優先順）

1. **本文案の採否を決め、本文へ反映する**（`docs/drafts/2026-10-14-paper-text-draft.md`）。A5の一致条件、C5の三段階、B4・B6の対応範囲、D の評価条件を対象とする。
2. **#34（本書、再確認の証拠、本文案）のマージを判断する**。主要5ケースの記録と測定は#33でmainに入った。
3. **Kanonの未解決の問題**（2026-10-14、Kanon `60fa5c8` で確認）：
   - fallbackで、実演が付け替えた `const` 変数への代入が実行を止める（`eval(__variableName + ' = __object')`）。実演を拒否すべきである。
   - overlayの7件のテストが失敗する（`inferMethodParamNamesForCall` の実装がどのcommitにもない）。Kanon#7のCIは除外している。完成させるか取り下げるかは未決定。
   - 真偽値リテラルを入力できない（B6）。論文の例と評価対象には不要。
4. **公開デモのブラウザバンドルでのUI確認**。公開デモを更新するときに行う。今回はネイティブ経路とローカルのWASM（webテスト）だけを確認した。
5. （任意）主要5ケースのC1・C5検査をCIに加えるかどうか。今回は1回の確認だけである。

## 今回の確認範囲（2026-10-14、`88927bc`）

- 5ケースのtaskを `88927bc` で再生成し、測定時（`b42d9ca`）のtaskとバイト単位で一致することを確認した。baselineのtask、アダプタ、task生成のソース、escher-ts（SHAとdist）、Nodeも一致した。探索に影響する差分はないので再測定していない。
- 1回の整合確認で、成否と得られた式が本測定の全10回と一致した。
- 5ケースでC1・C5の検査を1回実行し、すべて成功した（CI外）。
- #32の修正（`b9ad0a8`、`src` と `runtime` は `88927bc` と同一）で、`this.val = 0` のinsertをKanonで再記録し、passed 3/3、反映後の再実行も成功した。
- 論文本文は読んだだけで編集していない。公開バンドルでのUI確認、Kanonの未解決の問題の修正は行っていない。

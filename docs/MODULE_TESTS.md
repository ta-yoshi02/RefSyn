# モジュールとテストの対応

各モジュールについて、何をどのテストで確かめているか、何を確かめていないかを示す。
テストの実行方法と詳細は [TESTS.md](TESTS.md) を参照する。

## Kanon で実演し直さずに検証する

`tests/recordings/` の各ディレクトリが、Kanon の記録1件と、各段階で期待する結果を持つ。
`cargo test` の `tests/recorded_stage_tests.rs` が全ディレクトリを読み、段階ごとに確かめる。
事例を増やすには、ディレクトリを1つ追加すればよく、コードの変更は要らない。
必要なファイルが欠けたディレクトリは、無視せずテストの失敗にする。
形式は [tests/recordings/README.md](../tests/recordings/README.md) にある。

| ファイル | 内容 |
| --- | --- |
| `request.json` | Kanon が送った request（`docs/evaluation_cases/reruns/` からの写し） |
| `expected.json` | 段階ごとに期待する値。書いた段階と値だけを確かめる |
| `reference.mjs` | 参照実装と入力。参照との比較を期待する場合だけ置く |

| `expected.json` のキー | 確かめる段階の結果 |
| --- | --- |
| `read` | 各実演の型付き実演前状態、引数、編集列 |
| `graphs` | 各実演の操作グラフ（頂点、書込み列、戻り値） |
| `template` | 実演間の対応、各 hole の出力例、hole 付きメソッド |
| `problems` | hole ごとの escher-ts タスクと、照合する期待出力 |
| `validation` | 合成後の全体検証の結果 |
| `comparison` | 参照実装との比較の結果 |

期待値は、結果の同じ位置にすべて含まれていなければならない。
オブジェクトには余分なキーがあってもよく、配列は長さまで一致する必要がある。
満たさない位置は、事例名と経路を付けて全事例分まとめて報告する。
段階が失敗すると、その段階の結果は `{"error": ...}` になり、合成を含む後段は実行しない。
期待値に書いた後段の結果は欠落として報告される。

insert の期待値は、論文の途中結果の一部である。
引数、全実演の書込み列、第1・第3実演の頂点、対応、各 hole の出力例、各 PBE タスクで照合する期待出力を指定している。
PBE タスクの入力は指定しておらず、`paper_pipeline_tests` が論文の fixture で確かめている。
ほかの5事例は、全体検証と参照との比較だけを指定している。

参照は ES module で書き、`classSource`（参照メソッドを含むクラス）と `cases`（入力を作る関数の列）を export する。
例は `tests/recordings/*/reference.mjs` にある。
比較では、呼出し後の状態を全オブジェクトのフィールドの有無と値、参照の同一性と共有関係、戻り値または例外について比べる。
オブジェクトには一定の走査順で名前を付けるため、生成順が参照と違っても正しい解は一致する。

比べるのは、Kanon が全体検証のために記録できる状態（`Kanon/src/js/testize.js` の `captureValidationState`）に限る。
条件は次のとおりで、実行前の入力と、実行後に観測する全オブジェクトに同じく適用する。実行後には、呼出しで切り離された既存オブジェクトも含む。

- オブジェクトは対象クラスのインスタンスである。
- 自身のプロパティはすべて文字列キーで、enumerable、writable、configurable がすべて真のデータプロパティである。getter は呼ばない。
- 値は undefined、null、文字列、真偽値、`-0` 以外の有限の数値、上の条件を満たすオブジェクトへの参照のいずれかである。
- オブジェクトは拡張可能である。これは Kanon が記録しない条件で、比較器だけが追加で課している。

条件を満たさない状態は、一致とは判定しない。入力が満たさない場合と参照側の結果が満たさない場合はエラー、合成解の結果が満たさない場合は理由付きの不一致になる。
対象は symbol、`-0`、NaN、無限大、bigint、配列、Map、関数、通常のオブジェクト、アクセサ、列挙できないフィールドなどである。
Kanon はクラスをコンストラクタ名で判定するが、比較器は prototype の同一性で判定する。

各呼出しは2秒で打ち切り、生成側なら不一致、参照側ならエラーとする。比較のプロセス全体にも120秒の上限がある（打切りの経路は短い上限で回帰テストしている）。
到達できない生成物と、呼出しの途中で一時的に行った変更は観測しない。

## モジュール別

各行の「確かめていること」は、挙げたテストの assertion を読んで確認した範囲である。
「確かめていないこと」は気づいた範囲であり、網羅ではない。

### RefSyn（Rust）

| モジュール | 役割 | 確かめていること（主なテスト） | 確かめていないこと |
| --- | --- | --- | --- |
| `demonstration` | Kanon の記録を型付き状態と編集列として読む | null、undefined、フィールドの欠落を区別する。未知の参照、`validation` の欠落、版の不一致を拒否する（`paper_pipeline_tests` の `reading_*`）。実記録6件を読める（`recorded_stage_tests`） | 真偽値型と記録されたリテラルが真偽値でない場合の拒否 |
| `operation_graph` | 編集を MOLD の書込み列へ正規化する | 書込みの順序と重複を保つ。削除は共有の null 頂点への書込みになる。旧参照の不整合と未対応の操作を拒否する（`paper_pipeline_tests`）。ランダムな操作列の全 prefix で意味を保つ（`operation_graph_randomized_tests`、3,072段階）。実記録の insert の書込み列（`recorded_stage_tests`） | 単一クラス `Cell` 以外のランダム操作列。正準化はしない設計で、異なる操作列が同じグラフになることは確かめない |
| `alignment` | 実演間の対応を選ぶ | 論文の insert と同じ対応を選ぶ。種類、クラス、向き、多重度を保つ。矛盾しない最初の対応を取る。探索が上限の10,000手で打ち切られた場合を矛盾と区別して報告する（`paper_pipeline_tests`） | 上限内で解が見つかる規模の範囲 |
| `template` | hole 付きメソッドを作る | 論文の7文とその順序。生成が hole より前、共有頂点の束縛は1回、書込みは順序と多重度を保つ（`paper_pipeline_tests`）。実記録の insert の hole 出力例が論文と一致する（`recorded_stage_tests`） | 出力した JS を単独では実行しない。実行は全体検証と比較のテストで間接的に行う |
| `pbe_problem` | hole ごとの PBE 問題を作る | 入力に書込み前の状態、生成物、先行 hole を含み、自分と後続 hole を含まない。表現できない入力と種類の混在を拒否する（`paper_pipeline_tests`）。真偽値の型付けと、真偽値と整数・参照・文字列の混在の拒否（`boolean_values_tests`） | — |
| `escher_bridge` | escher-ts を起動する | メモリ設定の解析（単体4件）。実際の起動は、合成を含む全テストで間接的に通る | runner が無い、異常終了するなどの起動失敗の扱い |
| `validation` | Node の全体検証を呼び、通らないコードを消す | 検証を通らない、または検証できないコードを返さない（`whole_method_validation_tests` の `http_*`） | Rust 側の10秒のプロセス時間制限 |
| `lib`（`handle_synthesis`、`synthesize_core`） | 段階をつなぎ応答を作る | 空の request、異なるメソッドの実演、非同型の実演を合成しない。hole ごとに1タスクを渡す（`paper_pipeline_tests`）。保存した5事例と実記録6件の合成と採用（`whole_method_validation_tests`、`recorded_stage_tests`） | 実ポートでの通信 |
| `server` | route と CORS | なし | route、CORS、実ポートでの通信 |
| `models` | 表示グラフの型 | request の読込みを通して間接的に使う | 専用のテスト |
| `error` | `RefSynError` | なし。どこからも使われていない | — |

### JS の実行部（`runtime/`、`scripts/`）

| モジュール | 役割 | 確かめていること（主なテスト） | 確かめていないこと |
| --- | --- | --- | --- |
| `runtime/refsyn-escher-adapter.mjs` | escher-ts の項を JS に翻訳する | 翻訳した整数・真偽値の項が、テストで与えた入力のすべてで escher-ts の評価と一致する。null、undefined、欠落、循環での失敗。演算子の優先順位、`loadInt` が読むヒープ、引数名の扱い（`runtime/refsyn-escher-adapter*.test.mjs`）。翻訳した部品が自分の局所変数以外へ書き込まない（`web/src/lib/binding-conditions.test.ts`） | テストで与えた入力以外での一致 |
| `runtime/whole-method-validation.mjs` | 実演前状態でメソッドを再実行し、期待状態と比べる | 参照の同一性、循環、undefined、constructor の既定値、戻り値。2つ目の実演だけの誤り、helper の出力例との不一致、型の違う同じラベルの値を拒否する。非停止を時間制限で打ち切る（`runtime/whole-method-validation.test.mjs`、23件） | 呼出し途中で元に戻した変更。監視対象外の副作用 |
| `runtime/whole-method-validator-{client,worker}.mjs` | ブラウザで検証器を Worker 上で動かす | 終わらない検証の打切り、起動しない Worker を `unavailable` とすること、無関係なメッセージの無視（`web/src/lib/whole-method-validator.test.ts`） | 実ブラウザの Worker |
| `scripts/compare_with_reference.mjs` | 合成解と参照実装を同じ入力で比べる | 正しい解と生成順の違う正しい解を受理する。誤った値、参照の複製、記録できない状態（実行前の入力、実行後の切り離された既存オブジェクトを含む）、余分な戻り値、フィールドの削除、切り離したノードの破壊、例外、終わらない呼出し、メソッド欠落を拒否する。終わらない参照はエラーにする（`tests/js/compare-with-reference.test.mjs`） | 呼出し途中の変更。到達できない生成物 |
| `tests/js/list-method-differential.cjs` | `Obj`/`f`/`g` のリストで多数入力を比べる | 単発11,454入力、連続14,400呼出しで5メソッドが一致する（`whole_method_validation_tests`）。誤候補9種を拒否する（`tests/js/list-method-differential.test.cjs`） | `Obj`/`f`/`g` 以外のクラス |

### ブラウザ版と Kanon

| モジュール | 役割 | 確かめていること（主なテスト） | 確かめていないこと |
| --- | --- | --- | --- |
| `web/src/lib/browser-runner.ts` | WASM の合成、翻訳、検証をブラウザでつなぐ | 全実演を再現するメソッドだけを採用する。PBE が成功しても翻訳が失敗すれば失敗とする。helper の結果が出力例と違えば拒否する（`web/src/lib/browser-runner.test.ts`） | 実ブラウザでの実行（テストは Node 上） |
| `Kanon/src/js/testize.js`（合成の部分） | 実演前状態を取得し、合成を依頼し、検証済みのコードだけを反映する | 型付き実演前状態の取得。未検証や失敗した応答で、エディタを変えない。合成中にソースが変わったら反映しない。直近の結果と反映しなかった理由を `lastSynthesis` に保持する（`testize-whole-method-validation.test.js`） | 実際の UI 操作。エディタと合成応答は代替物で置き換えている |

## 合成解と参照実装の比較

`recorded_stage_tests` が、Kanon の実記録から合成し、参照実装と比べる。

| 記録 | 入力 | 結果 |
| --- | ---: | --- |
| insert、setAt、append、prepend（`2026-10-14-fresh-recording`） | 各15 | 全件一致 |
| popBack（同上） | 12 | 全件一致 |
| rotateRight（`2026-10-15-rotate-right-4demos-494d6d4`） | 12 | **8件で不一致** |

リストの入力は、長さ 1、2、3、5、8 のそれぞれについて3種類の環境を用意したもの。
環境は、receiver が先頭の場合、より長いリストの途中の場合、外部から共有され切り離されたノードがある場合である。
popBack は長さ2以上に限る。

rotateRight の入力は、実演と同じく `this.left` と `this.left.right` が null でない木6種類である。
それぞれについて、receiver が根の場合と、別ノードの左の子の場合を試した。
4つの実演についての全体検証は通るが、`this.left.right` を求める helper が実演に特有の式になっている。
そのため、6種類の木のうち4種類で参照実装と一致しない。
これは既知の結果（同ディレクトリの README）と同じで、`expected.json` はこの不一致を期待値として書いており、テストはこれを検出できることを確かめている。
この事例のテストの成功は、既知の8件の不一致を再び検出したという意味であり、正しい右回転を合成できたという意味ではない。
全体検証を通過しても、実演していない入力で正しいことにはならない。

`Obj`/`f`/`g` による整数リストの多数入力での比較（`whole_method_validation_tests`）は、これとは別に残している。

## Kanon 側

合成結果を反映したかどうかと、反映しなかった理由は `__$__.Testize.lastSynthesis` に保持する。
画面には表示しない。
保持する理由は、応答なし、検証の状態、検証した実演数の不足、合成中のソース変更、反映の失敗のいずれかである。
`Kanon/src/js/test/testize-whole-method-validation.test.js` で確かめている。

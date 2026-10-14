# 論文に合わせた修正の残作業

確認日：2026-10-14。
RefSynの基準コミット：`b42d9cafa08ea32ea2d79e622e58b25a233d2b02`（#29のマージ後）。
初版のソース照合は `af76b2e` を対象とし、今回の更新では #29 のC1とC5の検査と保存記録を確認した。
他の項目を新しい版で全件照合し直したわけではない。
論文の基準コミット：`949ffd2cfffd63ea39c45e03fdefcdaa831c1f63`（`/Users/taku/workspace/paint2026-yoshio`）。
本書は現ソースとの照合による作業表であり、全経路の再実行や形式的な適合証明ではない。

## 仕様の参照範囲と、実際に読んだ範囲

**仕様の参照範囲**：`ja/main_ja.tex` が読み込む本文のうち、1〜5章を手法と実装説明の根拠とする。
該当するのは `ja/sections/` の `introduction.tex`、`background.tex`、`method.tex`、`operation-graphs.tex`、`implementation.tex` である。
`main_ja.tex` はappendixを読み込んでいない。
コメントアウトされた旧説明と、6章（`evaluation.tex`）の評価結果は実装仕様の根拠に使わない。

**実際に読んだ範囲**（参照範囲と同じではない）：

| 時点 | 読んだ範囲 |
| --- | --- |
| 本書の初版（2026-10-14） | 有効な3〜5章、本番の主要モジュール、既存テストの検査内容、サブモジュールの固定版と手元の差 |
| 2026-10-13の照合 | 1〜5章の全文（定義・構文・部品・翻訳・クラスと型の範囲を抜き出す目的） |
| #28後の改訂 | `operation-graphs.tex` 14〜24行・72〜78行・122〜130行・164〜170行、`implementation.tex` 183〜188行、`method.tex` 136行・201行付近 |
| #29後の更新 | `web/src/lib/binding-conditions.test.ts`、保存した型検査と負例のログ、固定版insertの要求、応答、再送と独立実行の記録 |

1〜2章（導入と背景）を、本書のために全文照合し直したわけではない。
上の範囲外の記述を根拠に、適合や不適合を判断しない。

## 既に本番へ接続されている処理

旧経路の置換を未着手として計画し直す必要はない。
[pipeline](../src/pipeline.rs) は、型付き記録の読込み、操作グラフ構築、全体同型、雛形、PBE問題生成を本番の入口へ接続している。

| 論文の規則または実装上の採用条件 | 現ソースで確認できる処理 |
| --- | --- |
| 既存物と生成物、型とクラスを区別した全体同型 | [alignment](../src/alignment.rs) と [Vertex::corresponds](../src/operation_graph.rs)。辺の向き、field名、重複数を保存し、非同型を拒否する |
| 既存物と定数を頂点ごとの穴にする | [template](../src/template.rs)。同じ頂点は一つの束縛を共有する |
| 全生成、全穴束縛、全書込みの順。生成は宣言順、書込みは第1実演の辺列順 | `Template::statements` と `render_method` |
| 生成を `new C()` とする | 操作グラフと雛形、Kanonの生成直後snapshot、全体検証 |
| 誤った候補を適用しない追加条件 | [validation](../src/validation.rs)、[browser runner](../web/src/lib/browser-runner.ts)、[全体検証器](../runtime/whole-method-validation.mjs) |

これらの存在はソースで確認した。
この表だけで、テストの成功や論文全体への適合を主張しない。

## 優先順位と完了条件

### 1. 再確認する構成を固定する

mainが固定するKanonは `60fa5c8e80d05a28dd12438ee1f4876a49bba79f`、escher-tsは `4e7c4bca45d98a0b5f2f76fd70486ed823c731f1` である（#28のマージ前後で変わっていない）。
手元のcheckoutは次のとおりで、固定構成とは一致しない。

- 文書整理前のRefSynのHEADは `b42d9ca`（#29のマージ後）。今回のcommitには実装変更とサブモジュールの更新を含めない。
- 手元のescher-tsは `dc13e6a`（escher-ts#3のマージコミット）を指す。固定版の `4e7c4bc` とツリーは同一（`9d17335`）で、内容の差はない。
- Kanonは固定版と同じ `60fa5c8` だが、`.DS_Store` と生成済み `src/js/menubar.js` の変更がある。
- 論文側には未コミットのappendix変更などがあるが、`main_ja.tex` はappendixを読み込んでいない。

`af76b2e` の別checkoutによる [insertの記録](evaluation_cases/reruns/2026-10-14-recording/README.md) は保存済みである。
この記録を `b42d9ca` の主要5ケースの評価結果に読み替えない。
次の作業は、既存変更を保持したまま、評価対象の版を固定し、主要5ケースとbaselineの基準結果を揃えることである。
escher-tsをビルドしてから、Rust、runtime、WASM、web、Kanonの記録と反映を確認する。
検査の起動順は [CI](../.github/workflows/ci.yml) を参照し、論文で使う結果には実際のSHAとコマンドと出力を保存する。
現在の手元状態で通った検査を、mainの固定構成の検査と呼ばない。

完了条件：固定した3リポジトリのSHA、依存のlockfile、実行コマンド、成否、未確認項目を対応付けて保存できること。

### 2. 出力例を束縛する方式の成立条件を明文化し、確認する

逐次合成への書換えは行わない。
現在の方式は、後続穴の入力へ、先行穴の **出力例**（実演から得た値）を束縛する。
[pbe_problem::build](../src/pbe_problem.rs) は `earlier.examples[i]` を入力へ追加し、[pipeline::plan](../src/pipeline.rs) は合成の前に全taskを作る。

論文 `ja/sections/method.tex` の「穴ごとの入出力例の構成」（201行付近）は、`<value>`、`<prev>`、`<next>` の順に合成し、**先行する合成式の評価結果** を次の入力環境へ加えると記述している。
次の条件がすべて成り立てば、各実演において、各穴へ渡す入出力例は両者で一致する。
入力の一致から言えるのはここまでであり、合成される式まで同じかどうかは、次の点に分けて扱う。

- 探索条件：現実装は穴ごとに新しい `AscendRecSynthesizer` を、全穴で同じ設定（`maxCost`、`searchSizeFactor`、`timeoutMs`）で作る（`runtime/refsyn-escher-adapter.mjs` の `runRefsynTasks`）。論文の手順を実装する場合も、同じ設定と同じ部品列を使うことが前提になる。
- 探索の決定性：AscendRecの探索に乱数は見当たらない（`synthesizer.ts` に `Math.random` はない）。同じtaskなら、時間内に終われば同じ式を返すと見込めるが、これを確かめたテストはない。
- 時間制限：既定の `timeoutMs` は2000msの実時間である。限界付近の穴は、機械の負荷で成功と失敗が入れ替わり得る。これは両方式に共通するが、「同じ式が得られる」とは言えなくなる。評価では制限と実行環境を記録する。
- 実行順：現実装は全taskを作ってから、taskを1つずつ順に合成する。論文の手順では、先行穴の合成に失敗した時点で後続穴へ進まない。現実装は残りの穴も合成し、1つでも失敗すれば採用しない。採用の可否は同じだが、合成時間と、失敗時に得られる診断は異なる。
- 穴の順：現実装の穴順は第1実演の辺列に従う。論文の `<value>`、`<prev>`、`<next>` の順と同じになるのは、辺列がその順のときだけである（下の完了条件を参照）。

| 条件 | 内容 | 現在の根拠 | 確認の状態 |
| --- | --- | --- | --- |
| C1 解は全例を再現する | escher-tsが返す式は、各実演の入力で出力例と等しい値（参照は同じ番号）を返す | AscendRecの `validateBody` と、#29の `web/src/lib/binding-conditions.test.ts`。WASMが生成したtaskに対し、返った式をescher-tsで評価する | paper_insertと真偽値の3fixtureで直接検査する。Int、Bool、Refを区別し、誤った式を検出する。式の読み戻しは `showTerm` の表示形式に依存し、汎用ではない |
| C2 入力は先行穴までに限る | 穴kの入力は、呼出し前状態、引数、生成物（生成直後の状態）、先行穴の出力例だけで、自身と後続の穴、書込み後の状態を含まない | `pbe_problem::build`。テスト `hole_inputs_add_created_objects_and_earlier_holes_but_no_writes`（`tests/paper_pipeline_tests.rs`） | テストあり |
| C3 生成JSも出力例を返す | 翻訳した補助メソッドが、実際の実行で出力例と同じ値（参照はidentity）を返す | 全体検証器が `hole_checks` で、各補助メソッドの実際の戻り値をちょうど1回、`Object.is` で照合する（`runtime/whole-method-validation.mjs` 221〜228行）。nativeは `validation.rs` から同じ検証器を使う | テストあり（`runtime/whole-method-validation.test.mjs` の `hole_checks` を使う各テスト） |
| C4 実行順が符号化と同じ | 生成メソッドは、全 `new` → 穴を固定順に1回ずつ評価 → 書込み、の順に実行する。そのため実行時の穴の入力は符号化した入力と一致する | `Template::statements`。テスト `insert_template_has_seven_statements_in_the_paper_order`、`creations_precede_holes_shared_vertices_bind_once_and_writes_keep_order_and_multiplicity` | テストあり |
| C5 補助メソッドは状態を変えない | 先行穴の評価が、後続穴の入力となる状態を変えない | #29の `web/src/lib/binding-conditions.test.ts` は、入力から到達できるobjectをProxyで包み、set、defineProperty、deleteProperty、setPrototypeOf、preventExtensionsを捕捉する | 4fixtureのhelperと、insertの入力上の部品別検査がある。通常入力ではescher-tsの値、参照番号、失敗と一致させる。環状入力では再訪を検出する3部品の失敗と、他の部品の結果を検査する。戻した書込みと後で上書きされる書込みも検出する。未観測入力や、大域、prototype、static、局所変数への書込みはこのProxy検査の範囲外 |

条件が成り立たない場合の扱い：
- C1が破れる（例を再現しない式が返る）と、C3の照合で採用を拒否する。誤った式は採用されない。
- C3が破れる（翻訳がescher-tsの評価と食い違う）と、全体検証で拒否する。翻訳と評価の一致は、別途 `runtime/refsyn-escher-adapter*.test.mjs` が、列挙した部品・式・入力の範囲で確かめている。

追加済みの検査と残る範囲：
- C1とC5の動的検査は #29 で追加済みである。[保存したログ](evaluation_cases/reruns/2026-10-14-pr29-checks/README.md) では `edf6ac1` の対象ファイルが14件成功し、即座に例外を投げるhelperと、走査中の書込みを注入した負例で失敗している。今回、この負例を再実行したわけではない。
- C5の静的検査として、翻訳が出力し得る全コードの形を部品一覧に対して自動検査する仕組みは追加されていない。動的検査の対象も、列挙した部品と入力に限る。
- 未観測入力での純粋性や、escher-tsの全ての部品の純粋性は保証しない。論文で純粋性を主張する場合は、検査した範囲を書く。
- 論文本文の修正案（本文は編集しない）：「後続穴の入力例には先行穴の出力例を用いる。PBEの解は全実演でこの値を返すため、先行式の評価結果と一致する。生成メソッドの採用前に、各補助メソッドの実際の結果がこれと一致することを確かめる。」

完了条件：C1〜C5のそれぞれについて、根拠となるコードとテストを示し、C1・C5の追加テストが通ること。
第1実演の辺列に基づく穴順と、生成コードの束縛順が同じであることも確かめる。

### 3. 論文の型とクラスの範囲を、実装の対応範囲と照合する

論文4章の構文は、型付きプリミティブ定数とクラス名を持つ（`operation-graphs.tex` 19・22行）。
同じ型の定数どうしだけを対応させ（76行）、定数の頂点は穴になる（127行）。
操作グラフと雛形は、数値・文字列・真偽値を区別する。

| 範囲 | #28のマージ前（`2a49c25`まで） | #28のマージ後（`af76b2e`） |
| --- | --- | --- |
| 整数のfield・引数・穴 | 対応 | 対応 |
| 単一クラスの参照（nullを含む） | 対応 | 対応 |
| 真偽値のfield・引数・穴 | `field_kinds` と `task_type` が拒否 | 対応。fieldはescher-tsで `Bool` と型付けしてslotに直接持ち、heapを作らない。引数と穴も `Bool` |
| 文字列 | 拒否 | 拒否（「string-valued holes cannot be encoded」。escher-tsに文字列の型と値がない） |
| 複数クラス | 拒否 | 拒否（「PBE problems support a single class」） |

#28のマージ後の真偽値の扱い：
- 観測値が真偽値だけ（undefined・不在は可）のfieldだけをBoolとする。数値・null・文字列と混在すれば変換せずに拒否する。
- 生成JSは、Boolのfieldを読むときに `typeof === "boolean"` を確かめる。undefined、null、`1`、`"true"` はfalseに補完せず、`TypeError` で失敗する。
- 根拠となるテスト：`tests/boolean_values_tests.rs`（nativeの合成・検証と独立した実行）、`runtime/refsyn-escher-adapter-booleans.test.mjs`（escher-tsと生成JSの値・失敗の一致）、`web/src/lib/booleans.test.ts`（WASM経路）。

field種別は観測値から決め、nullしか現れないfieldは参照fieldとする。

これは確認済みの対応範囲の差であり、全てのJavaScript型を追加する指示ではない。
論文が実装として対応を主張する範囲と、評価で必要な範囲を先に確定し、その範囲の型とクラスを境界全体で保持する。
整数の範囲、null、undefined、不在、生成物、外部aliasの符号化も同じ表で確認する。
未対応入力を推測で別の型へ変換しない。

完了条件：論文上の各対象型について、記録、雛形、task符号化、backend評価、JS翻訳、全体検証まで対応の有無が追えること。
実装が狭い箇所は、必要な修正と実装制限を区別する。

### 4. 本文にない拡張と実装の前提を切り分ける

`OperationGraph::ret` とreturn頂点の対応は、論文4章のグラフ構文にはない拡張である。
削除を先に決めず、論文の手法部分と返り値を扱う実装拡張を分けて検査する。
constructorの引数なし実行、初期fieldがプリミティブだけであること、通常のdata propertyなどの条件も、本文の `new C()` 規則そのものとは区別する。
現在の全体検証は有限の実演での採用条件であり、未知入力への正しさの証明ではない。

同じfieldへの複数回書込みは、順序と重複を保持したままにする。
今回の正しさの検討対象から外し、高々1回という入力制限は追加しない。
複数の同型から一つを選ぶことは本文にあるため、全写像の列挙や穴の成功に基づく対応の探索を当然の残作業にはしない。

完了条件：各条件が本文の規則、実装制限、拡張、採用検査のどれかを明示し、制限を論文由来の必須条件と取り違えないこと。

### 5. 固定版で翻訳と実際のKanon操作を再確認する

次の修正はマージ済みである。未修正の不具合として再実装しない。

| リポジトリ | PR | 内容 |
| --- | --- | --- |
| RefSyn | #19〜#21 | fieldの欠落とnullの区別、演算子の優先順位、`loadInt` が読むヒープの選択 |
| RefSyn | #22、#27 | 固定するKanonとescher-tsの更新 |
| RefSyn | #23 | null読出し、循環、and/orの正格評価などで、backendと生成JSの値・失敗・停止を一致させる |
| RefSyn | #24 | PRごとのCI |
| RefSyn | #25 | 日本語などの仮引数名の衝突 |
| RefSyn | #26 | Webのサンプルをv3形式に更新 |
| RefSyn | #28 | 真偽値のfield・引数・穴 |
| RefSyn | #29 | 先行穴の出力例を束縛する方式について、C1とC5を有限のfixtureと部品で直接検査する |
| escher-ts | #2、#3 | `findByValueRef` の未定義値と循環、`strictFieldAccess`、整数fieldの判定 |
| Kanon | #5 | Pagesのfixtureをv3にし、全体検証の `passed` を確かめる |
| Kanon | #6 | fallback後の捕捉と、最新の実行と食い違う保存済み捕捉を合成に使わない |
| Kanon | #7 | PRでjestを実行する（overlayの7件は除外） |
| Kanon | #8 | 実行結果の照合で、リテラルを型と値で比べる |
| Kanon | #9 | fallbackが、変化していない変数を再代入しない |

固定版のbackend評価と翻訳後JSを比較し、値と参照だけでなく失敗時の結果も確認する。
既存の比較検査は `runtime/refsyn-escher-adapter*.test.mjs` にある。

Kanonでは、呼出し前snapshot、操作ID、実演保存、合成、検証、ソース反映、反映後の再実行を一続きで確認する。
失敗時にエディタが変わらないこと、古い呼出しの記録が再利用されないことも対象とする。
従来の表示グラフによる合否表示だけで型やpropertyの一致を判断しない。

Kanonの未解決の問題（2026-10-14にKanon `60fa5c8` で再確認）：

1. **fallbackで、実演が付け替えた `const` 変数への代入が実行を止める。**
   - Kanon#9で、変化していない変数は返さなくなった。しかし、実演が別のオブジェクトへ付け替えた変数は、従来どおり `override` が返す（`src/js/testize.js` 2009行付近の条件）。
   - 呼出しの計装コードは、返された変数ごとに `eval(__variableName + ' = __object')` を実行する（`src/js/code-instrumentation/ast-transforms.js` の912行付近でASTとして生成。266・328・537行のコメントに同じ形がある）。
   - 再現の根拠：
     - `npx jest src/js/test/testize-fallback-override.test.js` の「a variable the demonstration points elsewhere is still reassigned」が通る（`override` が付け替えた `p` を返す）。
     - Nodeで `(() => { const p = {}; eval("p = __object"); })()` を実行すると、`TypeError: Assignment to constant variable.` になる。
     - 2026-10-13の実ブラウザでも、同じ例外で実行が止まることを確認した。
   - コードでは実現できない実演なので、実行を止めるのではなく、その実演を拒否すべきである。
2. **overlayの7件のテストが、どの環境でも失敗する。**
   - `npx jest src/js/test/testize-argument-overlay.test.js` を実行すると、7件とも `beforeEach` で「Property `inferMethodParamNamesForCall` does not exist in the provided object」となって失敗する。
   - テストは20ea4a6で追加されたが、`inferMethodParamNamesForCall` などの実装は、Kanonのどのcommitにも存在しない（`git log --all -S inferMethodParamNamesForCall` は20ea4a6のテストだけを示す）。
   - 実装は、手元のKanonリポジトリの未pushのstash（`stash@{0}`、`WIP on syn: db7ad45`）にだけある。
   - Kanon#7のCIはこの7件を除外している。機能を完成させるか取り下げるかは未決定である。
3. **真偽値のリテラルを入力できない。**
   - `literalInputType` は数値と文字列の型しか付けないため、`"true"` と入力すると文字列として記録される。黙って真偽値には変えない。
   - そのため、引数の真偽値をfieldへ書く実演は、Kanonの画面からは記録できない。真偽値のfieldをコピーする実演は記録できる（2026-10-13、#28のブランチで実画面確認）。
   - 論文の例と評価対象には不要と判断し、対応しない。真偽値への対応を主張する場合は、この制約を明記する。

完了条件：少なくとも論文のinsertを新規に記録し、型付き状態と送信payload、task、解、適用したソース、再実行結果が対応した証拠を保存する。
`af76b2e` での [insertの新規記録](evaluation_cases/reruns/2026-10-14-recording/README.md) は、この成功経路の証拠として保存済みである。
採用拒否の例も一つ通して確認する。
主要5ケースの既存Rust検査は `precondGraph` から型付き状態を補っており（`tests/whole_method_validation_tests.rs` の `with_typed_pre_state`）、実際の記録経路の証拠に代用しない。

### 6. 修正後の構成で論文の評価を取り直す

保存済みの結果は旧経路の結果であり、現在の実装の実行時性能や成功率に読み替えない。
新しいKanonで必要なsnapshotを含む実演を記録し、MOLDとbaselineの入力条件、部品集合、コストと時間制限を揃えて測定する。
部品として与えた走査と、穴から合成された式を区別して報告する。
手法の適合確認が終わる前に、本評価を先行させない。

完了条件：各ケースのpayload、生成task、backend結果、翻訳結果、全体検証、時間、使用SHAを保存し、失敗段階を区別して評価表を作れること。
本文のコード例、実装規模、公開デモがどの版に基づくかも最後に確認する。
公開デモへの同期や公開操作は別作業である。

## 今回の確認範囲

**初版（2026-10-14）**：上記の「実際に読んだ範囲」を読み、実装、論文、サブモジュールは変更していない。ビルド、合成、UI操作、本評価は再実行していない。

**今回の改訂**：
- #28のマージ後のmain（`af76b2e`）とそのCIの成否を確認した。
- escher-tsの固定版と手元版のツリーを比較した。
- Kanon `60fa5c8` で、overlayのテストと `testize-fallback-override.test.js` を再実行した。
- 上記のソース箇所を読んだ。
- 実装、論文、サブモジュールは変更していない。合成、UI操作、本評価は再実行していない。

**再改訂（2026-10-14、レビュー指摘への対応）**：
- 手元のHEADが `af76b2e` になったことを確認し、作業表1番を更新した。
- `runtime/refsyn-escher-adapter.mjs` の `runRefsynTasks` と翻訳の出力形、`synthesizer.ts` の時間制限を読み、作業表2番の「式も変わらない」という断定を、探索条件・決定性・時間制限・実行順・穴順に分けた。
- C5の根拠と、最終状態の照合では見逃す変更、追加する検査の範囲を書いた。
- テストは再実行していない。

**#29後の文書整理（2026-10-14）**：
- `b42d9ca` のC1とC5のテストを読み、追加済みの検査と残る範囲を更新した。
- 保存済みの負例、型検査、insert記録を照合した。保存した型検査ログは35件の診断を含み、全体の型検査成功を示さない。
- 固定版insertのサーバ終了と再起動失敗を記録のREADMEに反映し、チェックサムを更新した。
- 保存ソースを実行する2つのNode replayを再実行し、どちらも9件のhole照合が成功し、保存結果と一致することを確認した。Rustのfmtとclippyも成功した。
- 今回はビルド元を変更せず、合成要求の再送、負例の再実行、UI操作、本評価は行っていない。

上の作業表は、残作業の優先順位と根拠を示すものであり、全不具合の網羅や現在の実行成功を主張しない。

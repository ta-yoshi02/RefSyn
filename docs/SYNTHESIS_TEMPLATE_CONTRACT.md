# 合成経路の境界契約

全体の要件と設計判断は [PAPER_ALIGNED_REQUIREMENTS.md](PAPER_ALIGNED_REQUIREMENTS.md)、モジュール構成は `architecture.md` を参照する。
基準は論文の日本語版1〜5章（`method.tex` の3.2節、`operation-graphs.tex` の4章）である。評価章と旧来の共通／差分抽出は仕様の根拠にしない。

## 境界で交換するデータ

| データ | 所有するモジュール | 定義・不変条件 |
| --- | --- | --- |
| `Recording` | `demonstration` | クラスごとの `new C()` 直後の状態（または記録できなかった理由）と、実演ごとのヒープ、受信物、引数、仮引数名、編集列。IDはKanonの記録どおりで、実演をまたいで同じオブジェクトは同じID |
| `demonstration::Value` | `demonstration` | `Undefined`、`Null`、真偽値、数値、文字列、`Ref(ID)` を区別する。mapにないfieldは不在 |
| `OperationGraph` | `operation_graph` | 頂点宣言列（初出順）、書込み列（順序と重複を保持）、返り値の頂点。頂点は既存物、生成物、型付き定数、null。null頂点はグラフに一つ |
| `Mapping` | `alignment` | 第1実演の頂点から第 `i` 実演の頂点への全単射。第1実演は恒等写像 |
| `Template` | `template` | 生成（宣言順）、穴（束縛順、型、実演ごとの出力）、書込み（第1実演の辺列）、返り値 |
| `EscherTaskSpec` | `escher_bridge` | 穴ごとのescher-ts task。値の符号化は `REFSYN_TO_ESCHER.md` |
| `HoleCheck` | `pbe_problem` | 補助メソッド名と、実演ごとの期待出力（数値、または呼出し前状態のオブジェクトID） |

診断用の文字列（`common_pattern`、`hole_information`、`list_environment_info`）は表示専用で、後段で再解析しない。

## 変換規則

1. **正規化**：`addEdge`、`editEdgeReference`、`deleteEdge` を書込みにする。`deleteEdge` はnull頂点への書込みで、旧対象がオブジェクト参照の場合に限る。
   各編集の記録された旧対象は、呼出し前状態から再生した状態と照合する。`addEdge` はfieldがnull、undefined、不在のときに限る。
   生成物の再生状態は、そのクラスの `new C()` 直後の状態から始める。
   `return` への編集は返り値の頂点にする（論文の操作グラフにない拡張）。その他の変数操作とノード削除は失敗とする。
2. **対応付け**：頂点の対応 `~` を満たし、各書込み `(始点, field, 終点)` の重複数を保存する全単射を探す。辺の順序は比較しない。
   第1実演の頂点を宣言順に、相手の候補を宣言順に試し、最初に見つかった同型を使う。見つからなければ失敗とする。
3. **雛形**：既存物と定数の頂点を穴にし、生成物とnullは固定する。穴の出力例は対応先の頂点が表す値またはオブジェクトである。
   穴の束縛順は、第1実演の辺列で最初に現れる順（各辺で始点、終点の順）とし、返り値の穴を最後に置く。
4. **文列**：生成（宣言順）→ 穴の束縛 → 書込み（辺列の順、重複を保持）→ `return`。穴 `k` の補助メソッドは、仮引数、全生成物、穴 `0..k-1` を引数に取る。
   `insert` は生成1、穴3、書込み3の7文になる。関数名やフィールド名による分岐はない。
5. **入力例**：穴 `k` の入力は、全生成を済ませて書込みを実行する前の環境に、穴 `0..k-1` の出力例を加えたものである。穴 `k` 自身と後続穴の出力例は含めない。
   出力例を先に束縛してよいのは、PBEの成功条件が `eval(e_j, E_j) = y_j` だからである。生成メソッドの実行時には先行式の評価結果が束縛され、全体検証でそれが出力例と一致することを確かめる。

## 本番採用前の実演検証

`runtime/whole-method-validation.mjs` は、本番が生成したJavaScriptを各実演の呼出し前状態から実行して検査する。

### 保証する範囲

検証が `passed` を返すとき、保証するのは次の2点である。

1. **実演との一致**：生成メソッドは、与えられた全実演について、以下の観測範囲で期待状態を再現する。各補助メソッドは各実演で出力例をそのまま返す。
2. **検証の健全性**：`passed` は、この観測範囲での全実演一致を意味する。観測範囲の外の正しさは含まない。

未観測の入力に対する正しさ（汎化）は保証しない。実演が区別しない誤りは検出できず、その対策は実演の追加である。
同一fieldへの複数回書込みは、件数で拒否も圧縮もせず記録順に再生するが、その一般的な正しさは主張しない。

### 期待状態

期待状態は、各実演の型付き呼出し前状態から、記録順にMOLDの操作を再生して構成する。生成コードには依存しない。

- `addNode(C)` は送信されたクラス定義の `new C()` とする。constructorの既定値を保ち、全fieldをnullにはしない。
- `addEdge(m,f,n)` は、Kanonが辺を描かないnull、undefined、または不在のfieldにだけ適用できる。辺がある場合は記録と状態が矛盾するため検証不能とする。
- `editEdgeReference` と `deleteEdge` は、記録された旧対象が現在値と一致することを確認する。
- `deleteEdge` はfieldへのnull代入であり、propertyは残す。旧対象がオブジェクトである参照fieldにだけ適用し、値fieldへの削除は検証不能とする。
- 返り値は、`return` への編集があればその対象、なければ `undefined` とする。
- literalは記録された型のまま読む。Kanonは入力時に型を決めて記録する（`literalInputType`）。正規の10進表記はnumber、それ以外（`"007"` を含む）はstringである。合成側も記録された型を使い、文字列を出力する穴はescher-tsで表現できないため合成しない。
- 期待状態の導出中の例外（外部変数を読むconstructor等）は、候補の誤りではなく実演の問題として検証不能とする。

### 比較する観測範囲

- 全既存物と、生成物を含むそこから到達できる物の値、propertyの有無とdescriptor、undefinedと不在の区別、クラス、identityの全単射（循環、共有、外部alias）
- 返り値（値またはidentity）
- 各補助メソッドの実際の戻り値と `hole_checks` の期待出力（各実演でちょうど1回呼ばれること）
- 生成数。到達不能な生成物は、生成順ではなく構造で対応付ける（バックトラック探索）
- ヒープ外の状態：`globalThis` の名前付きproperty、`Object.prototype`、`Function.prototype`、`Array.prototype`、各クラスのconstructorとprototypeの own property。比較は浅く、これらから辿れる別オブジェクトの内部変更は見ない。hostが自ら登録するSymbolキーのglobalは比較しない。

### 入力形式（`validation.version = 3`）

各classは `name`、元の `source`、`instance`（`new C()` 直後のown data property）または `instanceError` を持つ。各caseは実演ID、`objects`、`values`、`arguments` を持つ。
各objectの `fields` は完全なown data propertyのmapであり、数値、文字列、真偽値、null、`{undefined:true}`、`{ref:"ID"}` を区別する。mapにないpropertyは不在を表す。
`values` は操作が指す既存literalのIDと値の対応である。KanonはliteralノードをID `<object id>-<field>` で表示し、操作もこのIDで記録するため、この命名規約に依存する。
Kanonは呼出し直前に受信物、引数、登録済み実オブジェクトを走査し、テスト保存時にそのsnapshotを複製する。表示グラフから欠落fieldやconstructorの既定値を推測しない。
操作IDの変換とsnapshot中のIDの変換には同じ対応表を使う。全実演は同じクラスの同じメソッドを対象としなければならない。

### 対象とするプログラム

ソース中のトップレベルclass宣言と、通常のdata fieldを持つ有限のobject graphに限る。
constructorは引数なしで実行でき、決定的で、既存状態や外部状態を変更せず、プリミティブ、null、undefinedだけでfieldを初期化することを前提とする。範囲外のconstructorは `instanceError` として記録し、そのクラスを生成する実演は合成しない。乱数や時刻を使うconstructorでは、正しい候補を不採用にし得る。
継承、accessor、Proxy、private field、Symbolキー、関数値、配列、Map/Set、非有限数、負のゼロ、外部変数の更新は対象外であり、検出した場合は `passed` にしない。
既存メソッドが実演編集の前にheapを変更した場合は、編集列だけでは期待最終状態を構成できないため検証不能とする。

### 実行と採用

結果は `validation.status`（`passed`、`failed`、`unavailable`）、`checked_demonstrations`、`error` で返す。
nativeはNode VM（同期実行1秒）を子プロセスで動かし、プロセス全体にも10秒の上限を設ける。
browserは専用Workerで実行し、Workerの起動を待ってから1秒の実行上限を計る（起動は10秒まで）。Workerからの応答は要求ごとのnonceを持つものだけを受理する。
この実行分離は一般の悪意あるJSに対するセキュリティ保証ではない。
`passed` 以外では `composed_method_code` と適用用 `code` を返さず、穴ごとの結果は診断として残す。
この採用判定を行うのは `handle_synthesis` とbrowserのラッパーである。`synthesize_core` とwasmの `synthesize_browser` は検証前の候補を返すので、直接呼ぶ場合は採用に使わない。

## Kanonでの反映

Kanonは、全実演の検証成功と送信時からの元ソース不変を確認したうえで、検証器が実行したのと同じ変更だけをソースへ適用する（`buildValidatedSource`）。

- 対象クラスがトップレベルにちょうど1つあり、対象メソッドがその通常のメソッドとしてちょうど1つあること
- 補助メソッドは対象メソッドの直前に `// refsyn: generated helper` を付けて追加する。同名のメンバーが既にある場合は、この印の付いた以前の生成物だけを置き換え、利用者が書いたメンバーとの衝突は適用を拒否する
- 再構築したソースが構文解析できること。拒否した場合はエディタを変更しない

検証器はメソッドをprototypeへ定義して実行し、Kanonはclass本体のメソッド定義を置き換える。上の条件（継承なし、同名メンバーは対象か印付き補助だけ、通常のメソッド）の下では両者は同じ動作になる。

テストが失敗している間の状態反映（実演した状態から実行を続けること）は、MOLDの意味で行う（`applyMoldEdits`）。生成物はそのクラスの `new C()` で作り、`deleteEdge` はnullを代入し、編集されないpropertyとオブジェクトはそのまま残す。
型付きsnapshotがない、既存メソッドがheapを変更した、またはMOLDにない編集を含む実演では、heapを変更せずに従来の表示グラフからの再構築へ委ねる。その実演はsnapshotの誤りまたは未対応の編集として合成されない。
テストの合否表示（`checkActualGraph`）は従来どおり表示グラフの比較であり、null、undefined、不在を区別しない。採用の可否は上記の型付き検証で決める。

## 確認済みの範囲と限界

- 置換後のKanonでの再実行結果を採用処理の中で自動確認する機能はない。
- 実際のKanonで、論文の `insert`（3実演、生成物を含む）について、記録、実演、合成、検証、ソース反映、再実行までを確認した。反映後のソースを独立に実行した結果は `[39, 89, 71, 27, 46, 53]` であった。
- 保存済みの主要5ケース（append、insert_general、popBack、prepend、setAt）は、記録済みの `precondGraph` から型付き呼出し前状態を組み立てたfixtureで、合成から検証、採用までと、未実演の入力での参照実装との一致を確認している（`tests/whole_method_validation_tests.rs`）。fixtureは表示グラフで省略されたfieldをObjのconstructorに合わせてnullとして補っており、Kanonの実際の記録経路の確認ではない。
- 旧payloadには必要なsnapshotがないため、対応するKanonで実演を記録し直す必要がある。

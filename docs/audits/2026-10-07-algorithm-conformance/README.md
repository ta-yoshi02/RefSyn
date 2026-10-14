# 論文のアルゴリズムとRefSyn実装の照合

確認日：2026-10-07。
RefSyn：`cc370bc56eca3655bdf09de97a4fa555705e7ad4`。
論文：`949ffd2cfffd63ea39c45e03fdefcdaa831c1f63` の作業ツリー。
対象は `ja/main_ja.tex` が読み込む日本語本文であり、コメントアウトされた旧説明は仕様として扱わない。
ファイルのハッシュは [provenance.json](artifacts/provenance.json) に保存した。

これは上記の修正前の版を対象とした診断記録である。
本文の「現行」「今回」は2026-10-07の確認を指し、その後の修正状況は [現在の作業表](../../PAPER_ALIGNMENT_WORKLIST.md) で扱う。
以下の診断を、現在のmainで未修正の不具合という意味に読み替えない。

実装は論文の現行アルゴリズムに一致していない。
とくにPBE環境の選択と生成コードの評価時点の不一致により、穴の合成がすべて成功しても、合成に使用した実演の更新結果を再現できない。
操作記録の自動補正と、途中生成したオブジェクトの符号化にも、独立した誤りを再現した。
ただし、論文の形式化も書込み順の扱いについて追加条件を必要とし、一般的な正しさがこの確認によって成立したわけではない。

## 確認範囲と証拠

操作記録の正規化、操作グラフへの変換、2記録の対応付け、複数記録の共通部分の縮約、穴の構成、環境の導出、escher-tsへの符号化、式のJavaScript化、メソッドの組立て、成功判定を確認した。
主な対象は `src/lib.rs`、`src/unify_ops.rs`、`src/list_env.rs`、`src/escher_bridge.rs`、`runtime/refsyn-escher-adapter.mjs` と、呼ばれるescher-tsのフィールド部品および走査部品である。
本番の対応付けは `unify_operation_graphs` を通る。
`src/isomorphism.rs` の `unify_isomorphic_graphs` は別の実装であり、そのテストの成功を本番経路の検証と同一視できない。

以下では、現行ソースから確定した挙動、実行によって再現した不具合、論文に必要な条件を区別する。
再現結果は今回の実行で新規に取得した。
既存の実験結果や金曜日までの評価議論は根拠に使用していない。
既存のinsert入力は診断用入力としてのみ再利用し、合成と実行は新たに行った。

KanonのUI操作と公開WebAssemblyデモは再実行していない。
全ベンチマークの再計測も行っていない。
この記録は修正前の診断であり、実装全域の正しさの証明でも、再実験後の評価議論でもない。
元から存在したKanonの変更、論文の変更、未追跡ファイルは保持した。

## 論文が定める手順

| 段階 | 現行本文の規定 | 実装の挙動 |
| --- | --- | --- |
| 操作グラフ | 頂点は既存オブジェクト、新規オブジェクト、型付き定数、null。辺はフィールドへの書込み列 | 操作種別ごとにaddEdge、editEdgeReference、deleteEdgeを区別し、旧参照先もグラフへ入れる |
| 対応付け | 型、クラス、既存と新規の区別、辺の向き、フィールド名、重複数を保存する同型対応。対応がなければ失敗 | 部分的な頂点対応を探索し、合う辺の個数を得点化する。型とクラス、辺の多重度の条件が不足 |
| 穴の構成 | 対応対象の既存オブジェクトとプリミティブ定数を頂点ごとの穴にし、同じ頂点の全出現で共有 | 同じIDの既存頂点、同じラベルの生成頂点を共通部分へ残す。差分操作と役割から穴を作る |
| PBE環境 | 呼出し時の環境に新規生成オブジェクトを追加し、雛形の穴順に、先行する穴の結果の局所束縛を追加 | 既存対象では呼出し前、それ以外では元の操作記録の接頭辞を再生。先行穴の結果を引数に追加しない |
| コード生成 | 全生成文、穴の束縛文、辺列に従う代入文の順 | 全穴の呼出しを本文より先に配置。生成文と代入文は本文側に置く。穴のメタデータから代入を補う場合もある |

本文の根拠は [method.tex](/Users/taku/workspace/paint2026-yoshio/ja/sections/method.tex:194) と [operation-graphs.tex](/Users/taku/workspace/paint2026-yoshio/ja/sections/operation-graphs.tex:62) である。
前者の199–219行は、新規ノード生成と先行する穴の結果を含む `E1, E2, E3` を定義している。
後者の67–97行、124–145行、166–182行は、それぞれ同型条件、穴の条件、コードの文順を定義している。

## 1. PBE環境と式の評価時点

### 実演を再現できない時間の不一致

実装は [build_environment_prefix](/Users/taku/workspace/kenkyu/proj-ref/refsyn/src/lib.rs:2949) で、初期環境へ元の操作列を再生する。
[環境の選択](/Users/taku/workspace/kenkyu/proj-ref/refsyn/src/lib.rs:6740) は、既存対象に対する例外を除き、対応する差分操作の直前状態を使用する。
これは操作グラフだけから状態を求める処理ではなく、初期グラフと、順序を持つ元の操作記録を使う処理である。
順序を捨てた操作グラフから、ある操作の直前状態を一意に求めることはできない。

一方、[メソッド組立て](/Users/taku/workspace/kenkyu/proj-ref/refsyn/src/lib.rs:5547) は、穴の呼出しをすべて本文より前に出力する。
したがって、前のフィールド更新を含むPBE入力と、更新をまだ実行していないJavaScriptの入力が食い違う。

新規に作った `copyMiddle` の入力は、`A.next=B; B.next=C` という3ノードのリストである。
実演は中央Bの値をAとCへ設定する。

| 記録 | 呼出し前の値 `(A,B,C)` | 実演後の期待値 | 生成コード実行後 |
| --- | --- | --- | --- |
| 1 | `(1,7,0)` | `(7,7,7)` | `(7,7,1)` |
| 2 | `(2,9,0)` | `(9,9,9)` | `(9,9,2)` |

PBEは2穴とも成功した。
中央Bの値を取る式に加えて、末尾Cのために `this.val` を取る式が合成された。
後者のPBE入力には先行操作で更新されたA.val（7または9）が含まれるが、生成メソッドではA.valを更新する前にその式を評価してしまう。

```javascript
copyMiddle() {
    const h_int_0 = this.copyMiddle_f(); // this.val: 1または2
    const h_int_1 = this.copyMiddle_g(); // 中央の値: 7または9
    const snap0 = this.next.next;
    this.val = h_int_1;
    snap0.val = h_int_0;
}
```

根拠は [入力](artifacts/timing-payload.json)、[合成結果](artifacts/timing-response.json)、[PBE問題](artifacts/timing-tasks.json)、[メソッドの実行比較](artifacts/timing-replay.json) に保存した。
これは未見入力への一般化の問題ではなく、訓練に用いた2実演そのものへの不一致である。

### 新規オブジェクトと先行穴の束縛がない

論文では穴の評価より先に新規オブジェクトを生成し、先行する穴の値を局所変数へ保持する。
実装ではPBE問題を独立に構成してまとめて合成器へ渡し、先行する合成結果を次の問題へ追加していない。
[穴の呼出し引数](/Users/taku/workspace/kenkyu/proj-ref/refsyn/src/lib.rs:4956) もメソッドの仮引数から作られ、生成したオブジェクトや先行穴の局所変数を渡していない。
既存参照の穴が呼出し前の状態を使うことも、論文の `E2/E3` と同じではない。

修正では、雛形で実際に評価される時点の環境を、各実演について構成する必要がある。
論文の順序を採用するなら「生成済みオブジェクトを含み、書込みは未実行で、先行穴の局所束縛を含む状態」を入力にする。
穴を単に元の操作位置へ移動する修正では、論文が定めた手順と一致しない。

## 2. 環境の符号化

### 途中で生成したオブジェクトが消える

[オブジェクト列の収集](/Users/taku/workspace/kenkyu/proj-ref/refsyn/src/escher_bridge.rs:502) は、再生後の `env` に存在する全オブジェクトではなく、与えられた `vis_graph.nodes` に含まれるものだけを採用する。
PBE問題へ渡す `vis_graph` は、途中状態を反映するよう更新されていない。
[参照フィールドの変換](/Users/taku/workspace/kenkyu/proj-ref/refsyn/src/escher_bridge.rs:557) は、この列にない参照先を `null` にする。
出力参照と参照引数にも同種の対象欠落があり、`fixed_id_index_for_object` はその場合に番号を返せない。

再現では、初期状態にAがあり、その後Nを生成して `A.next=N` とした。
再生後の環境には `[1,null]` があるのに、初期グラフを使ったPBE入力は `[null]` になった。
Nをグラフの頂点列へ追加して同じ環境を符号化すると、`[1,null]` が保持される。
結果は [probes.json](artifacts/probes.json) の `prefix_new_object_dropped` にある。

参照番号の対象集合を、入力環境に存在するオブジェクトと一致させる必要がある。
番号は入力の参照、出力の参照、ヒープの配列、JavaScriptへ戻すオブジェクトの対応で共通に扱う。
未登録の参照と、本来のnullを同じ値へ落とすと、失われた対象を検出できない。

### 負の整数と未定義値が混同される

[build_object_heap](/Users/taku/workspace/kenkyu/proj-ref/refsyn/src/escher_bridge.rs:951) は、値が負なら、値フィールドの参照を `-1` にする。
`val=-2` という定義済みの整数も、escher-tsでは値フィールドが存在しない表現になる。
JavaScriptの `hasVal` への変換は [adapter](/Users/taku/workspace/kenkyu/proj-ref/refsyn/runtime/refsyn-escher-adapter.mjs:200) で `this.val !== undefined` を返すため、この表現と一致しない。

診断用に初期部品を自動フィールド部品へ絞り、`val=-2` と `val=2` の2入力を与えた。
符号化された状態での期待値を `[false,true]` とすると、escher-tsは `hasVal` を合成する。
同じ合成式をJavaScriptへ変換して元のオブジェクト上で実行すると `[true,true]` になった。
これは診断用の部品制限を使った境界の再現であり、現行ベンチマークの部品集合でこの式が選択されたという主張ではない。
根拠は [negative-task.json](artifacts/negative-task.json) と [negative-result.json](artifacts/negative-result.json) にある。

undefinedと整数を別に表現し、フィールドが存在するかどうかを値の符号から判定しない修正が必要である。
同じ型を保った符号化とJavaScript化の双方で、読取りと存在判定の意味を一致させる。

### 値の型と範囲が保たれない

[値列の変換](/Users/taku/workspace/kenkyu/proj-ref/refsyn/src/escher_bridge.rs:538) は、フィールドを `i32` に変換し、変換できなければ `-1` とする。
再現では、真偽値trueが `-1` へ、整数4294967296が0へ変換された。
`OutputType` もIntとPtrだけであり、任意の型付きプリミティブをそのまま合成する実装にはなっていない。
結果は `value_encoding` にある。
対応する型と数値範囲を明示し、その範囲外は情報を変更して継続するのではなく、入力エラーとして検出する必要がある。

## 3. 操作記録から操作グラフへの変換

### 正しい記録を自動補正が変更する

[repair_set_reference_operations](/Users/taku/workspace/kenkyu/proj-ref/refsyn/src/lib.rs:7466) は、既存ノードから新規ノードへの接続後、その新規ノードの同じフィールドへの最初の設定を、既存ノードの旧参照先へ補正する。
自己ループだけが対象ではなく、別の参照先を明示した設定も変更される。

初期リスト `A→B→C` に対し、Nを生成して `A.next=N; N.next=C` と記録した。
この実演はNの後でBを飛ばし、Cにつなぐ更新である。
実装は `N.next=C` を `N.next=B` に変更した。
生成メソッドを初期状態から実行し、N.nextが期待したCではなくBを指すことを確認した。
根拠は [入力](artifacts/repair-payload.json)、[補正と生成結果](artifacts/repair-response.json)、[実行比較](artifacts/repair-replay.json) にある。

実演が指定した更新を保つ処理と、Kanonの特定の記録不具合への補正を区別する必要がある。
一般の合成経路で「旧後続への挿入だろう」と推測して設定先を上書きしてはいけない。

### 数値ラベルが黙って読み捨てられる

[convert_json_to_unify_ops](/Users/taku/workspace/kenkyu/proj-ref/refsyn/src/lib.rs:1468) 内の `OperationRecord.label` は文字列であり、公開の `GraphOperation.label` のJSON値型と一致しない。
デコードは `if let Ok` で行うため、数値や真偽値のラベルを持つ操作をエラーにせず読み捨てる。
既存対象を作る際にも、元のクラスラベルを保持せず空文字列を入れる。

再現では、`label: 1` のリテラル生成が消え、そのリテラルへの辺の参照先が、`is_literal:false` の既存オブジェクトとして作られた。
`numeric_label_conversion` に変換結果を保存した。
受け付けるラベル型の整合と、失敗時に操作を落とさないエラー処理が必要である。

### 書込みの意味を表す中間表現が違う

論文の辺は新しい値を設定する `editEdge(m.f,n)` である。
実装ではaddEdgeとeditEdgeReferenceを別種として対応させ、editEdgeReferenceの旧参照先も比較する。
さらに、定数をUI上で新たに作ったか、初期グラフに既にあったかで、AddNodeとExistNodeを区別する。
論文ではどちらの定数も型付き定数cであり、このUI上の違いは対応条件に含まれていない。
ここはソース上で確認した表現の不一致であり、すべての組合せについて合成失敗を実行したわけではない。

## 4. 共通部分と穴の構成

### 型とクラスの条件が足りない

[unify_operation_graphs](/Users/taku/workspace/kenkyu/proj-ref/refsyn/src/unify_ops.rs:123) の候補群は、新規か、リテラルか、nullかという3つの真偽値だけで作られる。
再現では、Nodeの生成頂点がTreeの生成頂点へ対応し、その間のval書込みが共通辺として残った。
値1の頂点とtrueの頂点も対応した。
結果は `class_mismatch` と `primitive_type_mismatch` にある。
公開のNodeExpr自体がリテラルの型情報を持たないことも、論文の型条件を満たせない原因である。

### 辺の重複数を保存しない

辺の照合は `find_map` または `any` で行い、対応に使った相手の辺を消費しない。
同じ辺を2回書いたAと1回書いたBを比較すると、Aの2辺ともBの同じ1辺に対応し、両側の差分が空になる。

論文の「書込み総数が等しい」条件内でも検査した。
Aをleft,left,right、Bをleft,right,rightという3書込みにすると、共通部分に残る書込み数がA側3、B側2となった。
Bの1辺をAの複数辺に使っており、重複数を保存する同型対応ではない。
`edge_multiplicity` と `equal_total_edge_multiplicity` に結果がある。

### 同型がない場合でも部分対応を返す

[find_best_mapping_recursive](/Users/taku/workspace/kenkyu/proj-ref/refsyn/src/unify_ops.rs:596) は頂点を対応させない分岐も探索し、合う辺の個数と対応頂点数で結果を選ぶ。
全対象頂点と全書込みについての同型成立を合成の前提として検査していない。
`nonisomorphic_partial_acceptance` は、追加のother辺があるグラフにも部分的な共通部分を返すことを示す。
総書込み数が異なるこの入力は論文の対象条件外であり、その条件外入力を拒否していない証拠として扱う。
前項の同じ総書込み数の入力も、辺の重複数が違うため同型は存在しない。

### 論文のHole条件に一致しない

実装は、生成リテラルのラベルが等しければ共通頂点、既存対象のIDが等しければ共通頂点とする。
同じ操作グラフ同士の比較では、定数も共通部分へ残り、差分は空になった。
`equal_operands_not_holed` がこの挙動を示す。
論文のHole条件は、値が実演間で等しい場合も、既存オブジェクトとプリミティブ定数を穴にする。
「差分だけを穴にする実装」と現行の頂点ごとの規則は別のアルゴリズムである。

複数記録では、共通操作へ縮約して次の記録と比較し、その後に差分候補を組み合わせて仕様をまとめる。
同じ第一記録の頂点に対する全記録の対応と、頂点ごとに共有する穴を、一つの構造として保持する実装にはなっていない。
そのため、2記録の処理だけを交換して終わらず、[縮約](/Users/taku/workspace/kenkyu/proj-ref/refsyn/src/lib.rs:4183) と [仕様の集約](/Users/taku/workspace/kenkyu/proj-ref/refsyn/src/lib.rs:4441) も変更対象になる。
複数記録でのすべての対応候補の誤りを再現したという主張はしない。

## 5. 組立てと成功判定

代入文は、共通操作だけでなく、穴の役割と記述文字列から補充される。
[materialize_operand_common_ops](/Users/taku/workspace/kenkyu/proj-ref/refsyn/src/lib.rs:5751) と、組立て後半の旧方式へのフォールバックがこの処理を担う。
新しく再実行したinsertでも、common_patternには既存ノードから新規ノードへの書込みがない一方、生成メソッドにはその書込みがあった。
このinsertは3実演すべての値、参照、生成数、返り値の比較を通過した。
ただし、動作する一例があることと、論文の辺列を忠実に変換していることは別に確認する必要がある。
根拠は [insert-response.json](artifacts/insert-response.json) と [insert-replay.json](artifacts/insert-replay.json) にある。

[候補選択](/Users/taku/workspace/kenkyu/proj-ref/refsyn/src/lib.rs:4608) は穴仕様の対応範囲を基準にし、生成メソッドを実演の初期状態から実行して選別していない。
[合成完了処理](/Users/taku/workspace/kenkyu/proj-ref/refsyn/src/lib.rs:1283) も、各穴の結果を収集し、メソッド全体の更新結果を比較しない。
JavaScript化に失敗した場合でも、adapterはPBE成功を `success:true` として返せる。
実行比較には少なくとも、既存オブジェクトの同一性、参照、値、新規生成数、返り値を含める必要がある。

走査部品の意味にも条件がある。
escher-tsのlast_ptrとpenultimateRefは循環を検出してエラーを返し、findByValueRefは循環で探索を終了する。
JavaScriptへの展開には対応する循環検出がなく、循環入力では終了しない場合がある。
これはソースから確認した差であり、今回の単方向リストの実行比較に循環ケースは含めていない。

## 6. 論文の形式化に必要な条件

書込み総数とクラス別生成数が同じでも、書込み順を無視してよいとは限らない。
同じA.valへ、第一実演で1、2の順、第二実演で2、1の順に設定する例を考える。
入力の引数はそれぞれ0、1とし、初期状態はどちらもA.val=0とする。
この2記録は辺順を無視すると同じグラフで、頂点の恒等対応も合法な同型である。
その対応を選ぶと、値1と値2の穴にはそれぞれ全記録で1と2が出力例として与えられ、定数式1と2が両穴の仕様を満たす。
第一実演の辺列で生成すると、両入力で最終値は2になるが、第二実演の期待最終値は1である。

これは論文の規則から構成した反例であり、現行実装でその対応が実際に選択されたという実行報告ではない。
「同型があれば一つ選び、第一記録の辺列を使う」だけでは、全実演の事後状態との一致が導けないことを示す。
insertの説明は書込み先が異なると明記しているが、形式的な対象条件には同一フィールドへの繰返しを含めた順序の扱いが必要である。
同じ書込み先について順序を保存する条件を設けるか、実行比較によって不適切な対応を棄却する必要がある。

## 修正の優先順と完了条件

1. **実演を保つ入力処理とメソッド全体の比較を用意する。** 正しい参照設定を推測で補正しない。repairの反例が意図したN.next=Cを保ち、copyMiddleの失敗を合成成功として扱わないことを確認する。
2. **環境とコードの評価順を論文に合わせる。** 生成文を先に出し、書込み前の雛形状態と先行穴の局所束縛からPBE問題を構成する。copyMiddleの2実演が `(7,7,7)` と `(9,9,9)` になり、新規参照と先行穴への依存も扱えることを確認する。
3. **環境の表現を修正する。** 全オブジェクトの参照対応、undefined、null、整数を区別する。途中生成したNが符号化後も残り、負の整数でフィールドの存在が変わらず、部品評価とJavaScript評価が一致することを確認する。
4. **操作グラフ、同型対応、穴を規則どおりに構成する。** 型とクラスを保持し、書込みを共通表現にし、辺の多重度を保存する。同型のない入力は失敗させ、頂点ごとの穴を全記録で共有する。数値ラベルを落とさず、共通操作と生成した代入文が対応することを確認する。
5. **修正版を固定して実験をやり直す。** 合成に使った全実演の実行比較を成功条件に含め、未見入力の検査と探索時間を別に記録する。旧結果を混ぜず、成功、合成失敗、組立て失敗、実行不一致を区別して、新しい結果から評価議論を作る。

修正を入れる前に長時間の性能計測をやり直すと、誤った入力状態や更新を測定した結果を再び作ることになる。
この順序で意味の不一致を除き、測定する合成問題と成功条件を固定する必要がある。

## 実行した検査と再現方法

`cargo test --locked`：172 passed、1 ignored。
`cargo fmt --all -- --check`：成功。
`cargo clippy --locked --all-targets --all-features`：終了成功、libに26件、lib testに37件の警告（重複を含む）。
これらの成功と、今回の意味に関する失敗は両立している。

診断用の [probes.rs](probes.rs) は公開APIを使い、対応付けと符号化を検査する。
[replay.mjs](replay.mjs) はNodeの既知の初期値 `val=undefined, next=null` を用い、各実演を別々に初期化し、記録した操作から得る期待状態と生成メソッドの状態を比較する。
このスクリプトは今回のNode入力用であり、任意のJavaScriptクラスや任意の操作の汎用検証器ではない。
[negative-replay.mjs](negative-replay.mjs) は、符号化された問題とJavaScript化された式の違いを確認する。

RefSynのルートから、同じソース版とビルド済みescher-tsを用いて実行する。
`probes.rs` は現行依存のserde_jsonをリンクするため、rlibのハッシュが変わった場合はビルド出力に対応するパスへ置き換える。

```sh
cargo build --locked --lib --example run_payload
rustc --edition 2021 docs/audits/2026-10-07-algorithm-conformance/probes.rs -L dependency=target/debug/deps --extern refsyn=target/debug/deps/librefsyn.rlib --extern serde_json=target/debug/deps/libserde_json-452ccc559ddbcb99.rlib -o /tmp/refsyn-audit-probes
/tmp/refsyn-audit-probes /tmp/refsyn-audit-repro
node docs/audits/2026-10-07-algorithm-conformance/negative-replay.mjs /tmp/refsyn-audit-repro/negative-task.json /tmp/refsyn-audit-repro/negative-result.json "$PWD/runtime/refsyn-escher-adapter.mjs"
target/debug/examples/run_payload docs/audits/2026-10-07-algorithm-conformance/artifacts/timing-payload.json
target/debug/examples/run_payload docs/audits/2026-10-07-algorithm-conformance/artifacts/repair-payload.json
node docs/audits/2026-10-07-algorithm-conformance/replay.mjs docs/audits/2026-10-07-algorithm-conformance/artifacts/timing-payload.json docs/audits/2026-10-07-algorithm-conformance/artifacts/timing-response.json /tmp/refsyn-audit-repro/timing-replay.json
node docs/audits/2026-10-07-algorithm-conformance/replay.mjs docs/audits/2026-10-07-algorithm-conformance/artifacts/repair-payload.json docs/audits/2026-10-07-algorithm-conformance/artifacts/repair-response.json /tmp/refsyn-audit-repro/repair-replay.json
```

保存済みresponseでの実行比較は、保存した今回の生成コードを再実行する検査である。
修正後の検査ではrun_payloadの新しいJSON応答を抽出し、replayのresponse引数をそのファイルへ置き換える。

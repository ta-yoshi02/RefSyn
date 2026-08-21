# 評価比較方針メモ

## 目的

本メモは、MOLD の評価で使う実行時性能比較の候補を整理する。比較したい対象は次の 2 つである。

1. MOLD: 操作列により代入を主問題の骨組みへ分離し、差分部分だけを純粋関数 PBE として合成する。
2. 非分離 baseline: メソッド呼び出し前の環境を list 環境へ変換し、メソッド実行後の環境全体を PBE の出力として直接合成する。

率直に言うと、現時点で「既存合成器と比べて実行時が優位」と書くのは危ない。比較対象の入出力エンコードと部品集合が未固定で、baseline が成立するかをまだ検証していないためである。先に baseline を定義し、それが解ける範囲と解けない範囲を分ける必要がある。

## 現時点の確認結果

`escher-ts` の型・値表現には `Pair[A,B]` が存在する。

- `external/escher-ts/src/types/type.ts` に `TPair` / `tyPair` がある。
- `external/escher-ts/src/types/value.ts` に `ValuePair` / `valuePair` がある。
- JSON literal は `{ "pair": [left, right] }` で `Pair` を表現できる。
- type parser は `Pair[Int,Int]` や `Pair[List[Int],List[Int]]` を読める。

ただし、現行 RefSyn ブリッジは legacy 値変換として `Int`, `Bool`, `Ptr`, `List[Int]`, `List[Ptr]`, `Ref[...]` しか扱っていない。`Pair[...]` を返す実験を RefSyn 側からそのまま生成することはできない。

小さい実行確認では、次の結果になった。

| 確認内容 | 結果 | 含意 |
| --- | --- | --- |
| `Pair[Int,Int]` を `createPair` で返す | 成功 | `Pair` 出力自体は拒否されていない |
| `List[Int]` をそのまま返す | 成功 | list 出力自体も可能 |
| `Pair[List[Int],List[Int]]` を polymorphic `createPair` で返す | 失敗 | 標準の多相部品だけでは list 環境の束ね返しを期待できない |
| 単相 JS 部品 `pairLists : List[Int] -> List[Int] -> Pair[List[Int],List[Int]]` を追加 | 成功 | baseline 用の「束ねるだけ」の部品を明示すれば複合出力は可能 |
| `Pair[List[Int],List[Ref[Object[Node]]]]` を単相 JS 部品で返す | 成功 | value field と pointer field から作る heap list 表現も束ねられる |

この結果から、既存 engine は「list 環境のペアを絶対に返せない」わけではない。しかし、現行の MOLD/RefSyn 経路と同じ設定では返せない。比較実験には baseline 専用の変換器と、出力を束ねる単相部品が必要である。

## 比較として成立させる条件

比較を成立させるには、次の条件を固定する必要がある。

| 項目 | MOLD | 非分離 baseline |
| --- | --- | --- |
| 入力 | 呼び出し前環境、引数、操作列 | 呼び出し前環境、引数 |
| 出力 | 差分 hole の値や参照 | 呼び出し後の list 環境全体 |
| 合成対象 | 小さい純粋関数の集合 | 環境変換関数 1 個 |
| 代入 | 操作列から骨組みに反映 | list 出力として表現 |
| 実行時間の測定対象 | decomposition + helper synthesis + recomposition | whole-environment synthesis |

ここで重要なのは、baseline に操作列を与えないことではない。MOLD と比較したい差は「代入を操作列で分離するかどうか」なので、baseline は同じ実行前環境と期待される実行後環境だけから合成する形にするべきである。

## 推奨する baseline エンコード

最初は singly linked list に限定する。ここで `valueHeap` / `pointerHeap` と書くものは固定 field 名ではなく、Kanon の graph から検出した field 集合を決定的な順序に並べて作る list 環境の役割名である。現行ブリッジも `value_fields` / `pointer_fields` を検出し、`fieldHeapNames` で field 名と heap 名の対応を保持している。

入力:

```text
thisRef : Ref[Object[Node]]
arg0    : Int または Ref[Object[Node]]
valueHeap_<field>   : List[Int]
pointerHeap_<field> : List[Ref[Object[Node]]]
```

出力:

```text
Pair[List[Int], List[Ref[Object[Node]]]]
```

この出力型は、value field が 1 個、pointer field が 1 個の場合の最小形である。たとえば `val` と `next` なら、意味としては `Pair[valHeap, nextHeap]` になる。ただし `val` や `next` という名前を合成器に特権的に埋め込むのではなく、検出した field 名から実験用 task を生成する。

複数ポインタフィールドや複数値フィールドを扱う場合は、ひとまず nested pair にできる。

```text
Pair[List[Int], Pair[List[Ref[Object[Node]]], List[Ref[Object[Node]]]]]
```

ただし、nested pair はデータ構造プログラムと単純な Escher 型の相性の悪さをさらに強める可能性がある。理由は、更新対象でない field も含めて後状態全体を構成する必要があり、さらに出力のどの成分がどの field に対応するかが項の構造へ埋め込まれるためである。これは「見通しが悪い」という曖昧な話ではなく、baseline が field 数に応じて大きい直積的な出力を合成することになる、という問題である。

この点は MOLD の仮説そのものに近い。つまり、データ構造更新を list 環境全体の変換として合成すると、代入や不変部分の保持まで PBE の探索対象に入ってしまう。MOLD はそこを操作列で分離し、必要な差分だけを補助 PBE 問題にする。この仮説を評価で示すには、まず field 1 個 + pointer field 1 個の最小設定で比較し、その後に field 数が増えた場合を制限または発展課題として扱うのがよい。

## 必要な最小実装

実験に進むなら、最小実装は次で足りる。

1. `EscherCase` から post-state の value-field heap と pointer-field heap を抽出する baseline 変換器を追加する。
2. `Pair[List[Int],List[Ref[Object[Node]]]]` の task JSON を生成できる経路を作る。
3. 出力を束ねる単相部品を baseline にだけ追加する。
4. 合成時間、成功/失敗、生成項の cost、timeout を CSV に保存する。
5. MOLD 側も同じ例集合、同じ timeout、同じ maxCost で測る。

この実装は RefSyn 本流の recomposition へ混ぜない方がよい。baseline は論文評価用の実験コードであり、MOLD の生成経路とは役割が違う。

## 評価対象候補

最初に使うべき候補:

| メソッド | 理由 | 注意 |
| --- | --- | --- |
| `set` | 出力環境変化が局所的で baseline の sanity check に向く | 簡単すぎるので優位性主張には弱い |
| `append` | MOLD の既存 evidence が比較的強い | baseline は list 末尾探索と環境再構成を同時に学ぶ必要がある |
| `insert` / `insertAt` | 実行時差が出やすい | 入力例が弱いと固定位置の過学習になる |
| `popBack` | predecessor-of-tail が必要で比較として意味がある | 後状態の pointer-field heap 生成が baseline には重い |

現時点で避けるべき候補:

| メソッド | 理由 |
| --- | --- |
| `splitAt(i)` | 現在の evidence では成功例として扱えない。未評価または調査対象に留めるべき |
| `swapValue` | 対応関係の曖昧性があり、性能比較より分解限界の議論になりやすい |
| `rotateLeft` | snapshot と return timing の問題が混ざり、baseline 比較の主張が濁る |

## 論文上の主張ライン

言ってよい可能性がある主張:

- MOLD は代入を含むメソッド合成を、操作列から得た骨組みと純粋関数の補助 PBE 問題に分解する。
- 非分離 baseline では、同じメソッドを post-state list 環境全体を返す純粋関数として表現できる。
- 少なくとも singly linked list の一部メソッドでは、非分離 baseline は出力構造が大きくなり、探索対象が MOLD の補助問題より重くなる。

まだ言ってはいけない主張:

- 「既存手法では適用できない」。適用不能ではなく、環境エンコードと複合出力部品を足せば一部は適用可能である。
- 「実行時の優位性を確認した」。まだ測っていない。
- 「簡易性を確認した」。仕様記述難易度の比較基準が未定義である。
- `splitAt(i)` を成功例として扱うこと。現状では過大主張になる。

## 優先計画

1. baseline task generator を実験用に作る。
   - 入力: pre-state の field heap list、receiver、args。
   - 出力: post-state の `Pair[List[Int],List[Ref[Object[Node]]]]`。
   - verify: 小さい hand-written task で `set` 相当が解けること。

2. 束ね部品を明示的に固定する。
   - `pairHeap(valueHeap_<field>, pointerHeap_<field>)` のような単相部品にする。
   - verify: 検出した field heap を束ねる `pairHeap(...)` が合成されること。

3. `set`, `append`, `insert`, `popBack` の順で baseline を測る。
   - 先に簡単な `set` で変換器の正しさを確認する。
   - `append` 以降で MOLD との差を見る。
   - verify: 成功/失敗、runtime、timeout、program を保存する。

4. MOLD 側の同じケースを再測定する。
   - decomposition、helper synthesis、recomposition の合計時間を測る。
   - verify: 既存テストだけでなく、同じ payload から実行する。

5. 評価章では「成功例だけの性能表」と「baseline が失敗した理由」を分ける。
   - 成功例だけで速度比を出す。
   - 失敗例は性能比較ではなく、探索空間または表現力の限界として述べる。

## 評価成果物の保存方針

評価した関数ごとにファイルを分ける。実験時に「あとで足りない」と気づく資料を避けるため、合成結果だけでなく、入力プログラム、MOLD の中間結果、Escher-ts に渡した仕様、baseline 仕様、実行ログを同じ関数ディレクトリに保存する。

推奨レイアウトは [evaluation_cases/README.md](evaluation_cases/README.md) に置く。最低限、各関数について次を残す。

- 評価対象プログラム: `class Obj { ... }` 形式の元プログラムまたは穴あきメソッド。
- 仕様数: MOLD に渡した操作記録数、baseline に渡した入出力例数。
- MOLD 入力: Kanon payload、操作列、前後グラフ。
- MOLD 出力: common plan、hole 情報、補助 PBE 仕様、合成結果、再構成コード。
- backend 仕様: Escher-ts / AscendRec に渡した task JSON。
- baseline 仕様: 非分離 baseline の task JSON、追加した束ね部品、部品集合、timeout、maxCost。
- 実行結果: 成功/失敗、実行時間、timeout、有効な生成項、失敗理由。
- 不足資料: まだ取れていない evidence を明示した notes。

## 判断

実験は行う価値がある。ただし、今のまま比較に入ると「MOLD に有利な変換だけを作った」ように見える。baseline にも post-state を返すための最小限の表現能力を与え、その条件を明記した上で測るべきである。

再実装し直す必要はまだない。まずは `escher-ts` に実験用 task generator と単相 pair 部品を足して、非分離 baseline がどこまで解けるかを測る。もし `append` や `insert` が timeout するなら、それは評価結果として使える。ただし、timeout は「表現できない」ではなく「この部品集合と探索予算では解けない」と書く必要がある。

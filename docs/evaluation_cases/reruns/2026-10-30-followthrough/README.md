# 2026-10-30 評価の継続結果

主比較は現行設定（探索係数3、部品コスト1）、ベースラインは参照ヒープ型に合わせたJS走査部品を使う。
候補設定（係数1、走査部品コスト3）は追加実験として扱い、製品の既定値には採用していない。
実装と共有チェックアウトの既存変更は編集していない。

## 10回測定と同一入力での比較

各タスクの上限はコスト32、15秒。時間は準備と探索の合計の中央値で、通信や翻訳などを含まない。
未実演入力の領域は protocol.md を参照。prependは状態と戻り値をともに検査する。

| 操作 | MOLD 実演適合 | 比較手法 実演適合 | MOLD 中央値 ms | 比較手法 中央値 ms | MOLD 未実演一致 | 比較手法 未実演一致 |
| --- | --- | --- | ---: | ---: | --- | --- |
| setAt | 10/10 | 10/10 | 4.995 | 3083.092 | 42/42 | 42/42 |
| append | 10/10 | 0/10 | 1.925 | 15000.875 | 60/60 | 未合成 |
| prepend | 10/10 | 10/10 | 0.672 | 245.402 | 60/60 | 24/60（状態）、60/60（戻り値） |
| insert | 10/10 | 0/10 | 6.485 | 15000.081 | 210/210 | 未合成 |
| popBack | 10/10 | 10/10 | 0.974 | 6425.676 | 10/10 | 1/10 |

有限入力で全件一致した操作はMOLDが5/5、比較手法が1/5である。未合成の2操作は実行していない。
以前のMOLD検査も再実行し、実演の前提内97件は全件一致した。範囲外の添字や単一ノードのpopBackの失敗は残る。
全10回のMOLDの項は過去の測定と一致した（mold_term_reproduction.json）。
型修正後の参照項16個は全て型検査に成功し、手書き参照項は有限入力検査に全件一致した。
JS走査部品とライブラリ部品の比較も20,000入力で差分0だった。この有限比較は全入力での同値性の証明ではない。

## 候補設定のメソッド全体の実行

保存タスクから再探索し、固定版アダプタで翻訳した補助関数を保存済みのメソッドに組み込んで実行した。
Kanonへの反映とUI再実行は今回行っていない。

| ケースと入力領域 | 現行設定 | 候補設定 |
| --- | ---: | ---: |
| rotateRight3: trees | 64/257 | 257/257 |
| rotateRight3: graphs | 384/3072 | 3072/3072 |
| rotateRight4: trees | 97/257 | 257/257 |
| rotateRight4: graphs | 528/3072 | 3072/3072 |
| union: equal_rank_outside_contract | 0/25 | 0/25 |
| union: distinct_rank | 138/288 | 288/288 |
| union: reachable_rank_trees | 24/42 | 42/42 |

木の検査と、循環や共有参照を許すグラフの検査は別の領域である。
unionのdistinct_rankは人工的なrank割当てを含む。reachable_rank_treesの42組は通常のrankによる結合で構成できる木である。
同rankの25件は対象外の負例であり、完全なunionが解けたとは数えない。

unionの候補設定は10/10回で同じ項を得た。h0の探索時間は中央値1408.6 ms、最小1361.9 ms、最大1719.8 ms。
2秒制限で成功したが、計算機負荷や問題拡大に対する余裕は保証しない。

## 採用方針と残る作業

- 成否は実演適合と有限入力での一致を分けて示す。
- 論文用ベースラインはタスク生成スクリプトで型の合うJS部品を供給する。ライブラリへの正式追加は今回行わない。
- 新構造は現行設定と候補設定を併記し、unionは条件付きの部分操作と手法の限界を示す例にする。
- コスト3は事後選択であり、最適性や独立した一般化性能を主張しない。
- 論文本文は未編集。paper_revision_notes.mdに具体的な修正箇所を記録した。現行本文のMOLD上限14と測定上限32の不一致も修正対象である。
- 候補設定を標準に採用する場合の両手法の10回測定、新構造の広い独立評価、同rankを含むunionへの対応は未実施。
- 記録は codex/evaluation-followthrough の独立worktreeに隔離した。コミットとpushは行っていない。

## 再実行

固定版の escher-ts dist が external/escher-ts/dist にあるリポジトリルートから実行する。
provenance.jsonに元のdistのJSファイルとアダプタのSHA-256を記録した。

```sh
D=docs/evaluation_cases/reruns/2026-10-30-followthrough
python3 "$D/prepare_weighted.py"
sh "$D/run_measurements.sh" > "$D/measurement.log" 2>&1
WEIGHTED_DIST="$PWD/.evaluation-weighted/dist" node "$D/check_methods.mjs" > "$D/method_checks.log" 2>&1
WEIGHTED_DIST="$PWD/.evaluation-weighted/dist" node "$D/check_union_budget.mjs" > "$D/union_budget.json"
node "$D/check_paired_lists.mjs" > "$D/paired_lists.json"
python3 "$D/summarize.py"
```

主な証拠は paired_lists.json、各 *_checks.json、各 *_candidate.js、測定JSON、typecheck.json、port_checks.log、witness_checks.log。
すべてのケースの入力と期待値、実測値を保存している。参照実装もスクリプト内にある。

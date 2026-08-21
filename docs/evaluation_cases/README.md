# 評価ケース成果物レイアウト

評価結果は関数ごとにディレクトリを分ける。目的は、論文本文や付録で使う数値・仕様数・合成結果・失敗理由を後から追跡できるようにすることである。

## ディレクトリ構成

```text
docs/evaluation_cases/
  README.md
  <method-name>/
    README.md
    target_program.js
    mold_payload.json
    mold_operations.json
    mold_result.md
    mold_escher_tasks.json
    mold_results_measured.json
    mold_runtime.csv
    baseline_task.json
    baseline_fixed_id_task.json
    baseline_components.json
    baseline_fixed_id_results_measured.json
    baseline_fixed_id_runtime.csv
    notes.md
```

空ファイルを先に増やす必要はない。実験した関数から順に作る。

## 各ファイルの役割

| ファイル | 内容 |
| --- | --- |
| `README.md` | 関数単位の要約。対象メソッド、仕様数、比較条件、最終判定を書く |
| `target_program.js` | `class Obj { ... }` のような評価対象プログラム、穴あきメソッド、期待する手書き oracle |
| `mold_payload.json` | Kanon / MOLD に渡した元 payload |
| `mold_operations.json` | 操作列だけを抜き出した確認用ファイル。複数仕様なら仕様ごとに分けてもよい |
| `mold_result.md` | common plan、hole 情報、MOLD 合成結果、再構成コード |
| `mold_escher_tasks.json` | MOLD が補助 PBE 問題として Escher-ts / AscendRec に渡した task JSON |
| `mold_results_measured.json` | MOLD 補助 PBE 問題の測定結果。試行ごとの時間、成功/失敗、生成項 |
| `mold_runtime.csv` | MOLD 補助 PBE 問題の測定値を表計算しやすくした CSV |
| `baseline_task.json` | 旧実験や診断用 baseline の task JSON。正式評価では名前付き task を優先する |
| `baseline_fixed_id_task.json` | 操作で分離しない固定 ID 順 baseline の task JSON。post-state の list 環境全体を返す仕様 |
| `baseline_components.json` | baseline に追加した束ね部品、部品集合、timeout、maxCost などの条件 |
| `baseline_fixed_id_results_measured.json` | 固定 ID 順 baseline の測定結果。試行ごとの時間、成功/失敗、生成項 |
| `baseline_fixed_id_runtime.csv` | 固定 ID 順 baseline の測定値を表計算しやすくした CSV |
| `runtime.csv` | 旧実験や診断用の実行時間。正式評価では名前付き runtime CSV を優先する |
| `notes.md` | 足りない evidence、過学習の疑い、field 数や例分布の偏り、論文に書けない制約 |

## 関数 README の最小項目

```markdown
# <method-name>

## Target

- Data structure:
- Method signature:
- Expected behavior:

## Specifications

- MOLD operation records:
- Baseline input/output examples:
- Fields detected:
- Field heap order:

## MOLD Result

- Common plan:
- Hole specs:
- Backend solved:
- Recomposition:
- Runtime:

## Baseline Result

- Output encoding:
- Components:
- Backend solved:
- Runtime:
- Failure reason:

## Paper Use

- Main-table label:
- Appendix evidence:
- Claims allowed:
- Claims not allowed:
```

## Field 名の扱い

`val`, `next`, `prev` などの field 名は評価対象プログラムから検出される。評価コードや記録では、検出した field 名と生成した heap list の対応を必ず保存する。

例:

```text
value fields:   ["val"]
pointer fields: ["next"]
field heap order: ["val", "next"]
output encoding: Pair[valHeap, nextHeap]
```

この対応を残さないと、baseline の出力ペアの各成分が何を意味するのか分からなくなる。そこを曖昧にした評価は論文に使えない。

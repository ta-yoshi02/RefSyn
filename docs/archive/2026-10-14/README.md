# 隔離した文書

2026-10-14に、過去の設計や検証条件を現在の仕様と取り違えないため、次の資料を元の相対パスを保って移動した。
13ファイルの本文とバイナリは変更していない。
文中のパス、リンク、実行指示、確認済みという記述は、作成当時の情報として読む。
現在の修正は [文書の入口](../../README.md) と論文本文から進める。

| 元のパス | 隔離理由 |
| --- | --- |
| `INTEGRATED_SYNTHESIS_API.md` | 削除済みの操作差異分析と旧リクエスト形式を説明している |
| `doc/AssociationGraph_MaximumClique.md` | 最大共通部分探索の参考資料。論文の全体同型の実装指示ではない |
| `doc/VF2_Algorithm.md` | アルゴリズムの参考資料。現実装の多重辺の扱いや対応選択を定義する資料ではない |
| `doc/ex_abstract.tex`、`doc/ex_abstract.pdf` | 現在参照する論文プロジェクトとは別の原稿と出力 |
| `docs/PAPER_ALIGNED_REQUIREMENTS.md` | 置換前の監査、要件草案、置換結果が混在している |
| `docs/SYNTHESIS_TEMPLATE_CONTRACT.md` | 実装独自の条件を含む。先行穴の出力例を使う方式などを論文の規則として扱うと修正を妨げる |
| `docs/IMPLEMENTATION_REVIEW_PROMPT.md` | 旧worktree、旧固定版、削除済みファイルを対象としたレビュー指示 |
| `docs/REFSYN_TO_ESCHER.md` | #23以前のnull読出しと失敗時の意味の説明が残る。全穴の問題を先に作る実装方針も含む |
| `docs/KANON_CODEX_VERIFICATION.md` | 過去のUI操作環境に依存する手順。現在の記録から再実行までの証拠と分ける |
| `docs/EVALUATION_BASELINE_PLAN.md` | 過去の比較計画と当時の進捗。新経路の評価条件として再利用しない |
| `docs/NONSEPARATED_BASELINE_WITNESSES.md` | 過去のbaseline表現可能性の分析。新経路の測定結果とは別 |
| `docs/PAPER_CASE_STUDY_AND_LIMITS_NOTES.md` | 置換前のケース分析と削除済みテストへの参照 |

評価データ自体は移動していない。
`tests/whole_method_validation_tests.rs` が `docs/evaluation_cases/` の入力を参照しているためである。
既存の未追跡資料、サブモジュール、実装コードも変更していない。

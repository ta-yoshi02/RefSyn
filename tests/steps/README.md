# 合成の各段階を確認するサンプル

リポジトリのルートから実行する。escher-ts のビルドと Node.js が必要。

- `cargo test --test check`: 記録 JSON から生成した PBE タスクの入力と出力を検査する。探索は実行しない。
- `cargo test --test step_read -- --nocapture`: 記録を読み込んだ結果を表示する。
- `cargo test --test step_stages -- --nocapture`: 操作グラフ、対応、雛形、穴の問題を表示する。対応の比較以外は出力を観察するためのサンプルであり、各段階の正しさを assertion で保証するテストではない。
- `node tests/steps/00-insert.mjs` から `03-setat.mjs`: その場で合成したメソッドを手書きの参照実装と有限入力で比較する。各ファイルを個別に実行する。
- `node tests/steps/step-translate.mjs`: setVal の穴の問題を解き、得られた項と翻訳結果を表示する。
- `node tests/check.mjs`: 貼り付けた過去の insert 合成例と手書き版を連続呼び出しで比較する。現在の合成器の出力を検査するものではない。

`01`〜`03` の JSON は手書きの記録であり、Kanon UI の操作を検証しない。参照実装との比較は記載された有限入力での構造的な一致を検査し、一般的な同値性や全オブジェクトの参照同一性を証明しない。

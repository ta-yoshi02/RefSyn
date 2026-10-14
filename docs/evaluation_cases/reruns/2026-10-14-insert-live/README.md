# Kanonで記録したinsertの実験

取得日：2026-10-14（Asia/Tokyo）。
ユーザーが実演と合成を行った後の、Codex terminalと `http://localhost:8000/` の既存画面を取得した。
ページの再読込み、エディタの変更、合成の再実行は行っていない。

## 取得した記録

| ファイル | 内容と限界 |
| --- | --- |
| [terminal-snapshot.txt](terminal-snapshot.txt) | Codex appが保持する直近のterminalバッファ。先頭が切れている。取得ツールの注記を含み、CRLFはLFに揃えた。完全なraw requestではない |
| [escher-tasks.json](escher-tasks.json) | terminalに記載された `target/escher/ts/insert_1791942373_349922000_p63926_0.json` のコピー |
| [validation-snapshot.json](validation-snapshot.json) | terminalの末尾側に完全に残っていた `validation` オブジェクト。version 3、3実演、クラス定義と生成直後状態を含む。検証の成否ではない |
| [editor-source.js](editor-source.js) | Aceの表示行31行から取得した反映後ソース |
| [editor-dom.txt](editor-dom.txt)、[editor-evidence.json](editor-evidence.json) | エディタの表示と10行目の `ace_warning` の記録 |
| [editor.png](editor.png) | 取得時のKanon画面 |
| [browser-console.json](browser-console.json) | ブラウザが保持する直近100件までのconsoleログ。objectの詳細が展開されていない出力もある |
| [independent-replay.json](independent-replay.json) | 保存したエディタソースをNode VMで独立に実行した結果。実行上限1000ms |
| [provenance.json](provenance.json) | 取得時のcheckoutのSHAと変更状態。起動済みサーバのバイナリがそのSHAから作られたことまでは確認していない |
| [SHA256SUMS](SHA256SUMS) | 保存ファイルのハッシュ |

`editor-source.js` は取得した表示ソースとして保存し、空行の末尾空白も保持している。
文書整理時の整形で取得内容を変えないためである。

## 確認できた結果

実演の引数は `insert(0,89)`、`insert(2,46)`、`insert(1,71)` である。
terminalにはPBEが3成功、0失敗と記録されている。
穴は順に `arg`、`nthNextRef(this,i)`、先行穴で得た参照の `next` を返す。
生成した本体は、生成1文、穴の束縛3文、書込み3文の順である。
これらの補助メソッドと本体が実際のエディタに反映されていることを確認した。

保存したソースをNodeで独立に実行し、リストが `[39, 89, 71, 27, 46, 53]` となることを確認した。
これは今回の3呼出しに対する値列と終端までの非循環性の確認であり、未観測入力全般や全aliasの一致の証明ではない。

## 10行目の警告

Aceの10行目にwarningマーカーがある。
警告文 `Don't make functions within a loop` はユーザーの報告によるもので、今回のDOM取得では文言自体を取り出せていない。
当該行の `insert_h1` は、`while` の中で未定義fieldを拒否する関数を作り、その場で呼び出している。
後で呼び出すclosureを保存する構造ではない。
警告は実行時例外とは別で、今回保存したコードの独立実行は成功した。
生成コードの不要な関数生成を減らすかどうかは、失敗時の意味を保った別の改善として扱う。
警告を消すためのコード変更は今回行っていない。

## 未取得の証拠

terminalバッファの完全なraw request、HTTP応答の `validation.status` と検査件数、ブラウザの各テストの合否の詳細は保存できていない。
反映後の呼出しとhelperが実行トレースに現れているが、今回の画面だけで全テストの合格とは断定しない。
新しい実験記録ではあるが、固定版の再ビルドから全操作を記録した評価結果や、実行時性能の測定ではない。

# 固定構成での新規記録（2026-10-14）

構成は [setup/provenance.txt](setup/provenance.txt) のとおりである。
RefSyn `af76b2e`、Kanon `60fa5c8`、escher-ts `4e7c4bc` のworktreeを使った。
追跡対象のソース変更はなく、依存のインストールで生成した未追跡のlockfileがある（`setup/provenance.txt` のgit statusを参照）。
記録時のサーバはそのworktreeのreleaseビルドで、PIDは90770、ポートは3030だった。
Kanonはそのworktreeを静的に配信し（`http://localhost:8100/`）、Claude Codeの内蔵ブラウザで開いた。
ページには [setup/record-hooks.js](setup/record-hooks.js) を読込み後に注入した。このスクリプトは、関数の引数と戻り値を変えずに記録だけを行う。
実演はユーザーが行った。

## 記録の終了とサーバの切替

[setup/server-start.txt](setup/server-start.txt) に、PID 90770の終了コード144と、14:21:30の再起動失敗が記録されている。
3030番は別checkoutのサーバが使用していたため、固定版サーバは再起動できなかった。
終了の原因と、誰が停止したかはこの記録からは特定できない。
以下のcase1と再送はPID 90770の記録であり、別サーバへの送信結果を追加していない。
本記録は `af76b2e` のinsertについての記録として閉じ、#29後の主要5ケースの評価とは分ける。

`setup/server.log` と `case1-capture/server.log.snapshot` は、この保存時点では同じ内容である。
取得したソースとログの末尾空白、末尾の空行は、取得内容を変えないため保持している。
`SHA256SUMS` はREADMEと起動記録の訂正後の保存ファイルを対象とし、稼働中のログに追従するものではない。
次の確認では、このディレクトリ内で実行する。

```bash
shasum -a 256 -c SHA256SUMS
```

## case1：insert（論文の例）

### 実演の内容

- プログラムは `class Node { constructor(v) { this.next = null; this.val = v; } insert(i, arg) {} }` である。リスト `39 → 27 → 53` に対し、`insert(0,89)`、`insert(2,46)`、`insert(1,71)` を実演した。
- 保存済みの評価ケース `insert_general` は、クラスが `Obj`、fieldが `f`/`g`、引数が `i`/`b` のプログラムを使っている。名前の付け方が違うので、今回の記録はそれと同一のプログラムではない。

### 保存したもの

| ファイル | 内容 |
| --- | --- |
| `case1-capture/request.json` | Kanonが送った要求本文そのもの（26206 bytes）。version 3の型付き状態と操作列を含む |
| `case1-capture/response.json` | そのときの応答。`validation` は `passed`、検査した実演は3件 |
| `case1-capture/page-record.json` | ページ側の記録。ID正規化の各関数の呼出し前後の引数と戻り値、`console.warn`、要求本文と応答 |
| `case1-capture/editor-after.js` | 反映後のエディタのソース |
| `case1-capture/escher-tasks.json` | サーバが保存したtask（`…_p90770_1.json`） |
| `case1-capture/server.log.snapshot` | 取出し時点のサーバの全出力。最初の合成は、記録開始前の動作確認（`setup/smoke-*`）による |
| `case1-resend/` | `request.json` を同じサーバへcurlで再送した結果 |
| `case1-replay/` | 反映後のソースをNode VMで実行し、helperの戻り値をtaskの出力例と照合した結果 |

独立実行は、このディレクトリから次のコマンドで行う。

```bash
node case1-replay/replay.mjs case1-capture/editor-after.js case1-capture/escher-tasks.json
```

### 確認結果

- **再送**：`validation` は元と同じ `passed`（3実演）だった。`code`、`composed_method_code`、`escher_results`、`hole_checks`、`hole_information`、`individual_codes`、`common_pattern` も元と一致した。違ったのは `list_environment_info` 中のtaskファイル名（時刻）だけで、2つのtaskファイルはバイト単位で同一だった。
- **反映**：応答のhelper 3つと7文の本体は、反映後のエディタのソースにそのまま含まれる。
- **独立実行**：結果は `[39, 89, 71, 27, 46, 53]` で、helperの戻り値9件はすべてtaskの出力例と一致した。
- **ID正規化**：
  - `idMapping generation failed` は、call8〜call10 のそれぞれで出た。
  - 照合候補は空ではなかった。例えばcall8では `{"__temp1": "main-new6"}` が作られている。しかし、どの候補の対応先にも実行時ID（`-call` を含むID）がなく、`sanitizeIdMapping` がすべて捨てて `null` を返した。
  - `runtimeToTempMap` と `runtimeAliasMap` は空のままで、`normalizeIdWithRuntimeMap` が書き換えたIDは0件、操作列も正規化の前後で同一だった。
  - したがって、この記録では、正規化すべき実行時IDがなかったことが警告の原因である。ID正規化は要求の内容を変えていない。
  - なお、call8の候補 `__temp1 → main-new6` は、新しく作った頂点を既存の頂点へ対応させており、正しい対応ではない。今回は捨てられたので影響はないが、実行時IDが現れる記録では確認が要る。

### この記録が示さないこと

- 実行時性能の測定ではない。
- 実演していない入力での正しさは示さない。
- 保存済み `insert_general` の入力の再現ではない。

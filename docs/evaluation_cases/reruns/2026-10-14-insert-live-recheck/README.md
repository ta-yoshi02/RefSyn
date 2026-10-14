# insert実験（2026-10-14）の独立確認

[元の記録](../2026-10-14-insert-live/README.md) を変更せず、保存された証拠を照合し、一部を再実行した結果である。
元の記録とは別の結果として扱う。

## 実行条件

- RefSyn `af76b2edad6f954498ce147c0e74afcfef75435b` のクリーンなworktree（`git worktree add`、未コミット変更なし）
- escher-ts `4e7c4bca45d98a0b5f2f76fd70486ed823c731f1` を `pnpm install --frozen-lockfile && pnpm build` でビルド
- Node v22.17.0
- 手元checkoutのescher-ts `dc13e6a` は、`4e7c4bc` とツリー（`9d17335`）が同一で、`git diff 4e7c4bc dc13e6a` も空であることを確認した

## ファイル

| ファイル | 内容 | コマンド |
| --- | --- | --- |
| `consistency.json` | 元記録の相互照合。`validation-snapshot.json` がterminal中の `validation` と一致、terminalの補助メソッド3つがエディタソースに含まれる、再実行の結果がterminalとエディタソースに一致 | 確認用の一時スクリプト（保存していない） |
| `escher-rerun.json` | 保存された `escher-tasks.json` を固定版escher-tsで再合成した結果。3穴とも成功し、式と翻訳JSは元の記録と同じ | `node scripts/run_escher.js --file ../2026-10-14-insert-live/escher-tasks.json` |
| `replay.mjs`、`replay-result.json` | 保存したエディタソースをNode VMで実行し、各呼出しの補助メソッドの戻り値を、PBE taskの出力例（参照は呼出し前リスト上の位置）と照合した | `node replay.mjs ../2026-10-14-insert-live/editor-source.js ../2026-10-14-insert-live/escher-tasks.json` |

## 稼働していたサーバについて（取得時点の観察）

- 3030番で待ち受けていたのはPID 63926（`target/debug/refsyn`、親はzsh、10:43:36開始）で、taskファイル名の `p63926` と一致する。
- 実行中のイメージと手元の `target/debug/refsyn` は同じinode（85493490）だった。
- そのバイナリは #28 で追加された文字列（`holds both booleans and integers`、`in a boolean field`）を含み、#28 で削除された文字列（`boolean-valued holes are not supported yet`）を含まない。
- これは、真偽値対応に関する #28 の変更の一部を含むソースからビルドされた、という判断を支える手掛かりにとどまる。#28の実装全体を含むか、ビルド元のcommit、未コミット変更の有無は特定できない。debugビルドはSHAを埋め込んでいない。
- 補助メソッドの合成と翻訳は、要求ごとに手元checkoutの `scripts/run_escher.js` と `runtime/` と `external/escher-ts/dist` を読み込む。これらの取得時点の内容は記録されていない。

## 確認できないこと

- 当時の `validation.status` と検査件数。HTTP応答が保存されていない。
- 当時の元リクエストによる全体検証の再現。検証器（`runtime/whole-method-validation.mjs`）は、呼出し前状態と操作列から期待状態を作るので、呼出し後のsnapshotは必須ではない。足りないのは操作列を含む完全な元リクエストで、terminalの先頭が欠けているため手元にない。操作列を再構成して検証する場合は、当時の結果ではなく別の結果として保存する。今回は再構成していない。
- Kanon上の各テストの合否。

## `idMapping generation failed` の警告

- `browser-console.json` には、call1〜call3 のそれぞれについて `[Testize] idMapping generation failed …; empty idMapping is sent` が記録されている。
- Kanonは送信前に `idMapping` を使い、receiver、操作、snapshotのIDを、実行時のID（`-call` を含むID）から実演時の `__temp` IDへ正規化する（`Kanon/src/js/testize.js` 1205〜1243行付近）。RefSyn側が送信済みの `idMapping` を使わないことは、この警告が無害である根拠にはならない。
- `sanitizeIdMapping` は、`__temp` IDを実行時IDへ対応させる組だけを残す。対応候補が無いか、実行時IDを1つも含まなければ、空として警告を出す。
- call3：terminalに残る実行後グラフのIDは `main-new1`、`__temp1`、`main-new2`、`__temp4`、`main-new3` と値の頂点だけで、実行時IDを含まない。terminal全体にも `-call` は現れない。したがってcall3の警告は、正規化すべき実行時IDがなかったことと整合する。ただし、照合自体が失敗したのか、照合は成功して残す組が無かっただけなのかは、保存された情報からは区別できない。
- call1・call2：実行後グラフが残っていないため、必要な対応が無かったのか、生成に失敗したのかは判断できない。
- 言えるのは、この警告が出た状態でも、今回の要求は合成でき、RefSynの読込み（未知のIDを拒否する）を通ったことまでである。

## 10行目の警告

- 警告文はAceのJavaScript worker（JSHint）の `W083: "Don't make functions within a loop."` にあたる。この文言は `Kanon/node_modules/ace-builds/src-noconflict/worker-javascript.js` に含まれる。今回の取得で文言自体は取り出せていない。
- 10行目の `insert_h1` は、`while` の各反復で未定義チェック用のアロー関数を作り、その場で1回呼ぶ。
  - **実行時エラー**：ではない。独立実行で例外は起きない。
  - **closureの意味の問題**：起きない。この関数は引数だけを参照し、ループ変数を捕捉せず、後で呼ばれることもない。
  - **不要な関数生成**：ある。反復ごとに関数objectを1つ作る。
- 警告を消すための生成コードの変更は行っていない。

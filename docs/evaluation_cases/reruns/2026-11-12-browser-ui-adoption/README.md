# 実ブラウザでの実演記録から反映まで（2026-11-12）

Kanon の画面で実演を記録し、ブラウザ内の Worker で合成・全体検証し、エディタに反映するまでを、実ブラウザで通した記録である。
公開 Pages ではなく、ローカルで配信した Pages ビルドを使った。手動の操作記録であり、自動テストではない。

## 構成

| 項目 | 値 |
| --- | --- |
| Kanon | `8876576`（ta-yoshi02/Kanon#10 の Worker 失敗処理を含む）を `npm run build-refsyn-runtime` と `npm run build-pages` でビルドした `dist/` |
| RefSyn（WASM と runtime） | main `16edf05` の `src` から `node web/scripts/build-wasm.mjs`（wasm-bindgen 0.2.114）。#49 の変更はネイティブ側だけで WASM に入らない |
| escher-ts | `4e7c4bc` の `dist` |
| 配信 | `python3 -m http.server`（`127.0.0.1`） |
| ブラウザ | Claude Code の内蔵ブラウザ（Chrome 156 系）。表示を 1440×900 に設定 |
| 合成の経路 | `refsyn-browser-runtime.js` → Worker チャンク `793-….js`（WASM、escher-ts、翻訳）→ 入れ子の検証 Worker。`localhost:3030` のサーバは使わない |

## 操作の範囲

実際の UI 操作で行ったこと：

- エディタ上の呼出しへのホバーと「set test」のクリック
- 実演窓の Edit、Add Node、Add Edge のクリック
- キャンバス上のクリック（ノードの配置）とドラッグ（辺）
- ポップアップへのラベルの入力、Literal のチェック、save
- accept、Synthesize ボタン
- 合成中のエディタへの打鍵

スクリプトで行ったこと（UI の意味は変えない）：

- エディタへのプログラムの設定（`editor.setValue`）
- 重なったノードの表示位置の移動（`network.moveNode`）と表示の調整（`fit`）
- 状態の読取り（`storedTest`、`focusedTestOperations`、`lastSynthesis`、エディタの内容）
- 3回目の実行では、送信内容を読むために `dispatchSynthesisRequest` を包んだ。引数をそのまま元の関数へ渡している

## 実演

プログラムは 2026-10-14 の記録（`../2026-10-14-recording/`）と同じ `class Node { constructor(v) { this.next = null; this.val = v; } insert(i, arg) {} }` に、リスト `39 → 27 → 53` と `insert(0, 89)`、`insert(2, 46)`、`insert(1, 71)` である。
各呼出しの実演前状態には、それより前の実演の結果が反映されていた（call3 の前は `39 → 89 → 27 → 46 → 53`）。
記録された操作列は 10 月の記録と同じだった。

| 呼出し | 操作列 |
| --- | --- |
| call1 `insert(0, 89)` | addNode Node、addNode 89、addEdge val、addEdge next、editEdgeReference next |
| call2 `insert(2, 46)` | 同上（46） |
| call3 `insert(1, 71)` | addNode Node、addNode 71、editEdgeReference next、addEdge val、addEdge next |

## 結果

| 実行 | 条件 | `lastSynthesis` | エディタ |
| --- | --- | --- | --- |
| 1 | 3実演で Synthesize | `applied: true`、`reason: null`、`validation: passed`（3/3）。クリックから約2.7秒 | helper 3つと7文の `insert` に置き換わった。Kanon の再実行で3呼出しとも `testPassed`、実行後のリストは `[39, 89, 71, 27, 46, 53]` |
| 2 | 実行1の反映後に、同じ実演のまま再度 Synthesize | `applied: false`、`reason: validation unavailable`。`list_environment_info` は `Kanon could not record the pre-call state: Existing method changed the heap before demonstration edits` | 変わらない。合成前の拒否であり、実行3の経路とは別 |
| 3 | 空の `insert` に戻して3実演を入れ直し、Synthesize の直後にエディタへ `// edited during synthesis` を打鍵 | `applied: false`、`reason: source changed during synthesis`。応答は `validation: passed`（3/3）で `composed_method_code` を含んでいた | 打鍵した1行だけが増え、それ以外は合成前と同じ。helper は入らない |

実行2は狙った経路ではなかったので、実行3で狙いの経路（検証済みの応答を、ソースの変更を理由に反映しない）を確かめた。

## 見つかったこと

- **Kanon の不具合**：ツールバーの Add Node のポップアップが前回の Literal チェックを解除しない。リテラルの直後に追加した `Node` が `{"isLiteral": true, "type": "string"}` として記録された。その実演は破棄してやり直した。ta-yoshi02/Kanon#11 で修正し、修正後のビルドで解除されることを同じ手順で確かめた。
- **操作上の注意**：ドラッグの開始点がノードを外れるとキャンバスが動き、辺は追加されない。既存のフィールドに辺を追加する操作（`editEdgeReference` になる）の後は編集モードが外れる。10 月の dll-unlink の記録にも、辺の端のドラッグでキャンバスが動いた記述がある。
- Worker のチャンクを消した場合の失敗処理は、Kanon#10 に記録した（ランタイムを直接呼ぶ確認であり、この UI の経路では確かめていない）。

## 示さないこと

- 公開 Pages（https://ta-yoshi02.github.io/Kanon/）での動作。公開版は 2026-10-16 の配備で、この構成を含まない。
- 自動化された回帰テスト。再実行は下の手順を手で行う。
- 他のブラウザ、他の事例、実演していない入力での正しさ。

## 再現

1. RefSyn で `node web/scripts/build-wasm.mjs`（`external/escher-ts/dist` が必要）。
2. Kanon で `REFSYN_DIR=<RefSyn> npm run build-refsyn-runtime && npm run build-pages`。
3. `python3 -m http.server <port> --bind 127.0.0.1 --directory dist` で配信し、ブラウザで開く。
4. 上のプログラムを入力し、各呼出しにホバーして「set test」から上の操作列のとおり実演し、accept する。
5. Synthesize を押し、`__$__.Testize.lastSynthesis` とエディタを確かめる。

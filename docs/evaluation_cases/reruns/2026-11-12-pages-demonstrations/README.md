# 公開 Pages での実演と合成（2026-11-12、マージ後）

RefSyn #49〜#52 と Kanon #10・#11 のマージ後に再配備された公開 Pages（https://ta-yoshi02.github.io/Kanon/）で、Kanon の画面から実演して合成した記録である。
手動の操作記録で、自動テストではない。操作の範囲と手順は [2026-11-12-browser-ui-adoption](../2026-11-12-browser-ui-adoption/README.md) と同じで、今回は配信元が公開 Pages である。

## 配備の確認

| 項目 | 値 |
| --- | --- |
| Pages の配備 | Kanon Actions「Deploy GitHub Pages」run 43165023352（build、deploy とも success）。`refsyn-browser-runtime.js` の Last-Modified は 2026-11-12 06:03:05 GMT |
| 使われたリビジョン | Kanon `451d764`（#10、#11 を含む）、RefSyn `afb6257`（#49〜#52 を含む main）、escher-ts `dc13e6a` |
| 配備時の検査 | 同じ run の `verify-refsyn-browser` が `status: passed`、`checked_demonstrations: 3` |
| 公開ファイルの中身 | `refsyn-browser-runtime.js` に Kanon#10 の失敗処理の文言（`did not finish within`、`it could not be loaded or crashed`、`sent a message that could not be read`）がある。`src/js/testize.js` に Kanon#11 の Literal 解除がある |
| 画面での確認 | Literal をチェックしたままの状態から Add Node を開くと、未チェックで開いた（Kanon#11 の挙動） |

RefSyn main の CI は最後の push（#52 のマージ）で success。それ以前の3件は同じブランチの後続 push による cancel（`cancel-in-progress`）。Kanon の Test と Pages も最後の push で success、#10 のマージ時の run は #11 のマージで cancel された。

## プログラムと期待

```js
class Node { constructor(v) { this.next = null; this.val = v; } insert(i, arg) {} }
let list = new Node(39); list.next = new Node(27); list.next.next = new Node(53);
list.insert(0, 89); list.insert(2, 46); list.insert(1, 71);
```

`insert(i, arg)` は、先頭を 0 番目として i 番目のノードの後ろに値 arg のノードを挿入する。期待する最終リストは `[39, 89, 71, 27, 46, 53]`。
各呼出しの実演は、10 月の記録（`../2026-10-14-recording/`）と同じ操作列で行った。

## 結果

| 実演 | 全体検証 | 反映 | 合成された helper（要旨） | 3呼出しを実行した結果 |
| --- | --- | --- | --- | --- |
| call1 だけ（1件） | passed、1/1 | した（`applied: true`） | 値 = arg、直前 = `this`、後続 = 末尾の1つ前（`penultimateRef(this)`） | `[39, 71, 27, 53]`。89 と 46 のノードが切り離される |
| call1、call2（2件） | passed、2/2 | しなかった（反映を止めるため合成中にエディタへ `//` を打鍵。理由 `source changed during synthesis`）。応答のメソッドを別に実行した | `i === 0` なら直前 = `this`・後続 = 末尾の1つ前、そうでなければ直前 = 末尾の1つ前・後続 = 末尾 | `[39, 89, 27, 46, 71, 53]`。71 が末尾の手前に入る |
| call1〜call3（3件） | passed、3/3（クリックから約3.2秒） | した | 値 = arg、後続 = i+1 個先、直前 = i 個先 | `[39, 89, 71, 27, 46, 53]`。Kanon の再実行で3呼出しとも `testPassed` |

1件と2件の実演では、合成は成功し、全体検証も通り、1件のときはエディタに反映された。
しかし、それらのメソッドは実演した呼出し以外で意図と異なる結果になる。
全体検証が確かめるのは与えた実演を再現することだけであり、実演が少ないと、実演を満たす別の一般化（末尾の1つ前、`i === 0` での分岐）が選ばれる。
これは手法の性質であり、利用者は実演を追加するか、反映後の実行結果を確かめる必要がある。
3件の helper は、10 月の記録（RefSyn `af76b2e`、直前 = `nthNextRef(this, i)`、後続 = 直前の `next`）と式の形が違うが、この3呼出しでは同じ結果になる。

## 示さないこと

- 実演していない入力全般での正しさ（3件のメソッドについても、この3呼出しでの一致だけを確認した）。
- 他のブラウザ、他の事例。
- Worker の読込失敗を画面の経路で起こした場合の表示（ランタイム単体では Kanon#10 で確認した）。

# Codex による Kanon UI 検証手順

このメモは、Codex がローカルの Kanon 画面を操作して、Kanon の `set test` 操作列から RefSyn 合成までを確認するための手順をまとめる。

## 前提

- Kanon は `Kanon/` で `npm start` 済みで、`http://localhost:8000/` が開けること。
- native backend を使う場合は RefSyn 側で `cargo run` を起動し、`http://127.0.0.1:3030/synthesize` が受けられること。
- browser runtime が読み込まれている Kanon では、`runRefsynBrowser` 経由で in-browser 合成され、localhost backend に fallback しない場合がある。
- Codex はユーザーの VSCode terminal を直接見られない。`handle_synthesis` の `--- RAW REQUEST PAYLOAD ---` を確認する必要がある場合は、ユーザーに terminal 出力を共有してもらう。

## Codex 側の観測方針

1. in-app browser は Browser Use の `iab` backend で操作する。
2. ユーザーがすでに `set test` を入れた直後は、むやみに reload しない。保存済みテストや editor 状態が消える可能性がある。
3. 画面確認は DOM snapshot を主に使う。Kanon 画面では screenshot が timeout することがある。
4. console log は古いログが残ることがある。`var __temp...` のログは参考にはなるが、新規操作列の完全な証拠としては扱わない。
5. `Synthesize` 前に、最低限次を確認する。
   - test modal が開いたままではないこと。
   - editor に対象メソッド呼び出しが残っていること。
   - 直前の失敗表示、特に `Runtime Error: this.append_f is not a function` が残っていないこと。

## Kanon UI で仕様を入れる時の注意

`append(26)` のような list append を仕様として入れる場合、期待グラフ操作は少なくとも次の形になる。

- 新しい object node を追加する。
  - label: `Node` または現在の class 名
  - `isLiteral`: unchecked
- 新しい literal node を追加する。
  - label: `26` などの値
  - `isLiteral`: checked
- 既存の末尾ノードから新 object node へ `next` edge を追加する。
- 新 object node から literal node へ `val` edge を追加する。

数値ノードで `isLiteral` を入れ忘れると、Kanon は値ではなく class/object として扱う。過去の失敗例では console に次のようなログが残った。

```text
var __temp1 = new Node();
var __temp2 = new ();
```

これは literal label が空の class/object node として保存された強い兆候であり、その状態の合成結果は信用しない。

## `Synthesize` 実行後の確認

append の健康な合成結果は、おおむね次の構造を持つ。

```js
append_f(arg0) { return arg0; }

append_h(arg0) {
    // this から next をたどって末尾 node を返す helper
}

append(arg) {
    const h_ptr_0 = this.append_h(arg);
    const h_int_0 = this.append_f(arg);
    const tmp0 = new Node();
    tmp0.val = h_int_0;
    if (h_ptr_0 !== null) { h_ptr_0.next = tmp0; }
}
```

class 名は editor の定義に応じて `Node` ではなく `ListNode` などになる。

成功寄りと判断できる最低条件:

- `Runtime Error` が表示されていない。
- `append_f` など、本体から呼ばれている helper が定義されている。
- 新規 node 作成がある。
- `tmp0.val = ...` がある。
- `tail.next = tmp0` 相当の代入がある。

注意点:

- `append_f`, `append_g`, `append_h` の suffix は意味名ではなく、spec 生成順に `f`, `g`, `h`, ... と採番される。
- 本体から呼ばれていない helper が残る場合がある。これは helper 掃除の問題であり、合成本体の成否とは分けて判断する。
- `append_f(arg0) { return arg0; }` は一見余計だが、`tmp0.val` に入れる値を返す value-hole として使われているなら未使用ではない。

## 失敗時に見る場所

- Kanon UI:
  - editor の合成結果
  - `Runtime Error` 表示
  - `set test` / `modify` tooltip の状態
- browser console:
  - `var __temp...` の保存ログ
  - `合成結果:` と `個別メソッド呼び出しのコード:`
- RefSyn terminal:
  - `--- RAW REQUEST PAYLOAD ---`
  - `method_calls[].operations`
  - `method_calls[].actualGraph`
  - `method_calls[].idMapping`
- 生成 artifact:
  - `target/escher/ts/*.json`
  - `list_environment_info` に出る task JSON path

## 代表的な切り分け

- `var __temp2 = new ();`
  - label 未入力または `isLiteral` チェック漏れ。UI 入力の失敗として扱う。
- `Runtime Error: this.append_f is not a function`
  - composed method が未定義 helper を呼んでいる。helper spec の coverage / rename / postprocess を疑う。
- `append_g(arg0) { return arg0; }` だけが本体から呼ばれていない
  - 未使用 helper が `response.code` から掃除されていない。合成本体とは別問題。
- `append_h` が `last_ptr` 相当ではなく固定 hop になる
  - 複数仕様が不足しているか、tail predecessor の spec がうまくできていない。`target/escher/ts/*.json` の Ptr-returning task を確認する。

## 直接 payload を見たい場合

Kanon 側の送信組み立ては `Kanon/src/js/testize.js` の `synthesize()` にある。重要なフィールドは次。

- `method_calls`
- `receiverObject`
- `methodName`
- `arguments`
- `argumentTypes`
- `argumentNames`
- `methodParamNames`
- `operations`
- `precondGraph`
- `actualGraph`
- `idMapping`
- `vis_graph`

native server 経由なら `handle_synthesis` が raw request を terminal に出す。browser runtime 経由では terminal に出ないため、必要なら一時的に `testize.js` 側で payload を console に出す。native 実行では、RefSyn response の `list_environment_info` に含まれる task JSON path も確認材料になる。

## Codex の作業メモ

- UI 操作の成否を座標クリックだけで判断しない。
- `isLiteral` と label の確認を最優先する。
- 合成器の限界と UI 入力ミスを混同しない。
- 画面で成功に見えても、必要なら `target/escher/ts/*.json` と Rust 統合テストで再現性を確認する。
- 変更を加える場合は、Kanon サブモジュール本体ではなく、まず RefSyn 側の docs/test/backend postprocess のどこに責務があるかを切り分ける。

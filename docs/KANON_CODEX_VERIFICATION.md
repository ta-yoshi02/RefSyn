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
   - 直前の失敗表示（`Runtime Error: ...`）が残っていないこと。

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

合成に成功すると、エディタの対象メソッドが論文4.3節の文列に置き換わり、穴ごとの補助メソッドが `// refsyn: generated helper` 付きで直前に入る。論文の `insert` では次の形になる。

```js
insert(i, arg) {
    const created0 = new Node();
    const hole0 = this.insert_h0(i, arg, created0);
    const hole1 = this.insert_h1(i, arg, created0, hole0);
    const hole2 = this.insert_h2(i, arg, created0, hole0, hole1);
    created0.val = hole0;
    hole1.next = created0;
    created0.next = hole2;
}
```

成功と判断できる最低条件:

- RefSyn response の `validation.status` が `passed` で、`checked_demonstrations` が実演数と一致する。
- 置換後の再実行で、各呼出しのテストが通る（呼出しが緑になる）。
- 補助メソッド名は `<メソッド名>_h<穴の番号>` で、穴の番号は束縛順（第1実演の辺列での初出順）である。

置換されない場合は、反映を拒否した理由がコンソールに出る（`検証済みコードを反映できません` または `合成結果は未検証または実演に不一致のため反映しません`）。エディタには何も挿入されない。

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
- `list_environment_info` に `not isomorphic`
  - 実演どうしの操作グラフが同型でない。生成数、書込み数、書込み先のfield、nullと非nullの違いを見直す。
- `list_environment_info` に `typed pre-call state` や `instanceError`
  - 型付き状態を記録できていない。実演を記録し直すか、constructorが対応範囲（引数なし、プリミティブ・null・undefinedだけで初期化）かを確認する。
- `validation.status` が `failed`
  - 合成した式が実演を再現しない。`validation.error` に、どの値・参照・補助メソッドが食い違ったかが出る。実演を増やして穴の出力例を区別できるようにする。

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

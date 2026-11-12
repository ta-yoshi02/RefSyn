// 段階1: いちばん小さいメソッド setVal(arg) { this.val = arg; } を合成して、手書き版と比べる。
// 記録 (01-setval.json) は手で書いた 2 回の呼び出し。どちらも「受け手の val を arg に書き換える」。
// 00-insert.mjs と同じ形。違うのは記録ファイル、手書きの参照、試す入力だけ。
// 実行: node tests/steps/01-setval.mjs   (リポジトリのルートで)
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";

// 1. パイプラインを動かす。run_payload は ログ → "HTTP 200 OK" の行 → 応答JSON の順に出力する。
const output = execFileSync(
  "cargo",
  ["run", "-q", "--example", "run_payload", "--", "tests/steps/01-setval.json"],
  { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 },
);
const [, status, json] = output.match(/^HTTP (\d+).*\n([\s\S]*)$/m);
assert.equal(status, "200");
const response = JSON.parse(json);

// 2. 合成された補助メソッドと本体を見せる。
console.log(response.code.join("\n"));
console.log(response.composed_method_code);

// 3. 1つのクラスに、手書きの setValRef と、合成された setVal を両方入れる。
const Node = eval(`(class Node {
  setValRef(arg) {
    this.val = arg;
  }
  ${response.code.join("\n")}
  ${response.composed_method_code}
})`);

// 4. 値の並びからリストを作る。[10, 20] → 10 → 20 → null
function list(values) {
  let head = null;
  for (const val of [...values].reverse()) {
    const node = new Node();
    node.val = val;
    node.next = head;
    head = node;
  }
  return head;
}

// 5. 長さ 1〜3 のリストに、記録に無かった値 (0, 負の数, 元と同じ値) も渡してみる。
for (let length = 1; length <= 3; length += 1) {
  for (const arg of [0, -1, 10, 1000]) {
    const values = Array.from({ length }, (_, k) => (k + 1) * 10);
    const expected = list(values);
    const actual = list(values);
    expected.setValRef(arg);
    actual.setVal(arg);
    assert.deepStrictEqual(actual, expected, `length ${length}, arg ${arg}`);
  }
}
console.log("ok: 合成された setVal は手書きの setVal と同じ結果になった");

// 段階0: 実際のパイプラインが今出す insert を、手書きの insert と比べる。
// check.mjs との違いは、合成コードを貼り付けずに、その場でパイプラインから受け取ること。
// 実行: node tests/steps/00-insert.mjs   (リポジトリのルートで)
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";

// 1. パイプラインを動かす。run_payload は ログ → "HTTP 200 OK" の行 → 応答JSON の順に出力する。
const output = execFileSync(
  "cargo",
  ["run", "-q", "--example", "run_payload", "--", "tests/data/paper_insert.json"],
  { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 },
);
const [, status, json] = output.match(/^HTTP (\d+).*\n([\s\S]*)$/m);
assert.equal(status, "200");
const response = JSON.parse(json);

// 2. 合成された補助メソッド (code) と本体 (composed_method_code) を見せる。
console.log(response.code.join("\n"));
console.log(response.composed_method_code);

// 3. 1つのクラスに、手書きの insertRef と、合成された insert を両方入れる。
//    同じクラスなので、check.mjs と同じく deepStrictEqual でそのまま比べられる。
const Node = eval(`(class Node {
  insertRef(i, arg) {
    let node = this;
    for (let k = 0; k < i; k += 1) node = node.next;
    const added = new Node();
    added.val = arg;
    added.next = node.next;
    node.next = added;
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

// 5. 長さ 1〜4 のリストで、挿入位置 i を 0 から 長さ-1 まで全部試す。
for (let length = 1; length <= 4; length += 1) {
  for (let i = 0; i < length; i += 1) {
    const values = Array.from({ length }, (_, k) => (k + 1) * 10);
    const expected = list(values);
    const actual = list(values);
    expected.insertRef(i, 99);
    actual.insert(i, 99);
    assert.deepStrictEqual(actual, expected, `length ${length}, i ${i}`);
  }
}
console.log("ok: 合成された insert は手書きの insert と同じ結果になった");

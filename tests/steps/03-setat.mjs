// 段階3: setAt(i, arg) を合成して、手書き版と比べる。段階1から増えるのは「i 回たどって書き込み先を決める」。
// 記録 (03-setat.json) は手で書いた 4 回の呼び出し。i 番目のノード (受け手が 0 番目) の val を arg にする。
// 長さ 4 と 5 のリストを混ぜ、i = 0, 1, 2, 2 にしてある。3 回・同じ長さにすると
// 「i === 0 なら受け手、それ以外は最後から 2 番目」のような別の式でも記録を満たせてしまうため。
// 実行: node tests/steps/03-setat.mjs   (リポジトリのルートで)
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";

// 1. パイプラインを動かす。run_payload は ログ → "HTTP 200 OK" の行 → 応答JSON の順に出力する。
const output = execFileSync(
  "cargo",
  ["run", "-q", "--example", "run_payload", "--", "tests/steps/03-setat.json"],
  { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 },
);
const [, status, json] = output.match(/^HTTP (\d+).*\n([\s\S]*)$/m);
assert.equal(status, "200");
const response = JSON.parse(json);

// 2. 合成された補助メソッドと本体を見せる。
console.log(response.code.join("\n"));
console.log(response.composed_method_code);

// 3. 1つのクラスに、手書きの setAtRef と、合成された setAt を両方入れる。
const Node = eval(`(class Node {
  setAtRef(i, arg) {
    let node = this;
    for (let k = 0; k < i; k += 1) node = node.next;
    node.val = arg;
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

// 5. 長さ 1〜6 のリストで、i を 0 から 長さ-1 まで全部試す。記録に無かった長さ 1〜3 と 6、
//    記録に無かった i = 3〜5 も含む。
for (let length = 1; length <= 6; length += 1) {
  for (let i = 0; i < length; i += 1) {
    const values = Array.from({ length }, (_, k) => (k + 1) * 10);
    const expected = list(values);
    const actual = list(values);
    expected.setAtRef(i, 99);
    actual.setAt(i, 99);
    assert.deepStrictEqual(actual, expected, `length ${length}, i ${i}`);
  }
}
console.log("ok: 合成された setAt は手書きの setAt と同じ結果になった");

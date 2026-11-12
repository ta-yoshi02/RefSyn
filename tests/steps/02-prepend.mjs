// 段階2: prepend(arg) を合成して、手書き版と比べる。段階1から増えるのは「オブジェクトを作る」と「値を返す」。
// 記録 (02-prepend.json) は手で書いた 2 回の呼び出し。どちらも「新しい Node を作り、val に arg、
// next に受け手を入れて、それを返す」。
// 実行: node tests/steps/02-prepend.mjs   (リポジトリのルートで)
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";

// 1. パイプラインを動かす。run_payload は ログ → "HTTP 200 OK" の行 → 応答JSON の順に出力する。
const output = execFileSync(
  "cargo",
  ["run", "-q", "--example", "run_payload", "--", "tests/steps/02-prepend.json"],
  { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 },
);
const [, status, json] = output.match(/^HTTP (\d+).*\n([\s\S]*)$/m);
assert.equal(status, "200");
const response = JSON.parse(json);

// 2. 合成された補助メソッドと本体を見せる。
console.log(response.code.join("\n"));
console.log(response.composed_method_code);

// 3. 1つのクラスに、手書きの prependRef と、合成された prepend を両方入れる。
const Node = eval(`(class Node {
  prependRef(arg) {
    const added = new Node();
    added.val = arg;
    added.next = this;
    return added;
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

// 5. 長さ 1〜3 のリストで試す。戻り値は新しい先頭なので、戻り値どうしを比べれば
//    新しいノードと、その後ろに続く元のリストの両方を比べたことになる。
for (let length = 1; length <= 3; length += 1) {
  for (const arg of [0, -1, 10, 1000]) {
    const values = Array.from({ length }, (_, k) => (k + 1) * 10);
    const expected = list(values).prependRef(arg);
    const actual = list(values).prepend(arg);
    assert.deepStrictEqual(actual, expected, `length ${length}, arg ${arg}`);
  }
}
console.log("ok: 合成された prepend は手書きの prepend と同じ結果になった");

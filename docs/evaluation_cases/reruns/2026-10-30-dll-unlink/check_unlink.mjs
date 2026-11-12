// Runs the applied unlink() (class taken from unlink/source_after.js) on inputs that were not demonstrated
// and compares it with a reference unlink, as fixed in plan.md.
// Usage (from this directory): node check_unlink.mjs > unlink/off_demo_result.json
import { readFileSync } from "node:fs";

const src = readFileSync(new URL("./unlink/source_after.js", import.meta.url), "utf8");
const classText = src.slice(0, src.indexOf("\nlet head"));
const Node = new Function(`${classText}\nreturn Node;`)();

const build = (n) => {
  const nodes = Array.from({ length: n }, (_, i) => {
    const node = new Node();
    node.val = (i + 1) * 10;
    return node;
  });
  nodes.forEach((node, i) => {
    if (i > 0) node.prev = nodes[i - 1];
    if (i < n - 1) node.next = nodes[i + 1];
  });
  return nodes;
};

const reference = (self) => {
  self.prev.next = self.next;
  self.next.prev = self.prev;
};

// Fields of every node, by index, so two runs on separate copies can be compared.
const snapshot = (nodes) =>
  nodes.map((node) => ({
    val: node.val,
    prev: node.prev === null ? null : nodes.indexOf(node.prev),
    next: node.next === null ? null : nodes.indexOf(node.next),
  }));

const run = (n, pos, apply) => {
  const nodes = build(n);
  try {
    apply(nodes[pos]);
    return { state: snapshot(nodes) };
  } catch (error) {
    return { error: String(error) };
  }
};

const cases = [];
for (let n = 3; n <= 7; n += 1) {
  for (let pos = 0; pos < n; pos += 1) {
    const inCondition = pos > 0 && pos < n - 1;
    const generated = run(n, pos, (self) => self.unlink());
    const expected = inCondition ? run(n, pos, reference) : null;
    cases.push({
      length: n,
      position: pos,
      inDemonstratedCondition: inCondition,
      agrees: inCondition ? JSON.stringify(generated) === JSON.stringify(expected) : null,
      generated,
      ...(inCondition ? { expected } : {}),
    });
  }
}

const inside = cases.filter((c) => c.inDemonstratedCondition);
console.log(
  JSON.stringify(
    {
      inCondition: { total: inside.length, agree: inside.filter((c) => c.agrees).length },
      outsideCondition: cases
        .filter((c) => !c.inDemonstratedCondition)
        .map((c) => ({ length: c.length, position: c.position, result: c.generated.error ?? "returned" })),
      cases,
    },
    null,
    2,
  ),
);

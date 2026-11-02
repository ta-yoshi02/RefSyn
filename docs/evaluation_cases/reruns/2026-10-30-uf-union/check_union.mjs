// Runs the applied union() (class taken from union/source_after.js) on inputs that were not demonstrated
// and compares it with a reference union by rank, as fixed in plan.md: two separate roots without
// children, ranks r1 != r2 in 0..3, in both orders.
// Usage (from this directory): node check_union.mjs > union/off_demo_result.json
import { readFileSync } from "node:fs";

const src = readFileSync(new URL("./union/source_after.js", import.meta.url), "utf8");
const UF = new Function(`${src.slice(0, src.indexOf("\nlet a "))}\nreturn UF;`)();

const reference = (self, other) => {
  if (self.rank < other.rank) self.parent = other;
  else other.parent = self;
};

const run = (r1, r2, apply) => {
  const x = new UF();
  x.rank = r1;
  const y = new UF();
  y.rank = r2;
  try {
    apply(x, y);
  } catch (error) {
    return { error: String(error) };
  }
  const name = (n) => (n === null ? null : n === x ? "this" : n === y ? "other" : "?");
  return { this: { parent: name(x.parent), rank: x.rank }, other: { parent: name(y.parent), rank: y.rank } };
};

const cases = [];
for (let r1 = 0; r1 <= 3; r1 += 1) {
  for (let r2 = 0; r2 <= 3; r2 += 1) {
    if (r1 === r2) continue;
    const generated = run(r1, r2, (x, y) => x.union(y));
    const expected = run(r1, r2, reference);
    cases.push({ thisRank: r1, otherRank: r2, agrees: JSON.stringify(generated) === JSON.stringify(expected), generated, expected });
  }
}
console.log(JSON.stringify({ total: cases.length, agree: cases.filter((c) => c.agrees).length, cases }, null, 2));

// Re-run the captured editor source in a fresh VM context and record, per call,
// the helper results next to the hole output examples from the saved PBE tasks.
import fs from "node:fs";
import vm from "node:vm";
const [srcPath, tasksPath] = process.argv.slice(2);
const src = fs.readFileSync(srcPath, "utf8");
const tasks = JSON.parse(fs.readFileSync(tasksPath, "utf8"));
const ctx = vm.createContext({});
const prog = src + `
;(() => {
  const listOf = (n) => { const out = []; const seen = new Set(); while (n !== null) { if (seen.has(n)) throw new Error("cycle"); seen.add(n); out.push(n); n = n.next; } return out; };
  const calls = [];
  const proto = Node.prototype;
  for (const h of ["insert_h0", "insert_h1", "insert_h2"]) {
    const f = proto[h];
    proto[h] = function (...a) { const r = f.apply(this, a); calls.push({ helper: h, result: r }); return r; };
  }
  const fresh = new Node(39); fresh.next = new Node(27); fresh.next.next = new Node(53);
  const results = [];
  for (const [i, v] of [[0, 89], [2, 46], [1, 71]]) {
    const before = listOf(fresh);
    calls.length = 0;
    fresh.insert(i, v);
    results.push({ args: [i, v], helpers: calls.map(({ helper, result }) => ({ helper,
      value: typeof result === "number" ? result : result === null ? null : { refIndexInPreState: before.indexOf(result) } })) });
  }
  return { finalTopLevel: listOf(lst).map((n) => n.val), finalFresh: listOf(fresh).map((n) => n.val), results };
})()`;
const out = new vm.Script(prog).runInContext(ctx, { timeout: 1000 });
const expected = tasks.map((t) => t.examples.map((e) => e[e.length - 1]));
const checks = out.results.map((r, d) => r.helpers.map((h, k) => {
  const ex = expected[k][d];
  const ok = typeof ex === "number" ? h.value === ex : h.value?.refIndexInPreState === ex.ref;
  return { helper: h.helper, demo: d, actual: h.value, expected: ex, ok };
})).flat();
console.log(JSON.stringify({ runtime: process.version, ...out, holeExampleChecks: checks, allHoleChecksPass: checks.every((c) => c.ok) }, null, 1));

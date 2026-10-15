// Runs the generated methods of the five main cases (the applied sources of
// 2026-10-14-fresh-recording, equal to what 494d6d4 generates) on inputs that were not
// demonstrated and compares each run with a reference implementation.
// Per run it compares: the list reachable from the receiver (or from the returned node for
// prepend), as a sequence of object identities and values; every field of every object
// that existed before the call; the fields of a created object; and the return value.
// Inputs are split into those within the demonstrated condition and those outside it.
// Usage: node <this> <out.json> [<recordings-dir>]   (any working directory)
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const here = dirname(fileURLToPath(import.meta.url));
// A second argument replaces the recordings directory (used by negative_control.sh).
const recordings = process.argv[3] ?? join(here, "../2026-10-14-fresh-recording");
const outFile = process.argv[2];

const loadClass = (dir) => {
  const source = readFileSync(join(recordings, dir, "source_after.js"), "utf8");
  const ctx = {};
  vm.createContext(ctx);
  vm.runInContext(source.slice(0, source.indexOf("\nlet lst")) + "\n;globalThis.Node = Node;", ctx);
  return ctx.Node;
};

// Values are distinct, and differ from every argument, so a wrong node or a wrong value shows.
const valuesFor = (n) => Array.from({ length: n }, (_, k) => 100 + 7 * k);
const build = (Node, vals) => {
  const nodes = vals.map((v) => Object.assign(new Node(), { val: v }));
  nodes.forEach((node, k) => (node.next = nodes[k + 1] ?? null));
  return nodes;
};
// The list from `head` as [label, val] pairs; original objects are o0.., others "new".
const walk = (head, originals) => {
  const out = [];
  const seen = new Set();
  for (let n = head; n !== null && n !== undefined; n = n.next) {
    if (seen.has(n)) return [...out, ["cycle", null]];
    seen.add(n);
    const k = originals.indexOf(n);
    out.push([k >= 0 ? `o${k}` : "new", n.val]);
  }
  return out;
};
const fieldsOf = (o) => Object.fromEntries(Object.keys(o).sort().map((k) => [k, o[k]]));

// Reference implementations on a plain description of the list.
// Each returns { list: [[label, val]...], fields: per original index {val, next label},
//   created: {val, next label} | null, returns: "new" | "undefined" }.
const label = (k, n) => (k === null ? null : k < n ? `o${k}` : "new");
const reference = {
  setAt: (vals, [a, b]) => {
    const v = vals.slice();
    v[a] = b;
    return { list: v.map((x, k) => [`o${k}`, x]), fields: v.map((x, k) => ({ val: x, next: label(k + 1 < v.length ? k + 1 : null, v.length) })), created: null, returns: "undefined" };
  },
  append: (vals, [x]) => {
    const n = vals.length;
    return {
      list: [...vals.map((y, k) => [`o${k}`, y]), ["new", x]],
      fields: vals.map((y, k) => ({ val: y, next: k + 1 < n ? `o${k + 1}` : "new" })),
      created: { val: x, next: null },
      returns: "undefined",
    };
  },
  prepend: (vals, [x]) => {
    const n = vals.length;
    return {
      list: [["new", x], ...vals.map((y, k) => [`o${k}`, y])],
      fields: vals.map((y, k) => ({ val: y, next: k + 1 < n ? `o${k + 1}` : null })),
      created: { val: x, next: "o0" },
      returns: "new",
    };
  },
  insert: (vals, [i, x]) => {
    const n = vals.length;
    const list = vals.map((y, k) => [`o${k}`, y]);
    list.splice(i + 1, 0, ["new", x]);
    return {
      list,
      fields: vals.map((y, k) => ({ val: y, next: k === i ? "new" : k + 1 < n ? `o${k + 1}` : null })),
      created: { val: x, next: i + 1 < n ? `o${i + 1}` : null },
      returns: "undefined",
    };
  },
  popBack: (vals) => {
    const n = vals.length;
    return {
      list: vals.slice(0, n - 1).map((y, k) => [`o${k}`, y]),
      fields: vals.map((y, k) => ({ val: y, next: k + 1 < n - 1 ? `o${k + 1}` : null })),
      created: null,
      returns: "undefined",
    };
  },
};

const cases = [
  { method: "setAt", dir: "setAt", inputs: (n) => Array.from({ length: n + 1 }, (_, a) => [a, -5 + a]), within: (n, [a]) => a < n },
  { method: "append", dir: "append", inputs: () => [[0], [-3], [41]], within: () => true },
  { method: "prepend", dir: "prepend", inputs: () => [[0], [-3], [41]], within: () => true },
  { method: "insert", dir: "insert_general", inputs: (n) => Array.from({ length: n }, (_, i) => [i, 1000 + i]), within: (n, [i]) => i + 1 < n },
  { method: "popBack", dir: "popBack", inputs: () => [[]], within: (n) => n >= 2 },
];
const lengths = [1, 2, 3, 4, 5, 6, 7];

const runs = [];
for (const c of cases) {
  const Node = loadClass(c.dir);
  for (const n of lengths) {
    for (const args of c.inputs(n)) {
      const vals = valuesFor(n);
      const originals = build(Node, vals);
      let result;
      let error = null;
      try {
        result = originals[0][c.method](...args);
      } catch (e) {
        error = `${e.constructor.name}: ${e.message}`;
      }
      const expected = reference[c.method](vals, args);
      let actual = null;
      if (error === null) {
        const head = c.method === "prepend" ? result : originals[0];
        // The created object: the returned one for prepend, otherwise the first object on the
        // list that did not exist before (none for setAt and popBack).
        let created = null;
        if (c.method === "prepend") created = result;
        else if (c.method === "append" || c.method === "insert") {
          const seen = new Set();
          for (let p = originals[0]; p && !seen.has(p); p = p.next) {
            seen.add(p);
            if (!originals.includes(p)) { created = p; break; }
          }
        }
        const nextLabel = (o) => (o === null ? null : originals.includes(o) ? `o${originals.indexOf(o)}` : "new");
        actual = {
          list: walk(head, originals),
          fields: originals.map((o) => ({ val: o.val, next: nextLabel(o.next) })),
          created: created ? { val: created.val, next: nextLabel(created.next) } : null,
          created_own_fields: created ? Object.keys(fieldsOf(created)) : null,
          returns: result === undefined ? "undefined" : originals.includes(result) ? "original" : "new",
        };
      }
      const same =
        actual !== null &&
        JSON.stringify(actual.list) === JSON.stringify(expected.list) &&
        JSON.stringify(actual.fields) === JSON.stringify(expected.fields) &&
        JSON.stringify(actual.created) === JSON.stringify(expected.created) &&
        actual.returns === expected.returns &&
        (actual.created_own_fields === null || JSON.stringify(actual.created_own_fields) === JSON.stringify(["next", "val"]));
      runs.push({ method: c.method, length: n, args, within_demonstrated_condition: c.within(n, args), agrees: same, error, expected, actual });
    }
  }
}

const summary = {};
for (const r of runs) {
  const key = `${r.method} ${r.within_demonstrated_condition ? "within" : "outside"}`;
  summary[key] ??= { runs: 0, agree: 0, differ: [] };
  summary[key].runs += 1;
  if (r.agrees) summary[key].agree += 1;
  else summary[key].differ.push({ length: r.length, args: r.args, error: r.error, list: r.actual && r.actual.list });
}
writeFileSync(outFile, JSON.stringify({ lengths, summary, runs }, null, 2) + "\n");
for (const [k, s] of Object.entries(summary)) {
  console.log(`${k.padEnd(18)} ${s.agree}/${s.runs} agree`);
  for (const d of s.differ) console.log("    differs:", JSON.stringify(d));
}

// Compares, per call, the list reachable from the receiver before the call and the
// arguments, between an old evaluation payload (fields f/g) and a new recording (val/next).
import { readFileSync, writeFileSync } from "node:fs";
const [oldPath, newPath, outPath] = process.argv.slice(2);
const listFromGraph = (call, valueField, nextField) => {
  const g = call.precondGraph;
  const label = Object.fromEntries(g.nodes.map((n) => [n.id, n.label]));
  const out = {};
  for (const e of g.edges) (out[e.from] ??= {})[e.label] = e.to;
  const vals = [];
  for (let id = call.receiverObject, k = 0; id && k < 50; id = out[id]?.[nextField], k++) vals.push(Number(label[out[id]?.[valueField]]));
  return vals;
};
const oldP = JSON.parse(readFileSync(oldPath, "utf8"));
const newP = JSON.parse(readFileSync(newPath, "utf8"));
const rows = oldP.method_calls.map((oc, i) => {
  const nc = newP.method_calls[i];
  const o = { args: oc.arguments, list: listFromGraph(oc, "f", "g") };
  const n = nc ? { args: nc.arguments, list: listFromGraph(nc, "val", "next") } : null;
  return { call: i + 1, old: o, new: n, equal: !!n && JSON.stringify(o) === JSON.stringify(n) };
});
const result = { old: oldPath, new: newPath, calls_old: oldP.method_calls.length, calls_new: newP.method_calls.length, rows, all_equal: rows.every((r) => r.equal) && oldP.method_calls.length === newP.method_calls.length };
writeFileSync(outPath, JSON.stringify(result, null, 2) + "\n");
console.log(JSON.stringify({ case: newPath.split("/").at(-2), all_equal: result.all_equal, rows: rows.map((r) => [r.old.args, r.old.list, r.new?.list, r.equal]) }));

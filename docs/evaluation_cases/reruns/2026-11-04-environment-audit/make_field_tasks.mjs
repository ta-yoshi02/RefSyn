// The paper's list baselines with the retyped list components (../2026-10-30-baseline-recheck/
// tasks_list.json) plus what MOLD's list tasks have and they lack: the field accessors escher-ts
// generates (autoClassFieldComponents, strictFieldAccess, as in MOLD's tasks) and findByValueRef (the
// port of ../2026-10-30-baseline/make_tasks.mjs, typed for heaps of references). The generic list
// components stay, since the witnesses need insert and length to append the created object.
// The state encoding is aligned with MOLD's so that the accessors read the state: the Int field is
// declared Ref[Int] (as MOLD's val), so fOf returns the object's slot in fHeap and loadInt reads it, and
// every nodeHeap object holds { f: its slot, g: its gHeap entry } (the published insert tasks leave the
// objects empty, the others declared f: Int while storing a slot, so fOf returned a reference as an Int).
// Usage (from the repository root): node <this> > tasks_fields.json
import { readFileSync } from "node:fs";

const step = "const nx = (h, i) => (i >= 0 && i < h.length && h[i] && typeof h[i].ref === 'number' ? h[i].ref : null);";
const R = "Ref[Object[Obj]]", LR = "List[Ref[Object[Obj]]]", LO = "List[Object[Obj]]";
const findByValueRef = { name: "findByValueRef", kind: "js", inputTypes: [R, LO, LR, "List[Int]", "Int"], returnType: R, args: ["s", "nodes", "h", "vals", "t"], bodyJs: `${step} let c = s.ref; const seen = new Set(); while (c !== -1) { if (seen.has(c) || c >= nodes.length || c >= h.length || c >= vals.length) return 'error'; seen.add(c); if (typeof vals[c] === 'number' && vals[c] === t) return { ref: c }; const n = nx(h, c); if (n === null) return 'error'; c = n; } return { ref: -1 };` };
const tasks = JSON.parse(readFileSync("docs/evaluation_cases/reruns/2026-10-30-baseline-recheck/tasks_list.json", "utf8"));
const align = (t) => {
  const iNode = t.signature.inputNames.indexOf("nodeHeap");
  const iG = t.signature.inputNames.indexOf("gHeap");
  const examples = t.examples.map(([args, out]) => {
    const a = [...args];
    a[iNode] = a[iNode].map((_o, i) => ({ object: { className: "Obj", fields: { f: { ref: i }, g: a[iG][i] } } }));
    return [a, out];
  });
  return { ...t, classes: [{ name: "Obj", fields: { f: "Ref[Int]", g: "Ref[Object[Obj]]" } }], examples };
};
console.log(JSON.stringify(tasks.map((t) => ({ ...align(t), autoClassFieldComponents: true, strictFieldAccess: true, components: [...t.components, findByValueRef] })), null, 2));

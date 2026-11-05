// The paper's list baselines with the retyped list components (../2026-10-30-baseline-recheck/
// tasks_list.json) plus what MOLD's list tasks have and they lack: the field accessors escher-ts
// generates (autoClassFieldComponents, strictFieldAccess, as in MOLD's tasks) and findByValueRef (the
// port of ../2026-10-30-baseline/make_tasks.mjs, typed for heaps of references). The generic list
// components stay, since the witnesses need insert and length to append the created object.
// Usage (from the repository root): node <this> > tasks_fields.json
import { readFileSync } from "node:fs";

const step = "const nx = (h, i) => (i >= 0 && i < h.length && h[i] && typeof h[i].ref === 'number' ? h[i].ref : null);";
const R = "Ref[Object[Obj]]", LR = "List[Ref[Object[Obj]]]", LO = "List[Object[Obj]]";
const findByValueRef = { name: "findByValueRef", kind: "js", inputTypes: [R, LO, LR, "List[Int]", "Int"], returnType: R, args: ["s", "nodes", "h", "vals", "t"], bodyJs: `${step} let c = s.ref; const seen = new Set(); while (c !== -1) { if (seen.has(c) || c >= nodes.length || c >= h.length || c >= vals.length) return 'error'; seen.add(c); if (typeof vals[c] === 'number' && vals[c] === t) return { ref: c }; const n = nx(h, c); if (n === null) return 'error'; c = n; } return { ref: -1 };` };
const tasks = JSON.parse(readFileSync("docs/evaluation_cases/reruns/2026-10-30-baseline-recheck/tasks_list.json", "utf8"));
console.log(JSON.stringify(tasks.map((t) => ({ ...t, autoClassFieldComponents: true, strictFieldAccess: true, components: [...t.components, findByValueRef] })), null, 2));

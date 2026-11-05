// tasks_fields.json without the generic list components that only the baseline has (head, tail, cons,
// nil, concat) and without the object-value components (new_Obj, Obj_f, Obj_g; exposeClassComponents
// off, as in MOLD's tasks). insert and length stay: the witnesses need them to append the created
// object to each heap. The result has MOLD's list components plus the construction components
// (storeAtRef, freshRefFromHeap, insert, length, pairHeap).
// Usage (from the repository root): node <this> > tasks_minimal.json
import { readFileSync } from "node:fs";

const drop = new Set(["head", "tail", "cons", "nil", "concat"]);
const tasks = JSON.parse(readFileSync("docs/evaluation_cases/reruns/2026-11-04-environment-audit/tasks_fields.json", "utf8"));
console.log(JSON.stringify(tasks.map((t) => ({ ...t, exposeClassComponents: false, components: t.components.filter((c) => !drop.has(c.name)) })), null, 2));

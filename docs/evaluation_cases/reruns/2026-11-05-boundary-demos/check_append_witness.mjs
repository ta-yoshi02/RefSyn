// Is the baseline's append next-heap task on append_older_tail expressible? Evaluates the hand-written
// witness of ../2026-11-04-environment-audit/typecheck_witnesses_minimal.json on that recording's
// examples (built as in run_list_baseline.mjs).
// Usage (from the repository root): node <this>
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import path from "node:path";

const D = "docs/evaluation_cases/reruns/2026-11-05-boundary-demos";
const imp = (p) => import(pathToFileURL(path.join(process.cwd(), "external/escher-ts/dist", p)).href);
const L = { ...(await imp("components/component.js")), ...(await imp("components/user-friendly-json.js")), ...(await imp("types/term.js")) };
const task = JSON.parse(readFileSync("docs/evaluation_cases/reruns/2026-11-04-environment-audit/tasks_minimal.json", "utf8")).find((t) => t.name === "append-fixed-gHeap");
const calls = JSON.parse(readFileSync(`${D}/append_older_tail/operations.json`, "utf8"));
const idx = new Map([0, 1, 2, 3].map((i) => [`main-new${i + 1}`, i]));
const st = { f: [39, 27, 6, 7], g: [1, -1, 3, -1] };
const lit = new Map();
const examples = [];
for (const call of calls) {
  const pre = { f: [...st.f], g: [...st.g], head: idx.get(call.receiverObject) };
  for (const op of call.operations) {
    if (op.editType === "addNode") { if (op.isLiteral) lit.set(op.id, Number(op.label)); else { idx.set(op.id, st.f.length); st.f.push(undefined); st.g.push(-1); } }
    else if (op.label === "val") st.f[idx.get(op.from)] = lit.get(op.to);
    else if (op.label === "next") st.g[idx.get(op.from)] = idx.get(op.to);
  }
  const nodeHeap = pre.f.map((_, i) => ({ object: { className: "Obj", fields: { f: { ref: i }, g: { ref: pre.g[i] } } } }));
  examples.push([[{ ref: pre.head }, nodeHeap, [...pre.f], pre.g.map((r) => ({ ref: r })), call.arguments[0]], st.g.map((r) => ({ ref: r }))]);
}
const { evaluationMethod: _m, ...spec } = { ...task, examples };
const job = L.prepareJsonSynthesisJob(L.parseJsonSynthesisSpec(JSON.stringify(spec)));
const c = L.componentTerm, v = L.varTerm;
const stored = c("storeAtRef", [v("gHeap"), c("last_ptr", [v("thisRef"), v("gHeap")]), c("freshRefFromHeap", [v("nodeHeap")])]);
const witness = c("insert", [stored, c("length", [stored]), c("nthNextRef", [v("thisRef"), v("nodeHeap"), v("gHeap"), c("length", [v("nodeHeap")])])]);
const ok = job.examples.map(([args, want]) => { const vars = new Map(job.inputNames.map((n, k) => [n, args[k]])); return JSON.stringify(L.executeTerm((n) => vars.get(n), job.env, witness)) === JSON.stringify(want); });
console.log(JSON.stringify({ witness: L.showTerm(witness), examples: ok.length, satisfied: ok.filter(Boolean).length }));

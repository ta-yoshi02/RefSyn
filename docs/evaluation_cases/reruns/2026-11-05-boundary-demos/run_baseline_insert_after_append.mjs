// Copy of ../2026-10-30-baseline/run_insert_after_append.mjs with only the recordings and file paths changed to this
// directory's recordings (2026-11-05-boundary-demos).
// insertAfter's non-separated tasks need to append the created object to every heap, which the
// components of tasks.json cannot express. This adds the list components insert and length (as in
// ../../nonseparated_general_components/) to the insertAfter tasks (both variants), checks hand-written
// witnesses on the examples, and runs the search under both settings and both limits.
// Usage (from the repository root): WEIGHTED_DIST=<patched dist> node <this> > <out.json>
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import path from "node:path";

const root = process.cwd();
const load = async (dist) => {
  const imp = (p) => import(pathToFileURL(path.join(dist, p)).href);
  return { ...(await imp("components/component.js")), ...(await imp("components/user-friendly-json.js")), ...(await imp("synthesis/ascendrec/synthesizer.js")), ...(await imp("types/term.js")) };
};
const libs = { current: await load(path.join(root, "external/escher-ts/dist")), candidate: await load(process.env.WEIGHTED_DIST) };
const costs = { current: undefined, candidate: { last_ptr: 3, penultimateRef: 3, nthNextRef: 3, findByValueRef: 3 } };
const factor = { current: 3, candidate: 1 };
const limits = { adapter: { maxCost: 20, timeoutMs: 2000 }, paper: { maxCost: 32, timeoutMs: 15000 } };
const added = [{ name: "insert", kind: "libraryRef", ref: "insert" }, { name: "length", kind: "libraryRef", ref: "length" }];
const tasks = JSON.parse(readFileSync("docs/evaluation_cases/reruns/2026-11-05-boundary-demos/baseline_tasks.json", "utf8"))
  .filter((t) => t.case === "insertAfter" && !t.name.includes("pair"))
  .map((t) => ({ ...t, name: `${t.name}+append`, components: [...t.components, ...added] }));

const { componentTerm: c, varTerm: v } = libs.current;
const append = (h, x) => c("insert", [v(h), c("length", [v(h)]), x]);
const nextOfThis = c("nextOf", [v("thisRef"), v("nodeHeap"), v("nextHeap")]);
const witness = {
  nextHeap: c("storeAtRef", [append("nextHeap", nextOfThis), v("thisRef"), c("freshRefFromHeap", [v("nextHeap")])]),
  prevHeap: c("storeAtRef", [append("prevHeap", v("thisRef")), nextOfThis, c("freshRefFromHeap", [v("prevHeap")])]),
  valHeap: append("valHeap", v("x")),
};
const size = (t) => (t.kind === "var" ? 1 : 1 + t.args.reduce((s, a) => s + size(a), 0));

const rows = [];
for (const task of tasks) {
  const { case: _c, output, ...spec } = task;
  const L0 = libs.current;
  const job0 = L0.prepareJsonSynthesisJob(L0.parseJsonSynthesisSpec(JSON.stringify(spec)));
  const w = witness[output];
  const witnessOk = job0.examples.every(([args, out]) => {
    const vars = new Map(job0.inputNames.map((n, k) => [n, args[k]]));
    return JSON.stringify(L0.executeTerm((n) => vars.get(n), job0.env, w)) === JSON.stringify(out);
  });
  for (const setting of ["current", "candidate"]) {
    for (const [limitName, limit] of Object.entries(limits)) {
      const L = libs[setting];
      globalThis.__refsynCompCost = costs[setting];
      const job = L.prepareJsonSynthesisJob(L.parseJsonSynthesisSpec(JSON.stringify(spec)));
      let status = null;
      const t0 = Date.now();
      const r = new L.AscendRecSynthesizer({ ...limit, searchSizeFactor: factor[setting], useReductionRules: true, onlyForwardSearch: false, argListCompare: L.anyArgSmaller, onDiagnostics: (d) => { status = d.status; } })
        .synthesize(job.functionName, job.inputTypes, job.inputNames, job.returnType, job.env, job.examples);
      rows.push({ task: task.name, output, witness: L0.showTerm(w), witnessCost: size(w), witnessOk, setting, limits: limitName, ms: Date.now() - t0, status, cost: r?.program.cost ?? null, term: r ? L.showTerm(r.program.body) : null });
    }
  }
}
console.log(JSON.stringify(rows, null, 2));

// Copy of ../2026-10-30-baseline/check_insert_after_append.mjs with only the file paths changed to this
// directory (2026-11-05-boundary-demos), and rows without a term reported instead of re-searched.
// Evaluates the terms of results_insert_after_append.json against the hand-written witnesses on lists of
// length 2-7, every non-tail position, x in {0, 1, 2, 3, 99} (x < length makes `insert(heap, x, ...)`
// insert in the middle instead of appending).
// A search that does not repeat the saved term is reported on stderr and makes the script exit with 1.
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
const results = JSON.parse(readFileSync("docs/evaluation_cases/reruns/2026-11-05-boundary-demos/baseline_results_insert_after_append.json", "utf8"));
const tasks = JSON.parse(readFileSync("docs/evaluation_cases/reruns/2026-11-05-boundary-demos/baseline_tasks.json", "utf8"));

const list = (n) => Array.from({ length: n }, (_, i) => ({ val: (i + 1) * 10, prev: i > 0 ? i - 1 : null, next: i < n - 1 ? i + 1 : null }));
const encode = (objs, recv, x) => {
  const nodeHeap = objs.map((o, i) => ({ object: { className: "Node", fields: { next: { ref: o.next ?? -1 }, prev: { ref: o.prev ?? -1 }, val: { ref: i } } } }));
  return [{ ref: recv }, nodeHeap, objs.map((o) => ({ ref: o.next ?? -1 })), objs.map((o) => ({ ref: o.prev ?? -1 })), objs.map((o) => o.val), x];
};
const out = [];
for (const row of results) {
  if (row.term === null) { out.push({ task: row.task, setting: row.setting, limits: row.limits, term: null, status: row.status }); continue; }
  const base = tasks.find((t) => t.name === row.task.replace(/\+append$/, ""));
  const { case: _c, output: _o, ...spec } = { ...base, components: [...base.components, ...added] };
  const L = libs[row.setting];
  globalThis.__refsynCompCost = costs[row.setting];
  const job = L.prepareJsonSynthesisJob(L.parseJsonSynthesisSpec(JSON.stringify(spec)));
  const r = new L.AscendRecSynthesizer({ ...limits[row.limits], searchSizeFactor: factor[row.setting], useReductionRules: true, onlyForwardSearch: false, argListCompare: L.anyArgSmaller })
    .synthesize(job.functionName, job.inputTypes, job.inputNames, job.returnType, job.env, job.examples);
  if (!r || L.showTerm(r.program.body) !== row.term) {
    out.push({ task: row.task, setting: row.setting, limits: row.limits, error: "search did not repeat" });
    process.exitCode = 1;
    console.error(`${row.task} ${row.setting} ${row.limits}: search did not repeat`);
    continue;
  }
  const { componentTerm: c, varTerm: v } = L;
  const app = (h, x) => c("insert", [v(h), c("length", [v(h)]), x]);
  const nt = c("nextOf", [v("thisRef"), v("nodeHeap"), v("nextHeap")]);
  const w = { nextHeap: c("storeAtRef", [app("nextHeap", nt), v("thisRef"), c("freshRefFromHeap", [v("nextHeap")])]), prevHeap: c("storeAtRef", [app("prevHeap", v("thisRef")), nt, c("freshRefFromHeap", [v("prevHeap")])]), valHeap: app("valHeap", v("x")) }[row.output];
  let agree = 0, total = 0;
  for (let n = 2; n <= 7; n += 1) for (let p = 0; p < n - 1; p += 1) for (const x of [0, 1, 2, 3, 99]) {
    const [[args]] = L.prepareJsonSynthesisJob(L.parseJsonSynthesisSpec(JSON.stringify({ ...spec, examples: [[encode(list(n), p, x), []]] }))).examples;
    const vars = new Map(job.inputNames.map((nm, k) => [nm, args[k]]));
    const ev = (t) => JSON.stringify(L.executeTerm((nm) => vars.get(nm), job.env, t));
    total += 1;
    if (ev(r.program.body) === ev(w)) agree += 1;
  }
  out.push({ task: row.task, setting: row.setting, limits: row.limits, term: L.showTerm(r.program.body), agree, total });
}
console.log(JSON.stringify(out, null, 2));

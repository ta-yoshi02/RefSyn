// Copy of ../2026-10-30-baseline/run_tasks.mjs with only the recordings and file paths changed to this
// directory's recordings (2026-11-05-boundary-demos).
// Runs the non-separated tasks (tasks.json) under the current and candidate settings of
// ../2026-10-30-new-cases/plan.md, with the adapter limits (20, 2 s) and the paper's (32, 15 s).
// With SUFFIX=+list only the tasks whose name ends with it are run (otherwise only the others).
// Usage (from the repository root): WEIGHTED_DIST=<patched dist> [SUFFIX=+list] node <this> > <out.json>
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import path from "node:path";

const root = process.cwd();
const load = async (dist) => {
  const imp = (p) => import(pathToFileURL(path.join(dist, p)).href);
  return {
    ...(await imp("components/component.js")),
    ...(await imp("components/user-friendly-json.js")),
    ...(await imp("synthesis/ascendrec/synthesizer.js")),
    ...(await imp("types/term.js")),
  };
};
const libs = { current: await load(path.join(root, "external/escher-ts/dist")), candidate: await load(process.env.WEIGHTED_DIST) };
const costs = { current: undefined, candidate: { last_ptr: 3, penultimateRef: 3, nthNextRef: 3, findByValueRef: 3 } };
const factor = { current: 3, candidate: 1 };
const limits = { adapter: { maxCost: 20, timeoutMs: 2000 }, paper: { maxCost: 32, timeoutMs: 15000 } };

const suffix = process.env.SUFFIX;
const tasks = JSON.parse(readFileSync("docs/evaluation_cases/reruns/2026-11-05-boundary-demos/baseline_tasks.json", "utf8"))
  .filter((t) => (suffix ? t.name.endsWith(suffix) : !t.name.endsWith("+list")));
const rows = [];
for (const task of tasks) {
  const { case: caseName, output, ...spec } = task;
  for (const setting of ["current", "candidate"]) {
    for (const [limitName, limit] of Object.entries(limits)) {
      const L = libs[setting];
      globalThis.__refsynCompCost = costs[setting];
      const job = L.prepareJsonSynthesisJob(L.parseJsonSynthesisSpec(JSON.stringify(spec)));
      let status = null;
      const t0 = Date.now();
      const r = new L.AscendRecSynthesizer({
        ...limit, searchSizeFactor: factor[setting], useReductionRules: true, onlyForwardSearch: false,
        argListCompare: L.anyArgSmaller, onDiagnostics: (d) => { status = d.status; },
      }).synthesize(job.functionName, job.inputTypes, job.inputNames, job.returnType, job.env, job.examples);
      const row = { case: caseName, task: task.name, output, setting, limits: limitName, ms: Date.now() - t0, status, cost: r?.program.cost ?? null, term: r ? L.showTerm(r.program.body) : null };
      rows.push(row);
      console.error(`${task.name} ${setting} ${limitName}: ${row.term ?? status} (${row.ms}ms)`);
    }
  }
}
console.log(JSON.stringify(rows, null, 2));

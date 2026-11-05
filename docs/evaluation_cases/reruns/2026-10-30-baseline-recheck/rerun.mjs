// Runs the 16 tasks of the paper's non-separated baseline, as published (tasks.json) and with the list
// components usable (tasks_list.json), under the current setting (escher-ts 4e7c4bc, searchSizeFactor 3)
// and the candidate one (patched dist, factor 1, list components cost 3), at the paper's limits
// (maxCost 32, 15 s), once each. AscendRec options as in ../2026-10-30-baseline/run_tasks.mjs.
// Usage (from the repository root): WEIGHTED_DIST=<patched dist> node <this> > rerun_results.json
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import path from "node:path";

const root = process.cwd();
const D = "docs/evaluation_cases/reruns/2026-10-30-baseline-recheck";
const load = async (dist) => {
  const imp = (p) => import(pathToFileURL(path.join(dist, p)).href);
  return { ...(await imp("components/component.js")), ...(await imp("components/user-friendly-json.js")), ...(await imp("synthesis/ascendrec/synthesizer.js")), ...(await imp("types/term.js")) };
};
const libs = { current: await load(path.join(root, "external/escher-ts/dist")), candidate: await load(process.env.WEIGHTED_DIST) };
const costs = { current: undefined, candidate: { last_ptr: 3, penultimateRef: 3, nthNextRef: 3, findByValueRef: 3 } };
const factor = { current: 3, candidate: 1 };
const variants = {
  published: JSON.parse(readFileSync("docs/evaluation_cases/nonseparated_general_components/tasks.json", "utf8")),
  list: JSON.parse(readFileSync(`${D}/tasks_list.json`, "utf8")),
};
const rows = [];
for (const [variant, tasks] of Object.entries(variants)) {
  for (const task of tasks) {
    const { evaluationMethod: _m, ...spec } = task;
    for (const setting of ["current", "candidate"]) {
      const L = libs[setting];
      globalThis.__refsynCompCost = costs[setting];
      const job = L.prepareJsonSynthesisJob(L.parseJsonSynthesisSpec(JSON.stringify(spec)));
      let status = null;
      const t0 = Date.now();
      const r = new L.AscendRecSynthesizer({
        maxCost: 32, timeoutMs: 15000, searchSizeFactor: factor[setting], useReductionRules: true, onlyForwardSearch: false,
        argListCompare: L.anyArgSmaller, onDiagnostics: (d) => { status = d.status; },
      }).synthesize(job.functionName, job.inputTypes, job.inputNames, job.returnType, job.env, job.examples);
      const row = { variant, task: task.name, setting, ms: Date.now() - t0, status, cost: r?.program.cost ?? null, term: r ? L.showTerm(r.program.body) : null };
      rows.push(row);
      console.error(`${variant} ${task.name} ${setting}: ${row.term ?? status} (${row.ms}ms)`);
    }
  }
}
console.log(JSON.stringify(rows, null, 2));

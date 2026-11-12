// rerun.mjs of ../2026-10-30-baseline-recheck/ on tasks_minimal.json (the paper's list baselines with MOLD's
// list components plus the construction components only), current and candidate settings,
// maxCost 32, 15 s, once each.
// Usage (from the repository root): WEIGHTED_DIST=<patched dist> node <this> > rerun_minimal.json
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import path from "node:path";

const root = process.cwd();
const D = "docs/evaluation_cases/reruns/2026-11-04-environment-audit";
const load = async (dist) => {
  const imp = (p) => import(pathToFileURL(path.join(dist, p)).href);
  return { ...(await imp("components/component.js")), ...(await imp("components/user-friendly-json.js")), ...(await imp("synthesis/ascendrec/synthesizer.js")), ...(await imp("types/term.js")) };
};
const libs = { current: await load(path.join(root, "external/escher-ts/dist")), candidate: await load(process.env.WEIGHTED_DIST) };
const costs = { current: undefined, candidate: { last_ptr: 3, penultimateRef: 3, nthNextRef: 3, findByValueRef: 3 } };
const factor = { current: 3, candidate: 1 };
const variants = {
  minimal: JSON.parse(readFileSync(`${D}/tasks_minimal.json`, "utf8")),
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

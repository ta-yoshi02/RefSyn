// Searches each helper task of the three new recordings under the two settings fixed in plan.md:
// current (unpatched escher-ts dist, searchSizeFactor 3, all costs 1) and candidate (patched dist,
// searchSizeFactor 1, list components cost 3), under the adapter limits and the paper's, on all
// demonstrations; and under the adapter limits on every non-empty subset of the demonstrations.
// Usage (from the repository root): WEIGHTED_DIST=<patched dist> node <this> > <out.json>
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
const libs = {
  current: await load(path.join(root, "external/escher-ts/dist")),
  candidate: await load(process.env.WEIGHTED_DIST),
};
const costs = { current: undefined, candidate: { last_ptr: 3, penultimateRef: 3, nthNextRef: 3, findByValueRef: 3 } };
const factor = { current: 3, candidate: 1 };
const limits = { adapter: { maxCost: 20, timeoutMs: 2000 }, paper: { maxCost: 32, timeoutMs: 15000 } };

const run = (setting, task, examples, limit) => {
  const L = libs[setting];
  globalThis.__refsynCompCost = costs[setting];
  const job = L.prepareJsonSynthesisJob(L.parseJsonSynthesisSpec(JSON.stringify({ ...task, examples })));
  let status = null;
  const t0 = Date.now();
  const r = new L.AscendRecSynthesizer({
    ...limit, searchSizeFactor: factor[setting], useReductionRules: true, onlyForwardSearch: false,
    argListCompare: L.anyArgSmaller, onDiagnostics: (d) => { status = d.status; },
  }).synthesize(job.functionName, job.inputTypes, job.inputNames, job.returnType, job.env, job.examples);
  return { ms: Date.now() - t0, status, cost: r?.program.cost ?? null, term: r ? L.showTerm(r.program.body) : null };
};

const rows = [];
for (const name of ["swapChildren", "insertAfter", "rotateLeft"]) {
  const file = `docs/evaluation_cases/reruns/2026-10-30-new-cases/${name}/mold_escher_tasks.json`;
  for (const task of JSON.parse(readFileSync(file, "utf8"))) {
    const n = task.examples.length;
    for (const setting of ["current", "candidate"]) {
      for (const [limitName, limit] of Object.entries(limits)) {
        rows.push({ case: name, helper: task.name, setting, limits: limitName, demos: "all", ...run(setting, task, task.examples, limit) });
      }
      for (let mask = 1; mask < (1 << n) - 1; mask += 1) {
        const ex = task.examples.filter((_, i) => mask & (1 << i));
        const demos = [...Array(n).keys()].filter((i) => mask & (1 << i)).map((i) => i + 1).join(",");
        rows.push({ case: name, helper: task.name, setting, limits: "adapter", demos, ...run(setting, task, ex, limits.adapter) });
      }
    }
  }
}
console.log(JSON.stringify(rows, null, 2));

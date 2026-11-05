// Re-runs the unlink and union helper tasks on every non-empty subset of their demonstrations, with
// searchSizeFactor 1 and the list components costing 3, under the adapter limits.
// Usage (from the repository root): ESCHER_DIST=<weighted dist> node <this>
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import path from "node:path";
const dist = process.env.ESCHER_DIST;
const imp = (p) => import(pathToFileURL(path.join(dist, p)).href);
const { anyArgSmaller } = await imp("components/component.js");
const { parseJsonSynthesisSpec, prepareJsonSynthesisJob } = await imp("components/user-friendly-json.js");
const { AscendRecSynthesizer } = await imp("synthesis/ascendrec/synthesizer.js");
const { showTerm } = await imp("types/term.js");
globalThis.__refsynCompCost = { last_ptr: 3, penultimateRef: 3, nthNextRef: 3, findByValueRef: 3 };
const R = "docs/evaluation_cases/reruns";
for (const file of [`${R}/2026-10-30-dll-unlink/unlink/mold_escher_tasks.json`, `${R}/2026-10-30-uf-union/union/mold_escher_tasks.json`]) {
  for (const task of JSON.parse(readFileSync(file, "utf8"))) {
    const n = task.examples.length;
    for (let mask = 1; mask < 1 << n; mask += 1) {
      const ex = task.examples.filter((_, i) => mask & (1 << i));
      const job = prepareJsonSynthesisJob(parseJsonSynthesisSpec(JSON.stringify({ ...task, examples: ex })));
      let status = null;
      const r = new AscendRecSynthesizer({ maxCost: 20, timeoutMs: 2000, searchSizeFactor: 1, useReductionRules: true, onlyForwardSearch: false, argListCompare: anyArgSmaller, onDiagnostics: (d) => { status = d.status; } })
        .synthesize(job.functionName, job.inputTypes, job.inputNames, job.returnType, job.env, job.examples);
      const calls = [...Array(n).keys()].filter((i) => mask & (1 << i)).map((i) => i + 1).join(",");
      console.log(`${task.name} calls {${calls}}: ${r ? `c${r.program.cost} ${showTerm(r.program.body)}` : status}`);
    }
  }
}

// Re-runs the saved union helper tasks (A) unchanged and (B) with one added component
// lt: (Int, Int) -> Bool, under the adapter settings and under the paper's measurement limits,
// with searchSizeFactor 3 (the adapter default) and 1.
// Usage (from the repository root): node <this> . <task-file>
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import path from "node:path";

const [root, file] = process.argv.slice(2);
const imp = (p) => import(pathToFileURL(path.join(root, p)).href);
const { anyArgSmaller } = await imp("external/escher-ts/dist/components/component.js");
const { parseJsonSynthesisSpec, prepareJsonSynthesisJob } = await imp("external/escher-ts/dist/components/user-friendly-json.js");
const { AscendRecSynthesizer } = await imp("external/escher-ts/dist/synthesis/ascendrec/synthesizer.js");
const { showTerm } = await imp("external/escher-ts/dist/types/term.js");

const lt = { name: "lt", kind: "js", inputTypes: ["Int", "Int"], returnType: "Bool", args: ["x", "y"], bodyJs: "return x < y;" };
const tasks = JSON.parse(readFileSync(file, "utf8"));
const variants = { A: (t) => t, B: (t) => ({ ...t, components: [...t.components, lt] }) };
const limits = { adapter: { maxCost: 20, timeoutMs: 2000 }, paper: { maxCost: 32, timeoutMs: 15000 } };

for (const factor of [3, 1]) {
for (const [vName, variant] of Object.entries(variants)) {
  for (const [lName, limit] of Object.entries(limits)) {
    for (const task of tasks) {
      const job = prepareJsonSynthesisJob(parseJsonSynthesisSpec(JSON.stringify(variant(task))));
      let status = null;
      const t0 = Date.now();
      const r = new AscendRecSynthesizer({
        ...limit, searchSizeFactor: factor, useReductionRules: true, onlyForwardSearch: false,
        argListCompare: anyArgSmaller, onDiagnostics: (d) => { status = d.status; },
      }).synthesize(job.functionName, job.inputTypes, job.inputNames, job.returnType, job.env, job.examples);
      const ms = Date.now() - t0;
      console.log(`f${factor} ${vName} ${lName} ${task.name}: ${r ? `level ${r.program.depth} cost ${r.program.cost} ${ms}ms\n    ${showTerm(r.program.body)}` : `${status} ${ms}ms`}`);
    }
  }
}
}

// Usage (from the repository root): node <this> . <task-file>
// Re-runs each unlink helper task on every non-empty subset of its examples (adapter settings).
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import path from "node:path";
const [root, file] = process.argv.slice(2);
const imp = (p) => import(pathToFileURL(path.join(root, p)).href);
const { anyArgSmaller } = await imp("external/escher-ts/dist/components/component.js");
const { parseJsonSynthesisSpec, prepareJsonSynthesisJob } = await imp("external/escher-ts/dist/components/user-friendly-json.js");
const { AscendRecSynthesizer } = await imp("external/escher-ts/dist/synthesis/ascendrec/synthesizer.js");
const { showTerm } = await imp("external/escher-ts/dist/types/term.js");
for (const task of JSON.parse(readFileSync(file, "utf8"))) {
  const n = task.examples.length;
  for (let mask = 1; mask < 1 << n; mask += 1) {
    const ex = task.examples.filter((_, i) => mask & (1 << i));
    const job = prepareJsonSynthesisJob(parseJsonSynthesisSpec(JSON.stringify({ ...task, examples: ex })));
    const r = new AscendRecSynthesizer({ maxCost: 20, timeoutMs: 2000, searchSizeFactor: 3, useReductionRules: true, onlyForwardSearch: false, argListCompare: anyArgSmaller })
      .synthesize(job.functionName, job.inputTypes, job.inputNames, job.returnType, job.env, job.examples);
    const calls = [0, 1, 2].filter((i) => mask & (1 << i)).map((i) => i + 1).join(",");
    console.log(`${task.name} calls {${calls}}: ${r ? `c${r.program.cost} ${showTerm(r.program.body)}` : "FAIL"}`);
  }
}

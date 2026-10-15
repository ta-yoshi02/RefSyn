// Times escher-ts search alone, with the settings runtime/refsyn-escher-adapter.mjs uses,
// but without compiling the found term to JS. The adapter's compile step needs `refsynMeta`,
// which the fixed-ID baseline tasks (generated 2026-08-01) do not have, so through the
// adapter every baseline success is reported as a failure.
// Usage (from the repository root):
//   node <this> --file tasks.json --maxCost 32 --timeoutMs 15000 --repetitions 5 --out out.json
import { readFileSync, writeFileSync } from "node:fs";
import { performance } from "node:perf_hooks";
import { anyArgSmaller } from "../../../../../../external/escher-ts/dist/components/component.js";
import {
  parseJsonSynthesisSpec,
  prepareJsonSynthesisJob,
} from "../../../../../../external/escher-ts/dist/components/user-friendly-json.js";
import { AscendRecSynthesizer } from "../../../../../../external/escher-ts/dist/synthesis/ascendrec/synthesizer.js";
import { showTerm } from "../../../../../../external/escher-ts/dist/types/term.js";

const args = Object.fromEntries(
  process.argv.slice(2).reduce((acc, a, i, all) => (a.startsWith("--") ? [...acc, [a.slice(2), all[i + 1]]] : acc), []),
);
const maxCost = Number(args.maxCost);
const timeoutMs = Number(args.timeoutMs);
const repetitions = Number(args.repetitions ?? 1);
const tasks = JSON.parse(readFileSync(args.file, "utf8"));

const results = tasks.map((task) => {
  const runs = [];
  for (let iteration = 1; iteration <= repetitions; iteration++) {
    const started = performance.now();
    let success = false;
    let term = null;
    let error = null;
    try {
      const job = prepareJsonSynthesisJob(parseJsonSynthesisSpec(JSON.stringify(task)));
      const synth = new AscendRecSynthesizer({
        maxCost,
        timeoutMs,
        searchSizeFactor: 3,
        useReductionRules: true,
        onlyForwardSearch: false,
        argListCompare: anyArgSmaller,
      });
      const result = synth.synthesize(job.functionName, job.inputTypes, job.inputNames, job.returnType, job.env, job.examples);
      if (result === null) error = "no program within the budget";
      else {
        success = true;
        term = showTerm(result.program.body);
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
    runs.push({ iteration, elapsed_ms: Number((performance.now() - started).toFixed(3)), success, term, error });
  }
  const sorted = runs.map((r) => r.elapsed_ms).sort((a, b) => a - b);
  return {
    task: task.name,
    config: { maxCost, timeoutMs, searchSizeFactor: 3, repetitions, compile: false },
    success: runs.every((r) => r.success),
    summary: { min_ms: sorted[0], median_ms: sorted[Math.floor(sorted.length / 2)], max_ms: sorted.at(-1) },
    last_rendered: runs.at(-1).term ? `${task.name} =\n  ${runs.at(-1).term}` : null,
    last_error: runs.at(-1).error,
    runs,
  };
});
writeFileSync(args.out, JSON.stringify(results, null, 2) + "\n");
console.log(results.map((r) => `${r.task}: ${r.runs.filter((x) => x.success).length}/${r.runs.length} median ${r.summary.median_ms} ms ${r.last_error ?? ""}`).join("\n"));

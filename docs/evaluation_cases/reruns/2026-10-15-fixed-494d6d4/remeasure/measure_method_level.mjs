// Main measurement: per repetition, solves every task a method needs, one after another,
// and records the total. Time = escher-ts backend processing without translation to JS:
// JSON spec parsing, job preparation and synthesizer construction ("prepare") plus the
// AscendRec search ("search"), timed separately. Settings are those of
// runtime/refsyn-escher-adapter.mjs.
// Usage (from the repository root):
//   node <this> --side mold|baseline --method <name> --tasks <file> [--names a,b]
//               --maxCost N --timeoutMs N --repetitions N --out out.json
import { readFileSync, writeFileSync } from "node:fs";
import { performance } from "node:perf_hooks";
import { anyArgSmaller } from "../../../../../external/escher-ts/dist/components/component.js";
import {
  parseJsonSynthesisSpec,
  prepareJsonSynthesisJob,
} from "../../../../../external/escher-ts/dist/components/user-friendly-json.js";
import { AscendRecSynthesizer } from "../../../../../external/escher-ts/dist/synthesis/ascendrec/synthesizer.js";
import { showTerm } from "../../../../../external/escher-ts/dist/types/term.js";

const args = Object.fromEntries(
  process.argv.slice(2).reduce((acc, a, i, all) => (a.startsWith("--") ? [...acc, [a.slice(2), all[i + 1]]] : acc), []),
);
const maxCost = Number(args.maxCost);
const timeoutMs = Number(args.timeoutMs);
const repetitions = Number(args.repetitions);
let tasks = JSON.parse(readFileSync(args.tasks, "utf8"));
if (args.names) {
  const names = args.names.split(",");
  tasks = names.map((n) => tasks.find((t) => t.name === n));
}

const solve = (task) => {
  const t0 = performance.now();
  const job = prepareJsonSynthesisJob(parseJsonSynthesisSpec(JSON.stringify(task)));
  const synth = new AscendRecSynthesizer({
    maxCost,
    timeoutMs,
    searchSizeFactor: 3,
    useReductionRules: true,
    onlyForwardSearch: false,
    argListCompare: anyArgSmaller,
  });
  const t1 = performance.now();
  const result = synth.synthesize(job.functionName, job.inputTypes, job.inputNames, job.returnType, job.env, job.examples);
  const t2 = performance.now();
  return {
    task: task.name,
    prepare_ms: Number((t1 - t0).toFixed(3)),
    search_ms: Number((t2 - t1).toFixed(3)),
    total_ms: Number((t2 - t0).toFixed(3)),
    success: result !== null,
    term: result ? showTerm(result.program.body) : null,
  };
};

const reps = [];
for (let r = 1; r <= repetitions; r++) {
  const taskRuns = tasks.map(solve);
  reps.push({
    repetition: r,
    all_solved: taskRuns.every((x) => x.success),
    method_total_ms: Number(taskRuns.reduce((a, x) => a + x.total_ms, 0).toFixed(3)),
    method_search_ms: Number(taskRuns.reduce((a, x) => a + x.search_ms, 0).toFixed(3)),
    tasks: taskRuns,
  });
}
const median = (xs) => {
  const s = xs.slice().sort((a, b) => a - b);
  const n = s.length;
  return n % 2 ? s[(n - 1) / 2] : Number(((s[n / 2 - 1] + s[n / 2]) / 2).toFixed(3));
};
const stats = (xs) => ({ median: median(xs), min: Math.min(...xs), max: Math.max(...xs) });
const out = {
  side: args.side,
  method: args.method,
  tasks: tasks.map((t) => t.name),
  config: { maxCost, timeoutMs, searchSizeFactor: 3, repetitions, translation: false },
  node: process.version,
  solved_repetitions: reps.filter((r) => r.all_solved).length,
  method_total_ms: stats(reps.map((r) => r.method_total_ms)),
  method_search_ms: stats(reps.map((r) => r.method_search_ms)),
  repetitions: reps,
};
writeFileSync(args.out, JSON.stringify(out, null, 2) + "\n");
console.log(`${args.side} ${args.method}: solved ${out.solved_repetitions}/${repetitions}, total median ${out.method_total_ms.median} ms (search ${out.method_search_ms.median} ms)`);

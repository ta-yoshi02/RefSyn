// Differential check: the "+list" JS ports in tasks.json against the library components, on random
// heaps of references (including cycles, -1 and out-of-range entries) and random starts/arguments.
// Usage (from the repository root): node <this>
import { readFileSync } from "node:fs";
import { parseJsonSynthesisSpec, prepareJsonSynthesisJob } from "../../../../external/escher-ts/dist/components/user-friendly-json.js";
import { last_ptr, penultimateRef, nthNextRef, findByValueRef } from "../../../../external/escher-ts/dist/library/common-comps-heap.js";
import { valueRef, valueList, valueInt, valueError } from "../../../../external/escher-ts/dist/types/value.js";

const task = JSON.parse(readFileSync("docs/evaluation_cases/reruns/2026-10-30-baseline/tasks.json", "utf8")).find((t) => t.name === "unlink-whole-nextHeap+list");
const { case: _c, output: _o, ...spec } = task;
const env = prepareJsonSynthesisJob(parseJsonSynthesisSpec(JSON.stringify(spec))).env;
const lib = { last_ptr, penultimateRef, nthNextRef, findByValueRef };
let seed = 7;
const rnd = (n) => ((seed = (seed * 1103515245 + 12345) % 2147483648), seed % n);
let checked = 0, differ = 0;
for (let i = 0; i < 5000; i += 1) {
  const n = 1 + rnd(7);
  const heap = valueList(Array.from({ length: n }, () => valueRef(rnd(n + 2) - 1)));
  const nodes = valueList(Array.from({ length: n }, () => valueInt(0)));
  const vals = valueList(Array.from({ length: n }, () => (rnd(5) === 0 ? valueError : valueInt(rnd(4)))));
  const start = valueRef(rnd(n + 1) - 1);
  const argsOf = { last_ptr: [start, heap], penultimateRef: [start, heap], nthNextRef: [start, nodes, heap, valueInt(rnd(5))], findByValueRef: [start, nodes, heap, vals, valueInt(rnd(4))] };
  for (const name of Object.keys(lib)) {
    const a = JSON.stringify(lib[name].executeEfficient(argsOf[name]));
    const b = JSON.stringify(env.get(name).executeEfficient(argsOf[name]));
    checked += 1;
    if (a !== b) { differ += 1; if (differ <= 5) console.log(name, JSON.stringify(argsOf[name]).slice(0, 200), a, b); }
  }
}
console.log({ checked, differ });

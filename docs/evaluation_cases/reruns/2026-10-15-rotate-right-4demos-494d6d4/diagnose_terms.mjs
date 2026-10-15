// Diagnosis only: evaluates candidate terms for rotateRight_h1 (this.left.right) with
// escher-ts' own semantics on the examples of the saved task, without running a search.
// Usage (from the repository root): node <this> <task-file> <out.json>
import { readFileSync, writeFileSync } from "node:fs";
import {
  parseJsonSynthesisSpec,
  prepareJsonSynthesisJob,
} from "../../../../external/escher-ts/dist/components/user-friendly-json.js";
import { componentTerm as c, ifTerm, varTerm as v, executeTerm, showTerm } from "../../../../external/escher-ts/dist/types/term.js";

const [taskFile, outFile] = process.argv.slice(2);
const tasks = JSON.parse(readFileSync(taskFile, "utf8"));
const task = (Array.isArray(tasks) ? tasks : [tasks]).find((t) => t.name === "rotateRight_h1");
const job = prepareJsonSynthesisJob(parseJsonSynthesisSpec(JSON.stringify(task)));

const thisLeft = c("leftOf", [v("thisRef"), v("nodeHeap"), v("nodeHeap")]);
const candidates = {
  "found (4 demonstrations)": ifTerm(
    c("equal", [v("thisRef"), c("penultimateRef", [v("thisRef"), v("leftHeap")])]),
    c("rightOf", [c("last_ptr", [v("thisRef"), v("leftHeap")]), v("nodeHeap"), v("nodeHeap")]),
    c("last_ptr", [c("penultimateRef", [v("thisRef"), v("leftHeap")]), v("rightHeap")]),
  ),
  "found (3 demonstrations)": c("last_ptr", [thisLeft, v("rightHeap")]),
  "rightOf(leftOf(this)) with nodeHeap": c("rightOf", [thisLeft, v("nodeHeap"), v("nodeHeap")]),
  "rightOf(leftOf(this)) with field heaps": c("rightOf", [c("leftOf", [v("thisRef"), v("nodeHeap"), v("leftHeap")]), v("nodeHeap"), v("rightHeap")]),
};

const show = (x) => JSON.stringify(x);
const rows = Object.entries(candidates).map(([name, term]) => {
  const perExample = job.examples.map(([args, expected], i) => {
    const vars = new Map(job.inputNames.map((n, k) => [n, args[k]]));
    let got;
    try {
      got = executeTerm((n) => vars.get(n), job.env, term);
    } catch (e) {
      got = { error: String(e && e.message) };
    }
    return { example: i + 1, expected: show(expected), got: show(got), ok: show(got) === show(expected) };
  });
  return { name, term: showTerm(term), satisfies_all_examples: perExample.every((e) => e.ok), perExample };
});

const out = {
  task: task.name,
  inputs: job.inputNames,
  components: [...job.env.keys()].sort(),
  candidates: rows,
};
writeFileSync(outFile, JSON.stringify(out, null, 2) + "\n");
console.log("components:", out.components.join(", "));
for (const r of rows) console.log(r.satisfies_all_examples ? "satisfies  " : "fails      ", r.name, "|", r.term, "|", r.perExample.map((e) => (e.ok ? "ok" : `#${e.example} got ${e.got}`)).join(" "));

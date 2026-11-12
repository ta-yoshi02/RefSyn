// Diagnosis only: evaluates candidate terms for union_h0 (the root that gets a parent) with
// escher-ts' own semantics on the examples of the saved task, with lt added, without a search.
// Usage (from the repository root): node <this> . <task-file>
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import path from "node:path";

const [root, file] = process.argv.slice(2);
const imp = (p) => import(pathToFileURL(path.join(root, p)).href);
const { parseJsonSynthesisSpec, prepareJsonSynthesisJob } = await imp("external/escher-ts/dist/components/user-friendly-json.js");
const { componentTerm: c, ifTerm, varTerm: v, executeTerm, showTerm } = await imp("external/escher-ts/dist/types/term.js");

const lt = { name: "lt", kind: "js", inputTypes: ["Int", "Int"], returnType: "Bool", args: ["x", "y"], bodyJs: "return x < y;" };
const task = JSON.parse(readFileSync(file, "utf8")).find((t) => t.name === "union_h0");
const job = prepareJsonSynthesisJob(parseJsonSynthesisSpec(JSON.stringify({ ...task, components: [...task.components, lt] })));

const rank = (x) => c("loadInt", [v("rankHeap"), c("rankOf", [v(x), v("nodeHeap"), v("rankHeap")])]);
const candidates = {
  "intended: if lt(rank this, rank other) then this else other": ifTerm(c("lt", [rank("thisRef"), rank("other")]), v("thisRef"), v("other")),
  "found, factor 3": ifTerm(
    c("isZero", [rank("thisRef")]),
    v("thisRef"),
    ifTerm(c("isNull", [c("findByValueRef", [v("other"), v("nodeHeap"), v("parentHeap"), v("rankHeap"), c("inc", [c("inc", [c("zero", [])])])])]), v("other"), v("thisRef")),
  ),
  "found, factor 1": ifTerm(
    c("isNull", [c("findByValueRef", [v("thisRef"), v("nodeHeap"), v("parentHeap"), v("rankHeap"), c("inc", [rank("other")])])]),
    v("thisRef"),
    v("other"),
  ),
};

const size = (t) => (t.kind === "var" ? 1 : t.kind === "if" ? 1 + size(t.condition) + size(t.thenBranch) + size(t.elseBranch) : 1 + t.args.reduce((s, a) => s + size(a), 0));
const show = (x) => JSON.stringify(x);
const rows = Object.entries(candidates).map(([name, term]) => {
  const perExample = job.examples.map(([args, expected], i) => {
    const vars = new Map(job.inputNames.map((n, k) => [n, args[k]]));
    const got = executeTerm((n) => vars.get(n), job.env, term);
    return { example: i + 1, expected: show(expected), got: show(got), ok: show(got) === show(expected) };
  });
  return { name, term: showTerm(term), cost: size(term), satisfiesAllExamples: perExample.every((e) => e.ok), perExample };
});
console.log(JSON.stringify(rows, null, 2));

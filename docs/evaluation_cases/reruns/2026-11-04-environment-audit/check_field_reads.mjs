// Checks that the accessors read the state in tasks_fields.json and tasks_minimal.json: for every example
// input and every object i of nodeHeap, loadInt(fHeap, fOf(i, nodeHeap, fHeap)) must equal fHeap[i] and
// gOf(i, nodeHeap, gHeap) must equal gHeap[i]. MOLD's popBack and insert tasks (valOf, nextOf) are checked
// the same way for comparison. Before the encoding was aligned, insert's reads were errors and the
// other tasks' fOf returned the slot, a reference, where its type said Int.
// Usage (from the repository root): node <this> > field_reads.json
import { readFileSync } from "node:fs";
import { parseJsonSynthesisSpec, prepareJsonSynthesisJob } from "../../../../external/escher-ts/dist/components/user-friendly-json.js";
import { showType } from "../../../../external/escher-ts/dist/types/type.js";

const D = "docs/evaluation_cases/reruns/2026-11-04-environment-audit";
const R = "docs/evaluation_cases/reruns";
const read = (p) => JSON.parse(readFileSync(p, "utf8"));
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);

const check = (task, valField, ptrField) => {
  const { case: _c, output: _o, evaluationMethod: _m, refsynMeta: _r, ...spec } = task;
  const job = prepareJsonSynthesisJob(parseJsonSynthesisSpec(JSON.stringify(spec)));
  const comp = (n) => [...job.env.values()].find((c) => c.name === n);
  const [valOf, ptrOf, loadInt] = [comp(`${valField}Of`), comp(`${ptrField}Of`), comp("loadInt")];
  const types = Object.fromEntries([valOf, ptrOf, loadInt].map((c) => [c.name, `${c.inputTypes.map(showType).join(" × ")} → ${showType(c.returnType)}`]));
  let reads = 0;
  const failures = [];
  job.examples.forEach(([args], e) => {
    const v = new Map(job.inputNames.map((n, k) => [n, args[k]]));
    const nodes = v.get("nodeHeap");
    const valHeap = v.get(`${valField}Heap`);
    const ptrHeap = v.get(`${ptrField}Heap`);
    for (let i = 0; i < nodes.elems.length; i += 1) {
      const ref = { tag: "ref", value: i };
      const val = loadInt.executeEfficient([valHeap, valOf.executeEfficient([ref, nodes, valHeap])]);
      const ptr = ptrOf.executeEfficient([ref, nodes, ptrHeap]);
      reads += 2;
      if (!same(val, valHeap.elems[i])) failures.push({ example: e, object: i, field: valField, got: val, want: valHeap.elems[i] });
      if (!same(ptr, ptrHeap.elems[i])) failures.push({ example: e, object: i, field: ptrField, got: ptr, want: ptrHeap.elems[i] });
    }
  });
  return { task: task.name, types, reads, failures };
};

const out = [
  ...read(`${D}/tasks_fields.json`).map((t) => ({ variant: "fields", ...check(t, "f", "g") })),
  ...read(`${D}/tasks_minimal.json`).map((t) => ({ variant: "minimal", ...check(t, "f", "g") })),
  ...["popBack", "insert_general"].flatMap((m) => read(`${R}/2026-10-14-fresh-recording/${m}/mold_escher_tasks.json`).map((t) => ({ variant: "MOLD", ...check(t, "val", "next") }))),
];
console.log(JSON.stringify(out, null, 2));

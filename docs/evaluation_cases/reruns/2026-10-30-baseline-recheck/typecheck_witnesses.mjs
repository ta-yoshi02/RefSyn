// Type-checks the hand-written witnesses of ../../nonseparated_general_components/ (built exactly as in
// scripts/prepare_nonseparated_general_baseline.mjs) against the task signatures and component types
// that the synthesizer uses. The original witness check only executed the terms (executeTerm does not
// type-check), so a witness could match every example and still be outside the search space.
// Also reports, per task, which components of the task can be applied to the task's inputs at all.
// Usage (from the repository root): [TASKS=<tasks.json>] node <this> > <out.json>
import { readFileSync } from "node:fs";
import { parseJsonSynthesisSpec, prepareJsonSynthesisJob } from "../../../../external/escher-ts/dist/components/user-friendly-json.js";
import { showTerm } from "../../../../external/escher-ts/dist/types/term.js";
import { showType, shiftId, unify, TypeSubst } from "../../../../external/escher-ts/dist/types/type.js";

const tasks = JSON.parse(readFileSync(process.env.TASKS ?? "docs/evaluation_cases/nonseparated_general_components/tasks.json", "utf8"));

const v = (name) => ({ kind: "var", name });
const c = (name, ...args) => ({ kind: "component", name, args });
const append = (heap, value) => c("insert", heap, c("length", heap), value);
const nullFromInput = c("nthNextRef", v("thisRef"), v("nodeHeap"), v("gHeap"), c("length", v("nodeHeap")));
const setAtF = c("storeAtRef", v("fHeap"), c("nthNextRef", v("thisRef"), v("nodeHeap"), v("gHeap"), v("arg0")), v("arg1"));
const appendF = append(v("fHeap"), v("arg0"));
const appendG = append(c("storeAtRef", v("gHeap"), c("last_ptr", v("thisRef"), v("gHeap")), c("freshRefFromHeap", v("nodeHeap"))), nullFromInput);
const prependF = append(v("fHeap"), v("arg0"));
const prependG = append(v("gHeap"), v("thisRef"));
const insertF = append(v("fHeap"), v("arg1"));
const insertG = append(
  c("storeAtRef", v("gHeap"), c("nthNextRef", v("thisRef"), v("nodeHeap"), v("gHeap"), v("arg0")), c("freshRefFromHeap", v("nodeHeap"))),
  c("nthNextRef", v("thisRef"), v("nodeHeap"), v("gHeap"), c("inc", v("arg0"))),
);
const popBackG = c("storeAtRef", v("gHeap"), c("penultimateRef", v("thisRef"), v("gHeap")), nullFromInput);
const witnesses = {
  "setAt-fixed-fHeap": setAtF, "setAt-fixed-gHeap": v("gHeap"), "setAt-fixed-env-pair": c("pairHeap", setAtF, v("gHeap")),
  "append-fixed-fHeap": appendF, "append-fixed-gHeap": appendG, "append-fixed-env-pair": c("pairHeap", appendF, appendG),
  "prepend-fixed-fHeap": prependF, "prepend-fixed-gHeap": prependG, "prepend-fixed-returnRef": c("freshRefFromHeap", v("nodeHeap")),
  "prepend-fixed-env-pair": c("pairHeap", prependF, prependG),
  "insert-fixed-fHeap": insertF, "insert-fixed-gHeap": insertG, "insert-fixed-env-pair": c("pairHeap", insertF, insertG),
  "popBack-fixed-fHeap": v("fHeap"), "popBack-fixed-gHeap": popBackG, "popBack-fixed-env-pair": c("pairHeap", v("fHeap"), popBackG),
};

// Hindley-Milner style check: each component use gets fresh type variables.
const check = (job, term) => {
  let subst = TypeSubst.empty;
  let fresh = 1000;
  const errors = [];
  const infer = (t) => {
    if (t.kind === "var") return job.inputTypes[job.inputNames.indexOf(t.name)];
    const impl = job.env.get(t.name);
    const shift = fresh; fresh += 100;
    const params = impl.inputTypes.map((ty) => shiftId(ty, shift));
    t.args.forEach((arg, i) => {
      const actual = infer(arg);
      if (actual === null) return;
      const s = unify(subst.apply(params[i]), subst.apply(actual));
      if (s === null) errors.push(`${t.name} argument ${i + 1}: expected ${showType(subst.apply(params[i]))}, got ${showType(subst.apply(actual))} (${showTerm(arg)})`);
      else subst = subst.compose(s);
    });
    return shiftId(impl.returnType, shift);
  };
  const ty = infer(term);
  const s = unify(subst.apply(ty), job.returnType);
  if (s === null) errors.push(`result: expected ${showType(job.returnType)}, got ${showType(subst.apply(ty))}`);
  return errors;
};

const out = tasks.map((task) => {
  const job = prepareJsonSynthesisJob(parseJsonSynthesisSpec(JSON.stringify(task)));
  const w = witnesses[task.name];
  const errors = check(job, w);
  // Which list components accept (thisRef, gHeap) / (thisRef, nodeHeap, gHeap, ...) at all?
  const probes = {
    "last_ptr(thisRef, gHeap)": c("last_ptr", v("thisRef"), v("gHeap")),
    "penultimateRef(thisRef, gHeap)": c("penultimateRef", v("thisRef"), v("gHeap")),
    "nthNextRef(thisRef, nodeHeap, gHeap, length(nodeHeap))": nullFromInput,
  };
  return {
    task: task.name,
    witness: showTerm(w),
    well_typed: errors.length === 0,
    errors,
    probes: Object.fromEntries(Object.entries(probes).map(([k, t]) => [k, check({ ...job, returnType: shiftId(job.env.get(t.name).returnType, 50000) }, t).length === 0])),
  };
});
console.log(JSON.stringify(out, null, 2));

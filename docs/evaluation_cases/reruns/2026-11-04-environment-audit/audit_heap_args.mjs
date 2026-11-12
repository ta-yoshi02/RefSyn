// components.json counts a component as applicable if any well-typed term uses it, which also accepts
// meaningless uses such as last_ptr(thisRef, nodeHeap). This lists, for every component that takes a
// heap (a List-typed parameter), the well-typed applications to the task's own input variables (Int
// parameters filled by zero()), so one can see which heap each component can actually read.
// Usage (from the repository root): node <this> > heap_args.json
import { readFileSync } from "node:fs";
import { parseJsonSynthesisSpec, prepareJsonSynthesisJob } from "../../../../external/escher-ts/dist/components/user-friendly-json.js";
import { shiftId, showType, unify, TypeSubst, tyInt } from "../../../../external/escher-ts/dist/types/type.js";

const D = "docs/evaluation_cases/reruns/2026-11-04-environment-audit";
const R = "docs/evaluation_cases/reruns";
const read = (p) => JSON.parse(readFileSync(p, "utf8"));
const published = read("docs/evaluation_cases/nonseparated_general_components/tasks.json");
const paperFixed = read(`${R}/2026-10-30-baseline-recheck/tasks_list.json`);
const ownBaseline = read(`${R}/2026-10-30-baseline/tasks.json`);

const sources = {
  "list/MOLD": [`${R}/2026-10-14-fresh-recording/popBack/mold_escher_tasks.json`, null],
  "list/paper as published": [null, published.filter((t) => t.evaluationMethod === "popBack")],
  "list/paper retyped (followthrough)": [null, paperFixed.filter((t) => t.evaluationMethod === "popBack")],
  "tree/MOLD": [`${R}/2026-10-30-new-cases/rotateLeft/mold_escher_tasks.json`, null],
  "tree/2026-10-30 baseline": [null, ownBaseline.filter((t) => t.case === "rotateLeft" && !t.name.endsWith("+list"))],
  "tree/2026-10-30 baseline +list": [null, ownBaseline.filter((t) => t.case === "rotateLeft" && t.name.endsWith("+list"))],
  "dll/MOLD": [`${R}/2026-10-30-dll-unlink/unlink/mold_escher_tasks.json`, null],
  "dll/2026-10-30 baseline": [null, ownBaseline.filter((t) => t.case === "unlink" && !t.name.endsWith("+list"))],
  "dll/2026-10-30 baseline +list": [null, ownBaseline.filter((t) => t.case === "unlink" && t.name.endsWith("+list"))],
};

const jobOf = (task) => {
  const { case: _c, output: _o, evaluationMethod: _m, refsynMeta: _r, ...spec } = task;
  return prepareJsonSynthesisJob(parseJsonSynthesisSpec(JSON.stringify(spec)));
};

const applications = (job) => {
  const vars = job.inputNames.map((n, i) => [`@${n}`, job.inputTypes[i]]);
  const candidates = [...vars, ["zero()", tyInt]];
  const out = {};
  for (const impl of job.env.values()) {
    if (!impl.inputTypes.some((t) => t.kind === "apply" && t.constructor.name === "List")) continue;
    const params = impl.inputTypes.map((t) => shiftId(t, 10000));
    const found = [];
    const go = (i, subst, chosen) => {
      if (i === params.length) { found.push(`${impl.name}(${chosen.join(", ")})`); return; }
      for (const [label, ty] of candidates) {
        const s = unify(subst.apply(params[i]), subst.apply(ty));
        if (s !== null) go(i + 1, subst.compose(s), [...chosen, label]);
      }
    };
    go(0, TypeSubst.empty, []);
    out[impl.name] = { type: `${impl.inputTypes.map(showType).join(" × ")} → ${showType(impl.returnType)}`, applications: found };
  }
  return out;
};

const result = {};
for (const [label, [moldPath, tasks]] of Object.entries(sources)) {
  const task = moldPath ? read(moldPath)[0] : tasks[0];
  const job = jobOf(task);
  result[label] = {
    task: task.name,
    inputs: job.inputNames.map((n, i) => `${n}:${showType(job.inputTypes[i])}`),
    components: applications(job),
  };
}
console.log(JSON.stringify(result, null, 2));

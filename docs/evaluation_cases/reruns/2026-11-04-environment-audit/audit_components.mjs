// Compares, case by case, the synthesis environment that MOLD's helper tasks and the non-separated
// baseline tasks give escher-ts: the components after prepareJsonSynthesisJob (so including the ones
// escher-ts adds itself: field accessors, leInt, loadInt, ...), their types, and whether each one can be
// applied at all to the values the task provides.
// "Applicable" is decided by a closure over types, not values: starting from the task's input types,
// every component whose parameter types unify with available types adds its result type (5 rounds,
// result types longer than 80 characters dropped). It ignores costs and examples, so "applicable" means
// "some well-typed term uses it", not "the search reaches it within the limits".
// Usage (from the repository root): node <this> > components.json
import { readFileSync } from "node:fs";
import { parseJsonSynthesisSpec, prepareJsonSynthesisJob } from "../../../../external/escher-ts/dist/components/user-friendly-json.js";
import { alphaNormalForm, shiftId, showType, unify, TypeSubst } from "../../../../external/escher-ts/dist/types/type.js";

const R = "docs/evaluation_cases/reruns";
const read = (p) => JSON.parse(readFileSync(p, "utf8"));
const published = read("docs/evaluation_cases/nonseparated_general_components/tasks.json");
const paperFixed = read(`${R}/2026-10-30-baseline-recheck/tasks_list.json`);
const ownBaseline = read(`${R}/2026-10-30-baseline/tasks.json`);
const appendComps = [{ name: "insert", kind: "libraryRef", ref: "insert" }, { name: "length", kind: "libraryRef", ref: "length" }];

const listCase = (method, moldDir, baseMethod) => ({
  mold: read(`${R}/2026-10-14-fresh-recording/${moldDir}/mold_escher_tasks.json`),
  baselines: {
    "paper as published": published.filter((t) => t.evaluationMethod === baseMethod),
    "paper, list components retyped (followthrough)": paperFixed.filter((t) => t.evaluationMethod === baseMethod),
  },
});
const ownCase = (name, moldDir, extra = []) => {
  const mine = (suffix) => ownBaseline.filter((t) => t.case === name && (suffix ? t.name.endsWith(suffix) : !t.name.endsWith("+list")));
  const withExtra = (ts) => ts.map((t) => ({ ...t, components: [...t.components, ...extra] }));
  return {
    mold: read(`${R}/${moldDir}/mold_escher_tasks.json`),
    baselines: {
      "2026-10-30 baseline": withExtra(mine(null)),
      "2026-10-30 baseline +list": withExtra(mine("+list")),
    },
  };
};
const cases = {
  setAt: listCase("setAt", "setAt", "setAt"),
  append: listCase("append", "append", "append"),
  prepend: listCase("prepend", "prepend", "prepend"),
  insert: listCase("insert", "insert_general", "insert"),
  popBack: listCase("popBack", "popBack", "popBack"),
  rotateRight3: ownCase("rotateRight3", "2026-10-15-rotate-right-494d6d4/rotateRight"),
  rotateRight4: ownCase("rotateRight4", "2026-10-15-rotate-right-4demos-494d6d4/rotateRight"),
  unlink: ownCase("unlink", "2026-10-30-dll-unlink/unlink"),
  union: ownCase("union", "2026-10-30-uf-union/union"),
  swapChildren: ownCase("swapChildren", "2026-10-30-new-cases/swapChildren"),
  // As run by run_insert_after_append.mjs (insert and length added).
  insertAfter: ownCase("insertAfter", "2026-10-30-new-cases/insertAfter", appendComps),
  rotateLeft: ownCase("rotateLeft", "2026-10-30-new-cases/rotateLeft"),
};

const jobOf = (task) => {
  const { case: _c, output: _o, evaluationMethod: _m, refsynMeta: _r, ...spec } = task;
  return prepareJsonSynthesisJob(parseJsonSynthesisSpec(JSON.stringify(spec)));
};

const applicable = (job) => {
  const types = new Map(job.inputTypes.map((t) => [showType(t), t]));
  const used = new Set();
  for (let round = 0; round < 5; round += 1) {
    const avail = [...types.values()];
    let grew = false;
    for (const impl of job.env.values()) {
      const params = impl.inputTypes.map((t) => shiftId(t, 10000));
      const results = [];
      const go = (i, subst) => {
        if (results.length > 50) return;
        if (i === params.length) { results.push(alphaNormalForm(subst.apply(shiftId(impl.returnType, 10000)))); return; }
        avail.forEach((a, k) => {
          const s = unify(subst.apply(params[i]), subst.apply(shiftId(a, 20000 + 100 * (i * 64 + k))));
          if (s !== null) go(i + 1, subst.compose(s));
        });
      };
      go(0, TypeSubst.empty);
      if (results.length > 0) used.add(impl.name);
      for (const t of results) {
        const key = showType(t);
        if (key.length <= 80 && !types.has(key)) { types.set(key, t); grew = true; }
      }
    }
    if (!grew) break;
  }
  return used;
};

const summarize = (tasks) => {
  const comps = new Map();
  const usable = new Set();
  for (const task of tasks) {
    const job = jobOf(task);
    for (const impl of job.env.values()) comps.set(impl.name, `${impl.inputTypes.map(showType).join(" × ")} → ${showType(impl.returnType)}`);
    for (const n of applicable(job)) usable.add(n);
  }
  return { comps, usable };
};

const out = {};
for (const [name, c] of Object.entries(cases)) {
  const m = summarize(c.mold);
  const entry = {
    mold_tasks: c.mold.map((t) => t.name),
    mold_inputs: c.mold.map((t) => { const j = jobOf(t); return j.inputNames.map((n, i) => `${n}:${showType(j.inputTypes[i])}`).join(", "); }),
    mold_examples: c.mold.map((t) => t.examples.length),
    baselines: {},
  };
  for (const [bname, tasks] of Object.entries(c.baselines)) {
    const b = summarize(tasks);
    const names = new Set([...m.comps.keys(), ...b.comps.keys()]);
    const rows = [...names].sort().map((n) => ({
      component: n,
      mold: m.comps.has(n) ? (m.usable.has(n) ? "usable" : "present, never applicable") : "absent",
      baseline: b.comps.has(n) ? (b.usable.has(n) ? "usable" : "present, never applicable") : "absent",
      mold_type: m.comps.get(n) ?? null,
      baseline_type: b.comps.get(n) ?? null,
    }));
    entry.baselines[bname] = {
      tasks: tasks.map((t) => t.name),
      examples: tasks.map((t) => t.examples.length),
      inputs: tasks.length ? (() => { const j = jobOf(tasks[0]); return j.inputNames.map((n, i) => `${n}:${showType(j.inputTypes[i])}`).join(", "); })() : null,
      differences: rows.filter((r) => (r.mold === "usable") !== (r.baseline === "usable")),
      all: rows,
    };
  }
  out[name] = entry;
}
console.log(JSON.stringify(out, null, 2));

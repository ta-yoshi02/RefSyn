// Sanity check of check_offdemo.mjs: the hand-written witnesses (as in typecheck_witnesses.mjs), with
// the ported list components of tasks_list.json, must agree with the reference methods on every input.
// Usage (from the repository root): WEIGHTED_DIST=<patched dist> node <this>
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import path from "node:path";

const root = process.cwd();
const D = "docs/evaluation_cases/reruns/2026-10-30-baseline-recheck";
const load = async (dist) => {
  const imp = (p) => import(pathToFileURL(path.join(dist, p)).href);
  return { ...(await imp("components/component.js")), ...(await imp("components/user-friendly-json.js")), ...(await imp("synthesis/ascendrec/synthesizer.js")), ...(await imp("types/term.js")) };
};
const libs = { current: await load(path.join(root, "external/escher-ts/dist")), candidate: await load(process.env.WEIGHTED_DIST) };
const costs = { current: undefined, candidate: { last_ptr: 3, penultimateRef: 3, nthNextRef: 3, findByValueRef: 3 } };
const factor = { current: 3, candidate: 1 };
const variants = {
  published: JSON.parse(readFileSync("docs/evaluation_cases/nonseparated_general_components/tasks.json", "utf8")),
  list: JSON.parse(readFileSync(`${D}/tasks_list.json`, "utf8")),
};
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
// A state: f[i], g[i] (index or -1), head index.
const makeList = (n, reversed) => {
  const order = [...Array(n).keys()].map((k) => (reversed ? n - 1 - k : k)); // order[k] = index of k-th node
  const f = Array(n), g = Array(n);
  order.forEach((idx, k) => { f[idx] = (k + 1) * 10 + 1; g[idx] = k + 1 < n ? order[k + 1] : -1; });
  return { f, g, head: order[0], order };
};
const XS = [0, 1, 2, 3, 99];
const cases = [];
for (let n = 1; n <= 6; n += 1) {
  for (const reversed of [false, true]) {
    const s = () => makeList(n, reversed);
    for (let i = 0; i < n; i += 1) cases.push({ method: "setAt", s: s(), args: [i, 77], ref: (t) => { t.f[t.order[i]] = 77; return null; } });
    for (const x of XS) {
      cases.push({ method: "append", s: s(), args: [x], ref: (t) => { const m = t.f.length; t.f.push(x); t.g.push(-1); t.g[t.order[n - 1]] = m; return null; } });
      cases.push({ method: "prepend", s: s(), args: [x], ref: (t) => { const m = t.f.length; t.f.push(x); t.g.push(t.head); return m; } });
      for (let i = 0; i < n; i += 1) cases.push({ method: "insert", s: s(), args: [i, x], ref: (t) => { const m = t.f.length; const p = t.order[i]; t.f.push(x); t.g.push(t.g[p]); t.g[p] = m; return null; } });
    }
    if (n >= 2) cases.push({ method: "popBack", s: s(), args: [], ref: (t) => { t.g[t.order[n - 2]] = -1; return null; } });
  }
}

const encodeInput = (task, s, args) => {
  const emptyFields = Object.keys(task.examples[0][0][1][0].object.fields).length === 0;
  const nodeHeap = s.f.map((_, i) => ({ object: { className: "Obj", fields: emptyFields ? {} : { f: { ref: i }, g: { ref: s.g[i] } } } }));
  return [{ ref: s.head }, nodeHeap, [...s.f], s.g.map((r) => ({ ref: r })), ...args];
};
const encodeOutput = (task, t, ret) => {
  const output = task.name.replace(/^\w+-fixed-/, "");
  if (output === "fHeap") return t.f;
  if (output === "gHeap") return t.g.map((r) => ({ ref: r }));
  if (output === "returnRef") return { ref: ret };
  return { pair: [t.f, t.g.map((r) => ({ ref: r }))] };
};

const out = [];
const L = libs.current;
for (const task of variants.list) {
  const { evaluationMethod: method, ...spec } = task;
  const job = L.prepareJsonSynthesisJob(L.parseJsonSynthesisSpec(JSON.stringify(spec)));
  const mine = cases.filter((c) => c.method === method);
  let agree = 0;
  for (const c of mine) {
    const t = { f: [...c.s.f], g: [...c.s.g], head: c.s.head, order: c.s.order };
    const ret = c.ref(t);
    const [[args0, want]] = L.prepareJsonSynthesisJob(L.parseJsonSynthesisSpec(JSON.stringify({ ...spec, examples: [[encodeInput(task, c.s, c.args), encodeOutput(task, t, ret)]] }))).examples;
    const vars = new Map(job.inputNames.map((n, k) => [n, args0[k]]));
    if (JSON.stringify(L.executeTerm((n) => vars.get(n), job.env, witnesses[task.name])) === JSON.stringify(want)) agree += 1;
  }
  console.log(task.name, `${agree}/${mine.length}`);
}

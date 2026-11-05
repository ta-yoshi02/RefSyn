// check_offdemo.mjs of ../2026-10-30-baseline-recheck/ (same inputs and reference methods) on the terms
// of rerun_fields.mjs.
// Usage (from the repository root): WEIGHTED_DIST=<patched dist> node <this> > offdemo_fields.json
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import path from "node:path";

const root = process.cwd();
const D = "docs/evaluation_cases/reruns/2026-11-04-environment-audit";
const load = async (dist) => {
  const imp = (p) => import(pathToFileURL(path.join(dist, p)).href);
  return { ...(await imp("components/component.js")), ...(await imp("components/user-friendly-json.js")), ...(await imp("synthesis/ascendrec/synthesizer.js")), ...(await imp("types/term.js")) };
};
const libs = { current: await load(path.join(root, "external/escher-ts/dist")), candidate: await load(process.env.WEIGHTED_DIST) };
const costs = { current: undefined, candidate: { last_ptr: 3, penultimateRef: 3, nthNextRef: 3, findByValueRef: 3 } };
const factor = { current: 3, candidate: 1 };
const variants = {
  fields: JSON.parse(readFileSync(`${D}/tasks_fields.json`, "utf8")),
};
const results = JSON.parse(readFileSync(`${D}/rerun_fields.json`, "utf8"));

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

// The term is recovered by searching again; only the saved term string is timed. The limit here is
// 120 s so that a slow machine does not truncate the search, and the recovered term must equal the saved
// one. A row that cannot be checked is kept with its error and makes the script exit with code 1.
const out = [];
for (const row of results.filter((r) => r.term !== null)) {
  const task = variants[row.variant].find((t) => t.name === row.task);
  const { evaluationMethod: method, ...spec } = task;
  const L = libs[row.setting];
  globalThis.__refsynCompCost = costs[row.setting];
  const job = L.prepareJsonSynthesisJob(L.parseJsonSynthesisSpec(JSON.stringify(spec)));
  const r = new L.AscendRecSynthesizer({ maxCost: 32, timeoutMs: 120000, searchSizeFactor: factor[row.setting], useReductionRules: true, onlyForwardSearch: false, argListCompare: L.anyArgSmaller })
    .synthesize(job.functionName, job.inputTypes, job.inputNames, job.returnType, job.env, job.examples);
  if (!r || L.showTerm(r.program.body) !== row.term) { out.push({ ...row, error: "search did not repeat" }); continue; }
  const mine = cases.filter((c) => c.method === method);
  let agree = 0;
  const failures = [];
  for (const c of mine) {
    const t = { f: [...c.s.f], g: [...c.s.g], head: c.s.head, order: c.s.order };
    const ret = c.ref(t);
    const ex = [encodeInput(task, c.s, c.args), encodeOutput(task, t, ret)];
    const [[args0, want]] = L.prepareJsonSynthesisJob(L.parseJsonSynthesisSpec(JSON.stringify({ ...spec, examples: [ex] }))).examples;
    const vars = new Map(job.inputNames.map((n, k) => [n, args0[k]]));
    if (JSON.stringify(L.executeTerm((n) => vars.get(n), job.env, r.program.body)) === JSON.stringify(want)) agree += 1;
    else if (failures.length < 3) failures.push({ n: c.s.f.length, head: c.s.head, args: c.args });
  }
  out.push({ variant: row.variant, task: row.task, setting: row.setting, term: row.term, agree, total: mine.length, failures });
}
console.log(JSON.stringify(out, null, 2));
const unchecked = out.filter((r) => r.error);
if (unchecked.length > 0) {
  console.error(`${unchecked.length} row(s) not checked: ${unchecked.map((r) => `${r.task} ${r.setting}`).join(", ")}`);
  process.exitCode = 1;
}

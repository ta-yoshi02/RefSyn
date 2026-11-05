// Is the baseline's overfitting a matter of the demonstrations? Counterexample-guided loop on the
// decisive tasks of tasks_minimal.json (current setting, maxCost 32, 15 s): synthesize; check the term on
// the undemonstrated inputs of check_offdemo_minimal.mjs; if one disagrees, add the smallest such input
// (fewest objects, then first) with the reference method's output as a new example; repeat (at most 6
// added examples). The added examples come from the reference method, i.e. they are a stronger
// specification than the demonstrations, which MOLD did not get.
// Usage (from the repository root): node <this> > cegis_minimal.json
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import path from "node:path";

const root = process.cwd();
const D = "docs/evaluation_cases/reruns/2026-11-04-environment-audit";
const load = async (dist) => {
  const imp = (p) => import(pathToFileURL(path.join(dist, p)).href);
  return { ...(await imp("components/component.js")), ...(await imp("components/user-friendly-json.js")), ...(await imp("synthesis/ascendrec/synthesizer.js")), ...(await imp("types/term.js")) };
};
const libs = { current: await load(path.join(root, "external/escher-ts/dist")) };
const variants = {
  minimal: JSON.parse(readFileSync(`${D}/tasks_minimal.json`, "utf8")),
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

const L = libs.current;
const decisive = ["setAt-fixed-env-pair", "append-fixed-env-pair", "prepend-fixed-env-pair", "prepend-fixed-returnRef", "insert-fixed-env-pair", "popBack-fixed-env-pair"];
const out = [];
for (const name of decisive) {
  const task = variants.minimal.find((t) => t.name === name);
  const { evaluationMethod: method, ...spec0 } = task;
  const spec = { ...spec0, examples: [...spec0.examples] };
  const mine = cases.filter((c) => c.method === method);
  const rounds = [];
  for (let round = 0; round <= 6; round += 1) {
    const job = L.prepareJsonSynthesisJob(L.parseJsonSynthesisSpec(JSON.stringify(spec)));
    const t0 = Date.now();
    const r = new L.AscendRecSynthesizer({ maxCost: 32, timeoutMs: 15000, searchSizeFactor: 3, useReductionRules: true, onlyForwardSearch: false, argListCompare: L.anyArgSmaller })
      .synthesize(job.functionName, job.inputTypes, job.inputNames, job.returnType, job.env, job.examples);
    const ms = Date.now() - t0;
    if (!r) { rounds.push({ examples: spec.examples.length, ms, term: null }); break; }
    const failing = [];
    let agree = 0;
    for (const c of mine) {
      const t = { f: [...c.s.f], g: [...c.s.g], head: c.s.head, order: c.s.order };
      const ret = c.ref(t);
      const ex = [encodeInput(task, c.s, c.args), encodeOutput(task, t, ret)];
      const [[args0, want]] = L.prepareJsonSynthesisJob(L.parseJsonSynthesisSpec(JSON.stringify({ ...spec, examples: [ex] }))).examples;
      const vars = new Map(job.inputNames.map((n, k) => [n, args0[k]]));
      if (JSON.stringify(L.executeTerm((n) => vars.get(n), job.env, r.program.body)) === JSON.stringify(want)) agree += 1;
      else failing.push({ c, ex });
    }
    const term = L.showTerm(r.program.body);
    failing.sort((a, b) => a.c.s.f.length - b.c.s.f.length);
    const added = failing[0];
    rounds.push({ examples: spec.examples.length, ms, term, cost: r.program.cost, agree, total: mine.length, added: added ? { n: added.c.s.f.length, head: added.c.s.head, args: added.c.args } : null });
    console.error(`${name} round ${round}: ${term} ${agree}/${mine.length} (${ms}ms)`);
    if (!added) break;
    spec.examples.push(added.ex);
  }
  out.push({ task: name, rounds });
}
console.log(JSON.stringify(out, null, 2));

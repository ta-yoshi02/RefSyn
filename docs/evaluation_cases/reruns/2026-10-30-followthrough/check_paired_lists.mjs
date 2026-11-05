// Compare both methods on the same finite list inputs; require measured terms to reproduce.
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import path from "node:path";

const root = process.cwd();
const D = "docs/evaluation_cases/reruns/2026-10-30-baseline-recheck";
const load = async (dist) => {
  const imp = (p) => import(pathToFileURL(path.join(dist, p)).href);
  return { ...(await imp("components/component.js")), ...(await imp("components/user-friendly-json.js")), ...(await imp("synthesis/ascendrec/synthesizer.js")), ...(await imp("types/term.js")) };
};
const L = await load(path.join(root, "external/escher-ts/dist"));
const variants = {
  list: JSON.parse(readFileSync(`${D}/tasks_list.json`, "utf8")),
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

const output = [];
const measurementDir = "docs/evaluation_cases/reruns/2026-10-30-followthrough";
for (const method of ["setAt", "append", "prepend", "insert", "popBack"]) {
  const measurement = JSON.parse(readFileSync(`${measurementDir}/baseline_${method}.json`, "utf8"));
  const src = readFileSync(`docs/evaluation_cases/reruns/2026-10-14-fresh-recording/${method === "insert" ? "insert_general" : method}/source_after.js`, "utf8");
  const Node = new Function(`${src.slice(0, src.indexOf("\nlet lst"))}\nreturn Node;`)();
  for (const row of measurement.repetitions[0].tasks) {
    const task = variants.list.find(t => t.name === row.task);
    const { evaluationMethod: _, ...spec } = task;
    const job = L.prepareJsonSynthesisJob(L.parseJsonSynthesisSpec(JSON.stringify(spec)));
    let term = null;
    if (row.success) {
      term = new L.AscendRecSynthesizer({ maxCost: 32, timeoutMs: 15000, searchSizeFactor: 3, useReductionRules: true, onlyForwardSearch: false, argListCompare: L.anyArgSmaller })
        .synthesize(job.functionName, job.inputTypes, job.inputNames, job.returnType, job.env, job.examples);
      if (!term || L.showTerm(term.program.body) !== row.term) throw new Error(`search changed: ${row.task}`);
    }
    const checks = [];
    for (const c of cases.filter(c => c.method === method)) {
      const t = structuredClone(c.s);
      const ret = c.ref(t);
      const expected = encodeOutput(task, t, ret);
      let baseline = null;
      if (term) {
        const [[args, want]] = L.prepareJsonSynthesisJob(L.parseJsonSynthesisSpec(JSON.stringify({ ...spec, examples: [[encodeInput(task, c.s, c.args), expected]] }))).examples;
        const vars = new Map(job.inputNames.map((n, k) => [n, args[k]]));
        const actual = L.executeTerm(n => vars.get(n), job.env, term.program.body);
        baseline = { agrees: JSON.stringify(actual) === JSON.stringify(want), actual, expected: want };
      }
      const nodes = c.s.f.map(v => Object.assign(new Node(), { val: v }));
      nodes.forEach((n, k) => { n.next = nodes[c.s.g[k]] ?? null; });
      let mold;
      try {
        const returned = nodes[c.s.head][method](...c.args);
        if (returned && !nodes.includes(returned)) nodes.push(returned);
        for (let k = 0; k < nodes.length; k++) if (nodes[k].next && !nodes.includes(nodes[k].next)) nodes.push(nodes[k].next);
        const state = { f: nodes.map(n => n.val), g: nodes.map(n => n.next === null ? -1 : nodes.indexOf(n.next)) };
        const actual = encodeOutput(task, state, nodes.indexOf(returned));
        mold = { agrees: JSON.stringify(actual) === JSON.stringify(expected), actual, expected };
      } catch (e) { mold = { agrees: false, error: String(e) }; }
      checks.push({ input: { state: c.s, args: c.args }, baseline, mold });
    }
    output.push({ method, task: row.task, term: row.term, total: checks.length,
      baselineAgree: term ? checks.filter(c => c.baseline.agrees).length : null,
      moldAgree: checks.filter(c => c.mold.agrees).length, checks });
  }
}
console.log(JSON.stringify(output, null, 2));
if (output.some(r => r.moldAgree !== r.total)) process.exitCode = 1;

// Non-separated baseline for prepend and append on this directory's Kanon recordings, in the encoding
// of ../2026-11-04-environment-audit/tasks_minimal.json (objects in creation order; f = val heap,
// g = next heap; same components). Examples are obtained by replaying each call's recorded operations
// (<recording>/operations.json) on the program's initial lists; for prepend and append the first calls
// must reproduce the existing tasks' examples exactly (checked; append_older_tail has two more objects). Each task is then searched (current setting, 32 / 15 s, as
// rerun_minimal.mjs) and its term compared on the undemonstrated inputs of cegis_minimal.mjs.
// Usage (from the repository root): node <this> > list_baseline.json
import { readFileSync } from "node:fs";
import { isDeepStrictEqual } from "node:util";
import { pathToFileURL } from "node:url";
import path from "node:path";

const D = "docs/evaluation_cases/reruns/2026-11-05-boundary-demos";
const A = "docs/evaluation_cases/reruns/2026-11-04-environment-audit";
const imp = (p) => import(pathToFileURL(path.join(process.cwd(), "external/escher-ts/dist", p)).href);
const L = { ...(await imp("components/component.js")), ...(await imp("components/user-friendly-json.js")), ...(await imp("synthesis/ascendrec/synthesizer.js")), ...(await imp("types/term.js")) };
const minimal = JSON.parse(readFileSync(`${A}/tasks_minimal.json`, "utf8"));

// Replay: state = { f, g } indexed by creation order; ids main-new1, main-new2 are the initial nodes.
const replay = (dir, init) => {
  const calls = JSON.parse(readFileSync(`${D}/${dir}/operations.json`, "utf8"));
  const idx = new Map(init.f.map((_, i) => [`main-new${i + 1}`, i]));
  const st = { f: [...init.f], g: [...init.g] };
  const literal = new Map();
  const demos = [];
  for (const call of calls) {
    const pre = { f: [...st.f], g: [...st.g], head: idx.get(call.receiverObject) };
    let ret = null;
    for (const op of call.operations) {
      if (op.editType === "addNode") {
        if (op.isLiteral) literal.set(op.id, Number(op.label));
        else { idx.set(op.id, st.f.length); st.f.push(undefined); st.g.push(-1); }
      } else if (op.editType === "addEdge" && op.label === "val") st.f[idx.get(op.from)] = literal.get(op.to);
      else if (op.editType === "addEdge" && op.label === "next") st.g[idx.get(op.from)] = idx.get(op.to);
      else if (op.editType === "addVariable" && op.label === "return") ret = idx.get(op.to);
      else throw new Error(`unexpected operation ${JSON.stringify(op)}`);
    }
    demos.push({ pre, x: call.arguments[0], post: { f: [...st.f], g: [...st.g] }, ret });
  }
  return demos;
};

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

// Undemonstrated inputs, as in cegis_minimal.mjs.
const makeList = (n, reversed) => {
  const order = [...Array(n).keys()].map((k) => (reversed ? n - 1 - k : k));
  const f = Array(n), g = Array(n);
  order.forEach((i, k) => { f[i] = (k + 1) * 10 + 1; g[i] = k + 1 < n ? order[k + 1] : -1; });
  return { f, g, head: order[0], order };
};
const others = [];
for (let n = 1; n <= 6; n += 1) for (const reversed of [false, true]) for (const x of [0, 1, 2, 3, 99]) {
  others.push({ method: "append", s: makeList(n, reversed), args: [x], ref: (t) => { const m = t.f.length; t.f.push(x); t.g.push(-1); t.g[t.order[n - 1]] = m; return null; } });
  others.push({ method: "prepend", s: makeList(n, reversed), args: [x], ref: (t) => { const m = t.f.length; t.f.push(x); t.g.push(t.head); return m; } });
}

const out = [];
// [method, recording, initial objects in creation order, whether the first calls must equal tasks_minimal.json]
const two = { f: [39, 27], g: [1, -1] };
const recordings = [["prepend", "prepend", two, true], ["append", "append", two, true], ["append", "append_older_tail", { f: [39, 27, 6, 7], g: [1, -1, 3, -1] }, false]];
for (const [method, dir, init, sameStart] of recordings) {
  const demos = replay(dir, init);
  for (const task of minimal.filter((t) => t.evaluationMethod === method)) {
    const { evaluationMethod: _m, ...spec0 } = task;
    const examples = demos.map((d) => [encodeInput(task, d.pre, [d.x]), encodeOutput(task, d.post, d.ret)]);
    const old = task.examples.length;
    if (sameStart && !isDeepStrictEqual(examples.slice(0, old), task.examples)) throw new Error(`${task.name}: replayed examples differ from tasks_minimal.json`);
    const spec = { ...spec0, examples };
    const job = L.prepareJsonSynthesisJob(L.parseJsonSynthesisSpec(JSON.stringify(spec)));
    const t0 = Date.now();
    const r = new L.AscendRecSynthesizer({ maxCost: 32, timeoutMs: 15000, searchSizeFactor: 3, useReductionRules: true, onlyForwardSearch: false, argListCompare: L.anyArgSmaller })
      .synthesize(job.functionName, job.inputTypes, job.inputNames, job.returnType, job.env, job.examples);
    const ms = Date.now() - t0;
    const row = { recording: dir, task: task.name, examples: examples.length, old_examples: old, ms, term: r ? L.showTerm(r.program.body) : null };
    if (r) {
      const mine = others.filter((c) => c.method === method);
      let agree = 0;
      for (const c of mine) {
        const t = { f: [...c.s.f], g: [...c.s.g], head: c.s.head, order: c.s.order };
        const ret = c.ref(t);
        const [[args0, want]] = L.prepareJsonSynthesisJob(L.parseJsonSynthesisSpec(JSON.stringify({ ...spec, examples: [[encodeInput(task, c.s, c.args), encodeOutput(task, t, ret)]] }))).examples;
        const vars = new Map(job.inputNames.map((n, k) => [n, args0[k]]));
        if (JSON.stringify(L.executeTerm((n) => vars.get(n), job.env, r.program.body)) === JSON.stringify(want)) agree += 1;
      }
      Object.assign(row, { agree, total: mine.length });
    }
    console.error(`${dir} ${row.task}: ${row.term ?? "timeout"} ${row.agree ?? ""}/${row.total ?? ""} (${ms}ms)`);
    out.push(row);
  }
}
console.log(JSON.stringify(out, null, 2));

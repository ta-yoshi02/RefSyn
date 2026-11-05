// cegis_mold.mjs for union h0 only, with a wider checked domain: two roots with distinct ranks 0-7 (56
// pairs, both orders) instead of 0-3 (12), and up to 10 added examples. With 0-3 the loop stopped at a
// conditional on rank constants that is right there; this asks whether demonstrations with larger and
// non-adjacent ranks lead the current setting to the rank comparison. Current setting, server limits
// (20 / 2 s); the intended term is the candidate setting's (leInt on the two ranks).
// Usage (from the repository root): WEIGHTED_DIST=<patched dist> node <this> > cegis_union_wide.json
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import path from "node:path";

const root = process.cwd();
const R = "docs/evaluation_cases/reruns";
const load = async (dist) => {
  const imp = (p) => import(pathToFileURL(path.join(dist, p)).href);
  return { ...(await imp("components/component.js")), ...(await imp("components/user-friendly-json.js")), ...(await imp("synthesis/ascendrec/synthesizer.js")), ...(await imp("types/term.js")) };
};
const cur = await load(path.join(root, "external/escher-ts/dist"));
const cand = await load(process.env.WEIGHTED_DIST);
const candCosts = { last_ptr: 3, penultimateRef: 3, nthNextRef: 3, findByValueRef: 3 };
const read = (p) => JSON.parse(readFileSync(p, "utf8"));

const synth = (L, spec, factor, costs) => {
  globalThis.__refsynCompCost = costs;
  const { refsynMeta: _m, ...s } = spec;
  const job = L.prepareJsonSynthesisJob(L.parseJsonSynthesisSpec(JSON.stringify(s)));
  const t0 = Date.now();
  const r = new L.AscendRecSynthesizer({ maxCost: 20, timeoutMs: 2000, searchSizeFactor: factor, useReductionRules: true, onlyForwardSearch: false, argListCompare: L.anyArgSmaller })
    .synthesize(job.functionName, job.inputTypes, job.inputNames, job.returnType, job.env, job.examples);
  return { r, job, ms: Date.now() - t0 };
};
const evalTerm = (L, spec, job, term, rawInput) => {
  const { refsynMeta: _m, ...s } = spec;
  const [[args]] = L.prepareJsonSynthesisJob(L.parseJsonSynthesisSpec(JSON.stringify({ ...s, examples: [[rawInput, { ref: -1 }]] }))).examples;
  const vars = new Map(job.inputNames.map((n, k) => [n, args[k]]));
  return L.executeTerm((n) => vars.get(n), job.env, term);
};
const toRaw = (v) => (v && v.tag === "ref" ? { ref: v.value } : "error");

// Encoding as in MOLD's tasks: thisRef, nodeHeap, one heap per field, then the method's arguments.
const encode = (task, objs, receiver, args) => {
  const cls = task.classes[0];
  const sig = task.signature.autoExpandClassSignature;
  const fields = Object.keys(sig.fieldHeapNames);
  const isPtr = (f) => cls.fields[f].startsWith("Ref[Object");
  const nodeHeap = objs.map((o, i) => ({ object: { className: cls.name, fields: Object.fromEntries(fields.map((f) => [f, isPtr(f) ? { ref: o[f] ?? -1 } : { ref: i }])) } }));
  return [{ ref: receiver }, nodeHeap, ...fields.map((f) => objs.map((o) => (isPtr(f) ? { ref: o[f] ?? -1 } : o[f]))), ...args];
};
const tree = (shape) => {
  const objs = [];
  const mk = (s) => { if (!s) return null; const i = objs.length; objs.push({ val: i * 10, left: null, right: null }); objs[i].left = mk(s.l); objs[i].right = mk(s.r); return i; };
  mk(shape);
  return objs;
};
const variants = [null, {}, { r: {} }];
const mirror = (s) => (s ? { l: mirror(s.r), r: mirror(s.l) } : null);
const rotShapes = variants.flatMap((tl) => variants.flatMap((rr) => variants.flatMap((a) => variants.map((b) => ({ l: tl, r: { l: { l: a, r: b }, r: rr } })))));
const domains = {
  rotateLeft: rotShapes.map((s) => ({ objs: tree(s), recv: 0, args: [] })),
  rotateRight: rotShapes.map((s) => ({ objs: tree(mirror(s)), recv: 0, args: [] })),
  union: [0, 1, 2, 3, 4, 5, 6, 7].flatMap((r1) => [0, 1, 2, 3, 4, 5, 6, 7].filter((r2) => r2 !== r1).map((r2) => ({ objs: [{ parent: null, rank: r1 }, { parent: null, rank: r2 }], recv: 0, args: [{ ref: 1 }] }))),
};
const targets = [
  ["union", `${R}/2026-10-30-uf-union/union`, "union", 0],
];

const out = [];
for (const [name, dir, domain, hole] of targets) {
  const tasks = read(`${dir}/mold_escher_tasks.json`);
  // Intended terms of holes 0..hole, from the candidate setting.
  const intended = tasks.slice(0, hole + 1).map((t) => { const { r, job } = synth(cand, t, 1, candCosts); return { term: r.program.body, job, shown: cand.showTerm(r.program.body) }; });
  // Inputs of the target hole: base state plus earlier holes' intended values.
  const inputs = domains[domain].map(({ objs, recv, args }) => {
    let raw = encode(tasks[0], objs, recv, args);
    for (let k = 0; k < hole; k += 1) raw = [...raw, toRaw(evalTerm(cand, tasks[k], intended[k].job, intended[k].term, raw))];
    return { n: objs.length, raw };
  }).sort((a, b) => a.n - b.n);
  const task = tasks[hole];
  const spec = { ...task, examples: [...task.examples] };
  const rounds = [];
  for (let round = 0; round <= 10; round += 1) {
    const { r, job, ms } = synth(cur, spec, 3, undefined);
    if (!r) { rounds.push({ examples: spec.examples.length, ms, term: null }); break; }
    let agree = 0;
    let firstDiff = null;
    for (const inp of inputs) {
      const want = toRaw(evalTerm(cand, task, intended[hole].job, intended[hole].term, inp.raw));
      const got = toRaw(evalTerm(cur, task, job, r.program.body, inp.raw));
      if (JSON.stringify(want) === JSON.stringify(got)) agree += 1;
      else if (!firstDiff) firstDiff = { inp, want };
    }
    rounds.push({ examples: spec.examples.length, ms, term: cur.showTerm(r.program.body), agree_with_intended: agree, total: inputs.length, added_objects: firstDiff?.inp.n ?? null });
    console.error(`${name} h${hole} round ${round}: ${cur.showTerm(r.program.body)} ${agree}/${inputs.length} (${ms}ms)`);
    if (!firstDiff) break;
    spec.examples.push([firstDiff.inp.raw, firstDiff.want]);
  }
  out.push({ case: name, helper: task.name, intended: intended[hole].shown, rounds });
}
console.log(JSON.stringify(out, null, 2));

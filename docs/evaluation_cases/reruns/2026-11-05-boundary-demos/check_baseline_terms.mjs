// Copy of ../2026-10-30-baseline/check_terms.mjs with only the recordings and file paths changed to this
// directory's recordings (2026-11-05-boundary-demos).
// For every baseline task that a setting solved (results.json), repeats that search to obtain the term,
// evaluates it with escher-ts' own semantics on inputs that were not demonstrated, and compares it
// with the reference method's post-state in the same encoding.
// Inputs: unlink and insertAfter on lists of length 2-7 (middle / non-tail positions); swapChildren on
// every node with two children of complete trees of height 1-3; rotateLeft at the root of trees where
// this.right.left exists, the four other subtrees each absent, a leaf, or a node with a right child, and
// rotateRight on their mirror images; union on two childless roots with ranks 0-3, r1 != r2.
// Usage (from the repository root): WEIGHTED_DIST=<patched dist> [RESULTS=results_list.json] node <this> > <out.json>
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import path from "node:path";

const root = process.cwd();
const D = "docs/evaluation_cases/reruns/2026-11-05-boundary-demos";
const load = async (dist) => {
  const imp = (p) => import(pathToFileURL(path.join(dist, p)).href);
  return { ...(await imp("components/component.js")), ...(await imp("components/user-friendly-json.js")), ...(await imp("synthesis/ascendrec/synthesizer.js")), ...(await imp("types/term.js")) };
};
const libs = { current: await load(path.join(root, "external/escher-ts/dist")), candidate: await load(process.env.WEIGHTED_DIST) };
const costs = { current: undefined, candidate: { last_ptr: 3, penultimateRef: 3, nthNextRef: 3, findByValueRef: 3 } };
const factor = { current: 3, candidate: 1 };
const limits = { adapter: { maxCost: 20, timeoutMs: 2000 }, paper: { maxCost: 32, timeoutMs: 15000 } };
const tasks = JSON.parse(readFileSync(`${D}/baseline_tasks.json`, "utf8"));
const results = JSON.parse(readFileSync(`${D}/${process.env.RESULTS ?? "results.json"}`, "utf8"));

const isPtr = (cls, f) => cls.fields[f].startsWith("Ref[Object");
const heapOf = (cls, objs, f) => objs.map((o) => (isPtr(cls, f) ? { ref: o[f] ?? -1 } : o[f] ?? { error: true }));
const encode = (task, objs, receiver, args) => {
  const cls = task.classes[0];
  const fields = Object.keys(cls.fields);
  const nodeHeap = objs.map((o, i) => ({ object: { className: cls.name, fields: Object.fromEntries(fields.map((f) => [f, isPtr(cls, f) ? { ref: o[f] ?? -1 } : { ref: i }])) } }));
  return [{ ref: receiver }, nodeHeap, ...fields.map((f) => heapOf(cls, objs, f)), ...args];
};
const clone = (objs) => objs.map((o) => ({ ...o }));
const tree = (shape) => {
  const objs = [];
  const mk = (s) => { if (!s) return null; const i = objs.length; objs.push({ val: i * 10, left: null, right: null }); objs[i].left = mk(s.l); objs[i].right = mk(s.r); return i; };
  mk(shape);
  return objs;
};
const list = (n) => Array.from({ length: n }, (_, i) => ({ val: (i + 1) * 10, prev: i > 0 ? i - 1 : null, next: i < n - 1 ? i + 1 : null }));
const complete = (d) => (d === 0 ? null : { l: complete(d - 1), r: complete(d - 1) });
const variants = [null, {}, { r: {} }];
const mirror = (s) => (s ? { l: mirror(s.r), r: mirror(s.l) } : null);
const rotShapes = variants.flatMap((tl) => variants.flatMap((rr) => variants.flatMap((a) => variants.map((b) => ({ l: tl, r: { l: { l: a, r: b }, r: rr } })))));

// [objs, receiver index, method args (encoded), reference(objs) -> {objs, ret}]
const inputs = {
  unlink: [3, 4, 5, 6, 7].flatMap((n) => [...Array(n - 2).keys()].map((k) => [list(n), k + 1, [], (o) => { const t = o[k + 1]; o[t.prev].next = t.next; o[t.next].prev = t.prev; return { objs: o, ret: null }; }])),
  insertAfter: [2, 3, 4, 5, 6, 7].flatMap((n) => [...Array(n - 1).keys()].map((p) => [list(n), p, [99], (o) => { const t = o[p]; const m = o.length; o.push({ val: 99, prev: p, next: t.next }); o[t.next].prev = m; t.next = m; return { objs: o, ret: null }; }])),
  swapChildren: [2, 3, 4].flatMap((h) => tree(complete(h)).map((o, i) => [o, i]).filter(([o]) => o.left !== null && o.right !== null).map(([, i]) => [tree(complete(h)), i, [], (s) => { [s[i].left, s[i].right] = [s[i].right, s[i].left]; return { objs: s, ret: null }; }])),
  rotateLeft: rotShapes.map((sh) => [tree(sh), 0, [], (s) => { const r = s[0].right; s[0].right = s[r].left; s[r].left = 0; return { objs: s, ret: r }; }]),
  rotateRight3: rotShapes.map((sh) => [tree(mirror(sh)), 0, [], (s) => { const l = s[0].left; s[0].left = s[l].right; s[l].right = 0; return { objs: s, ret: l }; }]),
  union: [0, 1, 2, 3].flatMap((r1) => [0, 1, 2, 3].filter((r2) => r2 !== r1).map((r2) => [[{ parent: null, rank: r1 }, { parent: null, rank: r2 }], 0, [{ ref: 1 }], (s) => { if (s[0].rank < s[1].rank) s[0].parent = 1; else s[1].parent = 0; return { objs: s, ret: null }; }])),
};
inputs.rotateRight = inputs.rotateRight3;

const out = [];
for (const row of results.filter((r) => r.term !== null)) {
  const task = tasks.find((t) => t.name === row.task);
  const { case: _c, output, ...spec } = task;
  const L = libs[row.setting];
  globalThis.__refsynCompCost = costs[row.setting];
  const job = L.prepareJsonSynthesisJob(L.parseJsonSynthesisSpec(JSON.stringify(spec)));
  const r = new L.AscendRecSynthesizer({ ...limits[row.limits], searchSizeFactor: factor[row.setting], useReductionRules: true, onlyForwardSearch: false, argListCompare: L.anyArgSmaller })
    .synthesize(job.functionName, job.inputTypes, job.inputNames, job.returnType, job.env, job.examples);
  if (!r || L.showTerm(r.program.body) !== row.term) { out.push({ task: row.task, setting: row.setting, limits: row.limits, error: "search did not repeat" }); continue; }
  const cls = task.classes[0];
  let agree = 0;
  for (const [objs, recv, args, reference] of inputs[row.case]) {
    const enc = encode(task, objs, recv, args);
    const exp = reference(clone(objs));
    const expected = output === "returnRef" ? { ref: exp.ret }
      : output.startsWith("pair(") ? { pair: output.slice(5, -1).split(",").map((f) => heapOf(cls, exp.objs, f)) }
      : heapOf(cls, exp.objs, output.replace(/Heap$/, ""));
    const [[args0, want]] = L.prepareJsonSynthesisJob(L.parseJsonSynthesisSpec(JSON.stringify({ ...spec, examples: [[enc, expected]] }))).examples;
    const vars = new Map(job.inputNames.map((n, k) => [n, args0[k]]));
    if (JSON.stringify(L.executeTerm((n) => vars.get(n), job.env, r.program.body)) === JSON.stringify(want)) agree += 1;
  }
  out.push({ task: row.task, setting: row.setting, limits: row.limits, term: row.term, agree, total: inputs[row.case].length });
}
console.log(JSON.stringify(out, null, 2));

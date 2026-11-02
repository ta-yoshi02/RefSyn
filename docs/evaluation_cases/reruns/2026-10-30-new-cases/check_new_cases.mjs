// Runs the generated methods of the three new cases on inputs that were not demonstrated (plan.md) and
// compares every field and the return value with a reference.
//   current:   the class applied by Kanon (<case>/source_after.js)
//   candidate: the same class with each helper replaced by the candidate setting's term, searched on
//              the saved task and translated by the adapter's own compileRefsynMethod
// Usage (from the repository root): WEIGHTED_DIST=<patched dist> node <this> > <out.json>
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import path from "node:path";

const root = process.cwd();
const D = "docs/evaluation_cases/reruns/2026-10-30-new-cases";
const imp = (p) => import(pathToFileURL(path.join(root, p)).href);
const { compileRefsynMethod } = await imp("runtime/refsyn-escher-adapter.mjs");
const { parseJsonSynthesisSpec, prepareJsonSynthesisJob } = await imp("external/escher-ts/dist/components/user-friendly-json.js");
const W = (p) => import(pathToFileURL(path.join(process.env.WEIGHTED_DIST, p)).href);
const { anyArgSmaller } = await W("components/component.js");
const wJson = await W("components/user-friendly-json.js");
const { AscendRecSynthesizer } = await W("synthesis/ascendrec/synthesizer.js");
const { showTerm } = await W("types/term.js");
globalThis.__refsynCompCost = { last_ptr: 3, penultimateRef: 3, nthNextRef: 3, findByValueRef: 3 };

const classOf = (src, className) => new Function(`${src.slice(0, src.search(/\nlet (root|head) /))}\nreturn ${className};`)();

const candidateSource = (name) => {
  let src = readFileSync(`${D}/${name}/source_after.js`, "utf8");
  const terms = {};
  for (const task of JSON.parse(readFileSync(`${D}/${name}/mold_escher_tasks.json`, "utf8"))) {
    const wjob = wJson.prepareJsonSynthesisJob(wJson.parseJsonSynthesisSpec(JSON.stringify(task)));
    const r = new AscendRecSynthesizer({ maxCost: 20, timeoutMs: 2000, searchSizeFactor: 1, useReductionRules: true, onlyForwardSearch: false, argListCompare: anyArgSmaller })
      .synthesize(wjob.functionName, wjob.inputTypes, wjob.inputNames, wjob.returnType, wjob.env, wjob.examples);
    const job = prepareJsonSynthesisJob(parseJsonSynthesisSpec(JSON.stringify(task)));
    const code = compileRefsynMethod(task, job, r.program.body);
    terms[task.name] = showTerm(r.program.body);
    src = src.replace(new RegExp(`^    ${task.name}\\(.*$`, "m"), `    ${code}`);
  }
  return { src, terms };
};

// Structures as plain descriptions; build() makes objects of the class under test.
const runCase = (Cls, build, call, reference) => {
  const a = build(Cls);
  let ret, error = null;
  try {
    ret = call(a);
  } catch (e) {
    error = String(e);
  }
  const b = build(Cls);
  const expectedRet = reference(b);
  const snap = (s, r) => {
    const all = [...s.nodes];
    for (const n of s.nodes) for (const f of ["left", "right", "prev", "next"]) if (n[f] && !all.includes(n[f])) all.push(n[f]);
    const id = (n) => (n === null || n === undefined ? null : all.indexOf(n));
    return { fields: all.map((n) => ({ val: n.val, left: id(n.left), right: id(n.right), prev: id(n.prev), next: id(n.next) })), ret: id(r) };
  };
  if (error) return { agrees: false, error };
  return { agrees: JSON.stringify(snap(a, ret)) === JSON.stringify(snap(b, expectedRet)) };
};

// swapChildren: complete trees of height 1-3 and the recorded tree, every node with two children.
const treeCases = () => {
  const shapes = [];
  const complete = (h) => (h === 0 ? null : { l: complete(h - 1), r: complete(h - 1) });
  for (const h of [2, 3, 4]) shapes.push(complete(h));
  shapes.push({ l: { l: {}, r: {} }, r: { l: {}, r: { r: {} } } });
  return shapes;
};
const buildTree = (shape) => (Cls) => {
  const nodes = [];
  const mk = (s) => {
    if (!s) return null;
    const n = new Cls();
    n.val = nodes.length * 10;
    nodes.push(n);
    n.left = mk(s.l);
    n.right = mk(s.r);
    return n;
  };
  mk(shape);
  return { nodes };
};

const swapInputs = [];
for (const shape of treeCases()) {
  const probe = buildTree(shape)(class { constructor() { this.left = null; this.right = null; } });
  probe.nodes.forEach((n, i) => { if (n.left && n.right) swapInputs.push({ shape, at: i }); });
}

// insertAfter: lists of length 2-7, every non-tail position.
const buildList = (len) => (Cls) => {
  const nodes = Array.from({ length: len }, (_, i) => { const n = new Cls(); n.val = (i + 1) * 10; return n; });
  nodes.forEach((n, i) => { if (i > 0) n.prev = nodes[i - 1]; if (i < len - 1) n.next = nodes[i + 1]; });
  return { nodes };
};
const insertInputs = [];
for (let len = 2; len <= 7; len += 1) for (let at = 0; at < len - 1; at += 1) insertInputs.push({ len, at });

// rotateLeft: this.right = r and r.left = rl exist; vary this.left, r.right, rl.left, rl.right
// (each absent, a leaf, or a node with a right child) and a right chain below rl.
const variants = [null, {}, { r: {} }];
const rotateInputs = [];
for (const tl of variants) for (const rr of variants) for (const rll of variants) for (const rlr of variants) {
  rotateInputs.push({ l: tl, r: { l: { l: rll, r: rlr }, r: rr } });
}

const results = {};
for (const setting of ["current", "candidate"]) {
  const out = {};
  const srcs = {};
  const terms = {};
  for (const name of ["swapChildren", "insertAfter", "rotateLeft"]) {
    if (setting === "current") srcs[name] = readFileSync(`${D}/${name}/source_after.js`, "utf8");
    else ({ src: srcs[name], terms: terms[name] } = candidateSource(name));
  }
  const T1 = classOf(srcs.swapChildren, "TreeNode");
  out.swapChildren = swapInputs.map(({ shape, at }) => runCase(T1, buildTree(shape), (s) => s.nodes[at].swapChildren(), (s) => { const n = s.nodes[at]; [n.left, n.right] = [n.right, n.left]; }));
  const N = classOf(srcs.insertAfter, "Node");
  out.insertAfter = insertInputs.map(({ len, at }) => {
    const r = runCase(N, buildList(len), (s) => s.nodes[at].insertAfter(99), (s) => {
      const t = s.nodes[at], n = new N(); n.val = 99; n.prev = t; n.next = t.next; t.next.prev = n; t.next = n;
    });
    return { len, at, ...r };
  });
  const T2 = classOf(srcs.rotateLeft, "TreeNode");
  out.rotateLeft = rotateInputs.map((shape) => runCase(T2, buildTree(shape), (s) => s.nodes[0].rotateLeft(), (s) => {
    const t = s.nodes[0], r = t.right; t.right = r.left; r.left = t; return r;
  }));
  results[setting] = {
    terms: setting === "candidate" ? terms : undefined,
    summary: Object.fromEntries(Object.entries(out).map(([k, v]) => [k, `${v.filter((x) => x.agrees).length}/${v.length}`])),
    cases: out,
  };
}
console.log(JSON.stringify(results, null, 2));

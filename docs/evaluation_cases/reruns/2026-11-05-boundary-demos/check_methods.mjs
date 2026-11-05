// Runs the methods Kanon applied (<case>/source_after.js, current setting) on inputs that were not
// demonstrated and compares every field and the return value with a reference, on separate copies.
// Inputs as in the earlier checks: rotateLeft at the root of trees where this.right.left exists, with
// this.left, this.right.right, this.right.left.left and this.right.left.right each absent, a leaf, or a
// node with a right child (81), rotateRight on their mirror images (81); prepend and append on lists
// of length 1-6 (30 with x in {0, 1, 2, 3, 99}; append for both append recordings); insertAfter at every non-tail position of lists of
// length 2-7 with the same five x (105).
// Usage (from the repository root): node <this> > methods.json
import { readFileSync } from "node:fs";

const D = "docs/evaluation_cases/reruns/2026-11-05-boundary-demos";
const classOf = (name, className) => {
  const src = readFileSync(`${D}/${name}/source_after.js`, "utf8");
  return new Function(`${src.slice(0, src.search(/\nlet /))}\nreturn ${className};`)();
};

const runCase = (Cls, build, call, reference) => {
  const a = build(Cls);
  let ret, error = null;
  try { ret = call(a); } catch (e) { error = String(e); }
  const b = build(Cls);
  const expectedRet = reference(b, Cls);
  const snap = (s, r) => {
    const all = [...s.nodes];
    const add = (n) => { if (n && !all.includes(n)) { all.push(n); for (const f of ["left", "right", "prev", "next"]) add(n[f]); } };
    for (const n of s.nodes) for (const f of ["left", "right", "prev", "next"]) add(n[f]);
    add(r);
    const id = (n) => (n === null || n === undefined ? null : all.indexOf(n));
    return { fields: all.map((n) => ({ val: n.val, left: id(n.left), right: id(n.right), prev: id(n.prev), next: id(n.next) })), ret: id(r) };
  };
  if (error) return { agrees: false, error };
  return { agrees: JSON.stringify(snap(a, ret)) === JSON.stringify(snap(b, expectedRet)) };
};

const buildTree = (shape) => (Cls) => {
  const nodes = [];
  const mk = (s) => { if (!s) return null; const n = new Cls(); n.val = nodes.length * 10; nodes.push(n); n.left = mk(s.l); n.right = mk(s.r); return n; };
  mk(shape);
  return { nodes };
};
const variants = [null, {}, { r: {} }];
const rotShapes = variants.flatMap((tl) => variants.flatMap((rr) => variants.flatMap((a) => variants.map((b) => ({ l: tl, r: { l: { l: a, r: b }, r: rr } })))));
const mirror = (s) => (s ? { l: mirror(s.r), r: mirror(s.l) } : null);

const buildList = (len, dll) => (Cls) => {
  const nodes = Array.from({ length: len }, (_, i) => { const n = new Cls(); n.val = (i + 1) * 10; return n; });
  nodes.forEach((n, i) => { if (dll && i > 0) n.prev = nodes[i - 1]; if (i < len - 1) n.next = nodes[i + 1]; });
  return { nodes };
};
const XS = [0, 1, 2, 3, 99];

const out = {};
const T1 = classOf("rotateLeft", "TreeNode");
out.rotateLeft = rotShapes.map((sh) => runCase(T1, buildTree(sh), (s) => s.nodes[0].rotateLeft(), (s) => { const t = s.nodes[0], r = t.right; t.right = r.left; r.left = t; return r; }));
const T2 = classOf("rotateRight", "TreeNode");
out.rotateRight = rotShapes.map((sh) => runCase(T2, buildTree(mirror(sh)), (s) => s.nodes[0].rotateRight(), (s) => { const t = s.nodes[0], l = t.left; t.left = l.right; l.right = t; return l; }));
const P = classOf("prepend", "Node");
out.prepend = [1, 2, 3, 4, 5, 6].flatMap((len) => XS.map((x) => ({ len, x, ...runCase(P, buildList(len, false), (s) => s.nodes[0].prepend(x), (s, C) => { const n = new C(); n.val = x; n.next = s.nodes[0]; return n; }) })));
const A = classOf("append", "Node");
out.append = [1, 2, 3, 4, 5, 6].flatMap((len) => XS.map((x) => ({ len, x, ...runCase(A, buildList(len, false), (s) => s.nodes[0].append(x), (s, C) => { const n = new C(); n.val = x; s.nodes[len - 1].next = n; }) })));
const A2 = classOf("append_older_tail", "Node");
out.append_older_tail = [1, 2, 3, 4, 5, 6].flatMap((len) => XS.map((x) => ({ len, x, ...runCase(A2, buildList(len, false), (s) => s.nodes[0].append(x), (s, C) => { const n = new C(); n.val = x; s.nodes[len - 1].next = n; }) })));
const I = classOf("insertAfter", "Node");
out.insertAfter = [2, 3, 4, 5, 6, 7].flatMap((len) => [...Array(len - 1).keys()].flatMap((at) => XS.map((x) => ({ len, at, x, ...runCase(I, buildList(len, true), (s) => s.nodes[at].insertAfter(x), (s, C) => { const t = s.nodes[at], n = new C(); n.val = x; n.prev = t; n.next = t.next; t.next.prev = n; t.next = n; }) }))));

console.log(JSON.stringify({ summary: Object.fromEntries(Object.entries(out).map(([k, v]) => [k, `${v.filter((r) => r.agrees).length}/${v.length}`])), cases: out }, null, 2));

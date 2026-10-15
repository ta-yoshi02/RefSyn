// Re-runs the applied rotateRight outside Kanon (Node vm) and checks every reference.
// Part A replays the three demonstrated calls on a fresh copy of the demonstrated tree and
// checks, per call: the returned object is the receiver's original left child, the
// receiver's left is the original left.right, the returned object's right is the receiver,
// and no other reference changed. It also compares the final tree with the expected one.
// Part B (diagnostic, not a demonstration) runs the same method and a reference rotation on
// a tree whose left.right has a right child.
// Usage: node <this> <case-dir>   (writes <case-dir>/node_rerun_result.json)
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import vm from "node:vm";

const caseDir = process.argv[2];
const source = readFileSync(join(caseDir, "source_after.js"), "utf8");
const classSource = source.slice(0, source.indexOf("\nlet root"));

const load = () => {
  const ctx = {};
  vm.createContext(ctx);
  vm.runInContext(classSource + "\n;globalThis.TreeNode = TreeNode;", ctx);
  return ctx.TreeNode;
};
// Builds a tree from [val, left, right] (null for an empty child).
const build = (TreeNode, spec) => {
  if (spec === null) return null;
  const n = new TreeNode();
  n.val = spec[0];
  n.left = build(TreeNode, spec[1]);
  n.right = build(TreeNode, spec[2]);
  return n;
};
const shape = (n) => (n === null ? null : [n.val, shape(n.left), shape(n.right)]);
const nodes = (n, acc = []) => (n === null ? acc : (acc.push(n), nodes(n.left, acc), nodes(n.right, acc)));
// References of a fixed object list, so an object the method detaches is still compared.
const edges = (list) => new Map(list.flatMap((n) => [[`${n.val}.left`, n.left], [`${n.val}.right`, n.right]]));

const initial = [50, [30, [20, null, null], [40, [35, null, [37, null, null]], null]], [70, [60, null, [65, null, null]], [80, null, null]]];
const calls = [
  ["root", (r) => r, (r, v) => v],
  ["root.right.right", (r) => r.right.right, (r, v) => { r.right.right = v; return r; }],
  ["root.right.left", (r) => r.right.left, (r, v) => { r.right.left = v; return r; }],
];

const TreeNode = load();
let root = build(TreeNode, initial);
const steps = [];
for (const [where, get, set] of calls) {
  const self = get(root);
  const l = self.left;
  const lr = l.right;
  const keep = nodes(root);
  const before = edges(keep);
  const ret = self.rotateRight();
  // Compared before the caller's own assignment (e.g. root.right.right = ...), which is not
  // part of the method.
  const after = edges(keep);
  const changed = [...before.keys()].filter((k) => before.get(k) !== after.get(k)).sort();
  const checks = {
    returns_original_left: ret === l,
    receiver_left_is_original_left_right: self.left === lr,
    returned_right_is_receiver: ret.right === self,
    only_expected_references_changed:
      JSON.stringify(changed) === JSON.stringify([`${self.val}.left`, `${l.val}.right`].sort()),
  };
  root = set(root, ret);
  checks.same_objects_reachable = nodes(root).length === keep.length && keep.every((n) => nodes(root).includes(n));
  steps.push({ call: `${where} = ${where}.rotateRight()`, receiver: self.val, changed, checks });
}
const expectedFinal = [30, [20, null, null], [50, [35, null, [40, [37, null, null], null]], [60, null, [70, [65, null, null], [80, null, null]]]]];
const partA = {
  steps,
  final: shape(root),
  expected_final: expectedFinal,
  final_matches: JSON.stringify(shape(root)) === JSON.stringify(expectedFinal),
  all_checks_pass: steps.every((s) => Object.values(s.checks).every(Boolean)),
};

// Part B: left.right (40) has a right child (45); not a demonstrated input.
const probe = [50, [30, [20, null, null], [40, null, [45, null, null]]], [70, null, null]];
const T2 = load();
const r2 = build(T2, probe);
const got = shape(r2.rotateRight());
const reference = (s) => [s[1][0], s[1][1], [s[0], s[1][2], s[2]]];
const partB = {
  input: probe,
  generated_result: got,
  reference_rotation: reference(probe),
  agrees: JSON.stringify(got) === JSON.stringify(reference(probe)),
};

const out = { method: "node vm re-execution of source_after.js (independent of Kanon)", node: process.version, demonstrated_calls: partA, outside_the_demonstrations: partB };
writeFileSync(join(caseDir, "node_rerun_result.json"), JSON.stringify(out, null, 2) + "\n");
console.log(JSON.stringify({ A: { all_checks_pass: partA.all_checks_pass, final_matches: partA.final_matches }, B: partB }, null, 1));

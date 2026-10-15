// Re-runs the applied rotateRight outside Kanon (Node vm) and checks every reference.
// Part A replays the four demonstrated calls of rotateRight/source_before.js on a fresh copy
// of its tree and checks, per call: the returned object is the receiver's original left
// child, the receiver's left is the original left.right, the returned object's right is the
// receiver, and no other reference changed. It also compares the final tree.
// Part B (diagnostic, not demonstrations) runs the same method and a reference rotation on
// several trees that are not demonstrated inputs.
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
// A node met again on the same path is written as "cycle at <val>", so a result with a cycle
// is shown instead of overflowing the stack.
const shape = (n, path = new Set()) => {
  if (n === null) return null;
  if (path.has(n)) return `cycle at ${n.val}`;
  const inner = new Set(path).add(n);
  return [n.val, shape(n.left, inner), shape(n.right, inner)];
};
const nodes = (n, acc = []) => (n === null ? acc : (acc.push(n), nodes(n.left, acc), nodes(n.right, acc)));
// References of a fixed object list, so an object the method detaches is still compared.
const edges = (list) => new Map(list.flatMap((n) => [[`${n.val}.left`, n.left], [`${n.val}.right`, n.right]]));

const TreeNode = load();
const initial = [50, [30, [20, null, null], [40, [35, null, [37, null, null]], null]], [70, [60, null, [65, null, null]], [80, null, null]]];
const node = (val) => Object.assign(new TreeNode(), { val });
// [label, before (the program's statements preceding the call), receiver, caller's assignment]
const calls = [
  ["root = root.rotateRight()", () => {}, (r) => r, (r, v) => v],
  ["root.right.right = root.right.right.rotateRight()", () => {}, (r) => r.right.right, (r, v) => ((r.right.right = v), r)],
  ["root.right.left = root.right.left.rotateRight()", () => {}, (r) => r.right.left, (r, v) => ((r.right.left = v), r)],
  ["root = root.rotateRight()  (after adding 25 and 27 under 20)", (r) => { r.left.right = node(25); r.left.right.right = node(27); }, (r) => r, (r, v) => v],
];

let root = build(TreeNode, initial);
const steps = [];
for (const [call, before, get, set] of calls) {
  before(root);
  const self = get(root);
  const l = self.left;
  const lr = l.right;
  const keep = nodes(root);
  const prior = edges(keep);
  const ret = self.rotateRight();
  // Compared before the caller's own assignment, which is not part of the method.
  const after = edges(keep);
  const changed = [...prior.keys()].filter((k) => prior.get(k) !== after.get(k)).sort();
  const checks = {
    returns_original_left: ret === l,
    receiver_left_is_original_left_right: self.left === lr,
    returned_right_is_receiver: ret.right === self,
    only_expected_references_changed:
      JSON.stringify(changed) === JSON.stringify([`${self.val}.left`, `${l.val}.right`].sort()),
  };
  root = set(root, ret);
  checks.same_objects_reachable = nodes(root).length === keep.length && keep.every((n) => nodes(root).includes(n));
  steps.push({ call, receiver: self.val, changed, checks });
}
const expectedFinal = [20, null, [30, [25, null, [27, null, null]], [50, [35, null, [40, [37, null, null], null]], [60, null, [70, [65, null, null], [80, null, null]]]]]];
const partA = {
  steps,
  final: shape(root),
  expected_final: expectedFinal,
  final_matches: JSON.stringify(shape(root)) === JSON.stringify(expectedFinal),
  all_checks_pass: steps.every((s) => Object.values(s.checks).every(Boolean)),
};

const probes = [
  ["left.right has a right child; left has a left child", [50, [30, [20, null, null], [40, null, [45, null, null]]], [70, null, null]]],
  ["left.right has a right child; left has no left child", [50, [30, null, [40, null, [45, null, null]]], [70, null, null]]],
  ["left.right has no right child; left chain of length 3", [50, [30, [20, [10, null, null], null], [40, null, null]], [70, null, null]]],
  ["left.right has two right descendants", [50, [30, [20, null, null], [40, [35, null, null], [45, null, [47, null, null]]]], null]],
  ["left.right is null", [50, [30, [20, null, null], null], [70, null, null]]],
];
const reference = (s) => [s[1][0], s[1][1], [s[0], s[1][2], s[2]]];
const partB = probes.map(([what, input]) => {
  let got;
  try {
    got = shape(build(load(), input).rotateRight());
  } catch (e) {
    got = `throws ${e.constructor.name}: ${e.message}`;
  }
  return { what, input, generated_result: got, reference_rotation: reference(input), agrees: JSON.stringify(got) === JSON.stringify(reference(input)) };
});

const out = { method: "node vm re-execution of source_after.js (independent of Kanon)", node: process.version, demonstrated_calls: partA, outside_the_demonstrations: partB };
writeFileSync(join(caseDir, "node_rerun_result.json"), JSON.stringify(out, null, 2) + "\n");
console.log(JSON.stringify({ all_checks_pass: partA.all_checks_pass, final_matches: partA.final_matches }));
for (const s of steps) console.log("  ", s.call, "|", s.changed.join(","), JSON.stringify(s.checks));
for (const b of partB) console.log(b.agrees ? "agrees  " : "DIFFERS ", b.what, JSON.stringify(b.generated_result), "vs", JSON.stringify(b.reference_rotation));

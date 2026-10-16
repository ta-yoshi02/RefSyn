// Right rotation at the receiver, on trees where this.left and this.left.right are non-null,
// the condition every demonstration met. Trees are written val(left, right).
export const classSource = `class TreeNode {
  constructor() { this.left = null; this.right = null; }
  rotateRight() {
    const l = this.left;
    this.left = l.right;
    l.right = this;
    return l;
  }
}`;
const tree = (TreeNode, shape) => {
  if (shape === null) return null;
  const [val, left = null, right = null] = shape;
  const node = new TreeNode();
  node.val = val;
  node.left = tree(TreeNode, left);
  node.right = tree(TreeNode, right);
  return node;
};
const shapes = {
  "50(30(20, 40(-, 45)), 70)": [50, [30, [20], [40, null, [45]]], [70]],
  "50(30(-, 40(-, 45)), 70)": [50, [30, null, [40, null, [45]]], [70]],
  "50(30(20(10, -), 40), 70)": [50, [30, [20, [10]], [40]], [70]],
  "50(30(20, 40(35, 45(-, 47))), -)": [50, [30, [20], [40, [35], [45, null, [47]]]]],
  "50(30(20, 40), 70)": [50, [30, [20], [40]], [70]],
  "50(30(20(10, 25), 40(35, -)), 70)": [50, [30, [20, [10], [25]], [40, [35]]], [70]],
};
export const cases = Object.entries(shapes).flatMap(([text, shape]) => [
  { name: `root ${text}`, build: ({ TreeNode }) => { const root = tree(TreeNode, shape); return { receiver: root, args: [], roots: { root } }; } },
  { name: `left child of 90 in 90(${text}, -)`, build: ({ TreeNode }) => {
    const parent = tree(TreeNode, [90, shape]);
    return { receiver: parent.left, args: [], roots: { parent } };
  } },
]);

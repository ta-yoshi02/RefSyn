class TreeNode {
    constructor() {
        this.left = null;
        this.right = null;
    }

    swapChildren() {
    }
}

let root = new TreeNode();
root.val = 50;
root.left = new TreeNode();
root.left.val = 30;
root.left.left = new TreeNode();
root.left.left.val = 20;
root.left.right = new TreeNode();
root.left.right.val = 40;
root.right = new TreeNode();
root.right.val = 70;
root.right.left = new TreeNode();
root.right.left.val = 60;
root.right.right = new TreeNode();
root.right.right.val = 80;
root.right.right.right = new TreeNode();
root.right.right.right.val = 90;
root.swapChildren();
root.left.swapChildren();
root.right.swapChildren();

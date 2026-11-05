class TreeNode {
    constructor() {
        this.left = null;
        this.right = null;
    }

    rotateLeft() {
    }
}

let root = new TreeNode();
root.val = 50;
root.left = new TreeNode();
root.left.val = 30;
root.left.right = new TreeNode();
root.left.right.val = 40;
root.left.right.left = new TreeNode();
root.left.right.left.val = 35;
root.right = new TreeNode();
root.right.val = 70;
root.right.left = new TreeNode();
root.right.left.val = 60;
root.right.left.left = new TreeNode();
root.right.left.left.val = 55;
root.right.left.right = new TreeNode();
root.right.left.right.val = 65;
root.right.right = new TreeNode();
root.right.right.val = 80;
root = root.rotateLeft();
root.left = root.left.rotateLeft();
root.left.left.left = root.left.left.left.rotateLeft();

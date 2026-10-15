class TreeNode {
    constructor() {
        this.left = null;
        this.right = null;
    }

    rotateRight() {
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
root.left.right.left = new TreeNode();
root.left.right.left.val = 35;
root.left.right.left.right = new TreeNode();
root.left.right.left.right.val = 37;
root.right = new TreeNode();
root.right.val = 70;
root.right.left = new TreeNode();
root.right.left.val = 60;
root.right.left.right = new TreeNode();
root.right.left.right.val = 65;
root.right.right = new TreeNode();
root.right.right.val = 80;
root = root.rotateRight();
root.right.right = root.right.right.rotateRight();
root.right.left = root.right.left.rotateRight();
root.left.right = new TreeNode();
root.left.right.val = 25;
root.left.right.right = new TreeNode();
root.left.right.right.val = 27;
root = root.rotateRight();

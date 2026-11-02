class TreeNode {
    constructor() {
        this.left = null;
        this.right = null;
    }

    // refsyn: generated helper
    swapChildren_h0() { return this; }
    
    // refsyn: generated helper
    swapChildren_h1(hole0) { return ((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })((this).right); }
    
    // refsyn: generated helper
    swapChildren_h2(hole0, hole1) { return ((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })((this).left); }
    
    swapChildren() {
        const hole0 = this.swapChildren_h0();
        const hole1 = this.swapChildren_h1(hole0);
        const hole2 = this.swapChildren_h2(hole0, hole1);
        hole0.left = hole1;
        hole0.right = hole2;
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

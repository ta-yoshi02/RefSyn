class TreeNode {
    constructor() {
        this.left = null;
        this.right = null;
    }

    // refsyn: generated helper
    rotateLeft_h0() { return this; }
    
    // refsyn: generated helper
    rotateLeft_h1(hole0) { return ((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })((((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })((this).right)).left); }
    
    // refsyn: generated helper
    rotateLeft_h2(hole0, hole1) { return ((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })((this).right); }
    
    rotateLeft() {
        const hole0 = this.rotateLeft_h0();
        const hole1 = this.rotateLeft_h1(hole0);
        const hole2 = this.rotateLeft_h2(hole0, hole1);
        hole0.right = hole1;
        hole2.left = hole0;
        return hole2;
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
let a = new TreeNode();
a.val = 100;
a.right = new TreeNode();
a.right.val = 120;
a.right.left = new TreeNode();
a.right.left.val = 110;
a.right.left.left = new TreeNode();
a.right.left.left.val = 105;
let b = new TreeNode();
b.val = 200;
b.right = new TreeNode();
b.right.val = 220;
b.right.left = new TreeNode();
b.right.left.val = 210;
b.right.right = new TreeNode();
b.right.right.val = 230;
let c = new TreeNode();
c.val = 300;
c.right = new TreeNode();
c.right.val = 320;
c.right.left = new TreeNode();
c.right.left.val = 310;
c.right.right = new TreeNode();
c.right.right.val = 330;
c.right.right.right = new TreeNode();
c.right.right.right.val = 340;
root = root.rotateLeft();
root.left = root.left.rotateLeft();
root.left.left.left = root.left.left.left.rotateLeft();
a = a.rotateLeft();
b = b.rotateLeft();
c = c.rotateLeft();

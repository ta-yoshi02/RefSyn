class TreeNode {
    constructor() {
        this.left = null;
        this.right = null;
    }

    // refsyn: generated helper
    rotateRight_h0() { return this; }
    
    // refsyn: generated helper
    rotateRight_h1(hole0) { return ((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })((((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })((this).left)).right); }
    
    // refsyn: generated helper
    rotateRight_h2(hole0, hole1) { return ((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })((this).left); }
    
    rotateRight() {
        const hole0 = this.rotateRight_h0();
        const hole1 = this.rotateRight_h1(hole0);
        const hole2 = this.rotateRight_h2(hole0, hole1);
        hole0.left = hole1;
        hole2.right = hole0;
        return hole2;
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
let d = new TreeNode();
d.val = 400;
d.left = new TreeNode();
d.left.val = 380;
d.left.left = new TreeNode();
d.left.left.val = 370;
d.left.right = new TreeNode();
d.left.right.val = 390;
d.left.right.right = new TreeNode();
d.left.right.right.val = 395;
let e = new TreeNode();
e.val = 500;
e.left = new TreeNode();
e.left.val = 480;
e.left.left = new TreeNode();
e.left.left.val = 470;
e.left.left.left = new TreeNode();
e.left.left.left.val = 465;
e.left.right = new TreeNode();
e.left.right.val = 490;
root = root.rotateRight();
root.right.right = root.right.right.rotateRight();
root.right.left = root.right.left.rotateRight();
root.left.right = new TreeNode();
root.left.right.val = 25;
root.left.right.right = new TreeNode();
root.left.right.right.val = 27;
root = root.rotateRight();
d = d.rotateRight();
e = e.rotateRight();

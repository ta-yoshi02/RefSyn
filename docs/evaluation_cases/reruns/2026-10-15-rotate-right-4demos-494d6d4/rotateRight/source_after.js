class TreeNode {
    constructor() {
        this.left = null;
        this.right = null;
    }

    // refsyn: generated helper
    rotateRight_h0() { return this; }
    
    // refsyn: generated helper
    rotateRight_h1(hole0) { return ((this === (() => { let __refsynCur = this; const __refsynSeen = new Set(); while (__refsynCur !== null) { if (__refsynSeen.has(__refsynCur)) { throw new RangeError("refsyn: penultimateRef met a cycle"); } __refsynSeen.add(__refsynCur); const __refsynNext = ((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })((__refsynCur).left); if (__refsynNext === null) { return null; } if (((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })((__refsynNext).left) === null) { return __refsynCur; } __refsynCur = __refsynNext; } return null; })())) ? (((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })(((() => { let __refsynCur = this; const __refsynSeen = new Set(); while (__refsynCur !== null) { if (__refsynSeen.has(__refsynCur)) { throw new RangeError("refsyn: last_ptr met a cycle"); } __refsynSeen.add(__refsynCur); const __refsynNext = ((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })((__refsynCur).left); if (__refsynNext === null) { return __refsynCur; } __refsynCur = __refsynNext; } return null; })()).right)) : ((() => { let __refsynCur = (() => { let __refsynCur = this; const __refsynSeen = new Set(); while (__refsynCur !== null) { if (__refsynSeen.has(__refsynCur)) { throw new RangeError("refsyn: penultimateRef met a cycle"); } __refsynSeen.add(__refsynCur); const __refsynNext = ((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })((__refsynCur).left); if (__refsynNext === null) { return null; } if (((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })((__refsynNext).left) === null) { return __refsynCur; } __refsynCur = __refsynNext; } return null; })(); const __refsynSeen = new Set(); while (__refsynCur !== null) { if (__refsynSeen.has(__refsynCur)) { throw new RangeError("refsyn: last_ptr met a cycle"); } __refsynSeen.add(__refsynCur); const __refsynNext = ((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })((__refsynCur).right); if (__refsynNext === null) { return __refsynCur; } __refsynCur = __refsynNext; } return null; })()); }
    
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
root = root.rotateRight();
root.right.right = root.right.right.rotateRight();
root.right.left = root.right.left.rotateRight();
root.left.right = new TreeNode();
root.left.right.val = 25;
root.left.right.right = new TreeNode();
root.left.right.right.val = 27;
root = root.rotateRight();

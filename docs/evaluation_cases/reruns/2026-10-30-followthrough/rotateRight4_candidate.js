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


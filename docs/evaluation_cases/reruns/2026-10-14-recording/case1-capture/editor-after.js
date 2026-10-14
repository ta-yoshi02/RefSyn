class Node {
    constructor(v) {
        this.next = null;
        this.val = v;
    }
    // refsyn: generated helper
    insert_h0(i, arg, created0) { return arg; }
    
    // refsyn: generated helper
    insert_h1(i, arg, created0, hole0) { return (() => { let __refsynCur = this; let __refsynSteps = i; if (!Number.isInteger(__refsynSteps) || __refsynSteps < 0) { throw new RangeError("refsyn: nthNextRef needs a non-negative step count"); } while (__refsynSteps > 0 && __refsynCur !== null) { __refsynCur = ((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })((__refsynCur).next); __refsynSteps -= 1; } return __refsynCur; })(); }
    
    // refsyn: generated helper
    insert_h2(i, arg, created0, hole0, hole1) { return ((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })((hole1).next); }
    
    insert(i, arg) {
        const created0 = new Node();
        const hole0 = this.insert_h0(i, arg, created0);
        const hole1 = this.insert_h1(i, arg, created0, hole0);
        const hole2 = this.insert_h2(i, arg, created0, hole0, hole1);
        created0.val = hole0;
        hole1.next = created0;
        created0.next = hole2;
    }
}

const lst = new Node(39);
lst.next = new Node(27);
lst.next.next = new Node(53);
lst.insert(0, 89);
lst.insert(2, 46);
lst.insert(1, 71);
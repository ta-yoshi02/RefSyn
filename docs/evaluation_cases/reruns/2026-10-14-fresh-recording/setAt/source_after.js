class Node {
    constructor() {
        this.next = null;
    }

    // refsyn: generated helper
    setAt_h0(a, b) { return (() => { let __refsynCur = this; let __refsynSteps = a; if (!Number.isInteger(__refsynSteps) || __refsynSteps < 0) { throw new RangeError("refsyn: nthNextRef needs a non-negative step count"); } while (__refsynSteps > 0 && __refsynCur !== null) { __refsynCur = ((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })((__refsynCur).next); __refsynSteps -= 1; } return __refsynCur; })(); }
    
    // refsyn: generated helper
    setAt_h1(a, b, hole0) { return b; }
    
    setAt(a, b) {
        const hole0 = this.setAt_h0(a, b);
        const hole1 = this.setAt_h1(a, b, hole0);
        hole0.val = hole1;
    }
}

let lst = new Node();
lst.val = 39;
lst.next = new Node();
lst.next.val = 27;
lst.next.next = new Node();
lst.next.next.val = 53;
lst.next.next.next = new Node();
lst.next.next.next.val = 89;
lst.setAt(0, 46);
lst.setAt(2, 60);
lst.setAt(3, 18);

class Node {
    constructor() {
        this.next = null;
    }

    // refsyn: generated helper
    popBack_h0() { return (() => { let __refsynCur = this; const __refsynSeen = new Set(); while (__refsynCur !== null) { if (__refsynSeen.has(__refsynCur)) { throw new RangeError("refsyn: penultimateRef met a cycle"); } __refsynSeen.add(__refsynCur); const __refsynNext = ((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })((__refsynCur).next); if (__refsynNext === null) { return null; } if (((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })((__refsynNext).next) === null) { return __refsynCur; } __refsynCur = __refsynNext; } return null; })(); }
    
    popBack() {
        const hole0 = this.popBack_h0();
        hole0.next = null;
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
lst.popBack();
lst.popBack();

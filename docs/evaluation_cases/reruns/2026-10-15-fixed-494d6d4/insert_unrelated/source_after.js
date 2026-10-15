class Cell {
    constructor() {
        this.link = null;
    }

    // refsyn: generated helper
    insert_h0(i, arg, created0) { return arg; }
    
    // refsyn: generated helper
    insert_h1(i, arg, created0, hole0) { return (() => { let __refsynCur = this; let __refsynSteps = (i + 1); if (!Number.isInteger(__refsynSteps) || __refsynSteps < 0) { throw new RangeError("refsyn: nthNextRef needs a non-negative step count"); } while (__refsynSteps > 0 && __refsynCur !== null) { __refsynCur = ((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })((__refsynCur).link); __refsynSteps -= 1; } return __refsynCur; })(); }
    
    // refsyn: generated helper
    insert_h2(i, arg, created0, hole0, hole1) { return (() => { let __refsynCur = this; let __refsynSteps = i; if (!Number.isInteger(__refsynSteps) || __refsynSteps < 0) { throw new RangeError("refsyn: nthNextRef needs a non-negative step count"); } while (__refsynSteps > 0 && __refsynCur !== null) { __refsynCur = ((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })((__refsynCur).link); __refsynSteps -= 1; } return __refsynCur; })(); }
    
    insert(i, arg) {
        const created0 = new Cell();
        const hole0 = this.insert_h0(i, arg, created0);
        const hole1 = this.insert_h1(i, arg, created0, hole0);
        const hole2 = this.insert_h2(i, arg, created0, hole0, hole1);
        created0.key = hole0;
        created0.link = hole1;
        hole2.link = created0;
    }
}

let lst = new Cell();
lst.key = 39;
lst.link = new Cell();
lst.link.key = 27;
lst.link.link = new Cell();
lst.link.link.key = 53;
lst.insert(0, 89);
lst.insert(2, 46);
lst.insert(1, 71);

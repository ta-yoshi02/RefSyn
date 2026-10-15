class Cell {
    constructor() {
        this.link = null;
    }

    // refsyn: generated helper
    append_h0(x, created0) { return x; }
    
    // refsyn: generated helper
    append_h1(x, created0, hole0) { return (() => { let __refsynCur = this; const __refsynSeen = new Set(); while (__refsynCur !== null) { if (__refsynSeen.has(__refsynCur)) { throw new RangeError("refsyn: last_ptr met a cycle"); } __refsynSeen.add(__refsynCur); const __refsynNext = ((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })((__refsynCur).link); if (__refsynNext === null) { return __refsynCur; } __refsynCur = __refsynNext; } return null; })(); }
    
    append(x) {
        const created0 = new Cell();
        const hole0 = this.append_h0(x, created0);
        const hole1 = this.append_h1(x, created0, hole0);
        created0.key = hole0;
        hole1.link = created0;
    }
}

let lst = new Cell();
lst.key = 39;
lst.link = new Cell();
lst.link.key = 27;
lst.append(53);
lst.append(89);
lst.append(46);

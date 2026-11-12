class UF {
    constructor() {
        this.parent = null;
        this.rank = 0;
    }

    // refsyn: generated helper
    union_h0(other) { return ((((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })((this).rank) === 0)) ? (this) : ((((() => { let __refsynCur = other; const __refsynTarget = ((0 + 1) + 1); const __refsynSeen = new Set(); while (__refsynCur !== null) { if (__refsynSeen.has(__refsynCur)) { throw new RangeError("refsyn: findByValueRef met a cycle"); } __refsynSeen.add(__refsynCur); if ((__refsynCur).rank === __refsynTarget) { return __refsynCur; } __refsynCur = ((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })((__refsynCur).parent); } return null; })() === null)) ? (other) : (this)); }
    
    // refsyn: generated helper
    union_h1(other, hole0) { return ((this === hole0)) ? (other) : (this); }
    
    union(other) {
        const hole0 = this.union_h0(other);
        const hole1 = this.union_h1(other, hole0);
        hole0.parent = hole1;
    }
}

let a = new UF();
a.rank = 1;
let a1 = new UF();
a1.parent = a;
let b = new UF();
let c = new UF();
c.rank = 2;
let c1 = new UF();
c1.parent = c;
c1.rank = 1;
let c2 = new UF();
c2.parent = c1;
let d = new UF();
let e = new UF();
e.rank = 1;
let e1 = new UF();
e1.parent = e;
a.union(b);
d.union(e);
c.union(a);
e.union(c);

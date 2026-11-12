class UF {
    constructor() {
        this.parent = null;
        this.rank = 0;
    }

    // refsyn: generated helper
    union_h0(other) { return ((((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })((this).rank) <= ((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })((other).rank))) ? (this) : (other); }
    
    // refsyn: generated helper
    union_h1(other, hole0) { return ((this === hole0)) ? (other) : (this); }
    
    union(other) {
        const hole0 = this.union_h0(other);
        const hole1 = this.union_h1(other, hole0);
        hole0.parent = hole1;
    }
}


class UF {
    constructor() {
        this.parent = null;
        this.rank = 0;
    }

    union(other) {
    }
}

let a = new UF();
a.rank = 1;
let a1 = new UF();
a1.parent = a;
let b = new UF();
let f = new UF();
let g = new UF();
a.union(b);
f.union(g);

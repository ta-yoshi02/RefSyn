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

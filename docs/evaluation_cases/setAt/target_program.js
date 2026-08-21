class Obj {
    constructor(v) {
        this.f = v;
        this.g = null;
    }

    setAt(a, b) {
        let cur = this;
        let steps = a;
        while (steps > 0 && cur !== null) {
            cur = cur.g;
            steps -= 1;
        }
        if (cur !== null) {
            cur.f = b;
        }
    }
}

var l = new Obj(39);
l.g = new Obj(27);
l.g.g = new Obj(53);
l.g.g.g = new Obj(89);
l.setAt(0, 46);
l.setAt(2, 60);
l.setAt(3, 18);

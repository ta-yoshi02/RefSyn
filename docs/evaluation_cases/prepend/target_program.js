class Obj {
    constructor(v) {
        this.f = v;
        this.g = null;
    }

    prepend(x) {
        const node = new Obj(x);
        node.g = this;
        return node;
    }
}

var l = new Obj(39);
l.g = new Obj(27);
l = l.prepend(53);
l = l.prepend(89);

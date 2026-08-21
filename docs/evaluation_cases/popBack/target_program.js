class Obj {
    constructor(v) {
        this.f = v;
        this.g = null;
    }

    popBack() {
        let pred = this;
        while (pred !== null && pred.g !== null && pred.g.g !== null) {
            pred = pred.g;
        }
        if (pred !== null && pred.g !== null) {
            pred.g = null;
        }
    }
}

var l = new Obj(39);
l.g = new Obj(27);
l.g.g = new Obj(53);
l.g.g.g = new Obj(89);
l.popBack();
l.popBack();

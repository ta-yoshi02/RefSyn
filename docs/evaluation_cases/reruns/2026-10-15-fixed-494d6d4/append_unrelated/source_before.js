class Cell {
    constructor() {
        this.link = null;
    }

    append(x) {
    }
}

let lst = new Cell();
lst.key = 39;
lst.link = new Cell();
lst.link.key = 27;
lst.append(53);
lst.append(89);
lst.append(46);

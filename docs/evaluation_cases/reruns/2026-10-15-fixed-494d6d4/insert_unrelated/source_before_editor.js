class Cell {
    constructor() {
        this.link = null;
    }

    insert(i, arg) {
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

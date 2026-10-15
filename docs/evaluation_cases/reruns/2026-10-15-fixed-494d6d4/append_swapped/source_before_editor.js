class Node {
    constructor() {
        this.val = null;
    }

    append(x) {
    }
}

let lst = new Node();
lst.next = 39;
lst.val = new Node();
lst.val.next = 27;
lst.append(53);
lst.append(89);
lst.append(46);

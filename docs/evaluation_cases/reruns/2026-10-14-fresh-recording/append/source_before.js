class Node {
    constructor() {
        this.next = null;
    }

    append(x) {
    }
}

let lst = new Node();
lst.val = 39;
lst.next = new Node();
lst.next.val = 27;
lst.append(53);
lst.append(89);
lst.append(46);

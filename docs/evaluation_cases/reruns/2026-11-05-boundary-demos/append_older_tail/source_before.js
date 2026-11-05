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
let m = new Node();
m.val = 6;
m.next = new Node();
m.next.val = 7;
lst.append(53);
lst.append(89);
lst.append(46);
lst.append(0);
lst.append(1);
m.append(2);

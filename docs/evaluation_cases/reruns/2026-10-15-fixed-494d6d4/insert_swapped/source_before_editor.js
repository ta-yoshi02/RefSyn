class Node {
    constructor() {
        this.val = null;
    }

    insert(i, arg) {
    }
}

let lst = new Node();
lst.next = 39;
lst.val = new Node();
lst.val.next = 27;
lst.val.val = new Node();
lst.val.val.next = 53;
lst.insert(0, 89);
lst.insert(2, 46);
lst.insert(1, 71);

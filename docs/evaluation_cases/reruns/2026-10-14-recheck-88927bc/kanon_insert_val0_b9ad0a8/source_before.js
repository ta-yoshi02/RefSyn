class Node {
    constructor() {
        this.val = 0;
        this.next = null;
    }

    insert(i, arg) {
    }
}

let lst = new Node();
lst.val = 39;
lst.next = new Node();
lst.next.val = 27;
lst.next.next = new Node();
lst.next.next.val = 53;
lst.insert(0, 89);
lst.insert(2, 46);
lst.insert(1, 71);

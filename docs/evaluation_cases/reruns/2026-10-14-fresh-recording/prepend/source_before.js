class Node {
    constructor() {
        this.next = null;
    }

    prepend(x) {
    }
}

let lst = new Node();
lst.val = 39;
lst.next = new Node();
lst.next.val = 27;
lst = lst.prepend(53);
lst = lst.prepend(89);

class Node {
    constructor() {
        this.next = null;
    }

    setAt(a, b) {
    }
}

let lst = new Node();
lst.val = 39;
lst.next = new Node();
lst.next.val = 27;
lst.next.next = new Node();
lst.next.next.val = 53;
lst.next.next.next = new Node();
lst.next.next.next.val = 89;
lst.setAt(0, 46);
lst.setAt(2, 60);
lst.setAt(3, 18);

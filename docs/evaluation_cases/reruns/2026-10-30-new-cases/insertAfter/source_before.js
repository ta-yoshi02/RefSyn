class Node {
    constructor() {
        this.prev = null;
        this.next = null;
    }

    insertAfter(x) {
    }
}

let head = new Node();
head.val = 10;
head.next = new Node();
head.next.val = 20;
head.next.prev = head;
head.next.next = new Node();
head.next.next.val = 30;
head.next.next.prev = head.next;
head.next.next.next = new Node();
head.next.next.next.val = 40;
head.next.next.next.prev = head.next.next;
head.insertAfter(15);
head.next.next.insertAfter(25);
head.next.next.next.next.insertAfter(35);

class Node {
    constructor() {
        this.next = null;
    }

    popBack() {
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
lst.popBack();
lst.popBack();

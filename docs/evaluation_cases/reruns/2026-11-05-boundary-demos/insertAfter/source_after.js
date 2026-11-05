class Node {
    constructor() {
        this.prev = null;
        this.next = null;
    }

    // refsyn: generated helper
    insertAfter_h0(x, created0) { return x; }
    
    // refsyn: generated helper
    insertAfter_h1(x, created0, hole0) { return this; }
    
    // refsyn: generated helper
    insertAfter_h2(x, created0, hole0, hole1) { return ((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })((this).next); }
    
    insertAfter(x) {
        const created0 = new Node();
        const hole0 = this.insertAfter_h0(x, created0);
        const hole1 = this.insertAfter_h1(x, created0, hole0);
        const hole2 = this.insertAfter_h2(x, created0, hole0, hole1);
        created0.val = hole0;
        created0.prev = hole1;
        created0.next = hole2;
        hole2.prev = created0;
        hole1.next = created0;
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
head.next.insertAfter(0);
head.next.next.next.next.insertAfter(1);

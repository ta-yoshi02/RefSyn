class Node {
    constructor() {
        this.prev = null;
        this.next = null;
    }

    // refsyn: generated helper
    unlink_h0() { return ((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })((this).prev); }
    
    // refsyn: generated helper
    unlink_h1(hole0) { return ((__refsynValue) => { if (__refsynValue === undefined) { throw new TypeError("refsyn: read of an undefined field"); } return __refsynValue; })((this).next); }
    
    unlink() {
        const hole0 = this.unlink_h0();
        const hole1 = this.unlink_h1(hole0);
        hole0.next = hole1;
        hole1.prev = hole0;
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
head.next.next.next.next = new Node();
head.next.next.next.next.val = 50;
head.next.next.next.next.prev = head.next.next.next;
head.next.next.next.next.next = new Node();
head.next.next.next.next.next.val = 60;
head.next.next.next.next.next.prev = head.next.next.next.next;
head.next.next.unlink();
head.next.unlink();
head.next.next.unlink();

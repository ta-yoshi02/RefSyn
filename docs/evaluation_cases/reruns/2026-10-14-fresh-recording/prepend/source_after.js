class Node {
    constructor() {
        this.next = null;
    }

    // refsyn: generated helper
    prepend_h0(x, created0) { return x; }
    
    // refsyn: generated helper
    prepend_h1(x, created0, hole0) { return this; }
    
    prepend(x) {
        const created0 = new Node();
        const hole0 = this.prepend_h0(x, created0);
        const hole1 = this.prepend_h1(x, created0, hole0);
        created0.val = hole0;
        created0.next = hole1;
        return created0;
    }
}

let lst = new Node();
lst.val = 39;
lst.next = new Node();
lst.next.val = 27;
lst = lst.prepend(53);
lst = lst.prepend(89);

import { listCases } from "../lists.mjs";
// Kanon's demonstrations insert after the i-th node, counting the receiver as 0.
export const classSource = `class Node {
  constructor() { this.next = null; }
  insert(i, arg) {
    let node = this;
    for (let k = 0; k < i; k += 1) node = node.next;
    const added = new Node();
    added.val = arg;
    added.next = node.next;
    node.next = added;
  }
}`;
export const cases = listCases((length, k) => [(k * 2) % length, 100 - k * 13]);

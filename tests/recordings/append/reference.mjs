import { listCases } from "../lists.mjs";
export const classSource = `class Node {
  constructor() { this.next = null; }
  append(x) {
    let node = this;
    while (node.next !== null) node = node.next;
    const added = new Node();
    added.val = x;
    node.next = added;
  }
}`;
export const cases = listCases((length, k) => [k % 4 === 0 ? -1 : 10 * k]);

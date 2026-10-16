import { listCases } from "../lists.mjs";
// Kanon's demonstrations pop from lists of two or more nodes.
export const classSource = `class Node {
  constructor() { this.next = null; }
  popBack() {
    let node = this;
    while (node.next.next !== null) node = node.next;
    node.next = null;
  }
}`;
export const cases = listCases(() => [], { minLength: 2 });

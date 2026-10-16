import { listCases } from "../lists.mjs";
export const classSource = `class Node {
  constructor() { this.next = null; }
  setAt(a, b) {
    let node = this;
    for (let k = 0; k < a; k += 1) node = node.next;
    node.val = b;
  }
}`;
export const cases = listCases((length, k) => [(k * 2) % length, k % 3 === 0 ? 0 : -k * 7]);

import { listCases } from "../lists.mjs";
export const classSource = `class Node {
  constructor() { this.next = null; }
  prepend(x) {
    const added = new Node();
    added.val = x;
    added.next = this;
    return added;
  }
}`;
export const cases = listCases((length, k) => [k % 4 === 0 ? 0 : 5 - 3 * k]);

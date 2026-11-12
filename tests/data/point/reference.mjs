// The `point` demonstrations (operation_order_tests::distinct_lists): on A -> B -> C, A.other = C
// and B.other = C, and the same on P -> Q -> R with the two writes in either order.
//
// A reference specification chosen for the test, on a domain also chosen for it: the receiver is
// followed by exactly two cells (`this.next.next.next === null`). Three readings of the
// demonstrations were considered: "the receiver and its successor point to the cell two ahead",
// "... to the last cell", and "the receiver and the second-to-last cell point to the last cell".
// They agree on this domain, which is why it was chosen. That does not make this the meaning the
// demonstrations fix there: other readings are possible. Longer and shorter lists are undecided,
// and no reference is given for them.
export const classSource = `class Cell {
  constructor() { this.value = 0; this.next = null; this.other = null; }
  point() {
    const target = this.next.next;
    this.other = target;
    this.next.other = target;
  }
}`;

const cell = (Cell, value, next = null, other = null) => {
  const c = new Cell();
  c.value = value;
  c.next = next;
  c.other = other;
  return c;
};

// Values include 1, which one adopted method tests for, and repeats.
const valueSets = [[1, 2, 3], [4, 5, 6], [0, 0, 0], [1, 1, 1], [-3, 1, 7], [3, 2, 1]];

export const cases = [];
for (const values of valueSets) {
  for (const layout of ["alone", "after a prefix", "aliased and linked"]) {
    cases.push({
      name: `${layout}, values ${values.join(", ")}`,
      build({ Cell }) {
        const last = cell(Cell, values[2]);
        const middle = cell(Cell, values[1], last);
        const receiver = cell(Cell, values[0], middle);
        const roots = { receiver };
        if (layout === "after a prefix") {
          roots.head = cell(Cell, 9, cell(Cell, 8, receiver));
        }
        if (layout === "aliased and linked") {
          // `other` already set, an outside cell pointing into the list, and a detached cell.
          receiver.other = receiver;
          last.other = middle;
          roots.outside = cell(Cell, 1, middle, last);
          roots.detached = cell(Cell, 1);
        }
        return { receiver, args: [], roots };
      },
    });
  }
}

// Inputs shared by the list references: lengths 1-8, three surroundings of the receiver, and
// values with zeros, negatives and repeats. `args(length, k)` gives the k-th case's arguments.
const values = (length, k) => Array.from({ length }, (_, i) => [39, -27, 0, 53, -27, 89, 0, 46][(i + k) % 8]);

export function listCases(args, { minLength = 1 } = {}) {
  const cases = [];
  for (const length of [1, 2, 3, 5, 8].filter((n) => n >= minLength)) {
    for (const layout of ["head", "interior", "shared"]) {
      const k = cases.length;
      cases.push({
        name: `${layout}, length ${length}, args ${JSON.stringify(args(length, k))}`,
        build({ Node }) {
          const node = (val, next = null) => { const n = new Node(); n.val = val; n.next = next; return n; };
          const head = values(length, k).reduceRight((next, val) => node(val, next), null);
          const middle = Array.from({ length: Math.floor(length / 2) }).reduce((n) => n.next, head);
          const roots = { receiver: head, alias: head, middle };
          if (layout === "interior") roots.outer = node(7, node(8, head));
          if (layout === "shared") Object.assign(roots, { branch: node(77, middle), detached: node(-5, node(6)) });
          return { receiver: head, args: args(length, k), roots };
        },
      });
    }
  }
  return cases;
}

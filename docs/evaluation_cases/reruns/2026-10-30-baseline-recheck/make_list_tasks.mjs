// The tasks of ../../nonseparated_general_components/tasks.json with the library's list components
// (typed Ref[T] x List[T], so they never accept gHeap : List[Ref[Object[Obj]]]) replaced by the JS ports
// of ../2026-10-30-baseline/make_tasks.mjs: same names and semantics, typed for heaps of references
// (checked against the library on 20 000 heaps by ../2026-10-30-baseline/check_list_ports.mjs).
// Nothing else changes: examples, signatures, the other components.
// Usage (from the repository root): node <this> > tasks_list.json
import { readFileSync } from "node:fs";

const listPorts = (C) => {
  const R = `Ref[Object[${C}]]`, LR = `List[Ref[Object[${C}]]]`, LO = `List[Object[${C}]]`;
  const step = "const nx = (h, i) => (i >= 0 && i < h.length && h[i] && typeof h[i].ref === 'number' ? h[i].ref : null);";
  return [
    { name: "last_ptr", kind: "js", inputTypes: [R, LR], returnType: R, args: ["s", "h"], bodyJs: `${step} let c = s.ref; if (c === -1) return { ref: -1 }; const seen = new Set(); while (true) { if (seen.has(c)) return 'error'; seen.add(c); const n = nx(h, c); if (n === null) return 'error'; if (n === -1) return { ref: c }; c = n; }` },
    { name: "penultimateRef", kind: "js", inputTypes: [R, LR], returnType: R, args: ["s", "h"], bodyJs: `${step} let c = s.ref; if (c === -1) return { ref: -1 }; const seen = new Set(); while (true) { if (seen.has(c)) return 'error'; seen.add(c); const n = nx(h, c); if (n === null) return 'error'; if (n === -1) return { ref: -1 }; const nn = nx(h, n); if (nn === null) return 'error'; if (nn === -1) return { ref: c }; c = n; }` },
    { name: "nthNextRef", kind: "js", inputTypes: [R, LO, LR, "Int"], returnType: R, args: ["s", "nodes", "h", "k"], bodyJs: `${step} if (k < 0) return 'error'; let c = s.ref; if (c === -1) return { ref: -1 }; if (c >= nodes.length) return 'error'; for (let r = k; r > 0; r -= 1) { if (c === -1) return { ref: -1 }; const n = nx(h, c); if (n === null) return 'error'; c = n; } return { ref: c };` },
  ];
};
const ports = new Map(listPorts("Obj").map((p) => [p.name, p]));
const tasks = JSON.parse(readFileSync("docs/evaluation_cases/nonseparated_general_components/tasks.json", "utf8"));
console.log(JSON.stringify(tasks.map((t) => ({ ...t, components: t.components.map((c) => ports.get(c.name) ?? c) })), null, 2));

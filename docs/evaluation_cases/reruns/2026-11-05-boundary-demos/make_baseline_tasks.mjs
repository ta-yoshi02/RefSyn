// Copy of ../2026-10-30-baseline/make_tasks.mjs with only the recordings and file paths changed to this
// directory's recordings (2026-11-05-boundary-demos).
// Builds non-separated (whole-state) PBE tasks from the saved MOLD tasks of the non-list cases.
// Input per demonstration: the pre-call state as MOLD encodes it (thisRef, nodeHeap, every field heap,
// the method's own arguments), without the objects the method creates and without hole values.
// Output: one task per field heap the method changes (the heap after the call, created objects
// appended in creation order), one for the return reference if the method returns one, and one for the
// pair of changed pointer heaps when exactly two change. The post-state is computed from the hole
// outputs of the MOLD tasks and the template writes in the response's common_pattern; every recording
// passed whole-method validation, so this equals the demonstrated post-state.
// Components: the MOLD task's components plus storeAtRef, freshRefFromHeap and, for pair tasks, a pair
// constructor (as in ../../nonseparated_general_components/).
// Usage (from the repository root): node <this> > <out.json>
import { readFileSync } from "node:fs";

const R = "docs/evaluation_cases/reruns";
const cases = {
  rotateRight: `${R}/2026-11-05-boundary-demos/rotateRight`,
  insertAfter: `${R}/2026-11-05-boundary-demos/insertAfter`,
  rotateLeft: `${R}/2026-11-05-boundary-demos/rotateLeft`,
};

const parsePattern = (text) => {
  const creates = [...text.matchAll(/Create\(Create \{ vertex: (\d+), class: "(\w+)" \}\)/g)].map((m) => Number(m[1]));
  const holes = [...text.matchAll(/BindHole\((\d+), Hole \{ vertex: (\d+)/g)].map((m) => [Number(m[1]), Number(m[2])]);
  const writes = [...text.matchAll(/Write\(Write \{ from: (\d+), field: "(\w+)", to: (\d+) \}\)/g)].map((m) => ({ from: Number(m[1]), field: m[2], to: Number(m[3]) }));
  const ret = text.match(/Return\((\d+)\)/);
  return { creates, holes, writes, ret: ret ? Number(ret[1]) : null };
};

const tasks = [];
for (const [caseName, dir] of Object.entries(cases)) {
  const mold = JSON.parse(readFileSync(`${dir}/mold_escher_tasks.json`, "utf8"));
  const pattern = parsePattern(JSON.parse(readFileSync(`${dir}/response.json`, "utf8")).common_pattern);
  const h0 = mold[0];
  const sig = h0.signature.autoExpandClassSignature;
  const cls = h0.classes.find((c) => c.name === sig.className);
  const heapNames = Object.keys(sig.fieldHeapNames);
  const ownArgs = (h0.signature.args ?? []).filter((a) => !/^(created\d+|hole\d+)$/.test(a.name));
  const nCreated = pattern.creates.length;

  const demos = h0.examples.map(([input], d) => {
    const [thisRef, nodeHeap, ...rest] = input;
    const heaps = rest.slice(0, heapNames.length);
    const args = rest.slice(heapNames.length, heapNames.length + ownArgs.length);
    const n = nodeHeap.length - nCreated;
    const pre = { thisRef, nodeHeap: nodeHeap.slice(0, n), heaps: heaps.map((h) => h.slice(0, n)), args };
    const value = {};
    pattern.creates.forEach((v, k) => { value[v] = { ref: n + k }; });
    for (const [k, v] of pattern.holes) value[v] = mold[k].examples[d][1];
    const post = heaps.map((h) => h.map((x) => x));
    for (const w of pattern.writes) {
      const fi = heapNames.indexOf(w.field);
      post[fi][value[w.from].ref] = value[w.to];
    }
    return { pre, post, ret: pattern.ret === null ? null : value[pattern.ret] };
  });

  const changed = heapNames.filter((_, fi) => demos.some(({ pre, post }) => JSON.stringify(pre.heaps[fi]) !== JSON.stringify(post[fi])));
  const heapType = (f) => (cls.fields[f].startsWith("Ref[Object") ? `List[${cls.fields[f]}]` : "List[Int]");
  const base = {
    category: "baseline", classes: h0.classes, exposeClassComponents: false, autoClassFieldComponents: true, strictFieldAccess: true,
    components: [
      ...h0.components,
      { name: "storeAtRef", kind: "libraryRef", ref: "storeAtRef" },
      { name: "freshRefFromHeap", kind: "libraryRef", ref: "freshRefFromHeap" },
    ],
  };
  // Explicit signature as in ../../nonseparated_general_components/: pointer heaps are lists of
  // references. (The MOLD helper tasks type them List[Object[C]], under which no heap of references
  // can be the goal and storeAtRef cannot store a reference.)
  const signature = (returnType) => ({
    inputNames: [sig.thisRefName, sig.classHeapName, ...heapNames.map((f) => sig.fieldHeapNames[f]), ...ownArgs.map((a) => a.name)],
    inputTypes: [`Ref[Object[${cls.name}]]`, `List[Object[${cls.name}]]`, ...heapNames.map((f) => heapType(f)), ...ownArgs.map((a) => a.type)],
    returnType,
  });
  const input = ({ pre }) => [pre.thisRef, pre.nodeHeap, ...pre.heaps, ...pre.args];
  for (const f of changed) {
    const fi = heapNames.indexOf(f);
    tasks.push({ ...base, name: `${caseName}-whole-${f}Heap`, case: caseName, output: `${f}Heap`, signature: signature(heapType(f)), examples: demos.map((d) => [input(d), d.post[fi]]) });
  }
  if (pattern.ret !== null) {
    tasks.push({ ...base, name: `${caseName}-whole-returnRef`, case: caseName, output: "returnRef", signature: signature(`Ref[Object[${cls.name}]]`), examples: demos.map((d) => [input(d), d.ret]) });
  }
  const changedPtr = changed.filter((f) => cls.fields[f].startsWith("Ref[Object"));
  if (changedPtr.length === 2) {
    const [a, b] = changedPtr.map((f) => heapType(f));
    const pairType = `Pair[${a},${b}]`;
    tasks.push({
      ...base, name: `${caseName}-whole-pair`, case: caseName, output: `pair(${changedPtr.join(",")})`, signature: signature(pairType),
      components: [...base.components, { name: "pairHeap", kind: "js", inputTypes: [a, b], returnType: pairType, args: ["a", "b"], bodyJs: "return { pair: [a, b] };" }],
      examples: demos.map((d) => [input(d), { pair: changedPtr.map((f) => d.post[heapNames.indexOf(f)]) }]),
    });
  }
}
// The library's list components are typed Ref[T] x List[T] (for MOLD's List[Object[C]] heaps), so with
// heaps of references above they never type-check. Variant "+list": the same four components, same
// names and semantics, typed for heaps of references (JS ports of external/escher-ts/src/library/
// common-comps-heap.ts), replacing the library ones.
const listPorts = (C) => {
  const R = `Ref[Object[${C}]]`, LR = `List[Ref[Object[${C}]]]`, LO = `List[Object[${C}]]`;
  const step = "const nx = (h, i) => (i >= 0 && i < h.length && h[i] && typeof h[i].ref === 'number' ? h[i].ref : null);";
  return [
    { name: "last_ptr", kind: "js", inputTypes: [R, LR], returnType: R, args: ["s", "h"], bodyJs: `${step} let c = s.ref; if (c === -1) return { ref: -1 }; const seen = new Set(); while (true) { if (seen.has(c)) return 'error'; seen.add(c); const n = nx(h, c); if (n === null) return 'error'; if (n === -1) return { ref: c }; c = n; }` },
    { name: "penultimateRef", kind: "js", inputTypes: [R, LR], returnType: R, args: ["s", "h"], bodyJs: `${step} let c = s.ref; if (c === -1) return { ref: -1 }; const seen = new Set(); while (true) { if (seen.has(c)) return 'error'; seen.add(c); const n = nx(h, c); if (n === null) return 'error'; if (n === -1) return { ref: -1 }; const nn = nx(h, n); if (nn === null) return 'error'; if (nn === -1) return { ref: c }; c = n; }` },
    { name: "nthNextRef", kind: "js", inputTypes: [R, LO, LR, "Int"], returnType: R, args: ["s", "nodes", "h", "k"], bodyJs: `${step} if (k < 0) return 'error'; let c = s.ref; if (c === -1) return { ref: -1 }; if (c >= nodes.length) return 'error'; for (let r = k; r > 0; r -= 1) { if (c === -1) return { ref: -1 }; const n = nx(h, c); if (n === null) return 'error'; c = n; } return { ref: c };` },
    { name: "findByValueRef", kind: "js", inputTypes: [R, LO, LR, "List[Int]", "Int"], returnType: R, args: ["s", "nodes", "h", "vals", "t"], bodyJs: `${step} let c = s.ref; const seen = new Set(); while (c !== -1) { if (seen.has(c) || c >= nodes.length || c >= h.length || c >= vals.length) return 'error'; seen.add(c); if (typeof vals[c] === 'number' && vals[c] === t) return { ref: c }; const n = nx(h, c); if (n === null) return 'error'; c = n; } return { ref: -1 };` },
  ];
};
const listNames = new Set(["last_ptr", "penultimateRef", "nthNextRef", "findByValueRef"]);
const withList = tasks.map((t) => ({
  ...t,
  name: `${t.name}+list`,
  components: [...t.components.filter((c) => !listNames.has(c.name)), ...listPorts(t.classes[0].name).filter((p) => t.components.some((c) => c.name === p.name))],
}));
console.log(JSON.stringify([...tasks, ...withList], null, 2));

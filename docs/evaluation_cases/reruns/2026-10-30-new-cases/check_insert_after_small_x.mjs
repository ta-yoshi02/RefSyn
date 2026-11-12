// The applied insertAfter (insertAfter/source_after.js; its helpers equal the candidate's) on lists of
// length 2-7, every non-tail position, x in {0, 1, 2, 3, 99}, against a reference insertAfter.
// Usage (from the repository root): node <this>
import { readFileSync } from "node:fs";
const src = readFileSync("docs/evaluation_cases/reruns/2026-10-30-new-cases/insertAfter/source_after.js", "utf8");
const Node = new Function(`${src.slice(0, src.indexOf("\nlet head"))}\nreturn Node;`)();
const build = (n) => { const ns = Array.from({ length: n }, (_, i) => { const o = new Node(); o.val = (i + 1) * 10; return o; }); ns.forEach((o, i) => { if (i > 0) o.prev = ns[i - 1]; if (i < n - 1) o.next = ns[i + 1]; }); return ns; };
const snap = (ns) => { const all = [...ns]; for (const o of ns) for (const f of ["prev", "next"]) if (o[f] && !all.includes(o[f])) all.push(o[f]); return JSON.stringify(all.map((o) => [o.val, all.indexOf(o.prev), all.indexOf(o.next)])); };
let agree = 0, total = 0;
for (let n = 2; n <= 7; n += 1) for (let p = 0; p < n - 1; p += 1) for (const x of [0, 1, 2, 3, 99]) {
  const a = build(n); a[p].insertAfter(x);
  const b = build(n); const t = b[p]; const m = new Node(); m.val = x; m.prev = t; m.next = t.next; t.next.prev = m; t.next = m;
  total += 1; if (snap(a) === snap(b)) agree += 1;
}
console.log(JSON.stringify({ agree, total }));

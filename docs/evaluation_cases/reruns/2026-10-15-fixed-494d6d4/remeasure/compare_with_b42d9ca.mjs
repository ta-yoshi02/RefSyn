// Compares the 494d6d4 measurement with the b42d9ca one (2026-10-14-fresh-recording):
// solved repetitions, the set of found terms per task, and method time medians.
// Usage: node <this>   (any working directory)
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const old = join(here, "../../2026-10-14-fresh-recording/measurements");
const files = ["insert_general", "append", "popBack", "prepend", "setAt"].map((c) => `mold_${c}.json`)
  .concat(["insert", "append", "popBack", "prepend", "setAt"].map((m) => `baseline_${m}.json`));
const terms = (m) => Object.fromEntries(m.tasks.map((t) => [t, [...new Set(m.repetitions.map((r) => r.tasks.find((x) => x.task === t).term ?? null))].sort()]));
const rows = files.map((f) => {
  const a = JSON.parse(readFileSync(join(old, f), "utf8"));
  const b = JSON.parse(readFileSync(join(here, f), "utf8"));
  return {
    file: f,
    solved: [a.solved_repetitions, b.solved_repetitions],
    same_terms: JSON.stringify(terms(a)) === JSON.stringify(terms(b)),
    terms_494d6d4: terms(b),
    median_ms: [a.method_total_ms.median, b.method_total_ms.median],
    ratio: Number((b.method_total_ms.median / a.method_total_ms.median).toFixed(2)),
  };
});
writeFileSync(join(here, "compare_with_b42d9ca.json"), JSON.stringify(rows, null, 2) + "\n");
for (const r of rows) console.log(r.file.padEnd(28), "solved", r.solved.join("→"), "terms", r.same_terms ? "same" : "DIFFER", "median", r.median_ms.join(" → "), "x" + r.ratio);

// Lists, for each renamed recording and its original (2026-10-14-fresh-recording), the
// synthesized term of every helper, and checks that the renamed term equals the original
// after mapping the field heaps (val→value field, next→pointer field).
// Usage: node <this>   (any working directory)
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const orig = join(here, "../2026-10-14-fresh-recording");
const terms = (file) =>
  Object.fromEntries(
    JSON.parse(readFileSync(file, "utf8")).escher_results.map((e) => [e.name, e.success ? e.rendered.split("=\n")[1].trim() : null]),
  );
const cases = [
  ["insert_unrelated", "insert_general", { val: "key", next: "link" }],
  ["insert_swapped", "insert_general", { val: "next", next: "val" }],
  ["append_unrelated", "append", { val: "key", next: "link" }],
  ["append_swapped", "append", { val: "next", next: "val" }],
];
const rows = cases.map(([c, o, map]) => {
  const renamed = terms(join(here, c, "response.json"));
  const original = terms(join(orig, o, "response.json"));
  const mapped = Object.fromEntries(
    Object.entries(original).map(([k, t]) => [k, t && t.replace(/@(val|next)Heap/g, (_, f) => `@${map[f]}Heap`)]),
  );
  return { case: c, original_case: o, field_map: map, renamed, original, equal_after_mapping: JSON.stringify(renamed) === JSON.stringify(mapped) };
});
writeFileSync(join(here, "compare_terms.json"), JSON.stringify(rows, null, 2) + "\n");
for (const r of rows) {
  console.log(r.case, "equal_after_mapping:", r.equal_after_mapping);
  for (const [k, t] of Object.entries(r.renamed)) console.log("  ", k, "=", t);
}

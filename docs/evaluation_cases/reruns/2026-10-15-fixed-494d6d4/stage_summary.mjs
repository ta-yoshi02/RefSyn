// Counts the end-to-end stages of each recording from the files in its case directory.
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
const run = process.argv[2];
const cases = ["insert_unrelated", "insert_swapped", "append_unrelated", "append_swapped"];
const read = (c, f) => readFileSync(join(run, c, f), "utf8");
const rows = cases.map((c) => {
  const request = JSON.parse(read(c, "raw_request.json"));
  const response = JSON.parse(read(c, "response.json"));
  const markers = JSON.parse(read(c, "kanon_rerun_markers.json")).markers;
  const rerun = JSON.parse(read(c, "node_rerun_result.json"));
  const source = read(c, "source_after.js");
  const demos = request.method_calls.length;
  const helpers = response.escher_results ?? [];
  return {
    case: c,
    demonstrations: demos,
    edits_per_demonstration: request.method_calls.map((m) => m.operations.length),
    request_accepted: read(c, "http_status.txt").startsWith("200"),
    synthesis: { solved: helpers.filter((h) => h.success).length, helpers: helpers.length },
    translation: { translated: (response.code ?? []).length, helpers: helpers.length },
    validation: { status: response.validation?.status, checked: response.validation?.checked_demonstrations, demonstrations: demos },
    applied: source.includes("// refsyn: generated helper") && source.includes(response.composed_method_code.split("\n")[0]),
    kanon_rerun: { passed: markers.filter((m) => m.clazz === "testPassed").length, calls: demos },
    node_rerun_match: rerun.match,
  };
});
const all = (f) => rows.filter(f).length;
const summary = {
  recordings: rows.length,
  request_accepted: all((r) => r.request_accepted),
  synthesis_all_helpers: all((r) => r.synthesis.solved === r.synthesis.helpers && r.synthesis.helpers > 0),
  translation_all_helpers: all((r) => r.translation.translated === r.translation.helpers && r.translation.helpers > 0),
  validation_passed_all_demonstrations: all((r) => r.validation.status === "passed" && r.validation.checked === r.validation.demonstrations),
  applied: all((r) => r.applied),
  kanon_rerun_all_passed: all((r) => r.kanon_rerun.passed === r.kanon_rerun.calls),
  node_rerun_match: all((r) => r.node_rerun_match),
  note: "One recording per case.",
};
writeFileSync(join(run, "stage_summary.json"), JSON.stringify({ summary, rows }, null, 2) + "\n");
console.log(JSON.stringify(summary, null, 2));
for (const r of rows) console.log(r.case, JSON.stringify([r.synthesis, r.translation, r.validation, r.applied, r.kanon_rerun]));

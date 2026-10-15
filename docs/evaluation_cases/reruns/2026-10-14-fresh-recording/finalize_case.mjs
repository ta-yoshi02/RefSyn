// Extracts a case's evidence from the server log and checks the saved response.
// Usage (from the repository root):
//   node <run>/finalize_case.mjs <case-dir> <expected-final-vals-json>
// Reads <case-dir>/server_log_lines_before.txt and <case-dir>/response.json.
import { readFileSync, writeFileSync, mkdirSync, copyFileSync, renameSync, readdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { execFileSync } from "node:child_process";
import vm from "node:vm";

const [caseDir, expectedJson] = process.argv.slice(2);
const runDir = dirname(caseDir);
const log = readFileSync(join(runDir, "logs/refsyn_server.log"), "utf8").split("\n");
const start = Number(readFileSync(join(caseDir, "server_log_lines_before.txt"), "utf8").trim());

// The request sent by the page is the first raw payload after the recorded line count.
const marker = log.findIndex((line, i) => i >= start && line === "--- RAW REQUEST PAYLOAD ---");
if (marker < 0) throw new Error("no raw payload after line " + start);
const raw = log[marker + 1];
const end = log.findIndex((line, i) => i > marker + 2 && line === "=====================================");
writeFileSync(join(caseDir, "raw_request.json"), raw + "\n");
writeFileSync(join(caseDir, "server_output.txt"), log.slice(marker + 3, end + 1).join("\n") + "\n");

const request = JSON.parse(raw);
writeFileSync(
  join(caseDir, "operations.json"),
  JSON.stringify(
    request.method_calls.map((c) => ({
      callLabel: c.callLabel,
      receiverObject: c.receiverObject,
      arguments: c.arguments,
      operations: c.operations,
    })),
    null,
    2,
  ) + "\n",
);
writeFileSync(join(caseDir, "typed_precall_state.json"), JSON.stringify(request.validation, null, 2) + "\n");

const response = JSON.parse(readFileSync(join(caseDir, "response.json"), "utf8"));
const taskPath = /target\/escher\/ts\/\S+\.json/.exec(response.list_environment_info ?? "")?.[0];
if (taskPath) copyFileSync(taskPath, join(caseDir, "mold_escher_tasks.json"));

// Post the same request again and compare everything but the task file name.
const check = join(caseDir, "transcription_check");
mkdirSync(check, { recursive: true });
const before = new Set(readdirSync("target/escher/ts"));
const replay = execFileSync("curl", [
  "-s", "-H", "Content-Type: application/json", "--data-binary", "@" + join(caseDir, "raw_request.json"),
  "http://127.0.0.1:3030/synthesize",
]).toString();
writeFileSync(join(check, "replay_response.json"), replay + "\n");
const replayTask = [...readdirSync("target/escher/ts")].find((f) => !before.has(f));
const canon = (value) =>
  JSON.stringify(value, (key, v) =>
    v && typeof v === "object" && !Array.isArray(v)
      ? Object.fromEntries(Object.keys(v).sort().filter((k) => k !== "list_environment_info").map((k) => [k, v[k]]))
      : v,
  );
const sameResponse = canon(response) === canon(JSON.parse(replay));
let sameTask = null;
if (replayTask && taskPath) {
  sameTask = readFileSync(join("target/escher/ts", replayTask)).equals(readFileSync(taskPath));
  renameSync(join("target/escher/ts", replayTask), join(check, "replay_task.json"));
}
writeFileSync(
  join(check, "result.json"),
  JSON.stringify({ response_equal_except_task_path_and_key_order: sameResponse, task_json_identical: sameTask }, null, 2) + "\n",
);

// Re-run the applied source outside Kanon.
const source = readFileSync(join(caseDir, "source_after.js"), "utf8");
const context = {};
vm.createContext(context);
vm.runInContext(source + "\n;globalThis.__lst = lst;", context);
const vals = [];
for (let node = context.__lst, k = 0; node !== null && node !== undefined && k < 100; node = node.next, k++) vals.push(node.val);
const expected = JSON.parse(expectedJson);
writeFileSync(
  join(caseDir, "node_rerun_result.json"),
  JSON.stringify(
    { method: "node vm re-execution of source_after.js (independent of Kanon)", node: process.version, final_vals: vals, expected, match: JSON.stringify(vals) === JSON.stringify(expected) },
    null,
    2,
  ) + "\n",
);

console.log({
  calls: request.method_calls.map((c) => [c.callLabel, c.arguments, c.operations.length]),
  status: response.validation,
  sameResponse,
  sameTask,
  vals,
});

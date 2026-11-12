// 縦の段階6 escher-ts から得られた解の変換: 穴ごとの問題 → escher-ts の解 (rendered) → JS の補助メソッド (compiled_js)
// 問題は段階5 (step_stages.rs の problems) と同じもの。run_payload が target/escher/ts/ に保存した JSON を読む。
// 実行: node tests/steps/step-translate.mjs   (リポジトリのルートで)
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import { runRefsynTasks } from "../../runtime/refsyn-escher-adapter.mjs";

// 1. パイプラインを動かす (00〜03 と同じ)。応答の list_environment_info に、保存した問題のパスが書いてある。
const output = execFileSync(
  "cargo",
  ["run", "-q", "--example", "run_payload", "--", "tests/steps/01-setval.json"],
  { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 },
);
const [, status, json] = output.match(/^HTTP (\d+).*\n([\s\S]*)$/m);
assert.equal(status, "200");
const response = JSON.parse(json);
const [, tasksPath] = response.list_environment_info.match(/task JSON saved: (\S+)/);

// 2. 保存された問題を読み、パイプラインと同じ関数 runRefsynTasks で escher-ts に解かせて変換する。
const tasks = JSON.parse(fs.readFileSync(tasksPath, "utf8"));
for (const outcome of runRefsynTasks(tasks)) {
  console.log(`--- ${outcome.name} ---`);
  console.log("escher-ts の解:", outcome.rendered);
  console.log("変換後の JS  :", outcome.compiled_js);
}

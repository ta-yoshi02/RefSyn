import { readFileSync } from "node:fs";
import { expect, it } from "vitest";
import { initSync, synthesize_browser } from "../../pkg/refsyn.js";
import { applyRefsynTaskOutcomes, runRefsynTasks } from "../../../runtime/refsyn-escher-adapter.mjs";
import { validateWholeMethod } from "../../../runtime/whole-method-validation.mjs";
import { insertSample } from "./insert";

// The sample is what the page loads first, so it must go through the whole browser path.
it("the insert sample synthesizes a method that passes whole-method validation", () => {
  initSync({ module: readFileSync(new URL("../../pkg/refsyn_bg.wasm", import.meta.url)) });
  const artifacts = JSON.parse(synthesize_browser(JSON.stringify(insertSample), JSON.stringify({ trace: false })));
  const tasks = JSON.parse(artifacts.task_json);
  const response = { ...artifacts.response, code: [], individual_codes: [] };
  applyRefsynTaskOutcomes(response, runRefsynTasks(tasks, { quiet: true }));
  const validation = validateWholeMethod({ request: insertSample, response, taskNames: tasks.map((task: { name: string }) => task.name) });
  expect(validation).toEqual({ status: "passed", checked_demonstrations: 3, error: null });
});

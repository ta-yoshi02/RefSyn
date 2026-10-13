import { readFileSync } from "node:fs";
import { beforeAll, expect, it } from "vitest";
import { initSync, synthesize_browser } from "../../pkg/refsyn.js";
import { applyRefsynTaskOutcomes, runRefsynTasks } from "../../../runtime/refsyn-escher-adapter.mjs";
import { validateWholeMethod } from "../../../runtime/whole-method-validation.mjs";

// The boolean demonstrations of tests/data/booleans, through the browser path: WASM core,
// escher-ts, adapter, whole-method validation.
const request = (name: string) =>
  JSON.parse(readFileSync(new URL(`../../../tests/data/booleans/${name}.json`, import.meta.url), "utf8"));

const synthesize = (name: string) => {
  const req = request(name);
  const artifacts = JSON.parse(synthesize_browser(JSON.stringify(req), JSON.stringify({ trace: false })));
  if (!artifacts.task_json) return { validation: null, response: artifacts.response };
  const tasks = JSON.parse(artifacts.task_json);
  const response = { ...artifacts.response, code: [], individual_codes: [] };
  applyRefsynTaskOutcomes(response, runRefsynTasks(tasks, { quiet: true }));
  const validation = validateWholeMethod({ request: req, response, taskNames: tasks.map((task: { name: string }) => task.name) });
  return { validation, response };
};

type FlagObject = { on: unknown; next: FlagObject | null; set(b: boolean): void; copy(): void };
// The adopted method in a class built here, so the expectations below do not come from the
// demonstrations or the validator.
const flagClass = (response: { code: string[]; composed_method_code: string }) =>
  new Function(`return class Flag {
    constructor() { this.on = false; this.next = null; }
    ${response.code.join("\n")}
    ${response.composed_method_code}
  };`)() as new () => FlagObject;
const flag = (Flag: new () => FlagObject, on: unknown, next: FlagObject | null = null) => {
  const f = new Flag();
  f.on = on;
  f.next = next;
  return f;
};

beforeAll(() => {
  initSync({ module: readFileSync(new URL("../../pkg/refsyn_bg.wasm", import.meta.url)) });
});

it("assigns a boolean argument to a field", () => {
  const { validation, response } = synthesize("set_flag");
  expect(validation).toEqual({ status: "passed", checked_demonstrations: 2, error: null });
  const Flag = flagClass(response);
  for (const [before, b] of [[false, true], [true, false], [true, true], [false, false]]) {
    const r = flag(Flag, before);
    r.set(b);
    expect(r.on).toBe(b);
  }
});

it("copies a boolean field from another object", () => {
  const { validation, response } = synthesize("copy_flag");
  expect(validation).toEqual({ status: "passed", checked_demonstrations: 3, error: null });
  const Flag = flagClass(response);
  for (const [mine, theirs] of [[true, false], [false, true], [true, true], [false, false]]) {
    const d = flag(Flag, theirs);
    const r = flag(Flag, mine, d);
    r.copy();
    expect(d.on).toBe(mine);
    expect(r.next).toBe(d);
  }
  expect(() => flag(Flag, undefined, flag(Flag, true)).copy()).toThrow(TypeError);
});

it("keeps demonstrated numbers numbers and refuses demonstrated strings", () => {
  const numbers = synthesize("set_flag_number_literal");
  expect(numbers.validation?.status).toBe("passed");
  const Flag = flagClass(numbers.response);
  const r = flag(Flag, false);
  r.set(true);
  expect(r.on).toBe(1);
  r.set(false);
  expect(r.on).toBe(0);
  const strings = synthesize("set_flag_string_literal");
  expect(strings.validation).toBeNull();
  expect(strings.response.list_environment_info).toMatch(/string-valued holes/);
});

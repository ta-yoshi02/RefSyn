import { describe, expect, it, vi } from "vitest";
import { handleSynthesisMessage, type BrowserSynthesisArtifacts, type SynthesisRequest } from "./browser-runner";

vi.mock("../../../runtime/whole-method-validator-client.mjs", async () => {
  const { validateWholeMethod } = await import("../../../runtime/whole-method-validation.mjs");
  return { validateInWorker: async (payload: Parameters<typeof validateWholeMethod>[0]) => validateWholeMethod(payload) };
});

const sampleRequest: SynthesisRequest = {
  method_calls: [],
  vis_graph: {
    nodes: [],
    edges: [],
  },
};

const sampleArtifacts: BrowserSynthesisArtifacts = {
  response: {
    common_pattern: "COMMON_PLAN append",
    hole_information: null,
    code: [],
    composed_method_code: "append(arg) { return this.append_f(arg); }",
    individual_codes: [],
    list_environment_info: "base environment",
    operation_analysis: null,
    escher_results: null,
  },
  task_json: JSON.stringify([
    {
      name: "tail-ref",
      category: "refsyn",
      classes: [
        {
          name: "Node",
          fields: {
            next: "Ref[Node]",
          },
        },
      ],
      exposeClassComponents: false,
      autoClassFieldComponents: true,
      signature: {
        returnType: "Ref[Node]",
        autoExpandClassSignature: {
          className: "Node",
          thisRefName: "thisRef",
          classHeapName: "nodeHeap",
          fieldHeapNames: {
            next: "nextHeap",
          },
        },
      },
      components: [
        {
          name: "isNull",
          kind: "libraryRef",
          ref: "isNull",
        },
      ],
      examples: [
        [
          [
            { ref: 0 },
            [
              {
                object: {
                  className: "Node",
                  fields: {
                    next: { ref: -1 },
                  },
                },
              },
            ],
            [
              {
                object: {
                  className: "Node",
                  fields: {
                    next: { ref: -1 },
                  },
                },
              },
            ],
          ],
          { ref: 0 },
        ],
      ],
      refsynMeta: {
        jsMethodName: "tail_ref",
        className: "Node",
        thisRefName: "thisRef",
        classHeapName: "nodeHeap",
        valueFields: [],
        pointerFields: ["next"],
        fieldHeapNames: {
          next: "nextHeap",
        },
        explicitArgs: [],
      },
    },
  ]),
  warnings: ["worker mode: no network transport used"],
};

describe("browser runner", () => {
  it("retains hole outcomes but blocks unvalidated code without using fetch", async () => {
    const fetchSpy = vi.fn();
    vi.stubGlobal("fetch", fetchSpy);

    const response = await handleSynthesisMessage(async () => sampleArtifacts, sampleRequest, {});

    expect(fetchSpy).not.toHaveBeenCalled();
    expect(response.composed_method_code).toBeNull();
    expect(response.validation?.status).toBe("unavailable");
    expect(response.escher_results?.[0]?.success).toBe(true);
    expect(response.code).toEqual([]);
    expect(response.list_environment_info).toContain("no network transport");
  });
});

const verifiedRequest: SynthesisRequest = {
  method_calls: [{ callLabel: "c", contextSensitiveID: "ctx", receiverObject: "r", receiverClassName: "Box", methodName: "run", operations: [{ editType: "addVariable", label: "return", to: "r" }] }],
  vis_graph: { nodes: [], edges: [] },
  validation: { version: 2, classes: [{ name: "Box", source: "class Box {}" }], cases: [{ callLabel: "c", contextSensitiveID: "ctx", objects: [{ id: "r", className: "Box", fields: {} }], arguments: [] }] },
};

it("adopts only a method that reproduces the full demonstration", async () => {
  const artifacts: BrowserSynthesisArtifacts = { response: { code: [], individual_codes: [], composed_method_code: "run() { return this; }" } };
  const accepted = await handleSynthesisMessage(() => artifacts, verifiedRequest);
  expect(accepted.validation?.status).toBe("passed");
  expect(accepted.composed_method_code).toContain("return this");
  artifacts.response.composed_method_code = "run() { this.unexpected = 1; return this; }";
  const rejected = await handleSynthesisMessage(() => artifacts, verifiedRequest);
  expect(rejected.validation?.status).toBe("failed");
  expect(rejected.composed_method_code).toBeNull();
});

it("reports successful PBE with failed translation as failure and blocks adoption", async () => {
  const artifacts = structuredClone(sampleArtifacts);
  const tasks = JSON.parse(artifacts.task_json!);
  delete tasks[0].refsynMeta;
  artifacts.task_json = JSON.stringify(tasks);
  const response = await handleSynthesisMessage(() => artifacts, sampleRequest);
  expect(response.escher_results?.[0]?.success).toBe(false);
  expect(response.escher_results?.[0]?.error).toContain("refsynMeta");
  expect(response.validation?.status).toBe("failed");
  expect(response.composed_method_code).toBeNull();
  expect(response.code).toEqual([]);
});

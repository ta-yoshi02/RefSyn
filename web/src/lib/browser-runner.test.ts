import { describe, expect, it, vi } from "vitest";
import {
  handleSynthesisMessage,
  type BrowserSynthesisArtifacts,
  type SynthesisRequest,
  type SynthesisResponse,
} from "./browser-runner";

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
  validation: { version: 3, classes: [{ name: "Box", source: "class Box {}", instance: {} }], cases: [{ callLabel: "c", contextSensitiveID: "ctx", objects: [{ id: "r", className: "Box", fields: {} }], arguments: [] }] },
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

// `f() { const hole0 = this.f_h0(); hole0.next = hole0; }` from one demonstration where r.next
// is redirected from a to r; the response has the shape the Rust core returns.
const selfLoopRequest: SynthesisRequest = {
  method_calls: [{
    callLabel: "c0", contextSensitiveID: "main", receiverObject: "r", receiverClassName: "C",
    methodName: "f", methodParamNames: [], arguments: [],
    operations: [{ editType: "editEdgeReference", from: "r", label: "next", oldTo: "a", newTo: "r" }],
  }],
  vis_graph: { nodes: [], edges: [] },
  validation: {
    version: 3,
    classes: [{ name: "C", source: "class C { f() {} }", instance: {} }],
    cases: [{
      callLabel: "c0", contextSensitiveID: "main", arguments: [],
      objects: [
        { id: "r", className: "C", fields: { next: { ref: "a" } } },
        { id: "a", className: "C", fields: { next: null } },
      ],
    }],
  },
};

// Task JSON as the Rust core emits it for `selfLoopRequest`.
const selfLoopTask = {
  "name": "f_h0",
  "category": "refsyn",
  "classes": [
    {
      "name": "C",
      "fields": {
        "next": "Ref[Object[C]]"
      }
    }
  ],
  "exposeClassComponents": false,
  "autoClassFieldComponents": true,
  "signature": {
    "returnType": "Ref[Object[C]]",
    "autoExpandClassSignature": {
      "className": "C",
      "thisRefName": "thisRef",
      "classHeapName": "nodeHeap",
      "fieldHeapNames": {
        "next": "nextHeap"
      }
    }
  },
  "components": [
    {
      "name": "isNull",
      "kind": "libraryRef",
      "ref": "isNull"
    },
    {
      "name": "equal",
      "kind": "libraryRef",
      "ref": "equal"
    },
    {
      "name": "and",
      "kind": "libraryRef",
      "ref": "and"
    },
    {
      "name": "or",
      "kind": "libraryRef",
      "ref": "or"
    },
    {
      "name": "not",
      "kind": "libraryRef",
      "ref": "not"
    },
    {
      "name": "last_ptr",
      "kind": "libraryRef",
      "ref": "last_ptr"
    },
    {
      "name": "penultimateRef",
      "kind": "libraryRef",
      "ref": "penultimateRef"
    },
    {
      "name": "nthNextRef",
      "kind": "libraryRef",
      "ref": "nthNextRef"
    }
  ],
  "examples": [
    [
      [
        {
          "ref": 0
        },
        [
          {
            "object": {
              "className": "C",
              "fields": {
                "next": {
                  "ref": 1
                }
              }
            }
          },
          {
            "object": {
              "className": "C",
              "fields": {
                "next": {
                  "ref": -1
                }
              }
            }
          }
        ],
        [
          {
            "ref": 1
          },
          {
            "ref": -1
          }
        ]
      ],
      {
        "ref": 0
      }
    ]
  ],
  "refsynMeta": {
    "jsMethodName": "f_h0",
    "className": "C",
    "thisRefName": "thisRef",
    "classHeapName": "nodeHeap",
    "valueFields": [],
    "pointerFields": [
      "next"
    ],
    "fieldHeapNames": {
      "next": "nextHeap"
    },
    "explicitArgs": []
  }
};

const selfLoopArtifacts = (holeChecks: SynthesisResponse["hole_checks"]): BrowserSynthesisArtifacts => ({
  response: {
    code: [],
    composed_method_code: "f() {\n    const hole0 = this.f_h0();\n    hole0.next = hole0;\n}",
    hole_checks: holeChecks,
    individual_codes: [],
  },
  task_json: JSON.stringify([selfLoopTask]),
});

describe("browser validation with typed hole checks", () => {
  it("adopts a method whose helper returns its output example", async () => {
    const response = await handleSynthesisMessage(
      async () => selfLoopArtifacts([{ helper: "f_h0", expected: [{ ref: "r" }] }]),
      selfLoopRequest,
    );
    expect(response.validation).toEqual({ status: "passed", checked_demonstrations: 1, error: null });
    expect(response.code).toEqual(["f_h0() { return this; }"]);
  });

  it("rejects a helper whose result differs from its output example", async () => {
    const response = await handleSynthesisMessage(
      async () => selfLoopArtifacts([{ helper: "f_h0", expected: [{ ref: "a" }] }]),
      selfLoopRequest,
    );
    expect(response.validation?.status).toBe("failed");
    expect(response.validation?.error).toContain("Hole f_h0 did not reproduce its output example");
    expect(response.composed_method_code).toBeNull();
  });
});

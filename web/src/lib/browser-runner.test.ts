import { describe, expect, it, vi } from "vitest";
import { handleSynthesisMessage, type BrowserSynthesisArtifacts, type SynthesisRequest } from "./browser-runner";

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
  it("produces synthesized code without using fetch", async () => {
    const fetchSpy = vi.fn();
    vi.stubGlobal("fetch", fetchSpy);

    const response = await handleSynthesisMessage(async () => sampleArtifacts, sampleRequest, {});

    expect(fetchSpy).not.toHaveBeenCalled();
    expect(response.composed_method_code).toContain("append_f");
    expect(response.escher_results?.[0]?.success).toBe(true);
    expect(response.code[0]).toContain("tail_ref()");
    expect(response.list_environment_info).toContain("no network transport");
  });
});

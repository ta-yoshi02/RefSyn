import type { SynthesisRequest } from "../lib/browser-runner";

export const appendSample: SynthesisRequest = {
  method_calls: [
    {
      callLabel: "append",
      contextSensitiveID: "call1",
      receiverObject: "main-new1",
      methodName: "append",
      arguments: [26],
      argumentTypes: ["Int"],
      argumentNames: ["arg"],
      operations: [
        { editType: "addNode", id: "__temp1", isLiteral: false, label: "Node" },
        { editType: "addNode", id: "__temp2", isLiteral: true, label: "0", type: "string" },
        { editType: "addEdge", from: "__temp1", label: "val", to: "__temp2" },
        { editType: "addEdge", from: "main-new1", label: "next", to: "__temp1" },
      ],
    },
    {
      callLabel: "append",
      contextSensitiveID: "call2",
      receiverObject: "main-new1",
      methodName: "append",
      arguments: [10],
      argumentTypes: ["Int"],
      argumentNames: ["arg"],
      operations: [
        { editType: "addNode", id: "__temp3", isLiteral: false, label: "Node" },
        { editType: "addNode", id: "__temp4", isLiteral: true, label: "3", type: "string" },
        { editType: "addEdge", from: "__temp3", label: "val", to: "__temp4" },
        { editType: "addEdge", from: "__temp1", label: "next", to: "__temp3" },
      ],
    },
  ],
  vis_graph: {
    nodes: [
      { id: "main-new1", is_literal: false, label: "Node" },
      { id: "main-new1-val", is_literal: true, label: "2" },
      { id: "__temp1", is_literal: false, label: "Node" },
      { id: "__temp1-val", is_literal: true, label: "0" },
      { id: "__Variable-lst", is_literal: false, label: "__Variable-lst" },
      { id: "__RectForVariable__", is_literal: false, label: "def var" },
    ],
    edges: [
      { from: "main-new1", to: "main-new1-val", label: "val" },
      { from: "__temp1", to: "__temp1-val", label: "val" },
      { from: "main-new1", to: "__temp1", label: "next" },
      { from: "__Variable-lst", to: "main-new1", label: "lst" },
    ],
  },
};

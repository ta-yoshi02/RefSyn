import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const directory = path.dirname(fileURLToPath(import.meta.url));
const payload = JSON.parse(fs.readFileSync(path.join(directory, "payload.json"), "utf8"));

class Obj {
  constructor() {
    this.f = undefined;
    this.g = null;
  }
}
globalThis.Obj = Obj;

function installMethod(source) {
  const match = source.match(/^([A-Za-z_$][\w$]*)\s*\(([^)]*)\)\s*\{([\s\S]*)\}$/);
  if (!match) throw new Error(`cannot parse method: ${source}`);
  const [, name, params, body] = match;
  Obj.prototype[name] = new Function(params, body);
}

function buildGraph(graph) {
  const objects = new Map();
  for (const node of graph.nodes) {
    objects.set(node.id, node.isLiteral ? Number(node.label) : new Obj());
  }
  for (const edge of graph.edges) {
    if (edge.label !== "f" && edge.label !== "g") continue;
    const from = objects.get(edge.from);
    if (from instanceof Obj) from[edge.label] = objects.get(edge.to) ?? null;
  }
  return objects;
}

function replayOperations(call, idMapping) {
  const graph = structuredClone(call.precondGraph);
  for (const raw of call.operations) {
    const operation = { ...raw };
    for (const key of ["from", "to", "oldTo", "newTo"]) {
      if (
        operation[key]
        && !graph.nodes.some((node) => node.id === operation[key])
        && idMapping[operation[key]]
      ) {
        operation[key] = idMapping[operation[key]];
      }
    }
    if (operation.editType === "addNode") {
      graph.nodes.push(operation);
    } else if (operation.editType === "addEdge") {
      graph.edges.push({
        from: operation.from,
        to: operation.to,
        label: operation.label,
      });
    } else if (operation.editType === "editEdgeReference") {
      const edge = graph.edges.find(
        (candidate) => candidate.from === operation.from
          && candidate.label === operation.label,
      );
      if (!edge) throw new Error("edge to edit was not found");
      edge.to = operation.newTo;
    } else if (operation.editType === "deleteEdge") {
      graph.edges = graph.edges.filter(
        (candidate) => candidate.from !== operation.from
          || candidate.label !== operation.label,
      );
    } else if (operation.editType !== "addVariable") {
      throw new Error(`unsupported operation: ${operation.editType}`);
    }
  }
  return graph;
}

function observe(head) {
  const values = [];
  const seen = new Set();
  let current = head;
  while (current !== null) {
    if (seen.has(current)) return { values, acyclic: false };
    seen.add(current);
    values.push(current.f);
    current = current.g;
  }
  return { values, acyclic: true };
}

function sameObservation(actual, expected) {
  return actual.acyclic === expected.acyclic
    && JSON.stringify(actual.values) === JSON.stringify(expected.values);
}

const idMapping = Object.assign({}, ...payload.method_calls.map((call) => call.idMapping));
const responses = [];

for (let run = 1; run <= 3; run += 1) {
  const response = JSON.parse(
    fs.readFileSync(path.join(directory, `response-${run}.json`), "utf8"),
  );
  for (const helper of response.code) installMethod(helper);
  installMethod(response.composed_method_code);

  const records = payload.method_calls.map((call) => {
    const before = buildGraph(call.precondGraph);
    const expectedGraph = buildGraph(replayOperations(call, idMapping));
    const expected = observe(expectedGraph.get(call.receiverObject));
    let actual = null;
    let error = null;
    try {
      const head = before.get(call.receiverObject);
      head[call.methodName](...call.arguments);
      actual = observe(head);
    } catch (caught) {
      error = String(caught);
    }
    return {
      arguments: call.arguments,
      expected,
      actual,
      error,
      passed: error === null && sameObservation(actual, expected),
    };
  });

  responses.push({
    run,
    all_helpers_synthesized: response.escher_results.every((result) => result.success),
    common_operations_count: response.operation_analysis.common_operations_count,
    generated_code: response.composed_method_code,
    records,
    passed: records.filter((record) => record.passed).length,
    failed: records.filter((record) => !record.passed).length,
  });
}

const result = {
  source_commit: "3b323c5ac64852b91c2f3baaaabdb4cd4733daaa",
  observation: "value sequence reachable through g and absence of cycles",
  conclusion: "all helper tasks succeeded, but no generated method matched all three recorded executions",
  responses,
};

const serialized = `${JSON.stringify(result, null, 2)}\n`;
const outputOption = process.argv.indexOf("--out");
if (outputOption >= 0) {
  const outputPath = process.argv[outputOption + 1];
  if (!outputPath) throw new Error("--out requires a path");
  fs.writeFileSync(outputPath, serialized);
}
process.stdout.write(serialized);

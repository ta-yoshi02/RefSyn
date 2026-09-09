import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repo = process.env.REFSYN_DIR
  ?? path.resolve(scriptDirectory, "../../..");
const rerun = path.join(
  repo,
  "docs/evaluation_cases/reruns/2026-09-09-null-guard-removal",
);

class Obj {
  constructor(f = undefined, g = null) {
    this.f = f;
    this.g = g;
  }
}
globalThis.Obj = Obj;

function installMethod(source) {
  const match = source.match(/^([A-Za-z_$][\w$]*)\s*\(([^)]*)\)\s*\{([\s\S]*)\}$/);
  if (!match) throw new Error(`cannot parse method: ${source}`);
  const [, name, params, body] = match;
  Obj.prototype[name] = new Function(params, body);
}

function loadGenerated(caseName) {
  const response = JSON.parse(
    fs.readFileSync(path.join(rerun, caseName, "response.json"), "utf8"),
  );
  for (const helper of response.code) installMethod(helper);
  installMethod(response.composed_method_code);
  return response;
}

for (const name of ["setAt", "append", "prepend", "insert_general", "popBack"]) {
  loadGenerated(name);
}

function makeList(values) {
  if (values.length === 0) return null;
  const nodes = values.map((value) => new Obj(value));
  for (let i = 0; i + 1 < nodes.length; i += 1) nodes[i].g = nodes[i + 1];
  return nodes[0];
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

function baseValues(length) {
  return Array.from({ length }, (_, index) => 1000 + length * 20 + index * 3);
}

function sameObservation(actual, expected) {
  return actual.acyclic === expected.acyclic
    && JSON.stringify(actual.values) === JSON.stringify(expected.values);
}

const records = [];

function record(method, input, expected, actual, error = null) {
  records.push({
    method,
    input,
    expected,
    actual,
    error,
    passed: error === null && sameObservation(actual, expected),
  });
}

for (let length = 1; length <= 5; length += 1) {
  const values = baseValues(length);

  for (let index = 0; index < length; index += 1) {
    const assigned = 2000 + length * 20 + index;
    const expectedValues = values.slice();
    expectedValues[index] = assigned;
    const head = makeList(values);
    try {
      head.setAt(index, assigned);
      record("setAt", { values, index, assigned }, { values: expectedValues, acyclic: true }, observe(head));
    } catch (error) {
      record("setAt", { values, index, assigned }, { values: expectedValues, acyclic: true }, null, String(error));
    }
  }

  const appended = 3000 + length;
  {
    const head = makeList(values);
    try {
      head.append(appended);
      record("append", { values, appended }, { values: [...values, appended], acyclic: true }, observe(head));
    } catch (error) {
      record("append", { values, appended }, { values: [...values, appended], acyclic: true }, null, String(error));
    }
  }

  const prepended = 4000 + length;
  {
    const head = makeList(values);
    try {
      const returned = head.prepend(prepended);
      record("prepend", { values, prepended }, { values: [prepended, ...values], acyclic: true }, observe(returned));
    } catch (error) {
      record("prepend", { values, prepended }, { values: [prepended, ...values], acyclic: true }, null, String(error));
    }
  }

  for (let index = 0; index < length; index += 1) {
    const inserted = 5000 + length * 20 + index;
    const expectedValues = values.slice();
    expectedValues.splice(index + 1, 0, inserted);
    const head = makeList(values);
    try {
      head.insert(index, inserted);
      record("insert", { values, index, inserted }, { values: expectedValues, acyclic: true }, observe(head));
    } catch (error) {
      record("insert", { values, index, inserted }, { values: expectedValues, acyclic: true }, null, String(error));
    }
  }
}

for (let length = 2; length <= 6; length += 1) {
  const values = baseValues(length);
  const head = makeList(values);
  try {
    head.popBack();
    record("popBack", { values }, { values: values.slice(0, -1), acyclic: true }, observe(head));
  } catch (error) {
    record("popBack", { values }, { values: values.slice(0, -1), acyclic: true }, null, String(error));
  }
}

const controls = [
  { method: "setAt", values: [7, 7, 7], index: 1, value: 0 },
  { method: "setAt", values: [4, 9, 2], index: 1, value: 4 },
  { method: "insert", values: [10, -5, 10], index: 1, value: 0 },
  { method: "insert", values: [10, -5, 10], index: 0, value: 10 },
];

for (const { method, values, index, value } of controls) {
  const expectedValues = values.slice();
  if (method === "setAt") expectedValues[index] = value;
  else expectedValues.splice(index + 1, 0, value);
  const head = makeList(values);
  try {
    head[method](index, value);
    record(
      `${method}Control`,
      { values, index, value },
      { values: expectedValues, acyclic: true },
      observe(head),
    );
  } catch (error) {
    record(
      `${method}Control`,
      { values, index, value },
      { values: expectedValues, acyclic: true },
      null,
      String(error),
    );
  }
}

const byMethod = {};
for (const item of records) {
  byMethod[item.method] ??= { total: 0, passed: 0, failed: 0 };
  byMethod[item.method].total += 1;
  byMethod[item.method][item.passed ? "passed" : "failed"] += 1;
}

const result = {
  source_commit: "3b323c5ac64852b91c2f3baaaabdb4cd4733daaa",
  source_directory: "docs/evaluation_cases/reruns/2026-09-09-null-guard-removal",
  observation: "value sequence reachable through g and absence of cycles",
  generation_rule: {
    setAt: "list lengths 1..5; every valid index; held-out values",
    append: "list lengths 1..5; one held-out appended value per length",
    prepend: "list lengths 1..5; one held-out prepended value per length; observe returned head",
    insert: "list lengths 1..5; every valid predecessor index; held-out values",
    popBack: "list lengths 2..6; singleton and empty lists excluded from the claimed domain",
  },
  summary: byMethod,
  total: records.length,
  passed: records.filter((item) => item.passed).length,
  failed: records.filter((item) => !item.passed).length,
  records,
};

const serialized = `${JSON.stringify(result, null, 2)}\n`;
const outputOption = process.argv.indexOf("--out");
if (outputOption >= 0) {
  const outputPath = process.argv[outputOption + 1];
  if (!outputPath) throw new Error("--out requires a path");
  fs.writeFileSync(outputPath, serialized);
}
process.stdout.write(serialized);

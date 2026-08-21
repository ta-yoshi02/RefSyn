#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  parseJsonSynthesisSpec,
  prepareJsonSynthesisJob,
} from "../external/escher-ts/dist/components/user-friendly-json.js";
import { executeTerm, showTerm } from "../external/escher-ts/dist/types/term.js";
import { equalTermValue, valueError } from "../external/escher-ts/dist/types/value.js";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const evaluationRoot = path.join(repoRoot, "docs", "evaluation_cases");
const outputDir = path.join(evaluationRoot, "nonseparated_general_components");

const sources = [
  ["setAt", "setAt"],
  ["append", "append"],
  ["prepend", "prepend"],
  ["insert_general", "insert"],
  ["popBack", "popBack"],
];

const addedComponents = [
  { name: "storeAtRef", kind: "libraryRef", ref: "storeAtRef" },
  { name: "freshRefFromHeap", kind: "libraryRef", ref: "freshRefFromHeap" },
];

const variable = (name) => ({ kind: "var", name });
const component = (name, ...args) => ({ kind: "component", name, args });
const append = (heap, value) => component("insert", heap, component("length", heap), value);

const vars = {
  thisRef: variable("thisRef"),
  nodeHeap: variable("nodeHeap"),
  fHeap: variable("fHeap"),
  gHeap: variable("gHeap"),
  arg0: variable("arg0"),
  arg1: variable("arg1"),
};

const nullFromInput = component(
  "nthNextRef",
  vars.thisRef,
  vars.nodeHeap,
  vars.gHeap,
  component("length", vars.nodeHeap),
);

const setAtF = component(
  "storeAtRef",
  vars.fHeap,
  component("nthNextRef", vars.thisRef, vars.nodeHeap, vars.gHeap, vars.arg0),
  vars.arg1,
);
const appendF = append(vars.fHeap, vars.arg0);
const appendG = append(
  component(
    "storeAtRef",
    vars.gHeap,
    component("last_ptr", vars.thisRef, vars.gHeap),
    component("freshRefFromHeap", vars.nodeHeap),
  ),
  nullFromInput,
);
const prependF = append(vars.fHeap, vars.arg0);
const prependG = append(vars.gHeap, vars.thisRef);
const prependReturn = component("freshRefFromHeap", vars.nodeHeap);
const insertF = append(vars.fHeap, vars.arg1);
const insertG = append(
  component(
    "storeAtRef",
    vars.gHeap,
    component("nthNextRef", vars.thisRef, vars.nodeHeap, vars.gHeap, vars.arg0),
    component("freshRefFromHeap", vars.nodeHeap),
  ),
  component("nthNextRef", vars.thisRef, vars.nodeHeap, vars.gHeap, component("inc", vars.arg0)),
);
const popBackG = component(
  "storeAtRef",
  vars.gHeap,
  component("penultimateRef", vars.thisRef, vars.gHeap),
  nullFromInput,
);

const witnesses = new Map([
  ["setAt-fixed-fHeap", setAtF],
  ["setAt-fixed-gHeap", vars.gHeap],
  ["setAt-fixed-env-pair", component("pairHeap", setAtF, vars.gHeap)],
  ["append-fixed-fHeap", appendF],
  ["append-fixed-gHeap", appendG],
  ["append-fixed-env-pair", component("pairHeap", appendF, appendG)],
  ["prepend-fixed-fHeap", prependF],
  ["prepend-fixed-gHeap", prependG],
  ["prepend-fixed-returnRef", prependReturn],
  ["prepend-fixed-env-pair", component("pairHeap", prependF, prependG)],
  ["insert-fixed-fHeap", insertF],
  ["insert-fixed-gHeap", insertG],
  ["insert-fixed-env-pair", component("pairHeap", insertF, insertG)],
  ["popBack-fixed-fHeap", vars.fHeap],
  ["popBack-fixed-gHeap", popBackG],
  ["popBack-fixed-env-pair", component("pairHeap", vars.fHeap, popBackG)],
]);

const decisiveTaskNames = new Set([
  "setAt-fixed-env-pair",
  "append-fixed-env-pair",
  "prepend-fixed-returnRef",
  "prepend-fixed-env-pair",
  "insert-fixed-env-pair",
  "popBack-fixed-env-pair",
]);

const termCost = (term) => {
  if (term.kind === "var") {
    return 1;
  }
  if (term.kind === "component") {
    return 1 + term.args.reduce((sum, arg) => sum + termCost(arg), 0);
  }
  return 1 + termCost(term.condition) + termCost(term.thenBranch) + termCost(term.elseBranch);
};

const addGeneralComponents = (task) => {
  const existing = new Set(task.components.map((entry) => entry.name));
  return {
    ...task,
    components: [
      ...task.components,
      ...addedComponents.filter((entry) => !existing.has(entry.name)),
    ],
  };
};

const allTasks = [];
for (const [directory, method] of sources) {
  const sourcePath = path.join(evaluationRoot, directory, "baseline_fixed_id_task.json");
  const tasks = JSON.parse(fs.readFileSync(sourcePath, "utf8"));
  for (const task of tasks) {
    allTasks.push({ ...addGeneralComponents(task), evaluationMethod: method });
  }
}

const witnessReport = allTasks.map((task) => {
  const witness = witnesses.get(task.name);
  if (witness === undefined) {
    throw new Error(`No hand-written witness for ${task.name}`);
  }
  const spec = parseJsonSynthesisSpec(JSON.stringify(task));
  const job = prepareJsonSynthesisJob(spec);
  const exampleResults = job.examples.map(([inputs, expected], index) => {
    const inputByName = new Map(job.inputNames.map((name, inputIndex) => [name, inputs[inputIndex]]));
    const actual = executeTerm(
      (name) => inputByName.get(name) ?? valueError,
      job.env,
      witness,
    );
    return { index, matches: equalTermValue(actual, expected) };
  });
  return {
    task: task.name,
    method: task.evaluationMethod,
    witness: showTerm(witness),
    witness_cost: termCost(witness),
    examples: exampleResults.length,
    all_examples_match: exampleResults.every((result) => result.matches),
    example_results: exampleResults,
  };
});

if (!witnessReport.every((entry) => entry.all_examples_match)) {
  throw new Error("At least one hand-written witness does not match its saved examples");
}

fs.mkdirSync(outputDir, { recursive: true });
fs.writeFileSync(path.join(outputDir, "tasks.json"), `${JSON.stringify(allTasks, null, 2)}\n`);
fs.writeFileSync(
  path.join(outputDir, "decisive_tasks.json"),
  `${JSON.stringify(allTasks.filter((task) => decisiveTaskNames.has(task.name)), null, 2)}\n`,
);
fs.writeFileSync(
  path.join(outputDir, "witness_check.json"),
  `${JSON.stringify(
    {
      generated_at: new Date().toISOString(),
      added_components: addedComponents.map((entry) => entry.name),
      all_witnesses_match: true,
      maximum_witness_cost: Math.max(...witnessReport.map((entry) => entry.witness_cost)),
      tasks: witnessReport,
    },
    null,
    2,
  )}\n`,
);

process.stdout.write(
  `${JSON.stringify({
    output_dir: outputDir,
    tasks: allTasks.length,
    decisive_tasks: allTasks.filter((task) => decisiveTaskNames.has(task.name)).length,
    maximum_witness_cost: Math.max(...witnessReport.map((entry) => entry.witness_cost)),
    all_witnesses_match: true,
  })}\n`,
);

import { readFileSync } from "node:fs";
import { afterEach, beforeAll, describe, expect, it } from "vitest";
import { initSync, synthesize_browser } from "../../pkg/refsyn.js";
import { compileRefsynMethod, runRefsynTasks, type RefsynTaskMeta } from "../../../runtime/refsyn-escher-adapter.mjs";
import { parseJsonSynthesisSpec, prepareJsonSynthesisJob } from "../../../external/escher-ts/dist/components/user-friendly-json.js";
import { executeTerm, showTerm, type Term } from "../../../external/escher-ts/dist/types/term.js";
import type { TermValue } from "../../../external/escher-ts/dist/types/value.js";

// A later hole's inputs carry the earlier holes' output examples, not the earlier expressions'
// results. The two agree when the returned expressions reproduce every example (C1) and the
// translated helpers leave the state the later holes read unchanged (C5). These tests check both
// on the tasks the WASM core builds for the demonstrations below, and only on those inputs.
const fixtures = ["paper_insert", "booleans/set_flag", "booleans/copy_flag", "booleans/set_flag_number_literal"];

type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
type RawTask = {
  name: string;
  examples: [Json[], Json][];
  signature: { returnType: string };
  refsynMeta: RefsynTaskMeta;
};

const tasksOf = (fixture: string): RawTask[] => {
  const request = readFileSync(new URL(`../../../tests/data/${fixture}.json`, import.meta.url), "utf8");
  const artifacts = JSON.parse(synthesize_browser(request, JSON.stringify({ trace: false })));
  return JSON.parse(artifacts.task_json);
};

const jobOf = (task: RawTask) => prepareJsonSynthesisJob(parseJsonSynthesisSpec(JSON.stringify(task)));

// The adapter reports the expression only as text; read it back so that the expression checked
// is the one escher-ts returned, not one re-synthesized here under copied settings. This relies on
// showTerm's format; an expression it cannot read back exactly fails the test.
const parseTerm = (text: string): Term => {
  let at = 0;
  const expect = (token: string) => {
    if (!text.startsWith(token, at)) throw new Error(`expected '${token}' at ${at} in ${text}`);
    at += token.length;
  };
  const name = (stop: RegExp) => {
    const start = at;
    while (at < text.length && !stop.test(text[at]!)) at += 1;
    return text.slice(start, at);
  };
  const term = (): Term => {
    if (text.startsWith("if ", at)) {
      at += 3;
      const condition = term();
      expect(" then ");
      const thenBranch = term();
      expect(" else ");
      return { kind: "if", condition, thenBranch, elseBranch: term() };
    }
    if (text[at] === "@") {
      at += 1;
      return { kind: "var", name: name(/[,) ]/) };
    }
    const component = name(/\(/);
    expect("(");
    const args: Term[] = [];
    while (text[at] !== ")") {
      if (args.length > 0) expect(", ");
      args.push(term());
    }
    expect(")");
    return { kind: "component", name: component, args };
  };
  const parsed = term();
  if (at !== text.length || showTerm(parsed) !== text) throw new Error(`could not read back ${text}`);
  return parsed;
};

const isRef = (value: Json): value is { ref: number } =>
  typeof value === "object" && value !== null && !Array.isArray(value) && typeof value.ref === "number";

// Integers, booleans and references never stand for one another; a reference must name the
// same heap entry (-1 is null).
const sameValue = (expected: Json, actual: TermValue): boolean => {
  if (typeof expected === "number") return actual.tag === "int" && actual.value === expected;
  if (typeof expected === "boolean") return actual.tag === "bool" && actual.value === expected;
  if (isRef(expected)) return actual.tag === "ref" && actual.value === expected.ref;
  return false;
};

const evaluateOnExamples = (task: RawTask, term: Term) => {
  const job = jobOf(task);
  return job.examples.map(([inputs], index) => {
    const env = new Map(job.inputNames.map((name, i) => [name, inputs[i]!]));
    const actual = executeTerm((name) => {
      const value = env.get(name);
      if (value === undefined) throw new Error(`${name} is not an input of ${task.name}`);
      return value;
    }, job.env, term);
    const expected = task.examples[index]![1];
    return { example: index, expected, actual, ok: sameValue(expected, actual) };
  });
};

const solved = (task: RawTask) => {
  const [outcome] = runRefsynTasks([task]);
  if (!outcome.success || outcome.rendered === null) throw new Error(`${task.name}: ${outcome.error}`);
  const body = outcome.rendered.slice(outcome.rendered.indexOf("=\n  ") + 4);
  return { term: parseTerm(body), compiledJs: outcome.compiled_js as string };
};

beforeAll(() => {
  initSync({ module: readFileSync(new URL("../../pkg/refsyn_bg.wasm", import.meta.url)) });
});

describe("C1: the returned expression reproduces every output example", () => {
  for (const fixture of fixtures) {
    it(fixture, () => {
      const tasks = tasksOf(fixture);
      const demonstrations = JSON.parse(
        readFileSync(new URL(`../../../tests/data/${fixture}.json`, import.meta.url), "utf8"),
      ).method_calls.length;
      for (const task of tasks) {
        expect(task.examples).toHaveLength(demonstrations);
        const checks = evaluateOnExamples(task, solved(task).term);
        expect(checks.filter((check) => !check.ok), task.name).toEqual([]);
      }
    });
  }

  it("covers integers, booleans, references and a conditional", () => {
    const kinds = new Set<string>();
    for (const fixture of fixtures) {
      for (const task of tasksOf(fixture)) {
        kinds.add(task.signature.returnType.startsWith("Ref[") ? "Ref" : task.signature.returnType);
        if (solved(task).term.kind === "if") kinds.add("if");
      }
    }
    expect([...kinds].sort()).toEqual(["Bool", "Int", "Ref", "if"]);
  });

  it("detects expressions that miss an example", () => {
    const [h0, h1, h2] = tasksOf("paper_insert");
    const v = (name: string): Term => ({ kind: "var", name });
    const failing = (task: RawTask, term: Term) =>
      evaluateOnExamples(task, term).filter((check) => !check.ok).map((check) => check.example);
    // The index instead of the value, the receiver instead of the i-th node (right only for
    // insert(0, _)), the node itself instead of its successor.
    expect(failing(h0!, v("i"))).toEqual([0, 1, 2]);
    expect(failing(h1!, v("thisRef"))).toEqual([1, 2]);
    expect(failing(h2!, v("hole1"))).toEqual([0, 1, 2]);
  });

  it("tells integers, booleans and references apart", () => {
    const int = (value: number): TermValue => ({ tag: "int", value });
    const bool = (value: boolean): TermValue => ({ tag: "bool", value });
    const ref = (value: number): TermValue => ({ tag: "ref", value });
    expect(sameValue(1, int(1))).toBe(true);
    expect(sameValue(true, bool(true))).toBe(true);
    expect(sameValue({ ref: 2 }, ref(2))).toBe(true);
    expect(sameValue({ ref: -1 }, ref(-1))).toBe(true);
    expect(sameValue(1, bool(true))).toBe(false);
    expect(sameValue(true, int(1))).toBe(false);
    expect(sameValue(0, ref(0))).toBe(false);
    expect(sameValue({ ref: 0 }, int(0))).toBe(false);
    expect(sameValue({ ref: 2 }, ref(1))).toBe(false);
    expect(sameValue({ ref: -1 }, ref(0))).toBe(false);
    expect(sameValue(0, { tag: "error" })).toBe(false);
  });
});

// Every object reachable from a helper's inputs is a Proxy that logs writes, including writes
// later undone. All references, including `this`, `next` and the arguments, are proxies, so
// identity among them is kept and a cycle is a cycle of proxies. Not seen: writes to objects the
// helper did not receive (globals, prototypes, statics, its own locals such as the Set that
// cycle detection uses) and anything on inputs other than the demonstrated ones.
type Guarded = { objects: object[]; self: object; args: unknown[]; writes: string[] };

const guardedInputs = (task: RawTask, example: number, helperClass: new () => object, cycle = false): Guarded => {
  const meta = task.refsynMeta;
  const names = jobOf(task).inputNames;
  const inputs = task.examples[example]![0];
  const input = (name: string) => inputs[names.indexOf(name)]!;
  const heap = input(meta.classHeapName) as { object: { fields: Record<string, Json> } }[];
  const writes: string[] = [];
  const targets = heap.map(() => Object.create(helperClass.prototype) as Record<string, unknown>);
  const objects = targets.map((target, index) => {
    const log = (what: string, key: PropertyKey) => writes.push(`${what} #${index}.${String(key)}`);
    return new Proxy(target, {
      set: (t, key, value) => (log("set", key), Reflect.set(t, key, value)),
      defineProperty: (t, key, descriptor) => (log("define", key), Reflect.defineProperty(t, key, descriptor)),
      deleteProperty: (t, key) => (log("delete", key), Reflect.deleteProperty(t, key)),
      setPrototypeOf: (t, proto) => (log("setPrototypeOf", ""), Reflect.setPrototypeOf(t, proto)),
      preventExtensions: (t) => (log("preventExtensions", ""), Reflect.preventExtensions(t)),
    });
  });
  const object = (slot: Json) => (isRef(slot) ? (slot.ref === -1 ? null : objects[slot.ref]) : undefined);
  heap.forEach(({ object: { fields } }, index) => {
    for (const field of meta.pointerFields) targets[index]![field] = object(fields[field]!);
    for (const field of meta.boolFields ?? []) targets[index]![field] = typeof fields[field] === "boolean" ? fields[field] : undefined;
    for (const field of meta.valueFields) {
      const slot = fields[field]!;
      const stored = isRef(slot) ? (input(meta.fieldHeapNames[field]!) as Json[])[slot.ref] : undefined;
      targets[index]![field] = typeof stored === "number" ? stored : undefined;
    }
  });
  if (cycle) {
    // Close the list into a ring so that the traversals meet a revisit.
    const last = targets.findIndex((target) => meta.pointerFields.every((field) => target[field] === null));
    targets[last]![meta.pointerFields[0]!] = objects[0];
  }
  const args = meta.explicitArgs.map(({ name }) => {
    const value = input(name);
    return isRef(value) ? object(value) : value;
  });
  return { objects, self: object(input(meta.thisRefName)) as object, args, writes };
};

const helperClassOf = (code: string) => new Function(`return class Helper { ${code} };`)() as new () => object;

const runHelper = (task: RawTask, code: string, example: number, cycle = false) => {
  const Helper = helperClassOf(code);
  const { objects, self, args, writes } = guardedInputs(task, example, Helper, cycle);
  let result: unknown;
  let failed = false;
  try {
    result = (self as Record<string, (...a: unknown[]) => unknown>)[task.refsynMeta.jsMethodName]!(...args);
  } catch {
    failed = true;
  }
  return { objects, result, failed, writes };
};

// The local Set of a traversal is written; count those writes to show they are not mistaken
// for writes to the inputs.
let setAdds = 0;
const realAdd = Set.prototype.add;
const countSetAdds = () => {
  setAdds = 0;
  Set.prototype.add = function (this: Set<unknown>, value: unknown) {
    setAdds += 1;
    return realAdd.call(this, value);
  };
};
afterEach(() => {
  Set.prototype.add = realAdd;
});

describe("C5: translated helpers leave the state the later holes read unchanged", () => {
  for (const fixture of fixtures) {
    it(fixture, () => {
      for (const task of tasksOf(fixture)) {
        const { compiledJs } = solved(task);
        task.examples.forEach(([, expected], example) => {
          const { objects, result, failed, writes } = runHelper(task, compiledJs, example);
          expect(writes, `${task.name} #${example}`).toEqual([]);
          // The inputs were built right: the helper returns its output example on them.
          expect(failed).toBe(false);
          expect(isRef(expected) ? (result === null ? -1 : objects.indexOf(result as object)) : result).toEqual(
            isRef(expected) ? expected.ref : expected,
          );
        });
      }
    });
  }

  it("translations of the traversal and field components give their outcome and write only their own locals", () => {
    const task = tasksOf("paper_insert")[2]!;
    const job = jobOf(task);
    const v = (name: string): Term => ({ kind: "var", name });
    const c = (name: string, ...args: Term[]): Term => ({ kind: "component", name, args });
    const [T, N, X, V] = [v("thisRef"), v("nodeHeap"), v("nextHeap"), v("valHeap")];
    // On the ring, the traversals that detect a revisit fail; every other term reads only the
    // first nodes and keeps its outcome on the list.
    const failsOnRing = new Set(["last_ptr", "penultimateRef", "findByValueRef"]);
    const terms = [
      c("nextOf", T, N, X), c("nthNextRef", T, N, X, v("i")), c("last_ptr", T, X), c("penultimateRef", T, X),
      c("findByValueRef", T, N, X, V, v("arg")), c("hasNext", T, N, X), c("hasVal", T, N, V),
      c("loadInt", V, c("valOf", T, N, V)), c("equal", T, c("nextOf", T, N, X)), c("isNull", c("nextOf", v("hole1"), N, X)),
      { kind: "if", condition: c("hasNext", T, N, X), thenBranch: c("nextOf", T, N, X), elseBranch: T } as Term,
    ];
    const outcomeOf = (objects: object[], result: unknown, failed: boolean) =>
      failed ? "failure" : result === null ? "ref -1" : typeof result === "object" ? `ref ${objects.indexOf(result)}` : `${typeof result} ${result}`;
    const escherOutcome = (term: Term, example: number) => {
      const env = new Map(job.inputNames.map((name, i) => [name, job.examples[example]![0][i]!]));
      const value = executeTerm((name) => env.get(name)!, job.env, term);
      return value.tag === "error" ? "failure" : value.tag === "int" ? `number ${value.value}` : value.tag === "bool" ? `boolean ${value.value}` : value.tag === "ref" ? `ref ${value.value}` : `unexpected ${value.tag}`;
    };
    const seen = { failure: 0, value: 0 };
    let traversed = 0;
    for (const term of terms) {
      const code = compileRefsynMethod(task, job, term);
      const head = term.kind === "component" ? term.name : "if";
      for (let example = 0; example < task.examples.length; example += 1) {
        const expected = escherOutcome(term, example);
        for (const cycle of [false, true]) {
          countSetAdds();
          const { objects, result, failed, writes } = runHelper(task, code, example, cycle);
          Set.prototype.add = realAdd;
          const label = `${showTerm(term)} #${example}${cycle ? " (ring)" : ""}`;
          expect(outcomeOf(objects, result, failed), label).toBe(cycle && failsOnRing.has(head) ? "failure" : expected);
          expect(writes, label).toEqual([]);
          seen[failed ? "failure" : "value"] += 1;
          traversed += setAdds;
        }
      }
    }
    // Both outcomes occur, so neither a helper that always throws nor one that never fails passes.
    expect(seen.failure).toBeGreaterThan(0);
    expect(seen.value).toBeGreaterThan(0);
    expect(traversed).toBeGreaterThan(0);
  });

  it("detects a write that is undone before the helper returns", () => {
    const task = tasksOf("paper_insert")[2]!;
    const restoring = `insert_h2(i, arg, created0, hole0, hole1) {
      const saved = hole1.next; hole1.next = null; delete hole1.next; hole1.next = saved; return hole1.next; }`;
    // Fields by name, references by heap index. The re-added property moves to the end, so the
    // comparison ignores property order, as a structural comparison of states does.
    const snapshot = (objects: object[]) =>
      objects.map((o) =>
        Object.entries(o)
          .sort(([a], [b]) => a.localeCompare(b))
          .map(([k, value]) => [k, typeof value === "object" && value !== null ? objects.indexOf(value) : value]),
      );
    const plain = runHelper(task, solved(task).compiledJs, 0);
    const undone = runHelper(task, restoring, 0);
    // The result and the state afterwards agree with the real helper's, so comparing final states
    // cannot tell the two apart.
    expect(undone.objects.indexOf(undone.result as object)).toBe(plain.objects.indexOf(plain.result as object));
    expect(snapshot(undone.objects)).toEqual(snapshot(plain.objects));
    expect(undone.writes).toEqual(["set #0.next", "delete #0.next", "set #0.next"]);
  });

  it("detects a write that a later write of the method overwrites", () => {
    const request = readFileSync(new URL("../../../tests/data/paper_insert.json", import.meta.url), "utf8");
    const { response } = JSON.parse(synthesize_browser(request, JSON.stringify({ trace: false })));
    const tasks = tasksOf("paper_insert");
    const helpers = tasks.map((task) => solved(task).compiledJs);
    // created0.next is written again by the method itself, after every hole.
    const overwritten = `insert_h2(i, arg, created0, hole0, hole1) { created0.next = this; return hole1.next; }`;
    const listAfterInsert = (h2: string) => {
      const Node = new Function(`return class Node {
        constructor() { this.val = undefined; this.next = null; }
        ${helpers[0]} ${helpers[1]} ${h2} ${response.composed_method_code} };`)();
      const head = new Node();
      let node = head;
      for (const val of [39, 27, 53]) {
        node.next = new Node();
        node = node.next;
        node.val = val;
      }
      head.next.insert(0, 89);
      const out = [];
      for (let n = head.next; n !== null; n = n.next) out.push(n.val);
      return out;
    };
    expect(listAfterInsert(overwritten)).toEqual(listAfterInsert(helpers[2]!));
    expect(runHelper(tasks[2]!, overwritten, 0).writes).toEqual(["set #3.next"]);
  });
});

// Runs an adopted method and a reference implementation on the same inputs and compares the
// states they leave. Reads {reference, className, method, helpers, composedMethod} from stdin.
//
// The reference module exports `classSource` (every class, with the reference method) and
// `cases`, each `{ name, build(classes) }` returning `{ receiver, args, roots }`. Objects of the
// input must be reachable from `roots`, the receiver or the arguments; detached ones go in roots.
// Objects are named by the order a fixed traversal meets them, so creation order does not
// matter, and an object the method creates but leaves unreachable is not observed. Each call
// is stopped after CALL_TIMEOUT_MS, and a reference that does not finish is an error.
//
// A state is supported only under the conditions Kanon's captureValidationState records:
// every object is an instance of one of the classes, every own property has a string key and
// is a data property that is enumerable, writable and configurable, and every value is
// undefined, null, a string, a boolean, a finite number other than -0 or such an object. It is
// also required that objects stay extensible, which Kanon does not record. `snapshot` applies
// the conditions to every object it observes, before and after the call: an unsupported state
// is an error on the reference's side and a mismatch on the candidate's.
import fs from "node:fs";
import { pathToFileURL } from "node:url";
import vm from "node:vm";

const CALL_TIMEOUT_MS = 2000;

const { reference, className, method, helpers, composedMethod } = JSON.parse(fs.readFileSync(0, "utf8"));
const spec = await import(pathToFileURL(reference).href);
const classNames = [...spec.classSource.matchAll(/^\s*class\s+([A-Za-z_$][\w$]*)/gm)].map((m) => m[1]);
if (!classNames.includes(className)) throw new Error(`classSource does not define ${className}`);

const loadClasses = () => new Function(`${spec.classSource}\nreturn { ${classNames.join(", ")} };`)();
const referenceWorld = () => loadClasses();
const generatedWorld = () => {
  const classes = loadClasses();
  const prototype = classes[className].prototype;
  delete prototype[method];
  const members = new Function(...classNames, `return ({${[...helpers, composedMethod].join(",\n")}});`)(
    ...classNames.map((name) => classes[name]));
  if (!Object.hasOwn(members, method)) throw new Error(`the adopted code does not define ${method}`);
  Object.defineProperties(prototype, Object.getOwnPropertyDescriptors(members));
  return classes;
};

const isObject = (value) => value !== null && (typeof value === "object" || typeof value === "function");
const unsupported = (what) => {
  throw new Error(`unsupported state: ${what}`);
};

// The state reachable from the input and the outcome, with `existing` named first in order.
function snapshot(classes, input, outcome, existing) {
  const ids = new Map(existing.map((object, i) => [object, `existing${i}`]));
  const queue = [...existing];
  let created = 0;
  const name = (object) => {
    if (!ids.has(object)) {
      ids.set(object, `created${created++}`);
      queue.push(object);
    }
    return ids.get(object);
  };
  const encode = (value, where) => {
    if (value === undefined) return { undefined: true };
    if (value === null || typeof value === "string" || typeof value === "boolean") return value;
    if (typeof value === "number") {
      if (Number.isFinite(value) && !Object.is(value, -0)) return value;
      unsupported(`${where} is ${Object.is(value, -0) ? "-0" : value}`);
    }
    if (isObject(value) && classNames.some((n) => Object.getPrototypeOf(value) === classes[n].prototype)) {
      return { ref: name(value) };
    }
    unsupported(`${where} is ${typeof value === "object" || typeof value === "function"
      ? `${Object.prototype.toString.call(value)}, not an instance of ${classNames.join(", ")}` : `a ${typeof value}`}`);
  };
  const result = {
    roots: Object.fromEntries(Object.keys(input.roots ?? {}).sort().map((key) => [key, encode(input.roots[key], `root ${key}`)])),
    receiver: encode(input.receiver, "the receiver"),
    args: input.args.map((arg, i) => encode(arg, `argument ${i}`)),
    outcome: "returned" in outcome ? { returned: encode(outcome.returned, "the return value") } : outcome,
  };
  const objects = {};
  for (let i = 0; i < queue.length; i += 1) {
    const object = queue[i];
    const id = ids.get(object);
    encode(object, id);
    if (!Object.isExtensible(object)) unsupported(`${id} is not extensible`);
    const keys = Reflect.ownKeys(object);
    const symbol = keys.find((key) => typeof key === "symbol");
    if (symbol) unsupported(`${id} has the symbol key ${String(symbol)}`);
    const fields = keys.sort().map((key) => {
      const d = Object.getOwnPropertyDescriptor(object, key);
      if (!Object.hasOwn(d, "value") || !d.enumerable || !d.writable || !d.configurable) {
        unsupported(`${id}.${key} is not an enumerable, writable and configurable data property`);
      }
      return [key, encode(d.value, `${id}.${key}`)];
    });
    objects[id] = { class: classNames.find((n) => classes[n].prototype === Object.getPrototypeOf(object)), fields };
  }
  result.objects = objects;
  return result;
}

function run(classes, testCase, isReference) {
  const input = testCase.build(classes);
  // Every object of the input, followed through every own data property without calling
  // getters, so that `snapshot` checks the detached and unsupported ones after the call too.
  const existing = [];
  const seen = new Set();
  const visit = (value) => {
    if (!isObject(value) || seen.has(value)) return;
    seen.add(value);
    existing.push(value);
    for (const key of Reflect.ownKeys(value).sort((a, b) => String(a).localeCompare(String(b)))) {
      const d = Object.getOwnPropertyDescriptor(value, key);
      if (Object.hasOwn(d, "value")) visit(d.value);
    }
  };
  for (const key of Object.keys(input.roots ?? {}).sort()) visit(input.roots[key]);
  visit(input.receiver);
  input.args.forEach(visit);
  snapshot(classes, input, { returned: undefined }, existing);
  let outcome;
  try {
    const call = () => input.receiver[method](...input.args);
    outcome = { returned: vm.runInNewContext("call()", { call }, { timeout: CALL_TIMEOUT_MS }) };
  } catch (error) {
    if (error?.code !== "ERR_SCRIPT_EXECUTION_TIMEOUT") {
      outcome = { threw: error?.constructor?.name ?? String(error) };
    } else if (isReference) {
      throw new Error(`the reference did not finish within ${CALL_TIMEOUT_MS} ms: ${testCase.name}`);
    } else {
      outcome = { timedOut: CALL_TIMEOUT_MS };
    }
  }
  return snapshot(classes, input, outcome, existing);
}

const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
const mismatches = [];
for (const testCase of spec.cases) {
  const expected = run(referenceWorld(), testCase, true);
  let actual;
  try {
    actual = run(generatedWorld(), testCase, false);
  } catch (error) {
    actual = { error: String(error?.message ?? error) };
  }
  if (!same(expected, actual)) mismatches.push({ case: testCase.name, expected, actual });
}
process.stdout.write(JSON.stringify({ cases: spec.cases.length, mismatches }));

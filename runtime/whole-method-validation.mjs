// This function is self-contained so Node can execute it in a time-bounded VM.
export function validateWholeMethod({ request, response, taskNames = [] }) {
  const unavailable = (message) => { const error = new Error(message); error.unavailable = true; throw error; };
  let checked = 0;
  try {
    if (!response.composed_method_code) unavailable("No composed method to validate");
    const results = response.escher_results ?? [];
    if (new Set(taskNames).size !== taskNames.length || results.length !== taskNames.length) throw new Error("Hole result set does not match the requested tasks");
    if (results.some((result) => !result.success)) throw new Error("A hole failed synthesis or translation");
    for (const name of taskNames) {
      if (results.filter((result) => result.name === name && result.success).length !== 1) {
        throw new Error(`Missing or duplicate successful hole: ${name}`);
      }
    }
    if ((response.code ?? []).length !== taskNames.length) throw new Error("Not all hole implementations are available");
    const validation = request.validation;
    if (validation?.version !== 2 || !Array.isArray(validation.cases) || !Array.isArray(validation.classes)) {
      unavailable(validation?.error ?? "Typed pre-state and constructor definitions are required (validation.version=2)");
    }
    const calls = request.method_calls;
    if (!Array.isArray(calls) || calls.length === 0 || validation.cases.length !== calls.length) {
      unavailable("Validation must cover every demonstration exactly once");
    }
    if (new Set(calls.map((call) => JSON.stringify([call.methodName, call.receiverClassName]))).size !== 1) {
      unavailable("All demonstrations must target the same method of the same class");
    }
    const identifier = /^[$_\p{ID_Start}][$_‌‍\p{ID_Continue}]*$/u;
    const names = validation.classes.map((entry) => entry.name);
    if (new Set(names).size !== names.length || names.some((name) => typeof name !== "string" || !identifier.test(name))) {
      unavailable("Invalid or duplicate constructor name");
    }
    for (let index = 0; index < calls.length; index++) {
      const call = calls[index];
      const example = validation.cases[index];
      if (example.callLabel !== call.callLabel || example.contextSensitiveID !== call.contextSensitiveID) {
        unavailable("Validation example does not match the demonstration");
      }
      if (!Array.isArray(example.objects) || !Array.isArray(example.arguments)) {
        unavailable("Each demonstration needs objects and arguments");
      }
      const build = () => {
        const allocations = [];
        const track = (constructor) => new Proxy(constructor, {
          construct(target, args, newTarget) {
            const object = Reflect.construct(target, args, newTarget);
            allocations.push(object);
            return object;
          },
        });
        let trackerName = "__track";
        while (names.includes(trackerName)) trackerName += "_";
        const declarations = validation.classes.map(({ name, source }) => {
          if (typeof source !== "string") unavailable(`Missing constructor source: ${name}`);
          return `let ${name} = (${source}); ${name} = ${trackerName}(${name});`;
        }).join("\n");
        const constructors = new Function(trackerName, `"use strict"; ${declarations}\nreturn [${names.join(",")}];`)(track);
        const classes = new Map(names.map((name, i) => [name, constructors[i]]));
        const objects = new Map();
        const values = new Map();
        const decode = (value) => {
          if (value === null || ["string", "boolean"].includes(typeof value) || (typeof value === "number" && Number.isFinite(value))) return value;
          if (value && Object.keys(value).length === 1 && value.undefined === true) return undefined;
          if (value && Object.keys(value).length === 1 && typeof value.ref === "string" && objects.has(value.ref)) return objects.get(value.ref);
          unavailable("Invalid value or unresolved reference in validation snapshot");
        };
        for (const entry of example.objects) {
          if (typeof entry.id !== "string" || objects.has(entry.id) || !classes.has(entry.className)) unavailable("Invalid object ID or class in validation snapshot");
          objects.set(entry.id, Object.create(classes.get(entry.className).prototype));
        }
        for (const entry of example.objects) {
          if (!entry.fields || Array.isArray(entry.fields) || typeof entry.fields !== "object") unavailable("Missing complete field map");
          for (const [field, value] of Object.entries(entry.fields)) {
            Object.defineProperty(objects.get(entry.id), field, { value: decode(value), writable: true, enumerable: true, configurable: true });
          }
        }
        for (const [id, value] of Object.entries(example.values ?? {})) {
          if (objects.has(id)) unavailable("Literal ID collides with an object ID");
          const decoded = decode(value);
          if (decoded !== null && typeof decoded === "object") unavailable("Literal map contains an object");
          values.set(id, decoded);
        }
        const resolve = (id) => {
          if (objects.has(id)) return objects.get(id);
          if (values.has(id)) return values.get(id);
          if (id === "null") return null;
          unavailable(`Unknown operation target: ${id}`);
        };
        if (!objects.has(call.receiverObject)) unavailable("Receiver is missing from the typed pre-state");
        const receiver = objects.get(call.receiverObject);
        if (call.receiverClassName && Object.getPrototypeOf(receiver) !== classes.get(call.receiverClassName)?.prototype) unavailable("Receiver class mismatch");
        return { objects, values, classes, allocations, decode, resolve, receiver, args: example.arguments.map(decode) };
      };
      let expected;
      let actual;
      let expectedReturn;
      try {
        expected = build();
        actual = build();
        for (const op of call.operations) {
          switch (op.editType) {
            case "addNode": {
              if (expected.objects.has(op.id) || expected.values.has(op.id)) unavailable("Duplicate created ID");
              if (op.isLiteral) {
                let value = op.label;
                // Kanon records every literal typed into the editor as a string, and RefSyn's synthesis reads
                // numerals among them as numbers (js_literal_expr_from_graph_label); validation reads them the same way.
                if (op.type === "number" || (op.type === "string" && /^[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?$/.test(value))) value = Number(value);
                else if (op.type === "boolean" && typeof value === "string") {
                  if (!["true", "false"].includes(value)) unavailable("Invalid boolean literal");
                  value = value === "true";
                } else if (typeof value === "string" && op.type !== "string") unavailable("Literal type is missing");
                if (!["string", "number", "boolean"].includes(typeof value) || (typeof value === "number" && !Number.isFinite(value))) unavailable("Unsupported literal");
                expected.values.set(op.id, value);
              } else {
                const constructor = expected.classes.get(op.label);
                if (!constructor) unavailable(`Unknown constructor: ${op.label}`);
                const object = new constructor();
                if ([...expected.objects.values()].includes(object)) unavailable("Constructor did not create a fresh object");
                expected.objects.set(op.id, object);
              }
              break;
            }
            case "addEdge":
            case "editEdgeReference":
            case "deleteEdge": {
              const object = expected.objects.get(op.from);
              if (!object || typeof op.label !== "string") unavailable("Unknown write source or field");
              const current = object[op.label];
              if (op.editType === "addEdge") {
                // Kanon draws no edge for null, undefined or absent fields, so addEdge can only start from one of them.
                if (current !== null && current !== undefined) unavailable("addEdge on a field that already has an edge");
              } else {
                const old = expected.resolve(op.editType === "deleteEdge" ? op.to : op.oldTo);
                if (op.editType === "deleteEdge" && (old === null || typeof old !== "object")) unavailable("deleteEdge is supported only on object reference fields");
                if (current !== old) unavailable("Recorded old target does not match the typed state");
              }
              object[op.label] = op.editType === "deleteEdge" ? null : expected.resolve(op.newTo ?? op.to);
              break;
            }
            case "addVariable":
            case "editVariableReference":
              if (op.label !== "return") unavailable("External variable updates are not supported by this validator");
              expectedReturn = expected.resolve(op.newTo ?? op.to);
              break;
            default: unavailable(`Unsupported validation operation: ${op.editType}`);
          }
        }
      } catch (error) {
        if (error?.unavailable) throw error;
        // A failure here concerns the demonstration or its classes, not the candidate code.
        unavailable(`Expected state could not be derived: ${error?.message ?? error}`);
      }
      const methods = new Function(...names, `"use strict"; return ({${[...response.code, response.composed_method_code].join(",\n")}});`)(...actual.classes.values());
      if (Reflect.ownKeys(methods).length !== response.code.length + 1) throw new Error("Generated method names collide");
      const main = Object.hasOwn(methods, call.methodName) ? methods[call.methodName] : undefined;
      if (typeof main !== "function") throw new Error("Generated method is missing");
      const prototype = Object.getPrototypeOf(actual.receiver);
      for (const [name, descriptor] of Object.entries(Object.getOwnPropertyDescriptors(methods))) {
        if (typeof descriptor.value !== "function") throw new Error("Generated member is not a method");
        Object.defineProperty(prototype, name, descriptor);
      }
      // The comparison below only sees the demonstrated heap, so state outside it must stay untouched.
      const ambient = [["globalThis", globalThis], ["Object.prototype", Object.prototype], ["Function.prototype", Function.prototype], ["Array.prototype", Array.prototype],
        ...[...actual.classes].flatMap(([name, constructor]) => [[name, constructor], [`${name}.prototype`, constructor.prototype]])];
      // Hosts may register symbol-keyed globals on their own (e.g. Node's fetch dispatcher), so globals are compared by name.
      const keysOf = (target) => target === globalThis ? Object.getOwnPropertyNames(target) : Reflect.ownKeys(target);
      const snapshot = () => ambient.map(([, target]) => new Map(keysOf(target).map((key) => [key, Object.getOwnPropertyDescriptor(target, key)])));
      const before = snapshot();
      const actualReturn = main.apply(actual.receiver, actual.args);
      const same = (a, b) => a && b && ["value", "get", "set", "writable", "enumerable", "configurable"].every((key) => Object.is(a[key], b[key]));
      snapshot().forEach((after, i) => {
        const changed = [...new Set([...before[i].keys(), ...after.keys()])].filter((key) => !same(before[i].get(key), after.get(key)));
        if (changed.length) throw new Error(`Side effect outside the demonstrated heap: ${ambient[i][0]}.${changed.map(String).join(", ")}`);
      });
      const initialIds = [...expected.objects.keys()].filter((id) => actual.objects.has(id));
      const state = { pairs: new Map(initialIds.map((id) => [expected.objects.get(id), actual.objects.get(id)])), visited: new Set() };
      state.reverse = new Map([...state.pairs].map(([a, b]) => [b, a]));
      const classOf = (object, classes) => [...classes].find(([, c]) => Object.getPrototypeOf(object) === c.prototype)?.[0];
      const compare = (left, right, path, s) => {
        if (left === null || typeof left !== "object") {
          if (!Object.is(left, right)) throw new Error(`Value or return mismatch at ${path}`);
          return;
        }
        if (right === null || typeof right !== "object") throw new Error(`Reference mismatch at ${path}`);
        if ((s.pairs.has(left) && s.pairs.get(left) !== right) || (s.reverse.has(right) && s.reverse.get(right) !== left)) throw new Error(`Identity mismatch at ${path}`);
        s.pairs.set(left, right); s.reverse.set(right, left);
        if (s.visited.has(left)) return;
        s.visited.add(left);
        const className = classOf(left, expected.classes);
        if (!className || className !== classOf(right, actual.classes)) throw new Error(`Class mismatch at ${path}`);
        const a = Object.getOwnPropertyDescriptors(left), b = Object.getOwnPropertyDescriptors(right);
        const keys = Reflect.ownKeys(a), other = Reflect.ownKeys(b);
        if (keys.some((key) => typeof key !== "string") || keys.length !== other.length || keys.some((key) => !Object.hasOwn(b, key))) throw new Error(`Property presence mismatch at ${path}`);
        for (const key of keys) {
          if (!Object.hasOwn(a[key], "value") || !Object.hasOwn(b[key], "value")) unavailable("Accessor fields are not supported by the validator");
          for (const flag of ["enumerable", "writable", "configurable"]) if (a[key][flag] !== b[key][flag]) throw new Error(`Property descriptor mismatch at ${path}.${key}`);
          compare(a[key].value, b[key].value, `${path}.${key}`, s);
        }
      };
      for (const id of initialIds) compare(expected.objects.get(id), actual.objects.get(id), id, state);
      compare(expectedReturn, actualReturn, "return", state);
      if (expected.allocations.length !== actual.allocations.length) throw new Error("Allocation count mismatch");
      // Unreachable allocations still belong to the demonstrated heap; pair them by structure, not by creation order.
      const pending = expected.allocations.filter((object) => !state.pairs.has(object));
      const remaining = actual.allocations.filter((object) => !state.reverse.has(object));
      const match = (i, s) => {
        if (i === pending.length) return true;
        if (s.pairs.has(pending[i])) return match(i + 1, s);
        for (const candidate of remaining) {
          if (s.reverse.has(candidate)) continue;
          const trial = { pairs: new Map(s.pairs), reverse: new Map(s.reverse), visited: new Set(s.visited) };
          try {
            compare(pending[i], candidate, "allocation", trial);
          } catch (error) {
            if (error?.unavailable) throw error;
            continue;
          }
          if (match(i + 1, trial)) return true;
        }
        return false;
      };
      if (!match(0, state)) throw new Error("Unreachable allocations do not match the demonstration");
      checked++;
    }
    return { status: "passed", checked_demonstrations: checked, error: null };
  } catch (error) {
    return { status: error?.unavailable ? "unavailable" : "failed", checked_demonstrations: checked, error: String(error?.message ?? error) };
  }
}

export function applyValidationResult(response, result) {
  response.validation = result;
  if (result.status !== "passed") {
    response.composed_method_code = null;
    response.code = [];
  }
  return response;
}

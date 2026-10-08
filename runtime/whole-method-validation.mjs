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
    if (validation?.version !== 1 || !Array.isArray(validation.cases) || !Array.isArray(validation.classes)) {
      unavailable(validation?.error ?? "Typed pre-state, return value and constructor definitions are required (validation.version=1)");
    }
    const calls = request.method_calls;
    if (!Array.isArray(calls) || calls.length === 0 || validation.cases.length !== calls.length) {
      unavailable("Validation must cover every demonstration exactly once");
    }
    const identifier = /^[$_\p{ID_Start}][$_\u200c\u200d\p{ID_Continue}]*$/u;
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
      if (!Array.isArray(example.objects) || !Array.isArray(example.arguments) || (!Object.hasOwn(example, "returnValue") && !Object.hasOwn(example, "returnTarget"))) {
        unavailable("Each demonstration needs objects, arguments and an explicit returnValue");
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
      const expected = build();
      const actual = build();
      const initialIds = [...expected.objects.keys()];
      let operationReturn;
      let hasOperationReturn = false;
      for (const op of call.operations) {
        switch (op.editType) {
          case "addNode": {
            if (expected.objects.has(op.id) || expected.values.has(op.id)) unavailable("Duplicate created ID");
            if (op.isLiteral) {
              let value = op.label;
              if (op.type === "number") value = Number(value);
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
            const className = [...expected.classes].find(([, c]) => Object.getPrototypeOf(object) === c.prototype)?.[0];
            if (op.editType === "deleteEdge" && !validation.classes.find((c) => c.name === className)?.referenceFields?.includes(op.label)) unavailable("deleteEdge requires a declared nullable reference field");
            if (op.editType !== "addEdge") {
              const old = expected.resolve(op.editType === "deleteEdge" ? op.to : op.oldTo);
              if (object[op.label] !== old) unavailable("Recorded old target does not match the typed state");
            }
            const value = op.editType === "deleteEdge" ? null : expected.resolve(op.newTo ?? op.to);
            object[op.label] = value;
            break;
          }
          case "addVariable":
          case "editVariableReference":
            if (op.label !== "return") unavailable("External variable updates are not supported by this validator");
            operationReturn = expected.resolve(op.newTo ?? op.to);
            hasOperationReturn = true;
            break;
          default: unavailable(`Unsupported validation operation: ${op.editType}`);
        }
      }
      const expectedReturn = Object.hasOwn(example, "returnTarget") ? expected.resolve(example.returnTarget) : expected.decode(example.returnValue);
      if (hasOperationReturn && !Object.is(operationReturn, expectedReturn)) unavailable("Return expectation conflicts with the demonstration");
      const methods = new Function(...names, `"use strict"; return ({${[...response.code, response.composed_method_code].join(",\n")}});`)(...actual.classes.values());
      if (Reflect.ownKeys(methods).length !== response.code.length + 1) throw new Error("Generated method names collide");
      const main = Object.hasOwn(methods, call.methodName) ? methods[call.methodName] : undefined;
      if (typeof main !== "function") throw new Error("Generated method is missing");
      const prototype = Object.getPrototypeOf(actual.receiver);
      for (const [name, descriptor] of Object.entries(Object.getOwnPropertyDescriptors(methods))) {
        if (typeof descriptor.value !== "function") throw new Error("Generated member is not a method");
        Object.defineProperty(prototype, name, descriptor);
      }
      const actualReturn = main.apply(actual.receiver, actual.args);
      const pairs = new Map(initialIds.map((id) => [expected.objects.get(id), actual.objects.get(id)]));
      const reverse = new Map([...pairs].map(([a, b]) => [b, a]));
      const visited = new Set();
      const classOf = (object, classes) => [...classes].find(([, c]) => Object.getPrototypeOf(object) === c.prototype)?.[0];
      const compare = (left, right, path) => {
        if (left === null || typeof left !== "object") {
          if (!Object.is(left, right)) throw new Error(`Value or return mismatch at ${path}`);
          return;
        }
        if (right === null || typeof right !== "object") throw new Error(`Reference mismatch at ${path}`);
        if ((pairs.has(left) && pairs.get(left) !== right) || (reverse.has(right) && reverse.get(right) !== left)) throw new Error(`Identity mismatch at ${path}`);
        pairs.set(left, right); reverse.set(right, left);
        if (visited.has(left)) return;
        visited.add(left);
        const className = classOf(left, expected.classes);
        if (!className || className !== classOf(right, actual.classes)) throw new Error(`Class mismatch at ${path}`);
        const a = Object.getOwnPropertyDescriptors(left), b = Object.getOwnPropertyDescriptors(right);
        const keys = Reflect.ownKeys(a), other = Reflect.ownKeys(b);
        if (keys.some((key) => typeof key !== "string") || keys.length !== other.length || keys.some((key) => !Object.hasOwn(b, key))) throw new Error(`Property presence mismatch at ${path}`);
        for (const key of keys) {
          if (!Object.hasOwn(a[key], "value") || !Object.hasOwn(b[key], "value")) unavailable("Accessor fields are not supported by the validator");
          for (const flag of ["enumerable", "writable", "configurable"]) if (a[key][flag] !== b[key][flag]) throw new Error(`Property descriptor mismatch at ${path}.${key}`);
          compare(a[key].value, b[key].value, `${path}.${key}`);
        }
      };
      for (const id of initialIds) compare(expected.objects.get(id), actual.objects.get(id), id);
      compare(expectedReturn, actualReturn, "return");
      if (expected.allocations.length !== actual.allocations.length) throw new Error("Allocation count mismatch");
      // Unreachable allocations still belong to the demonstrated heap.
      const remaining = actual.allocations.filter((object) => !reverse.has(object));
      for (const object of expected.allocations) {
        if (!pairs.has(object)) compare(object, remaining.shift(), "allocation");
      }
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

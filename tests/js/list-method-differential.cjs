const assert = require("node:assert/strict");
const fs = require("node:fs");
const vm = require("node:vm");

function compareMethods({ method, classSource, helpers, composedMethod, candidates, sequences = false }) {
  candidates ??= [{ method, classSource, helpers, composedMethod }];
  const lengths = [1, 2, 3, 5, 8, 16, 32];
  const layouts = ["head", "shared", "interior"];
  const patterns = ["distinct", "zero", "repeated", "seeded"];
  const makeValues = (length, pattern) => {
    let seed = 0x5eed + length;
    return Array.from({ length }, (_, i) => {
      if (pattern === "distinct") return 101 + 17 * i;
      if (pattern === "zero") return 0;
      if (pattern === "repeated") return [-9, 4, -9][i % 3];
      seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
      return (seed % 2001) - 1000;
    });
  };
  const makeClass = () => {
    const allocations = [];
    const track = (Class) => new Proxy(Class, {
      construct(target, args, newTarget) {
        const object = Reflect.construct(target, args, newTarget);
        allocations.push(object);
        return object;
      },
    });
    // A named class expression would bind new Obj() to the untracked constructor.
    const anonymousClass = classSource.replace(/^class Obj\b/, "class");
    const Obj = new Function("track", `
      let Obj = track((${anonymousClass}));
      ${candidates.map((candidate) => `Object.defineProperty(Obj.prototype, ${JSON.stringify(candidate.method)},
        Object.getOwnPropertyDescriptor((${candidate.classSource.replace(/^class Obj\b/, "class")}).prototype,
          ${JSON.stringify(candidate.method)}));`).join("\n")}
      return Obj;
    `)(track);
    return { Obj, allocations };
  };
  const reference = makeClass();
  const generated = makeClass();
  for (const candidate of candidates) delete generated.Obj.prototype[candidate.method];
  for (const candidate of candidates) {
    const methods = new Function("Obj", `return ({${[...candidate.helpers, candidate.composedMethod].join(",\n")}});`)(generated.Obj);
    assert.ok(Object.hasOwn(methods, candidate.method), "the candidate must define the method being compared");
    Object.defineProperties(generated.Obj.prototype, Object.getOwnPropertyDescriptors(methods));
  }
  const build = (world, values, layout) => {
    world.allocations.length = 0;
    const nodes = new Array(values.length);
    const order = Array.from(values.keys());
    if (layout === "shared") order.reverse();
    for (const i of order) nodes[i] = new world.Obj(values[i]);
    nodes.forEach((node, i) => { node.g = nodes[i + 1] ?? null; });
    const roots = {
      head: nodes[0], receiver: nodes[0],
      middle: nodes[Math.floor(values.length / 2)], tail: nodes.at(-1),
      sameReceiver: nodes[0],
    };
    if (layout !== "head") {
      roots.detached = new world.Obj(-555);
      roots.detached.g = new world.Obj(0);
      roots.detached.g.g = null;
      roots.branch = new world.Obj(777);
      roots.branch.g = roots.middle;
    }
    if (layout === "interior") {
      const prefix = new world.Obj(-42);
      prefix.g = new world.Obj(42);
      prefix.g.g = roots.receiver;
      roots.head = prefix;
    }
    return roots;
  };
  const observe = (world, roots, returned, initialCount) => {
    const ids = new Map(world.allocations.map((node, i) =>
      [node, i < initialCount ? `existing${i}` : `created${i - initialCount}`]));
    const encode = (value) => {
      if (value === undefined) return { undefined: true };
      if (value === null || ["number", "boolean", "string"].includes(typeof value)) return value;
      assert.ok(ids.has(value), "references must name an existing or constructed Obj");
      return { ref: ids.get(value) };
    };
    return {
      roots: Object.fromEntries(Object.entries(roots).map(([key, value]) => [key, encode(value)])),
      returned: encode(returned),
      objects: world.allocations.map((node) => {
        assert.equal(Object.getPrototypeOf(node), world.Obj.prototype, "object class must stay Obj");
        const descriptors = Object.getOwnPropertyDescriptors(node);
        assert.ok(Reflect.ownKeys(node).every((key) => typeof key === "string"));
        return Object.keys(descriptors).sort().map((key) => {
          const d = descriptors[key];
          assert.ok(Object.hasOwn(d, "value"), "fields must remain data properties");
          return [key, encode(d.value), d.writable, d.enumerable, d.configurable];
        });
      }),
    };
  };
  if (sequences) {
    const byMethod = Object.fromEntries(candidates.map(({ method }) => [method, 0]));
    const sequenceLengths = [1, 2, 8, 32];
    let calls = 0;
    for (let seed = 1; seed <= 24; seed += 1) {
      for (const length of sequenceLengths) {
        for (const layout of layouts) {
          let state = seed;
          const choose = (bound) => {
            state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
            return (state >>> 8) % bound;
          };
          const values = Array.from({ length }, () => choose(2001) - 1000);
          const expectedRoots = build(reference, values, layout);
          const actualRoots = build(generated, values, layout);
          const initialCount = reference.allocations.length;
          const history = [];
          const reachable = (root) => {
            const nodes = [];
            const seen = new Set();
            for (let node = root; node !== null; node = node.g) {
              assert.ok(!seen.has(node), "list must remain acyclic");
              seen.add(node);
              nodes.push(node);
            }
            return nodes;
          };
          for (let step = 0; step < 50; step += 1) {
            try {
              const expectedNodes = reachable(expectedRoots.receiver);
              const actualNodes = reachable(actualRoots.receiver);
              let selected = candidates[choose(candidates.length)].method;
              if (selected === "popBack" && expectedNodes.length === 1) selected = "append";
              const offset = choose(expectedNodes.length - (selected === "popBack" ? 1 : 0));
              const value = choose(2001) - 1000;
              const args = selected === "popBack" ? []
                : selected === "setAt" || selected === "insert"
                  ? [choose(expectedNodes.length - offset), value] : [value];
              history.push({ method: selected, offset, args });
              const expectedReturn = expectedNodes[offset][selected](...args);
              const actualReturn = actualNodes[offset][selected](...args);
              if (selected === "prepend" && offset === 0) {
                expectedRoots.receiver = expectedReturn;
                actualRoots.receiver = actualReturn;
              }
              assert.deepEqual(
                observe(generated, actualRoots, actualReturn, initialCount),
                observe(reference, expectedRoots, expectedReturn, initialCount),
              );
              byMethod[selected] += 1;
              calls += 1;
            } catch (error) {
              throw new Error(JSON.stringify({ seed, length, layout, step, values, history }), { cause: error });
            }
          }
        }
      }
    }
    for (const count of Object.values(byMethod)) assert.ok(count > 0);
    return { sequences: 24 * sequenceLengths.length * layouts.length, calls, byMethod,
      seeds: [1, 24], stepsPerSequence: 50, lengths: sequenceLengths, layouts };
  }
  let cases = 0;
  const byLayout = Object.fromEntries(layouts.map((layout) => [layout, 0]));
  for (const length of lengths) {
    if (method === "popBack" && length < 2) continue;
    for (const pattern of patterns) {
      const values = makeValues(length, pattern);
      const replacements = [...new Set([-10001, -1, 0, 1, 10007, values[0], values.at(-1)])];
      const indices = method === "setAt" || method === "insert"
        ? Array.from({ length }, (_, i) => i) : [null];
      const argumentsList = indices.flatMap((index) => method === "popBack" ? [[]]
        : replacements.map((value) => index === null ? [value] : [index, value]));
      for (const layout of layouts) {
        for (const args of argumentsList) {
          const expectedRoots = build(reference, values, layout);
          const actualRoots = build(generated, values, layout);
          const initialCount = reference.allocations.length;
          assert.equal(generated.allocations.length, initialCount);
          const expectedReturn = reference.Obj.prototype[method].apply(expectedRoots.receiver, args);
          const actualReturn = generated.Obj.prototype[method].apply(actualRoots.receiver, args);
          assert.deepEqual(
            observe(generated, actualRoots, actualReturn, initialCount),
            observe(reference, expectedRoots, expectedReturn, initialCount),
            JSON.stringify({ method, length, pattern, layout, values, args }),
          );
          cases += 1;
          byLayout[layout] += 1;
        }
      }
    }
  }
  return { method, cases, byLayout, lengths: lengths.filter((n) => method !== "popBack" || n >= 2), patterns };
}

const payload = JSON.parse(fs.readFileSync(0, "utf8"));
const result = vm.runInNewContext(`(${compareMethods.toString()})(payload)`, { payload, assert }, { timeout: 10000 });
process.stdout.write(JSON.stringify(result));

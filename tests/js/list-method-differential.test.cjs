const assert = require("node:assert/strict");
const { spawnSync } = require("node:child_process");
const { test } = require("node:test");
const path = require("node:path");

const methods = {
  setAt: "setAt(i, value) { let node = this; while (i-- > 0) node = node.g; node.f = value; }",
  insert: "insert(i, value) { let node = this; while (i-- > 0) node = node.g; const added = new Obj(value); added.g = node.g; node.g = added; }",
  append: "append(value) { let node = this; while (node.g !== null) node = node.g; node.g = new Obj(value); }",
  prepend: "prepend(value) { const added = new Obj(value); added.g = this; return added; }",
  popBack: "popBack() { let node = this; while (node.g.g !== null) node = node.g; node.g = null; }",
};
const classSource = `class Obj {
  constructor(value) { this.f = value; this.g = null; }
  ${Object.values(methods).join("\n")}
}`;
const candidate = (method, composedMethod = methods[method]) => ({ method, classSource, helpers: [], composedMethod });
const run = (payload) => {
  const result = spawnSync(process.execPath, [path.join(__dirname, "list-method-differential.cjs")], {
    input: JSON.stringify(payload), encoding: "utf8", timeout: 15000,
  });
  assert.ifError(result.error);
  assert.equal(result.signal, null, result.stderr);
  return result;
};

for (const method of Object.keys(methods)) {
  test(`accepts the reference ${method}`, () => {
    const result = run(candidate(method));
    assert.equal(result.status, 0, result.stderr);
    assert.ok(JSON.parse(result.stdout).cases > 0);
  });
}

const faults = [
  ["wrong value", "setAt", "setAt(i, value) { let node = this; while (i-- > 0) node = node.g; node.f = value + 1; }"],
  ["wrong index", "setAt", "setAt(i, value) { let node = this; while (i-- > 1) node = node.g; node.f = value; }"],
  ["same values with copied identity", "setAt", `setAt(i, value) {
    let node = this; while (i-- > 0) node = node.g; node.f = value;
    if (this.g) { const copy = new Obj(this.g.f); copy.g = this.g.g; this.g = copy; }
  }`],
  ["wrong return", "setAt", methods.setAt.replace("node.f = value;", "node.f = value; return this;")],
  ["damage to a detached node", "popBack", methods.popBack.replace("node.g = null;", "node.g.f = 99999; node.g = null;")],
  ["unreachable extra allocation", "setAt", methods.setAt.replace("node.f = value;", "node.f = value; new Obj(value);")],
  ["deletion instead of null", "popBack", methods.popBack.replace("node.g = null;", "delete node.g;")],
  ["missing method", "setAt", "unrelated() {}"],
];
for (const [fault, method, code] of faults) {
  test(`rejects ${fault}`, () => {
    const result = run(candidate(method, code));
    assert.equal(result.status, 1, result.stderr);
    assert.match(result.stderr, /AssertionError/);
    assert.match(result.stderr, fault === "missing method" ? /candidate must define the method/ : /operator: 'deepStrictEqual'/);
  });
}

const mixed = (setAt = methods.setAt) => ({
  classSource, sequences: true,
  candidates: Object.keys(methods).map((method) => candidate(method, method === "setAt" ? setAt : methods[method])),
});
test("accepts mixed reference calls with persistent state", () => {
  const result = run(mixed());
  assert.equal(result.status, 0, result.stderr);
  assert.equal(JSON.parse(result.stdout).calls, 14400);
});
test("reports the seed and history when a later call corrupts state", () => {
  const code = methods.setAt.replace("node.f = value;", `
    globalThis.setAtCalls = (globalThis.setAtCalls ?? 0) + 1;
    node.f = globalThis.setAtCalls === 2 ? value + 1 : value;
  `);
  const result = run(mixed(code));
  assert.equal(result.status, 1, result.stderr);
  assert.match(result.stderr, /AssertionError/);
  assert.match(result.stderr, /"seed":1/);
  assert.match(result.stderr, /"history":\[/);
});

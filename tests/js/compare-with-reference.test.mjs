import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const run = (reference, method, composedMethod, helpers = []) => spawnSync(process.execPath, [path.join(root, "scripts/compare_with_reference.mjs")], {
  input: JSON.stringify({ reference, className: "Node", method, helpers, composedMethod }),
  encoding: "utf8", timeout: 15000,
});
const compare = (method, composedMethod, helpers = [], reference = path.join(root, `tests/recordings/${method}/reference.mjs`)) => {
  const result = run(reference, method, composedMethod, helpers);
  assert.equal(result.status, 0, result.stderr);
  return JSON.parse(result.stdout);
};
// A reference module with one input, written for a single test.
const singleCase = (methodSource, build) => {
  const file = path.join(fs.mkdtempSync(path.join(os.tmpdir(), "refsyn-compare-")), "reference.mjs");
  fs.writeFileSync(file, `export const classSource = \`class Node { constructor() { this.next = null; } ${methodSource} }\`;
export const cases = [{ name: "only", build: ${build} }];`);
  return file;
};
const append = "append(x) { let node = this; while (node.next !== null) node = node.next; const added = new Node(); added.val = x; node.next = added; }";
const popBack = "popBack() { let node = this; while (node.next.next !== null) node = node.next; node.next = null; }";

test("accepts the reference and an equivalent method that creates its objects in another order", () => {
  assert.deepEqual(compare("append", append).mismatches, []);
  const reordered = compare("append", "append(x) { const spare = new Node(); const added = new Node(); added.val = x; this.last().next = added; }", [
    "last() { return this.next === null ? this : this.next.last(); }",
  ]);
  assert.equal(reordered.cases, 15);
  assert.deepEqual(reordered.mismatches, []);
});

for (const [fault, method, code] of [
  ["a wrong value", "append", append.replace("added.val = x;", "added.val = x + 1;")],
  ["a copy in place of the original last node", "append", append.replace("node.next = added;",
    "if (node === this) { node.next = added; return; } let p = this; while (p.next !== node) p = p.next; const copy = new Node(); copy.val = node.val; copy.next = added; p.next = copy;")],
  ["a return value the reference does not give", "append", append.replace("node.next = added;", "node.next = added; return added;")],
  ["a deleted field in place of null", "popBack", popBack.replace("node.next = null;", "delete node.next;")],
  ["damage to the node it unlinks", "popBack", popBack.replace("node.next = null;", "node.next.val = 0; node.next = null;")],
  ["throwing", "append", "append(x) { throw new RangeError('no'); }"],
  ["not defining the method", "append", "other(x) {}"],
]) {
  test(`rejects ${fault}`, () => {
    assert.ok(compare(method, code).mismatches.length > 0, fault);
  });
}

test("stops a candidate that does not finish and reports it as a mismatch", () => {
  const reference = singleCase("touch() { this.val = 1; }", "({ Node }) => { const a = new Node(); return { receiver: a, args: [], roots: { a } }; }");
  const [mismatch] = compare("touch", "touch() { while (true) {} }", [], reference).mismatches;
  assert.deepEqual(mismatch.actual.outcome, { timedOut: 2000 });
});

test("a reference that does not finish is an error rather than an agreement", () => {
  const reference = singleCase("touch() { while (true) {} }", "({ Node }) => { const a = new Node(); return { receiver: a, args: [], roots: { a } }; }");
  const result = run(reference, "touch", "touch() { while (true) {} }");
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /the reference did not finish/);
});

// Inputs Kanon would not record; the first three are review counterexamples that agreed before
// the conditions were applied to every object observed.
const unrecorded = {
  "a Map in the input, even once detached": "a.link = new Map([['x', 1]]);",
  "a symbol value": "a.tag = Symbol('t');",
  "a getter": "Object.defineProperty(a, 'value', { get() { return 1; }, enumerable: true, configurable: true });",
  "a symbol key": "a[Symbol.for('edge')] = null;",
  "a non-enumerable field": "Object.defineProperty(a, 'hidden', { value: null, writable: true, configurable: true });",
  "-0": "a.v = -0;",
  "NaN": "a.v = NaN;",
  "Infinity": "a.v = Infinity;",
  "a frozen object": "Object.freeze(a);",
};
for (const [what, setup] of Object.entries(unrecorded)) {
  test(`refuses an input with ${what} instead of comparing it`, () => {
    const reference = singleCase("touch() { this.link = null; }", `({ Node }) => { const a = new Node(); ${setup} return { receiver: a, args: [], roots: { a } }; }`);
    const result = run(reference, "touch", "touch() { this.link = null; }");
    assert.notEqual(result.status, 0, what);
    assert.match(result.stderr, /unsupported state/, what);
  });
}

test("a candidate that leaves a state Kanon would not record is a mismatch", () => {
  const pair = "({ Node }) => { const a = new Node(), b = new Node(); a.next = b; return { receiver: a, args: [], roots: { a } }; }";
  const reference = singleCase("touch() { this.v = 0; this.next = null; }", pair);
  assert.deepEqual(compare("touch", "touch() { this.v = 0; this.next = null; }", [], reference).mismatches, []);
  for (const change of [
    "this.v = -0;", "this.v = NaN;", "this.v = Symbol('t');", "this.v = 1n;", "this.v = new Map();",
    "this.v = () => 1;", "this.v = [1];", "this.v = {};",
    "Object.defineProperty(this, 'v', { get() { return 0; }, enumerable: true, configurable: true });",
    "this[Symbol.for('edge')] = 1;",
    "Object.defineProperty(this, 'h', { value: 1, writable: true, configurable: true });",
    "Object.preventExtensions(this);",
    // b is detached by the call but existed before it, so it is still checked.
    "this.next.v = new Map();",
  ]) {
    const code = `touch() { this.v = 0; ${change} this.next = null; }`;
    const [mismatch] = compare("touch", code, [], reference).mismatches;
    assert.match(mismatch?.actual.error ?? "", /unsupported state/, change);
  }
});

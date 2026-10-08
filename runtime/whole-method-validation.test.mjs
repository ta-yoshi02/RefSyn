import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { validateWholeMethod, applyValidationResult } from './whole-method-validation.mjs';

export const fixture = (code = 'cut() { this.link = null; return this; }') => ({
  request: {
    method_calls: [{ callLabel: 'c', contextSensitiveID: 'ctx', receiverObject: 'a', receiverClassName: 'Cell', methodName: 'cut', arguments: [], operations: [{ editType: 'deleteEdge', from: 'a', label: 'link', to: 'b' }, { editType: 'addVariable', label: 'return', to: 'a' }] }],
    validation: { version: 2, classes: [{ name: 'Cell', source: 'class Cell { constructor() { this.payload = 7; this.link = null; } }' }], cases: [{ callLabel: 'c', contextSensitiveID: 'ctx', objects: [{ id: 'a', className: 'Cell', fields: { payload: 1, link: { ref: 'b' }, extra: { undefined: true } } }, { id: 'b', className: 'Cell', fields: { payload: 1, link: { ref: 'a' } } }], values: { 'a-payload': 1, 'b-payload': 1 }, arguments: [] }] },
  },
  response: { composed_method_code: code, code: [], escher_results: [] }, taskNames: [],
});

test('accepts null assignment and retains identity, cycles and undefined fields', () => {
  const result = validateWholeMethod(fixture());
  assert.equal(result.status, 'passed', result.error);
});
for (const [name, code] of [
  ['property deletion is different from null', 'cut() { delete this.link; return this; }'],
  ['return identity is not value equality', 'cut() { const other = this.link; this.link = null; return other; }'],
  ['unchanged unreachable object is still checked', 'cut() { this.link.payload = 999; this.link = null; return this; }'],
  ['undefined is not an absent field', 'cut() { this.link = null; delete this.extra; return this; }'],
  ['throwing generated code is rejected', 'cut() { throw new Error("bad"); }'],
]) test(name, () => assert.equal(validateWholeMethod(fixture(code)).status, 'failed'));

test('constructor defaults and fresh return aliases are compared', () => {
  const payload = fixture('cut() { const cell = new Cell(); this.link = cell; return cell; }');
  payload.request.method_calls[0].operations = [{ editType: 'addNode', id: 'new', label: 'Cell' }, { editType: 'editEdgeReference', from: 'a', label: 'link', oldTo: 'b', newTo: 'new' }, { editType: 'addVariable', label: 'return', to: 'new' }];
  assert.equal(validateWholeMethod(payload).status, 'passed');
  payload.response.composed_method_code = 'cut() { const cell = new Cell(); cell.payload = null; this.link = cell; return cell; }';
  assert.equal(validateWholeMethod(payload).status, 'failed');
});

test('a wrong second demonstration rejects the entire candidate', () => {
  const payload = fixture();
  payload.request.method_calls.push({ ...payload.request.method_calls[0], contextSensitiveID: 'ctx2', operations: [] });
  payload.request.validation.cases.push({ ...payload.request.validation.cases[0], contextSensitiveID: 'ctx2' });
  const result = validateWholeMethod(payload);
  assert.equal(result.status, 'failed');
  assert.equal(result.checked_demonstrations, 1);
  applyValidationResult(payload.response, result);
  assert.equal(payload.response.composed_method_code, null);
  assert.deepEqual(payload.response.code, []);
});

test('missing state and missing successful holes never produce adoptable code', () => {
  const payload = fixture();
  delete payload.request.validation;
  assert.equal(validateWholeMethod(payload).status, 'unavailable');
  payload.taskNames = ['hole'];
  assert.equal(validateWholeMethod(payload).status, 'failed');
});

test('value-field delete, stale old targets and unresolved sources are unavailable', () => {
  for (const operation of [
    { editType: 'deleteEdge', from: 'a', label: 'payload', to: 'a-payload' },
    { editType: 'deleteEdge', from: 'a', label: 'link', to: 'a' },
    { editType: 'addEdge', from: 'a', label: 'link', to: 'a' },
    { editType: 'deleteEdge', from: 'missing', label: 'link', to: 'b' },
  ]) {
    const payload = fixture();
    payload.request.method_calls[0].operations = [operation];
    assert.equal(validateWholeMethod(payload).status, 'unavailable');
  }
});

test('hole helpers are installed with composed code', () => {
  const payload = fixture('cut() { this.link = this.nullValue(); return this; }');
  payload.response.code = ['nullValue() { return null; }'];
  payload.response.escher_results = [{ name: 'h', success: true }];
  payload.taskNames = ['h'];
  assert.equal(validateWholeMethod(payload).status, 'passed');
  payload.response.escher_results[0].success = false;
  assert.equal(validateWholeMethod(payload).status, 'failed');
});

test('repeated writes are replayed in demonstration order without compression or rejection', () => {
  const payload = fixture('cut() { this.link = this; return this; }');
  payload.request.method_calls[0].operations.splice(1, 0, { editType: 'addEdge', from: 'a', label: 'link', to: 'a' });
  assert.equal(validateWholeMethod(payload).status, 'passed');
  payload.response.composed_method_code = 'cut() { this.link = null; return this; }';
  assert.equal(validateWholeMethod(payload).status, 'failed');
});

test('native validation bounds non-termination and returns a failure', () => {
  const output = spawnSync(process.execPath, ['scripts/validate_method.mjs'], { cwd: new URL('..', import.meta.url), input: JSON.stringify(fixture('cut() { while (true) {} }')), encoding: 'utf8', timeout: 5000 });
  assert.equal(output.status, 0);
  const result = JSON.parse(output.stdout);
  assert.equal(result.status, 'failed');
  assert.match(result.error, /timed out/);
});


test('class names cannot capture the allocation tracker', () => {
  const payload = JSON.parse(JSON.stringify(fixture()).replaceAll('Cell', '__track'));
  assert.equal(validateWholeMethod(payload).status, 'passed');
});

test('duplicate generated method names are rejected before execution', () => {
  const payload = fixture();
  payload.taskNames = ['h'];
  payload.response.escher_results = [{ name: 'h', success: true }];
  payload.response.code = ['cut() { return null; }'];
  assert.equal(validateWholeMethod(payload).status, 'failed');
});

for (const [name, code] of [
  ['doing nothing is rejected', 'cut() { return this; }'],
  ['updating a different object is rejected', 'cut() { this.link.link = null; return this; }'],
  ['allocating instead of writing null is rejected', 'cut() { this.link = new Cell(); return this; }'],
  ['prototype mutation is rejected', 'cut() { this.link = null; Object.getPrototypeOf(this).leak = 1; return this; }'],
  ['static state mutation is rejected', 'cut() { this.link = null; Cell.count = 1; return this; }'],
  ['global mutation is rejected', 'cut() { this.link = null; globalThis.leak = 1; return this; }'],
  ['builtin prototype mutation is rejected', 'cut() { this.link = null; Array.prototype.leak = 1; return this; }'],
]) test(name, () => {
  const result = validateWholeMethod(fixture(code));
  assert.equal(result.status, 'failed', result.error);
});

test('without a return edit the demonstrated return value is undefined', () => {
  const payload = fixture('cut() { this.link = null; }');
  payload.request.method_calls[0].operations.pop();
  assert.equal(validateWholeMethod(payload).status, 'passed');
  payload.response.composed_method_code = 'cut() { this.link = null; return this; }';
  assert.equal(validateWholeMethod(payload).status, 'failed');
});

test('addEdge starts from a field without an edge', () => {
  const payload = fixture('cut() { this.link = null; this.link = this; return this; }');
  payload.request.method_calls[0].operations.splice(1, 0, { editType: 'addEdge', from: 'a', label: 'link', to: 'a' });
  assert.equal(validateWholeMethod(payload).status, 'passed');
});

test('unreachable allocations are paired by structure rather than creation order', () => {
  const payload = fixture();
  payload.request.method_calls[0].operations.splice(1, 0,
    { editType: 'addNode', id: 'n1', label: 'Cell' },
    { editType: 'addNode', id: 'n2', label: 'Cell' },
    { editType: 'addEdge', from: 'n1', label: 'link', to: 'a' });
  for (const code of [
    'cut() { const x = new Cell(); new Cell(); x.link = this; this.link = null; return this; }',
    'cut() { new Cell(); const x = new Cell(); x.link = this; this.link = null; return this; }',
  ]) {
    payload.response.composed_method_code = code;
    assert.equal(validateWholeMethod(payload).status, 'passed', code);
  }
  payload.response.composed_method_code = 'cut() { new Cell(); new Cell(); this.link = null; return this; }';
  assert.equal(validateWholeMethod(payload).status, 'failed');
});

test('a constructor that cannot run in isolation makes the demonstration unavailable', () => {
  const payload = fixture('cut() { this.link = new Cell(); return this; }');
  payload.request.validation.classes[0].source = 'class Cell { constructor() { this.id = nextId++; this.link = null; } }';
  payload.request.method_calls[0].operations.splice(1, 0, { editType: 'addNode', id: 'n', label: 'Cell' });
  const result = validateWholeMethod(payload);
  assert.equal(result.status, 'unavailable');
  assert.match(result.error, /Expected state could not be derived/);
});

test('demonstrations of different methods are not validated together', () => {
  const payload = fixture();
  payload.request.method_calls.push({ ...payload.request.method_calls[0], contextSensitiveID: 'ctx2', methodName: 'other' });
  payload.request.validation.cases.push({ ...payload.request.validation.cases[0], contextSensitiveID: 'ctx2' });
  assert.equal(validateWholeMethod(payload).status, 'unavailable');
});

test('literals keep their recorded type', () => {
  const payload = fixture();
  payload.request.method_calls[0].operations.splice(1, 0,
    { editType: 'addNode', id: 'lit', label: '007', isLiteral: true, type: 'string' },
    { editType: 'editEdgeReference', from: 'a', label: 'payload', oldTo: 'a-payload', newTo: 'lit' });
  const run = (code) => { payload.response.composed_method_code = code; return validateWholeMethod(payload).status; };
  assert.equal(run('cut() { this.link = null; this.payload = "007"; return this; }'), 'passed');
  assert.equal(run('cut() { this.link = null; this.payload = 7; return this; }'), 'failed');
  payload.request.method_calls[0].operations[1] = { editType: 'addNode', id: 'lit', label: '53', isLiteral: true, type: 'number' };
  assert.equal(run('cut() { this.link = null; this.payload = 53; return this; }'), 'passed');
  assert.equal(run('cut() { this.link = null; this.payload = "53"; return this; }'), 'failed');
});

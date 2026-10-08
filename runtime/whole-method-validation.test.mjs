import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { validateWholeMethod, applyValidationResult } from './whole-method-validation.mjs';

export const fixture = (code = 'cut() { this.link = null; return this; }') => ({
  request: {
    method_calls: [{ callLabel: 'c', contextSensitiveID: 'ctx', receiverObject: 'a', receiverClassName: 'Cell', methodName: 'cut', arguments: [], operations: [{ editType: 'deleteEdge', from: 'a', label: 'link', to: 'b' }] }],
    validation: { version: 1, classes: [{ name: 'Cell', source: 'class Cell { constructor() { this.payload = 7; this.link = null; } }', referenceFields: ['link'] }], cases: [{ callLabel: 'c', contextSensitiveID: 'ctx', objects: [{ id: 'a', className: 'Cell', fields: { payload: 1, link: { ref: 'b' }, extra: { undefined: true } } }, { id: 'b', className: 'Cell', fields: { payload: 1, link: { ref: 'a' } } }], arguments: [], returnValue: { ref: 'a' } }] },
  },
  response: { composed_method_code: code, code: [], escher_results: [] }, taskNames: [],
});

test('accepts null assignment and retains identity, cycles and undefined fields', () => {
  assert.equal(validateWholeMethod(fixture()).status, 'passed');
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
  payload.request.method_calls[0].operations = [{ editType: 'addNode', id: 'new', label: 'Cell' }, { editType: 'editEdgeReference', from: 'a', label: 'link', oldTo: 'b', newTo: 'new' }];
  payload.request.validation.cases[0].returnValue = { ref: 'new' };
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

test('non-reference delete and unresolved source are unavailable', () => {
  for (const operation of [{ editType: 'deleteEdge', from: 'a', label: 'payload', to: 'n' }, { editType: 'deleteEdge', from: 'missing', label: 'link', to: 'b' }]) {
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
  payload.request.method_calls[0].operations.push({ editType: 'editEdgeReference', from: 'a', label: 'link', oldTo: 'null', newTo: 'a' });
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

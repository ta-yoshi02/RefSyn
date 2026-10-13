import test from 'node:test';
import assert from 'node:assert/strict';
import { compileRefsynMethod } from './refsyn-escher-adapter.mjs';
import { parseJsonSynthesisSpec, prepareJsonSynthesisJob } from '../external/escher-ts/dist/components/user-friendly-json.js';
import { executeTerm } from '../external/escher-ts/dist/types/term.js';

// class C { key; next; val } with two integer fields, encoded as src/pbe_problem.rs does: an integer
// field's slot is the index of its own object, and each field's heap holds the value or the error value.
const objects = [{ key: 7, next: 1, val: 100 }, { key: 3, next: null, val: undefined }];
const ref = (index) => ({ ref: index === null ? -1 : index });
const int = (value) => (value === undefined ? { error: true } : value);
const task = (returnType, x) => ({
  name: 'f_h0',
  category: 'refsyn',
  classes: [{ name: 'C', fields: { key: 'Ref[Int]', next: 'Ref[Object[C]]', val: 'Ref[Int]' } }],
  exposeClassComponents: false,
  autoClassFieldComponents: true,
  signature: {
    returnType,
    args: [{ name: 'x', type: 'Ref[Object[C]]' }],
    autoExpandClassSignature: {
      className: 'C', thisRefName: 'thisRef', classHeapName: 'nodeHeap',
      fieldHeapNames: { key: 'keyHeap', next: 'nextHeap', val: 'valHeap' },
    },
  },
  components: ['isNull', 'equal', 'zero', 'inc'].map((name) => ({ name, kind: 'libraryRef', ref: name })),
  examples: [[[
    ref(0),
    objects.map((o, i) => ({ object: { className: 'C', fields: { key: ref(i), next: ref(o.next), val: ref(i) } } })),
    objects.map((o) => int(o.key)),
    objects.map((o) => ref(o.next)),
    objects.map((o) => int(o.val)),
    ref(x),
  ], returnType === 'Int' ? 0 : true]],
  refsynMeta: {
    jsMethodName: 'f_h0', className: 'C', thisRefName: 'thisRef', classHeapName: 'nodeHeap',
    valueFields: ['key', 'val'], pointerFields: ['next'],
    fieldHeapNames: { key: 'keyHeap', next: 'nextHeap', val: 'valHeap' },
    explicitArgs: [{ name: 'x', legacyType: 'Ptr', taskType: 'Ref[Object[C]]' }],
  },
});

const v = (name) => ({ kind: 'var', name });
const c = (name, ...args) => ({ kind: 'component', name, args });
const heap = (field) => v(`${field}Heap`);
const owners = [v('thisRef'), v('x'), c('nextOf', v('thisRef'), v('nodeHeap'), heap('next'))];
const fields = ['key', 'val'];

const evaluate = (returnType, x, term) => {
  const raw = task(returnType, x);
  const job = prepareJsonSynthesisJob(parseJsonSynthesisSpec(JSON.stringify(raw)));
  const inputs = new Map(job.inputNames.map((name, index) => [name, job.examples[0][0][index]]));
  return { raw, job, expected: executeTerm((name) => inputs.get(name), job.env, term) };
};

const run = (returnType, x, term) => {
  const { raw, job, expected } = evaluate(returnType, x, term);
  const code = compileRefsynMethod(raw, job, term);
  class C {}
  Object.assign(C.prototype, new Function(`return ({ ${code} });`)());
  const live = objects.map(() => new C());
  objects.forEach((o, i) => Object.assign(live[i], { key: o.key, next: o.next === null ? null : live[o.next], val: o.val }));
  return { expected, actual: live[0].f_h0(x === null ? null : live[x]), code };
};

test('loadInt reads the field of the heap it is given, whichever field reference names the object', () => {
  const mismatches = [];
  for (const x of [0, 1]) {
    for (const owner of owners) {
      for (const read of fields) {
        for (const named of fields) {
          for (const checked of fields) {
            const term = c('loadInt', heap(read), c(`${named}Of`, owner, v('nodeHeap'), heap(checked)));
            const { expected, actual, code } = run('Int', x, term);
            if (expected.tag === 'error') continue;
            if (actual !== expected.value) mismatches.push(`x=${x}: ${code} gave ${actual}, escher ${expected.value}`);
          }
        }
      }
    }
  }
  assert.deepEqual(mismatches, []);
});

test('has<Field> of an integer field tests the heap it is given', () => {
  const mismatches = [];
  for (const owner of [v('thisRef'), c('nextOf', v('thisRef'), v('nodeHeap'), heap('next'))]) {
    for (const named of fields) {
      for (const checked of fields) {
        const term = c(`has${named[0].toUpperCase()}${named.slice(1)}`, owner, v('nodeHeap'), heap(checked));
        const { expected, actual, code } = run('Bool', 0, term);
        if (expected.tag === 'error') continue;
        if (actual !== expected.value) mismatches.push(`${code} gave ${actual}, escher ${expected.value}`);
      }
    }
  }
  assert.deepEqual(mismatches, []);
});

test('an integer field reference outside loadInt is refused, since escher-ts compares it as the object', () => {
  const keyOfThis = c('keyOf', v('thisRef'), v('nodeHeap'), heap('key'));
  const valOfThis = c('valOf', v('thisRef'), v('nodeHeap'), heap('val'));
  // Both name object 0, so escher-ts finds them equal although this.key is 7 and this.val is 100.
  assert.equal(evaluate('Bool', 0, c('equal', keyOfThis, valOfThis)).expected.value, true);
  for (const term of [c('equal', keyOfThis, valOfThis), c('isNull', valOfThis)]) {
    assert.throws(() => run('Bool', 0, term), /only supported as the reference read by loadInt/);
  }
});

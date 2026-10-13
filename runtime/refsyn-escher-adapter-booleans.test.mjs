import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { compileRefsynMethod } from './refsyn-escher-adapter.mjs';
import { parseJsonSynthesisSpec, prepareJsonSynthesisJob } from '../external/escher-ts/dist/components/user-friendly-json.js';
import { executeTerm } from '../external/escher-ts/dist/types/term.js';

// class Flag { on; next } with `on` typed Bool, encoded as src/pbe_problem.rs does: the slot holds
// the boolean, or the error value when it is undefined or absent; there is no heap for it.
const U = 'undefined';
const A = 'absent';
const heaps = {
  trueThenFalse: [{ on: true, next: 1 }, { on: false, next: null }],
  falseThenTrue: [{ on: false, next: 1 }, { on: true, next: null }],
  undefinedThenTrue: [{ on: U, next: 1 }, { on: true, next: null }],
  trueThenAbsent: [{ on: true, next: 1 }, { on: A, next: null }],
};

const ref = (index) => ({ ref: index === null ? -1 : index });
const slot = (on) => (on === U || on === A ? { error: true } : on);
const task = (heap, b, x) => ({
  name: 'f_h0',
  category: 'refsyn',
  classes: [{ name: 'Flag', fields: { next: 'Ref[Object[Flag]]', on: 'Bool' } }],
  exposeClassComponents: false,
  autoClassFieldComponents: true,
  strictFieldAccess: true,
  signature: {
    returnType: 'Bool',
    args: [{ name: 'b', type: 'Bool' }, { name: 'x', type: 'Ref[Object[Flag]]' }],
    autoExpandClassSignature: { className: 'Flag', thisRefName: 'thisRef', classHeapName: 'nodeHeap', fieldHeapNames: { next: 'nextHeap' } },
  },
  components: ['isNull', 'equal', 'and', 'or', 'not'].map((name) => ({ name, kind: 'libraryRef', ref: name })),
  examples: [[[
    ref(0),
    heap.map((o) => ({ object: { className: 'Flag', fields: { next: ref(o.next), on: slot(o.on) } } })),
    heap.map((o) => ref(o.next)),
    b,
    ref(x),
  ], true]],
  refsynMeta: {
    jsMethodName: 'f_h0', className: 'Flag', thisRefName: 'thisRef', classHeapName: 'nodeHeap',
    valueFields: [], pointerFields: ['next'], boolFields: ['on'], fieldHeapNames: { next: 'nextHeap' },
    explicitArgs: [{ name: 'b', legacyType: 'Bool', taskType: 'Bool' }, { name: 'x', legacyType: 'Ptr', taskType: 'Ref[Object[Flag]]' }],
  },
});

const v = (name) => ({ kind: 'var', name });
const c = (name, ...args) => ({ kind: 'component', name, args });
const iff = (condition, thenBranch, elseBranch) => ({ kind: 'if', condition, thenBranch, elseBranch });
const T = v('thisRef');
const X = v('x');
const B = v('b');
const onOf = (o) => c('onOf', o, v('nodeHeap'));
const hasOn = (o) => c('hasOn', o, v('nodeHeap'));
const nextOf = (o) => c('nextOf', o, v('nodeHeap'), v('nextHeap'));

const terms = [
  onOf(T), onOf(X), onOf(nextOf(T)), hasOn(T), hasOn(X), hasOn(nextOf(T)),
  c('not', onOf(X)), c('and', onOf(T), B), c('or', B, onOf(X)), c('and', c('isNull', X), onOf(X)),
  c('equal', onOf(T), onOf(X)), c('equal', onOf(T), B), c('equal', B, c('not', B)),
  iff(onOf(T), B, c('not', B)), iff(B, onOf(X), c('trueConst')), c('falseConst'),
];

const escherOutcome = (raw, term) => {
  const job = prepareJsonSynthesisJob(parseJsonSynthesisSpec(JSON.stringify(raw)));
  const inputs = new Map(job.inputNames.map((name, index) => [name, job.examples[0][0][index]]));
  const value = executeTerm((name) => inputs.get(name), job.env, term);
  return { job, outcome: value.tag === 'error' ? 'failure' : `${value.tag} ${value.value}` };
};

const jsOutcome = (code, heap, b, x) => {
  const script = `
    class Flag {}
    Object.assign(Flag.prototype, ({ ${code} }));
    const objects = HEAP.map(() => new Flag());
    HEAP.forEach((o, i) => {
      if (o.on !== 'absent') objects[i].on = o.on === 'undefined' ? undefined : o.on;
      objects[i].next = o.next === null ? null : objects[o.next];
    });
    let outcome;
    try {
      const r = objects[0].f_h0(B, X === null ? null : objects[X]);
      outcome = typeof r === 'boolean' ? 'bool ' + r : 'not a boolean: ' + String(r);
    } catch { outcome = 'failure'; }
    outcome;`;
  try {
    return vm.runInNewContext(script, { HEAP: heap, B: b, X: x }, { timeout: 200 });
  } catch (error) {
    if (/timed out/.test(String(error))) return 'nonterminating';
    throw error;
  }
};

test('boolean field components have escher-ts\'s outcome, failures included', () => {
  const mismatches = [];
  const counts = { value: 0, failure: 0 };
  for (const [name, heap] of Object.entries(heaps)) {
    for (const x of [null, 0, 1]) {
      for (const b of [true, false]) {
        const raw = task(heap, b, x);
        for (const term of terms) {
          const { job, outcome: expected } = escherOutcome(raw, term);
          const actual = jsOutcome(compileRefsynMethod(raw, job, term), heap, b, x);
          counts[expected === 'failure' ? 'failure' : 'value'] += 1;
          if (actual !== expected) mismatches.push(`${name} b=${b} x=${x}: ${JSON.stringify(term)} js ${actual}, escher ${expected}`);
        }
      }
    }
  }
  assert.deepEqual(mismatches, []);
  assert.ok(counts.value > 0 && counts.failure > 0, JSON.stringify(counts));
});

test('a boolean field read fails on what escher-ts cannot hold instead of yielding false', () => {
  const raw = task(heaps.trueThenFalse, true, 1);
  const { job } = escherOutcome(raw, onOf(X));
  const code = compileRefsynMethod(raw, job, onOf(X));
  for (const held of [undefined, null, 0, 1, 'false', 'true']) {
    const script = `class Flag {}; Object.assign(Flag.prototype, ({ ${code} })); const x = new Flag(); x.on = HELD; new Flag().f_h0(true, x);`;
    assert.throws(() => vm.runInNewContext(script, { HELD: held }), (error) => error.name === 'TypeError', String(held));
  }
  // A Bool field has no heap, so the read takes the receiver and the object heap only.
  assert.equal(job.inputNames.includes('onHeap'), false);
});

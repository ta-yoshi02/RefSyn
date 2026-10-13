import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { compileRefsynMethod } from './refsyn-escher-adapter.mjs';
import { parseJsonSynthesisSpec, prepareJsonSynthesisJob } from '../external/escher-ts/dist/components/user-friendly-json.js';
import { executeTerm } from '../external/escher-ts/dist/types/term.js';

// The translated helper must have the outcome escher-ts gives the term: the same value, or a
// failure (escher-ts error, JS exception) where escher-ts fails. It must also terminate.
// class C { next; val }, encoded as src/pbe_problem.rs does: `undefined` marks an undefined
// field, `absent` a missing property, both the error value in escher-ts.
const U = 'undefined';
const A = 'absent';
const heaps = {
  list: [{ next: 1, val: 1 }, { next: 2, val: 2 }, { next: null, val: 3 }],
  single: [{ next: null, val: 7 }],
  undefinedNext: [{ next: 1, val: 1 }, { next: U, val: 2 }],
  absentNext: [{ next: 1, val: 1 }, { next: A, val: 2 }],
  undefinedVal: [{ next: 1, val: U }, { next: 2, val: 5 }, { next: null, val: A }],
  cycle: [{ next: 1, val: 1 }, { next: 0, val: 2 }],
  selfLoop: [{ next: 0, val: 4 }],
};

const ref = (index) => ({ ref: index === null ? -1 : index });
const slot = (value) => (value === U || value === A ? { error: true } : ref(value));
const int = (value) => (value === U || value === A ? { error: true } : value);
const task = (returnType, heap, k, x) => ({
  name: 'f_h0',
  category: 'refsyn',
  classes: [{ name: 'C', fields: { next: 'Ref[Object[C]]', val: 'Ref[Int]' } }],
  exposeClassComponents: false,
  autoClassFieldComponents: true,
  strictFieldAccess: true,
  signature: {
    returnType,
    args: [{ name: 'k', type: 'Int' }, { name: 'x', type: 'Ref[Object[C]]' }],
    autoExpandClassSignature: { className: 'C', thisRefName: 'thisRef', classHeapName: 'nodeHeap', fieldHeapNames: { next: 'nextHeap', val: 'valHeap' } },
  },
  components: ['isNull', 'equal', 'and', 'or', 'not', 'isZero', 'isNonNeg', 'zero', 'inc', 'dec', 'last_ptr', 'penultimateRef', 'nthNextRef', 'findByValueRef']
    .map((name) => ({ name, kind: 'libraryRef', ref: name })),
  examples: [[[
    ref(0),
    heap.map((o, i) => ({ object: { className: 'C', fields: { next: slot(o.next), val: ref(i) } } })),
    heap.map((o) => slot(o.next)),
    heap.map((o) => int(o.val)),
    k,
    ref(x),
  ], returnType === 'Int' ? 0 : returnType === 'Bool' ? true : { ref: -1 }]],
  refsynMeta: {
    jsMethodName: 'f_h0', className: 'C', thisRefName: 'thisRef', classHeapName: 'nodeHeap',
    valueFields: ['val'], pointerFields: ['next'], fieldHeapNames: { next: 'nextHeap', val: 'valHeap' },
    explicitArgs: [{ name: 'k', legacyType: 'Int', taskType: 'Int' }, { name: 'x', legacyType: 'Ptr', taskType: 'Ref[Object[C]]' }],
  },
});

const v = (name) => ({ kind: 'var', name });
const c = (name, ...args) => ({ kind: 'component', name, args });
const iff = (condition, thenBranch, elseBranch) => ({ kind: 'if', condition, thenBranch, elseBranch });
const H = { nodes: v('nodeHeap'), next: v('nextHeap'), val: v('valHeap') };
const nextOf = (o) => c('nextOf', o, H.nodes, H.next);
const T = v('thisRef');
const X = v('x');
const K = v('k');

const terms = {
  'Ref[Object[C]]': [
    nextOf(T), nextOf(X), nextOf(nextOf(T)),
    c('nthNextRef', T, H.nodes, H.next, K), c('nthNextRef', X, H.nodes, H.next, K),
    c('last_ptr', T, H.next), c('last_ptr', X, H.next),
    c('penultimateRef', T, H.next), c('penultimateRef', X, H.next),
    c('findByValueRef', T, H.nodes, H.next, H.val, K), c('findByValueRef', X, H.nodes, H.next, H.val, K),
    iff(c('isNull', X), T, nextOf(X)),
  ],
  Bool: [
    c('hasNext', T, H.nodes, H.next), c('hasNext', X, H.nodes, H.next), c('hasNext', nextOf(T), H.nodes, H.next),
    c('hasVal', T, H.nodes, H.val), c('hasVal', X, H.nodes, H.val),
    c('isNull', nextOf(X)), c('equal', nextOf(X), T),
    // Operands that fail on some inputs, on either side: escher-ts evaluates both.
    c('and', c('isNull', X), c('hasNext', X, H.nodes, H.next)),
    c('and', c('hasNext', X, H.nodes, H.next), c('isNull', X)),
    c('or', c('not', c('isNull', X)), c('hasNext', X, H.nodes, H.next)),
    c('or', c('hasNext', X, H.nodes, H.next), c('not', c('isNull', X))),
  ],
  Int: [
    c('loadInt', H.val, c('valOf', T, H.nodes, H.val)), c('loadInt', H.val, c('valOf', X, H.nodes, H.val)),
    c('inc', c('loadInt', H.val, c('valOf', nextOf(T), H.nodes, H.val))),
  ],
};

const escherOutcome = (raw, term) => {
  const job = prepareJsonSynthesisJob(parseJsonSynthesisSpec(JSON.stringify(raw)));
  const inputs = new Map(job.inputNames.map((name, index) => [name, job.examples[0][0][index]]));
  const value = executeTerm((name) => inputs.get(name), job.env, term);
  return { job, outcome: value.tag === 'error' ? 'failure' : value.tag === 'ref' ? `ref ${value.value}` : `${value.tag} ${value.value}` };
};

const jsOutcome = (code, heap, k, x) => {
  const script = `
    class C {}
    Object.assign(C.prototype, ({ ${code} }));
    const objects = HEAP.map(() => new C());
    HEAP.forEach((o, i) => {
      if (o.next !== 'absent') objects[i].next = o.next === 'undefined' ? undefined : o.next === null ? null : objects[o.next];
      if (o.val !== 'absent') objects[i].val = o.val === 'undefined' ? undefined : o.val;
    });
    let outcome;
    try {
      const r = objects[0].f_h0(K, X === null ? null : objects[X]);
      outcome = r === null ? 'ref -1' : typeof r === 'object' ? 'ref ' + objects.indexOf(r) : typeof r === 'boolean' ? 'bool ' + r : r === undefined ? 'undefined' : 'int ' + r;
    } catch { outcome = 'failure'; }
    outcome;`;
  try {
    return vm.runInNewContext(script, { HEAP: heap, K: k, X: x }, { timeout: 200 });
  } catch (error) {
    if (/timed out/.test(String(error))) return 'nonterminating';
    throw error;
  }
};

for (const [returnType, termList] of Object.entries(terms)) {
  test(`${returnType} helpers have escher-ts's outcome, failures and termination included`, () => {
    const mismatches = [];
    const counts = { value: 0, failure: 0 };
    for (const [name, heap] of Object.entries(heaps)) {
      for (const x of [null, 0, heap.length - 1]) {
        for (const k of [-1, 0, 1, 2, 5]) {
          const raw = task(returnType, heap, k, x);
          for (const term of termList) {
            const { job, outcome: expected } = escherOutcome(raw, term);
            const code = compileRefsynMethod(raw, job, term);
            const actual = jsOutcome(code, heap, k, x);
            counts[expected === 'failure' ? 'failure' : 'value'] += 1;
            if (actual !== expected) mismatches.push(`${name} k=${k} x=${x}: ${JSON.stringify(term)} js ${actual}, escher ${expected}`);
          }
        }
      }
    }
    assert.deepEqual(mismatches, []);
    // Both outcomes must actually be exercised for the comparison to mean anything.
    assert.ok(counts.value > 0 && counts.failure > 0, JSON.stringify(counts));
  });
}

test('the cases the contract singles out', () => {
  const outcome = (returnType, heapName, term, k = 0, x = null) => {
    const raw = task(returnType, heaps[heapName], k, x);
    const { job, outcome: expected } = escherOutcome(raw, term);
    return { expected, actual: jsOutcome(compileRefsynMethod(raw, job, term), heaps[heapName], k, x) };
  };
  // A field of null fails rather than reading as null.
  assert.deepEqual(outcome('Ref[Object[C]]', 'list', nextOf(X)), { expected: 'failure', actual: 'failure' });
  assert.deepEqual(outcome('Bool', 'list', c('hasNext', X, H.nodes, H.next)), { expected: 'failure', actual: 'failure' });
  // A cycle fails in every traversal instead of hanging or claiming absence.
  for (const term of [c('last_ptr', T, H.next), c('penultimateRef', T, H.next), c('findByValueRef', T, H.nodes, H.next, H.val, K)]) {
    assert.deepEqual(outcome('Ref[Object[C]]', 'cycle', term, 9), { expected: 'failure', actual: 'failure' });
  }
  // A bounded walk around a cycle is still an answer.
  assert.deepEqual(outcome('Ref[Object[C]]', 'cycle', c('nthNextRef', T, H.nodes, H.next, K), 5), { expected: 'ref 1', actual: 'ref 1' });
  // and/or do not short-circuit past a failing operand.
  assert.deepEqual(outcome('Bool', 'list', c('and', c('not', c('isNull', X)), c('hasNext', X, H.nodes, H.next))), { expected: 'failure', actual: 'failure' });
  // An undefined integer field tests false.
  assert.deepEqual(outcome('Bool', 'undefinedVal', c('hasVal', T, H.nodes, H.val)), { expected: 'bool false', actual: 'bool false' });
});

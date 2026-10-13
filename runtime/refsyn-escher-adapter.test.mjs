import test from 'node:test';
import assert from 'node:assert/strict';
import { compileRefsynMethod } from './refsyn-escher-adapter.mjs';
import { parseJsonSynthesisSpec, prepareJsonSynthesisJob } from '../external/escher-ts/dist/components/user-friendly-json.js';
import { executeTerm } from '../external/escher-ts/dist/types/term.js';

// One class C { next } with two objects (0 -> 1 -> null), called on object 0 with k: Int and x: C.
// The input encoding follows src/pbe_problem.rs: a reference is { ref: index }, null is { ref: -1 }.
const ref = (index) => ({ ref: index === null ? -1 : index });
const task = (returnType, k, x) => ({
  name: 'f_h0',
  category: 'refsyn',
  classes: [{ name: 'C', fields: { next: 'Ref[Object[C]]' } }],
  exposeClassComponents: false,
  autoClassFieldComponents: true,
  strictFieldAccess: true,
  signature: {
    returnType,
    args: [{ name: 'k', type: 'Int' }, { name: 'x', type: 'Ref[Object[C]]' }],
    autoExpandClassSignature: { className: 'C', thisRefName: 'thisRef', classHeapName: 'nodeHeap', fieldHeapNames: { next: 'nextHeap' } },
  },
  components: ['isNull', 'equal', 'and', 'or', 'not', 'isZero', 'isNonNeg', 'zero', 'inc', 'dec', 'neg', 'plus']
    .map((name) => ({ name, kind: 'libraryRef', ref: name })),
  examples: [[[ref(0), [{ object: { className: 'C', fields: { next: ref(1) } } }, { object: { className: 'C', fields: { next: ref(null) } } }],
    [ref(1), ref(null)], k, ref(x)], returnType === 'Int' ? 0 : true]],
  refsynMeta: {
    jsMethodName: 'f_h0', className: 'C', thisRefName: 'thisRef', classHeapName: 'nodeHeap',
    valueFields: [], pointerFields: ['next'], fieldHeapNames: { next: 'nextHeap' },
    explicitArgs: [{ name: 'k', legacyType: 'Int', taskType: 'Int' }, { name: 'x', legacyType: 'Ptr', taskType: 'Ref[Object[C]]' }],
  },
});

const v = (name) => ({ kind: 'var', name });
const c = (name, ...args) => ({ kind: 'component', name, args });
const iff = (condition, thenBranch, elseBranch) => ({ kind: 'if', condition, thenBranch, elseBranch });

// Terms nest every operator under every other one, which is where a missing parenthesis changes the meaning.
const ints = [v('k'), c('zero'), c('inc', v('k')), c('dec', v('k')), c('neg', v('k')), c('plus', v('k'), c('dec', v('k')))];
const atoms = [c('isNull', v('x')), c('isZero', v('k')), c('isNonNeg', c('dec', v('k'))), c('equal', v('x'), v('thisRef'))];
const bools1 = [
  ...atoms,
  ...atoms.map((a) => c('not', a)),
  ...atoms.flatMap((a) => atoms.flatMap((b) => ['and', 'or', 'equal'].map((op) => c(op, a, b)))),
  ...ints.flatMap((i) => [c('isZero', i), c('isNonNeg', i), c('equal', i, c('inc', c('zero')))]),
];
const bools = [
  ...bools1,
  ...bools1.flatMap((a) => atoms.flatMap((b) => ['and', 'or', 'equal'].flatMap((op) => [c(op, a, b), c(op, b, a)]))),
];
const intTerms = [
  ...ints,
  ...ints.flatMap((i) => [c('inc', i), c('dec', i), c('neg', i), c('plus', i, c('neg', i)), c('plus', c('dec', i), i)]),
  ...atoms.map((b) => iff(b, c('inc', v('k')), c('neg', v('k')))),
];

const shown = (value) => (value.tag === 'error' ? 'failure' : value.tag === 'ref' ? `ref ${value.value}` : `${value.tag} ${value.value}`);
const jsShown = (value, objects) => (typeof value === 'object' ? `ref ${value === null ? -1 : objects.indexOf(value)}` : `${typeof value === 'boolean' ? 'bool' : 'int'} ${value}`);

const mismatches = (returnType, terms) => {
  const found = [];
  for (const k of [-1, 0, 1, 2]) {
    for (const x of [null, 0, 1]) {
      const raw = task(returnType, k, x);
      const job = prepareJsonSynthesisJob(parseJsonSynthesisSpec(JSON.stringify(raw)));
      const inputs = new Map(job.inputNames.map((name, index) => [name, job.examples[0][0][index]]));
      for (const term of terms) {
        const expected = executeTerm((name) => inputs.get(name), job.env, term);
        const code = compileRefsynMethod(raw, job, term);
        class C {}
        Object.assign(C.prototype, new Function(`return ({ ${code} });`)());
        const objects = [new C(), new C()];
        objects[0].next = objects[1];
        objects[1].next = null;
        let actual;
        try {
          actual = jsShown(objects[0].f_h0(k, x === null ? null : objects[x]), objects);
        } catch {
          actual = 'failure';
        }
        if (actual !== shown(expected)) found.push(`k=${k} x=${x}: ${code} gave ${actual}, escher ${shown(expected)}`);
      }
    }
  }
  return found;
};

test('translated boolean terms agree with escher-ts on every input', () => {
  assert.deepEqual(mismatches('Bool', bools), []);
});

test('translated integer terms agree with escher-ts on every input', () => {
  assert.deepEqual(mismatches('Int', intTerms), []);
});

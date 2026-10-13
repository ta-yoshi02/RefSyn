import test from 'node:test';
import assert from 'node:assert/strict';
import { runRefsynTasks } from './refsyn-escher-adapter.mjs';

// A helper whose two Int parameters differ only in non-ASCII characters, and which returns the second.
const task = (names) => ({
  name: 'f_h0',
  category: 'refsyn',
  classes: [{ name: 'C', fields: { next: 'Ref[Object[C]]' } }],
  exposeClassComponents: false,
  autoClassFieldComponents: true,
  signature: {
    returnType: 'Int',
    args: names.map((name) => ({ name, type: 'Int' })),
    autoExpandClassSignature: { className: 'C', thisRefName: 'thisRef', classHeapName: 'nodeHeap', fieldHeapNames: { next: 'nextHeap' } },
  },
  components: ['zero', 'inc'].map((name) => ({ name, kind: 'libraryRef', ref: name })),
  examples: [[3, 8], [5, 1], [0, 4]].map(([a, b]) => [[{ ref: 0 }, [{ object: { className: 'C', fields: { next: { ref: -1 } } } }], [{ ref: -1 }], a, b], b]),
  refsynMeta: {
    jsMethodName: 'f_h0', className: 'C', thisRefName: 'thisRef', classHeapName: 'nodeHeap',
    valueFields: [], pointerFields: ['next'], fieldHeapNames: { next: 'nextHeap' },
    explicitArgs: names.map((name) => ({ name, legacyType: 'Int', taskType: 'Int' })),
  },
});

// The parameter list of the compiled helper, which returns its last parameter.
const paramsOf = (names) => {
  const raw = task(names);
  raw.examples = raw.examples.map(([input]) => [[...input.slice(0, 3), ...names.map((_, i) => input[3 + Math.min(i, 1)])], input[4]]);
  const [outcome] = runRefsynTasks([raw], { quiet: true });
  assert.equal(outcome.success, true, outcome.error ?? '');
  new Function(`return ({ ${outcome.compiled_js} });`);
  return outcome.compiled_js.slice(outcome.compiled_js.indexOf('(') + 1, outcome.compiled_js.indexOf(')')).split(', ');
};

const helper = (names) => {
  const [outcome] = runRefsynTasks([task(names)], { quiet: true });
  assert.equal(outcome.success, true, outcome.error ?? '');
  return { code: outcome.compiled_js, method: new Function(`return ({ ${outcome.compiled_js} });`)().f_h0 };
};

test('non-ASCII parameter names are kept and stay distinct', () => {
  const { code, method } = helper(['x値', 'x数']);
  assert.match(code, /^f_h0\(x値, x数\)/);
  assert.equal(method(2, 9), 9);
});

test('names that are not identifiers get distinct replacements', () => {
  const { code, method } = helper(['a-b', 'a+b']);
  assert.match(code, /^f_h0\(a_b, a_b_1\)/);
  assert.equal(method(2, 9), 9);
});

// A replacement must not take a name that a later parameter keeps as written, nor one already given.
for (const names of [['a-b', 'a_b_2', 'a+b'], ['a-b', 'a+b', 'a_b_1'], ['a_b', 'a-b', 'a+b', 'a_b_1']]) {
  test(`replacements avoid every other parameter name: ${names.join(', ')}`, () => {
    const params = paramsOf(names);
    assert.equal(new Set(params).size, names.length, params.join(', '));
    names.forEach((name, index) => {
      if (/^[A-Za-z_$][\w$]*$/.test(name)) assert.equal(params[index], name);
    });
  });
}

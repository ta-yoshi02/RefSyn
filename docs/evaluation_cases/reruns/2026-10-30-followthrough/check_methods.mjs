import { readFileSync, writeFileSync } from 'node:fs';
import { isDeepStrictEqual } from 'node:util';
import { pathToFileURL } from 'node:url';
import path from 'node:path';
import { compileRefsynMethod } from '../../../../runtime/refsyn-escher-adapter.mjs';
import { parseJsonSynthesisSpec, prepareJsonSynthesisJob } from '../../../../external/escher-ts/dist/components/user-friendly-json.js';

const D = path.dirname(new URL(import.meta.url).pathname);
const R = path.dirname(D);
const imp = p => import(pathToFileURL(path.resolve(process.env.WEIGHTED_DIST, p)).href);
const { AscendRecSynthesizer } = await imp('synthesis/ascendrec/synthesizer.js');
const { anyArgSmaller } = await imp('components/component.js');
const weighted = await imp('components/user-friendly-json.js');
const { showTerm } = await imp('types/term.js');
globalThis.__refsynCompCost = { last_ptr: 3, penultimateRef: 3, nthNextRef: 3, findByValueRef: 3 };

const cases = {
  rotateRight3: ['2026-10-15-rotate-right-494d6d4/rotateRight', 'TreeNode', 'rotateRight'],
  rotateRight4: ['2026-10-15-rotate-right-4demos-494d6d4/rotateRight', 'TreeNode', 'rotateRight'],
  union: ['2026-10-30-uf-union/union', 'UF', 'union'],
};
const outputs = {};
for (const [name, [dir, className, method]] of Object.entries(cases)) {
  let source = readFileSync(`${R}/${dir}/source_after.js`, 'utf8').split(/\nlet /)[0];
  const current = source;
  const terms = [];
  for (const task of JSON.parse(readFileSync(`${R}/${dir}/mold_escher_tasks.json`, 'utf8'))) {
    const job = weighted.prepareJsonSynthesisJob(weighted.parseJsonSynthesisSpec(JSON.stringify(task)));
    let status;
    const start = performance.now();
    const result = new AscendRecSynthesizer({ maxCost: 20, timeoutMs: 2000, searchSizeFactor: 1,
      useReductionRules: true, onlyForwardSearch: false, argListCompare: anyArgSmaller,
      onDiagnostics: d => { status = d.status; },
    }).synthesize(job.functionName, job.inputTypes, job.inputNames, job.returnType, job.env, job.examples);
    if (!result) throw new Error(`${name}/${task.name}: ${status}`);
    const code = compileRefsynMethod(task, prepareJsonSynthesisJob(parseJsonSynthesisSpec(JSON.stringify(task))), result.program.body);
    const pattern = new RegExp(`^    ${task.name}\\(.*$`, 'gm');
    if ([...source.matchAll(pattern)].length !== 1) throw new Error(`helper replacement count: ${task.name}`);
    source = source.replace(pattern, () => `    ${code}`);
    terms.push({ helper: task.name, term: showTerm(result.program.body), ms: performance.now() - start });
  }
  writeFileSync(`${D}/${name}_candidate.js`, `${source}\n`);
  const classes = Object.fromEntries(Object.entries({ current, candidate: source }).map(([setting, s]) => [setting, new Function(`${s}\nreturn ${className};`)()]));
  const inputs = [];
  if (method === 'rotateRight') {
    const variants = [null, {}, { left: {} }, { right: {} }];
    for (const ll of variants) for (const r of variants) for (const ml of variants) for (const mr of variants) {
      inputs.push({ domain: 'trees', shape: { left: { left: ll, right: { left: ml, right: mr } }, right: r } });
    }
    inputs.push({ domain: 'trees', shape: { left: {} } });
    for (let encoding = 0; encoding < 4096; encoding++) {
      const edges = Array.from({ length: 6 }, (_, i) => Math.floor(encoding / 4 ** i) % 4 - 1);
      if (edges[0] !== -1) inputs.push({ domain: 'graphs', edges });
    }
  } else {
    for (let a = 0; a <= 12; a++) for (let b = 0; b <= 12; b++) {
      for (const descendants of [false, true]) {
        if (descendants && (a === 0 || b === 0)) continue;
        inputs.push({ domain: a === b ? 'equal_rank_outside_contract' : 'distinct_rank', ranks: [a, b], descendants });
      }
    }
    for (let a = 0; a <= 6; a++) for (let b = 0; b <= 6; b++) {
      if (a !== b) inputs.push({ domain: 'reachable_rank_trees', ranks: [a, b], reachable: true });
    }
  }
  const build = (Cls, input) => {
    const nodes = [];
    const make = () => { const n = new Cls(); n.marker = nodes.length; nodes.push(n); return n; };
    if (input.shape) {
      const tree = shape => { if (!shape) return null; const n = make(); n.left = tree(shape.left); n.right = tree(shape.right); return n; };
      tree(input.shape);
    } else if (input.edges) {
      for (let i = 0; i < 3; i++) make();
      nodes.forEach((n, i) => { n.left = nodes[input.edges[2 * i]] ?? null; n.right = nodes[input.edges[2 * i + 1]] ?? null; });
    } else {
      for (const rank of input.ranks) make().rank = rank;
      if (input.descendants) for (const root of nodes.slice()) { const child = make(); child.parent = root; }
      if (input.reachable) {
        const fill = root => {
          for (let rank = 0; rank < root.rank; rank++) {
            const child = make(); child.rank = rank; child.parent = root; fill(child);
          }
        };
        nodes.slice().forEach(fill);
      }
    }
    return nodes;
  };
  const reference = nodes => {
    const [self, other] = nodes;
    if (method === 'rotateRight') { const left = self.left; self.left = left.right; left.right = self; return left; }
    if (self.rank < other.rank) self.parent = other;
    else { other.parent = self; if (self.rank === other.rank) self.rank++; }
  };
  const snapshot = (nodes, ret) => {
    const all = nodes.slice();
    const encode = value => {
      if (value === undefined) return { undefined: true };
      if (value === null || typeof value !== 'object') return value;
      if (!all.includes(value)) all.push(value);
      return { ref: all.indexOf(value) };
    };
    const result = encode(ret);
    const fields = [];
    for (let i = 0; i < all.length; i++) fields.push(Object.fromEntries(Object.keys(all[i]).sort().map(k => [k, encode(all[i][k])])));
    return { fields, ret: result };
  };
  const results = {};
  for (const [setting, Cls] of Object.entries(classes)) {
    results[setting] = inputs.map(input => {
      const nodes = build(Cls, input), ideal = build(Cls, input);
      const expected = snapshot(ideal, reference(ideal));
      try {
        const generated = snapshot(nodes, nodes[0][method](...(method === 'union' ? [nodes[1]] : [])));
        return { input, agrees: isDeepStrictEqual(generated, expected), generated, expected };
      } catch (error) { return { input, agrees: false, error: String(error), expected }; }
    });
  }
  const summary = Object.fromEntries(Object.entries(results).map(([setting, rows]) => [setting,
    Object.fromEntries([...new Set(inputs.map(i => i.domain))].map(domain => {
      const subset = rows.filter(r => r.input.domain === domain);
      return [domain, { agree: subset.filter(r => r.agrees).length, total: subset.length }];
    })),
  ]));
  outputs[name] = { terms, summary };
  writeFileSync(`${D}/${name}_checks.json`, JSON.stringify(results, null, 2) + '\n');
}
writeFileSync(`${D}/method_summary.json`, JSON.stringify(outputs, null, 2) + '\n');
console.log(JSON.stringify(outputs, null, 2));
if (Object.values(outputs).some(o => Object.entries(o.summary.candidate).some(([domain, r]) => domain !== 'equal_rank_outside_contract' && r.agree !== r.total))) process.exitCode = 1;

import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
import path from 'node:path';
const imp = p => import(pathToFileURL(path.resolve(process.env.WEIGHTED_DIST, p)).href);
const { AscendRecSynthesizer } = await imp('synthesis/ascendrec/synthesizer.js');
const { anyArgSmaller } = await imp('components/component.js');
const { parseJsonSynthesisSpec, prepareJsonSynthesisJob } = await imp('components/user-friendly-json.js');
const { showTerm } = await imp('types/term.js');
const tasks = JSON.parse(readFileSync('docs/evaluation_cases/reruns/2026-10-30-uf-union/union/mold_escher_tasks.json', 'utf8'));
globalThis.__refsynCompCost = { last_ptr: 3, penultimateRef: 3, nthNextRef: 3, findByValueRef: 3 };
const rows = [];
for (let repetition = 1; repetition <= 10; repetition++) {
  const helpers = [];
  for (const task of tasks) {
    const job = prepareJsonSynthesisJob(parseJsonSynthesisSpec(JSON.stringify(task)));
    let status;
    const synth = new AscendRecSynthesizer({ maxCost: 20, timeoutMs: 2000, searchSizeFactor: 1,
      useReductionRules: true, onlyForwardSearch: false, argListCompare: anyArgSmaller,
      onDiagnostics: d => { status = d.status; },
    });
    const start = performance.now();
    const result = synth.synthesize(job.functionName, job.inputTypes, job.inputNames, job.returnType, job.env, job.examples);
    helpers.push({ helper: task.name, search_ms: performance.now() - start, status, term: result ? showTerm(result.program.body) : null });
  }
  rows.push({ repetition, helpers });
}
console.log(JSON.stringify(rows, null, 2));

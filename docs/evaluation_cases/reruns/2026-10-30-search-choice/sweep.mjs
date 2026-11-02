// Runs every saved helper task under candidate search settings and records the term each returns.
// Settings: searchSizeFactor 3 (current) or 1, list components kept or removed, adapter limits
// (maxCost 20, 2 s) or the paper's measurement limits (32, 15 s). Union is also run with lt added.
// With LIST_COST=n and ESCHER_DIST=<dir>, the list components cost n instead of 1, using an escher-ts dist
// whose AscendRec reads per-component costs from globalThis.__refsynCompCost (see README); then only the
// list-kept settings are run.
// Usage (from the repository root): node <this> > <out.json>
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import path from "node:path";

const root = process.cwd();
const dist = process.env.ESCHER_DIST ?? path.join(root, "external/escher-ts/dist");
const listCost = Number(process.env.LIST_COST ?? 1);
const imp = (p) => import(pathToFileURL(path.join(dist, p.replace(/^external\/escher-ts\/dist\//, ""))).href);
const { anyArgSmaller } = await imp("external/escher-ts/dist/components/component.js");
const { parseJsonSynthesisSpec, prepareJsonSynthesisJob } = await imp("external/escher-ts/dist/components/user-friendly-json.js");
const { AscendRecSynthesizer } = await imp("external/escher-ts/dist/synthesis/ascendrec/synthesizer.js");
const { showTerm } = await imp("external/escher-ts/dist/types/term.js");

const R = "docs/evaluation_cases/reruns";
const cases = {
  append: `${R}/2026-10-14-fresh-recording/append/mold_escher_tasks.json`,
  insert: `${R}/2026-10-14-fresh-recording/insert_general/mold_escher_tasks.json`,
  popBack: `${R}/2026-10-14-fresh-recording/popBack/mold_escher_tasks.json`,
  prepend: `${R}/2026-10-14-fresh-recording/prepend/mold_escher_tasks.json`,
  setAt: `${R}/2026-10-14-fresh-recording/setAt/mold_escher_tasks.json`,
  rotateRight3: `${R}/2026-10-15-rotate-right-494d6d4/rotateRight/mold_escher_tasks.json`,
  rotateRight4: `${R}/2026-10-15-rotate-right-4demos-494d6d4/rotateRight/mold_escher_tasks.json`,
  unlink: `${R}/2026-10-30-dll-unlink/unlink/mold_escher_tasks.json`,
  union: `${R}/2026-10-30-uf-union/union/mold_escher_tasks.json`,
};
const listComps = new Set(["last_ptr", "penultimateRef", "nthNextRef", "findByValueRef"]);
if (listCost !== 1) globalThis.__refsynCompCost = Object.fromEntries([...listComps].map((n) => [n, listCost]));
const lt = { name: "lt", kind: "js", inputTypes: ["Int", "Int"], returnType: "Bool", args: ["x", "y"], bodyJs: "return x < y;" };
const limits = { adapter: { maxCost: 20, timeoutMs: 2000 }, paper: { maxCost: 32, timeoutMs: 15000 } };

const configs = [];
for (const factor of [3, 1]) {
  for (const list of listCost === 1 ? ["kept", "removed"] : ["kept"]) {
    configs.push({ factor, list, lt: false });
  }
}

const rows = [];
for (const [caseName, file] of Object.entries(cases)) {
  for (const task of JSON.parse(readFileSync(file, "utf8"))) {
    const runs = configs.concat(caseName === "union" ? configs.map((c) => ({ ...c, lt: true })) : []);
    for (const config of runs) {
      for (const [limitName, limit] of Object.entries(limits)) {
        let components = task.components.filter((c) => config.list === "kept" || !listComps.has(c.name));
        if (config.lt) components = [...components, lt];
        const job = prepareJsonSynthesisJob(parseJsonSynthesisSpec(JSON.stringify({ ...task, components })));
        let status = null;
        const t0 = Date.now();
        const r = new AscendRecSynthesizer({
          ...limit, searchSizeFactor: config.factor, useReductionRules: true, onlyForwardSearch: false,
          argListCompare: anyArgSmaller, onDiagnostics: (d) => { status = d.status; },
        }).synthesize(job.functionName, job.inputTypes, job.inputNames, job.returnType, job.env, job.examples);
        const row = {
          case: caseName, helper: task.name, ...config, listCost, limits: limitName, ms: Date.now() - t0,
          status, level: r?.program.depth ?? null, cost: r?.program.cost ?? null, term: r ? showTerm(r.program.body) : null,
        };
        rows.push(row);
        console.error(`${caseName} ${task.name} f${config.factor} list:${config.list}${config.lt ? " +lt" : ""} ${limitName}: ${row.term ?? status} (${row.ms}ms)`);
      }
    }
  }
}
console.log(JSON.stringify(rows, null, 2));

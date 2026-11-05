// Timing of MOLD and the minimal baseline with the order balanced and a hard limit outside the
// synthesizer. Each trial is one process of ../2026-10-15-fixed-494d6d4/remeasure/measure_method_level.mjs
// with --repetitions 1 (so every trial is a cold start, on both sides alike), killed with SIGKILL if it
// runs 60 s (the synthesizer's own 15 s limit is checked by the search itself with Date.now(), which keeps
// running while the machine sleeps). Rounds 1..10: the method order is rotated by one each round, and
// within each method the side that goes first alternates (MOLD first in odd rounds). Every trial is kept,
// with its start time, load average and outcome; the script exits with code 1 if a trial was killed or
// failed to produce a result. Run it under caffeinate -i (see run_all.sh) so that the machine does not
// sleep. Usage (from the repository root): node <this> > timing_alternating.json
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync } from "node:fs";
import { loadavg, tmpdir } from "node:os";
import path from "node:path";

const D = "docs/evaluation_cases/reruns/2026-11-04-environment-audit";
const M = "docs/evaluation_cases/reruns/2026-10-15-fixed-494d6d4/remeasure/measure_method_level.mjs";
const C = ["--maxCost", "32", "--timeoutMs", "15000", "--repetitions", "1"];
const methods = [
  { method: "setAt", mold: "setAt", names: "setAt-fixed-env-pair" },
  { method: "append", mold: "append", names: "append-fixed-env-pair" },
  { method: "prepend", mold: "prepend", names: "prepend-fixed-env-pair,prepend-fixed-returnRef" },
  { method: "insert", mold: "insert_general", names: "insert-fixed-env-pair" },
  { method: "popBack", mold: "popBack", names: "popBack-fixed-env-pair" },
];
const args = (side, m, out) => side === "mold"
  ? ["--side", "mold", "--method", m.mold, "--tasks", `docs/evaluation_cases/reruns/2026-10-14-fresh-recording/${m.mold}/mold_escher_tasks.json`, ...C, "--out", out]
  : ["--side", "baseline", "--method", m.method, "--tasks", `${D}/tasks_minimal.json`, "--names", m.names, ...C, "--out", out];

const tmp = mkdtempSync(path.join(tmpdir(), "refsyn-timing-"));
const trials = [];
for (let round = 1; round <= 10; round += 1) {
  const order = methods.map((_, k) => methods[(k + round - 1) % methods.length]);
  for (const m of order) {
    const sides = round % 2 === 1 ? ["mold", "baseline"] : ["baseline", "mold"];
    for (const side of sides) {
      const out = path.join(tmp, `${round}-${m.method}-${side}.json`);
      const started = new Date().toISOString();
      const load = loadavg().map((x) => Number(x.toFixed(2)));
      const p = spawnSync("node", [M, ...args(side, m, out)], { timeout: 60000, killSignal: "SIGKILL", encoding: "utf8" });
      const trial = { round, method: m.method, side, started, load, status: p.status, signal: p.signal };
      try {
        const r = JSON.parse(readFileSync(out, "utf8"));
        Object.assign(trial, { solved: r.solved_repetitions === 1, total_ms: r.method_total_ms?.median ?? null, search_ms: r.method_search_ms?.median ?? null, terms: r.repetitions[0].tasks.map((t) => t.term ?? null) });
      } catch (e) {
        trial.error = p.signal ? `killed (${p.signal})` : `no result: ${(p.stderr || e.message).slice(0, 200)}`;
      }
      trials.push(trial);
      console.error(`${round} ${m.method} ${side}: ${trial.error ?? `${trial.solved ? "solved" : "unsolved"} ${trial.total_ms} ms`} (load ${load[0]})`);
    }
  }
}
const summary = methods.map((m) => Object.fromEntries([["method", m.method], ...["mold", "baseline"].map((side) => {
  const ts = trials.filter((t) => t.method === m.method && t.side === side && !t.error);
  const ms = ts.filter((t) => t.solved).map((t) => t.total_ms).sort((a, b) => a - b);
  const median = ms.length ? (ms.length % 2 ? ms[(ms.length - 1) / 2] : (ms[ms.length / 2 - 1] + ms[ms.length / 2]) / 2) : null;
  return [side, { trials: ts.length, solved: ms.length, median_total_ms: median, min: ms[0] ?? null, max: ms.at(-1) ?? null }];
})]));
console.log(JSON.stringify({ node: process.version, summary, trials }, null, 2));
if (trials.some((t) => t.error)) process.exitCode = 1;

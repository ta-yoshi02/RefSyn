#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { performance } from "node:perf_hooks";
import { pathToFileURL } from "node:url";

const usage = () => {
  const script = path.relative(process.cwd(), process.argv[1]);
  console.error(
    [
      `Usage: node ${script} --file tasks.json [--out results.json] [--csv runtime.csv]`,
      "  --maxCost N       Override ESCHER_TS_MAX_COST for every task",
      "  --timeoutMs N     Override ESCHER_TS_TIMEOUT_MS for every task",
      "  --repetitions N   Run each task N times; default: 1",
    ].join("\n"),
  );
};

const parseIntOption = (name, raw, fallback = null) => {
  if (raw === undefined) {
    return fallback;
  }
  const parsed = Number.parseInt(raw, 10);
  if (!Number.isFinite(parsed) || parsed < 0) {
    throw new Error(`${name} must be a non-negative integer`);
  }
  return parsed;
};

const parseArgs = (argv) => {
  const opts = {
    file: null,
    out: null,
    csv: null,
    maxCost: null,
    timeoutMs: null,
    repetitions: 1,
  };

  for (let i = 2; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--file") {
      opts.file = argv[++i];
    } else if (arg === "--out") {
      opts.out = argv[++i];
    } else if (arg === "--csv") {
      opts.csv = argv[++i];
    } else if (arg === "--maxCost") {
      opts.maxCost = parseIntOption("--maxCost", argv[++i]);
    } else if (arg === "--timeoutMs") {
      opts.timeoutMs = parseIntOption("--timeoutMs", argv[++i]);
    } else if (arg === "--repetitions") {
      opts.repetitions = parseIntOption("--repetitions", argv[++i], 1);
      if (opts.repetitions < 1) {
        throw new Error("--repetitions must be at least 1");
      }
    } else if (arg === "--help" || arg === "-h") {
      usage();
      process.exit(0);
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }

  if (!opts.file) {
    throw new Error("--file is required");
  }
  return opts;
};

const quoteCsv = (value) => {
  const text = value === null || value === undefined ? "" : String(value);
  return /[",\n]/.test(text) ? `"${text.replaceAll('"', '""')}"` : text;
};

const summarizeRuns = (runs) => {
  const elapsed = runs.map((run) => run.elapsed_ms);
  const sorted = elapsed.slice().sort((a, b) => a - b);
  const sum = elapsed.reduce((acc, x) => acc + x, 0);
  return {
    min_ms: sorted[0],
    median_ms: sorted[Math.floor(sorted.length / 2)],
    max_ms: sorted[sorted.length - 1],
    mean_ms: sum / sorted.length,
  };
};

const main = async () => {
  let opts;
  try {
    opts = parseArgs(process.argv);
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    usage();
    process.exit(1);
  }

  const adapterPath = path.join(process.cwd(), "runtime", "refsyn-escher-adapter.mjs");
  const { runRefsynTasks } = await import(pathToFileURL(adapterPath).href);
  const tasks = JSON.parse(fs.readFileSync(opts.file, "utf8"));
  if (!Array.isArray(tasks)) {
    throw new Error("task file must contain a JSON array");
  }

  const options = {};
  if (opts.maxCost !== null) {
    options.maxCost = opts.maxCost;
  }
  if (opts.timeoutMs !== null) {
    options.timeoutMs = opts.timeoutMs;
  }
  options.quiet = true;

  const results = [];
  for (const task of tasks) {
    const taskName = task.name ?? "synthesized";
    const runs = [];
    for (let iteration = 1; iteration <= opts.repetitions; iteration += 1) {
      const started = performance.now();
      const [outcome] = runRefsynTasks([task], options);
      const elapsedMs = performance.now() - started;
      runs.push({
        iteration,
        elapsed_ms: Number(elapsedMs.toFixed(3)),
        success: outcome.success,
        rendered: outcome.rendered,
        error: outcome.error,
        compiled_js: outcome.compiled_js,
      });
    }
    const last = runs[runs.length - 1];
    results.push({
      task: taskName,
      config: {
        maxCost: opts.maxCost,
        timeoutMs: opts.timeoutMs,
        repetitions: opts.repetitions,
      },
      success: runs.every((run) => run.success),
      summary: summarizeRuns(runs),
      last_rendered: last.rendered,
      last_error: last.error,
      runs,
    });
  }

  const jsonText = `${JSON.stringify(results, null, 2)}\n`;
  if (opts.out) {
    fs.writeFileSync(opts.out, jsonText);
  } else {
    process.stdout.write(jsonText);
  }

  if (opts.csv) {
    const lines = [
      "task,iteration,success,elapsed_ms,max_cost,timeout_ms,error",
      ...results.flatMap((result) =>
        result.runs.map((run) =>
          [
            result.task,
            run.iteration,
            run.success,
            run.elapsed_ms,
            opts.maxCost ?? "",
            opts.timeoutMs ?? "",
            quoteCsv(run.error ?? ""),
          ].join(","),
        ),
      ),
    ];
    fs.writeFileSync(opts.csv, `${lines.join("\n")}\n`);
  }
};

main().catch((error) => {
  console.error(error instanceof Error ? error.stack : String(error));
  process.exit(1);
});

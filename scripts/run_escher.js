#!/usr/bin/env node
"use strict";

const fs = require("fs");
const path = require("path");
const { spawnSync } = require("child_process");
const { pathToFileURL } = require("url");

function usage() {
  const script = path.relative(process.cwd(), __filename);
  console.error(
    [
      `Usage: node ${script} [--file specs.json] [--quiet]`,
      "  Uses ESCHER_BACKEND=ts|scala (default: ts)",
      "  --file   Read JSON from file instead of stdin",
      "  --quiet  Suppress backend-side noise where supported",
    ].join("\n")
  );
}

function parseArgs(argv) {
  const opts = {
    filePath: null,
    quiet: false,
    passthrough: [],
  };

  for (let i = 2; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--file") {
      if (!argv[i + 1]) {
        throw new Error("Missing value for --file");
      }
      opts.filePath = argv[i + 1];
      opts.passthrough.push(arg, argv[i + 1]);
      i += 1;
    } else if (arg === "--quiet") {
      opts.quiet = true;
      opts.passthrough.push(arg);
    } else if (arg === "--help" || arg === "-h") {
      usage();
      process.exit(0);
    } else if (arg === "--module") {
      if (!argv[i + 1]) {
        throw new Error("Missing value for --module");
      }
      opts.passthrough.push(arg, argv[i + 1]);
      i += 1;
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }

  return opts;
}

function readInput(filePath) {
  if (filePath) {
    return fs.readFileSync(filePath, "utf8");
  }
  return fs.readFileSync(0, "utf8");
}

function runScalaBackend(opts) {
  const scalaScript = path.join(__dirname, "run_escher_scala.js");
  const child = spawnSync(process.execPath, [scalaScript, ...opts.passthrough], {
    stdio: "inherit",
    env: process.env,
  });
  process.exit(child.status === null ? 1 : child.status);
}

async function runTsBackend(opts) {
  const distModule = path.join(__dirname, "..", "external", "escher-ts", "dist", "refsyn.js");
  if (!fs.existsSync(distModule)) {
    console.error(
      `escher-ts build output not found at ${distModule}. Run 'pnpm install && pnpm build' in external/escher-ts first.`
    );
    process.exit(1);
  }

  const { runRefsynTasksJson } = await import(pathToFileURL(distModule).href);
  const inputJson = readInput(opts.filePath);
  const outputJson = runRefsynTasksJson(inputJson, { quiet: opts.quiet });
  process.stdout.write(outputJson);
}

async function main() {
  let opts;
  try {
    opts = parseArgs(process.argv);
  } catch (err) {
    console.error(err.message);
    usage();
    process.exit(1);
  }

  const backend = (process.env.ESCHER_BACKEND || "ts").trim().toLowerCase();
  if (backend === "scala") {
    runScalaBackend(opts);
    return;
  }
  if (backend !== "ts" && backend !== "") {
    console.error(`Unsupported ESCHER_BACKEND='${backend}'. Expected 'ts' or 'scala'.`);
    process.exit(1);
  }

  await runTsBackend(opts);
}

main().catch((err) => {
  console.error(err && err.message ? err.message : String(err));
  process.exit(1);
});

#!/usr/bin/env node
"use strict";

const fs = require("fs");
const path = require("path");
const { pathToFileURL } = require("url");

function usage() {
  const script = path.relative(process.cwd(), __filename);
  console.error(
    [
      `Usage: node ${script} [--file tasks.json] [--quiet]`,
      "  --file   Read JSON from file instead of stdin",
      "  --quiet  Suppress backend-side noise where supported",
    ].join("\n")
  );
}

function parseArgs(argv) {
  const opts = {
    filePath: null,
    quiet: false,
  };

  for (let i = 2; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--file") {
      if (!argv[i + 1]) {
        throw new Error("Missing value for --file");
      }
      opts.filePath = argv[i + 1];
      i += 1;
    } else if (arg === "--quiet") {
      opts.quiet = true;
    } else if (arg === "--help" || arg === "-h") {
      usage();
      process.exit(0);
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

async function runTsBackend(opts) {
  const distRoot = path.join(__dirname, "..", "external", "escher-ts", "dist");
  const distModule = path.join(distRoot, "index.js");
  if (!fs.existsSync(distModule)) {
    console.error(
      `escher-ts build output not found at ${distModule}. Run 'pnpm install && pnpm build' in external/escher-ts first.`
    );
    process.exit(1);
  }

  const adapterModule = path.join(__dirname, "..", "runtime", "refsyn-escher-adapter.mjs");
  const { runRefsynTasksJson } = await import(pathToFileURL(adapterModule).href);
  const inputJson = readInput(opts.filePath);
  process.stdout.write(runRefsynTasksJson(inputJson, { quiet: opts.quiet }));
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

  await runTsBackend(opts);
}

main().catch((err) => {
  console.error(err && err.message ? err.message : String(err));
  process.exit(1);
});

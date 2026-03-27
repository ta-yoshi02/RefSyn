#!/usr/bin/env node
"use strict";

// Node wrapper around Escher-Scala's Scala.js bundle.
// Reads JSON specs from stdin or a file and emits normalized results to stdout.

const fs = require("fs");
const path = require("path");

function usage() {
  const script = path.relative(process.cwd(), __filename);
  console.error(
    [
      `Usage: node ${script} [--file specs.json] [--module path/to/escher-scala-opt.js] [--quiet]`,
      "  --file    Read specs JSON from file instead of stdin",
      "  --module  Path to escher-scala-opt.js (defaults to Escher-Scala/target/scala-2.12/... )",
      "  --quiet   Suppress Scala-side console.log noise (still sent to stderr if not quiet)",
    ].join("\n")
  );
}

function parseArgs(argv) {
  const defaults = {
    modulePath: path.join(
      __dirname,
      "..",
      "Escher-Scala",
      "target",
      "scala-2.12",
      "escher-scala-opt.js"
    ),
    filePath: null,
    quiet: false,
  };

  const opts = { ...defaults };
  for (let i = 2; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--module") {
      if (!argv[i + 1]) {
        throw new Error("Missing value for --module");
      }
      opts.modulePath = argv[i + 1];
      i += 1;
    } else if (arg === "--file") {
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

function normalizeText(val) {
  if (val === null || val === undefined) {
    return null;
  }
  if (Array.isArray(val)) {
    if (val.length === 0) {
      return null;
    }
    return val.map((v) => String(v)).join("\n");
  }
  if (typeof val === "string") {
    return val;
  }
  return String(val);
}

function normalizeOutcome(outcome) {
  return {
    name: outcome.name || "",
    success: Boolean(outcome.success),
    rendered: normalizeText(outcome.rendered),
    error: normalizeText(outcome.error),
  };
}

function main() {
  let opts;
  try {
    opts = parseArgs(process.argv);
  } catch (err) {
    console.error(err.message);
    usage();
    process.exit(1);
  }

  const inputJson = readInput(opts.filePath);

  const resolvedModule = path.resolve(opts.modulePath);
  let escher;
  try {
    escher = require(resolvedModule);
  } catch (err) {
    console.error(`Failed to require module at ${resolvedModule}: ${err.message}`);
    process.exit(1);
  }

  const originalLog = console.log;
  const originalWarn = console.warn;
  const forward = (args) => {
    const msg = args.join(" ");
    if (!opts.quiet) {
      process.stderr.write(`${msg}\n`);
    }
  };
  console.log = (...args) => forward(args);
  console.warn = (...args) => forward(args);

  let raw;
  try {
    raw = escher.runSynthesisJson(inputJson);
  } catch (err) {
    console.log = originalLog;
    console.warn = originalWarn;
    console.error(`runSynthesisJson failed: ${err.message}`);
    process.exit(1);
  }

  console.log = originalLog;
  console.warn = originalWarn;

  let parsed;
  try {
    parsed = JSON.parse(raw);
  } catch (err) {
    console.error(`Failed to parse Escher output as JSON: ${err.message}`);
    process.exit(1);
  }

  if (!Array.isArray(parsed)) {
    console.error("Escher output was not an array of results");
    process.exit(1);
  }

  const normalized = parsed.map(normalizeOutcome);
  process.stdout.write(JSON.stringify(normalized));
}

main();

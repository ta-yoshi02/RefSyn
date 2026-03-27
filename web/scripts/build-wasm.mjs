import { mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const webDir = resolve(scriptDir, "..");
const repoRoot = resolve(webDir, "..");
const wasmOutDir = resolve(webDir, "pkg");
const wasmInput = resolve(
  repoRoot,
  "target/wasm32-unknown-unknown/release/refsyn.wasm",
);

const run = (command, args, cwd = repoRoot) => {
  const result = spawnSync(command, args, {
    cwd,
    stdio: "inherit",
  });
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
};

const ensureTool = (tool) => {
  const result = spawnSync(tool, ["--version"], { stdio: "ignore" });
  if (result.status !== 0) {
    console.error(
      `${tool} is required. Install wasm-bindgen-cli or wasm-pack before running the web build.`,
    );
    process.exit(1);
  }
};

mkdirSync(wasmOutDir, { recursive: true });
run("cargo", ["build", "--release", "--lib", "--target", "wasm32-unknown-unknown"]);
ensureTool("wasm-bindgen");
run("wasm-bindgen", [wasmInput, "--out-dir", wasmOutDir, "--target", "web"]);

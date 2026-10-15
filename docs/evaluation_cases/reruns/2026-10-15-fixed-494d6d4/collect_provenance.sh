#!/bin/sh
# Writes the revisions, tool versions and hashes of the files a recording ran on.
# Usage: collect_provenance.sh <out-file>   (run from the repository root)
set -eu
out="$1"
{
  echo "collected_at_utc: $(date -u +%FT%TZ)"
  echo "platform: $(uname -srm)"
  echo
  echo "## Revisions"
  echo "refsyn: $(git rev-parse HEAD)"
  echo "refsyn_dirty_tracked_files: $(git status --porcelain --untracked-files=no | wc -l | tr -d ' ')"
  git submodule status --recursive | sed 's/^/submodule: /'
  echo
  echo "## Tools"
  echo "rustc: $(rustc -V)"
  echo "cargo: $(cargo -V)"
  echo "node: $(node -v)"
  echo "npm: $(npm -v)"
  echo "pnpm: $(pnpm -v)"
  echo
  echo "## Environment variables read by the native path"
  env | grep -E '^(ESCHER_TS|REFSYN)_' || echo "(none set: ESCHER_TS_MAX_COST=20, ESCHER_TS_TIMEOUT_MS=2000, ESCHER_TS_SEARCH_SIZE_FACTOR=3 defaults in runtime/refsyn-escher-adapter.mjs)"
  echo
  echo "## Lockfiles"
  shasum -a 256 Cargo.lock web/package-lock.json external/escher-ts/pnpm-lock.yaml Kanon/package-lock.json 2>/dev/null || true
  echo
  echo "## Native backend binary and the JS it runs"
  shasum -a 256 target/debug/refsyn scripts/run_escher.js runtime/*.mjs
  echo
  echo "## escher-ts dist (sorted file hashes, then their combined hash)"
  (cd external/escher-ts/dist && find . -type f | LC_ALL=C sort | xargs shasum -a 256) > /tmp/refsyn_dist_hashes.$$
  echo "files: $(wc -l < /tmp/refsyn_dist_hashes.$$ | tr -d ' ')"
  echo "combined: $(shasum -a 256 < /tmp/refsyn_dist_hashes.$$ | cut -d' ' -f1)"
  rm -f /tmp/refsyn_dist_hashes.$$
  echo
  echo "## WASM package (built by npm run test:rebuild; not used by the native path)"
  shasum -a 256 web/pkg/* 2>/dev/null || echo "(web/pkg absent)"
  echo
  echo "## Kanon page sources served on :8001"
  (cd Kanon && shasum -a 256 index.html src/js/testize.js src/js/eventHandler.js)
} > "$out"

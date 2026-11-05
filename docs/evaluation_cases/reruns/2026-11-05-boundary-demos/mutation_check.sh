#!/bin/sh
# Checks that check_methods.mjs rejects broken methods: copies the recordings' source_after.js to a
# temporary directory, breaks append in two ways (an added `return null`; the stored value replaced by
# -999) and runs the checker on each copy. Both runs must fail with exit code 1.
# Usage (from the repository root): sh <this> > mutation_log.txt 2>&1
set -u
D=docs/evaluation_cases/reruns/2026-11-05-boundary-demos
T=$(mktemp -d)
trap 'rm -rf "$T"' EXIT
status=0
for m in return_null val_minus_999; do
  for c in rotateLeft rotateRight prepend append append_older_tail insertAfter; do
    mkdir -p "$T/$m/$c" && cp "$D/$c/source_after.js" "$T/$m/$c/"
  done
  case $m in
    return_null) sed -i.bak 's/^        hole1.next = created0;$/        hole1.next = created0;\n        return null;/' "$T/$m/append/source_after.js" ;;
    val_minus_999) sed -i.bak 's/^        created0.val = hole0;$/        created0.val = -999;/' "$T/$m/append/source_after.js" ;;
  esac
  cmp -s "$D/append/source_after.js" "$T/$m/append/source_after.js" && { echo "$m: mutation not applied"; status=1; continue; }
  summary=$(CHECK_DIR="$T/$m" node "$D/check_methods.mjs" 2>/dev/null | node -e 'let s="";process.stdin.on("data",d=>s+=d).on("end",()=>console.log(JSON.stringify(JSON.parse(s).summary)))')
  CHECK_DIR="$T/$m" node "$D/check_methods.mjs" >/dev/null 2>&1; code=$?
  echo "$m: exit $code $summary"
  [ "$code" -eq 1 ] || status=1
done
exit $status

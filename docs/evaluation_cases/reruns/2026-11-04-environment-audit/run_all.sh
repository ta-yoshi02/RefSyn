#!/bin/sh
# Runs every experiment of this directory once, under caffeinate -i (no idle sleep while it runs; it
# does not prevent the sleep of a closed lid on battery), and keeps the run: all outputs, logs, exit
# codes and load averages are copied to runs/<UTC time>/, whether the scripts succeeded or not, so that
# a failed or diverging run is not overwritten by a later one. The top-level outputs are those of the
# latest run. Exits with code 1 if any script failed.
# Usage (from the repository root): WEIGHTED_DIST=<patched dist> sh <this>
set -u
D=docs/evaluation_cases/reruns/2026-11-04-environment-audit
RUN=$D/runs/$(date -u +%Y%m%dT%H%M%SZ)
mkdir -p "$RUN"
fail=0
step() { # name, output, stderr log, command...
  name=$1; out=$2; log=$3; shift 3
  echo "$(date -u +%H:%M:%S) start $name $(uptime | sed 's/.*load/load/')" >> "$RUN/conditions.txt"
  "$@" > "$out" 2> "$log"
  code=$?
  echo "$(date -u +%H:%M:%S) end   $name exit $code" >> "$RUN/conditions.txt"
  cp "$out" "$log" "$RUN/" 2>/dev/null
  [ $code -eq 0 ] || fail=1
}
caffeinate -i -w $$ &
step rerun_fields $D/rerun_fields.json $D/rerun_fields_log.txt node $D/rerun_fields.mjs
step offdemo_fields $D/offdemo_fields.json $D/offdemo_fields_log.txt node $D/check_offdemo_fields.mjs
step rerun_minimal $D/rerun_minimal.json $D/rerun_minimal_log.txt node $D/rerun_minimal.mjs
step offdemo_minimal $D/offdemo_minimal.json $D/offdemo_minimal_log.txt node $D/check_offdemo_minimal.mjs
step cegis_minimal $D/cegis_minimal.json $D/cegis_minimal_log.txt node $D/cegis_minimal.mjs
step timing $D/timing_alternating.json $D/timing_alternating_log.txt node $D/measure_alternating.mjs
pmset -g log | grep -E " (Sleep|Wake|DarkWake) +	" | tail -20 > "$RUN/pmset_tail.txt"
echo "fail=$fail" >> "$RUN/conditions.txt"
exit $fail

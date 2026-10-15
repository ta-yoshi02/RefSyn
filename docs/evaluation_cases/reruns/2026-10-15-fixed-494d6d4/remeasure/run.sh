#!/bin/sh
# Main measurement of the five cases at 494d6d4, same conditions as 2026-10-14-fresh-recording.
# Run from the repository root.
set -eu
D=docs/evaluation_cases/reruns/2026-10-15-fixed-494d6d4/remeasure
B=docs/evaluation_cases/nonseparated_general_components/tasks.json
L=$D/run_log.txt
C="--maxCost 32 --timeoutMs 15000 --repetitions 10"
{ echo "started_utc=$(date -u +%FT%TZ)"; echo "uptime_before: $(uptime)"; } > $L
for c in insert_general append popBack prepend setAt; do
  node $D/measure_method_level.mjs --side mold --method $c --tasks $D/$c/tasks_494d6d4.json $C --out $D/mold_$c.json >> $L
done
for m in insert append popBack setAt; do
  node $D/measure_method_level.mjs --side baseline --method $m --tasks $B --names $m-fixed-env-pair $C --out $D/baseline_$m.json >> $L
done
node $D/measure_method_level.mjs --side baseline --method prepend --tasks $B --names prepend-fixed-env-pair,prepend-fixed-returnRef $C --out $D/baseline_prepend.json >> $L
{ echo "uptime_after: $(uptime)"; echo "finished_utc=$(date -u +%FT%TZ)"; } >> $L

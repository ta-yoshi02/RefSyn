#!/bin/sh
set -eu
D=docs/evaluation_cases/reruns/2026-10-30-followthrough
M=docs/evaluation_cases/reruns/2026-10-15-fixed-494d6d4/remeasure/measure_method_level.mjs
B=docs/evaluation_cases/reruns/2026-10-30-baseline-recheck/tasks_list.json
C="--maxCost 32 --timeoutMs 15000 --repetitions 10"
date -u
uptime
for m in insert_general append popBack prepend setAt; do
 node "$M" --side mold --method "$m" --tasks "docs/evaluation_cases/reruns/2026-10-14-fresh-recording/$m/mold_escher_tasks.json" $C --out "$D/mold_$m.json"
done
for m in insert append popBack setAt; do
 node "$M" --side baseline --method "$m" --tasks "$B" --names "$m-fixed-env-pair" $C --out "$D/baseline_$m.json"
done
node "$M" --side baseline --method prepend --tasks "$B" --names prepend-fixed-env-pair,prepend-fixed-returnRef $C --out "$D/baseline_prepend.json"
uptime
date -u

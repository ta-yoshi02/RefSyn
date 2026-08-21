# Non-separated baseline with general heap components

## Purpose

This experiment separates component-language incompleteness from search failure in
the non-separated fixed-ID baseline.  It adds only two operations that are generic
to the chosen state representation:

```text
storeAtRef[A,C] : List[A] -> Ref[Object[C]] -> A -> List[A]
freshRefFromHeap[C] : List[Object[C]] -> Ref[Object[C]]
```

No component names or implements any benchmark method.  The original operation
records remain unavailable to the baseline.

## Artifacts

| File | Role |
| --- | --- |
| `tasks.json` | all 16 fixed-ID output tasks with the two general components added |
| `decisive_tasks.json` | six tasks needed to decide the five complete methods |
| `witness_check.json` | hand-written typed terms, term costs, and saved-example checks |
| `all_results_cost32_timeout15s.json` | three synthesis runs for all 16 tasks |
| `all_runtime_cost32_timeout15s.csv` | per-run runtime data for all 16 tasks |
| `decisive_results_cost14_timeout15s.json` | same-budget component-addition comparison |
| `decisive_results_cost32_timeout15s.json` | witness-feasible-cost comparison |

All files were generated on 2026-08-01 with the current local Escher-ts build.

## Representability check

Every hand-written term was executed by the same typed component evaluator used by
the synthesizer.  All 41 task/example cells matched.  The maximum witness cost is
32.

| Method/output | Witness cost |
| --- | ---: |
| `setAt` whole field environment | 10 |
| `append` whole field environment | 28 |
| `prepend` whole field environment | 11 |
| `prepend` return reference | 2 |
| `insert` whole field environment | 32 |
| `popBack` whole field environment | 13 |

Thus the original `maxCost=14` excludes the correct `append` and `insert` whole-
environment witnesses even after the component set is made complete.  The
`maxCost=32` run includes every hand-written witness.

## Synthesis results

All measurements use `timeoutMs=15000`, `repetitions=3`, AscendRec, the same saved
examples, and the same non-separated fixed-ID input/output encoding.

| Complete-method requirement | Original components, cost 14 | General components, cost 14 | General components, cost 32 | Median at cost 32 |
| --- | --- | --- | --- | ---: |
| `setAt` field environment | timeout | success | success | 3740.013 ms |
| `append` field environment | timeout | timeout | timeout | 15002.511 ms |
| `prepend` field environment | success | success | success | 276.724 ms |
| `prepend` return reference | timeout | success | success | 0.175 ms |
| `insert` field environment | timeout | timeout | timeout | 15005.863 ms |
| `popBack` field environment | timeout | success | success | 5151.638 ms |

For the all-output cost-32 run, 13 of 16 tasks succeeded.  The three failures were:

- `append-fixed-env-pair`;
- `insert-fixed-gHeap`;
- `insert-fixed-env-pair`.

`append` is diagnostically important: its value heap and pointer heap were each
synthesized separately (median 1.982 ms and 32.271 ms), while their whole-state
pair timed out in every run.  `insert` is harder: its value heap succeeded, but the
pointer heap and whole-state pair both timed out.

## Interpretation limits

The timeout for `append` and `insert` is now a bounded search result rather than a
component-language impossibility: a typed correct term exists, its cost is within
the configured bound, and it matches every supplied example.

The synthesized successes must not be presented as generally correct methods.
Several are accidental fits to the saved examples:

- `prepend` and the value heaps use `insert(heap, value, value)`, which appends only
  because the recorded values exceed the heap lengths;
- `setAt` branches on the three recorded indices and uses list-position shortcuts;
- `popBack` branches on fixed list positions visible in the two recorded heaps.

Accordingly, the supported claim is limited to the supplied specifications and
configuration: after adding general state-construction components, the
non-separated `append` and `insert` whole-state tasks remained unsolved within
15 seconds at `maxCost=32`, whereas their hand-written witnesses existed in the
language.  This does not prove that all non-separated mutation synthesis is hard,
nor that the successful synthesized terms generalize to arbitrary heaps.

## Reproduction

```sh
cd external/escher-ts
pnpm test:run
pnpm typecheck
pnpm build
cd ../..
node scripts/prepare_nonseparated_general_baseline.mjs
node scripts/measure_escher_tasks.mjs \
  --file docs/evaluation_cases/nonseparated_general_components/tasks.json \
  --out docs/evaluation_cases/nonseparated_general_components/all_results_cost32_timeout15s.json \
  --csv docs/evaluation_cases/nonseparated_general_components/all_runtime_cost32_timeout15s.csv \
  --maxCost 32 --timeoutMs 15000 --repetitions 3
```

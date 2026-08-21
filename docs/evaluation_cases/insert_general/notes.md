# Notes

## What This Case Shows

This case is useful because the MOLD result uses `arg0` in both pointer helpers:

```text
insert-h = nthNextRef(..., arg0)
insert-i = nthNextRef(..., inc(arg0))
```

That makes it stronger than the earlier case where the result was effectively tail-oriented.

The MOLD helper tasks remain synthesizable under fixed object-ID heaps. The replayed fixed-ID helper tasks are stored in `mold_fixed_id_escher_tasks.json`, with results in `mold_fixed_id_results.json`.

The current paper-facing measurement is stored in `mold_results_measured.json` and `baseline_fixed_id_results_measured.json`. Both use `maxCost=14`, `timeoutMs=15000`, and `repetitions=3`.

Summary:

| side | task | result | median runtime |
| --- | --- | --- | ---: |
| MOLD | `insert-f` | success: `@arg1` | 0.583 ms |
| MOLD | `insert-h` | success: `nthNextRef(..., arg0)` | 9.374 ms |
| MOLD | `insert-i` | success: `nthNextRef(..., inc(arg0))` | 17.142 ms |
| baseline | `insert-fixed-fHeap` | success: append-like `insert(@fHeap, @arg1, @arg1)` | 20.312 ms |
| baseline | `insert-fixed-gHeap` | failure: timeout | 15000.294 ms |
| baseline | `insert-fixed-env-pair` | failure: timeout | 15000.360 ms |

## What It Does Not Show

Three traces are still not enough to prove general insert behavior. They show that the current system can infer the intended `i`-dependent helper functions for this recorded operation set.

The list-environment baseline is sensitive to representation. The paper-facing baseline must use fixed object-ID field heaps, because only that encoding can be inverted back to the object graph without silently changing object identity.

## Baseline Interpretation

Without `freshRefFromHeap`, the pointer heap and whole pair tasks fail. This is the cleanest evidence that plain Escher-ts components do not naturally express allocation into a reference heap.

The earlier chain-order baseline made `gHeap` look too easy because it reordered the heap into traversal order. That representation should be treated as a diagnostic dead end, not as evaluation evidence.

With fixed object IDs, `gHeap` must encode both rewiring the existing predecessor and initializing the new object's pointer. Under the current component set, `insert-fixed-gHeap` and `insert-fixed-env-pair` fail within the 15s budget.

The honest interpretation is:

```text
MOLD obtains small helper tasks automatically from operations. A non-separated fixed-ID list-environment baseline must synthesize allocation-sensitive pointer rewiring as a whole and fails under the current component set.
```

Do not describe this as "Escher-ts cannot represent list insertion." It can synthesize some list transformations, and it does synthesize the fixed-ID `fHeap` output here. The observed failure is specifically the fixed-ID pointer heap update under the current component set and search budget.

Also do not count `insert-fixed-fHeap` as meaningful evidence for baseline success. The synthesized term uses `arg1` as the insertion index. That only appends because all recorded inserted values are larger than the heap length.

## Open Work

- Repeat this for at least `append`, `setAt`, and one deletion-like operation.
- Decide whether the baseline is allowed to use `freshRefFromHeap`.
- If it is allowed, explicitly count it as a baseline helper not present in ordinary Escher-ts.
- Measure with fixed budgets and multiple repetitions before using runtime numbers in the main paper.

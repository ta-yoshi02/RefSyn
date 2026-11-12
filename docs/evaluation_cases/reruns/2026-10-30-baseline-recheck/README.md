# 2026-10-30: re-examination of the paper's non-separated baseline

Subject: `../../nonseparated_general_components/` (tasks of 2026-08-01, used unchanged by the paper's
measurements `../2026-10-14-fresh-recording/` and `../2026-10-15-fixed-494d6d4/`: maxCost 32, 15 s,
`*-fixed-env-pair` and `prepend-fixed-returnRef`). Question raised by `../2026-10-30-baseline/`: the
library's list components are typed `Ref[T] × List[T]` (escher-ts `bb77750`, before the `e187df0` used on
2026-08-01), while these tasks type `gHeap` as `List[Ref[Object[Obj]]]`. Nothing in `src`, `runtime`,
escher-ts or the published task files was changed.

State: RefSyn `a21e42c` (checkout on `fix/translation-success-contract`, switched by another session; not
used here), escher-ts `4e7c4bc`. Everything here uses only `external/escher-ts/dist`.

## 1. The hand-written witnesses are not well-typed (`typecheck_witnesses.mjs`)

The 2026-08-01 check ran the witnesses with `executeTerm`, which does not type-check. Checked against the
task signatures and component types the synthesizer uses (unification as in escher-ts' `unify`):

- **9 of 16 witnesses are ill-typed**: setAt fHeap / pair, append gHeap / pair, insert gHeap / pair,
  popBack gHeap / pair (and setAt pair). Every error is the same: `last_ptr`, `penultimateRef` or
  `nthNextRef` expects `List[Object<Obj>]` and gets `gHeap : List[Ref[Object<Obj>]]`.
- None of the three list components can be applied to `gHeap` in any of the 16 tasks
  (`typecheck_witnesses.json`, `probes`). None of the terms found on 2026-08-01 uses one.

So the README's statement "a typed correct term exists, its cost is within the configured bound" is false
for append and insert, and the paper draft's reason for maxCost 32 ("the maximum cost of the hand-written
terms") rests on terms outside the search space. The baseline also effectively ran without the list
components that MOLD's helpers use (`nthNextRef(@thisRef, @nodeHeap, @nextHeap, inc(@i))`).

## 2. With the list components usable (`make_list_tasks.mjs` → `tasks_list.json`)

The three components replaced by the JS ports of `../2026-10-30-baseline/` (same names and semantics,
typed for heaps of references; 0 differences from the library on 20 000 heaps). Then all 16 witnesses are
well-typed (`typecheck_witnesses_list.json`) with unchanged costs (max 32).

## 3. Search results (`rerun.mjs` → `rerun_results.json`, `rerun_log.txt`)

maxCost 32, 15 s, one run each. "current" = escher-ts `4e7c4bc` as in the paper (searchSizeFactor 3);
"candidate" = factor 1, list components cost 3 (`../2026-10-30-new-cases/plan.md`). The published/current
column reproduces the 2026-08-01 and paper results term for term.

## 4. Found terms on lists that were not demonstrated (`check_offdemo.mjs` → `offdemo_results.json`)

Lists of length 1–6 (popBack 2–6), laid out in list order and reversed; every position for setAt and
insert (insert includes the last node); x ∈ {0, 1, 2, 3, 99}. `check_witnesses_offdemo.mjs` validates the
checker: every witness (with ports) agrees on every input (`check_witnesses_offdemo.txt`).

| Paper's decisive task | Published, current (paper) | Published, candidate | +list, current | +list, candidate |
| --- | --- | --- | --- | --- |
| setAt pair | ok, **16/42** (branches on index 0 / 2) | timeout | ok, **42/42** (`nthNextRef`) | timeout |
| append pair | timeout | timeout | timeout | timeout |
| prepend pair | ok, **24/60** (`insert(@fHeap, @arg0, @arg0)`) | ok, 24/60 | ok, 24/60 | ok, 24/60 |
| prepend returnRef | ok, 60/60 | ok, 60/60 | ok, 60/60 | ok, 60/60 |
| insert pair | timeout | timeout | timeout | timeout |
| popBack pair | ok, **2/10** (fixed positions) | timeout | ok, **1/10** (`penultimateRef`, null read at a fixed position) | timeout |

Component tasks of append / insert: fHeap is solved everywhere by `insert(@fHeap, @x, @x)` (value used as
index; 24/60, 62/210), append gHeap by `insert(@gHeap, dec(length(@nodeHeap)), fresh)` (35/60, fails when
the tail is not the last index); insert gHeap times out in every configuration.

For comparison, MOLD's five generated methods agree with the reference on 97/97 inputs within the
demonstrated condition (`../2026-10-15-off-demo-five-cases/`, other inputs and checker; not rerun here).

## What this means for the paper

- **The × for append and insert stands**, and is now justified: with the list components usable, the
  correct witnesses are well-typed and within cost 32, and the search still times out (both settings).
  But the published justification is wrong and must be replaced, and the tasks used must be the
  `tasks_list.json` kind, or the text must say the baseline lacks the traversal components.
- **The ○ for setAt, prepend and popBack measures only "satisfies the examples"**. Of those, only setAt
  with the list components yields a correct method; the published setAt (16/42), prepend (24/60) and
  popBack (2/10) are accidental fits — the 2026-08-01 README says so, the paper table does not. On "the
  generated method is correct", the baseline gets 0/5 as published and 1/5 with the list components; MOLD
  5/5. The current table understates the difference and rests on a criterion that the baseline passes by
  overfitting.
- The candidate search setting makes the baseline slower (setAt and popBack pairs time out). If that
  setting is adopted, both sides must be remeasured.

## Not covered

- One run per configuration; times are indicative. No repetition of 10 as in the paper.
- The ported list components are JS components; their speed differs from the library ones, which could
  shift times but not well-typedness.
- The insert tasks declare `f : Ref[Int]` and give objects without fields, unlike the other four; left as is.

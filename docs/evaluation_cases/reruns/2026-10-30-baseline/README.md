# 2026-10-30: non-separated baseline on the seven non-list cases

The "naive" conversion: each method becomes one PBE problem per output, from the whole pre-call state to
the whole post-call state, without splitting by the demonstrated operations. Same recordings as MOLD
(rotateRight with 3 and 4 demonstrations, unlink, union, swapChildren, insertAfter, rotateLeft), same
settings (current / candidate of `../2026-10-30-new-cases/plan.md`), same limits (adapter 20 / 2 s, paper
32 / 15 s). Nothing in `src`, `runtime` or escher-ts was changed.

## Tasks (`make_tasks.mjs` → `tasks.json`)

- Input: `thisRef`, `nodeHeap`, every field heap, the method's own arguments. Pointer heaps are lists of
  references, as in `../../nonseparated_general_components/`. Objects the method creates are not in the
  input.
- Outputs, one task each: every field heap the method changes (created objects appended), the return
  reference if any, and the pair of changed pointer heaps when exactly two change. A method counts as
  solved only if all its heap tasks are solved; the pair task is the stricter whole-state version.
- Post-states are computed from the MOLD tasks' hole outputs and the template writes; every recording
  passed whole-method validation, so they equal the demonstrated post-states.
- Components: those of the MOLD tasks, plus `storeAtRef` and `freshRefFromHeap` (and a pair constructor).

**Two corrections made during this run, both in the baseline's favour:**

1. A first version typed pointer heaps as MOLD's helper tasks do (`List[Object[C]]`). Then no heap of
   references can be the goal and `storeAtRef` cannot store a reference; nothing solved. Fixed by typing
   them `List[Ref[Object[C]]]` (the unlink witness `storeAtRef(@nextHeap, prevOf(this), nextOf(this))`,
   cost 10, then solves in 33 ms). The first results were discarded.
2. The library's list components are typed `Ref[T] × List[T]` (for `List[Object[C]]` heaps), so with
   heaps of references they never type-check. Variant **"+list"** replaces them with JS ports of the same
   four components, same names and semantics, typed for heaps of references; `check_list_ports.mjs`
   compares ports and library on 20 000 random heaps (including cycles and out-of-range entries): 0
   differences. Without "+list" the baseline effectively runs without list components.
3. insertAfter cannot be expressed without appending to a heap; `run_insert_after_append.mjs` adds the
   library's `insert` and `length` (as the 2026-08-01 baseline had) for that case only.

## Results, adapter limits (`results.json`, `results_list.json`, `results_insert_after_append.json`)

"ok" = all heap tasks of the method solved; the fraction is agreement on the inputs that were not
demonstrated (`check_terms.mjs`, `check_insert_after_append.mjs`; same inputs as the MOLD checks, plus
x ∈ {0,1,2,3,99} for insertAfter). Pair = the whole-state pair task.

| Case | Baseline, current, no list comps | Baseline, current, +list | Baseline, candidate, +list | MOLD, current | MOLD, candidate |
| --- | --- | --- | --- | --- | --- |
| rotateRight (3) | ok, 81/81; pair 81/81 | ok but **18/81, 54/81**; pair timeout | ok, 81/81; pair 81/81 (1.7 s) | wrong (`last_ptr`) | intended terms (method not run) |
| rotateRight (4) | ok, 81/81; pair 81/81 | ok but **36/81, 54/81**; pair timeout | ok, 81/81; pair **timeout** (2.6 s at 15 s) | 1/4 (earlier check, other inputs) | intended terms (method not run) |
| unlink | ok, 15/15; pair exhausted (15/15 at 15 s) | ok but **9/15**; pair timeout | ok, 15/15; pair **timeout** (3.1 s at 15 s) | 15/15 | 15/15 |
| union | ok but **9/12** | ok but **9/12** | **timeout** (also at 15 s) | 9/12 | `leInt(rank this, rank other)` (method not run) |
| swapChildren | ok, 11/11; pair 11/11 | ok but **10/11** | ok, 11/11; pair 11/11 | 14/14 (a larger input set) | 14/14 |
| insertAfter (+insert, length) | ok but **25–28/105** | ok but **25–28/105** | ok but **25–28/105** | 105/105 | 105/105 |
| rotateLeft | ok, 81/81; pair 81/81 | ok but **27/81, 21/81**; pair timeout | ok, 81/81; pair 81/81 (1.0 s) | **21/81** | 81/81 |

(Return-reference tasks are solved correctly everywhere; they are one-hole problems.)

## What the comparison shows

- **The shortcut terms are not caused by the separation.** With the list components usable, the
  non-separated baseline under the current setting returns the same kind of conditional built from
  `penultimateRef` / `last_ptr` as MOLD did, and it is wrong on other inputs. Without them (or with the
  candidate setting) it returns the field paths. The first column looked better than MOLD only because
  the baseline's typing hid the list components.
- **Under the candidate setting, MOLD and the baseline find the same field paths for the pure
  rewirings**, but MOLD's helpers take milliseconds while the baseline's whole-state pair takes 1–3 s and
  times out under the 2 s server limit in two of five cases (rotateRight 4, unlink).
- **Where MOLD is clearly better: object creation and the condition.** For insertAfter every baseline
  term uses the inserted value `x` as the insertion index (`insert(@nextHeap, @x, …)`), which appends
  only because the demonstrated x (15, 25, 35) exceeded the list length; 25–28 of 105 other inputs agree,
  under every setting. MOLD's template creates the object structurally and gets 105/105. For union the
  baseline either returns a 28-cost conditional on rank values (9/12) or times out; MOLD candidate
  returns `leInt` on two ranks.
- Both approaches share the failure of the current setting; the separation does not fix it, and the
  candidate setting fixes it for both.

## Not covered

- The baseline here is given more than the 2026-08-01 one in two ways (which heaps change is known; each
  heap is a separate task) and its component set was repaired twice in its favour. A stricter whole-state
  baseline (one task for the whole environment including unchanged heaps) was not run.
- **The 2026-08-01 baseline (`../../nonseparated_general_components/`) has the same two problems**: its
  tasks type `gHeap` as `List[Ref[Object[Obj]]]` while the list components have been typed
  `Ref[T] × List[T]` since escher-ts `bb77750` (2026-04-10), and none of its found terms uses a list
  component; its found terms include `insert(@fHeap, @arg0, @arg0)` (the value used as the index). Its
  reported append/insert timeouts and successes therefore need re-examination: done in
  `../2026-10-30-baseline-recheck/`.

## Concurrent change in the checkout

At 13:56 another session switched this checkout to branch `fix/translation-success-contract` and modified
`runtime/refsyn-escher-adapter.mjs` (sha256 now `c1861cdc…`) and several tests. Everything in this
directory uses only `external/escher-ts/dist` (unchanged, `4e7c4bc`), not the adapter. In
`../2026-10-30-new-cases/` the Kanon recordings (server responses 13:21–13:34) and `check_new_cases.mjs`
(13:35, the only script there that imports the adapter) ran before the change, with adapter `9540366b…`.

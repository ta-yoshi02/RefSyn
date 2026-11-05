# 2026-11-04: are MOLD and the non-separated baseline given the same synthesis environment?

Before asking whether the verified cases can be synthesized without the separation, this checks that
the two sides get the same components, the same search settings and the same demonstrations. Only
saved task files are read; nothing is synthesized and nothing in `src`, `runtime` or escher-ts changed.
State: RefSyn `a21e42c` (checkout on `fix/translation-success-contract`), escher-ts dist `4e7c4bc`.

Compared, per case:

- MOLD: the saved helper tasks (`mold_escher_tasks.json`; lists from `../2026-10-14-fresh-recording/`).
- Lists, baseline: the paper's `../../nonseparated_general_components/tasks.json` as published, and with
  the three list components retyped (`../2026-10-30-baseline-recheck/tasks_list.json`, the version
  measured 10 times in `../2026-10-30-followthrough/`).
- Other seven cases, baseline: `../2026-10-30-baseline/tasks.json` without and with "+list"; insertAfter
  with `insert` and `length` added as in `run_insert_after_append.mjs`.

## Scripts

- `audit_components.mjs` → `components.json`: the components after `prepareJsonSynthesisJob` (so also
  the field accessors, `leInt` and `loadInt` that escher-ts adds), their types, and whether some
  well-typed term uses each one (a closure over types).
- `audit_heap_args.mjs` → `heap_args.json`: the closure also counts meaningless uses such as
  `last_ptr(@thisRef, @nodeHeap)`. This lists, for every component that reads a heap, every well-typed
  application to the task's own variables, which shows which heap it can actually read. Run on one task
  per family (popBack, rotateLeft, unlink); the component sets do not differ within a family
  (`components.json`).

## Results

### Same on both sides

- Search options: AscendRec, searchSizeFactor 3, reduction rules on, `onlyForwardSearch: false`,
  `anyArgSmaller`, in the adapter (`runtime/refsyn-escher-adapter.mjs` at HEAD), the paper's
  `measure_method_level.mjs` and the 2026-10-30 runners. Limits are the same per run (paper 32 / 15 s
  for both; the 2026-10-30 comparisons ran both sides at 20 / 2 s and 32 / 15 s).
- Same escher-ts dist.
- Demonstrations: the seven non-list baselines are built from MOLD's own examples (same number of
  demonstrations, `components.json`). The list baselines come from the older recording (`f`/`g`);
  `../2026-10-14-fresh-recording/baseline_spec_check.json` shows that all 13 calls have the same
  arguments, pre-states and post-states under `val`→`f`, `next`→`g`.

### Differences that belong to the method

- Only the baseline has `storeAtRef`, `freshRefFromHeap` and `pairHeap` (and, for insertAfter, `insert`
  and `length`). They build the post-state, which MOLD's template does with recorded writes.
- Only MOLD's tasks have `createdN` and `holeN` as inputs: the objects the call creates and the values
  of earlier holes. That is what the separation provides, so it is not an unfair difference.

### Seven non-list cases: equal with "+list", not without

With "+list", the heap-reading components are the same in name and use: `leftOf`/`rightOf`/`hasLeft`/
`hasRight`/`valOf`/`hasVal` (trees), `nextOf`/`prevOf`/`hasNext`/`hasPrev`/`valOf`/`hasVal`
(doubly linked lists), and `last_ptr`, `penultimateRef`, `nthNextRef` and `findByValueRef` on each
pointer heap. (MOLD's helper tasks type pointer heaps `List[Object<C>]`, so they additionally accept
meaningless arguments such as `nextOf(@thisRef, @nextHeap, @nextHeap)`; the baseline does not.)

Without "+list", the four list components accept only `@nodeHeap` in the baseline. Those columns of
`../2026-10-30-baseline/README.md` are a baseline without traversal components.

### Five list cases: **not equal**, in either version of the paper's baseline

| Component | MOLD | Paper baseline as published | Paper baseline retyped (measured 10×) |
| --- | --- | --- | --- |
| `nextOf`, `hasNext`, `valOf`, `hasVal` (field reads by reference) | yes | **absent** | **absent** |
| `findByValueRef` | yes | **absent** | **absent** |
| `last_ptr`, `penultimateRef`, `nthNextRef` on the pointer heap | yes | only on `@nodeHeap` | yes |
| `head`, `tail`, `cons`, `nil`, `concat`, `insert`, `length` | absent | present | present |
| `new_Obj`, `Obj_f`, `Obj_g` (object values) | absent | present | present |

- The paper's list tasks do not set `autoClassFieldComponents`, so escher-ts generates no field
  accessors. In those tasks, the baseline can read a pointer field only positionally
  (`head(tail(@gHeap))`), which is the form of its accidental popBack and setAt terms. There is no
  component that reads `gHeap` at a reference.
- The retyping of 2026-10-30 fixed only the three list components; the measured version still lacks
  five components that MOLD has, and has ten that MOLD lacks. The extra generic list components enlarge
  the baseline's search space (slower) and supply the positional shortcuts (overfitting). The missing
  field reads remove the terms that would express "the node after x".
- So the 10-run comparison in `../2026-10-30-followthrough/` (MOLD 5/5, baseline 1/5 on undemonstrated
  inputs; baseline append and insert time out) compares different component sets. Adding the missing ones
  changes no result (follow-up below); the extra generic list components remain untested.

## What a fair list comparison needs (not done)

Regenerate the five list baselines with `autoClassFieldComponents: true` and `strictFieldAccess: true`
(field accessors typed for `List[Ref[…]]` heaps, as the seven non-list baselines already have), the four
retyped list components including `findByValueRef`, and either MOLD's component set plus only the
construction components, or the generic list components given to both sides. Then rerun and recheck
off-demo. The hand-written witnesses would then also need rechecking for type and cost.

## Not covered

- Usability is decided by types only, not by costs or examples. A component that is usable may still be
  out of reach within the limits.
- The heap-argument listing was run on one task per family. The component sets within a family are the
  same (`components.json`), but the task inputs differ (setAt, insert and append have arguments).

## Follow-up: do the missing components matter? (`make_field_tasks.mjs`, `rerun_fields.mjs`, `check_offdemo_fields.mjs`)

MOLD's own list terms (`../2026-10-30-followthrough/mold_*.json`) use only `nthNextRef`, `last_ptr`,
`penultimateRef` and variables; never `nextOf`, `valOf`, `hasNext`, `hasVal` or `findByValueRef`.

`tasks_fields.json` = `tasks_list.json` plus the field accessors (`autoClassFieldComponents`,
`strictFieldAccess`; generated as `fOf`, `gOf`, `hasF`, `hasG`, typed for `List[Ref[…]]`) and the
`findByValueRef` port. The witnesses stay well-typed (`typecheck_witnesses_fields.json`, 16/16). Same
runs as `../2026-10-30-baseline-recheck/` (32 / 15 s, current and candidate, once each):

- **Encoding fix (2026-11-05).** The first version of `tasks_fields.json` kept the paper's state
  encoding, in which the accessors did not read the state: the class declared `f: Int` while every
  `nodeHeap` object stored its slot in `fHeap` (so `fOf` returned a reference typed as `Int`), and the
  insert tasks left the `nodeHeap` objects empty (so `fOf` and `gOf` were errors). `make_field_tasks.mjs`
  now aligns the encoding with MOLD's: `f: Ref[Int]` (as MOLD's `val`, read with `loadInt`) and every
  object holding `{ f: its slot, g: its gHeap entry }`. `check_field_reads.mjs` → `field_reads.json`
  checks every read on every example input: 0 failures in all 32 tasks (and in MOLD's popBack and insert
  tasks, checked the same way). All results below were rerun with the aligned encoding; they are
  identical to those of the first version, terms and off-demo agreement alike.
- Runs and their conditions. The first rerun with the aligned encoding ran while the machine was
  asleep (lid closed, on battery; `pmset_sleep_wake.txt`) and gave two extra timeouts, recorded at
  87 742 ms and 934 208 ms against the 15 s limit. They coincide with sleeps of 86 s and 933 s: the
  synthesizer checks its limit with `Date.now()` inside the search, and that clock advances during sleep.
  This is what the logs are consistent with, not a measured suspension of the process. The surviving
  logs of that run are in `runs/20261105-first-rerun-recovered/`; its other outputs were overwritten. A
  second rerun, started 12 s after the machine woke (load average 37), timed out on one fields task
  (setAt fHeap, candidate), for a reason not determined; a third, awake at load 2–7, matched the first
  version again (`run_conditions.txt`). From then on, `run_all.sh` runs everything under `caffeinate -i`,
  keeps every run in `runs/<UTC time>/` with its exit codes and load, and the off-demo checks exit with
  code 1 when a row cannot be checked (they used to record "search did not repeat" and exit 0).

- **All 32 terms and timeouts are identical** to those without the added components
  (`rerun_fields.json`); no term found in these runs uses one. That shows they did not change these
  searches, not that the baseline never needs them. Off-demo agreement is therefore identical too
  (`offdemo_fields.json`): setAt 42/42; prepend 24/60; popBack 1/10; append and insert pairs time out.
- The added components change nothing in these searches, within these limits. The missing field reads therefore do not
  explain the baseline's results. The differences that can matter are the generic list components
  (`head`/`tail`/`insert`/…), which only the baseline has: its overfitting terms are built from them
  (`head(tail(tail(tail(@gHeap))))` for null is cost 5, cheaper than `gOf(last_ptr(@thisRef, @gHeap),
  @nodeHeap, @gHeap)`, cost 6). Only the construction part of them (`insert`, `length`) is needed by the
  witnesses; a baseline without `head`/`tail`/`cons`/`nil`/`concat`/`new_Obj`/`Obj_f`/`Obj_g` was not run.
- The patched escher-ts dist used for "candidate" was rebuilt in the session scratchpad on 2026-11-04
  (the earlier copies under `/private/tmp`, including the followthrough worktree's
  `.evaluation-weighted/dist`, had been removed by the OS's periodic cleanup of temporary files).

## Follow-up 2: without the generic list components (`make_minimal_tasks.mjs`, `rerun_minimal.mjs`, `check_offdemo_minimal.mjs`)

`tasks_minimal.json` = `tasks_fields.json` without `head`, `tail`, `cons`, `nil`, `concat` and with
`exposeClassComponents: false` (so no `new_Obj`, `Obj_f`, `Obj_g`). Against the union of MOLD's five list
tasks, the remaining differences are the construction components (`storeAtRef`, `freshRefFromHeap`,
`insert`, `length`, `pairHeap`) and the accessor names (`fOf`/`gOf`/`hasF`/`hasG` for fields `f`/`g`
instead of `valOf`/`nextOf`/`hasVal`/`hasNext`). The witnesses stay well-typed (16/16). Same runs (32 / 15
s, once each); "before" = `rerun_fields.json` / `offdemo_fields.json`.

| Decisive task | Current: before → now | Candidate: before → now |
| --- | --- | --- |
| setAt pair | 42/42, 2.5 s → **42/42, 0.14 s** | timeout → **42/42, 2.2 s** |
| append pair | timeout → **found in 0.27 s, 16/60** | timeout → found in 0.19 s, 16/60 |
| prepend pair | 24/60 → 24/60 (`insert(@fHeap, @arg0, @arg0)`) | 24/60 → 24/60 |
| prepend returnRef | 60/60 → 60/60 | 60/60 → 60/60 |
| insert pair | timeout → timeout | timeout → timeout |
| popBack pair | 1/10 → **10/10, 54 ms** (`penultimateRef(last_ptr(@thisRef, @gHeap), @gHeap)` as null) | timeout → **10/10, 5.4 s** |

- The generic list components were what the baseline's popBack overfit was built from, and they slowed
  every search. Without them the baseline produces correct setAt and popBack methods: **2/5** correct
  methods (was 1/5), MOLD 5/5.
- append is now synthesized, but as `insert(@fHeap, @arg0, @arg0)` paired with
  `insert(@gHeap, dec(length(@nodeHeap)), fresh)`: the value used as the index, and the tail assumed to be
  the last index. 16/60. prepend's state is the same kind of fit. Both come from `insert` taking an index;
  the witnesses only ever insert at `length(heap)`.
- insert still times out in every configuration.
- Times. Two measurements, both with `measure_method_level.mjs` of the followthrough (preparation +
  search, 32 / 15 s, current setting):
  - `measure_alternating.mjs` → `timing_alternating.json` (run by `run_all.sh`, machine awake, load
    2.0–2.8). Each trial runs as its own process (a cold start on both sides) and is killed after 60 s.
    Ten rounds: the method order rotates each round and the side that goes first alternates. All 100
    trials produced a result and none was killed. Every side gave the same term in all 10 trials.
  - `run_measurements_minimal.sh` → `minimal_*.json` (10 repetitions in one process, so warm after the
    first). Its order is all of MOLD, then the baseline, so the two sides ran at different moments. It
    uses the same procedure but does not control for load. Kept for comparison with the followthrough
    numbers.

  Time to a term that fits the demonstrations (medians; ratio = baseline / MOLD):

  | Method | Baseline, alternating (cold) | MOLD, alternating (cold) | Ratio | Baseline, warm | MOLD, warm |
  | --- | --- | --- | --- | --- | --- |
  | setAt | 187 ms | 9.4 ms | 19.9 | 117 ms | 3.3 ms |
  | append | 363 ms | 5.5 ms | 65.5 | 316 ms | 1.1 ms |
  | prepend | 54 ms | 3.4 ms | 15.9 | 17.5 ms | 0.47 ms |
  | insert | no term in 10/10 (15 s limit) | 18.3 ms | — | no term | 6.3 ms |
  | popBack | 108 ms | 4.6 ms | 23.6 | 51 ms | 0.84 ms |

  Whether that term is the method (`offdemo_minimal.json`, finite checked inputs):

  | Method | Baseline | MOLD |
  | --- | --- | --- |
  | setAt | agrees on all 42 | agrees |
  | append | **overfit**, 16/60 | agrees |
  | prepend | **overfit**, 24/60 | agrees |
  | insert | no term | agrees |
  | popBack | agrees on all 10 | agrees |

  Removing the generic list components shrinks the gap from seconds to tens or hundreds of
  milliseconds, but the gap remains. The ratios measure the time to a term that fits the
  demonstrations. Where both sides' terms agree on every checked input (setAt and popBack), MOLD is
  about 20 and 24 times faster cold (35 and 61 times warm). append and prepend ratios compare MOLD's
  method with the baseline's overfit term. insert has no ratio. The cold and warm ratios differ, so
  the factor depends on how one measures; only the order of magnitude is stable here. (The single-run
  times in the table above are search only and are not comparable with these.)

Not run: replacing `insert`/`length` by a single "append at the end" component, which is all the
fixed-ID encoding needs to add the created object and would remove the value-as-index fits.

## Follow-up 3: is the overfitting a matter of the demonstrations?

`insert` in the baseline is escher-ts' generic list component `insert : List[T] × Int × T → List[T]`
(`external/escher-ts/src/library/common-comps-list.ts`), not the method being synthesized. It clamps the
index to the list length, so `insert(@fHeap, @x, @x)` appends whenever x ≥ length. It has been in the
paper's baseline tasks since they were first recorded (2026-08-21).

### What the demonstrations leave open (list cases, `examples` of the paper's tasks)

- Every value argument exceeds the heap length (46–89 against lengths 2–5), so "value as index" and
  "append" agree on every demonstration.
- In append and setAt every list is laid out in index order (head at 0, tail at the last index), so
  "the tail is the last index" holds on every demonstration. popBack: same, plus one unreachable node.
- Only prepend and insert contain a list not in index order.

### Counterexample-guided runs

`cegis_minimal.mjs` → `cegis_minimal.json` (baseline, `tasks_minimal.json`, current setting, 32 / 15 s)
and `cegis_mold.mjs` → `cegis_mold.json` (MOLD's overfit helpers, current setting, server limits 20 /
2 s). Each round adds the smallest undemonstrated input on which the term is wrong, with the correct
output (reference method for the baseline; the candidate setting's verified term for MOLD's holes).

| Case | Added examples until correct on the checked inputs | Outcome |
| --- | --- | --- |
| baseline setAt, popBack, prepend returnRef | 0 | correct |
| baseline prepend | 2 (n = 1 and 2 with x = 0) | `insert(@heap, length(@nodeHeap), …)`, 60/60 |
| baseline append | 3, then **timeout** (16 → 18 → 35/60) | not reached |
| baseline insert | — (**timeout** with the demonstrations alone) | not reached |
| MOLD rotateRight (3 demos) h1 | 4 | `rightOf(leftOf(@thisRef))`, 81/81 |
| MOLD rotateRight (4 demos) h1 | 2 | same |
| MOLD rotateLeft h1 | 3 | `leftOf(rightOf(@thisRef))`, 81/81 |
| MOLD union h0 | 3 | 12/12, but a 3-way conditional on rank constants, not `leInt` |

- **Yes, the demonstrations are insufficient**, for both methods: the overfit terms agree with every
  demonstration, and adding a few discriminating examples turns most of them into the intended term.
- **Not only the demonstrations, as far as was tried**: with the added examples tried here and these
  limits (32 / 15 s), the baseline did not solve append (timeout after 3 added examples) or insert
  (timeout before any example was added), while MOLD solves both from the original demonstrations. This
  does not show that no specification would make them reachable: only these examples and limits were
  tried. That is the part of the comparison that survives, with that scope.
- For MOLD the list cases need nothing extra; the trees need 2–4 more demonstrations of other shapes
  under the current setting (none under the candidate one). union's checked domain (ranks 0–3, two roots)
  is too small to force `leInt`: the loop stops at a term that is right there and wrong for larger ranks.
- union with a wider domain (`cegis_union_wide.mjs` → `cegis_union_wide.json`; distinct ranks 0–7, 56
  pairs; current setting, 20 / 2 s): each added example is answered with one more threshold on a rank
  constant (31 → 41 → 41 → 38 → 46 of 56). After 5 added examples the search times out at 2 s. It never
  reaches `leInt(rank this, rank other)` (cost 16), which the candidate setting returns from the
  original 4 demonstrations. Adding wider demonstrations makes the overfit conditionals longer; it does
  not move the current setting to the comparison.
- The added examples are generated from reference implementations, not recorded in Kanon. For MOLD they
  correspond to demonstrations of trees of other shapes, which give isomorphic operation graphs, so they
  could be recorded; this was not done.

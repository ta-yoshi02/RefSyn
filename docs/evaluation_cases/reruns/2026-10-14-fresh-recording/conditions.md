# Measurement conditions

## Preliminary runs (not evaluation results)

Two runs were made on 2026-10-14 under a plan written before them. They are kept in
`measurements/preliminary/` as the record of how the measuring tools were checked, and are
not used as results.

- **P1** (05:57–06:02 UTC): `scripts/measure_escher_tasks.mjs`, i.e. the adapter's
  `runRefsynTasks` (parse, prepare, search, compile to JS). Every baseline task was
  reported as failed, 13 with `refsynMeta is required for <task>`: the compile step needs
  `refsynMeta`, which the baseline tasks generated on 2026-08-01 do not carry, and the term
  is dropped when compilation throws. P1 found a defect of the measuring path, not a
  search outcome.
- **P2** (06:02–06:07 UTC): `preliminary/measure_search_only.mjs`, same settings without
  the compile step, per task, 5 repetitions. It showed the baseline path works without
  compilation; per-method figures were sums of per-task medians, not measured totals.

Both used `maxCost=32`, `timeoutMs=15000`. That P2 matched the 2026-08-01 baseline outcome
is not evidence that either is correct.

## Main measurement (fixed before running it)

Written on 2026-10-14 after P1 and P2, before any main run.

| Item | MOLD | Non-separated baseline |
| --- | --- | --- |
| Tasks per method | all helper tasks of the recording (`<case>/mold_escher_tasks.json`): insert 3, append 2, popBack 1, prepend 2, setAt 2 | the tasks that decide the method in `../../nonseparated_general_components/tasks.json` (unchanged): `<m>-fixed-env-pair`, plus `prepend-fixed-returnRef` for prepend |
| Information given | pre-call heap, arguments, each hole's output examples derived from the recorded edits | pre-call heap, arguments, whole post-call heap (and return reference); no edit sequence |
| Same problem | `baseline_spec_check.json` (`check_baseline_spec.mjs`): for every call, arguments, the whole pre-call object graph (all captured objects, references, null), the state after replaying the recorded edits, and prepend's return reference equal the baseline examples under a one-to-one object mapping (`val`/`next` = old `f`/`g`). `check_baseline_spec_negative.sh` shows the check rejects altered recordings. | |
| Components | the 14 the backend emits for every helper (`isNull, equal, and, or, not, isZero, isNonNeg, zero, inc, dec, last_ptr, penultimateRef, nthNextRef, findByValueRef`) plus class field components | 23, including `storeAtRef`, `freshRefFromHeap`, `pairHeap` |
| Search | AscendRec, settings of `runtime/refsyn-escher-adapter.mjs`: reduction rules on, forward-only off, `searchSizeFactor=3` | same |
| Limits | `maxCost=32`, `timeoutMs=15000` per task | same |
| Repetitions | 10 | 10 |

### Measured quantity: escher-ts backend processing time, without translation

`measurements/measure_method_level.mjs`. For each repetition it solves every task of the
method one after another and records, per task, `prepare` (JSON spec parsing, job
preparation, synthesizer construction) and `search` (the AscendRec call), timed with
`performance.now()`. The method time of a repetition is the sum over its tasks; reported
are the median, min and max over the 10 repetitions of that sum, both for
`prepare + search` and for `search` alone. A repetition counts as solved only if every
task of the method was solved in it.

Not included: the Rust planning (operation graph, correspondence, template, task
generation), translation of terms to JS, whole-method validation, HTTP and the Kanon
edit. This is not the total running time of either approach.

One Node process per side and method, tasks in the order listed in the task file;
repetition 1 includes JIT warm-up and is not discarded.

### Environment

Darwin 27.0.0 arm64, Node v22.17.0, RefSyn `b42d9ca`, escher-ts `4e7c4bc` (dist hash in
`<case>/provenance.txt`). The machine is not isolated (other sessions' processes and the
browser keep running); load average is recorded before and after.

## End-to-end stages (separate from timing)

Per recording: request accepted, synthesis (all helpers solved), translation (all helpers
translated), whole-method validation (`passed`, every demonstration checked), applied,
re-execution in Kanon (every call `testPassed`). One recording per case: counts, not rates.

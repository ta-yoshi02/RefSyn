# Recheck of the 2026-10-14 measurement against main 88927bc

The main measurement of the five cases (`eval/five-cases-b42d9ca`, commit `925917d`,
`docs/evaluation_cases/reruns/2026-10-14-fresh-recording/`) ran at RefSyn `b42d9ca`.
main has since merged #32 (`88927bc`). This directory checks whether that measurement
still describes main, without measuring again.

## Result: no difference that reaches the search; not re-measured

| Item | b42d9ca (measured) vs 88927bc (main) | Evidence |
| --- | --- | --- |
| MOLD tasks of the five recordings | byte-identical | `<case>/tasks_88927bc.json` (regenerated from `<case>/raw_request.json` with `examples/run_payload` at 88927bc) vs `<case>/saved_tasks_b42d9ca.json` |
| Baseline tasks | identical file (`nonseparated_general_components/tasks.json`, sha256 `fc97ef0a…`) | `git show` of both commits |
| Components | contained in the task JSON, so identical with it; field components come from escher-ts | as above |
| Search settings | `runtime/refsyn-escher-adapter.mjs` identical (sha256 `9540366b…`); the measuring script sets the same AscendRec options (`searchSizeFactor=3`, reduction rules on, forward-only off, `anyArgSmaller`) | file hashes, `consistency/measure_method_level.mjs` |
| Task generation | `src/pbe_problem.rs`, `src/escher_bridge.rs`, `src/template.rs`, `src/alignment.rs`, `scripts/run_escher.js`, `Cargo.lock` identical | file hashes |
| escher-ts | submodule `4e7c4bc` in both; dist combined hash `50b69f3c…` equals the one recorded in the measurement's `provenance.txt` | |
| Node | v22.17.0 in both | |
| Code changed by #32 | `src/operation_graph.rs` and `runtime/whole-method-validation.mjs` (which edits are accepted, adoption check); not task generation or search | `git diff --stat b42d9ca 88927bc -- src runtime` |

The five requests still pass at 88927bc (`<case>/run_payload_output.txt`: `validation.status`
`passed`, all demonstrations checked).

## Consistency run (one repetition, not a measurement)

`consistency/`: one run per method with `maxCost=32`, `timeoutMs=15000` on the 88927bc
tasks. Solved/unsolved and every found term equal those of all ten repetitions of the
main measurement (`consistency/result.json`). Times of this single run are not reported.

## C1 and C5 on the five recordings (one-off, not in CI)

`web/src/lib/binding-conditions.test.ts` (C1: the found term reproduces every output
example; C5: the translated helper writes nothing reachable from its inputs, checked with
Proxies) covers paper_insert and three boolean fixtures in CI. A temporary copy with only
the fixture list and input paths changed (`consistency/c1_c5_test_copy.diff`) was run on
the five recordings with the WASM rebuilt at 88927bc (`web/pkg/refsyn_bg.wasm` sha256
`0d6e6b9c…`) and then deleted. Output: `consistency/c1_c5_five_cases.txt`.

- C1 and C5 passed for insert, append, popBack, prepend and setAt (10 tests).
- The copy's 5 other tests failed only because of the path change: four load
  `paper_insert` by name (ENOENT under the changed path) and one expects the union of
  kinds over the fixtures to include booleans and a conditional, which the five cases do
  not have. They pass unmodified in CI.

## Kanon recording with a numeric constructor default (after #32)

`kanon_insert_val0_b9ad0a8/`: insert recorded in the Kanon UI with
`constructor() { this.val = 0; this.next = null; }` at `b9ad0a8` (the #32 fix; its `src`
and `runtime` equal 88927bc). The request carries `instance: {"val":0,"next":null}` and
`addEdge` on the created node's `val`; response `passed`, 3/3; the applied source keeps
`this.val = 0;` and all three calls are `testPassed`.

## Written values of another kind than the default (type probes)

`type_probes/`: the three requests from the #32 review (copied from
`/private/tmp/refsyn-pr32-review/`) rerun with `examples/run_payload` built at 88927bc.
Each pre-call state holds the default's kind in the field; the demonstrations write a
value of another kind (`false` → number, `0` → boolean, `0` → reference). All three:
HTTP 200, `passed`, 3/3. `field_kinds` reads the pre-call objects and the `new C()` state,
not written values, so these are not rejected.

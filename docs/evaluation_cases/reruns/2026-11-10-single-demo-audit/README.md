# Single-demonstration audit

Recorded payloads were tested on 2026-11-09 JST; the second fresh UI demonstration was completed on 2026-11-10 JST.

Scope: current working tree at a21e42c2197b8f918dbec73f2632024a55b4fd61, including pre-existing uncommitted changes. No production source or paper changes were made by this audit.

## Recorded demonstrations

Each individual recorded call was synthesized independently. `selection` is zero-based; `all` uses the complete original recording. Validation checks demonstrated inputs only. Agreement below uses the independent reference and its existing finite case set. Different single demonstrations are different conditions, not repeated trials.

| Method | Selection | Demonstrations | Holes | Validation | Reference agreement |
|---|---:|---:|---:|---|---:|
| insert | 0 | 1 | 3 | passed | 4/15 |
| insert | 1 | 1 | 3 | passed | 5/15 |
| insert | 2 | 1 | 3 | passed | 1/15 |
| insert | all | 3 | 3 | passed | 15/15 |
| setAt | 0 | 1 | 2 | passed | 9/15 |
| setAt | 1 | 1 | 2 | passed | 5/15 |
| setAt | 2 | 1 | 2 | passed | 4/15 |
| setAt | all | 3 | 2 | passed | 15/15 |
| append | 0 | 1 | 2 | passed | 15/15 |
| append | 1 | 1 | 2 | passed | 15/15 |
| append | 2 | 1 | 2 | passed | 15/15 |
| append | all | 3 | 2 | passed | 15/15 |
| prepend | 0 | 1 | 2 | passed | 15/15 |
| prepend | 1 | 1 | 2 | passed | 15/15 |
| prepend | all | 2 | 2 | passed | 15/15 |
| popBack | 0 | 1 | 1 | passed | 12/12 |
| popBack | 1 | 1 | 1 | passed | 12/12 |
| popBack | all | 2 | 1 | passed | 12/12 |
| rotateRight | 0 | 1 | 3 | passed | 2/12 |
| rotateRight | 1 | 1 | 3 | passed | 0/12 |
| rotateRight | 2 | 1 | 3 | passed | 0/12 |
| rotateRight | 3 | 1 | 3 | passed | 2/12 |
| rotateRight | all | 4 | 3 | passed | 4/12 |

The comparator exits zero even when it reports mismatches. Counts were read from the JSON `mismatches` array, not inferred from process exit status. All 23 requests passed whole-method validation. This does not establish correctness for other inputs.

## Fresh UI demonstrations

Kanon was served locally from this checkout; synthesis used the native RefSyn backend. These were new manual graph edits through the browser, not injected requests.

- `ui-0`: `Cell.set(arg)`, initial `val=3`, argument `7`; add literal 7 and redirect the val edge. Generated helpers return `this` and `arg`. Editor reflection observed. Native replay passed 1/1; independent comparison passed 20/20.
- `ui-1`: `Cell.setSeven()`, initial `val=3`, no arguments; same graph edit. Generated value helper increments zero seven times. Editor reflection observed. Native replay passed 1/1; independent comparison passed 5/5.
- UI comparisons vary initial value across -9, 0, 3, 7, 89 and argument across -5, 0, 7, 100, with an additional next node. For setSeven there are 5 distinct initial heaps and no arguments.
- `ui-set.png` and `ui-setSeven.png` show the reflected code. UI request bodies are captured in `ui-server.log`; each ui folder includes a native replay response and reference comparison.
- Mutation: replacing the composed `set` method with an empty body while retaining successful PBE results is rejected: `Value or return mismatch at main-new1.val` (`ui-0/mutation.json`).

## Checks and reproduction

- `cargo build --locked --example run_payload`
- `python3 run.py` (from the checkout, after building the example; regenerates recorded-request results).
- `node --test runtime/whole-method-validation.test.mjs tests/js/compare-with-reference.test.mjs`: 55 passed.
- `cargo test --locked --test paper_pipeline_tests --test whole_method_validation_tests`: see rust-tests.log.

The UI gestures are not automated by run.py. Replaying captured requests does not constitute a new UI demonstration.

## Interpretation

The implementation applies the same hole rule to one demonstration. Single-example replay succeeds across the sampled methods, but finite reference agreement depends on the method and which demonstration is supplied. The all-example rotateRight candidate still disagrees on 8/12 reference cases. Validator success is not generalization, but it is also not implied solely by PBE success: translation and composition errors can be rejected.

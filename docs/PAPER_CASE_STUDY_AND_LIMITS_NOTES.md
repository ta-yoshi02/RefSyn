# Paper Case Study and Limits Notes

This memo records implementation-side questions that should stay synchronized with the MOLD paper repository at `/Users/taku/workspace/kenkyu/paint26-tmp`. It is not manuscript prose. Use it to track what can be claimed, what still needs evidence, and where to look in the RefSyn/MOLD implementation.

## Proposed Section-4 Evidence Shape

The paper should not present the case study as only a narrative for append and insert. A stronger structure is:

1. A table of candidate methods and current evidence status.
2. Short in-paper explanations of representative cases.
3. Appendix-level details for selected cases: program skeleton, demonstrated graph operations, generated task JSON, backend result, and reconstructed method body.
4. A discussion of why failed or partial cases fail.

## Evidence Matrix

| method / family | data structure | number of traces | common plan extraction | holes generated | backend task generated | backend solved | recomposition | validated by test | failure stage | evidence |
| --- | --- | ---: | --- | --- | --- | --- | --- | --- | --- | --- |
| `set` | singly linked list / object field | 2 traces in duplicated-fixture test; 2 traces in mismatched-receiver integration case | success for duplicated `editEdgeReference`; composition path succeeds for mismatched receiver | value | yes for synthesis path, but backend result is not the primary evidence | unknown / not the main assertion | success; emits `set(arg)` with `this.next.val = h_int_0;` | integration test + unit test | none in covered path | `tests/integrated_synthesis_tests.rs::test_operations_json_set_supports_edit_edge_reference_analysis`; `tests/integrated_synthesis_tests.rs::test_set_with_existing_kanon_id_and_mismatched_receiver_still_composes`; `tests/unify_ops_tests.rs::test_unify_set` |
| `append` | singly linked list | 3 traces in integration test | success | value + pointer | yes; grouped Int/Ptr specs with 3 examples each | yes in backend-gated integration path; pointer helper is `last_ptr`-style | success; helper use is checked and fixed-hop replay is rejected | end-to-end integration test | none in covered path | `tests/integrated_synthesis_tests.rs::test_integrated_synthesis_three_append_like_specs_group_holes`; `docs/KANON_CODEX_VERIFICATION.md` for browser verification notes |
| `insert` / `insertAt` | singly linked list | 3 traces in fixture | success | value + edge-source / pointer | yes; exactly two tasks retained, Ptr task includes `nthNextRef`, `last_ptr`, `findByValueRef` and 3 examples | yes in backend-gated integration test | success; preserves `tmp0.val = h_int_0;` after pruning unused alternatives | end-to-end integration test + replayable fixture | none in covered path | `examples/current_user_insert_three_traces.json`; `tests/integrated_synthesis_tests.rs::test_integrated_synthesis_insert_groups_value_and_edge_source_holes`; `test_integrated_synthesis_insert_three_traces_task_json_keeps_ts_ptr_components_and_ref_heaps`; `test_integrated_synthesis_insert_three_traces_executes_backend_tasks` |
| `popBack` | singly linked list | 2 traces in integration test | success for `deleteEdge` common plan | edge-source / pointer; return value none; field clear to `null` | partial / implicit through generated predecessor helper | partial / checked through composed output, not separately documented as task JSON in paper | success; emits `.next = null;` and avoids replaying `this.next.next.next = null;` | integration test + unit tests for `deleteEdge` semantics | none in covered field-clear path | `tests/integrated_synthesis_tests.rs` popBack test around `deleteEdge`; `src/list_env.rs` `delete_edge`; `src/lib.rs` deleteEdge recomposition tests |
| `splitAt(i)` | singly linked list | unknown / no confirmed current fixture | unknown | expected pointer + return, but not confirmed | unknown | unknown | unknown; prior probes suggested fixed-hop return underfitting such as `return this.next.next` | not validated | needs verification: return / cut-point / fixed-hop underfitting | no current test found by repository search; keep as investigation item |
| `swapValue` | list or object pair | synthetic unit candidates for 2 traces | ambiguous / multiple maximum same-signature candidates | ambiguous value choices | partial; grouped candidate specs can be enumerated | unknown | no confirmed composed method | unit test only | correspondence ambiguity / candidate selection | `src/lib.rs::aggregate_multi_trace_specs_enumerates_same_signature_choices` |
| `rotateLeft` | binary tree local rotation | unit-level recomposition fixture | not validated from clean traces | return + snapshot-sensitive pointer rewrites in recomposition fixture | unknown | unknown | partial success at recomposition level; emits snapshots and `return snap1;` | unit test only | clean-trace end-to-end unknown; boundary is recomposition timing and return snapshot | `src/lib.rs::build_composed_method_code_returns_snapshot_for_rewired_object`; `src/lib.rs::build_composed_method_code_snapshots_rewire_references_before_updates` |
| recursive tree / B+ tree operations | recursive structures | none | unknown | unknown | unknown | unknown | none | not validated | outside current scope: recursive control and size-proportional updates | no current end-to-end evidence; keep as future/out-of-scope |

## Appendix Record Template

For each selected case, keep one appendix subsection with:

- Method skeleton: class fields, method signature, and initial incomplete method body.
- Demonstrations: number of traces, method arguments, receiver object, and high-level graph edit sequence.
- Operation vocabulary used: `addNode`, `addEdge`, `editEdgeReference`, `deleteEdge`, `addVariable`, or `editVariableReference`.
- MOLD intermediate result: common plan, hole roles, generated task names, and task return types.
- Backend result: generated helper code or failure message.
- Recomposition result: emitted method body and whether it is semantically correct for the intended method.
- Evidence source: fixture path, test name, command, and date of verification.

## Limit Analysis Axes

The paper should separate at least four different reasons a method may be unsupported. These are not interchangeable.

### A. Theoretical limit of common/diff extraction

Question: even with perfect traces and perfect normalization, does the demonstrated method have a stable common operation structure across examples?

Examples to examine:

- Symmetric updates such as `swapValue`, where multiple correspondences can be equally plausible.
- Methods whose intended generalization requires choosing among many candidate alignments with no evidence in the traces.
- Cases where adding more traces disambiguates the correspondence versus cases where ambiguity remains inherent.

Relevant implementation handles:

- `build_relaxed_parallel_diff_pair_variants(...)`
- `MAX_RELAXED_DIFF_PAIR_VARIANTS`
- `aggregate_multi_trace_specs(...)`
- Tests around ambiguous same-signature choices.

Paper phrasing should avoid saying the target function is impossible. The precise claim is whether the current abstraction can select a unique useful decomposition from the demonstrated traces.

### B. Normalization and trace-quality limit

Question: does the trace accurately represent the intended pre-state, post-state, and runtime object identities?

Examples to examine:

- Runtime-scoped IDs and `idMapping` collisions.
- Missing or stale `precondGraph` / `actualGraph`.
- Self-loop or rewiring repairs that make the output plausible but may hide a bad trace.

Relevant implementation handles:

- Runtime ID normalization and repair logic in `src/lib.rs`.
- Diagnostics around duplicate node IDs, `oldTo` repair, and precondition fallback.
- Insert fixture `examples/current_user_insert_three_traces.json` as a replayable known case.

Paper phrasing should distinguish algorithmic failure from invalid or underspecified trace capture.

### C. Operation-vocabulary limit

Question: can the method be expressed as graph edit operations currently modeled by MOLD?

Currently central operations:

- `addNode`
- `addEdge`
- `editEdgeReference`
- `deleteEdge` as field clear
- `addVariable`
- `editVariableReference`

Known unsupported or partial operations:

- `removeNode`
- `removeEdge`
- `deleteNode`
- `deleteVariable`

Important nuance: `deleteEdge` is supported as clearing the single `(from, label)` slot to null. This is not full graph deletion or multiedge deletion.

Relevant implementation handles:

- `detect_unsupported_remove_operation(...)` in `src/lib.rs` rejects `removeNode`, `removeEdge`, `deleteNode`, and `deleteVariable`.
- `ListEnvironment::delete_edge(...)` treats `deleteEdge` as field clearing.
- `build_composed_method_code(...)` can emit `field = null` for `deleteEdge` but still rejects `removeNode` and `removeEdge` in composed generation.

### D. Backend component-vocabulary limit

Question: after MOLD extracts a hole, can the backend synthesize a helper from the available components?

Examples:

- `append` needs a tail selector such as `last_ptr`.
- `insertAt` may need `nthNextRef`.
- `popBack` needs a predecessor-of-tail helper such as `penultimateRef`.
- Value-guided pointer selection may need `findByValueRef`.

Relevant implementation handles:

- `default_task_components(...)` in `src/escher_bridge.rs` injects list-oriented pointer helpers when there is one pointer field.
- Current helper injection is stronger for list-shaped domains than for arbitrary multi-pointer structures.

Paper phrasing should say MOLD reduces side-effectful updates to pure helper-synthesis tasks, but the success of those tasks depends on the backend component vocabulary.

### E. Recomposition limit

Question: even if holes are generated and solved, can MOLD reconstruct a correct imperative method body?

Examples:

- Return timing after rewiring, as in `rotateLeft`, may require snapshotting the old reference before mutation.
- Some assignments are recovered from hole metadata when they no longer appear in the common plan; this can produce useful code but is a weaker evidence surface than a clean common plan.
- Generated code may be partial when unsupported operations remain.

Relevant implementation handles:

- `build_composed_method_code(...)`
- Snapshot tests for rewired return values.
- `prune_unused_holes_and_specs(...)` and related tests.

## Near-Term Investigation Checklist

1. Re-run evidence for the positive examples: `set`, `append`, `insert`, and `popBack`.
2. For `insert`, save the generated task JSON and composed method output produced from `examples/current_user_insert_three_traces.json`.
3. Re-test `splitAt(i)` and classify the failure precisely: common/diff extraction, backend synthesis, or recomposition.
4. Use `swapValue` to enumerate correspondence candidates and decide whether it belongs in Section 4 or limitations.
5. Re-test `rotateLeft` with clean input shapes and inspect whether snapshot-based return handling is now enough.
6. Decide the final table labels: supported, partially supported, unsupported by current implementation, or unverified.
7. Mirror any final, evidence-backed claims into the paper repository's Japanese source first.

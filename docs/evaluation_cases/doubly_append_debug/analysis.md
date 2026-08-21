# Doubly Linked List append Debug Notes

## Source

- Terminal log: `terminal_log.txt`
- Reproduced payloads:
  - `payload_1spec.json`
  - `payload_2spec.json`
  - `payload_3spec.json`
- Escher-ts tasks from replay:
  - `escher_task_2spec.json`
  - `escher_task_3spec.json`

The recorded operations use:

- `f`: value field
- `g`: next-like pointer
- `h`: prev-like pointer

The expected append skeleton is:

```js
append(x) {
  const value = this.append_f(x);
  const tail = this.append_g(x);
  const tmp0 = new Obj();
  tmp0.f = value;
  if (tail !== null) { tail.g = tmp0; }
  tmp0.h = tail;
}
```

## Observed Results

### 1 specification

No operation analysis is performed for a single operation sequence.

Generated method:

```js
append(x) {
    const tmp0 = new Obj();
    tmp0.f = 89;
    this.g.g.g = tmp0;
    tmp0.h = this.g.g;
}
```

This is not a meaningful generalization. It is the literal first trace replayed as code.

### 2 specifications

MOLD extracts two holes:

- `append-f`: value, synthesized as `arg0`
- `append-g`: pointer, synthesized as a fixed-depth conditional expression

Generated helper:

```js
append_g(arg0) {
  return ((((this).g).g).g !== null)
    ? ((((this).g).g).g)
    : (((this).g).g);
}
```

This satisfies the two traces but is not the intended `last_ptr`-like result. It is an overfit expression caused by insufficient specifications and the current component/search behavior.

### 3 specifications

The pointer task disappears. Only `append-f` is sent to Escher-ts.

Generated method:

```js
append(x) {
    const h_int_0 = this.append_f(x);
    const tmp0 = new Obj();
    this.g.g.g = tmp0;
    tmp0.h = this.g.g;
    tmp0.f = h_int_0;
}
```

This is worse than the 2-spec case: the tail pointer is fixed to the original three-node list shape.

## Cause

This is not an Escher-ts synthesis failure. In the 3-spec run, Escher-ts receives only the value task:

- `append-f(...): Int = arg0`

The missing pointer task is lost before the backend call.
This is directly visible in `escher_task_3spec.json`, which contains only `append-f`.

Trace output shows the multi-trace grouping keeps only value signatures:

```text
TRACE: aggregate signature=ret=Int|role=value|op=addEdge|label=f|is_literal=false members=3
TRACE: aggregate signature=ret=Int|role=value|op=addNode|label=<literal>|is_literal=true members=3
Grouped multi-trace Escher specs: 6 candidates -> 2 grouped specs (dedup: 1)
SYNTHESIS DIAGNOSTIC: multi-trace grouping collapsed 1 alias specs into 1 canonical specs: append-g -> append-f
```

Before that, the pointer candidate for trace 2 has a missing/null output:

```text
TRACE: diff_pair[0] outputs A=2 (Some(Ptr)) B=null (Some(Ptr)) return=Ptr
```

Because pointer outputs with `null` are treated as missing, the Ptr candidate does not survive full multi-trace aggregation.

The immediate reason for that `null` is the third call's operation IDs. `call3` contains:

```json
{"editType":"addEdge","from":"main-call3-FunctionExpression4-new4","to":"__temp8","label":"g"}
{"editType":"addEdge","from":"__temp8","to":"main-call3-FunctionExpression4-new4","label":"h"}
```

However, `main-call3-FunctionExpression4-new4` is not present in `call3.precondGraph`; it appears in `call3.actualGraph`. The old tail should be an object already present before the append, but the operation refers to an actual/future runtime ID. Since `call3.idMapping` is also empty, MOLD cannot canonicalize this runtime ID back to the fresh temp node or to the pre-state tail. The output extractor asks `fixed_id_index_for_object(...)` for that ID, fails to find it in the current list environment, and emits `null`.

So the failure has two layers:

1. The third payload has an ID-consistency problem around the tail/fresh-node reference.
2. MOLD silently treats that unresolved pointer as a missing output instead of rejecting the trace or reporting an inconsistent operation ID.

Relevant implementation points:

- `candidate_has_missing_output` rejects candidates where `is_missing_example_output` says the output is missing.
- `aggregate_multi_trace_specs` requires coverage for all traces, then removes candidates with missing outputs when a non-missing candidate exists for that trace.
- Existing tests also encode a conservative assumption that plain `addEdge` diffs should not synthesize edge-source holes. That assumption is too weak for allocation plus pointer rewiring in doubly linked append.

## Classification

This is not an algorithmic impossibility. The immediate trigger is an inconsistent third operation trace, and the implementation bug is that MOLD silently degrades the unresolved pointer to `null` instead of surfacing the trace inconsistency.

The algorithm should be able to synthesize this case if it extracts the old tail pointer as one shared Ptr hole and uses it in both pointer updates:

- `tail.g = tmp0`
- `tmp0.h = tail`

The current implementation sometimes extracts this hole for 2 specs, but the synthesized expression overfits. For 3 specs, the hole extraction/grouping path drops the pointer task entirely and leaves a fixed path in the composed method.

## Next Fix Direction

1. Add a regression test using `payload_3spec.json` or a reduced equivalent fixture.
2. Preserve an edge-source Ptr hole for `addEdge(existingTail, freshNode, g)` across multi-trace aggregation.
3. Bind the same Ptr hole to the reverse edge `addEdge(freshNode, existingTail, h)`.
4. Do not treat a trace-local null in the pairwise comparison as grounds to erase the semantic tail pointer candidate when the other side identifies a valid existing node.
5. Verify that the generated helper becomes `last_ptr`-like rather than a fixed-hop conditional.

## Latest 3-Spec Retry

Files:

- `latest_retry/terminal_log.txt`
- `latest_retry/payload_3spec.json`
- `latest_retry/escher_task_3spec.json`
- `latest_retry/escher_task_3spec_after_fix.json`

This retry has a different failure mode from the earlier `payload_3spec.json`.

The pointer hole is now extracted:

```text
__hole_0 -> append-h (Ptr)
__hole_1 -> append-f (Int)
```

The composed method is structurally what we want:

```js
append(x) {
    const h_ptr_0 = this.append_h(x);
    const h_int_0 = this.append_f(x);
    const tmp0 = new Obj();
    if (h_ptr_0 !== null) { h_ptr_0.g = tmp0; }
    tmp0.h = h_ptr_0;
    tmp0.f = h_int_0;
}
```

However, Escher-ts fails on `append-h`:

```text
[FAIL] append-h -> escher-ts could not synthesize 'append-h' within the configured budget
```

The generated task asks for the tail reference:

- input lengths 3, 4, 5
- expected outputs `{ref:2}`, `{ref:3}`, `{ref:4}`

So this is no longer the previous missing/null-output problem. The current failure is component selection. `latest_retry/escher_task_3spec.json` contains only the generic boolean/int components for `append-h`; it does not include `last_ptr`, `penultimateRef`, or `nthNextRef`.

The current Rust bridge only adds heap pointer components when there is exactly one pointer field:

```rust
if meta.pointer_fields.len() == 1 {
    push_library_component(&mut components, &mut seen, "last_ptr");
    push_library_component(&mut components, &mut seen, "penultimateRef");
    push_library_component(&mut components, &mut seen, "nthNextRef");
}
```

For the doubly linked list case, pointer fields are `g` and `h`, so the condition is false and the necessary tail component is omitted. This is an implementation limitation in the Escher task construction, not evidence that the operation-based decomposition failed.

### Fix Result

The bridge was changed to include heap traversal components whenever at least one pointer field exists, not only when there is exactly one pointer field. The runtime renderer was also changed to infer the JavaScript field from the heap argument passed to the synthesized component:

- `last_ptr(@thisRef, @gHeap)` renders traversal over field `g`
- `last_ptr(@thisRef, @hHeap)` renders traversal over field `h`

After the fix, replaying `latest_retry/payload_3spec.json` succeeds:

```text
escher-ts synthesis completed (2 success / 0 failure)

[OK] append-f =
  @arg0

[OK] append-h =
  last_ptr(@thisRef, @gHeap)
```

Generated helper:

```js
append_h(arg0) {
  return (() => {
    let __refsynCur = this;
    while (__refsynCur !== null && (__refsynCur).g !== null) {
      __refsynCur = (__refsynCur).g;
    }
    return __refsynCur;
  })();
}
```

The composed method now matches the intended doubly linked append structure.

## Non-Separated Fixed-ID Baseline

Files:

- `latest_retry/baseline_fixed_id_examples.json`
- `latest_retry/baseline_fixed_id_task.json`
- `latest_retry/baseline_components.json`
- `latest_retry/baseline_fixed_id_results_measured.json`
- `latest_retry/baseline_fixed_id_runtime.csv`

The baseline receives the pre-state fixed-ID list environment and the method argument, but not
the operation sequence. The output is the post-state field heaps:

- `fHeap: List[Int]`
- `gHeap: List[Ref[Object[Obj]]]`
- `hHeap: List[Ref[Object[Obj]]]`
- `env: Pair[List[Int], Pair[List[Ref[Object[Obj]]], List[Ref[Object[Obj]]]]]`

Measured with `maxCost=14`, `timeoutMs=15000`, and one repetition:

| task | result | time |
| --- | --- | ---: |
| `doubly-append-fixed-fHeap` | success: `insert(@fHeap, @arg0, @arg0)` | 10.689 ms |
| `doubly-append-fixed-gHeap` | timeout | 14999.850 ms |
| `doubly-append-fixed-hHeap` | success: recursive tail-sensitive expression | 157.926 ms |
| `doubly-append-fixed-env` | timeout | 15000.057 ms |

The `fHeap` success is not a meaningful append model. It works for these traces because the
arguments `89`, `46`, and `61` are all larger than the heap length, so `insert` clamps to the end.

The meaningful failure is `gHeap`. The baseline must synthesize both:

1. replacing the old tail's `g` slot with the fresh object reference, and
2. appending a new `null` slot for the allocated object.

The current component set has list construction and insertion, but no direct indexed replacement
or fresh-reference constructor in the paper-facing baseline. Under that condition, Escher-ts fails
to synthesize the forward pointer heap and the whole post-state environment. This is the relevant
comparison against MOLD, where allocation and field writes are represented by the operation
skeleton and only the tail reference/value helpers are synthesized.

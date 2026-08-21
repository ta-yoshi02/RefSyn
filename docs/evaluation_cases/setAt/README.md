# setAt

## Target

- Data structure: singly linked objects.
- Object class: `Obj`.
- Value field: `f`.
- Pointer field: `g`.
- Method signature: `Obj.setAt(a: Int, b: Int)`.
- Expected behavior in this case: follow `g` from `this` for `a` steps and update the reached object's `f` field to `b`.

The oracle shape is recorded in `target_program.js`.

## Specifications

- Source: terminal log from the localhost:8000 Kanon/MOLD run.
- MOLD operation records: 3.
- Arguments: `(0, 46)`, `(2, 60)`, `(3, 18)`.
- Detected class: `Obj`.
- Detected value field: `f`.
- Detected pointer field: `g`.
- Object order: fixed object-ID order.
- Baseline output construction: apply each `editEdgeReference(label=f)` operation to the corresponding pre-state fixed-ID `fHeap`; do not rely on `actualGraph` as the post-state oracle.
- Literal caveat: the payload's added value nodes have `type: "string"`, but the method argument types are `Int`; the baseline records numeric heap values.
- Measurement config: `maxCost=14`, `timeoutMs=15000`, `repetitions=3`.

## Artifacts

| file | role |
| --- | --- |
| `terminal_log.txt` | Original terminal log pasted by the user |
| `mold_payload.json` | Extracted Kanon/MOLD payload |
| `target_program.js` | Hand-written oracle behavior for the recorded setAt case |
| `mold_escher_tasks.json` | MOLD helper tasks regenerated from the saved payload |
| `mold_results_measured.json` | Per-task MOLD helper synthesis results with 3 measured runs |
| `mold_runtime.csv` | MOLD helper runtime table |
| `baseline_fixed_id_task.json` | Non-separated fixed-ID list-environment baseline tasks |
| `baseline_fixed_id_examples.json` | Fixed-ID pre/post heap examples derived from the payload operations |
| `baseline_components.json` | Baseline component set |
| `baseline_fixed_id_results_measured.json` | Per-task baseline synthesis results with 3 measured runs |
| `baseline_fixed_id_runtime.csv` | Baseline runtime table |
| `notes.md` | Interpretation and limits |

## MOLD Result

MOLD decomposes the three operation records into two helper synthesis tasks.

| task | result | median runtime |
| --- | --- | ---: |
| `setAt-g` | `@arg1` | 0.667 ms |
| `setAt-h` | `nthNextRef(@thisRef, @nodeHeap, @gHeap, @arg0)` | 6.071 ms |

Reconstructed method:

```js
setAt(a, b) {
    const h_ptr_0 = this.setAt_h(a, b);
    const h_int_0 = this.setAt_g(a, b);
    if (h_ptr_0 !== null) { h_ptr_0.f = h_int_0; }
}
```

The operation-level `COMMON_PLAN` is empty for this multi-spec run. The assignment in the reconstructed method is recovered from hole metadata.

## Fixed-ID Baseline Result

The non-separated baseline receives the pre-state fixed-ID list environment and arguments, and attempts to synthesize the post-state list environment. It does not receive the operation sequence.

The three value-heap examples are:

```text
call1: [39, 27, 53, 89] -> [46, 27, 53, 89]  args=(0, 46)
call2: [46, 27, 53, 89] -> [46, 27, 60, 89]  args=(2, 60)
call3: [46, 27, 60, 89] -> [46, 27, 60, 18]  args=(3, 18)
```

The pointer heap is unchanged in all three examples.

Measured results:

| task | output | result | median runtime |
| --- | --- | --- | ---: |
| `setAt-fixed-fHeap` | `List[Int]` | failure: timeout | 15000.045 ms |
| `setAt-fixed-gHeap` | `List[Ref[Object[Obj]]]` | success: `@gHeap` | 0.233 ms |
| `setAt-fixed-env-pair` | `Pair[List[Int],List[Ref[Object[Obj]]]]` | failure: timeout | 14999.886 ms |

The successful `gHeap` term is trivial because `setAt` does not change pointers. The value heap task is the meaningful baseline comparison. It must synthesize indexed replacement in a list; under the current component set and budget, it fails.

## Paper Use

Allowed claim:

```text
For the recorded setAt case, MOLD decomposes the update into a value helper and a target-reference helper, and synthesizes both. A non-separated fixed-ID list-environment baseline can synthesize the unchanged pointer heap but fails to synthesize the changed value heap and the whole post-state pair within the same fixed budget.
```

Not allowed:

```text
setAt proves that all value-only updates are impossible for the baseline.
```

This result only covers the current component set, budget, and three recorded traces. A baseline with an explicit list replacement component would be a different experiment.

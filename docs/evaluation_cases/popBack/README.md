# popBack

## Target

- Data structure: singly linked objects.
- Object class: `Obj`.
- Value field: `f`.
- Pointer field: `g`.
- Method signature: `Obj.popBack()`.
- Expected behavior in this case: remove the final reachable node from the list by setting the predecessor node's `g` field to `null`.

The oracle shape is recorded in `target_program.js`.

## Specifications

- Source: terminal log from the localhost:8000 Kanon/MOLD run.
- Browser check: localhost:8000 currently shows the synthesized `popBack_f` and composed `popBack` code, so the displayed program is a post-synthesis state, not the original pre-synthesis source.
- MOLD operation records: 2.
- Arguments: none.
- Detected class: `Obj`.
- Detected value field: `f`.
- Detected pointer field: `g`.
- Object order: fixed object-ID order.
- Baseline output construction: apply each `deleteEdge(label=g)` operation to the corresponding pre-state fixed-ID `gHeap`; do not rely on `actualGraph` as the post-state oracle.
- Measurement config: `maxCost=14`, `timeoutMs=15000`, `repetitions=3`.

## Artifacts

| file | role |
| --- | --- |
| `terminal_log.txt` | Original terminal log pasted by the user |
| `mold_payload.json` | Extracted Kanon/MOLD payload |
| `target_program.js` | Hand-written oracle behavior for the recorded popBack case |
| `mold_escher_tasks.json` | MOLD helper task regenerated from the saved payload |
| `mold_results_measured.json` | Per-task MOLD helper synthesis result with 3 measured runs |
| `mold_runtime.csv` | MOLD helper runtime table |
| `baseline_fixed_id_task.json` | Non-separated fixed-ID list-environment baseline tasks |
| `baseline_fixed_id_examples.json` | Fixed-ID pre/post heap examples derived from the payload operations |
| `baseline_components.json` | Baseline component set |
| `baseline_fixed_id_results_measured.json` | Per-task baseline synthesis results with 3 measured runs |
| `baseline_fixed_id_runtime.csv` | Baseline runtime table |
| `notes.md` | Interpretation and limits |

## MOLD Result

MOLD decomposes the two operation records into one helper synthesis task.

| task | result | median runtime |
| --- | --- | ---: |
| `popBack-f` | `penultimateRef(@thisRef, @gHeap)` | 1.498 ms |

Reconstructed method:

```js
popBack() {
    const h_ptr_0 = this.popBack_f();
    h_ptr_0.g = null;
}
```

The generated recomposition assumes the recorded traces have a removable tail. Empty and singleton-list behavior is not covered by these two specifications.

## Fixed-ID Baseline Result

The non-separated baseline receives the pre-state fixed-ID list environment and attempts to synthesize the post-state list environment. It does not receive the operation sequence.

The two pointer-heap examples are:

```text
call1: [1, 2, 3, null] -> [1, 2, null, null]
call2: [1, 2, null, null] -> [1, null, null, null]
```

Measured results:

| task | output | result | median runtime |
| --- | --- | --- | ---: |
| `popBack-fixed-fHeap` | `List[Int]` | success: `@fHeap` | 0.649 ms |
| `popBack-fixed-gHeap` | `List[Ref[Object[Obj]]]` | failure: timeout | 15000.037 ms |
| `popBack-fixed-env-pair` | `Pair[List[Int],List[Ref[Object[Obj]]]]` | failure: timeout | 15000.176 ms |

The successful `fHeap` term is trivial because `popBack` does not change values. The pointer heap task is the meaningful baseline comparison. It must synthesize a list transformation that sets the predecessor pointer to `null`; under the current component set and budget, it fails.

## Paper Use

Allowed claim:

```text
For the recorded popBack case, MOLD decomposes the update into one predecessor-reference helper and synthesizes it as penultimateRef. A non-separated fixed-ID list-environment baseline can synthesize the unchanged value heap but fails to synthesize the pointer heap update and the whole post-state pair within the same fixed budget.
```

Not allowed:

```text
MOLD synthesizes a fully general safe popBack for empty and singleton lists.
```

The recorded traces cover length 4 and length 3 pre-states only. They justify the predecessor-selection helper for those traces, not total correctness over all list lengths.

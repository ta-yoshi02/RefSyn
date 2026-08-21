# insert_general

## Target

- Data structure: singly linked objects.
- Object class: `Obj`.
- Value field: `f`.
- Pointer field: `g`.
- Method signature: `Obj.insert(i: Int, b: Int)`.
- Expected behavior in this case: insert a new `Obj` with `f = b` after the `i`-th object reachable from `this` through `g`.

The oracle shape is recorded in `target_program.js`.

## Specifications

- MOLD operation records: 3.
- Arguments: `(0, 89)`, `(2, 46)`, `(1, 71)`.
- Detected class: `Obj`.
- Detected value field: `f`.
- Detected pointer field: `g`.
- Object order: fixed object-ID order.
- Measurement config: `maxCost=14`, `timeoutMs=15000`, `repetitions=3`.
- Measurement commit: `eb0dbbf`.

## Artifacts

| file | role |
| --- | --- |
| `target_program.js` | Hand-written oracle behavior for the recorded insert case |
| `mold_payload.json` | Kanon/MOLD payload used for this case |
| `mold_escher_tasks.json` | MOLD helper tasks regenerated after fixed-ID ordering |
| `mold_results_measured.json` | Per-task MOLD helper synthesis results with 3 measured runs |
| `mold_runtime.csv` | MOLD helper runtime table |
| `baseline_fixed_id_task.json` | Non-separated fixed-ID list-environment baseline tasks |
| `baseline_fixed_id_results_measured.json` | Per-task baseline synthesis results with 3 measured runs |
| `baseline_fixed_id_runtime.csv` | Baseline runtime table |
| `mold_result.md` | Common plan, hole bindings, synthesized helpers, recomposition |
| `notes.md` | Interpretation and limits |

Older diagnostic files such as `baseline_results.json`, `baseline_task.json`, and `runtime.csv` are chain-order experiments. They are not the main paper-facing baseline.

## MOLD Result

MOLD decomposes the three operation records into three helper synthesis tasks.

| task | result | median runtime |
| --- | --- | ---: |
| `insert-f` | `@arg1` | 0.583 ms |
| `insert-h` | `nthNextRef(@thisRef, @nodeHeap, @gHeap, @arg0)` | 9.374 ms |
| `insert-i` | `nthNextRef(@thisRef, @nodeHeap, @gHeap, inc(@arg0))` | 17.142 ms |

All 3 helper tasks succeeded in all 3 repetitions.

Reconstructed method:

```js
insert(i, b) {
    const h_ptr_0 = this.insert_h(i, b);
    const h_ptr_1 = this.insert_i(i, b);
    const h_int_0 = this.insert_f(i, b);
    const tmp0 = new Obj();
    tmp0.g = h_ptr_1;
    if (h_ptr_0 !== null) { h_ptr_0.g = tmp0; }
    tmp0.f = h_int_0;
}
```

## Fixed-ID Baseline Result

The non-separated baseline receives the pre-state fixed-ID list environment and arguments, and attempts to synthesize the post-state list environment. It does not receive the operation sequence.

For the first operation:

```text
before:
fHeap = [39, 27, 53]
gHeap = [1, 2, null]

after:
fHeap = [39, 27, 53, 89]
gHeap = [3, 2, null, 1]
```

Measured results:

| task | output | result | median runtime |
| --- | --- | --- | ---: |
| `insert-fixed-fHeap` | `List[Int]` | success: `insert(@fHeap, @arg1, @arg1)` | 20.312 ms |
| `insert-fixed-gHeap` | `List[Ref[Object[Obj]]]` | failure: timeout | 15000.294 ms |
| `insert-fixed-env-pair` | `Pair[List[Int],List[Ref[Object[Obj]]]]` | failure: timeout | 15000.360 ms |

The successful `fHeap` term is not a semantic insertion model. It appends because the fixed-ID heap places the newly allocated object at the end, and because every `arg1` in these examples is larger than the heap length.

The pointer heap task is the real baseline test. It must synthesize both:

- rewiring the predecessor object's `g` field to the newly allocated object ID;
- initializing the new object's `g` field to the old successor.

Under the current baseline component set and budget, Escher-ts does not synthesize that transformation.

## Paper Use

Allowed claim:

```text
For the recorded insert case, MOLD decomposes the update into three small helper tasks and synthesizes all of them. A non-separated fixed-ID list-environment baseline can synthesize the value heap extension but fails to synthesize the pointer heap update and the whole post-state pair within the same fixed budget.
```

Not allowed:

```text
MOLD is faster than Escher-ts for general insert.
```

This case supports a decomposition claim, not a universal runtime superiority claim. Three traces are not a proof of arbitrary insert behavior.

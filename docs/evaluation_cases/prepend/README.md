# prepend

## Target

- Data structure: singly linked objects.
- Object class: `Obj`.
- Value field: `f`.
- Pointer field: `g`.
- Method signature: `Obj.prepend(x: Int): Obj`.
- Expected behavior in this case: allocate a new `Obj`, set its `f` field to `x`, set its `g` field to the old receiver, and return the new object.

The oracle shape is recorded in `target_program.js`.

## Specifications

- Source: terminal log from the localhost:8000 Kanon/MOLD run.
- MOLD operation records: 2.
- Arguments: `(53)`, `(89)`.
- Detected class: `Obj`.
- Detected value field: `f`.
- Detected pointer field: `g`.
- Return operation: `addVariable(label=return)`.
- Object order: fixed object-ID order in generated Escher tasks.
- Measurement config: `maxCost=14`, `timeoutMs=15000`, `repetitions=3`.

## Artifacts

| file | role |
| --- | --- |
| `terminal_log.txt` | Original terminal log pasted by the user |
| `mold_payload.json` | Extracted return-value style Kanon/MOLD payload |
| `target_program.js` | Hand-written oracle behavior for the recorded prepend case |
| `mold_escher_tasks.json` | MOLD helper tasks regenerated from the saved payload |
| `mold_results_measured.json` | Per-task MOLD helper synthesis results |
| `mold_runtime.csv` | MOLD helper runtime table |
| `baseline_fixed_id_task.json` | Non-separated fixed-ID list-environment baseline tasks |
| `baseline_fixed_id_examples.json` | Fixed-ID pre/post heap and return-ref examples derived from the MOLD task examples |
| `baseline_components.json` | Baseline component set |
| `baseline_fixed_id_results_measured.json` | Per-task baseline synthesis results |
| `baseline_fixed_id_runtime.csv` | Baseline runtime table |
| `notes.md` | Interpretation and limits |

## MOLD Result

MOLD decomposes the two operation records into two helper synthesis tasks.

| task | result | median runtime |
| --- | --- | ---: |
| `prepend-f` | `@thisRef` | 0.536 ms |
| `prepend-g` | `@arg0` | 0.359 ms |

The helper names follow the current hole naming. Semantically:

- `prepend-f` supplies the old head pointer for the new node's `g` field.
- `prepend-g` supplies the new value for the new node's `f` field.

Reconstructed method:

```js
prepend(x) {
    const h_ptr_0 = this.prepend_f(x);
    const h_int_0 = this.prepend_g(x);
    const tmp0 = new Obj();
    tmp0.f = h_int_0;
    tmp0.g = h_ptr_0;
    return tmp0;
}
```

This is the clean prepend shape. Unlike the older diagnostic trace, it does not require the method to mutate caller variable `l`.

## Fixed-ID Baseline Result

The non-separated baseline receives the pre-state fixed-ID list environment and argument, and attempts to synthesize post-state heap components and the returned reference. It does not receive the operation sequence.

The fixed-ID examples encode the newly allocated object as the next heap slot:

```text
call1:
  fHeap [39, 27] -> [39, 27, 53]
  gHeap [1, null] -> [1, null, 0]
  returnRef = 2

call2:
  fHeap [39, 27, 53] -> [39, 27, 53, 89]
  gHeap [1, null, 0] -> [1, null, 0, 2]
  returnRef = 3
```

Measured results:

| task | output | result | median runtime |
| --- | --- | --- | ---: |
| `prepend-fixed-fHeap` | `List[Int]` | success: `insert(@fHeap, @arg0, @arg0)` | 6.991 ms |
| `prepend-fixed-gHeap` | `List[Ref[Object[Obj]]]` | success: `insert(@gHeap, @arg0, @thisRef)` | 3.912 ms |
| `prepend-fixed-returnRef` | `Ref[Object[Obj]]` | failure: timeout | 15000.060 ms |
| `prepend-fixed-env-pair` | `Pair[List[Int],List[Ref[Object[Obj]]]]` | success: `pairHeap(insert(@fHeap, @arg0, @arg0), insert(@gHeap, @arg0, @thisRef))` | 183.457 ms |

The `fHeap`, `gHeap`, and pair successes are weak evidence. The generated terms append by using `@arg0` as an index, and the recorded arguments are larger than the heap length. This fits these examples because `insert` clamps to the end; it is not a robust allocation-at-next-ID model.

The returned reference is the meaningful baseline comparison. The baseline must produce the fresh object reference, but `prepend-fixed-returnRef` fails within the current budget.

## Paper Use

Allowed claim:

```text
For the recorded return-value prepend case, MOLD synthesizes only the two data-dependent helper values, while allocation and return of the fresh node are represented by the operation skeleton. A non-separated fixed-ID baseline can fit the heap extension examples, but fails to synthesize the returned fresh reference within the same budget.
```

Not allowed:

```text
The fixed-ID baseline fully synthesizes prepend because fHeap and gHeap succeeded.
```

That would ignore the returned reference and overcount accidental `insert` fits caused by large argument values.

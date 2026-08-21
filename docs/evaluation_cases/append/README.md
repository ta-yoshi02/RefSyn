# append

## Target

- Data structure: singly linked objects.
- Object class: `Obj`.
- Value field: `f`.
- Pointer field: `g`.
- Method signature: `Obj.append(x: Int)`.
- Expected behavior in this case: append a new `Obj` with `f = x` after the current tail reachable from `this` through `g`.

The oracle shape is recorded in `target_program.js`.

## Specifications

- Source: terminal log from the localhost:8000 Kanon/MOLD run.
- MOLD operation records: 3.
- Arguments: `(53)`, `(89)`, `(46)`.
- Detected class: `Obj`.
- Detected value field: `f`.
- Detected pointer field: `g`.
- Object order: fixed object-ID order.
- Measurement config: `maxCost=14`, `timeoutMs=15000`, `repetitions=3`.

## Artifacts

| file | role |
| --- | --- |
| `terminal_log.txt` | Original terminal log pasted by the user |
| `mold_payload.json` | Extracted Kanon/MOLD payload |
| `mold_escher_tasks.json` | MOLD helper tasks regenerated from the saved payload |
| `mold_results_measured.json` | Measured result with the normal component set |
| `mold_runtime.csv` | Runtime CSV for the normal component set |
| `mold_escher_tasks_no_last_ptr.json` | Diagnostic task with `last_ptr` removed |
| `mold_no_last_ptr_results_measured.json` | Measured result without `last_ptr` |
| `mold_escher_tasks_nth_only_tail.json` | Diagnostic task with `last_ptr`, `penultimateRef`, and `findByValueRef` removed |
| `mold_nth_only_tail_results_measured.json` | Measured result for the `nthNextRef`-only tail diagnostic |
| `baseline_fixed_id_task.json` | Non-separated fixed-ID list-environment baseline tasks |
| `baseline_fixed_id_results_measured.json` | Measured baseline result |

## MOLD Result

Normal component set:

| task | result | median runtime |
| --- | --- | ---: |
| `append-f` | `@arg0` | 0.674 ms |
| `append-h` | `last_ptr(@thisRef, @gHeap)` | 1.635 ms |

Reconstructed method:

```js
append(x) {
    const h_ptr_0 = this.append_h(x);
    const h_int_0 = this.append_f(x);
    const tmp0 = new Obj();
    if (h_ptr_0 !== null) { h_ptr_0.g = tmp0; }
    tmp0.f = h_int_0;
}
```

## Fixed-ID Baseline Result

The non-separated baseline receives the pre-state fixed-ID list environment and argument, and attempts to synthesize the post-state list environment. It does not receive the operation sequence.

Measured results:

| task | output | result | median runtime |
| --- | --- | --- | ---: |
| `append-fixed-fHeap` | `List[Int]` | success: `insert(@fHeap, @arg0, @arg0)` | 11.480 ms |
| `append-fixed-gHeap` | `List[Ref[Object[Obj]]]` | failure: timeout | 15001.333 ms |
| `append-fixed-env-pair` | `Pair[List[Int],List[Ref[Object[Obj]]]]` | failure: timeout | 15002.551 ms |

The successful `fHeap` term is not a semantic append model. It appends only because all recorded arguments are larger than the heap length, so `insert` clamps to the end.

The pointer heap task is the meaningful baseline comparison. It must synthesize both changing the old tail's pointer to the new object ID and appending the new null pointer slot. Under the current component set and budget, it fails.

## last_ptr Diagnostic

Removing only `last_ptr` still succeeds:

```text
append-h =
  gOf(penultimateRef(@thisRef, @gHeap), @nodeHeap, @nodeHeap)
```

Removing `last_ptr`, `penultimateRef`, and `findByValueRef` also succeeds on these three examples, but the result is a fixed-depth conditional:

```text
append-h =
  if isNull(nthNextRef(@thisRef, @nodeHeap, @gHeap, 2))
  then gOf(@thisRef, @nodeHeap, @nodeHeap)
  else if isNull(nthNextRef(@thisRef, @nodeHeap, @gHeap, 3))
  then nthNextRef(@thisRef, @nodeHeap, @gHeap, 2)
  else nthNextRef(@thisRef, @nodeHeap, @gHeap, 3)
```

This fits the recorded lengths but is not a general tail selector. It is slower as well: median 847.848 ms for `append-h`, compared with 1.635 ms using `last_ptr`.

## Paper Use

Allowed claim:

```text
For the three recorded append traces, MOLD decomposes the update into a value helper and a tail-reference helper. With the normal component set, the tail helper is synthesized as last_ptr. A non-separated fixed-ID list-environment baseline fails to synthesize the pointer heap and whole post-state pair within the same budget.
```

Not allowed:

```text
nthNextRef alone is sufficient for general append.
```

The `nthNextRef`-only result is a fixed-depth expression for the recorded list lengths, not a general append implementation.

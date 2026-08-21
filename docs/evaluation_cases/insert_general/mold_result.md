# MOLD Result: insert_general

## Source

- Repository commit: `eb0dbbf`
- Source log: `run_log.txt`
- Payload: `mold_payload.json`
- MOLD Escher tasks: `mold_escher_tasks.json`
- Measured results: `mold_results_measured.json`, `mold_runtime.csv`

## Specifications

Three operation records were captured for `Obj.insert(i, b)`.

| call | arguments | operations |
| --- | --- | --- |
| `call1` | `(0, 89)` | 5 |
| `call2` | `(2, 46)` | 5 |
| `call3` | `(1, 71)` | 5 |

## Common Plan

```text
COMMON_PLAN (operation-level, ordered)
[op_0] addNode(id=__temp2, label=Obj, isLiteral=false)
[op_1] addEdge(from=__temp2, to=__hole_1, label=g)
[op_2] editEdgeReference(from=__hole_0, to=__temp2, label=g)
[op_3] addEdge(from=__temp2, to=__hole_2, label=f)

HOLE_BINDINGS
__hole_0 -> insert-h (Ptr)
__hole_1 -> insert-i (Ptr)
__hole_2 -> insert-f (Int)
__hole_3 -> insert-f (Int)
```

## Synthesized Helpers

```text
insert-f =
  @arg1

insert-h =
  nthNextRef(@thisRef, @nodeHeap, @gHeap, @arg0)

insert-i =
  nthNextRef(@thisRef, @nodeHeap, @gHeap, inc(@arg0))
```

All three helper synthesis tasks succeeded.

Measured with `maxCost=14`, `timeoutMs=15000`, `repetitions=3`:

| task | median runtime | result |
| --- | ---: | --- |
| `insert-f` | 0.583 ms | `@arg1` |
| `insert-h` | 9.374 ms | `nthNextRef(@thisRef, @nodeHeap, @gHeap, @arg0)` |
| `insert-i` | 17.142 ms | `nthNextRef(@thisRef, @nodeHeap, @gHeap, inc(@arg0))` |

## Recomposition

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

## Interpretation

This is a better insert case than the earlier tail-biased trace because the synthesized pointer helpers depend on `i`.

The safe claim is not that arbitrary insert has been proven. The safe claim is that, for this three-trace operation record, MOLD decomposed the mutation into three small pure synthesis tasks, and the pointer tasks synthesized `nthNextRef(..., i)` and `nthNextRef(..., i + 1)`.

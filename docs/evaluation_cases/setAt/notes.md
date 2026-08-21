# setAt Notes

## Payload

The saved payload comes from `terminal_log.txt` and contains three `setAt` method calls:

- call1: `setAt(0, 46)`
- call2: `setAt(2, 60)`
- call3: `setAt(3, 18)`

Each call records two operations:

- `addNode` for the new literal value;
- `editEdgeReference` that changes an object's `f` edge to the new literal.

The payload marks the added literal node as `type: "string"`, while the method argument type is `Int`. The evaluation treats `fHeap` as `List[Int]`, matching the synthesized helper signature and explicit method arguments.

## MOLD Behavior

The current implementation synthesizes:

```text
setAt-g = @arg1
setAt-h = nthNextRef(@thisRef, @nodeHeap, @gHeap, @arg0)
```

The composed method is:

```js
setAt(a, b) {
    const h_ptr_0 = this.setAt_h(a, b);
    const h_int_0 = this.setAt_g(a, b);
    if (h_ptr_0 !== null) { h_ptr_0.f = h_int_0; }
}
```

The multi-spec operation unification reports no common operation in `COMMON_PLAN`; the field assignment is recovered from hole metadata. This is acceptable as an implementation result, but it should be described carefully in the paper if this case is used.

## Fixed-ID Baseline Construction

Objects are ordered by fixed object ID:

```text
main-new1, main-new2, main-new3, main-new4
```

The fixed-ID baseline inputs are derived from each `precondGraph`. The baseline outputs are derived by applying the recorded `editEdgeReference(label=f)` operation to the pre-state heap. `actualGraph` is not treated as authoritative post-state evidence.

The resulting `fHeap` examples are:

```text
call1:
  input  [39, 27, 53, 89]
  output [46, 27, 53, 89]

call2:
  input  [46, 27, 53, 89]
  output [46, 27, 60, 89]

call3:
  input  [46, 27, 60, 89]
  output [46, 27, 60, 18]
```

The `gHeap` examples are all identity transformations.

## Interpretation

This case is a useful stress test because it removes pointer rewiring from the baseline's meaningful output. The baseline still fails on `fHeap`, which suggests the current component set is not just struggling with object references; it also lacks a compact way to express indexed list replacement.

Do not overstate this. The honest claim is:

- MOLD helper synthesis: success, medians 0.667 ms and 6.071 ms.
- fixed-ID `fHeap` synthesis: timeout at 15 s.
- fixed-ID environment-pair synthesis: timeout at 15 s.
- fixed-ID `gHeap` synthesis: success only because it is unchanged.

## Limits

The traces cover indices 0, 2, and 3 on a four-object list. They do not cover out-of-range indices or empty lists.

The baseline result depends on the current component set. Adding an explicit `replaceAt` or `update` component would change the comparison and should be treated as a separate, less neutral baseline.

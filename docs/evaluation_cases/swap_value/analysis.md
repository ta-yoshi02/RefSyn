# swapValue Debug Notes

## Files

- `payload_3spec.json`
- `terminal_log.txt`

The method name in the payload is `swap`, but the intended behavior is value swap on a singly
linked list:

```js
swap(i, j) {
  const nodeI = nthNextRef(this, gHeap, i);
  const nodeJ = nthNextRef(this, gHeap, j);
  const valueI = nodeI.f;
  const valueJ = nodeJ.f;
  nodeI.f = valueJ;
  nodeJ.f = valueI;
}
```

## Observed Result

The backend reports synthesis success:

```text
escher-ts synthesis completed (4 success / 0 failure)
```

However, the operation-level common plan is empty:

```text
COMMON_PLAN (operation-level, ordered)
```

The field assignments are recovered from hole metadata rather than from a common operation
skeleton:

```text
SYNTHESIS DIAGNOSTIC: COMMON_PLAN has no edge ops but composed method still contains field
assignments; code was recovered from hole metadata
```

Generated method:

```js
swap(i, j) {
    const h_ptr_0 = this.swap_n(i, j);
    const h_ptr_1 = this.swap_r(i, j);
    const h_int_0 = this.swap_f(i, j);
    const h_int_1 = this.swap_j(i, j);
    if (h_ptr_0 !== null) { h_ptr_0.f = h_int_1; }
    if (h_ptr_1 !== null) { h_ptr_1.f = h_int_0; }
}
```

The synthesized helpers overfit the three examples. For example, `swap_r` depends on `arg0` and
does not generally select the node at `j`:

```text
swap-r =
  if isZero(@arg0) then penultimateRef(@thisRef, @gHeap) else gOf(@thisRef, @nodeHeap, @nodeHeap)
```

For the second trace, `swap(1, 3)` should update node 1 and node 3. The generated `swap_r` selects
`this.g`, i.e. node 1, so both assignments target the same node. This does not implement the
recorded swap behavior.

## Diagnosis

This is not simply an Escher-ts timeout. It is a correspondence/recomposition failure.

The payload contains two same-kind operations per trace:

```text
editEdgeReference(... label=f)
editEdgeReference(... label=f)
```

The intended correspondence is:

- first update: node at index `i` receives the old value at index `j`
- second update: node at index `j` receives the old value at index `i`

The current implementation has some local enumeration for ambiguous correspondences, but it does
not perform a global search that validates the final composed method against the full before/after
graph examples. As a result, a locally plausible set of holes can be retained even when the
recomposed method is semantically wrong.

## Classification

This should be treated as a current implementation limitation, not as a fundamental limitation of
the research idea.

A correct decomposition is representable with the current style of operation skeleton:

- synthesize `nodeI = nthNextRef(this, gHeap, i)`
- synthesize `nodeJ = nthNextRef(this, gHeap, j)`
- synthesize/read `valueI = nodeI.f`
- synthesize/read `valueJ = nodeJ.f`
- emit two simultaneous-style assignments: `nodeI.f = valueJ`, `nodeJ.f = valueI`

The missing piece is principled correspondence selection and end-to-end validation of the composed
method, especially when multiple same-label field updates occur in one method.

## Intended Helper Check

Files:

- `intended_helper_tasks.json`
- `intended_helper_results_measured.json`
- `intended_helper_runtime.csv`

As a diagnostic, the helper examples were manually corrected to match the intended decomposition:

- `swap-n`: reference to the node at `i`
- `swap-r`: reference to the node at `j`
- `swap-f`: value at node `i`
- `swap-j`: value at node `j`

With those corrected examples, Escher-ts can synthesize all four helper tasks. However, the
solutions are still not the desired fully general `nthNextRef(..., i/j)` form. For example:

```text
swap-r-intended =
  if isZero(dec(dec(@arg1))) then penultimateRef(@thisRef, @gHeap)
  else last_ptr(@thisRef, @gHeap)
```

This satisfies the three recorded examples because `j` only takes `2` and `3` on a length-4 list.
It does not prove that the intended general indexed selector was learned.

So there are two separate issues:

1. The current MOLD correspondence step produces wrong helper examples for `swap-r` and `swap-j`.
2. Even with corrected helper examples, the current three traces are too weak to force the
   general indexed selector; cheaper overfitted terms satisfy the examples.

## Paper Treatment

Do not use this case as positive evaluation evidence in the current state.

It can be used as a limitation case:

> When a method contains multiple same-kind updates to the same field, the current implementation
> may fail to infer the intended correspondence between operation records. In the `swapValue` case,
> the backend can synthesize all generated helper tasks, but the operation-level common plan is
> empty and the recomposed method does not implement the recorded swap behavior.

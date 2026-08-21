# swapAdjacent Value Debug Notes

## Files

- `payload_3spec.json`
- `terminal_log.txt`
- `escher_task_3spec.json`

The payload method name is `swapAd`. The behavior appears to be adjacent value swap:

```js
swapAd(i) {
  const nodeI = nthNextRef(this, gHeap, i);
  const nodeJ = nthNextRef(this, gHeap, i + 1);
  const valueI = nodeI.f;
  const valueJ = nodeJ.f;
  nodeI.f = valueJ;
  nodeJ.f = valueI;
}
```

## Observed Result

The backend reports success:

```text
escher-ts synthesis completed (4 success / 0 failure)
```

However, the operation-level common plan is empty:

```text
COMMON_PLAN (operation-level, ordered)
```

The recomposed assignments are recovered from hole metadata:

```text
SYNTHESIS DIAGNOSTIC: COMMON_PLAN has no edge ops but composed method still contains field assignments; code was recovered from hole metadata
```

Generated method:

```js
swapAd(i) {
    const h_ptr_0 = this.swapAd_n(i);
    const h_ptr_1 = this.swapAd_r(i);
    const h_int_0 = this.swapAd_f(i);
    const h_int_1 = this.swapAd_j(i);
    if (h_ptr_0 !== null) { h_ptr_0.f = h_int_1; }
    if (h_ptr_1 !== null) { h_ptr_1.f = h_int_0; }
}
```

## Incorrect Helper Examples

The intended helper outputs for the three examples are:

| helper | arg=1 | arg=3 | arg=2 |
| --- | --- | --- | --- |
| `swapAd-n` | ref 1 | ref 3 | ref 2 |
| `swapAd-r` | ref 2 | ref 4 | ref 3 |
| `swapAd-f` | 27 | 46 | 27 |
| `swapAd-j` | 53 | 89 | 89 |

The actual Escher task outputs are:

| helper | arg=1 | arg=3 | arg=2 |
| --- | --- | --- | --- |
| `swapAd-n` | ref 1 | ref 3 | ref 2 |
| `swapAd-r` | ref 2 | ref 3 | ref 2 |
| `swapAd-f` | 27 | 46 | 27 |
| `swapAd-j` | 53 | 46 | 27 |

So `swapAd-n` and `swapAd-f` correspond to the `i` side and are correct, but `swapAd-r` and
`swapAd-j` only match the adjacent `i+1` side in the first trace. In the second and third traces
they collapse back to the `i` side.

This is why the synthesized helpers contain conditionals such as:

```text
swapAd-r =
  if isZero(dec(@arg0)) then nthNextRef(..., inc(@arg0))
  else nthNextRef(..., @arg0)
```

The term satisfies the wrong helper examples, but it does not implement adjacent swap.

## Diagnosis

This is a correspondence failure, not a PBE backend timeout.

Each trace contains two same-kind operations:

```text
editEdgeReference(... label=f)
editEdgeReference(... label=f)
```

The intended correspondence is:

- first update: node at index `i` receives value from node `i+1`
- second update: node at index `i+1` receives value from node `i`

The current pipeline does not globally validate operation correspondences by executing the final
recomposed method against every before/after graph. As a result, it accepts helper examples where
the second operation is aligned to the wrong side for later traces.

## Classification

This is the same class of issue as `swapValue`, but cleaner because the intended relation is
`i` and `i+1` rather than two independent indices.

It is a PBD-side implementation limitation:

- operation count is fixed,
- operation kinds are fixed,
- the required PBE terms are expressible using `nthNextRef`,
- but same-kind operation correspondence is not selected by end-to-end semantic validation.

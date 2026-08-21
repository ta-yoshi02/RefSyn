# Notes

## What This Case Shows

The saved payload reproduces the browser/terminal run for `append(x)` with three examples:

- `append(53)`
- `append(89)`
- `append(46)`

MOLD produces two helper tasks:

```text
append-f = @arg0
append-h = last_ptr(@thisRef, @gHeap)
```

The normal MOLD helper tasks are tiny and solve quickly:

| task | median runtime |
| --- | ---: |
| `append-f` | 0.674 ms |
| `append-h` | 1.635 ms |

## Baseline Interpretation

The fixed-ID list-environment baseline can synthesize the value heap extension, but the generated term is not meaningful:

```text
append-fixed-fHeap = insert(@fHeap, @arg0, @arg0)
```

This appends only because `53`, `89`, and `46` are all greater than the heap length in the recorded examples.

The pointer heap task and whole environment pair both fail within the 15s budget:

| task | result | median runtime |
| --- | --- | ---: |
| `append-fixed-gHeap` | timeout | 15001.333 ms |
| `append-fixed-env-pair` | timeout | 15002.551 ms |

The failure should be described as a failure under the current component set and search budget, not as a theoretical impossibility.

## last_ptr vs nthNextRef

`last_ptr` is not strictly necessary to fit these three examples.

Without `last_ptr`, the backend uses:

```text
gOf(penultimateRef(@thisRef, @gHeap), @nodeHeap, @nodeHeap)
```

With `last_ptr`, `penultimateRef`, and `findByValueRef` removed, the backend still succeeds using `nthNextRef`, but only through fixed-depth branching:

```text
if isNull(nthNextRef(..., 2))
then gOf(thisRef, ...)
else if isNull(nthNextRef(..., 3))
then nthNextRef(..., 2)
else nthNextRef(..., 3)
```

That expression is not a general tail selector. A fourth or fifth trace would likely make this expression grow or fail unless the component vocabulary contains a genuine tail traversal helper.

## Paper Use

This case is now usable as append evidence with three operation records.

Use it to support:

- MOLD decomposes append into value and tail-reference helpers.
- The normal component set solves the tail helper with `last_ptr`.
- The non-separated fixed-ID baseline fails on pointer heap reconstruction under the current budget.

Do not use it to claim:

- `nthNextRef` alone is generally enough for append.
- The scalar `fHeap` baseline result demonstrates a semantic append solution.

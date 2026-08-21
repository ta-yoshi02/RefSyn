# popBack Notes

## Payload and Browser State

The saved payload comes from `terminal_log.txt` and contains two `popBack` method calls. The operations are both `deleteEdge` operations on field `g`:

- call1: delete `main-new3.g -> main-new4`
- call2: delete `main-new2.g -> main-new3`

The localhost:8000 page was checked after the run. It currently displays code with synthesized `popBack_f` already inserted:

```js
popBack_f() { ... }
popBack() {
    const h_ptr_0 = this.popBack_f();
    h_ptr_0.g = null;
}
```

Therefore the browser state is useful as a sanity check for the synthesized program, but it is not evidence of the original source before synthesis.

## Fixed-ID Baseline Construction

Objects are ordered by fixed object ID:

```text
main-new1, main-new2, main-new3, main-new4
```

The fixed-ID baseline inputs are derived from each `precondGraph`. The baseline outputs are derived by applying the recorded `deleteEdge(label=g)` operation to the pre-state heap. This avoids relying on `actualGraph`, which is not treated as the authoritative post-state for this payload.

The resulting `gHeap` examples are:

```text
call1:
  input  [1, 2, 3, null]
  output [1, 2, null, null]

call2:
  input  [1, 2, null, null]
  output [1, null, null, null]
```

## Interpretation

The MOLD result is small and direct:

```text
popBack-f = penultimateRef(@thisRef, @gHeap)
```

The non-separated baseline exposes the hard part directly: synthesize an updated pointer heap. `fHeap` succeeds only because it is unchanged. This should not be counted as evidence that the baseline synthesized `popBack`.

The useful comparison is:

- MOLD helper synthesis: success, median 1.498 ms.
- fixed-ID `gHeap` synthesis: timeout at 15 s.
- fixed-ID environment-pair synthesis: timeout at 15 s.

## Limits

This case has only two specifications and no empty/singleton examples. Do not claim total correctness of generated `popBack` from this evidence.

The recomposed method does not guard against `popBack_f()` returning `null` before assigning `h_ptr_0.g = null`. That is acceptable for the recorded traces but should be called out if using this as a general method example.

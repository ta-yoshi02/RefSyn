# prepend Notes

## Replacement of Older Trace

The older prepend case used `editVariableReference(label=l)`. That crossed the method boundary because it modeled the method as mutating the caller's variable binding.

This directory has been replaced with the cleaner return-value trace. The current payload uses:

```text
addVariable(label=return)
```

The composed method now includes:

```js
return tmp0;
```

This is the version that should be used as the evaluation basis.

## Payload

The saved payload contains two `prepend` calls:

- call1: `prepend(53)` on `main-new1`
- call2: `prepend(89)` on the object returned by call1

Each operation sequence records:

```text
addNode Obj
addNode value
addEdge new.f -> value
addEdge new.g -> oldHead
addVariable return -> new
```

## Why This Synthesizes Cleanly

The synthesis no longer has to encode an external variable update. The operation skeleton handles:

- allocate new object;
- assign new object's `f`;
- assign new object's `g`;
- return the new object.

The only synthesized helper values are:

```text
old head pointer = @thisRef
new value        = @arg0
```

This is why the current trace succeeds cleanly while the older `l`-mutation trace produced a composed method that omitted the variable update.

## Baseline Interpretation

The fixed-ID baseline examples are:

```text
call1:
  input fHeap  [39, 27]
  output fHeap [39, 27, 53]
  input gHeap  [1, null]
  output gHeap [1, null, 0]
  returnRef    2

call2:
  input fHeap  [39, 27, 53]
  output fHeap [39, 27, 53, 89]
  input gHeap  [1, null, 0]
  output gHeap [1, null, 0, 2]
  returnRef    3
```

The baseline succeeds on heap extension:

```text
fHeap = insert(@fHeap, @arg0, @arg0)
gHeap = insert(@gHeap, @arg0, @thisRef)
```

This is not a reliable prepend model. It works because `arg0` is `53` or `89`, both larger than the heap length, so `insert` behaves like append.

The critical result is:

```text
prepend-fixed-returnRef: timeout
```

For return-value prepend, the returned fresh object reference is required. The current baseline component set does not synthesize that value under the fixed budget.

## Paper Guidance

Use this prepend case, not the previous `l`-mutation case.

Do not claim the baseline succeeds on prepend just because it can synthesize `fHeap` and `gHeap` for these two examples. The stronger and fairer statement is that the baseline can fit the heap extension examples but cannot synthesize the return reference, while MOLD avoids that search by representing allocation and return in the operation skeleton.

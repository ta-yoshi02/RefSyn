# Non-separated baseline witness analysis

## Purpose

This document checks whether the paper-facing non-separated fixed-ID baseline can
represent a correct post-state transformer before interpreting a synthesis timeout
as search difficulty.  The baseline receives the pre-state object/field heaps,
receiver, and method arguments, but not the operation sequence.  Its output is the
complete post-state field heaps and, when applicable, the return reference.

The witness analysis is separate from synthesis.  A witness establishes that a
correct term exists in a stated language.  It does not show that the synthesizer can
find that term within a given cost or time bound.

## Fixed-ID state model

For the current singly linked object cases, use the following names.

```text
thisRef : Ref[Object[Obj]]
nodeHeap : List[Object[Obj]]
fHeap : List[Int]
gHeap : List[Ref[Object[Obj]]]
```

Object identity is the fixed index in `nodeHeap`.  Therefore extending the heap
allocates the reference `{ ref: length(nodeHeap) }`.  This convention is specific
to the fixed-ID baseline encoding; it is not a general property of PBE.

The witnesses below use the existing read/selection operations and the following
minimal state-construction operations.

```text
append(xs, x) = insert(xs, length(xs), x)

storeAtRef(xs, r, x)
  = the list obtained by replacing xs[r] with x

freshRefFromHeap(nodeHeap)
  = { ref: length(nodeHeap) }

nullRef()
  = { ref: -1 }

pairHeap(fs, gs)
  = { pair: [fs, gs] }
```

`append` and `pairHeap` are already expressible in the experiment setup.  The
paper-facing baseline does not currently provide `storeAtRef`,
`freshRefFromHeap`, or `nullRef` as general components.  A diagnostic
`freshRefFromHeap` exists for `insert_general`, but it is excluded from the
paper-facing component set.

## Verdict for the current paper-facing component set

Under the intended fixed-ID graph semantics, none of the five complete method
transformers is representable by the current paper-facing component set.

| Method | Complete transformer representable now? | Decisive missing capability |
| --- | --- | --- |
| `setAt` | no | write a value-field heap at an object reference selected through `gHeap` |
| `append` | no | construct a fresh reference and write the old tail's pointer slot |
| `prepend` | no | construct and return the fresh reference; the two field-heap extensions alone are expressible |
| `insert` | no | construct a fresh reference and write the selected predecessor's pointer slot |
| `popBack` | no | write `null` into the pointer slot selected by `penultimateRef` |

This verdict is stronger than observing a timeout.  It follows from the types and
value provenance of the current components.

### Fresh-reference impossibility

The current inputs contain references only to pre-state objects and the null
sentinel.  Every current component returning `Ref[Object[Obj]]` is a selector:

- a reference-valued input variable;
- `head` over an existing reference list;
- an automatically generated field reader such as `gOf`;
- `nthNextRef`, `last_ptr`, or `penultimateRef`.

These components can only return a reference already reachable from an input
value.  List constructors such as `cons` and `insert` can rearrange or duplicate
such references but cannot create a new one.  `length` returns `Int`, and the type
system provides no conversion from `Int` to `Ref[Object[Obj]]`.  Therefore the
fresh fixed ID `{ ref: length(nodeHeap) }` required by `append`, `prepend`, and
`insert` is not in the value closure of the current component set.

### Reference-indexed write impossibility

The baseline encodes a field as a list indexed by fixed object ID.  The current
pointer selectors return `Ref[Object[Obj]]`, while list positions are supplied as
`Int`.  There is no `Ref -> Int` conversion and no component that writes a list at
a `Ref` index.  Consequently a target such as
`nthNextRef(thisRef, nodeHeap, gHeap, i)` cannot be used to update the corresponding
slot of `fHeap` or `gHeap`.

A recursive `head`/`tail`/`cons` function can update the `i`-th list position in
the saved chain-ordered examples.  That is not a general witness: traversal
position and fixed object ID need not coincide.  Likewise, rebuilding `gHeap` by
matching an outgoing successor is unsound when several objects contain the same
reference, especially the null sentinel.

### What remains expressible now

The following heap fragments are representable with the current general list
components:

```text
appendField(xs, x) = insert(xs, length(xs), x)

prepend.fHeap' = appendField(fHeap, value)
prepend.gHeap' = appendField(gHeap, thisRef)
```

They do not form a complete `prepend` witness because the required return value is
the new reference, which remains unconstructible.  The existing synthesized term
`insert(heap, value, value)` should not be used as evidence of this expressibility;
the correct general index is `length(heap)`.

## Correct witnesses

The following terms describe the intended method semantics, not merely an
expression that happens to fit the recorded examples.

### `setAt(i, value)`

```text
target = nthNextRef(thisRef, nodeHeap, gHeap, i)
fHeap' = storeAtRef(fHeap, target, value)
gHeap' = gHeap
result = pairHeap(fHeap', gHeap')
```

The current baseline does not have a reference-indexed list replacement
operation.  A recursive `head`/`tail`/`cons` term can replace the `i`-th list
position only when traversal position happens to equal fixed object ID.  That is
true of the saved examples but is not a correct witness for the fixed-ID graph
encoding in general.  `storeAtRef` (or an equivalent reference-to-index/write
operation) is required for the intended semantics.

### `append(value)`

```text
newRef = freshRefFromHeap(nodeHeap)
tailRef = last_ptr(thisRef, gHeap)
fHeap' = append(fHeap, value)
gHeap' = append(storeAtRef(gHeap, tailRef, newRef), nullRef())
result = pairHeap(fHeap', gHeap')
```

The current paper-facing component set cannot construct `newRef`.  This is a
type-level expressiveness gap: integer `length` results cannot be used as
`Ref[Object[Obj]]`.  It also lacks the general reference-indexed write used to
rewire the old tail.

### `prepend(value)`

```text
newRef = freshRefFromHeap(nodeHeap)
fHeap' = append(fHeap, value)
gHeap' = append(gHeap, thisRef)
returnRef = newRef
result = (pairHeap(fHeap', gHeap'), returnRef)
```

The field-heap extension is expressible with `insert` and `length`.  The complete
method result is not expressible in the current paper-facing component set because
the fresh return reference cannot be constructed.  The existing synthesized term
`insert(heap, value, value)` is not a witness: it appends only because the recorded
values exceed the heap length.

### `insert(i, value)`

```text
newRef = freshRefFromHeap(nodeHeap)
predecessor = nthNextRef(thisRef, nodeHeap, gHeap, i)
successor = nthNextRef(thisRef, nodeHeap, gHeap, inc(i))
fHeap' = append(fHeap, value)
gHeap' = append(storeAtRef(gHeap, predecessor, newRef), successor)
result = pairHeap(fHeap', gHeap')
```

The current paper-facing component set cannot construct `newRef` and cannot
perform the reference-indexed rewrite.  The successful value-heap expression
`insert(fHeap, value, value)` is an accidental fit to the recorded values and is
not a correct witness.

### `popBack()`

```text
predecessor = penultimateRef(thisRef, gHeap)
fHeap' = fHeap
gHeap' = storeAtRef(gHeap, predecessor, nullRef())
result = pairHeap(fHeap', gHeap')
```

No allocation is required, but the current baseline still lacks a general
reference-indexed write and a direct null-reference constructor.  A term that
updates `length(gHeap)-2` would fit the saved chain-ordered examples but would not
be a correct witness for arbitrary fixed-ID object order.

For the current non-empty list inputs, a null reference can also be selected by
reading the successor of `last_ptr(thisRef, gHeap)`.  Thus a direct `nullRef`
constant is useful as a general heap-language primitive, but it is not the
decisive missing capability for this particular `popBack` case; the decisive gap
is the reference-indexed write.

## Mechanical check against saved examples

The witnesses above were evaluated as deterministic JavaScript state
transformers against every saved baseline example on 2026-08-01.

| Method | Saved examples | Witness matches expected output |
| --- | ---: | --- |
| `setAt` | 3 | yes, 3/3 |
| `append` | 3 | yes, 3/3 |
| `prepend` | 2 | yes, 2/2, including return reference |
| `insert` | 3 | yes, 3/3 |
| `popBack` | 2 | yes, 2/2 |

This check validates the state equations and the fixed-ID convention.  It does
not yet validate that the exact witnesses are accepted by the current Escher-ts
term grammar or that their costs are below `maxCost=14`.

## Missing-component assessment

| Capability | Needed by explicit-state synthesis in general? | Needed as a MOLD PBE component? | Where MOLD obtains the capability |
| --- | --- | --- | --- |
| Reference-indexed field write (`storeAtRef`) | yes, for general heap updates represented as pure state transformers | no for the covered cases | the operation-derived assignment statement performs the write |
| Fresh reference construction (`freshRefFromHeap`) | yes, when the synthesized transformer represents allocation | no for the covered cases | an operation-derived object-creation statement allocates the object |
| Null reference construction (`nullRef`) | yes, when the post-state transformer must synthesize pointer clearing | no for the covered cases | `deleteEdge` is recomposed as assignment to `null` |
| Product/record construction (`pairHeap`) | yes, when all post-state fields are returned as one pure value | no | MOLD synthesizes holes separately and recomposes a method |
| Pointer selection (`nthNextRef`, `last_ptr`, `penultimateRef`) | domain-dependent; required for these list witnesses unless synthesized recursively | yes | these are supplied by the current list-oriented PBE component library |

These capabilities are not optional in a semantic sense.  Any system that
constructs allocating heap updates must represent allocation, field writes, and
null in some layer.  The difference is where they live.  A non-separated pure
baseline must expose them in the functional state-transformer language.  MOLD
moves them out of PBE search and into the operation-derived imperative template.
This is the decomposition being evaluated.

### General additions that are acceptable

The following additions are generic operations of the fixed-ID state
representation.  They do not encode any one benchmark method and are suitable for
consideration in a fair non-separated baseline.

```text
storeAtRef[A,C] : List[A] -> Ref[Object[C]] -> A -> List[A]
freshRefFromHeap[C] : List[Object[C]] -> Ref[Object[C]]
nullRef[A] : Ref[A]
```

`storeAtRef` is the functional-state counterpart of a field assignment.
`freshRefFromHeap` exposes the allocation convention already chosen by the
fixed-ID encoding.  `nullRef` exposes the encoding's null sentinel without forcing
the synthesizer to recover it from a non-empty pointer heap.

The output-packaging operation should also be generic or mechanically generated
from the detected fields.  The existing monomorphic `pairHeap` is acceptable as
an encoding adapter because it only packages two independently computed outputs;
it does not perform a data-structure update.

### Specialized additions that must be rejected

Do not add components whose name or behavior directly implements a benchmark
method, for example:

```text
setAtHeap
appendHeap
prependHeap
insertHeap
popBackHeap
rewireForInsert
removeTailPointer
```

Also reject a component that bundles several benchmark-specific writes and an
allocation into one operation.  Such a component would move the oracle into the
baseline vocabulary rather than make the state representation complete.

The current timeout results therefore combine two effects:

1. MOLD reduces the size and output type of each PBE problem.
2. MOLD's template supplies update/allocation capabilities that are absent from
   the paper-facing non-separated component set.

Until the non-separated baseline is given the missing generic capabilities and a
machine-checked witness within the configured cost bound, its timeout must not be
described purely as search-space difficulty.

## Follow-up experiment completed on 2026-08-01

The first three follow-up requirements were completed.  Generic `storeAtRef` and
`freshRefFromHeap` components were added to the Escher-ts heap domain.  All 16
hand-written typed witnesses matched all 41 saved task/example cells.  Their
maximum cost was 32, so synthesis was rerun with `maxCost=32`,
`timeoutMs=15000`, and three repetitions.

The whole-state `append` and `insert` tasks timed out in every repetition even
though their correct witnesses were within the cost bound.  `setAt`, `prepend`,
and `popBack` produced terms fitting the saved examples.  Several successful terms
are visibly example-specific, so independent examples with values below heap
length and varied fixed-ID/traversal orders remain required before claiming
generalization.  Full artifacts and exact timings are in
`docs/evaluation_cases/nonseparated_general_components/README.md`.

## Timing boundary in Kanon

End-to-end synthesis latency is measurable in the current Kanon control flow.
The request is dispatched in `Kanon/src/js/testize.js` by
`dispatchSynthesisRequest(...)`.  The returned code is then installed with
`replaceMethodDefinitionSource(...)`, or inserted through the fallback editor
path.

Record at least two intervals with `performance.now()`:

```text
request latency:
  immediately before dispatchSynthesisRequest(payload)
  -> immediately after its Promise resolves or rejects

visible synthesis latency:
  entry to synthesize(), after the user requests synthesis
  -> immediately after method replacement or fallback editor insertion
```

The second interval corresponds to the user-visible delay from submitting the
stored specifications until synthesized code appears in the editor.  Report the
browser/WASM path and localhost HTTP path separately because they include
different transport overheads.  The current helper-runtime CSV files do not
measure this interval.

## Validation terminology

Use the following terms separately.

1. **Training-example satisfaction**: the PBE backend reports that a helper or
   whole-state term matches the examples used for synthesis.
2. **Reconstruction check**: MOLD emits a complete method, the method parses and
   executes, and all required helpers are present.
3. **Behavioral validation**: execute the reconstructed method and a hand-written
   reference implementation on inputs not used for synthesis, then compare the
   complete post-state object graph and return value modulo the documented object
   identity normalization.

Only the third check evaluates whether the synthesized output generalizes beyond
the examples supplied to the synthesizer.  It should be performed outside the
latency-to-code-insertion interval unless the product deliberately validates code
before inserting it.

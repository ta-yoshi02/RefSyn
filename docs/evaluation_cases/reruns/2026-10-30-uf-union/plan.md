# Plan, written before recording (2026-10-30T02:10:34Z)

Target: `union(other)` of union by rank, where `this` and `other` are distinct roots
(`parent === null`) with **different** ranks: the root of lower rank gets `parent` = the other root.
Ranks are not changed (that happens only on equal ranks, see the tie recording).

## Recording 1 (`union/source_before.js`), four calls, all with different ranks

| Call | this (rank) | other (rank) | Write |
| --- | --- | --- | --- |
| `a.union(b)` | a (1) | b (0) | `b.parent = a` |
| `d.union(e)` | d (0) | e (1) | `d.parent = e` |
| `c.union(a)` | c (2) | a (1) | `a.parent = c` |
| `e.union(c)` | e (1) | c (2) | `e.parent = c` |

Operation per call: `addEdge X.parent → Y` (the root's `parent` is null before).

Runs on its task: (A) the server as is (adapter defaults, current components); (B) the same saved task
with one added component `lt: (Int, Int) → Bool`, searched offline with the adapter settings; nothing else changed.

Prediction: the current components have no comparison of two integers (`isZero, isNonNeg, zero, inc, dec,
equal`), so (A) cannot express `rank(this) < rank(other)`. In all four calls the higher rank is the lower
one plus 1, so e.g. `equal(rank(this), inc(rank(other)))` fits; (A) is expected to return such a term
or fail. (B) is expected to return `lt(rank(this), rank(other))`-based terms.

Other inputs (fixed in advance): two separate roots, each with rank r in 0..3 and r1 ≠ r2, in both
orders (12 pairs), each root with no children; compare every `parent` and `rank` field with the reference.

## Recording 2 (`union_tie/source_before.js`)

`a.union(b)` (1 vs 0: `b.parent = a`, one write) then `f.union(g)` (0 vs 0: `g.parent = f` and
`f.rank = 1`, two writes). Prediction: rejected before synthesis, because the two operation graphs have
different numbers of writes and the alignment requires isomorphic graphs.

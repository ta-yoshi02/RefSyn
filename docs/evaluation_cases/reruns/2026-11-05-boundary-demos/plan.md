# Plan (written before recording)

Re-record, in Kanon, the cases where a method overfitted the recorded demonstrations and the
counterexample-guided runs of `../2026-11-04-environment-audit/` (`cegis_mold.*`, `cegis_minimal.*`) fixed
it with a few added examples. Those added examples were computed, not demonstrated; here they become
demonstrations made by hand in Kanon. The new recordings replace the old ones as the reference for both
MOLD and the non-separated baseline (same demonstrations for both).

Out of scope: union (the search order, not the demonstrations, decides the term under the current
setting), the full union with equal ranks (not isomorphic, outside MOLD's problem domain), baseline
append/insert timeouts (expected).

## Programs (`<case>/source_before.js`)

Each program is the old recording's program with calls added at the end; the old calls are kept
unchanged and recorded again.

| Case | Old recording | Added calls | Why |
| --- | --- | --- | --- |
| rotateLeft | `2026-10-30-new-cases/rotateLeft` (3 calls) | `a`, `b`, `c` rotated at their roots: `a.right.left` has a left child; `b.right` has both children as leaves; `c.right.right` has a right child; none of `a`, `b`, `c` has a left child | the three inputs added by `cegis_mold` (rotateLeft) |
| rotateRight | `2026-10-15-rotate-right-4demos-494d6d4/rotateRight` (4 calls) | `d`: `d.left.left` a leaf, `d.left.right` has a right child; `e`: `e.left.left` has a left child, `e.left.right` a leaf; neither has a right child | the two inputs added by `cegis_mold` (rotateRight, 4 demonstrations) |
| prepend | `2026-10-14-fresh-recording/prepend` (2 calls) | `prepend(0)`, `prepend(1)` | in the old calls `x` exceeds the number of objects, so `insert(heap, x, …)` clamps to the end and equals appending; `cegis_minimal` added `x = 0` |
| append | `2026-10-14-fresh-recording/append` (3 calls) | `append(0)`, `append(1)` | same overfit term in the baseline (`insert(@fHeap, @arg0, @arg0)`) |
| insertAfter | `2026-10-30-new-cases/insertAfter` (3 calls) | `insertAfter(0)` after 15, `insertAfter(1)` after 25 | same pattern in the baseline (`insert(@nextHeap, @x, …)`) |

The added tree shapes were checked offline before recording: appended to the old MOLD tasks, they give
the intended helpers under the current setting (rotateRight needed the exact shapes from `cegis_mold`;
mirrors of the rotateLeft shapes did not suffice).

## What is checked

1. Kanon (current setting, server limits 20 / 2 s): synthesis, translation, whole-method validation,
   re-run in Kanon.
2. MOLD's helpers and generated method on the inputs not demonstrated, with the same input sets and
   references as before (`2026-10-30-new-cases/check_new_cases.mjs` and `2026-10-30-followthrough`).
3. The non-separated baseline built from the same recordings, same encodings, components and limits as
   `../2026-11-04-environment-audit/` (lists) and `../2026-10-30-baseline/` (trees, doubly linked list).

If a recording still gives a wrong helper, no demonstration is discarded; further calls are added in a
new recording and both are reported.

## Addendum (after the first five recordings, before recording `append_older_tail`)

With the append recording above, the baseline's next-heap task returned
`insert(@gHeap, dec(length(@nodeHeap)), freshRefFromHeap(@nodeHeap))` (35/60): in every append call the
tail is the most recently created object, so "the last object" and "the tail" coincide. Small x does
not separate them. New recording `append_older_tail`: the same program with a second list `m` (6, 7)
created before the calls and a last call `m.append(2)`, whose tail is older than the appended objects.
Checked offline before recording: the baseline's gHeap and pair tasks then time out (32 / 15 s).

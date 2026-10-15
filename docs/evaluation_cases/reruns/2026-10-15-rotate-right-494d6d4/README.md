# 2026-10-15: binary-tree right rotation at 494d6d4

One Kanon recording of `rotateRight` on a binary tree, to see whether the current
template, unchanged, handles a structure other than a list: a class with two pointer
fields, two writes whose second needs a reference the first overwrites, and a return value.
Nothing in `src`, `runtime`, Kanon or escher-ts was changed, and no component was added.

## Conditions

Same as `../2026-10-15-fixed-494d6d4/README.md` (RefSyn `494d6d4`, Kanon `60fa5c8`,
escher-ts `4e7c4bc`, the 14 library components plus the field components escher-ts derives
from the class, the native server with the adapter defaults `maxCost=20`, `timeoutMs=2000`).
`rotateRight/provenance.txt` records the revisions and hashes.

Program (`rotateRight/source_before.js`): `class TreeNode { constructor() { this.left = null; this.right = null; } rotateRight() {} }`,
one tree, three calls:

| Call | Receiver (this) | this.left | this.left.right | Shape around it |
| --- | --- | --- | --- | --- |
| `root = root.rotateRight()` | 50 (root) | 30 | 40 | left has a left child (20); this has a right child (70) |
| `root.right.right = root.right.right.rotateRight()` | 70 (depth 2) | 60 | 65 | left has no left child; this has a right child (80) |
| `root.right.left = root.right.left.rotateRight()` | 40 (depth 2) | 35 | 37 | left has no left child; this has no right child |

In all three, `this.left` and `this.left.right` are nodes, and `this.left.right` has no
right child (in call 1 it has a left subtree, 35 and 37).

## Feasibility (checked first)

- Kanon showed the tree (10 nodes, `left`/`right`/`val` edges) in the test window.
- All three calls, including the assignment forms, were recognised as calls (`call1`–`call3`).
  Before synthesis the program stops at the second call (`root` is `undefined` because the
  method is empty), as with `prepend`; after a demonstration is accepted Kanon continued
  with the demonstrated state, and the next call's pre-state was the expected one
  (`root` → 30 before call 2; `50.right` → 60 before call 3).
- The return value was demonstrated as an edge from `def var` labelled `return`
  (`addVariable return`), as in the prepend recording.

## Result

Recorded operations per call (`rotateRight/operations.json`): `editEdgeReference this.left
l→lr`, `editEdgeReference l.right lr→this`, `addVariable return → l`.

| Stage | Outcome |
| --- | --- |
| Request | HTTP 200 |
| Synthesis | 3/3 helpers |
| Translation | 3/3 |
| Whole-method validation | `passed`, 3/3 demonstrations |
| Applied to the editor | yes |
| Kanon re-run | 3/3 `testPassed` (`rotateRight/kanon_rerun_markers.json`) |
| Node re-run of the demonstrated calls | every check true (`rotateRight/node_rerun_result.json`, part A) |
| Transcription check | response equal except task path and key order; task JSON identical |

Composed method (`rotateRight/source_after.js`):

```js
rotateRight() {
    const hole0 = this.rotateRight_h0();
    const hole1 = this.rotateRight_h1(hole0);
    const hole2 = this.rotateRight_h2(hole0, hole1);
    hole0.left = hole1;
    hole2.right = hole0;
    return hole2;
}
```

Both references the writes need (`this.left` and `this.left.right`) are read into holes
before `hole0.left` is overwritten, so the template keeps them.

Node re-run, part A (`check_rotate.mjs`, outside Kanon): for each demonstrated call the
returned object is the receiver's original left child, the receiver's `left` is the
original `left.right`, the returned object's `right` is the receiver, only those two
references changed (compared on the objects that existed before the call, before the
caller's own assignment), and the same objects stay reachable. The final tree equals the
expected one.

## The helper for `this.left.right` is not `this.left.right`

| Helper | Term (escher-ts) | Meaning |
| --- | --- | --- |
| `rotateRight_h0` | `@thisRef` | the receiver |
| `rotateRight_h1` | `last_ptr(leftOf(@thisRef, @nodeHeap, @nodeHeap), @rightHeap)` | **the last node on the `right` chain from `this.left`** |
| `rotateRight_h2` | `leftOf(@thisRef, @nodeHeap, @nodeHeap)` | `this.left` |

`last_ptr(x, @rightHeap)` follows `right` from `x` to the end. In all three demonstrations
`this.left.right` has no right child, so the end of that chain is `this.left.right` and
the term satisfies every example; C1 and the whole-method validation hold. On an input
where `this.left.right` has a right child the two differ. Part B of `node_rerun_result.json`
runs the applied method on such a tree (not a demonstrated input):

| | Result |
| --- | --- |
| input | `50(30(20, 40(-, 45)), 70)` |
| generated method | `30(20, 50(45, 70))` — node 40 is no longer reachable |
| right rotation | `30(20, 50(40(-, 45), 70))` |

By the paper's cost (each input variable, constant and application costs 1) the found term
costs 6; `rightOf(leftOf(@thisRef, @nodeHeap, @nodeHeap), @nodeHeap, @rightHeap)` would
cost 7. That the search returned the cheaper term is consistent with this, but the search
order was not examined further.

`leftOf(@thisRef, @nodeHeap, @nodeHeap)` passes `@nodeHeap` where escher-ts names a
`leftHeap`; escher-ts uses that third argument only for a bounds check and reads the field
from the object (`external/escher-ts/src/components/user-friendly-json.ts`, `accessField`),
so the term means `this.left`, as the translation (`(this).left`) does.

What this supports: with the template and components unchanged, a right rotation on a
two-pointer class was recorded, synthesized, validated, applied and re-run on its three
demonstrations, and the template kept the references the writes overwrite. What it does
not: that the synthesized method is a right rotation. The demonstrations did not separate
`this.left.right` from the end of its `right` chain, and the result is wrong on an input
that does.

## How it was recorded, and deviations

- As in `../2026-10-15-fixed-494d6d4/` (vis toolbar, page values read before each save,
  operations read after each edit, `fetch` wrapper copying the response to the receiver).
- **Layout only, through JS:** in each test window the node positions were set with
  `nodes.update({id, x, y, fixed: true})` and vis' physics was stopped with
  `network.stopSimulation()`, because the nodes overlapped and the edges kept moving, so a
  click on an edge missed it. This changes where nodes are drawn, not the graph or the
  recorded operations; every recorded operation was read back after the edit.
- No demonstration was discarded.
- `finalize_case.mjs` is the 2026-10-14 one without its list re-run; the re-run is
  `check_rotate.mjs`. A first version of `check_rotate.mjs` counted the caller's own
  assignment (`root.right.right = ...`) as a change made by the method and reported two
  false failures; it was corrected before the result above was written.

## Layout

| Path | Content |
| --- | --- |
| `rotateRight/` | the recording (same files as the renamed recordings) |
| `check_rotate.mjs` | Node re-run (part A: demonstrated calls; part B: one input outside them) |
| `finalize_case.mjs`, `collect_provenance.sh` | evidence extraction and provenance |
| `logs/` | server, Kanon and receiver logs (the `.log` files match `.gitignore`) |
| `SHA256SUMS` | sha256 of every other file here |

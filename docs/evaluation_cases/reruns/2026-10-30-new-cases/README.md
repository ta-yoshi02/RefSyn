# 2026-10-30: three methods recorded after the search settings were fixed

Three methods not recorded before (`plan.md`, written before recording): `swapChildren()` and
`rotateLeft()` on a binary tree, and `insertAfter(x)` on a doubly linked list (it creates an object).
They serve as cases that were not used to choose the candidate setting of
`../2026-10-30-search-choice/`. Both settings were fixed before recording and compared as they are:

- **current**: adapter defaults (`searchSizeFactor` 3, every component cost 1). Kanon's server ran this.
- **candidate**: `searchSizeFactor` 1, list components cost 3, with the patched dist described in
  `../2026-10-30-search-choice/README.md`. Run offline on the saved tasks.

Nothing in `src`, `runtime`, Kanon or escher-ts was changed (RefSyn `a21e42c`, Kanon `93e6ba7`, escher-ts
`4e7c4bc`, adapter sha256 `9540366b…`, native server `target/debug/refsyn`).

## Recordings (Kanon, current setting)

| Case | Demonstrations | Operations per call | Synthesis / translation | Validation | Kanon re-run |
| --- | --- | --- | --- | --- | --- |
| swapChildren | 3 (at 50, 70, 30) | 2 × `editEdgeReference` | 3/3, 3/3 | passed 3/3 | 3/3 `testPassed` |
| insertAfter | 3 (after 10, 20, 30) | `addNode Node`, `addNode x` (literal), 3 × `addEdge` from the new node, 2 × `editEdgeReference` | 3/3, 3/3 | passed 3/3 | 3/3 `testPassed` |
| rotateLeft | 3 (at 50, 50, 30) | 2 × `editEdgeReference`, `addVariable return` | 3/3, 3/3 | passed 3/3 | 3/3 `testPassed` |

Composed methods are in `<case>/source_after.js`; operations in `<case>/operations.json`.

## Helpers found (`search_settings.mjs`, `search_settings_result.json`)

| Case | Helper (intended) | current, all demonstrations | candidate, all demonstrations |
| --- | --- | --- | --- |
| swapChildren | `h1` this.right, `h2` this.left | `rightOf(this)`, `leftOf(this)` | same |
| insertAfter | `h0` x, `h1` this, `h2` this.next | `x`, `this`, `nextOf(this)` | same |
| rotateLeft | `h1` this.right.left | `if isNull(penultimateRef(this, @leftHeap)) then last_ptr(last_ptr(this, @rightHeap), @leftHeap) else leftOf(penultimateRef(this, @rightHeap))` (cost 16) | `leftOf(rightOf(this))` (7) |
| rotateLeft | `h2` this.right | `rightOf(this)` | same |

Same results under the adapter limits and the paper's.

**Every non-empty proper subset of the demonstrations** (adapter limits):

| Case | current | candidate |
| --- | --- | --- |
| swapChildren | `h1` wrong from {2}, {3} (`penultimateRef`, `last_ptr` along `right`); `h2` wrong from {1}, {2}, {3}, {2,3} | intended from every subset |
| insertAfter | `h2` wrong from {2}, {3} | intended from every subset |
| rotateLeft | `h1` wrong from every subset; `h2` wrong from {1}, {2}, {1,2}, {3} | intended from every subset |

With the current setting the result depends on which demonstrations are given, as in
`../2026-10-30-dll-unlink/`; with the candidate setting one demonstration is enough in these three cases.

## Other inputs (`check_new_cases.mjs`, `off_demo_result.json`)

Fixed in `plan.md`: swapChildren on every node with two children of complete trees of height 1–3 and of
the recorded tree (14); insertAfter on every non-tail position of lists of length 2–7 (21); rotateLeft
at the root of trees where `this.right.left` exists, with `this.left`, `this.right.right`,
`this.right.left.left` and `this.right.left.right` each absent, a leaf, or a node with a right child (81).
Every field of every object and the return value are compared with a reference. The candidate's
methods are the applied class with each helper replaced by the candidate's term, translated by the
adapter's own `compileRefsynMethod`.

| Case | current (applied in Kanon) | candidate |
| --- | --- | --- |
| swapChildren | 14/14 | 14/14 |
| insertAfter | 21/21 | 21/21 |
| rotateLeft | **21/81** | **81/81** |

## What this supports and does not

Supports: on three methods recorded after the candidate setting was fixed, the candidate gives the
intended helpers from every subset of the demonstrations, and the generated methods agree with the
references on all checked inputs; the current setting fails on rotateLeft with all three
demonstrations and on several subsets of the other two.

Does not: generality beyond small pointer rewirings of one class. All three methods are of the kind
the candidate was designed for (fixed field paths from the receiver); none needs a traversal of
unbounded length or a condition, which is where the candidate's cost and time were least favourable
(`../2026-10-30-search-choice/`: union `h0` 1.4 s of 2 s). The demonstrations were recorded by the
author of the setting, from a plan written by the same person.

## How it was recorded, and deviations

- Same set-up as `../2026-10-30-dll-unlink/`: ports checked, servers started by this session and
  stopped afterwards; program put in with `__$__.editor.setValue`; test windows opened through
  "set test"; node positions fixed and some edges drawn curved with JS (drawing only); every edit made
  with the mouse through vis' toolbar and read back from `__$__.Testize.focusedTestOperations`.
- The browser pane was hidden, so screenshots were stale; positions of nodes and of the edit handles
  were read from the page (`network.manipulation.temporaryIds`) instead.
- Kanon's `network-popUp` element stayed visible after loading; it was hidden with JS (drawing only).
- insertAfter call 2: the first click meant for "Edit" hit "Add Node" (the toolbar was already open),
  so the new object was placed under the toolbar; it was moved with `nodes.update` (drawing only). The
  recorded operations are the intended ones.
- rotateLeft call 3: one drag for the `return` edge moved the canvas instead; nothing was recorded by it
  (read back) and the edge was drawn again.
- No demonstration was discarded.

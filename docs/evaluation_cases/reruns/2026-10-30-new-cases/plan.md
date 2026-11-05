# Plan, written before recording (2026-10-30T04:17:37Z)

Three methods not recorded before, on structures other than a singly linked list. Settings are fixed
before recording and compared as they are: **current** (adapter defaults: `searchSizeFactor` 3, every
component cost 1) and **candidate** (`searchSizeFactor` 1, list components cost 3, the patched dist of
`../2026-10-30-search-choice/`), each under the adapter limits (20, 2 s) and the paper's (32, 15 s).
Kanon is served by the current server; the candidate is run offline on the saved tasks. Demonstrations
were chosen to be natural for each method, not to avoid coincidences with list components.

## swapChildren() on a binary tree (`swapChildren/source_before.js`)

Exchange `this.left` and `this.right`; condition: both non-null.

| Call | this | left | right |
| --- | --- | --- | --- |
| `root.swapChildren()` | 50 | 30 | 70 |
| `root.left.swapChildren()` | 70 | 60 | 80 |
| `root.right.swapChildren()` | 30 | 20 | 40 |

Operations per call: `editEdgeReference this.left L→R`, `editEdgeReference this.right R→L`.
Intended helpers: `rightOf(this)` and `leftOf(this)` (in template order).

## insertAfter(x) on a doubly linked list (`insertAfter/source_before.js`)

Create `n` with `val = x`, link it between `this` and `this.next`; condition: `this.next` non-null.

| Call | this | this.next | x |
| --- | --- | --- | --- |
| `head.insertAfter(15)` | 10 (head) | 20 | 15 |
| `head.next.next.insertAfter(25)` | 20 | 30 | 25 |
| `head.next.next.next.next.insertAfter(35)` | 30 | 40 (tail) | 35 |

Operations per call: add the object, `val` → literal x, `n.prev` → this, `n.next` → this.next,
`this.next.prev` → n, `this.next` → n. Intended helpers: `x`, `this`, `nextOf(this)` (or equivalents).

## rotateLeft() on a binary tree (`rotateLeft/source_before.js`)

`r = this.right; this.right = r.left; r.left = this; return r`; condition: `this.right` and
`this.right.left` non-null.

| Call | this | this.right | this.right.left |
| --- | --- | --- | --- |
| `root = root.rotateLeft()` | 50 | 70 | 60 |
| `root.left = root.left.rotateLeft()` | 50 | 60 | 55 |
| `root.left.left.left = root.left.left.left.rotateLeft()` | 30 | 40 | 35 |

Operations per call: `editEdgeReference this.right r→rl`, `editEdgeReference r.left rl→this`,
`addVariable return → r`. Intended helpers: `this`, `leftOf(rightOf(this))`, `rightOf(this)`.

## Checks fixed in advance

- Other inputs: swapChildren on every node with two children of every complete binary tree of height
  1–3 plus the recorded tree; insertAfter on every non-tail position of lists of length 2–7;
  rotateLeft on trees where `this.right.left` exists: right spines with extra left/right subtrees of
  sizes 0–2 (generated in `check_new_cases.mjs`). Compare all fields and the return value with a
  reference. Inputs outside the condition are reported separately.
- Demonstration subsets: every non-empty subset, both settings, adapter limits.
- Non-separated baseline: see `../2026-10-30-baseline/`.

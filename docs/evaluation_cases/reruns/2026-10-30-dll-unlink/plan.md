# Plan, written before recording (2026-10-30T01:57:54Z)

Target: `unlink()` on a middle node of a doubly linked list (`this.prev` and `this.next` non-null):
`this.prev.next = this.next; this.next.prev = this.prev;` (the node's own fields are left as they are).

Demonstrations (fixed in advance, `unlink/source_before.js`), list 10..60:

| Call | Receiver | this.prev | this.next | Note |
| --- | --- | --- | --- | --- |
| 1 | 30 | 20 | 40 | neither neighbour is an end |
| 2 | 20 | 10 (head) | 40 | prev is the head |
| 3 | 50 | 40 | 60 (tail) | next is the tail |

Operations per call: `editEdgeReference prev.next this→next`, `editEdgeReference next.prev this→prev`.

Prediction from the 2026-10-30 diagnosis: `last_ptr(this, prevHeap)` equals `this.prev` only in call 2 and
`last_ptr(this, nextHeap)` equals `this.next` only in call 3, so call 1 should exclude both; the field reads
`prevOf(this)` / `nextOf(this)` cost 4.

Off-demonstration check (fixed in advance): for every list length 3..7 and every middle position, run the
generated method and a reference unlink on separate copies and compare all val/prev/next fields and reachability.
Inputs with `this.prev` or `this.next` null are outside the demonstrated condition and are reported separately.

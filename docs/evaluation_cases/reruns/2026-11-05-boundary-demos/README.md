# 2026-11-05: boundary demonstrations recorded in Kanon

The cases where a method overfitted its recorded demonstrations, re-recorded by hand in Kanon with the
added demonstrations the counterexample-guided runs of `../2026-11-04-environment-audit/` had pointed
to (`plan.md`, written before recording, with an addendum before the last recording). These recordings
replace the old ones as the reference for MOLD and for the non-separated baseline (both use the same
demonstrations). Out of scope (`plan.md`): union (the current setting's search order decides the term),
the full union with equal ranks (not isomorphic), the baseline's append/insert timeouts.

Set-up: RefSyn worktree at `53c6d1a` (adapter sha256 `9540366b…`, the same as every earlier recording;
the shared checkout's modified adapter was not used), native server built there, Kanon `93e6ba7` served
from the main checkout (unchanged), escher-ts `4e7c4bc`, current setting (`searchSizeFactor` 3, every cost
1, 20 / 2 s). Ports 3030/8001/8765 checked free; servers started by this session and stopped afterwards.

## Recordings (Kanon, current setting)

| Recording | Demonstrations (old + added) | Helpers | Validation | Re-run in Kanon |
| --- | --- | --- | --- | --- |
| `rotateLeft` | 3 + 3 (`a`, `b`, `c`) | `this`, **`leftOf(rightOf(this))`**, `rightOf(this)` | passed 6/6 | 6/6 |
| `rotateRight` | 4 + 2 (`d`, `e`) | `this`, **`rightOf(leftOf(this))`**, `leftOf(this)` | passed 6/6 | 6/6 |
| `prepend` | 2 + 2 (`prepend(0)`, `prepend(1)`) | `x`, `this` | passed 4/4 | 4/4 |
| `append` | 3 + 2 (`append(0)`, `append(1)`) | `x`, `last_ptr(this, next)` | passed 5/5 | 5/5 |
| `append_older_tail` | 5 + 1 (`m.append(2)`, tail older than the appended objects) | `x`, `last_ptr(this, next)` | passed 6/6 | 6/6 |
| `insertAfter` | 3 + 2 (`insertAfter(0)` after 15, `insertAfter(1)` after 25) | `x`, `this`, `nextOf(this)` | passed 5/5 | 5/5 |

Every old call was recorded again with the same operations as before (read back from
`__$__.Testize.focusedTestOperations` after every edit; `<recording>/operations.json`). With the old
demonstrations alone the current setting had returned wrong helpers for rotateLeft (21/81) and
rotateRight (36/81 with 4 demonstrations); with the added ones it returns the field paths.

## Other inputs

MOLD (`check_methods.mjs` → `methods.json`): the class Kanon applied (`<recording>/source_after.js`), run
on separate copies against a reference; same input sets as before.

| Recording | rotateLeft | rotateRight | prepend | append | append_older_tail | insertAfter |
| --- | --- | --- | --- | --- | --- | --- |
| agree | 81/81 | 81/81 | 30/30 | 30/30 | 30/30 | 105/105 |

The checker reproduces the old results on the old classes (rotateLeft 21/81, rotateRight 36/81).

Non-separated baseline, built from the same recordings with the earlier scripts unchanged except for
paths (`make_baseline_tasks.mjs`, `run_baseline_*.mjs`, `check_baseline_*.mjs`, from
`../2026-10-30-baseline/`; `run_list_baseline.mjs`, encoding and components of
`../2026-11-04-environment-audit/tasks_minimal.json`, examples replayed from the recorded operations;
the first calls reproduce the old tasks' examples exactly):

| Case | Old demonstrations | These demonstrations (current setting) |
| --- | --- | --- |
| rotateLeft (+list) | 27/81, 21/81; pair timeout | per-heap 81/81; pair timeout (candidate setting 81/81) |
| rotateRight (+list) | 36/81, 54/81; pair timeout | per-heap 81/81; pair timeout (candidate setting 81/81) |
| insertAfter (+insert, length) | 25–28/105 | 105/105 (all three heaps, both limits; +list variant: next/prev timeout at 2 s, 105/105 at 15 s) |
| prepend | 24/60 (pair) | 60/60 (every task, pair 328 ms) |
| append (`append`) | 16/60 (pair) | 35/60: `insert(@gHeap, dec(length(@nodeHeap)), …)` |
| append (`append_older_tail`) | — | val heap 60/60; **next heap and pair timeout (32 / 15 s)** |

(Without the list components the tree baselines were already right with the old demonstrations; they
stay 81/81.)

## What this shows

- The overfitting of rotateLeft and rotateRight (MOLD, current setting) and of the baseline's
  insertAfter and prepend is a property of the demonstrations: with the added demonstrations, recorded
  in Kanon, both methods give terms that agree on every checked input.
- The baseline's append needed a different kind of demonstration, not a different argument: while every
  appended tail is the most recently created object, "the last object" equals "the tail". With a
  demonstration on an older tail the baseline's next-heap and whole-state tasks no longer finish within
  32 / 15 s; MOLD's helpers are unchanged and its method agrees on all 30 inputs. The hand-written
  witness of `../2026-11-04-environment-audit/typecheck_witnesses_minimal.json` (size about 20) satisfies
  all 6 examples of this recording (`check_append_witness.mjs` → `append_witness.json`), so this is the
  "expressible, not reached" case, not an inexpressible one.
- These are the reference recordings from now on: rotateLeft, rotateRight, prepend, append
  (`append_older_tail`) and insertAfter. The other recordings (setAt, insert, popBack, swapChildren,
  unlink, union) are unchanged.

Does not show: that a user would choose these demonstrations unprompted; they were chosen from the
counterexamples of the offline runs. The added tree demonstrations are separate trees in the same
program, so every heap contains more objects than the old recordings'.

## How it was recorded, and deviations

- Program put in with `__$__.editor.setValue`. The "set test" tooltip was shown with
  `__$__.Testize.displayTooltip` and placed at a fixed position (mouse hover over the parentheses did not
  trigger it in the pane), then clicked with the mouse; that click opens Kanon's test window.
- Drawing only, with JS: node positions (`nodes.update`, the receiver's structure laid out, everything
  else moved out of view), straight edges (curved `prev` edges for the doubly linked list), view
  translation and scale. Every edit was made with the mouse through vis' toolbar (Add Node, Add Edge,
  Edit Edge, popup label and literal checkbox); each batch checked mode, selection and popup state before
  the next click and stopped on a mismatch.
- rotateLeft call 1: a click registered as a double click and opened Kanon's edge-label popup; it was
  closed with its cancel button (no operation recorded, read back).
- In most test windows the first "Add Node" click after opening the toolbar had no effect (mode stayed
  off, nothing recorded); it was clicked again.
- Evidence (`raw_request.json`, `response.json`, `source_after.js`, `kanon_rerun_markers.json`) was
  posted by the page to a one-shot receiver on 127.0.0.1:8765; `mold_escher_tasks.json` is the task file
  the server saved. No demonstration was discarded.

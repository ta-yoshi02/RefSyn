# append: fresh Kanon recording (2026-10-14)

Recorded from scratch in Kanon at RefSyn `b42d9ca`, same setup as
`../insert_general/README.md` (Conditions). The earlier record in
`../../../append/` is untouched.

- Program: `source_before.js` (`class Node { constructor() { this.next = null; } append(x) {} }`,
  list 39 → 27, then `append(53)`, `append(89)`, `append(46)`)
- Demonstrations: 3, four edits each (create node, literal, `val`, tail `next`); the second
  one was entered in a different order (literal first, `next` before `val`)
- Same arguments and pre-call lists as the old record: `equivalence_with_old_payload.json`

| Stage | Outcome |
| --- | --- |
| Request | HTTP 200 |
| Synthesis | 2/2: `append_h0 = @x`, `append_h1 = last_ptr(@thisRef, @nextHeap)` |
| Translation | 2/2 |
| Whole-method validation | `passed`, 3/3 demonstrations |
| Applied | yes |
| Kanon re-run | 3/3 `testPassed` |
| Node re-run (own check) | `39, 27, 53, 89, 46` as expected |

`response.json` was copied by hand from the browser's network log (the capture hook was
added after this case); `transcription_check/result.json` shows it equals a replay of
`raw_request.json` except for the task file name and hash-map key order, and the replay's
task JSON is byte-identical. `source_after.js` and `kanon_rerun_markers.json` were sent
from the page to a local receiver and are byte-exact.

File roles are as in `../insert_general/README.md`.

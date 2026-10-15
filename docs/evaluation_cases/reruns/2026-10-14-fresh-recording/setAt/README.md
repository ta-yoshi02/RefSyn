# setAt: fresh Kanon recording (2026-10-14)

Recorded from scratch in Kanon at RefSyn `b42d9ca`, same setup as
`../insert_general/README.md` (Conditions). The earlier record in
`../../../setAt/` is untouched.

- Program: `source_before.js` (list 39 → 27 → 53 → 89, then `setAt(0, 46)`, `setAt(2, 60)`, `setAt(3, 18)`)
- Demonstrations: 3, two edits each (literal, then a `val` edge from the target node to it;
  Kanon records an edge drawn over an existing `val` edge as `editEdgeReference`)
- Same arguments and pre-call lists as the old record: `equivalence_with_old_payload.json`

| Stage | Outcome |
| --- | --- |
| Request | HTTP 200 |
| Synthesis | 2/2: `setAt_h0 = nthNextRef(@thisRef, @nodeHeap, @nextHeap, @a)`, `setAt_h1 = @b` |
| Translation | 2/2 |
| Whole-method validation | `passed`, 3/3 demonstrations |
| Applied | yes (`hole0.val = hole1`) |
| Kanon re-run | 3/3 `testPassed` |
| Node re-run (own check) | `46, 27, 60, 18` as expected |

`response.json`, `http_status.txt`, `source_before.js`, `source_after.js` and
`kanon_rerun_markers.json` were sent from the page to a local receiver and are byte-exact
(`../logs/receiver.log`). File roles are as in `../insert_general/README.md`.

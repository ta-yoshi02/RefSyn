# popBack: fresh Kanon recording (2026-10-14)

Recorded from scratch in Kanon at RefSyn `b42d9ca`, same setup as
`../insert_general/README.md` (Conditions). The earlier record in
`../../../popBack/` is untouched.

- Program: `source_before.js` (list 39 → 27 → 53 → 89, then `popBack()` twice)
- Demonstrations: 2, one edit each (`deleteEdge` on the predecessor's `next`, via "Delete selected")
- Same pre-call lists as the old record: `equivalence_with_old_payload.json`
- Kanon keeps the detached node (89) on the canvas in the second demonstration, and the
  typed pre-call state of `call2` still lists it (`main-new4`, `next = null`) although it
  is unreachable from the receiver (`typed_precall_state.json`). Synthesis and validation
  passed with it present.

| Stage | Outcome |
| --- | --- |
| Request | HTTP 200 |
| Synthesis | 1/1: `popBack_h0 = penultimateRef(@thisRef, @nextHeap)` |
| Translation | 1/1 |
| Whole-method validation | `passed`, 2/2 demonstrations |
| Applied | yes (`hole0.next = null`) |
| Kanon re-run | 2/2 `testPassed` |
| Node re-run (own check) | `39, 27` as expected |

`response.json`, `http_status.txt`, `source_before.js`, `source_after.js` and
`kanon_rerun_markers.json` were sent from the page to a local receiver and are byte-exact
(`../logs/receiver.log`). File roles are as in `../insert_general/README.md`.

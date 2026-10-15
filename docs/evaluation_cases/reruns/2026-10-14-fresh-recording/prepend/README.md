# prepend: fresh Kanon recording (2026-10-14)

Recorded from scratch in Kanon at RefSyn `b42d9ca`, same setup as
`../insert_general/README.md` (Conditions). The earlier record in
`../../../prepend/` is untouched.

- Program: `source_before.js` (list 39 → 27, then `lst = lst.prepend(53)`, `lst = lst.prepend(89)`)
- Demonstrations: 2, five edits each (create node, literal, `val`, `next` to the receiver,
  and the return value: an edge from Kanon's "def var" box labelled `return`, recorded as
  `addVariable`); the second one creates the literal first
- Same arguments and pre-call lists as the old record: `equivalence_with_old_payload.json`
- Before the first test is set, the program stops with
  `Runtime Error: Cannot read properties of undefined (reading 'prepend')` because the
  empty method returns `undefined`; Kanon's test override removes it once the first
  return value is demonstrated. This is expected and does not affect the request.

| Stage | Outcome |
| --- | --- |
| Request | HTTP 200 |
| Synthesis | 2/2: `prepend_h0 = @x`, `prepend_h1 = @thisRef` |
| Translation | 2/2 |
| Whole-method validation | `passed`, 2/2 demonstrations |
| Applied | yes (ends with `return created0;`) |
| Kanon re-run | 2/2 `testPassed` |
| Node re-run (own check) | `89, 53, 39, 27` as expected |

`response.json`, `http_status.txt`, `source_before.js`, `source_after.js` and
`kanon_rerun_markers.json` were sent from the page to a local receiver and are byte-exact
(`../logs/receiver.log`). File roles are as in `../insert_general/README.md`.

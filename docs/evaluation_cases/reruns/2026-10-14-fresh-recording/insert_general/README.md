# insert: fresh Kanon recording (2026-10-14)

Recorded from scratch in the Kanon UI at RefSyn `b42d9ca` and sent to the native
backend. No saved payload was reused or patched. The earlier record in
`../../../insert_general/` is untouched.

## Result

| Stage | Outcome |
| --- | --- |
| Recording (3 demonstrations, 5 edits each) | done in the UI; operations checked before each accept |
| Request | `POST http://localhost:3030/synthesize`, HTTP 200 |
| Synthesis (escher-ts) | 3/3 helpers solved |
| Translation | 3 helpers translated to JS |
| Whole-method validation | `validation.status = "passed"`, `checked_demonstrations = 3` |
| Applied to the editor | yes: 3 helpers with `// refsyn: generated helper` and the composed `insert` |
| Re-execution in Kanon | all three calls marked `testPassed` |
| Re-execution in Node (own check) | final list `39, 89, 71, 27, 46, 53`, as demonstrated |

This is one recording and one synthesis run. It is evidence that this pipeline
reproduces these three demonstrations, not a performance figure.

Synthesized helpers (escher-ts terms):

- `insert_h0 = @arg`
- `insert_h1 = nthNextRef(@thisRef, @nodeHeap, @nextHeap, inc(@i))` (successor of the insertion point)
- `insert_h2 = nthNextRef(@thisRef, @nodeHeap, @nextHeap, @i)` (insertion point)

Hole numbering follows the first appearance in demonstration 1's edits; here the
`next` edge of the created node was added before the predecessor was redirected,
so the order differs from the paper's listing (`h1` = predecessor there).

## Files

| File | Content |
| --- | --- |
| `source_before.js` | Program in the editor when Synthesize was clicked |
| `raw_request.json` | Exact request body, copied from the server's `--- RAW REQUEST PAYLOAD ---` output |
| `operations.json` | Per call: receiver, arguments and the recorded edits (extracted from `raw_request.json`) |
| `typed_precall_state.json` | `validation` block of the request: class source, `new Node()` state, typed pre-call heaps |
| `mold_escher_tasks.json` | Task JSON the backend wrote (`target/escher/ts/insert_1791956232_451434000_p4348_1.json`) |
| `response.json` | HTTP response body, read from the browser's network log |
| `server_output.txt` | Server output for this request after the raw payload (escher-ts terms, composed code) |
| `source_after.js` | Editor text after the edit was applied; sha256 equals the one computed in the browser |
| `kanon_rerun_markers.json` | Kanon's per-call test markers after the re-run |
| `node_rerun_result.json` | Independent re-run of `source_after.js` in Node |
| `provenance.txt` | Revisions, tool versions, lockfile and binary/runtime/dist/WASM/Kanon hashes |
| `synthesize_clicked_at_utc.txt`, `server_log_lines_before.txt` | When Synthesize was clicked and where this request starts in `../logs/refsyn_server.log` |
| `transcription_check/` | Same raw request posted again with curl; see below |
| `attempt1_constructor_val0/` | First attempt, rejected before synthesis; see below |

`response.json` was copied by hand from the network log, so it was checked by posting
`raw_request.json` again to the same server (`transcription_check/replay_response.json`).
All fields are equal except `list_environment_info` (task file name) and the key order of
`hole_information`, which comes from a hash map. The replay's task JSON is byte-identical
to `mold_escher_tasks.json`.

## Conditions

- RefSyn `b42d9cafa08ea32ea2d79e622e58b25a233d2b02`, Kanon `60fa5c8`, escher-ts `4e7c4bc`
  (full list and hashes in `provenance.txt`)
- Build and start, from the repository root:
  - `git submodule update --init --recursive`
  - `cd external/escher-ts && pnpm install --frozen-lockfile && pnpm run build` (pnpm 10.30.1 from its `packageManager` field)
  - `npm ci --prefix web && npm run test:rebuild --prefix web` (5 files, 28 tests passed)
  - `cargo build`, then `./target/debug/refsyn` with stdout saved to `../logs/refsyn_server.log`
  - `cd Kanon && npm install --ignore-scripts && npm install --ignore-scripts --prefix external/escodegen`
  - `cd Kanon && npx grunt connect:server:keepalive --port=8001` (the watch task is left out:
    its livereload port 35729 was taken by another Kanon on :8000)
- Kanon's page has no browser runtime bundle, so on localhost it posts to `http://localhost:3030/synthesize`.
- escher-ts search: AscendRec with the adapter defaults `maxCost=20`, `timeoutMs=2000`,
  `searchSizeFactor=3` (no `ESCHER_TS_*` variables set)
- Browser: the app's built-in browser, viewport emulated at 1600x1000
- The program was put into the editor with `__$__.editor.setValue` (equivalent to pasting;
  typing is mangled by Ace's auto-closing brackets). All demonstration edits were made with
  the mouse in the test window.

## Problems met while recording

1. **Unfixed defect: Kanon and the backend disagree on a created object's fields**
   (`attempt1_constructor_val0/`). With `constructor() { this.val = 0; this.next = null; }`,
   the request was rejected: `demonstration 1: addEdge on __temp1.val, which already has
   an edge` (HTTP 400, `validation.status = "unavailable"`). The backend replays a created
   object from its captured `new Node()` state (`val = 0`) and accepts `addEdge` only into
   an absent, null or undefined slot (`src/operation_graph.rs`), but Kanon's test window
   shows a created node without any fields, so `addEdge` is the only edit the UI offers.
   Since the design keeps `new C()`'s defaults, the missing fields in the UI are the
   defect; it is not fixed here.
   Minimal reproduction: `cargo run --example run_payload -- attempt1_constructor_val0/raw_request.json`.
   **The accepted recording changes the target program to avoid it**: `this.val = 0` was
   removed from the constructor and nothing else. The result therefore covers classes whose
   `new C()` leaves every field the demonstration adds with `addEdge` absent, null or
   undefined; a class that initializes such a field to another value cannot be
   demonstrated through the UI until the defect is fixed.

   What decides support is the state `new C()` produces when run **without arguments**
   (Kanon `captureInstance`): it must not throw, and every own property must be a
   writable data property holding a primitive, null or undefined. Declared parameters are
   not a reason for rejection; they are `undefined` in that call. The old records'
   `constructor(f, g = null)` yields `f = undefined, g = null` and is within this range.
   Using `val`/`next` and a parameterless constructor here was a choice of this session,
   not a requirement.
2. **The Literal checkbox keeps its previous state** in the toolbar's Add Node dialog
   (only the double-click path resets it). One demonstration was discarded before accept
   because a `Node` object had been recorded as the string literal `"Node"`. The checkbox
   state was read before every save afterwards.
3. Edge-edit drags that miss the control point pan the canvas instead; this recorded nothing
   and was corrected by panning back.

A request not sent by this session reached the server just after it started; see
`../logs/FOREIGN_REQUESTS.md`. It is excluded here.

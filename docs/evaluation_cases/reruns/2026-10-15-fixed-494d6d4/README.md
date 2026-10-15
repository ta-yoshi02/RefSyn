# 2026-10-15: evaluation fixed at 494d6d4, and four renamed recordings

The implementation is fixed at RefSyn main `494d6d4`. Two things were done on it, and
nothing in `src`, `runtime`, Kanon or escher-ts was changed during the work:

1. The five main cases were measured again under the conditions of
   `../2026-10-14-fresh-recording/conditions.md` and compared with the `b42d9ca` results.
2. insert and append were recorded anew in the Kanon UI with renamed classes and fields
   (four recordings) and taken through synthesis, whole-method validation, the editor
   edit and Kanon's re-run.

## Fixed conditions

| Item | Value |
| --- | --- |
| RefSyn | `494d6d442163279ce9105a144ffec60518909f82` (worktree, no tracked change: `refsyn_dirty_tracked_files: 0` in each `provenance.txt`) |
| Kanon | `60fa5c8` (escodegen `0e8280a`) |
| escher-ts | `4e7c4bc`; dist combined hash `50b69f3c…`, equal to the one recorded at `b42d9ca` |
| Adapter | `runtime/refsyn-escher-adapter.mjs` sha256 `9540366b…`, equal to `b42d9ca` |
| Components given to every helper task | `isNull, equal, and, or, not, isZero, isNonNeg, zero, inc, dec, last_ptr, penultimateRef, nthNextRef, findByValueRef`, plus the field components escher-ts derives from the class (`autoClassFieldComponents`) |
| Baseline components | unchanged task file `../../nonseparated_general_components/tasks.json` (23 components, including `storeAtRef`, `freshRefFromHeap`, `pairHeap`) |
| Search | AscendRec, reduction rules on, forward-only off, `searchSizeFactor=3` |
| Limits, measurement (part 1) | `maxCost=32`, `timeoutMs=15000` per task, 10 repetitions |
| Limits, end-to-end server (part 2) | adapter defaults: `maxCost=20`, `timeoutMs=2000` (no `ESCHER_TS_*` set; `logs/refsyn_server_start.txt`) |
| Environment | Darwin 27.0.0 arm64, Rust 1.93.0, Node v22.17.0; machine not isolated |
| Checks of the build | `npm run test:rebuild --prefix web`: 5 files, 28 tests passed; `cargo test`: all passed; `node --test runtime/*.test.mjs`: 51 passed |

**The two limits differ.** The measurement uses 32 / 15 s (the values of the paper draft);
the native server that Kanon calls uses the adapter defaults 20 / 2 s. This was so at
`b42d9ca` as well. The two runs used different conditions; in these cases they found the
same terms. A text that states one limit for "MOLD" describes only one of the two runs.

## Part 1: the five cases re-measured (`remeasure/`)

The MOLD tasks were regenerated from each recorded request with `examples/run_payload`
at `494d6d4`: **byte-identical to the `b42d9ca` tasks** for all five, and validation
`passed` for all (`remeasure/<case>/`). Then `remeasure/run.sh` (same script and settings
as the 2026-10-14 main measurement).

`remeasure/compare_with_b42d9ca.json` (`compare_with_b42d9ca.mjs`):

| | solved (b42d9ca → 494d6d4) | found terms | median time (b42d9ca → 494d6d4) |
| --- | --- | --- | --- |
| MOLD insert | 10/10 → 10/10 | same | 17.10 → 6.81 ms |
| MOLD append | 10/10 → 10/10 | same | 1.96 → 1.09 ms |
| MOLD popBack | 10/10 → 10/10 | same | 1.24 → 0.84 ms |
| MOLD prepend | 10/10 → 10/10 | same | 0.93 → 0.41 ms |
| MOLD setAt | 10/10 → 10/10 | same | 4.90 → 4.14 ms |
| baseline insert | 0/10 → 0/10 | — | 15.00 s (limit) both |
| baseline append | 0/10 → 0/10 | — | 15.00 s (limit) both |
| baseline popBack | 10/10 → 10/10 | same | 5.82 → 5.80 s |
| baseline prepend | 10/10 → 10/10 | same | 0.248 → 0.271 s |
| baseline setAt | 10/10 → 10/10 | same | 2.76 → 3.82 s |

Solved/unsolved and every found term are unchanged. Between the two measurements compared
here, the medians differed by up to about a factor of 2.5 (MOLD insert; baseline setAt 1.38)
on the same tasks, code and machine. The load average was 4.7 before this run and 2.9
before the 2026-10-14 one (`remeasure/run_log.txt`); whether it caused the difference is not
established.

## Part 2: four renamed recordings

| Case | Class / value field / pointer field | Program |
| --- | --- | --- |
| `insert_unrelated` | `Cell` / `key` / `link` | `insert_unrelated/source_before.js` |
| `insert_swapped` | `Node` / **`next`** / **`val`** (meanings swapped) | `insert_swapped/source_before.js` |
| `append_unrelated` | `Cell` / `key` / `link` | `append_unrelated/source_before.js` |
| `append_swapped` | `Node` / **`next`** / **`val`** | `append_swapped/source_before.js` |

Same values, arguments, calls and edit order as the 2026-10-14 insert and append
recordings; only names differ. The constructor sets the pointer field to `null`.

`stage_summary.json` (`stage_summary.mjs`):

| Case | Demos | Request | Synthesis | Translation | Validation | Applied | Kanon re-run | Node re-run |
| --- | ---: | --- | --- | --- | --- | --- | --- | --- |
| insert_unrelated | 3 | 200 | 3/3 | 3/3 | passed 3/3 | yes | 3/3 | match |
| insert_swapped | 3 | 200 | 3/3 | 3/3 | passed 3/3 | yes | 3/3 | match |
| append_unrelated | 3 | 200 | 2/2 | 2/2 | passed 3/3 | yes | 3/3 | match |
| append_swapped | 3 | 200 | 2/2 | 2/2 | passed 3/3 | yes | 3/3 | match |

`compare_terms.json` (`compare_terms.mjs`): every helper term equals the term of the
original recording after mapping the heaps (`val`→value field, `next`→pointer field), e.g.
`insert_swapped`: `nthNextRef(@thisRef, @nodeHeap, @valHeap, inc(@i))`;
`append_swapped`: `last_ptr(@thisRef, @valHeap)`. The translated JS walks the pointer field
(`.link`, or `.val` in the swapped cases) and the composed methods write the value into the
value field (`created0.key = hole0`, `created0.next = hole0`).

What this supports: for these two methods, no step from recording to re-run depends on the
names `Node`, `val` or `next`, nor on which of two names is the value and which the pointer
field. What it does not: other methods, classes with more than one pointer field, or names
that collide with JS built-ins or with the generated identifiers (`created0`, `hole0`, …).

## How it was recorded, and deviations

- Servers: `./target/debug/refsyn` (3030), `npx grunt connect:server:keepalive --port=8001`
  (Kanon), `logs/receiver.mjs` (127.0.0.1:8765). All three were started by this session
  after checking the ports were free and stopped after the last case.
- The program was put into the editor with `__$__.editor.setValue`; the editor text was
  posted to the receiver and compared with `source_before.js` (`cmp`: identical in all four).
  The page was reloaded between cases, so no test of an earlier case was stored.
- **All edits were made in the test window with the mouse through vis' toolbar** (Edit →
  Add Node / Add Edge / Edit Edge). The double-click path of Kanon could not be used by the
  automated browser: vis 4.17 drops a release less than 10 ms after the press
  (`node_modules/vis/dist/vis.js`, `onRelease`), and an automated click is shorter, while a
  press-and-hold of the tool becomes vis' `hold`. This concerns the tool, not Kanon used by a
  person.
- Before every save of a node, the label and the Literal checkbox were read from the page;
  the checkbox kept its previous state several times (as reported on 2026-10-14) and was set
  by hand before saving. After every save, the recorded operation list was read from
  `__$__.Testize.focusedTestOperations`. No demonstration was discarded.
- In `insert_unrelated`'s second demonstration, the literal node `46` was placed at a
  different canvas position than intended (the placing click fell on the label field). The
  recorded operation is the same (`addNode 46`); positions are not recorded.
- `response.json` and `http_status.txt`: a wrapper around `window.fetch`, installed in the
  page before Synthesize, sent a clone of the `/synthesize` response to the receiver byte
  for byte; Kanon's own handling received the original response. The response was also
  checked by posting `raw_request.json` again (`<case>/transcription_check/result.json`:
  response equal except task path and key order, task JSON identical, in all four).
- `kanon_rerun_markers.json` and `source_after.js` were read in the page after the edit and
  posted to the receiver; the sha256 of the editor text is in the markers file.
- `finalize_case.mjs` was copied from 2026-10-14 and given the value and pointer field names
  as arguments for its Node re-run (it read `.val`/`.next` before). `stage_summary.mjs` lists
  the four cases.

## Layout

| Path | Content |
| --- | --- |
| `remeasure/` | part 1: regenerated tasks and run_payload output per case, `run.sh`, measurement JSON, `summary.md`, `run_log.txt`, `compare_with_b42d9ca.*` |
| `<case>/` | part 2: one recording each (same files as 2026-10-14, plus `source_before_editor.js`) |
| `compare_terms.*` | term comparison with the original recordings |
| `stage_summary.*` | stage counts of part 2 |
| `logs/` | server, Kanon and receiver logs; `refsyn_server_start.txt` |
| `finalize_case.mjs`, `collect_provenance.sh` | evidence extraction and provenance (from 2026-10-14) |
| `SHA256SUMS` | sha256 of every other file here (`shasum -a 256 -c SHA256SUMS` from this directory); the three `logs/*.log` are matched by `.gitignore` and were added explicitly |

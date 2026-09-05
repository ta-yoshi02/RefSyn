# 2026-09-02 editEdgeReference value-operand hole rerun

## Purpose

This rerun verifies the fix that keeps a literal-value `editEdgeReference` in the common operation plan when its `from`, `oldTo`, and `newTo` node IDs differ across demonstrations.

The fix is intentionally limited to literal-value updates. An initial unscoped version also generalized object-reference rewires and regressed `insert` from three holes to two by fixing the successor to the reference trace. Those responses are retained as `response_unscoped_regression.json` in each case directory. The first `setAt` response before preserving dependent `addNode` operations is retained as `setAt/response_before_incremental_fix.json`.

## Provenance and conditions

- Date: 2026-09-02 (Asia/Tokyo)
- Repository HEAD before the working-tree fix: `453b6c58419f75fbb451b56563b4b9003eb4c677`
- Platform: Darwin 25.6.0 arm64
- Rust: `rustc 1.93.0`, `cargo 1.93.0`
- Node.js: `v22.17.0`
- MOLD helper and per-case fixed-ID diagnostic measurement: `maxCost=14`, `timeoutMs=15000`, `repetitions=3`
- Paper-comparable fixed-ID baseline measurement: `maxCost=32`, `timeoutMs=15000`, `repetitions=3`
- Kanon: unchanged; all five payloads are byte-identical copies of the existing records.

Payload SHA-256 values:

| case | SHA-256 |
| --- | --- |
| setAt | `f1b17d3bb8c5fa06825b352774857c9727e4e938316dfeb9faf508d01bfafcf5` |
| append | `53931a953fa8dbb05ac79cb4228627018e1206bcafbc0a3b1169623c13123bec` |
| prepend | `4c28eb47cb003af0acb328b572457fda829ee02973ea962fb0bf3f3ccb4397e9` |
| insert_general | `e3eb596edfed17087860c9868418c5dddbb93962214e89ffaff939f2009f1ac0` |
| popBack | `ee91736f585bb5dd222a3d5e52a2129a665da7aa5b907e469f8fd7aa2324b603` |

## Result

`setAt` now has the intended operation-level skeleton:

```text
COMMON_PLAN (operation-level, ordered)
[op_1] editEdgeReference(from=__hole_2, to=__hole_1, label=f)

HOLE_BINDINGS
__hole_1 -> setAt-h (Int)
__hole_2 -> setAt-p (Ptr)
```

The exact hole and helper suffixes are internal names and may vary with candidate enumeration. Their roles and synthesized terms are stable:

```text
value:       @arg1
edge source: nthNextRef(@thisRef, @nodeHeap, @gHeap, @arg0)
```

The reconstructed method is derived from the common edit operation, rather than the old metadata recovery path:

```js
setAt(a, b) {
    const h_int_1 = this.setAt_h(a, b);
    const h_ptr_0 = this.setAt_p(a, b);
    h_ptr_0.f = h_int_1;
}
```

All MOLD helper tasks and all fixed-ID baseline tasks retained their previous success/failure classifications and successful synthesized terms.

The per-case fixed-ID files below use `maxCost=14` and are diagnostic artifacts, not the condition reported in the paper.  A second rerun of all 16 fixed-ID tasks at the paper's `maxCost=32` condition is saved at the rerun root as `baseline_all_results_cost32_timeout15s.json` and `baseline_all_runtime_cost32_timeout15s.csv`.  It reproduced the prior result exactly: 13 of 16 tasks succeeded, while `append-fixed-env-pair`, `insert-fixed-gHeap`, and `insert-fixed-env-pair` timed out in all three repetitions.  Every successful synthesized term was identical to the prior cost-32 record.

| case | common operations | MOLD helpers | MOLD result | fixed-ID baseline success pattern |
| --- | ---: | ---: | --- | --- |
| setAt | 1 | 2 | 2/2 success | fail, success, fail |
| append | 3 | 2 | 2/2 success | success, fail, fail |
| prepend | 4 | 2 | 2/2 success | success, success, fail, success |
| insert_general | 4 | 3 | 3/3 success | success, fail, fail |
| popBack | 1 | 1 | 1/1 success | success, fail, fail |

MOLD median runtimes:

| case | helper role / synthesized term | old ms | rerun ms |
| --- | --- | ---: | ---: |
| setAt | value / `@arg1` | 0.667 | 0.667 |
| setAt | source / `nthNextRef(..., arg0)` | 6.071 | 4.329 |
| append | value / `@arg0` | 0.674 | 0.669 |
| append | source / `last_ptr(...)` | 1.635 | 1.384 |
| prepend | value / `@arg0` | 0.536 | 0.526 |
| prepend | target / `@thisRef` | 0.359 | 0.297 |
| insert_general | value / `@arg1` | 0.583 | 0.684 |
| insert_general | source / `nthNextRef(..., arg0)` | 9.374 | 6.277 |
| insert_general | successor / `nthNextRef(..., inc(arg0))` | 17.142 | 6.368 |
| popBack | source / `penultimateRef(...)` | 1.498 | 1.420 |

Timing changes are ordinary rerun variation; no performance claim is made from them.

## Artifacts

Each case directory contains:

- `mold_payload.json`: byte-identical copy of the existing payload
- `response.json`: final `/synthesize` response
- `mold_escher_tasks.json`: PBE tasks emitted by the final response
- `mold_results_measured.json`, `mold_runtime.csv`: three-run MOLD helper measurements
- `baseline_fixed_id_task.json`, `baseline_components.json`: copies of the existing baseline inputs and conditions
- `baseline_fixed_id_results_measured.json`, `baseline_fixed_id_runtime.csv`: three-run baseline measurements
- `response_unscoped_regression.json`: retained result from the rejected unscoped implementation

The rerun root additionally contains the paper-comparable all-task cost-32 results and runtime CSV described above.

The original case directories and all of their records remain unchanged.

## Commands

The backend responses were obtained by running `cargo run` and posting each copied payload to `http://127.0.0.1:3030/synthesize`.

Measurements used:

```sh
node scripts/measure_escher_tasks.mjs \
  --file <task-file> \
  --out <result-file> \
  --csv <runtime-file> \
  --maxCost 14 \
  --timeoutMs 15000 \
  --repetitions 3
```

## Validation

- `cargo fmt --all -- --check`: passed
- `cargo test --all-targets --all-features`: passed (163 passed, 1 ignored)
- `cargo clippy --all-targets --all-features`: completed with pre-existing warnings; no new warning remains in the changed code
- `cargo clippy --all-targets --all-features -- -D warnings`: blocked by 26 existing library warnings and 11 additional existing test warnings
- API replay: all five saved payloads returned HTTP 200
- `git diff --check`: passed

The repository's `test_api.sh` was not used because it targets the removed `/analyze` endpoint and creates a temporary payload with shell redirection. The five direct `/synthesize` replays above cover the active endpoint and the actual evaluation payloads.

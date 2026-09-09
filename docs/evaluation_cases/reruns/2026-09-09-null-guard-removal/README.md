# 2026-09-09 null-guard removal rerun

## Purpose

This rerun fixes the implementation version and evaluation artifacts used after
removing code-generation-only null guards from recorded assignments. The change
does not add a branch to the synthesized update skeleton: every recorded field
assignment is emitted unconditionally.

## Provenance and conditions

- Date: 2026-09-09 (Asia/Tokyo)
- Implementation commit: `3b323c5ac64852b91c2f3baaaabdb4cd4733daaa`
- Platform: Darwin 25.6.0 arm64
- Rust: 1.93.0
- Node.js: 22.17.0
- MOLD helper search: AscendRec, `maxCost=14`, `timeoutMs=15000`,
  `searchSizeFactor=3`, simplification and goal search enabled
- Generated-code observation: value sequence reachable through `g` and absence
  of cycles

The five payloads are copied from the existing evaluation cases. Each case
directory contains the posted `payload.json` and the resulting `response.json`,
including the helper tasks, synthesized terms, and composed method code.

## Result

All ten helper tasks succeeded. The generated methods contain no code-generation
null guard. Running the generated code produced the following bounded results:

| Method | Passed |
| --- | ---: |
| `setAt` | 15/15 |
| `append` | 5/5 |
| `prepend` | 5/5 |
| `insert` | 15/15 |
| `popBack` | 5/5 |
| `setAt` controls | 2/2 |
| `insert` controls | 2/2 |

The exact inputs and observations are stored in
`../../generated_code_validation/results.json`. The controls include duplicate
and negative values and vary the assigned value independently from list length
and index.

## `swapAdjacent` diagnostic

The `swap_adjacent_value` directory contains three responses generated from the
same payload and implementation commit. All four helper tasks succeeded in every
run, but none of the three composed methods matched all three recorded
executions. The chosen helper expressions and assignment order varied across
runs. Therefore the stable finding is a correspondence and recomposition
failure for two same-kind updates, not one fixed sequence of erroneous helper
outputs. See `swap_adjacent_value/validation.json` for the exact observations.

## Non-separated comparison artifacts

The complete fixed-ID comparison tasks, component definitions, cost-32 results,
and witness checks are stored in
`../../nonseparated_general_components/`. In this cost model, each input,
constant, and component application costs one, and the cost of a compound term
is the sum over its syntax tree. The maximum hand-written witness cost is 32.

## Reproduction

Start the backend from the repository root:

```sh
cargo run
```

Post each `payload.json` to `http://127.0.0.1:3030/synthesize` and save the body
as the corresponding `response.json`. Then run:

```sh
node docs/evaluation_cases/generated_code_validation/validate.mjs \
  --out docs/evaluation_cases/generated_code_validation/results.json
node docs/evaluation_cases/reruns/2026-09-09-null-guard-removal/swap_adjacent_value/validate.mjs \
  --out docs/evaluation_cases/reruns/2026-09-09-null-guard-removal/swap_adjacent_value/validation.json
```

`SHA256SUMS` records the payload, response, script, and result hashes.

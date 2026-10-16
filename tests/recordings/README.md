# Recordings checked by `cargo test`

Each directory is one request recorded from Kanon and what `tests/recorded_stage_tests.rs`
expects of it; adding a directory adds a case without code. A directory that lacks a required
file fails the test instead of being skipped.

- `request.json`: the request Kanon sent (copied from `docs/evaluation_cases/reruns/`).
- `expected.json`: values the stages must give, by key: `read`, `graphs`, `template`,
  `problems` (results before synthesis), `validation` (whole-method validation) and
  `comparison` (agreement with the reference). A key that is left out is not checked. Every
  value given must appear at the same path; objects may have more keys, arrays must match in
  length. A stage that fails gives `{"error": ...}`; the later stages, synthesis included, are
  not run.
- `reference.mjs` (optional): the reference implementation and inputs for
  `scripts/compare_with_reference.mjs`; required when `comparison` is expected.

`insert` expects some of the paper's intermediate results (A..C are main-new1..3, D is
__temp1): the arguments, every write list, the vertices of the first and third graphs, the
correspondence, each hole's outputs and the outputs each PBE task is checked against. The
inputs of the PBE tasks are not given here; `paper_pipeline_tests` checks them on its fixture.
The other cases give only validation and comparison.

`rotateRight` passes validation and is expected to differ from a right rotation on 8 of 12
inputs, as found in `docs/evaluation_cases/reruns/2026-10-15-rotate-right-4demos-494d6d4`.
Its passing means that those known differences were found again, not that a right rotation
was synthesized.

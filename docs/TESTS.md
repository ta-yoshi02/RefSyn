# RefSyn Test Suite (Current)

This project now focuses on the Kanon → escher-ts task bridge and the supporting
graph tooling. The remaining tests exercise only those components. Run the full
suite with `cargo test`.

## Integration Layer
- `tests/integrated_synthesis_tests.rs` validates the HTTP entry point
  (`handle_synthesis`) against representative payloads. It checks empty input
  handling, single-trace synthesis, multi-trace analysis, task JSON structure,
  and selected end-to-end backend executions. Tests that execute the Node /
  `escher-ts` backend require an initialized `external/escher-ts` submodule and
  a built `external/escher-ts/dist/index.js`.
- `tests/synthesis_core_tests.rs` exercises `synthesize_core` directly. It
  keeps the browser-safe core honest by checking that single-trace responses
  stay aligned with the HTTP adapter and that multi-trace requests emit task/spec
  artifacts for browser or native postprocessing.
- `tests/synthesis_api_tests.rs` covers lower level helpers used by the bridge:
  list-environment construction from VisGraph snapshots, graph operation
  application, and the synchronous variant of the synthesis handler.

## Graph / Unification Tooling
- `tests/operation_analysis_tests.rs` exercises the diffing routines in
  `operation_analyzer`. It ensures two traces produce matching environments when
  aligned, and that mismatches surface clear diagnostics with captured
  environments.
- `tests/unification_conversion_tests.rs` checks the JSON → `unify_ops::Op`
  conversion pipeline and verifies that we can run unification on converted
  traces without panics or misclassified operations.
- `tests/isomorphism_tests.rs` and `tests/unify_ops_tests.rs` stress the graph
  unifier and the flexible isomorphism matcher against common list operations
  (append/prepend/remove/etc.). They provide confidence that the structural
  comparison logic still recognises equivalent traces after code changes.

## Utilities
- `tests/unify_ops_tests.rs` also doubles as a regression suite for the helper
  enums used throughout the codebase (`GraphOp`, `EdgeExpr`, `NodeExpr`,
  `VarOp`). Keeping these tests ensures the conversion logic in the bridge keeps
  emitting the shapes expected by downstream tooling.

These are the only Rust tests that remain after removing the legacy MemoEnv /
AST pipeline. If you introduce new bridge behaviours or graph utilities, add
tests alongside the relevant module (unit tests inside `src`, or integration
tests under `tests/`).***

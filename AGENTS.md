# Repository Guidelines

## Project Structure & Module Organization
- Core interpreter logic lives in `src/`; `lib.rs` exposes the library API while `main.rs` drives the CLI harness.
- Scenario fixtures under `examples/` and `records/` capture Kanon environments; reference them when adding translator cases.
- Integration tests sit in `tests/`; unit tests may live alongside modules in `src/`.
- Supporting docs, diagrams, and specs are in `docs/` and `architecture.md`; keep them synchronized with code changes.
- Submodules `Kanon/` and `Escher-Scala/` mirror upstream projects; avoid editing them unless coordinating cross-repo changes.

## Build, Test, and Development Commands
- `cargo build` compiles the interpreter and validates dependencies.
- `cargo run -- --help` prints CLI usage for local experiments.
- `cargo fmt` and `cargo clippy --all-targets --all-features` enforce formatting and linting; run both before submitting changes.
- `cargo test` executes Rust unit and integration tests; use `cargo test -- tests::` to scope to shared suites.
- `./test_api.sh` runs the end-to-end API contract checks between Kanon payloads and Escher-Scala expectations.

## Coding Style & Naming Conventions
- Follow Rust 2021 defaults: 4-space indentation, snake_case modules/functions, CamelCase types.
- Prefer expressive field names discovered at runtime; avoid hard-coding Kanon field labels.
- Keep serialization helpers pure and side-effect free; use comments only for non-obvious graph traversal rules.

## Testing Guidelines
- Add regression tests in `tests/` for cross-structure behaviors and in-module `#[cfg(test)]` blocks for focused units.
- Model new fixtures after existing payloads in `examples/`; ensure sentinel handling of null-like values is covered.
- Aim for meaningful edge coverage (empty graph, cycles, multi-pointer fields) before opening a PR.

## Commit & Pull Request Guidelines
- Match the existing history: short imperative messages such as `docs: refine analyzer docs` or concise descriptors when appropriate.
- Reference related issues in the body and describe API impacts or schema changes.
- Include reproduction steps or CLI output for behavioral changes, and attach updated JSON examples when modified inputs/outputs are introduced.

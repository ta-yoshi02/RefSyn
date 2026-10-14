# RefSyn Test Suite (Current)

Run the Rust suite with `cargo test`. Tests that run escher-ts need a built
`external/escher-ts` (`cd external/escher-ts && pnpm install --frozen-lockfile && pnpm build`).
Passing these tests shows agreement on the listed demonstrations and inputs only; it
is not evidence of correctness on inputs that were not tried.

## Web tests from a clean checkout

`web/` tests load vitest's config from `external/escher-ts/node_modules`, run escher-ts from
`external/escher-ts/dist`, and import the WASM package from `web/pkg`, which is generated
and not committed. From a fresh clone, in the repository root:

1. Check out the pinned submodules: `git submodule update --init --recursive`
2. Install and build escher-ts (pnpm at the version in its `packageManager` field):
   `cd external/escher-ts && pnpm install --frozen-lockfile && pnpm run build && cd ../..`
3. Add the Rust WASM target: `rustup target add wasm32-unknown-unknown`
4. Install wasm-bindgen-cli at the same version as `wasm-bindgen` in `Cargo.lock`
   (currently 0.2.114): `cargo install wasm-bindgen-cli --version 0.2.114 --locked`
5. Install the web dependencies: `npm ci --prefix web`
6. Build the WASM package and run the tests: `npm run test:rebuild --prefix web`

`npm test --prefix web` runs the tests against whatever is already in `web/pkg`; it does
not rebuild it. Use `npm run test:rebuild --prefix web` for experiments and evaluation, so
the WASM matches the checked-out Rust sources.

Node.js: CI runs Node 22. The lowest version the web dependencies declare support for is
20.19 (vitest's bundled vite requires `^20.19.0 || >=22.12.0`); it is not run in CI.
`.github/workflows/ci.yml` runs the same steps.

## Paper pipeline (`tests/paper_pipeline_tests.rs`)

Uses the paper's three `insert` demonstrations as the fixture.

- Reading keeps null, undefined and absent fields apart, and rejects unknown references,
  missing typed state and old snapshot versions.
- Edits become writes in order with duplicates; `deleteEdge` writes one shared null vertex;
  stale old targets, value-field deletes, `addEdge` over an edge, node deletion and other
  variables are rejected; creating a class whose `new C()` could not be captured fails.
- The correspondence maps the demonstrations as in the paper, ignores edge order, and is
  rejected when kind, class, field, direction or multiplicity differ, or null meets non-null.
- The template has one creation, three hole bindings and three writes in the paper's order,
  the paper's hole outputs, and renders the seven-statement method.
- Hole inputs contain created objects with their constructor state and earlier holes, but no
  writes, no own result and no later holes.
- The native HTTP path synthesizes, validates and adopts `insert`; empty requests, mixed
  methods and non-isomorphic demonstrations produce no code; the core hands the browser one
  task and one hole check per hole.

## Fields and names (`tests/fields_and_names_tests.rs`)

- A class without a constructor: updating existing objects needs no absent fields; a created
  object's absent pointer field is encoded as the error value (slot and heap), not null; a
  method that creates such an object and later writes a reference to it is synthesized,
  validated and adopted.
- A non-ASCII method name (`追加`) calls exactly the helpers it defines, and `hole_checks`
  use the same name.
- A field that is not an identifier (`data-value`) is written and read with bracket notation.
- Field heaps that collide with other inputs (`node` → `nodeHeap`) are refused; fields whose
  accessors share a name (`next`, `Next` → `hasNext`) produce no code.
- escher-ts' `tests/unit/components/user-friendly-json.test.ts` checks that undefined
  reference and integer slots read as the error value in `fOf` and `hasF`.

## Whole-method adoption

- `tests/whole_method_validation_tests.rs` checks native adoption and rejection through HTTP,
  including the five saved evaluation cases with typed pre-states built from their recorded
  `precondGraph`, and runs the adopted code on inputs that were never demonstrated.
- `node --test runtime/whole-method-validation.test.mjs` checks the demonstrated heap
  (values, presence, identity, allocations by structure), return values, helper results
  against their output examples, side effects on globals, built-in and class prototypes and
  statics, rejection of incorrect candidates, unavailable inputs, repeated-write replay and timeout.
- `npm test --prefix web` checks browser adoption, translation failure, Worker startup and
  run timeouts, and reply nonces, and validates a response typed as `SynthesisResponse` with
  `hole_checks` (adopted when the helper reproduces its output example, rejected otherwise).
- Kanon's `src/js/test/testize-whole-method-validation.test.js` checks typed capture including
  `new C()` states, the MOLD state reflection (null writes keep properties and aliases, created
  objects run their constructor, unsupported edits, uncaptured or throwing constructors leave the
  heap untouched and mark the demonstration as unsynthesizable), payload delivery
  to the actual validator, editor mutation guards, application of exactly the validated change
  and re-execution of the applied class. Its jest setup requires `escodegen`, which Kanon ships
  as the `external/escodegen` submodule rather than an npm dependency.

## Bridge helpers

- `src/escher_bridge.rs` unit tests cover the escher-ts process memory setting.

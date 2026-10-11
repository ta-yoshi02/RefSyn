# RefSyn Test Suite (Current)

Run the Rust suite with `cargo test`. Tests that run escher-ts need a built
`external/escher-ts` (`cd external/escher-ts && pnpm install --frozen-lockfile && pnpm build`).
Passing these tests shows agreement on the listed demonstrations and inputs only; it
is not evidence of correctness on inputs that were not tried.

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

## Whole-method adoption

- `tests/whole_method_validation_tests.rs` checks native adoption and rejection through HTTP,
  including the five saved evaluation cases with typed pre-states built from their recorded
  `precondGraph`, and runs the adopted code on inputs that were never demonstrated.
- `node --test runtime/whole-method-validation.test.mjs` checks the demonstrated heap
  (values, presence, identity, allocations by structure), return values, helper results
  against their output examples, side effects on globals, built-in and class prototypes and
  statics, rejection of incorrect candidates, unavailable inputs, repeated-write replay and timeout.
- `npm test --prefix web` checks browser adoption, translation failure, Worker startup and
  run timeouts, and reply nonces.
- Kanon's `src/js/test/testize-whole-method-validation.test.js` checks typed capture including
  `new C()` states, the MOLD state reflection (null writes keep properties and aliases, created
  objects run their constructor, unsupported edits leave the heap untouched), payload delivery
  to the actual validator, editor mutation guards, application of exactly the validated change
  and re-execution of the applied class. Its jest setup requires `escodegen`, which Kanon ships
  as the `external/escodegen` submodule rather than an npm dependency.

## Bridge helpers

- `src/escher_bridge.rs` unit tests cover the escher-ts process memory setting.

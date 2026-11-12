# RefSyn Test Suite (Current)

This is an index of existing checks, not a specification of the paper or a fresh test report.
See [the worklist](PAPER_ALIGNMENT_WORKLIST.md) for the remaining conformance checks.

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
- `addEdge` also replaces a `new C()` default that a created object's field still holds
  before this demonstration writes it (Kanon shows created nodes without fields); a second
  `addEdge` while a written value remains, and `addEdge` on a pre-call object's value, are
  still rejected, and creation and write history do not carry over between demonstrations.
  The default stays in the hole inputs. The PBE encoding derives field kinds from the
  pre-call objects and the `new C()` state, not from written values, so a default it cannot
  represent (`""`) or one whose kind differs from the same field in the pre-call objects
  (`false` or `0` where they hold numbers or references) is accepted as an edit but rejected
  there.
- The correspondence maps the demonstrations as in the paper, ignores edge order, and is
  rejected when kind, class, field, direction or multiplicity differ, or null meets non-null.
- Among several isomorphisms, the first combination whose hole examples never give one input
  two outputs is taken: reversing the edit order of two symmetric writes no longer swaps their
  sources, mu_1 stays the identity, eight symmetric writers (8! isomorphisms) resolve at the
  first one, and exhausting the 10,000-step search is reported apart from a contradiction in
  every combination. Combinations that take each field's last write in the first graph to
  the last write to the corresponding field are tried first, since only they let the template,
  which keeps the first demonstration's write order, end every field as each demonstration
  does. The search goes on past consistent combinations that do not, and if it ends or runs
  out of steps without one that does, takes the first consistent one it met, so the preference
  never loses a correspondence the plain search found within the limit.
- The template has one creation, three hole bindings and three writes in the paper's order,
  the paper's hole outputs, and renders the seven-statement method.
- Hole inputs contain created objects with their constructor state and earlier holes, but no
  writes, no own result and no later holes.
- The native HTTP path synthesizes, validates and adopts `insert`; empty requests, mixed
  methods and non-isomorphic demonstrations produce no code; the core hands the browser one
  task and one hole check per hole.

## Operation order (`tests/operation_order_tests.rs`)

Demonstrations that make the same edits in another order, written as small `Cell` heaps. The
expected correspondence, hole types and outputs, PBE inputs (receiver, pre-call heap, arguments,
created objects, earlier holes, output) and the state after running the template are derived
from the edits and compared by the objects and values each vertex stands for, not by vertex or
hole numbers.

- Independent writes: two symmetric writes reversed, and every pair of orders of three writes
  to different fields (36 pairs).
- Creation order: `pushTwo(x, y)` with both creation orders times all 120 write orders.
- Order that matters: repeated writes to one field keep order and multiplicity; `set(a, b)` is
  synthesized and ends with `b` on new arguments; two demonstrations that give one input two
  results are aligned and every hole is synthesized, then the whole-method validation
  reproduces the first and fails on the second at `A.value` (checked exactly, so a refusal for
  another reason does not pass).
- Invalid reorderings (use before creation, stale old targets) are rejected by graph building.
- With different states, reversing two symmetric writes changes which cells the search pairs
  (the receiver is not part of the correspondence). Either pairing must reproduce both
  demonstrations. The methods adopted for the two orders are compared with each other (not
  with a reference) on 30 given lists where the receiver has a successor, two of them the
  demonstrated states: both finish and leave the same `value`, `next` and `other` in every
  existing cell. Return values are not compared, and receivers without a successor are left
  out of this check, which does not make them out of scope. This is recorded as
  order-dependent selection, not as a defect (see [the module map](MODULE_TESTS.md)).
- Two isomorphisms that differ only in which write to `next` comes last: the one that leaves
  the template's last write where the second demonstration leaves it is taken, although the
  search meets the other first in declaration order. Synthesis is not run here: the written
  cells are unreachable from the pre-call state, so no hole beyond the receiver can be found.
- Equal values from distinct literal nodes (p, q, p against p, p, q, both 1) alongside eight
  symmetric writers: no isomorphism keeps the last writes, yet the first consistent one is
  still taken although searching for a better one uses up the step limit.
- `run_template` runs the template in the test with each hole bound to its expected output; only
  `set` and the `point` comparison execute synthesized JavaScript.

## Edit kinds (`tests/edit_kind_alignment_tests.rs`)

Pairs of demonstrations whose edits are of different Kanon kinds (`addEdge`,
`editEdgeReference`, `deleteEdge`, `return`) but mean the same update, on small `Cell` heaps.
Each case states the pre-call states and edits, the expected writes of both graphs, every
allowed correspondence (by the objects and values the vertices stand for), the hole types and
each demonstration's PBE input before the earlier holes.

- An oracle tries every bijection between the vertex sets and keeps those that preserve vertex
  kind, class, constant type, field, direction, multiplicity and the return value. It uses
  neither `alignment`'s search nor `Vertex::corresponds`. Its set must equal the expected set;
  `align`'s first isomorphism and `align_consistently`'s choice must be in it.
- The template under the chosen correspondence must reproduce both demonstrations as replayed
  directly from their edits (hole outputs bound, not synthesized).
- Accepted: `addEdge` onto `editEdgeReference` (one write); `deleteEdge` onto `deleteEdge`
  through the shared null; two independent writes to different fields; two writes to one field;
  two sources sharing a target; a two-cell cycle with `return`; two created cells; a value write
  with two reference writes; a five-write doubly linked insertion.
- Two writes to one field admit two isomorphisms, and only the one that keeps the last write last
  reproduces the second demonstration. Here the search meets it first in declaration order;
  with that order reversed (the operation-order case above) it is still taken, because
  combinations that keep each field's last write are tried first. The
  shared-target case admits two pairings and both reproduce.
- Rejected: a stale old target (while normalizing); deletion against replacement; created against
  existing target; another field; another shape with the same counts; return in only one
  demonstration; union by rank with one and two writes (a limit of the one-to-one specification,
  not a supported case).

## Randomized operation normalization (`tests/operation_graph_randomized_tests.rs`)

- Seeds 1–64 generate 48 edits each, including object/literal creation, repeated writes,
  reference deletion and return updates, starting with cycles, sharing, detached objects,
  null, undefined and absent fields. The operation kind follows a fixed schedule; targets,
  fields and values vary. A direct state model is compared with graph replay after every
  prefix (3,072 comparisons), plus 64 empty prefixes. This checks reading and normalization,
  without invoking PBE or executing generated JavaScript.
- Corrupting a recorded old target in each sequence must be rejected (64 cases).
- See [the Japanese verification guide](RANDOMIZED_TESTING.md) for the independent oracles,
  sequence comparisons, deliberate faults, reproducible commands and scope limits.

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

- `tests/whole_method_validation_tests.rs` checks native adoption and rejection through the handler,
  including the five saved evaluation cases with typed pre-states built from their recorded
  `precondGraph`, and runs the adopted code on inputs that were never demonstrated. Each of the
  five `adopted_*_matches_the_reference_on_unseen_inputs` tests synthesizes its method afresh,
  then `tests/js/list-method-differential.cjs` compares it with the independent method in that
  case's `target_program.js`, using identical initial states and arguments:

  | Method | Compared calls |
  | --- | ---: |
  | `setAt` | 5,160 |
  | `insert` | 5,160 |
  | `append` | 531 |
  | `prepend` | 531 |
  | `popBack` | 72 |

  Receiver-list lengths are 1, 2, 3, 5, 8, 16 and 32 (`popBack`: 2 and above). Values are distinct
  positive integers, zeros, repeated mixed-sign integers, or pseudorandom integers with a fixed
  seed. Assigned/inserted values vary independently over -10001, -1, 0, 1, 10007 and the list's
  endpoint values, deduplicated. Every valid index is checked for `setAt` and `insert`; `insert`
  means insertion **after** that index, including the tail. Empty receivers, invalid indices
  and singleton `popBack` are outside this suite's domain.

  Each input is tested at the head, with a shared suffix and detached objects, and with a receiver
  inside a larger list. The shared layout also reverses the allocation order of the list nodes.
  The comparison retains IDs for all existing objects (including removed/detached nodes), tracks
  constructor allocations including unreachable ones, and compares every own field, property
  presence/attributes, references and aliases, class, and return value/identity. It does not
  monitor ambient globals/prototypes or exercise Kanon's capture/editor UI. Existing small
  follow-up checks, including sequential calls, are retained. These finite input checks are not
  a proof on all lists or all integers, and do not reuse the historical 49-input saved-code run.

  Run just these five tests with:

  ```sh
  cargo test --locked --test whole_method_validation_tests matches_the_reference_on_unseen_inputs
  ```

  Add `-- --nocapture` to see the per-method comparison counts alongside the synthesis diagnostics.
- `adopted_methods_match_the_reference_after_each_seeded_random_call` freshly synthesizes
  the five methods and mixes them in 288 persistent-state sequences of 50 calls (14,400
  calls). Every call is compared, using seeds 1–24, initial lengths 1, 2, 8 and 32, and the
  three layouts above. Receivers and valid arguments change with the current reference
  state. Returned heads from `prepend` at offset zero become the next root receiver;
  original aliases and all allocated objects remain observable.
- `reference_comparator_accepts_correct_methods_and_rejects_faulty_candidates` runs
  `node --test tests/js/list-method-differential.test.cjs` as part of `cargo test`.
  Its 15 Node tests include correct controls and nine deliberately faulty candidates,
  including a candidate that only corrupts the second `setAt` call. Failures must be
  comparison assertions, not syntax errors or process startup failures.
- `node --test runtime/whole-method-validation.test.mjs` checks the demonstrated heap
  (values, presence, identity, allocations by structure), return values, helper results
  against their output examples, side effects on globals, built-in and class prototypes and
  statics, rejection of incorrect candidates, unavailable inputs, repeated-write replay and timeout,
  and the same `addEdge` rule for a created object's unwritten `new C()` defaults (replaced
  value, untouched default, identity and return value of the created object).
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

## Recorded requests by stage (`tests/recorded_stage_tests.rs`)

- Reads every directory of `tests/recordings/` (a request recorded from Kanon, the values
  expected of `read`, `graphs`, `template`, `problems`, `validation` and `comparison`, and an
  optional reference); a new directory is a new case. See
  [the module map](MODULE_TESTS.md) and `tests/recordings/README.md`.
- On Kanon's recording of the paper's insert, the stages give the paper's arguments, write
  lists, vertices of two graphs, correspondence, hole outputs and PBE expected outputs; PBE
  inputs are left to `paper_pipeline_tests`. Unmet values are reported with their paths; a
  rejected recording stops at that stage without synthesis, and a directory missing a required
  file fails the test.
- The 2026-10-14 recordings of insert, setAt, append, prepend and popBack are adopted and
  agree with their references on 15, 15, 15, 15 and 12 inputs. The four-demonstration
  rotateRight recording passes validation and differs from a right rotation on 8 of 12
  inputs, the known result of that rerun; passing means those differences were found again.
- `tests/js/compare-with-reference.test.mjs` checks that the comparison accepts an equivalent
  method that creates objects in another order, rejects seven faulty ones, stops a call that
  does not finish (2 s) as a mismatch and treats a reference that does not finish as an error.
  The whole comparison process is stopped after 120 s; that path is tested with a 1 s limit.
  Only states Kanon's `captureValidationState` can record are compared (instances of the
  classes with ordinary string-keyed data fields holding undefined, null, strings, booleans,
  finite numbers other than -0 or such instances), and objects must stay extensible. The
  conditions apply to the input and to every object observed after the call, detached ones
  included; a state outside them is an error or a mismatch, never agreement.

## Failures and transports

Each check names the path it actually goes through; a check on one path says nothing about the others.

| Path | Check | What fails and what is asserted |
| --- | --- | --- |
| escher-ts runner process | `src/escher_bridge.rs` unit tests | Missing runner, non-zero exit, empty output, invalid JSON, wrong JSON shape, and a runner that neither reads its input nor exits (killed at a 500 ms test deadline). Each gives its own error. Also the memory setting |
| Child process deadline | `src/child_process.rs` unit tests | A program that cannot start, and stdin/stdout/stderr/exit status of one that does. The validator's 10 s limit uses this same function; it is not run at 10 s in a test |
| Handler, runner failing | `tests/runner_failure_tests.rs` | With `ESCHER_JS_RUNNER` replaced by each failing runner, the reply has `validation.status = "unavailable"` with `escher-ts runner failed: …` as its error (not a validation failure), no `composed_method_code`, no `code`, and returns within 15 s |
| Real HTTP port | `tests/http_server_tests.rs` | `server::routes()` on an ephemeral port: an adopted method with CORS for another origin, the preflight Kanon's POST needs, 400 with an empty reply for an empty, non-JSON, `{}` or mistyped body, 404/405 for other routes, and four concurrent requests each getting its own answer |
| Browser Worker client | Kanon `src/js/test/refsyn-runtime-client.test.js` | With a stand-in Worker: replies resolve only their own request; `error` (a script that cannot load), `messageerror` and the 60 s request limit fail every waiting request, terminate the Worker and let the next request start a new one |

Not covered by automated checks: a real browser. The runtime client was checked by hand in
Chromium on a locally served Pages build (a normal insert request, and the worker chunk removed);
see ta-yoshi02/Kanon#10 for the record. The Kanon editor flow from recording demonstrations to adoption runs
only with stand-ins (`testize-whole-method-validation.test.js`).

The native escher-ts runner is bounded from Rust by `ESCHER_JS_PROCESS_TIMEOUT_MS` (60 s by default),
separately from escher-ts's per-task search limit `ESCHER_TS_TIMEOUT_MS` (2 s by default). The browser
has no separate process: the Worker is bounded from the page by the 60 s request limit above.

## Translation and CI

- `web/src/lib/binding-conditions.test.ts` (#29) checks C1 and C5 on WASM-built
  paper_insert and boolean tasks. It evaluates each returned expression with
  escher-ts, keeping Int/Bool/Ref apart, and catches writes to reachable inputs
  with Proxies. Component cases compare translated outcomes with escher-ts on
  list inputs and check the expected outcomes on rings. Expression readback
  depends on `showTerm` formatting; these finite checks do not prove purity on
  unobserved inputs. Saved mutation results are indexed in
  [the PR #29 record](evaluation_cases/reruns/2026-10-14-pr29-checks/README.md).
- `node --test runtime/refsyn-escher-adapter*.test.mjs` checks operator precedence,
  the heap selected by `loadInt`, parameter names, and the translation contract: wherever
  escher-ts gives a term a value on an input (heaps with null, undefined, absent fields and
  cycles included), the translated JavaScript terminates with the same value. Term and
  input pairs where escher-ts gives the error value are not run; the JS outcome there is
  unspecified. Boolean field reads are covered since #28.
- Boolean fields, arguments and holes (since #28): `tests/boolean_values_tests.rs` checks the
  encoding and native adoption with independent execution, `web/src/lib/booleans.test.ts`
  checks the same fixtures (`tests/data/booleans/`) through WASM, and
  `runtime/whole-method-validation.test.mjs` checks that values sharing a label but not a type
  (`true`, `"true"`, `1`) are rejected.
- `.github/workflows/ci.yml` builds the pinned escher-ts before Rust and runtime checks,
  then builds WASM, runs web checks and checks Kanon's fixture, capture and reflection.
  Its existence does not establish that the latest run passed.

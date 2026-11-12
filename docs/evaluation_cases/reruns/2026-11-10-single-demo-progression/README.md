# Additional example-count and counterexample audit

Date: 2026-11-10 JST. Production code, grammar and search settings are unchanged. The previous audit source fingerprints were checked against current files and matched.

## 1. All missing subsets of existing recordings

All 19 previously untested subsets were tested. Together with the previous audit, all nonempty subsets of each recording have now been tested (42 conditions). Demonstration order remains the recording order; permutations were not tested.

| Method | Demonstrations (1-based) | Whole-method validation | Reference agreement |
|---|---|---|---|
| insert | 1,2 | passed (2) | 6/15 |
| insert | 1,3 | passed (2) | 10/15 |
| insert | 2,3 | passed (2) | 5/15 |
| setAt | 1,2 | passed (2) | 11/15 |
| setAt | 1,3 | passed (2) | 10/15 |
| setAt | 2,3 | passed (2) | 3/15 |
| append | 1,2 | passed (2) | 15/15 |
| append | 1,3 | passed (2) | 15/15 |
| append | 2,3 | passed (2) | 15/15 |
| rotateRight | 1,2 | passed (2) | 6/12 |
| rotateRight | 1,3 | passed (2) | 2/12 |
| rotateRight | 1,4 | passed (2) | 4/12 |
| rotateRight | 2,3 | passed (2) | 0/12 |
| rotateRight | 2,4 | passed (2) | 2/12 |
| rotateRight | 3,4 | passed (2) | 2/12 |
| rotateRight | 1,2,3 | passed (3) | 6/12 |
| rotateRight | 1,2,4 | passed (3) | 4/12 |
| rotateRight | 1,3,4 | passed (3) | 4/12 |
| rotateRight | 2,3,4 | passed (3) | 2/12 |

The comparison counts are read from JSON mismatches, not process exit status. The comparator can exit zero on mismatches.

## 2. Counterexample-selected additional edit specifications

These are automatically constructed operation sequences (`make_demo.mjs`) on inputs from the independent reference modules; they are NOT new manual Kanon UI recordings. Each includes typed pre-state, explicit recorded-format edits, and method arguments. Original saved constructor and method metadata is retained.

Start from the first original demonstration, synthesize, find the first remaining mismatching reference case, construct its edits, and try adding it. Retain an addition only if synthesis and whole-method validation succeed. Rejected additions are preserved and the next mismatching case is tried. Stop when the fixed selection set has zero mismatches or no untried counterexamples remain. rotateRight is also tested starting from all four original demonstrations.

| Method | Starting condition | Accepted demonstrations at end | Selection-set agreement |
|---|---|---:|---:|
| insert | first original | 4 | 15/15 |
| setAt | first original | 3 | 15/15 |
| append | first original | 1 | 15/15 |
| prepend | first original | 1 | 15/15 |
| popBack | first original | 1 | 12/12 |
| rotateRight | first original | 4 | 12/12 |
| rotateRight | all four originals | 6 | 12/12 |

### insert, start=0

- refine-insert-0-1: try 2 demonstrations; input `head, length 2, args [0,61]`; validation `passed`; agreement 9/15. See request.json, added-demo.json, response.json and run.log in that directory.
- refine-insert-0-2: try 3 demonstrations; input `interior, length 3, args [2,9]`; validation `unavailable`; agreement 0/0. See request.json, added-demo.json, response.json and run.log in that directory.
- refine-insert-0-3: try 3 demonstrations; input `shared, length 3, args [1,-4]`; validation `passed`; agreement 11/15. See request.json, added-demo.json, response.json and run.log in that directory.
- refine-insert-0-4: try 4 demonstrations; input `shared, length 5, args [2,-43]`; validation `passed`; agreement 15/15. See request.json, added-demo.json, response.json and run.log in that directory.

### setAt, start=0

- refine-setAt-0-1: try 2 demonstrations; input `interior, length 3, args [2,-49]`; validation `passed`; agreement 10/15. See request.json, added-demo.json, response.json and run.log in that directory.
- refine-setAt-0-2: try 3 demonstrations; input `shared, length 3, args [1,-56]`; validation `passed`; agreement 15/15. See request.json, added-demo.json, response.json and run.log in that directory.

### rotateRight, start=0

- refine-rotateRight-0-1: try 2 demonstrations; input `root 50(30(20, 40(-, 45)), 70)`; validation `passed`; agreement 6/12. See request.json, added-demo.json, response.json and run.log in that directory.
- refine-rotateRight-0-2: try 3 demonstrations; input `root 50(30(-, 40(-, 45)), 70)`; validation `passed`; agreement 8/12. See request.json, added-demo.json, response.json and run.log in that directory.
- refine-rotateRight-0-3: try 4 demonstrations; input `root 50(30(20(10, -), 40), 70)`; validation `passed`; agreement 12/12. See request.json, added-demo.json, response.json and run.log in that directory.

### rotateRight, start=all

- refine-rotateRight-all-1: try 5 demonstrations; input `root 50(30(20, 40(-, 45)), 70)`; validation `passed`; agreement 8/12. See request.json, added-demo.json, response.json and run.log in that directory.
- refine-rotateRight-all-2: try 6 demonstrations; input `root 50(30(20(10, -), 40), 70)`; validation `passed`; agreement 12/12. See request.json, added-demo.json, response.json and run.log in that directory.

One insert addition is rejected: the tail-insertion input `interior, length 3, args [2,9]`. Its generated trace leaves the new node next field at its constructor default null, so it has 3 vertices / 2 writes rather than the first trace 4 vertices / 3 writes. The pipeline reports non-isomorphic operation graphs. This attempt is not counted as an accepted demonstration; another counterexample is selected. This is evidence about that exact trace, not a claim that all possible recordings of tail insertion fail.

These additional examples are selected using observed mismatches, so success on the selection set is not independent generalization evidence. They are separate conditions from the original recordings.

## 3. Separate finite checks after refinement

No input from this group was used to select the extra demonstrations. List lengths are 1,2,3,4,6,9,16 (popBack excludes length 1); four value patterns, all valid indices for insert/setAt, plus parent, middle, tail and detached aliases. Trees are 160 fixed-seed shapes with non-null left child, with a null middle subtree in 32 cases; parent/root/pivot/middle aliases are observed. The null-middle cases extend the original rotation selection-set domain. Checks compare identity, reachable state, properties, return and exceptions through the existing reference comparator.

| Method | Start | Final demonstrations | Held-out agreement |
|---|---|---:|---:|
| insert | 0 | 4 | 164/164 |
| setAt | 0 | 3 | 164/164 |
| append | 0 | 1 | 28/28 |
| prepend | 0 | 1 | 28/28 |
| popBack | 0 | 1 | 24/24 |
| rotateRight | 0 | 4 | 160/160 |
| rotateRight | all | 6 | 160/160 |

This is bounded validation, not a proof of equivalence for all inputs. The two rotateRight final conditions use the same 160 check inputs, not 320 distinct inputs.

## Reproduction

From the repository root, first build `cargo build --locked --example run_payload`. Then run the following files with Python:

- `run.py`: missing subsets.
- `refine.py`: counterexample-selected additions, starting from the previous saved audit outputs.
- `holdout.py`: separate final checks. Exits nonzero if any final candidate mismatches.

Inputs, responses, operation sequences and mismatch details are preserved per condition. The scripts do not modify production code or the original recordings. A standalone whole-method validation accompanies each synthesized candidate; no UI reflection is claimed for this run.

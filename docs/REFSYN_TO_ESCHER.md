# Kanon → Escher-Scala JSON Bridge (Unification → Diff → List/Int JSON)

This document explains how to go from two Kanon operation traces to Escher-Scala `tests.json`:

1) run unification to find common/diff parts
2) build a local List/Int environment snapshot at the boundary
3) emit Escher-compatible JSON examples

You will then point Escher-Scala to the generated file and run it manually.

## What’s Implemented

- Module: `src/escher_bridge.rs`
  - `EscherCase`: holds a `ListEnvironment` + arguments + output
  - `build_escher_spec(name, return_type, cases) -> String`:
    - dynamically detects fields (value vs pointer)
    - BFS local indexing from detected root variable
    - sentinel `-1` for null/undefined/end
    - builds a single Escher spec with many `examples`
  - `write_spec_to_file(path, content)`

- Environment builder: `src/list_env.rs`
  - `ListEnvironment::from_vis_graph(&VisGraph)` builds initial lists
  - `ListEnvironment::apply_operation(&GraphOperation)` applies a Kanon edit
  - Stores pointer fields as indices and values as integers (numeric strings also supported by the bridge)

- Unification utilities: `src/lib.rs`
  - `pub fn analyze_operations_with_unification(vis_graph, operations_a, operations_b)`
    - returns `UnificationAnalysisResult` containing `unification_result` with `common_a`, `diff_a`, etc.
  - Note: You’ll map `common_a` back to the original JSON operations by the index in the op id (e.g. `op_3` → index 3)

## End-to-End Flow

Inputs per test case:
- `VisGraph` (Kanon environment: nodes/edges)
- Two operation sequences (JSON array of operations) to compare: A and B
- Function arguments (scalars, e.g., `Int`)
- Expected output (e.g., `Int`)

Steps:

1. Unify operations to find common/diff
   - Convert A and B to unify ops and compute unification.
   - Use: `refsyn::analyze_operations_with_unification(&vis_graph, &ops_a, &ops_b)`

2. Build environment at the boundary
   - Start from `ListEnvironment::from_vis_graph(&vis_graph)`.
   - Extract indices from `analysis.unification_result.common_a` (ids like `op_0`, `op_1`, …), sort ascending.
   - For each index i:
     - Parse the original JSON A[i] into `list_env::GraphOperation` (serde)
     - Apply via `env.apply_operation(&graph_op)`
   - The resulting `env` is the snapshot just before first difference.

3. Create Escher case(s)
   - Wrap `env`, `vis_graph`, `arguments`, `output` into `EscherCase`.
   - You can create multiple cases (one per Kanon test) and pass all to `build_escher_spec`.

4. Emit tests.json
   - Call `build_escher_spec("<func-name>", "<return-type>", &cases)` → `String`
   - Write to a path you choose (e.g., `Escher-Scala/src/main/resources/escher/tests.json`):
     - `write_spec_to_file(path, &spec)`
   - You will then adjust Escher-Scala’s loader and run it.

## Example Code (Rust)

Assume you already have for each test:
- `vis_graph: refsyn::models::VisGraph`
- `ops_a: Vec<serde_json::Value>`
- `ops_b: Vec<serde_json::Value>`
- `arguments: Vec<serde_json::Value>` (e.g., `[json!(0)]`)
- `expected_output: serde_json::Value` (e.g., `json!(2)`) 

```rust
use refsyn::escher_bridge::{EscherCase, build_escher_spec, write_spec_to_file};
use refsyn::list_env::{ListEnvironment, GraphOperation};
use serde_json::json;

// 1) Unification
let analysis = refsyn::analyze_operations_with_unification(&vis_graph, &ops_a, &ops_b)?;

// 2) Build env at the common boundary (apply common A-ops in original order)
let mut env = ListEnvironment::from_vis_graph(&vis_graph);
let mut common_indices: Vec<usize> = analysis
    .unification_result
    .common_a
    .iter()
    .filter_map(|op| op.id.strip_prefix("op_")?.parse::<usize>().ok())
    .collect();
common_indices.sort_unstable();

for idx in common_indices {
    let op_json = ops_a[idx].clone();
    let graph_op: GraphOperation = serde_json::from_value(op_json)?;
    env.apply_operation(&graph_op)?;
}

// 3) Wrap into an Escher case
let case = EscherCase {
    env,
    vis_graph: vis_graph.clone(),
    arguments: vec![json!(0)],          // your scalar args
    output: json!(2),                   // your expected output
};

// You can repeat the above to build multiple cases.
let spec = build_escher_spec("append-g", "Int", &[case])?;

// 4) Write to a file Escher will load
write_spec_to_file("Escher-Scala/src/main/resources/escher/tests.json", &spec)?;
```

## Field Detection and Indexing Details

- Dynamic fields:
  - Value fields: any edge label with at least one edge to a literal node
  - Pointer fields: any edge label with at least one edge to a non-literal object node
  - Special nodes (`__RectForVariable__`, `__Variable-*`) are ignored for classification

- Root detection for BFS:
  - Bridge searches variable nodes with id `__Variable-<name>`
  - If field `<name>` at that variable’s index points to an object, that object becomes the BFS root
  - BFS follows only pointer fields; this yields the local index order

- Normalization:
  - Scalar values: parsed as `Int` (numeric JSON or numeric string)
  - Missing/undefined/null: `-1`
  - Pointers: original indices remapped into BFS-local indices, terminals become `-1`

## Multiple Test Cases

Pass several `EscherCase`s to `build_escher_spec`. The bridge:
- derives `inputTypes` from the first case
- checks that detected field sets are consistent across all cases
- emits one function spec containing all cases under `examples`

## Tips and Caveats

- If your Kanon literals are strings like `"2"`, the bridge will parse them as numbers for value lists.
- Ensure the root variable node (e.g., `__Variable-lst` → label `lst`) exists; otherwise root detection fails.
- If you prefer a position-based fallback without unification, you can use:
  - `operation_analyzer::analyze_operations_with_environments`, which returns snapshots (`environment_before`) at each difference. You can convert those directly into `EscherCase`s.
- The internal helper in `lib.rs` used for printing env during unification uses a string form for some edges; prefer the `ListEnvironment` API shown above for the bridge.

## Where Things Live

- `src/escher_bridge.rs`: bridge implementation (build spec JSON)
- `src/list_env.rs`: list-based environment representation + operations
- `src/lib.rs`: public unify entry (`analyze_operations_with_unification`)
- `Escher-Scala/src/main/scala/escher/Test.scala`: Escher runner (loads `/escher/tests.json`)


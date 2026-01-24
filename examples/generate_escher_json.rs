//! Generate Escher-Scala tests.json from a minimal demo
//! Usage: cargo run --example generate_escher_json

use refsyn::escher_bridge::{build_escher_spec, specs_to_json, write_spec_to_file, EscherCase};
use refsyn::list_env::{GraphOperation, ListEnvironment};
use refsyn::models::{Edge, Node, VisGraph};
use serde_json::json;

fn main() -> anyhow::Result<()> {
    // 1) Minimal VisGraph with a variable root: __Variable-lst -> main-new1
    let vis_graph = VisGraph {
        nodes: vec![
            Node {
                id: "main-new1".to_string(),
                is_literal: false,
                label: json!("Node"),
            },
            Node {
                id: "main-new1-val".to_string(),
                is_literal: true,
                label: json!("2"),
            },
            Node {
                id: "__Variable-lst".to_string(),
                is_literal: false,
                label: json!("lst"),
            },
        ],
        edges: vec![
            Edge {
                from: "main-new1".to_string(),
                to: "main-new1-val".to_string(),
                label: "val".to_string(),
            },
            Edge {
                from: "__Variable-lst".to_string(),
                to: "main-new1".to_string(),
                label: "lst".to_string(),
            },
        ],
    };

    // 2) Two operation sequences; unify will find a common addNode
    let ops_a = vec![
        json!({"editType": "addNode", "id": "__temp1", "label": "Node", "isLiteral": false}),
        json!({"editType": "addNode", "id": "__temp2", "label": "0", "isLiteral": true}),
        json!({"editType": "addEdge", "from": "__temp1", "to": "__temp2", "label": "val"}),
        json!({"editType": "addEdge", "from": "main-new1", "to": "__temp1", "label": "next"}),
    ];
    let ops_b = vec![
        json!({"editType": "addNode", "id": "__temp1", "label": "Node", "isLiteral": false}),
        json!({"editType": "addNode", "id": "__temp3", "label": "3", "isLiteral": true}),
        json!({"editType": "addEdge", "from": "__temp1", "to": "__temp3", "label": "val"}),
        json!({"editType": "addEdge", "from": "main-new1", "to": "__temp1", "label": "next"}),
    ];

    // 3) Unification → common op ids (e.g., op_0)
    let analysis = refsyn::analyze_operations_with_unification(&vis_graph, &ops_a, &ops_b)?;
    let mut env = ListEnvironment::from_vis_graph(&vis_graph);

    // 4) Apply common A-ops (in original order) to reach the boundary snapshot
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
        // Map String error from ListEnvironment into anyhow::Error
        env.apply_operation(&graph_op)
            .map_err(|e| anyhow::anyhow!(e))?;
    }

    // 5) Build one Escher example (args and expected output are placeholders)
    let case = EscherCase {
        env,
        vis_graph: vis_graph.clone(),
        arguments: vec![json!(0)], // receiver pointer
        arg_names: vec!["this".to_string()],
        arg_types: Some(vec!["Ptr".to_string()]),
        receiver_arg_index: Some(0),
        output: json!(0),          // example expected output
    };

    // 6) Emit JSON and write to Escher-Scala resource path
    let spec = build_escher_spec("append-g", "Int", &[case], None)?;
    let spec_bundle = vec![spec];
    let json_text = specs_to_json(&spec_bundle)?;
    let out_path = "Escher-Scala/src/main/resources/escher/tests.json";
    write_spec_to_file(out_path, &json_text)?;
    println!("Wrote {} bytes to {}", json_text.len(), out_path);

    Ok(())
}

use refsyn::escher_bridge::{
    build_escher_spec, build_escher_task_spec, derive_spec_meta, EscherCase,
};
use refsyn::list_env::{GraphOperation, ListEnvironment};
use refsyn::models::VisGraph;
use refsyn::unify_ops::{unify_operation_graphs, EdgeExpr, GraphOp, NodeExpr, Op};
use serde_json::{json, Value};

fn node(id: &str, label: &str, literal: bool) -> Op {
    Op {
        id: id.into(),
        kind: GraphOp::Node(NodeExpr::AddNode {
            id: id.into(),
            label: label.into(),
            is_literal: literal,
        }),
    }
}
fn edge(id: &str, from: &str, to: &str, label: &str) -> Op {
    Op {
        id: id.into(),
        kind: GraphOp::Edge(EdgeExpr::AddEdge {
            from: from.into(),
            to: to.into(),
            label: label.into(),
        }),
    }
}
fn spec(env: ListEnvironment, graph: VisGraph) -> Value {
    let c = EscherCase {
        env,
        vis_graph: graph,
        class_name: Some("Node".into()),
        arguments: vec![],
        arg_names: vec![],
        arg_types: None,
        receiver_arg_index: None,
        output: Value::Null,
    };
    serde_json::to_value(build_escher_spec("audit", "Ptr", &[c], None).unwrap()).unwrap()
}
fn main() {
    let output_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/refsyn-audit-20261007".into());
    std::fs::create_dir_all(&output_dir).unwrap();
    let mut results = vec![];
    let a = vec![
        node("a", "Node", false),
        node("v", "1", true),
        edge("e", "a", "v", "val"),
    ];
    let b = vec![
        node("b", "Tree", false),
        node("w", "2", true),
        edge("f", "b", "w", "val"),
    ];
    let r = unify_operation_graphs(&a, &b);
    results.push(json!({"probe":"class_mismatch", "mapping": r.final_mapping,
        "common_a":r.common_a, "common_b":r.common_b}));
    let b = vec![
        node("b", "Node", false),
        node("w", "true", true),
        edge("f", "b", "w", "val"),
    ];
    let r = unify_operation_graphs(&a, &b);
    results.push(
        json!({"probe":"primitive_type_mismatch", "mapping":r.final_mapping,
        "common_a":r.common_a}),
    );
    let mut repeated = a.clone();
    repeated.push(edge("e2", "a", "v", "val"));
    let r = unify_operation_graphs(&repeated, &a);
    results.push(json!({"probe":"edge_multiplicity", "common_a":r.common_a,
        "common_b":r.common_b, "diff_a":r.diff_a, "diff_b":r.diff_b}));
    let equal_count_a = vec![
        node("a", "Node", false),
        node("t", "Node", false),
        edge("e1", "a", "t", "left"),
        edge("e2", "a", "t", "left"),
        edge("e3", "a", "t", "right"),
    ];
    let equal_count_b = vec![
        node("a", "Node", false),
        node("t", "Node", false),
        edge("e1", "a", "t", "left"),
        edge("e2", "a", "t", "right"),
        edge("e3", "a", "t", "right"),
    ];
    let r = unify_operation_graphs(&equal_count_a, &equal_count_b);
    results.push(
        json!({"probe":"equal_total_edge_multiplicity", "common_a":r.common_a,
        "common_b":r.common_b,"diff_a":r.diff_a,"diff_b":r.diff_b}),
    );
    let mut noniso = a.clone();
    noniso.push(edge("different", "a", "v", "other"));
    let r = unify_operation_graphs(&a, &noniso);
    results.push(
        json!({"probe":"nonisomorphic_partial_acceptance", "mapping":r.final_mapping,
        "common_a":r.common_a, "diff_b":r.diff_b}),
    );
    let r = unify_operation_graphs(&a, &a);
    results.push(
        json!({"probe":"equal_operands_not_holed", "common_a":r.common_a, "diff_a":r.diff_a}),
    );

    let empty: VisGraph = serde_json::from_value(json!({"nodes":[],"edges":[]})).unwrap();
    let numeric_ops = vec![
        json!({"editType":"addNode","id":"A","label":"Node","isLiteral":false}),
        json!({"editType":"addNode","id":"v","label":1,"isLiteral":true}),
        json!({"editType":"addEdge","from":"A","to":"v","label":"val"}),
    ];
    let r = refsyn::analyze_operations_with_unification(&empty, &empty, &numeric_ops, &numeric_ops)
        .unwrap();
    results.push(
        json!({"probe":"numeric_label_conversion", "common_a":r.unification_result.common_a,
        "diff_a":r.unification_result.diff_a}),
    );

    let graph: VisGraph = serde_json::from_value(json!({"nodes":[
        {"id":"A", "label":"Node", "isLiteral":false}],"edges":[]}))
    .unwrap();
    let mut env = ListEnvironment::from_vis_graph(&graph);
    for raw in [
        json!({"editType":"addNode","id":"N","label":"Node","isLiteral":false}),
        json!({"editType":"addEdge","from":"A","to":"N","label":"next"}),
    ] {
        env.apply_operation(&serde_json::from_value::<GraphOperation>(raw).unwrap())
            .unwrap();
    }
    let dropped = spec(env.clone(), graph.clone());
    let mut augmented = graph.clone();
    augmented
        .nodes
        .push(serde_json::from_value(json!({"id":"N","label":"Node","isLiteral":false})).unwrap());
    let retained = spec(env.clone(), augmented);
    results.push(
        json!({"probe":"prefix_new_object_dropped", "raw_env":env.to_debug_string(),
        "original_vis_graph":dropped, "augmented_vis_graph":retained}),
    );

    let graph: VisGraph = serde_json::from_value(json!({"nodes":[
        {"id":"A","label":"Node","isLiteral":false},
        {"id":"v","label":4294967296_i64,"isLiteral":true},
        {"id":"b","label":true,"isLiteral":true}],"edges":[
        {"from":"A","to":"v","label":"large"},
        {"from":"A","to":"b","label":"flag"}]}))
    .unwrap();
    results.push(json!({"probe":"value_encoding", "spec":spec(ListEnvironment::from_vis_graph(&graph),graph)}));
    let graph: VisGraph = serde_json::from_value(json!({"nodes":[
        {"id":"A","label":"Node","isLiteral":false},
        {"id":"v","label":-2,"isLiteral":true}],"edges":[
        {"from":"A","to":"v","label":"val"}]}))
    .unwrap();
    let c = EscherCase {
        env: ListEnvironment::from_vis_graph(&graph),
        vis_graph: graph,
        class_name: Some("Node".into()),
        arguments: vec![json!(0)],
        arg_names: vec!["this".into()],
        arg_types: Some(vec!["Ptr".into()]),
        receiver_arg_index: Some(0),
        output: json!(false),
    };
    let mut positive = c.clone();
    positive.env.field_lists.get_mut("val").unwrap()[0] = json!(2);
    positive
        .vis_graph
        .nodes
        .iter_mut()
        .find(|n| n.id == "v")
        .unwrap()
        .label = json!(2);
    positive.output = json!(true);
    let cases = [c, positive];
    let native = build_escher_task_spec(
        &build_escher_spec("negativeHas", "Bool", &cases, None).unwrap(),
        &derive_spec_meta(&cases).unwrap(),
    )
    .unwrap();
    let mut native = serde_json::to_value(native).unwrap();
    native["components"] = json!([]);
    std::fs::write(
        format!("{output_dir}/negative-task.json"),
        serde_json::to_string_pretty(&native).unwrap(),
    )
    .unwrap();
    results.push(json!({"probe":"negative_value_treated_as_missing", "native_task":native}));
    std::fs::write(
        format!("{output_dir}/probes.json"),
        serde_json::to_string_pretty(&results).unwrap(),
    )
    .unwrap();
    println!(
        "AUDIT_JSON\n{}",
        serde_json::to_string_pretty(&results).unwrap()
    );
}

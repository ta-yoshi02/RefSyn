use refsyn::{synthesize_core, SynthesisCoreOptions, SynthesisRequest};
use serde_json::{json, Value};

#[test]
fn insert_arguments_and_pre_call_heap_become_an_escher_example() {
    let request: SynthesisRequest =
        serde_json::from_str(include_str!("data/paper_insert.json")).unwrap();

    let converted = synthesize_core(request, SynthesisCoreOptions::default()).unwrap();
    let task_json = converted.task_json.unwrap();
    let tasks: Value = serde_json::from_str(&task_json).unwrap();

    let task = &tasks[0];
    let input = &task["examples"][0][0];
    let output = &task["examples"][0][1];

    assert_eq!(task["name"], json!("insert_h0"));
    assert_eq!(input[0], json!({"ref": 0}));
    assert_eq!(
        input[2],
        json!([
            {"ref": 1}, {"ref": 2}, {"ref": -1}, {"ref": -1}
        ])
    );
    assert_eq!(input[3], json!([39, 27, 53, {"error": true}]));
    assert_eq!(input[4], json!(0));
    assert_eq!(input[5], json!(89));
    assert_eq!(input[6], json!({"ref": 3}));
    assert_eq!(output, &json!(89));
}

// 縦の段階2〜5: 01-setval.json を合成器の段階に 1 つずつ通し、各段階が何を受け取って何を返すかを表示する。
// 段階の並びは src/pipeline.rs の plan と同じ。今は表示するだけ。予想してから実行し、
// 確かめたことを assert に書き足していく。
// 実行 (段階ごと): cargo test --test step_stages graph -- --nocapture
//                  (graph の代わりに mapping / template / problems)
use refsyn::alignment::{self, Mapping};
use refsyn::demonstration::{self, Recording};
use refsyn::escher_bridge;
use refsyn::operation_graph::{self, OperationGraph};
use refsyn::pbe_problem::{self, Problems};
use refsyn::template::Template;
use refsyn::SynthesisRequest;

// 前の段階までを順に実行する。各テストは、自分の段階の出力だけを表示する。
fn recording() -> Recording {
    let request: SynthesisRequest =
        serde_json::from_str(include_str!("steps/01-setval.json")).unwrap();
    demonstration::read(&request).unwrap()
}

fn graphs(recording: &Recording) -> Vec<OperationGraph> {
    recording
        .demonstrations
        .iter()
        .map(|demo| operation_graph::build(demo, &recording.classes).unwrap())
        .collect()
}

// pipeline.rs が使う align_consistently は、align_all と同じ探索をしたうえで、穴の例が矛盾する
// 対応関係を飛ばす。この記録ではどちらも同じ対応関係を返すことを mapping テストで確かめている。
fn mappings(graphs: &[OperationGraph]) -> Vec<Mapping> {
    alignment::align_all(graphs).unwrap()
}

fn build_template(graphs: &[OperationGraph], mappings: &[Mapping]) -> Template {
    Template::build(graphs, mappings).unwrap()
}

fn build_problems(
    recording: &Recording,
    graphs: &[OperationGraph],
    mappings: &[Mapping],
    template: &Template,
) -> Problems {
    let params = &recording.demonstrations[0].param_names;
    pbe_problem::build(recording, graphs, mappings, template, params).unwrap()
}

// 段階2 操作グラフの構築: 呼び出し 1 回分の編集列 → 頂点の列と書き込みの列
#[test]
fn graph() {
    let recording = recording();
    for (i, graph) in graphs(&recording).iter().enumerate() {
        println!("--- 呼び出し {} ---\n{graph:#?}", i + 1);
    }
}

// 段階3 対応関係: 全部の操作グラフ → 1 回目のグラフの各頂点が、他のグラフのどの頂点にあたるか
#[test]
fn mapping() {
    let recording = recording();
    let graphs = graphs(&recording);
    let mappings = mappings(&graphs);
    for (i, mapping) in mappings.iter().enumerate() {
        println!("--- 呼び出し 1 → 呼び出し {} ---\n{mapping:?}", i + 1);
    }
    let params = &recording.demonstrations[0].param_names;
    let (used, _, _) = alignment::align_consistently(&recording, &graphs, params).unwrap();
    assert_eq!(used, mappings);
}

// 段階4 雛形の構築: グラフと対応関係 → 生成・穴・書き込み・戻り値の並び
#[test]
fn template() {
    let recording = recording();
    let graphs = graphs(&recording);
    let mappings = mappings(&graphs);
    let template = build_template(&graphs, &mappings);
    for statement in template.statements() {
        println!("{statement:?}");
    }
    println!("{}", template.render_method("setVal", &["arg".to_string()]));
}

// 段階5 穴の入力環境の構築: 雛形 → 穴ごとの escher-ts の問題 (入力と、返すべき出力の例)
#[test]
fn problems() {
    let recording = recording();
    let graphs = graphs(&recording);
    let mappings = mappings(&graphs);
    let template = build_template(&graphs, &mappings);
    let problems = build_problems(&recording, &graphs, &mappings, &template);
    let tasks: serde_json::Value =
        serde_json::from_str(&escher_bridge::tasks_to_json(&problems.tasks).unwrap()).unwrap();
    println!("{}", serde_json::to_string_pretty(&tasks).unwrap());
}

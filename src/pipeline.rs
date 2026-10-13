//! Assembles the paper's synthesis steps up to the PBE problems: recording → operation graphs
//! → correspondence → template → per-hole problems and the composed method.

use anyhow::{ensure, Context, Result};

use crate::escher_bridge::EscherTaskSpec;
use crate::pbe_problem::HoleCheck;
use crate::template::{Example, Template};
use crate::{alignment, demonstration, operation_graph, pbe_problem, SynthesisRequest};

pub struct Plan {
    pub method: String,
    pub composed_method_code: String,
    pub tasks: Vec<EscherTaskSpec>,
    pub checks: Vec<HoleCheck>,
    /// Display-only rendering of the template and each hole's outputs.
    pub template_text: String,
    pub hole_information: std::collections::HashMap<String, Vec<String>>,
}

pub fn plan(request: &SynthesisRequest) -> Result<Plan> {
    let recording = demonstration::read(request)?;
    let graphs = recording
        .demonstrations
        .iter()
        .enumerate()
        .map(|(i, demo)| {
            operation_graph::build(demo, &recording.classes)
                .with_context(|| format!("demonstration {}", i + 1))
        })
        .collect::<Result<Vec<_>>>()?;
    let mappings = alignment::align_all(&graphs)?;
    let template = Template::build(&graphs, &mappings)?;
    let first = &recording.demonstrations[0];
    // Helper names extend the method name, so the definitions and the template's calls agree
    // only if it is an identifier; it is refused rather than renamed.
    ensure!(
        Template::is_identifier(&first.method),
        "method name {:?} is not a JavaScript identifier",
        first.method
    );
    ensure!(
        recording
            .demonstrations
            .iter()
            .all(|d| d.param_names == first.param_names),
        "demonstrations disagree on the method's parameter names"
    );
    let problems = pbe_problem::build(
        &recording,
        &graphs,
        &mappings,
        &template,
        &first.param_names,
    )?;
    let composed_method_code = template.render_method(&first.method, &first.param_names);
    let template_text = template
        .statements()
        .iter()
        .map(|s| format!("{s:?}"))
        .collect::<Vec<_>>()
        .join("\n");
    let hole_information = template
        .holes
        .iter()
        .enumerate()
        .map(|(k, hole)| {
            let outputs = hole
                .examples
                .iter()
                .map(|e| match e {
                    Example::Object(id) => format!("object {id}"),
                    Example::Constant(c) => format!("{c:?}"),
                })
                .collect();
            (Template::helper_name(&first.method, k), outputs)
        })
        .collect();
    Ok(Plan {
        method: first.method.clone(),
        composed_method_code,
        tasks: problems.tasks,
        checks: problems.checks,
        template_text,
        hole_information,
    })
}

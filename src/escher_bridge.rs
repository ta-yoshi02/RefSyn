//! Escher bridge: build intermediate specs and native escher-ts tasks from Kanon environments
//!
//! This module converts ListEnvironment snapshots (built from Kanon VisGraph + operations)
//! into either:
//! - intermediate grouped specs
//! - native escher-ts task JSON
//!
//! It supports:
//! - Dynamic field detection (value vs pointer) without hardcoding names
//! - Stable object indexing per test case using ListEnvironment object IDs
//! - nullPtr is encoded as JSON null; missing Int values remain -1
//! - Deterministic ordering of inputs (args, then value lists, then pointer lists)

#[cfg(not(target_arch = "wasm32"))]
use anyhow::anyhow;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
#[cfg(not(target_arch = "wasm32"))]
use std::io::Write;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::process::{Command, Stdio};

#[derive(Serialize, Debug, Clone)]
pub struct EscherTaskSpec {
    pub name: String,
    pub category: String,
    pub classes: Vec<EscherTaskClassSpec>,
    #[serde(rename = "exposeClassComponents")]
    pub expose_class_components: bool,
    #[serde(rename = "autoClassFieldComponents")]
    pub auto_class_field_components: bool,
    pub signature: EscherTaskSignature,
    pub components: Vec<EscherTaskComponentSpec>,
    pub examples: Vec<(Vec<Value>, Value)>,
    #[serde(rename = "refsynMeta")]
    pub refsyn_meta: EscherTaskMeta,
}

#[derive(Serialize, Debug, Clone)]
pub struct EscherTaskClassSpec {
    pub name: String,
    pub fields: BTreeMap<String, String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct EscherTaskSignature {
    #[serde(rename = "returnType")]
    pub return_type: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<EscherTaskSignatureArg>,
    #[serde(rename = "autoExpandClassSignature")]
    pub auto_expand_class_signature: EscherTaskAutoExpandSignature,
}

#[derive(Serialize, Debug, Clone)]
pub struct EscherTaskSignatureArg {
    pub name: String,
    #[serde(rename = "type")]
    pub arg_type: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct EscherTaskAutoExpandSignature {
    #[serde(rename = "className")]
    pub class_name: String,
    #[serde(rename = "thisRefName")]
    pub this_ref_name: String,
    #[serde(rename = "classHeapName")]
    pub class_heap_name: String,
    #[serde(rename = "fieldHeapNames")]
    pub field_heap_names: BTreeMap<String, String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct EscherTaskComponentSpec {
    pub name: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "ref")]
    pub ref_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "inputTypes")]
    pub input_types: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "returnType")]
    pub return_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "bodyJs")]
    pub body_js: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct EscherTaskMeta {
    #[serde(rename = "jsMethodName")]
    pub js_method_name: String,
    #[serde(rename = "className")]
    pub class_name: String,
    #[serde(rename = "thisRefName")]
    pub this_ref_name: String,
    #[serde(rename = "classHeapName")]
    pub class_heap_name: String,
    #[serde(rename = "valueFields")]
    pub value_fields: Vec<String>,
    #[serde(rename = "pointerFields")]
    pub pointer_fields: Vec<String>,
    #[serde(rename = "fieldHeapNames")]
    pub field_heap_names: BTreeMap<String, String>,
    #[serde(rename = "explicitArgs")]
    pub explicit_args: Vec<EscherTaskMetaArg>,
}

#[derive(Serialize, Debug, Clone)]
pub struct EscherTaskMetaArg {
    pub name: String,
    #[serde(rename = "legacyType")]
    pub legacy_type: String,
    #[serde(rename = "taskType")]
    pub task_type: String,
}

pub fn tasks_to_json(tasks: &[EscherTaskSpec]) -> Result<String> {
    Ok(serde_json::to_string_pretty(tasks)?)
}

/// Convenience: write the produced JSON string to a file path
/// (for example `target/escher/ts/tests.json`).
#[cfg(not(target_arch = "wasm32"))]
pub fn write_spec_to_file(path: &str, content: &str) -> Result<()> {
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, content)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscherJsOutcome {
    pub name: String,
    pub success: bool,
    pub rendered: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EscherJsInternalOutcome {
    pub name: String,
    pub success: bool,
    pub rendered: Option<String>,
    pub error: Option<String>,
    #[serde(default)]
    pub compiled_js: Option<String>,
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_escher_js_max_old_space_mb(raw: Option<&str>) -> Result<usize> {
    const DEFAULT_MB: usize = 8192;
    const MIN_MB: usize = 256;

    let Some(raw) = raw else {
        return Ok(DEFAULT_MB);
    };

    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(DEFAULT_MB);
    }

    let parsed = trimmed.parse::<usize>().map_err(|_| {
        anyhow!(
            "Invalid ESCHER_JS_MAX_OLD_SPACE_MB='{}'. Expected a positive integer (MB).",
            trimmed
        )
    })?;
    if parsed < MIN_MB {
        return Err(anyhow!(
            "ESCHER_JS_MAX_OLD_SPACE_MB must be >= {} (got {}).",
            MIN_MB,
            parsed
        ));
    }
    Ok(parsed)
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_escher_js_max_old_space_mb() -> Result<usize> {
    parse_escher_js_max_old_space_mb(std::env::var("ESCHER_JS_MAX_OLD_SPACE_MB").ok().as_deref())
}

/// Invoke the configured escher-ts runner via Node and parse normalized results.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_escher_js(task_json: &str) -> Result<Vec<EscherJsInternalOutcome>> {
    let default_runner = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("scripts")
        .join("run_escher.js");
    let runner = std::env::var("ESCHER_JS_RUNNER")
        .map(PathBuf::from)
        .unwrap_or(default_runner);

    if !runner.exists() {
        return Err(anyhow!(
            "Escher JS runner not found at {}",
            runner.display()
        ));
    }

    let max_old_space_mb = resolve_escher_js_max_old_space_mb()?;
    let mut command = Command::new("node");
    command
        .arg(format!("--max-old-space-size={}", max_old_space_mb))
        .arg(&runner)
        .arg("--quiet")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());

    let mut child = command
        .spawn()
        .map_err(|e| anyhow!("Failed to spawn Node runner: {}", e))?;

    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("Failed to open stdin for Node runner"))?;
        stdin.write_all(task_json.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(anyhow!(
            "Escher JS runner exited with status {} (node max-old-space-size={} MB; override via ESCHER_JS_MAX_OLD_SPACE_MB)",
            output.status,
            max_old_space_mb
        ));
    }

    let stdout = String::from_utf8(output.stdout)?;
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("Escher JS runner returned empty output"));
    }

    let outcomes: Vec<EscherJsInternalOutcome> = serde_json::from_str(trimmed)?;
    Ok(outcomes)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_escher_js_from_tasks(tasks: &[EscherTaskSpec]) -> Result<Vec<EscherJsInternalOutcome>> {
    let json_text = tasks_to_json(tasks)?;
    run_escher_js(&json_text)
}

impl From<&EscherJsInternalOutcome> for EscherJsOutcome {
    fn from(value: &EscherJsInternalOutcome) -> Self {
        Self {
            name: value.name.clone(),
            success: value.success,
            rendered: value.rendered.clone(),
            error: value.error.clone(),
        }
    }
}

/// Library components for a task whose values include integers and/or object pointers.
pub(crate) fn library_components(ints: bool, pointers: bool) -> Vec<EscherTaskComponentSpec> {
    let mut components = Vec::new();
    let mut seen = HashSet::new();
    for name in ["isNull", "equal", "and", "or", "not"] {
        push_library_component(&mut components, &mut seen, name);
    }
    if ints {
        for name in ["isZero", "isNonNeg", "zero", "inc", "dec"] {
            push_library_component(&mut components, &mut seen, name);
        }
    }
    if pointers {
        for name in ["last_ptr", "penultimateRef", "nthNextRef"] {
            push_library_component(&mut components, &mut seen, name);
        }
    }
    if ints && pointers {
        push_library_component(&mut components, &mut seen, "findByValueRef");
    }
    components
}

fn push_library_component(
    components: &mut Vec<EscherTaskComponentSpec>,
    seen: &mut HashSet<String>,
    name: &str,
) {
    if seen.insert(name.to_string()) {
        components.push(EscherTaskComponentSpec {
            name: name.to_string(),
            kind: "libraryRef".to_string(),
            ref_name: Some(name.to_string()),
            input_types: None,
            return_type: None,
            args: None,
            body_js: None,
        });
    }
}

pub(crate) fn field_to_heap_name(field: &str) -> String {
    let mut out = String::with_capacity(field.len() + 4);
    for (idx, ch) in field.chars().enumerate() {
        let valid = if idx == 0 {
            ch.is_ascii_alphabetic() || ch == '_' || ch == '$'
        } else {
            ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'
        };
        if valid {
            out.push(ch);
        } else if idx == 0 && ch.is_ascii_digit() {
            out.push('_');
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push('_');
    }
    out.push_str("Heap");
    out
}

pub(crate) fn sanitize_js_identifier(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        let valid = ch.is_ascii_alphanumeric() || ch == '_' || ch == '$';
        out.push(if valid { ch } else { '_' });
    }
    if out.is_empty() {
        "_".to_string()
    } else if out
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        format!("_{}", out)
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_escher_js_max_old_space_mb_defaults_to_8192() {
        let mb = parse_escher_js_max_old_space_mb(None).expect("default");
        assert_eq!(mb, 8192);
    }

    #[test]
    fn parse_escher_js_max_old_space_mb_accepts_valid_integer() {
        let mb = parse_escher_js_max_old_space_mb(Some("12288")).expect("parsed");
        assert_eq!(mb, 12288);
    }

    #[test]
    fn parse_escher_js_max_old_space_mb_rejects_small_value() {
        let err = parse_escher_js_max_old_space_mb(Some("128")).expect_err("must fail");
        assert!(err.to_string().contains("must be >= 256"));
    }

    #[test]
    fn parse_escher_js_max_old_space_mb_rejects_non_numeric() {
        let err = parse_escher_js_max_old_space_mb(Some("abc")).expect_err("must fail");
        assert!(err
            .to_string()
            .contains("Invalid ESCHER_JS_MAX_OLD_SPACE_MB"));
    }
}

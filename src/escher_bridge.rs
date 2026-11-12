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
use crate::child_process::run_with_deadline;
#[cfg(not(target_arch = "wasm32"))]
use anyhow::anyhow;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use std::process::{Command, Stdio};
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

#[derive(Serialize, Debug, Clone)]
pub struct EscherTaskSpec {
    pub name: String,
    pub category: String,
    pub classes: Vec<EscherTaskClassSpec>,
    #[serde(rename = "exposeClassComponents")]
    pub expose_class_components: bool,
    #[serde(rename = "autoClassFieldComponents")]
    pub auto_class_field_components: bool,
    /// Field components fail on a null receiver, as the emitted `receiver.field` does in JS.
    #[serde(rename = "strictFieldAccess")]
    pub strict_field_access: bool,
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
    /// Fields typed Bool in escher-ts, read straight from the object's slot.
    #[serde(rename = "boolFields")]
    pub bool_fields: Vec<String>,
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

#[cfg(not(target_arch = "wasm32"))]
fn resolve_escher_js_process_timeout() -> Result<Duration> {
    const DEFAULT_MS: u64 = 60_000;
    match std::env::var("ESCHER_JS_PROCESS_TIMEOUT_MS") {
        Err(_) => Ok(Duration::from_millis(DEFAULT_MS)),
        Ok(raw) => match raw.trim().parse::<u64>() {
            Ok(ms) if ms > 0 => Ok(Duration::from_millis(ms)),
            _ => Err(anyhow!(
                "Invalid ESCHER_JS_PROCESS_TIMEOUT_MS='{}'. Expected a positive integer (ms).",
                raw.trim()
            )),
        },
    }
}

/// Invoke the configured escher-ts runner via Node and parse normalized results.
///
/// escher-ts bounds each task's search itself (`ESCHER_TS_TIMEOUT_MS`); the runner process as a
/// whole is bounded here (`ESCHER_JS_PROCESS_TIMEOUT_MS`, 60 s by default) and killed past it.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_escher_js(task_json: &str) -> Result<Vec<EscherJsInternalOutcome>> {
    let default_runner = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("scripts")
        .join("run_escher.js");
    let runner = std::env::var("ESCHER_JS_RUNNER")
        .map(PathBuf::from)
        .unwrap_or(default_runner);
    let max_old_space_mb = resolve_escher_js_max_old_space_mb()?;
    let timeout = resolve_escher_js_process_timeout()?;
    run_escher_runner(&runner, task_json, max_old_space_mb, timeout)
}

#[cfg(not(target_arch = "wasm32"))]
fn run_escher_runner(
    runner: &Path,
    task_json: &str,
    max_old_space_mb: usize,
    timeout: Duration,
) -> Result<Vec<EscherJsInternalOutcome>> {
    if !runner.exists() {
        return Err(anyhow!(
            "Escher JS runner not found at {}",
            runner.display()
        ));
    }

    let mut command = Command::new("node");
    command
        .arg(format!("--max-old-space-size={}", max_old_space_mb))
        .arg(runner)
        .arg("--quiet")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());

    let output = run_with_deadline(
        "Escher JS runner",
        command,
        task_json.as_bytes().to_vec(),
        timeout,
    )?;
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

    let outcomes: Vec<EscherJsInternalOutcome> = serde_json::from_str(trimmed)
        .map_err(|error| anyhow!("Escher JS runner returned invalid JSON: {}", error))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Writes `body` as a Node script in a fresh directory and returns its path.
    fn runner(name: &str, body: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("refsyn-runner-{}-{}", std::process::id(), name));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("runner.js");
        std::fs::write(&path, body).unwrap();
        path
    }

    fn run(name: &str, body: &str, timeout_ms: u64) -> Result<Vec<EscherJsInternalOutcome>> {
        run_escher_runner(
            &runner(name, body),
            "[]",
            256,
            Duration::from_millis(timeout_ms),
        )
    }

    fn error_of(result: Result<Vec<EscherJsInternalOutcome>>) -> String {
        result.expect_err("the runner must fail").to_string()
    }

    #[test]
    fn a_runner_reply_is_parsed_into_outcomes() {
        let outcomes = run(
            "ok",
            r#"process.stdin.resume(); process.stdin.on("end", () => process.stdout.write('[{"name":"h0","success":true,"rendered":"x","error":null}]'));"#,
            10_000,
        )
        .unwrap();
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].name, "h0");
        assert!(outcomes[0].success);
    }

    #[test]
    fn a_missing_runner_is_reported_without_starting_node() {
        let missing = std::env::temp_dir().join("refsyn-no-such-runner/runner.js");
        let error = error_of(run_escher_runner(
            &missing,
            "[]",
            256,
            Duration::from_secs(10),
        ));
        assert!(
            error.starts_with("Escher JS runner not found at "),
            "{error}"
        );
    }

    #[test]
    fn each_kind_of_runner_failure_is_reported_as_such() {
        let cases = [
            (
                "exit",
                "process.exit(3);",
                "Escher JS runner exited with status",
            ),
            ("empty", "", "Escher JS runner returned empty output"),
            (
                "invalid",
                "process.stdout.write('not json');",
                "Escher JS runner returned invalid JSON",
            ),
            (
                "shape",
                "process.stdout.write('{\"name\":1}');",
                "Escher JS runner returned invalid JSON",
            ),
        ];
        for (name, body, expected) in cases {
            let error = error_of(run(name, body, 10_000));
            assert!(error.starts_with(expected), "{name}: {error}");
        }
    }

    #[test]
    fn a_runner_that_does_not_finish_is_killed_at_the_deadline() {
        let started = std::time::Instant::now();
        // The runner neither reads its input nor exits, so neither the write nor the wait returns.
        let error = error_of(run_escher_runner(
            &runner("hang", "setInterval(() => {}, 1000);"),
            &"x".repeat(1 << 20),
            256,
            Duration::from_millis(500),
        ));
        assert_eq!(error, "Escher JS runner did not finish within 500 ms");
        assert!(started.elapsed() < Duration::from_secs(5));
    }

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

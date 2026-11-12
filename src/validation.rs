use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    Passed,
    Failed,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub status: ValidationStatus,
    pub checked_demonstrations: usize,
    pub error: Option<String>,
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub fn validate_for_adoption(body: &[u8], artifacts: &mut crate::SynthesisArtifacts) {
    use std::process::{Command, Stdio};
    use std::time::Duration;

    const PROCESS_TIMEOUT: Duration = Duration::from_secs(10);

    let run = || -> anyhow::Result<ValidationResult> {
        let request: serde_json::Value = serde_json::from_slice(body)?;
        let tasks: Vec<serde_json::Value> = artifacts
            .task_json
            .as_deref()
            .map(serde_json::from_str)
            .transpose()?
            .unwrap_or_default();
        let names: Vec<&str> = tasks
            .iter()
            .map(|task| {
                task["name"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Validation task has no name"))
            })
            .collect::<anyhow::Result<_>>()?;
        let input = serde_json::to_vec(&serde_json::json!({
            "request": request,
            "response": artifacts.response,
            "taskNames": names,
        }))?;
        let mut command = Command::new("node");
        command
            .arg("--max-old-space-size=128")
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/scripts/validate_method.mjs"
            ))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // The VM timeout only bounds synchronous execution; this bounds the whole process.
        let crate::child_process::ChildOutput {
            status,
            stdout,
            stderr,
        } = crate::child_process::run_with_deadline(
            "Whole-method validator",
            command,
            input,
            PROCESS_TIMEOUT,
        )?;
        anyhow::ensure!(
            status.success(),
            "Whole-method validator exited: {}",
            String::from_utf8_lossy(&stderr)
        );
        Ok(serde_json::from_slice(&stdout)?)
    };
    let result = run().unwrap_or_else(|error| unavailable(error.to_string()));
    record(artifacts, result);
}

/// Records that nothing could be validated, for a reason found before validation, and drops the code.
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub fn reject_without_validation(artifacts: &mut crate::SynthesisArtifacts, error: String) {
    record(artifacts, unavailable(error));
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
fn unavailable(error: String) -> ValidationResult {
    ValidationResult {
        status: ValidationStatus::Unavailable,
        checked_demonstrations: 0,
        error: Some(error),
    }
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
fn record(artifacts: &mut crate::SynthesisArtifacts, result: ValidationResult) {
    if !matches!(result.status, ValidationStatus::Passed) {
        artifacts.response.composed_method_code = None;
        artifacts.response.code.clear();
    }
    artifacts.response.validation = Some(result);
}

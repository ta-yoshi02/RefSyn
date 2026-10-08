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
    use std::io::Write;
    use std::process::{Command, Stdio};

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
        let mut child = Command::new("node")
            .arg("--max-old-space-size=128")
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/scripts/validate_method.mjs"
            ))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let write_result = child.stdin.take().unwrap().write_all(&input);
        if let Err(error) = write_result {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error.into());
        }
        let output = child.wait_with_output()?;
        anyhow::ensure!(
            output.status.success(),
            "Whole-method validator exited: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(serde_json::from_slice(&output.stdout)?)
    };
    let result = run().unwrap_or_else(|error| ValidationResult {
        status: ValidationStatus::Unavailable,
        checked_demonstrations: 0,
        error: Some(error.to_string()),
    });
    if !matches!(result.status, ValidationStatus::Passed) {
        artifacts.response.composed_method_code = None;
        artifacts.response.code.clear();
    }
    artifacts.response.validation = Some(result);
}

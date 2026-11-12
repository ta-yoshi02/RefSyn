//! Run a child process with input on stdin and a wall-clock limit enforced by this process.
//!
//! A limit inside the child (escher-ts's search budget, the validator's VM timeout) does not
//! bound startup, a stuck read, or work outside what it measures; this one ends the process.

use anyhow::{anyhow, bail, Result};
use std::io::{Read, Write};
use std::process::{Command, ExitStatus};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub(crate) struct ChildOutput {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    /// Empty unless the command was given a piped stderr.
    pub stderr: Vec<u8>,
}

/// Spawns `command`, which must have piped stdin and stdout, writes `input`, and waits at most
/// `timeout`. On timeout the child is killed and an error naming `what` is returned.
pub(crate) fn run_with_deadline(
    what: &str,
    mut command: Command,
    input: Vec<u8>,
    timeout: Duration,
) -> Result<ChildOutput> {
    let mut child = command
        .spawn()
        .map_err(|error| anyhow!("Failed to start {}: {}", what, error))?;
    // Writing and reading run on their own threads so that a child that stops reading or
    // writing cannot block this thread past the deadline.
    let mut stdin = child.stdin.take().expect("stdin must be piped");
    let writer = std::thread::spawn(move || stdin.write_all(&input));
    let stdout = read_to_end(child.stdout.take().expect("stdout must be piped"));
    let stderr = child.stderr.take().map(read_to_end);

    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            bail!("{} did not finish within {} ms", what, timeout.as_millis());
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let stdout = join(stdout)?;
    let stderr = stderr.map(join).transpose()?.unwrap_or_default();
    // A child that exits without reading all of its input closes the pipe; its exit status and
    // output explain the failure better than the write error, so the write error is not reported.
    let _ = writer.join();
    Ok(ChildOutput {
        status,
        stdout,
        stderr,
    })
}

fn read_to_end(mut pipe: impl Read + Send + 'static) -> JoinHandle<std::io::Result<Vec<u8>>> {
    std::thread::spawn(move || {
        let mut buffer = Vec::new();
        pipe.read_to_end(&mut buffer).map(|_| buffer)
    })
}

fn join(handle: JoinHandle<std::io::Result<Vec<u8>>>) -> Result<Vec<u8>> {
    Ok(handle
        .join()
        .map_err(|_| anyhow!("pipe reader panicked"))??)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;

    fn piped(program: &str, args: &[&str]) -> Command {
        let mut command = Command::new(program);
        command
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }

    #[test]
    fn a_program_that_cannot_start_is_reported_as_such() {
        let error = run_with_deadline(
            "Missing tool",
            piped("refsyn-no-such-program", &[]),
            Vec::new(),
            Duration::from_secs(5),
        )
        .err()
        .expect("spawn must fail")
        .to_string();
        assert!(
            error.starts_with("Failed to start Missing tool: "),
            "{error}"
        );
    }

    #[test]
    fn input_output_and_exit_status_are_returned() {
        let output = run_with_deadline(
            "Echo",
            piped(
                "node",
                &["-e", "process.stdin.pipe(process.stdout); console.error('note'); process.exitCode = 2;"],
            ),
            b"payload".to_vec(),
            Duration::from_secs(10),
        )
        .unwrap();
        assert_eq!(output.stdout, b"payload");
        assert_eq!(output.stderr, b"note\n");
        assert_eq!(output.status.code(), Some(2));
    }
}

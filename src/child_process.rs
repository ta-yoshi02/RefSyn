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
/// `timeout` for the child to exit and for its output pipes to close. A descendant that inherits
/// a pipe keeps it open after the child exits, so both are needed. On timeout the child's process
/// group (on Unix, the child and the descendants that stayed in its group) is killed and an error
/// naming `what` is returned.
pub(crate) fn run_with_deadline(
    what: &str,
    mut command: Command,
    input: Vec<u8>,
    timeout: Duration,
) -> Result<ChildOutput> {
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut command, 0);
    let mut child = command
        .spawn()
        .map_err(|error| anyhow!("Failed to start {}: {}", what, error))?;
    // Writing and reading run on their own threads so that a child that stops reading or
    // writing cannot block this thread past the deadline. The writer is never joined: a
    // descendant can hold stdin open without reading it, and the child's exit status and
    // output explain a failure better than a write error.
    let mut stdin = child.stdin.take().expect("stdin must be piped");
    std::thread::spawn(move || stdin.write_all(&input));
    let stdout = read_to_end(child.stdout.take().expect("stdout must be piped"));
    let stderr = child.stderr.take().map(read_to_end);

    let deadline = Instant::now() + timeout;
    let mut status = None;
    loop {
        if status.is_none() {
            status = child.try_wait()?;
        }
        let read = stdout.is_finished() && stderr.as_ref().is_none_or(JoinHandle::is_finished);
        if let (Some(status), true) = (status, read) {
            let stdout = join(stdout)?;
            let stderr = stderr.map(join).transpose()?.unwrap_or_default();
            return Ok(ChildOutput {
                status,
                stdout,
                stderr,
            });
        }
        if Instant::now() >= deadline {
            kill_group(&mut child);
            bail!("{} did not finish within {} ms", what, timeout.as_millis());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Kills the child and, on Unix, every process left in its group. The reader threads end once
/// the last process holding a pipe is gone; they are not waited for.
fn kill_group(child: &mut std::process::Child) {
    #[cfg(unix)]
    // SAFETY: kill(2) with a negative pid only sends a signal to that process group.
    unsafe {
        libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL);
    }
    let _ = child.kill();
    let _ = child.wait();
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

    /// The child exits at once but leaves a descendant holding its stdout, so the pipes stay
    /// open after the child's exit. Before the deadline covered the reads, this returned success
    /// once the descendant ended. The descendant would write a marker after 1 s if it survived.
    #[test]
    fn a_descendant_holding_the_pipes_is_killed_at_the_deadline() {
        let marker = std::env::temp_dir().join(format!("refsyn-descendant-{}", std::process::id()));
        let _ = std::fs::remove_file(&marker);
        let descendant = format!(
            "setTimeout(() => require('fs').writeFileSync({:?}, 'alive'), 1000)",
            marker.display().to_string()
        );
        let script = format!(
            "require('child_process').spawn(process.execPath, ['-e', {descendant:?}], {{ stdio: ['ignore', 'inherit', 'inherit'] }}).unref();"
        );
        let started = Instant::now();
        let error = run_with_deadline(
            "Forking program",
            piped("node", &["-e", &script]),
            Vec::new(),
            Duration::from_millis(500),
        )
        .err()
        .map(|error| error.to_string());
        let elapsed = started.elapsed();
        assert_eq!(
            error.as_deref(),
            Some("Forking program did not finish within 500 ms")
        );
        assert!(elapsed < Duration::from_millis(1500), "{elapsed:?}");
        std::thread::sleep(Duration::from_millis(1500));
        assert!(!marker.exists(), "the descendant outlived the deadline");
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

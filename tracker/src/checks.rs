//! Running script checks (§2.4, §4.2): a criterion's `check` for G3 and at
//! `start`, and the project's `checks` for G4.
//!
//! Each check is a line of `sh`, run in the project's root with stdin
//! closed and stdout and stderr read together. It's printed on stderr
//! before it runs, and stopped once it runs longer than the project's
//! `check_timeout_secs`. Only the last lines of its output are kept.

use std::io::Read;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::text;

/// How many lines of a check's output are kept: for a failure's message,
/// and for the verification record (§4.3).
pub const KEPT_LINES: usize = 20;

/// How much of the end of a check's output is held while it runs, in
/// bytes. Only the last `KEPT_LINES` lines are kept, so a check that
/// prints a lot never fills memory.
const TAIL_BYTES: usize = 64 * 1024;

/// How often a running check is looked at, to see whether it has ended or
/// run out of time.
const POLL: Duration = Duration::from_millis(20);

/// What one run of a check found.
#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    /// The command, as the criterion or config gives it.
    pub cmd: String,
    /// Whether it exited 0 within the time allowed.
    pub ok: bool,
    /// Its exit code, or `None` if a signal stopped it, it ran out of time,
    /// or it couldn't be run at all.
    pub exit_code: Option<i32>,
    /// The signal that stopped it, if one did, other than at a timeout.
    pub signal: Option<i32>,
    /// Whether it ran longer than the timeout, and was stopped.
    pub timed_out: bool,
    /// Why it couldn't be run at all, such as a root that's gone.
    pub error: Option<String>,
    /// How long it ran, to the nearest second.
    pub secs: u64,
    /// Its last `KEPT_LINES` lines of output, stdout and stderr together,
    /// without a final line break. Empty if it printed nothing.
    pub output: String,
}

/// A run of `cmd` that never started, because of `why`, such as a root
/// that's gone.
pub fn not_run(cmd: &str, why: &str) -> Run {
    Run {
        cmd: cmd.to_string(),
        ok: false,
        exit_code: None,
        signal: None,
        timed_out: false,
        error: Some(why.to_string()),
        secs: 0,
        output: String::new(),
    }
}

/// Runs `cmd` with `sh -c` in `root`, stopping it after `timeout`. `what`
/// names it in the line printed before it runs, such as "criterion 2's
/// check". A check that can't be started is a run that failed, with
/// `error` saying why.
pub fn run(cmd: &str, root: &Path, timeout: Duration, what: &str) -> Run {
    eprintln!(
        "Running {what}: {}",
        text::for_terminal(&format!("`{cmd}`"), false)
    );
    let began = Instant::now();
    let failed = |why: String| not_run(cmd, &why);

    let (mut reader, writer) = match std::io::pipe() {
        Ok(pipe) => pipe,
        Err(e) => return failed(format!("can't make a pipe for its output: {e}")),
    };
    let writer_too = match writer.try_clone() {
        Ok(writer) => writer,
        Err(e) => return failed(format!("can't make a pipe for its output: {e}")),
    };
    let mut command = Command::new("sh");
    command
        .arg("-c")
        .arg(cmd)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(writer)
        .stderr(writer_too)
        // Its own process group, so a timeout stops everything it started,
        // not just the shell.
        .process_group(0);
    let spawned = command.spawn();
    // The command holds the pipe's write ends. Dropping it closes them
    // here, so the read below ends when the check's processes do.
    drop(command);
    let mut child = match spawned {
        Ok(child) => child,
        Err(e) => return failed(e.to_string()),
    };

    let tail = thread::spawn(move || {
        let mut tail = Vec::new();
        let mut buffer = [0; 8192];
        while let Ok(n) = reader.read(&mut buffer) {
            if n == 0 {
                break;
            }
            tail.extend_from_slice(&buffer[..n]);
            if tail.len() > 2 * TAIL_BYTES {
                tail.drain(..tail.len() - TAIL_BYTES);
            }
        }
        tail
    });

    let (status, timed_out) = wait(&mut child, began + timeout);
    // Anything the check left running, such as a server it started in the
    // background, would hold the output open, so it's stopped too.
    stop_group(&child);
    let tail = tail.join().unwrap_or_default();
    let secs = began.elapsed().as_secs_f64().round() as u64;

    let (exit_code, signal) = match status {
        Some(status) if !timed_out => (status.code(), status.signal()),
        _ => (None, None),
    };
    Run {
        cmd: cmd.to_string(),
        ok: !timed_out && exit_code == Some(0),
        exit_code,
        signal,
        timed_out,
        error: None,
        secs,
        output: last_lines(&tail),
    }
}

/// Waits for `child` to end, or stops it once `deadline` passes. Returns
/// its exit status, if waiting for it worked, and whether it was stopped.
fn wait(child: &mut Child, deadline: Instant) -> (Option<std::process::ExitStatus>, bool) {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return (Some(status), false),
            Ok(None) => {}
            Err(_) => return (None, false),
        }
        if Instant::now() >= deadline {
            stop_group(child);
            return (child.wait().ok(), true);
        }
        thread::sleep(POLL);
    }
}

/// Stops every process in `child`'s process group with SIGKILL. Its group
/// is numbered after it, since it was started with `process_group(0)`. The
/// standard library has no call for this, so it runs `kill`, which says
/// nothing if the group has already gone.
fn stop_group(child: &Child) {
    let group = format!("-{}", child.id());
    let _ = Command::new("kill")
        .args(["-KILL", "--", &group])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// The last `KEPT_LINES` lines of `output`, read as UTF-8 with anything
/// that isn't replaced, without a final line break or trailing blank lines.
pub fn last_lines(output: &[u8]) -> String {
    let text = String::from_utf8_lossy(output);
    let lines: Vec<&str> = text.trim_end().lines().collect();
    let start = lines.len().saturating_sub(KEPT_LINES);
    lines[start..].join("\n")
}

#[cfg(test)]
mod tests {
    use super::last_lines;

    #[test]
    fn last_lines_keeps_the_last_twenty_without_a_final_break() {
        let output: String = (1..=25).map(|i| format!("{i}\n")).collect();
        let kept: Vec<String> = (6..=25).map(|i| i.to_string()).collect();
        assert_eq!(last_lines(output.as_bytes()), kept.join("\n"));
        assert_eq!(last_lines(b"one\r\ntwo\n\n\n"), "one\ntwo");
        assert_eq!(last_lines(b""), "");
    }
}

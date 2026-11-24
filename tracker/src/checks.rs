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
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use crate::text;

/// How many lines of a check's output are kept: for a failure's message,
/// and for the verification record (§4.3).
pub const KEPT_LINES: usize = 20;

/// The most characters a kept line of output has. A longer one, such as
/// minified test output or a progress bar, is cut, so one check can't make
/// the item file hundreds of kilobytes long.
pub const KEPT_LINE_CHARS: usize = 500;

/// How much of the end of a check's output is held while it runs, in
/// bytes. Only the last `KEPT_LINES` lines are kept, so a check that
/// prints a lot never fills memory.
const TAIL_BYTES: usize = 64 * 1024;

/// How often a running check is looked at, to see whether it has ended or
/// run out of time.
const POLL: Duration = Duration::from_millis(20);

/// How long the rest of a check's output is waited for once its process
/// group is stopped. Only a process that left the group can hold the output
/// open past then.
const LEFTOVER_OUTPUT: Duration = Duration::from_secs(1);

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

    // The output is read on a thread of its own, into `tail`, which it
    // shares, so the wait for it below can be cut short. Its end is seen
    // when `finished` closes, as the thread drops it.
    let tail = Arc::new(Mutex::new(Vec::new()));
    let (finished, ended) = mpsc::channel::<()>();
    let reading = Arc::clone(&tail);
    thread::spawn(move || {
        let _finished = finished;
        let mut buffer = [0; 8192];
        while let Ok(n) = reader.read(&mut buffer) {
            if n == 0 {
                break;
            }
            let mut tail = reading.lock().unwrap_or_else(|e| e.into_inner());
            tail.extend_from_slice(&buffer[..n]);
            if tail.len() > 2 * TAIL_BYTES {
                let cut = tail.len() - TAIL_BYTES;
                tail.drain(..cut);
            }
        }
    });

    let (status, timed_out) = wait(&mut child, began + timeout);
    // Anything the check left running, such as a server it started in the
    // background, would hold the output open, so it's stopped too.
    stop_group(&child);
    // A process that left the group, with `setsid`, isn't stopped, and can
    // hold the output open for ever, so the rest of the output is waited
    // for only briefly, and the run goes on with what was read.
    let _ = ended.recv_timeout(LEFTOVER_OUTPUT);
    let tail = tail.lock().unwrap_or_else(|e| e.into_inner()).clone();
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
pub fn wait(child: &mut Child, deadline: Instant) -> (Option<std::process::ExitStatus>, bool) {
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
pub fn stop_group(child: &Child) {
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
/// A line longer than `KEPT_LINE_CHARS` is cut to that length, ending in
/// `…`.
pub fn last_lines(output: &[u8]) -> String {
    let text = String::from_utf8_lossy(output);
    let lines: Vec<&str> = text.trim_end().lines().collect();
    let start = lines.len().saturating_sub(KEPT_LINES);
    let kept: Vec<String> = lines[start..].iter().map(|line| cut(line)).collect();
    kept.join("\n")
}

/// `line`, cut to `KEPT_LINE_CHARS` characters, the last of them `…`, if
/// it's longer than that.
fn cut(line: &str) -> String {
    if line.chars().count() <= KEPT_LINE_CHARS {
        return line.to_string();
    }
    let mut cut: String = line.chars().take(KEPT_LINE_CHARS - 1).collect();
    cut.push('…');
    cut
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

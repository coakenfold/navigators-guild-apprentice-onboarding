//! The gates (§2.4): the rules an item must pass before it can start or be
//! submitted. A gate never changes anything. It reports every reason it
//! failed, and the command that ran it refuses with exit 2.
//!
//! This version has G1 (at `start`), and G2, G6, G3 and G4 (at `submit`,
//! and in the `gates` dry run). G3 and G4 run checks (`checks.rs`). Slice 8
//! adds G5, which reads git.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Value, json};

use crate::checks;
use crate::config::Config;
use crate::error::Error;
use crate::item::{Item, Status, is_valid_id};
use crate::states;
use crate::store::Skipped;

/// One reason a gate failed.
#[derive(Debug, Clone, PartialEq)]
pub struct Failure {
    /// Which gate, such as `G2`.
    pub gate: &'static str,
    /// The criterion it's about, numbered from 1, for a failure that's
    /// about one criterion.
    pub criterion: Option<usize>,
    /// What's wrong, and how to fix it, on one line.
    pub message: String,
    /// For a check that failed, its run: the command, exit code and last
    /// lines of output.
    pub check: Option<checks::Run>,
}

/// What a run of the gates found: the gates that ran, in order, every
/// reason any of them failed, and every check they ran.
#[derive(Debug, Default)]
pub struct Report {
    pub ran: Vec<&'static str>,
    pub failures: Vec<Failure>,
    pub checks: Vec<Ran>,
}

/// One check a gate ran: G3 for a criterion's, G4 for a project's.
#[derive(Debug, Clone, PartialEq)]
pub struct Ran {
    pub gate: &'static str,
    pub run: checks::Run,
}

/// Which command ran the gates, for what a refusal says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Run {
    Start,
    Submit,
    /// `tracker gates`, which changes nothing either way.
    DryRun,
}

impl Report {
    pub fn passed(&self) -> bool {
        self.failures.is_empty()
    }

    /// Adds what `other` found after what this one did.
    pub fn extend(&mut self, other: Report) {
        self.ran.extend(other.ran);
        self.failures.extend(other.failures);
        self.checks.extend(other.checks);
    }

    /// The gates that failed, each once, in the order they ran.
    fn failed(&self) -> Vec<String> {
        self.ran
            .iter()
            .filter(|gate| self.failures.iter().any(|f| f.gate == **gate))
            .map(|gate| gate.to_string())
            .collect()
    }

    /// Every gate that ran, with whether it passed, for `--json` (§3).
    pub fn gates_json(&self) -> Value {
        let gates: Vec<Value> = self
            .ran
            .iter()
            .map(|gate| {
                let ok = !self.failures.iter().any(|f| f.gate == *gate);
                let mut object = json!({ "gate": gate, "ok": ok });
                if matches!(*gate, "G3" | "G4") {
                    let secs: u64 = self
                        .checks
                        .iter()
                        .filter(|ran| ran.gate == *gate)
                        .map(|ran| ran.run.secs)
                        .sum();
                    object["secs"] = json!(secs);
                }
                object
            })
            .collect();
        Value::Array(gates)
    }

    /// Every reason a gate failed, for `--json`.
    fn failures_json(&self) -> Value {
        let failures: Vec<Value> = self
            .failures
            .iter()
            .map(|failure| {
                let mut object = json!({ "gate": failure.gate });
                if let Some(n) = failure.criterion {
                    object["criterion"] = json!(n);
                }
                object["message"] = json!(failure.message);
                if let Some(run) = &failure.check {
                    object["cmd"] = json!(run.cmd);
                    object["exit_code"] = json!(run.exit_code);
                    object["timed_out"] = json!(run.timed_out);
                    object["output"] = json!(run.output);
                }
                object
            })
            .collect();
        Value::Array(failures)
    }

    /// The refusal, exit 2, for item `id`, which is in state `status`: one
    /// line saying which gates failed, then every reason, as text below it
    /// and as `failures` in `--json`, with `id`, `status`, `result` and
    /// `gates` (§3). A dry run on an item that can't be submitted says so
    /// under the reasons, since fixing them wouldn't be enough.
    pub fn refusal(&self, id: &str, status: Status, run: Run) -> Error {
        let failed = states::join(&self.failed(), "and");
        let message = match run {
            Run::Start => format!("{id} can't be started: {failed} failed. Nothing was changed."),
            Run::Submit => {
                format!("{id} can't be submitted: {failed} failed. Nothing was changed.")
            }
            Run::DryRun => format!(
                "{id} would be refused at submit: {failed} failed. This was a dry run, so \
                 nothing was changed."
            ),
        };
        let fields = vec![
            ("id".to_string(), json!(id)),
            ("status".to_string(), json!(status.as_str())),
            ("result".to_string(), json!("refused")),
            ("gates".to_string(), self.gates_json()),
            ("failures".to_string(), self.failures_json()),
        ];
        // A failed check's last lines of output go under its reason, one
        // per line, indented past the gate.
        let mut details = Vec::new();
        for failure in &self.failures {
            details.push(format!("{}  {}", failure.gate, failure.message));
            if let Some(run) = &failure.check {
                for line in run.output.lines() {
                    details.push(format!("      {line}"));
                }
            }
        }
        if run == Run::DryRun
            && let Some(line) = not_submittable(status)
        {
            details.push(line);
        }
        Error::gate(message, fields, details)
    }
}

/// For the `gates` dry run: why an item in `status` can't be submitted
/// whatever the gates say, or `None` for an item in progress.
pub fn not_submittable(status: Status) -> Option<String> {
    if status == Status::InProgress {
        return None;
    }
    Some(format!(
        "It's {}, and only an in-progress item can be submitted.",
        status.phrase()
    ))
}

/// The gates `start` runs: G1 (§2.4). `items` is every item in the store,
/// which G1 looks through for another item in the same project that's in
/// progress or in review (§4.4), and `skipped` every file in `items/` that
/// can't be read as an item. One whose name puts it in the project, such
/// as `XX-1.json`, may be an item in progress, so it fails G1 too.
pub fn start(item: &Item, items: &[Item], skipped: &[Skipped]) -> Report {
    let mut failures = Vec::new();
    let id = &item.id;
    if item.criteria.is_empty() {
        failures.push(no_criteria(item, "G1"));
    }
    let busy = items.iter().filter(|other| {
        other.project == item.project
            && other.id != item.id
            && matches!(other.status, Status::InProgress | Status::Review)
    });
    for other in busy {
        let project = &item.project;
        let message = if other.status == Status::Review {
            format!(
                "{} is in review in {project}, waiting for a human to approve or reject it, and \
                 a project works on one item at a time. {id} can start once {} is decided.",
                other.id, other.id
            )
        } else {
            format!(
                "{} is in progress in {project}, and a project works on one item at a time. \
                 Submit or shelve {} first.",
                other.id, other.id
            )
        };
        failures.push(Failure {
            gate: "G1",
            criterion: None,
            message,
            check: None,
        });
    }
    for problem in skipped {
        // The file's name is checked against the ID it holds, so it says
        // which project the file would be in. A file not named after an
        // item ID isn't in any project.
        let named = problem
            .file
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default();
        let in_project = named
            .split_once('-')
            .is_some_and(|(key, _)| key == item.project);
        if !is_valid_id(named) || !in_project || named == item.id {
            continue;
        }
        failures.push(Failure {
            gate: "G1",
            criterion: None,
            message: format!(
                "Can't tell whether {named} is in progress or in review, since its file can't \
                 be read as an item: {} {}",
                problem.describe(),
                problem.fix
            ),
            check: None,
        });
    }
    Report {
        ran: vec!["G1"],
        failures,
        checks: Vec::new(),
    }
}

/// The gates `submit` and `gates` run first, since they read only files
/// (§2.4): G2, then G6. `scripts` runs the rest.
pub fn statics(item: &Item, config: &Config) -> Report {
    let mut failures = g2(item);
    failures.extend(g6(item, config));
    Report {
        ran: vec!["G2", "G6"],
        failures,
        checks: Vec::new(),
    }
}

/// The gates that run checks, after `statics` (§2.4): G3 runs each
/// criterion's check, and G4 each of the project's checks, in the project's
/// root. Every check runs, so the refusal lists every one that fails.
pub fn scripts(item: &Item, config: &Config) -> Report {
    let mut report = Report {
        ran: vec!["G3", "G4"],
        ..Report::default()
    };
    for (n, run) in criterion_runs(item, config) {
        if !run.ok {
            report
                .failures
                .push(check_failure(item, config, "G3", Some(n), &run));
        }
        report.checks.push(Ran { gate: "G3", run });
    }

    let key = &item.project;
    let Some(project) = config.projects.get(key) else {
        // Its checks aren't known, so G4 can't pass.
        report.failures.push(Failure {
            gate: "G4",
            criterion: None,
            message: format!(
                "{key} isn't in config.json, so its project checks aren't known. Register {key} \
                 again with `tracker project add`."
            ),
            check: None,
        });
        return report;
    };
    let root = root(item, config);
    for cmd in &project.checks {
        let run = run_in(cmd, &root, project.check_timeout(), "project check");
        if !run.ok {
            report
                .failures
                .push(check_failure(item, config, "G4", None, &run));
        }
        report.checks.push(Ran { gate: "G4", run });
    }
    report
}

/// Each criterion that has a check, numbered from 1, with its check.
pub fn criterion_checks(item: &Item) -> Vec<(usize, &str)> {
    item.criteria
        .iter()
        .enumerate()
        .filter_map(|(i, criterion)| criterion.check.as_deref().map(|check| (i + 1, check)))
        .collect()
}

/// Runs each criterion's check once, in the project's root, for G3 and for
/// `start`. A check that can't be run, because the root can't be found,
/// is a run that failed, with `error` saying why.
pub fn criterion_runs(item: &Item, config: &Config) -> Vec<(usize, checks::Run)> {
    let checks = criterion_checks(item);
    if checks.is_empty() {
        return Vec::new();
    }
    let root = root(item, config);
    let timeout = timeout(item, config);
    checks
        .into_iter()
        .map(|(n, cmd)| {
            let what = format!("criterion {n}'s check");
            (n, run_in(cmd, &root, timeout, &what))
        })
        .collect()
}

/// Runs criterion `n`'s check, `cmd`, once in the project's root, as
/// `criterion_runs` runs each one. For a check given after `start` (§2.4).
pub fn criterion_run(item: &Item, config: &Config, n: usize, cmd: &str) -> checks::Run {
    let what = format!("criterion {n}'s check");
    run_in(cmd, &root(item, config), timeout(item, config), &what)
}

/// Runs `cmd` in `root`, or, if the root can't be found, a run that failed
/// saying why, without running anything.
fn run_in(cmd: &str, root: &Result<PathBuf, String>, timeout: Duration, what: &str) -> checks::Run {
    match root {
        Ok(root) => checks::run(cmd, root, timeout, what),
        Err(why) => checks::not_run(cmd, why),
    }
}

/// The project's root, where its checks run, followed through symlinks,
/// or what's wrong with it.
fn root(item: &Item, config: &Config) -> Result<PathBuf, String> {
    let key = &item.project;
    let Some(project) = config.projects.get(key) else {
        return Err(format!(
            "{key} isn't in config.json, so there's no root to run it in. Register {key} again \
             with `tracker project add`"
        ));
    };
    match fs::canonicalize(project.root_path()) {
        Ok(root) if root.is_dir() => Ok(root),
        Ok(_) => Err(format!("{key}'s root, {}, isn't a folder", project.root)),
        Err(e) => Err(format!(
            "{key}'s root, {}, can't be read: {e}",
            project.root
        )),
    }
}

/// How long one of the item's project's checks may run (§4.2).
fn timeout(item: &Item, config: &Config) -> Duration {
    config.projects.get(&item.project).map_or(
        Duration::from_secs(crate::config::DEFAULT_CHECK_TIMEOUT_SECS),
        |project| project.check_timeout(),
    )
}

/// The failure, from `gate`, for a check that didn't pass: its command, how
/// it ended, and where its last lines of output are. `criterion` is the
/// criterion it belongs to, for G3.
fn check_failure(
    item: &Item,
    config: &Config,
    gate: &'static str,
    criterion: Option<usize>,
    run: &checks::Run,
) -> Failure {
    let what = match criterion {
        Some(n) => format!("Criterion {n}'s check"),
        None => "A project check".to_string(),
    };
    let cmd = &run.cmd;
    let key = &item.project;
    let mut message = if run.timed_out {
        format!(
            "{what}, `{cmd}`, ran longer than {}s, so it was stopped and fails. Make it faster, \
             or raise check_timeout_secs for {key} in config.json.",
            timeout(item, config).as_secs()
        )
    } else if let Some(why) = &run.error {
        format!("{what}, `{cmd}`, couldn't be run: {why}.")
    } else {
        let how = match (run.exit_code, run.signal) {
            (Some(code), _) => format!("exit {code}"),
            (None, Some(signal)) => format!("stopped by signal {signal}"),
            (None, None) => "no exit code".to_string(),
        };
        format!("{what} failed: `{cmd}` ({how}).")
    };
    if run.error.is_none() {
        if run.output.is_empty() {
            message.push_str(" It printed nothing.");
        } else {
            message.push_str(" Its last lines of output follow.");
        }
    }
    Failure {
        gate,
        criterion,
        message,
        check: Some(run.clone()),
    }
}

/// The failure, from `gate`, for an item with no criteria: nothing says
/// when it's done.
fn no_criteria(item: &Item, gate: &'static str) -> Failure {
    let id = &item.id;
    Failure {
        gate,
        criterion: None,
        message: format!(
            "{id} has no criteria, so nothing says when it's done. Add one with `tracker \
             criterion add {id} \"…\" --check \"…\"`."
        ),
        check: None,
    }
}

/// G2: the item has a criterion, and every manual criterion has evidence,
/// a note saying where to look. A script criterion's own output is its
/// proof, so it needs none (§2.3). G1 needs a criterion at start, but one
/// can be removed afterwards, so G2 checks again.
fn g2(item: &Item) -> Vec<Failure> {
    let id = &item.id;
    if item.criteria.is_empty() {
        return vec![no_criteria(item, "G2")];
    }
    item.criteria
        .iter()
        .enumerate()
        .filter(|(_, criterion)| criterion.is_manual() && criterion.evidence.is_none())
        .map(|(i, criterion)| {
            let n = i + 1;
            Failure {
                gate: "G2",
                criterion: Some(n),
                message: format!(
                    "Criterion {n}, \"{}\", is manual and has no evidence. Record where to \
                     look with `tracker evidence {id} {n} \"…\"`, or give it a check with \
                     `tracker criterion set {id} {n} --check \"…\"`.",
                    criterion.text
                ),
                check: None,
            }
        })
        .collect()
}

/// G6: if the item names a design section, the file exists inside the
/// project's root, and the section is a numbered heading with something
/// under it. The file is read relative to the root, and refused if it
/// leads outside it once symlinks are followed (§7).
fn g6(item: &Item, config: &Config) -> Vec<Failure> {
    let Some(design) = &item.design else {
        return Vec::new();
    };
    match check_design(item, design, config) {
        Ok(()) => Vec::new(),
        Err(message) => vec![Failure {
            gate: "G6",
            criterion: None,
            message,
            check: None,
        }],
    }
}

/// `g6`'s checks, with what's wrong as the error.
fn check_design(item: &Item, design: &str, config: &Config) -> Result<(), String> {
    let id = &item.id;
    let key = &item.project;
    let fix = format!("Fix it with `tracker edit {id} --design …`.");
    let (file, section) = match design.split_once('#') {
        Some((file, section)) => (file, Some(section)),
        None => (design, None),
    };
    let Some(project) = config.projects.get(key) else {
        return Err(format!(
            "{key} isn't in config.json, so there's no root to read {id}'s design doc, \
             {file}, from. Register {key} again with `tracker project add`."
        ));
    };
    let root = fs::canonicalize(project.root_path())
        .map_err(|e| format!("Can't read {key}'s root, {}: {e}.", project.root))?;

    // Followed through every symlink, so a file or folder that leads
    // outside the root is caught, however the reference was written.
    let path = match fs::canonicalize(root.join(file)) {
        Ok(path) => path,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(format!(
                "{id}'s design names {file}, which doesn't exist in {key}'s root, {}. {fix}",
                root.display()
            ));
        }
        Err(e) => return Err(format!("Can't read {id}'s design doc, {file}: {e}.")),
    };
    if !path.starts_with(&root) {
        return Err(format!(
            "{id}'s design names {file}, which leads to {}, outside {key}'s root, {}. G6 reads \
             only inside the root. {fix}",
            path.display(),
            root.display()
        ));
    }
    let text = read_design(&path).map_err(|why| format!("{id}'s design doc, {file}, {why}"))?;

    let Some(section) = section else {
        return Ok(());
    };
    check_section(&text, section).map_err(|why| format!("{file} {why} {fix}"))
}

/// The text of the design doc at `path`. Only a regular file is read: a
/// pipe would make G6 wait forever.
fn read_design(path: &Path) -> Result<String, String> {
    match fs::metadata(path) {
        Ok(meta) if meta.is_file() => {}
        Ok(_) => return Err("isn't a regular file.".to_string()),
        Err(e) => return Err(format!("can't be read: {e}.")),
    }
    fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::InvalidData {
            "isn't UTF-8 text.".to_string()
        } else {
            format!("can't be read: {e}.")
        }
    })
}

/// Checks that markdown `text` has a numbered heading for `section`, such
/// as `### 2.4 Gates` for `2.4`, with something under it before the next
/// heading of the same level or higher. A trailing dot on either side is
/// ignored, so `## 1. Overview` is section `1`. Code is never a heading,
/// and a code block's fences aren't something under it, as `lines` says.
/// The error completes a sentence that starts with the file's name.
pub fn check_section(text: &str, section: &str) -> Result<(), String> {
    let wanted = section.trim();
    let wanted = wanted.strip_suffix('.').unwrap_or(wanted);
    if !is_section_number(wanted) {
        return Err(format!(
            "is named with the section \"{section}\", but G6 looks for a numbered heading, so \
             the section is a number, such as 2.4 for `### 2.4 Gates`."
        ));
    }

    let lines = lines(text);
    let mut found = false;
    for (i, line) in lines.iter().enumerate() {
        let Line::Heading(level, title) = *line else {
            continue;
        };
        if heading_number(title) != Some(wanted) {
            continue;
        }
        found = true;
        let has_content = lines[i + 1..]
            .iter()
            .take_while(|line| !matches!(line, Line::Heading(l, _) if *l <= level))
            .any(|line| matches!(line, Line::Content));
        if has_content {
            return Ok(());
        }
    }
    if found {
        Err(format!(
            "has a heading numbered {wanted}, but nothing under it, so it doesn't say anything \
             yet."
        ))
    } else {
        Err(format!(
            "has no heading numbered {wanted}, such as `## {wanted} …`."
        ))
    }
}

/// What one line of a design doc is, for G6.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Line<'a> {
    /// A heading outside code: its level and text.
    Heading(usize, &'a str),
    /// Something under a heading: text, or a line of code that isn't blank.
    Content,
    /// A blank line, or a fence that opens or closes a code block.
    Nothing,
}

/// What each line of `text` is, following CommonMark's rules for fenced
/// code blocks: a fence is three or more backticks or tildes, with at most
/// three spaces of indent, and only a fence of the same character, at
/// least as long and with nothing after it, closes the block. A block left
/// open runs to the end of the text. A line with four spaces of indent is
/// indented code, so `heading` never takes it.
fn lines(text: &str) -> Vec<Line<'_>> {
    // The open fence's character and length.
    let mut open: Option<(char, usize)> = None;
    let mut lines = Vec::new();
    for line in text.lines() {
        let fence = fence(line);
        let kind = if let Some((c, length)) = open {
            let closes = fence.is_some_and(|(d, n, info)| d == c && n >= length && info.is_empty());
            if closes {
                open = None;
                Line::Nothing
            } else if line.trim().is_empty() {
                Line::Nothing
            } else {
                Line::Content
            }
        } else if let Some((c, length, _)) = fence {
            open = Some((c, length));
            Line::Nothing
        } else if let Some((level, title)) = heading(line) {
            Line::Heading(level, title)
        } else if line.trim().is_empty() {
            Line::Nothing
        } else {
            Line::Content
        };
        lines.push(kind);
    }
    lines
}

/// A code fence's character, its length and the rest of the line, its info
/// string, trimmed: ```` ```rust ```` is `('`', 3, "rust")`.
fn fence(line: &str) -> Option<(char, usize, &str)> {
    let rest = without_indent(line)?;
    let c = rest.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let length = rest.chars().take_while(|x| *x == c).count();
    if length < 3 {
        return None;
    }
    // Both characters are one byte long.
    let info = &rest[length..];
    // A backtick fence's info string can't hold a backtick (CommonMark),
    // so a line like ```` ```a`b ```` is text, not a fence.
    if c == '`' && info.contains('`') {
        return None;
    }
    Some((c, length, info.trim()))
}

/// `line` without its indent, if that's at most three spaces. More, or a
/// tab, makes indented code, which is never a heading or a fence.
fn without_indent(line: &str) -> Option<&str> {
    let rest = line.trim_start_matches(' ');
    if line.len() - rest.len() > 3 || rest.starts_with('\t') {
        return None;
    }
    Some(rest)
}

/// A markdown heading's level and text: `### 2.4 Gates` is `(3, "2.4 Gates")`.
fn heading(line: &str) -> Option<(usize, &str)> {
    let line = without_indent(line)?;
    let level = line.chars().take_while(|c| *c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }
    let rest = &line[level..];
    if !rest.is_empty() && !rest.starts_with([' ', '\t']) {
        return None;
    }
    Some((level, rest.trim()))
}

/// The number that starts a heading's text, without a trailing dot:
/// `2.4` for `2.4 Gates`, and `1` for `1. Overview`.
fn heading_number(title: &str) -> Option<&str> {
    let first = title.split_whitespace().next()?;
    let number = first.strip_suffix('.').unwrap_or(first);
    is_section_number(number).then_some(number)
}

/// Whether `text` is a section number: digits, with single dots between
/// them, such as `2` or `2.3.1`.
fn is_section_number(text: &str) -> bool {
    !text.is_empty()
        && text
            .split('.')
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::check_section;

    const DOC: &str = "# Title\n\n## 1. Overview\n\nText.\n\n## 2. Concepts\n\n### 2.1 Items\n\n\
                       Items.\n\n### 2.2 Empty\n\n### 2.3 Code only\n\n```\n## 9 not a heading\n\
                       ```\n\n## 3. Stub\n";

    #[test]
    fn a_numbered_heading_with_content_passes() {
        for section in ["1", "1.", "2.1", "2", "2.3"] {
            assert_eq!(check_section(DOC, section), Ok(()), "{section}");
        }
    }

    #[test]
    fn a_missing_empty_or_unnumbered_section_fails() {
        for (section, why) in [
            ("2.9", "no heading numbered 2.9"),
            ("9", "no heading numbered 9"),
            ("2.2", "nothing under it"),
            ("3", "nothing under it"),
            ("gates", "the section is a number"),
            ("", "the section is a number"),
            ("2..1", "the section is a number"),
        ] {
            let error = check_section(DOC, section).unwrap_err();
            assert!(error.contains(why), "{section}: {error}");
        }
    }

    #[test]
    fn code_is_never_a_heading_and_an_empty_code_block_says_nothing() {
        for (doc, section, why) in [
            // A closing fence has no info string, so ```rust doesn't close
            // the block, and the heading after it is code.
            (
                "```\ncode\n```rust\n## 2. Inside the fence\ntext\n```\n",
                "2",
                "no heading numbered 2",
            ),
            // Nor does a shorter fence, or one of the other character.
            (
                "````\n```\n~~~~\n## 2. Inside the fence\ntext\n````\n",
                "2",
                "no heading numbered 2",
            ),
            // Four spaces of indent, or a tab, make a code block.
            (
                "    ## 3. Indented code\n    text\n",
                "3",
                "no heading numbered 3",
            ),
            (
                "\t## 3. Indented code\n\ttext\n",
                "3",
                "no heading numbered 3",
            ),
            // A code block's fences say nothing, with or without an info
            // string, and neither does a blank line inside it.
            (
                "## 4. Empty code block\n```\n```\n",
                "4",
                "nothing under it",
            ),
            (
                "## 4. Empty code block\n~~~rust\n\n~~~\n",
                "4",
                "nothing under it",
            ),
        ] {
            let error = check_section(doc, section).unwrap_err();
            assert!(error.contains(why), "{doc:?}: {error}");
        }
    }

    #[test]
    fn headings_and_fences_may_have_up_to_three_spaces_of_indent() {
        for doc in [
            "   ## 2. Indented three spaces\ntext\n",
            "## 2. Code under it\n   ```\n   code\n   ```\n",
            // A longer closing fence closes the block, and so does one with
            // spaces after it.
            "```\ncode\n`````\n## 2. After the block\ntext\n",
            "~~~\ncode\n~~~  \n## 2. After the block\ntext\n",
            // A fence of the other character is code inside the block.
            "~~~\n```\n~~~\n## 2. After the block\ntext\n",
        ] {
            assert_eq!(check_section(doc, "2"), Ok(()), "{doc:?}");
        }
    }
}

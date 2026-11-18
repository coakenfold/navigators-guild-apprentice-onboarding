//! The gates (§2.4): the rules an item must pass before it can start or be
//! submitted. A gate never changes anything. It reports every reason it
//! failed, and the command that ran it refuses with exit 2.
//!
//! This version has G1 (at `start`), G2 and G6 (at `submit`, and in the
//! `gates` dry run). Slice 7 adds G3 and G4, which run checks, and slice 8
//! adds G5, which reads git.

use std::fs;
use std::path::Path;

use serde_json::{Value, json};

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
}

/// What a run of the gates found: the gates that ran, in order, and every
/// reason any of them failed.
#[derive(Debug, Default)]
pub struct Report {
    pub ran: Vec<&'static str>,
    pub failures: Vec<Failure>,
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
                json!({ "gate": gate, "ok": ok })
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
                object
            })
            .collect();
        Value::Array(failures)
    }

    /// The refusal, exit 2, for item `id`: one line saying which gates
    /// failed, then every reason, as text below it and as `failures` in
    /// `--json`, with `id`, `result` and `gates` (§3).
    pub fn refusal(&self, id: &str, run: Run) -> Error {
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
            ("result".to_string(), json!("refused")),
            ("gates".to_string(), self.gates_json()),
            ("failures".to_string(), self.failures_json()),
        ];
        let details = self
            .failures
            .iter()
            .map(|failure| format!("{}  {}", failure.gate, failure.message))
            .collect();
        Error::gate(message, fields, details)
    }
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
        failures.push(Failure {
            gate: "G1",
            criterion: None,
            message: format!(
                "{id} has no criteria, so nothing says when it's done. Add one with `tracker \
                 criterion add {id} \"…\" --check \"…\"`."
            ),
        });
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
        });
    }
    Report {
        ran: vec!["G1"],
        failures,
    }
}

/// The gates `submit` and `gates` run, cheapest first (§2.4): G2, then G6.
pub fn submit(item: &Item, config: &Config) -> Report {
    let mut failures = g2(item);
    failures.extend(g6(item, config));
    Report {
        ran: vec!["G2", "G6"],
        failures,
    }
}

/// G2: every manual criterion has evidence, a note saying where to look.
/// A script criterion's own output is its proof, so it needs none (§2.3).
fn g2(item: &Item) -> Vec<Failure> {
    let id = &item.id;
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

//! The review triggers (§2.5): once every gate has passed, they decide
//! whether `submit` moves an item straight to done or holds it in review
//! for a human. A trigger never refuses: it only holds.
//!
//! This version has T1 (a new capability), T2 (risky paths), T3 (a
//! dependency manifest), T4 (an existing test changed), T5 (the goalposts
//! moved), T6 (a manual criterion), T7 (a large change), T8 (the design doc
//! changed), T9 (the `needs-review` label), T10 (a judge answered unsure,
//! or the project doesn't auto-verify judged work), T11 (judges failed
//! `max_failures` different requests since start), T13 (a check or judge
//! passed before the work) and T14 (unscoped commits since start). T1 to T4, T7 and T8 read
//! the item's changed files (§2.1), and T14 reads the commits made since
//! start. A trigger in the project's `review.disabled` doesn't run, except
//! T6 and T10, which always do.

use std::collections::BTreeSet;
use std::path::Path;

use crate::config::{self, Config, Review};
use crate::error::{Error, Result};
use crate::git;
use crate::item::{Criterion, Hold, Item, Kind};
use crate::judge::{Answered, FailedRequest, Outcome};
use crate::states;
use sha2::{Digest, Sha256};

/// The label that asks for a human's review (T9).
pub const NEEDS_REVIEW: &str = "needs-review";

/// One trigger that ran: its name, such as `T6`, and, if it fired, why,
/// with the files that fired it, relative to the project's root.
#[derive(Debug, Clone, PartialEq)]
pub struct Ran {
    pub trigger: &'static str,
    pub hold: Option<String>,
    pub files: Vec<String>,
}

/// One commit's change to a file that existed before the item's work.
#[derive(Debug, Clone)]
pub struct Edit {
    pub path: String,
    /// Whether the commit deleted the file.
    pub gone: bool,
    /// The commit's short SHA.
    pub sha: String,
    /// Whether the commit was made after `start`.
    pub after: bool,
}

/// What the triggers read from git: the item's changed files (§2.1), as
/// the item's commits left them.
#[derive(Debug, Default)]
pub struct Changes {
    /// Every changed file under the root, relative to it, deleted ones
    /// included.
    pub files: BTreeSet<String>,
    /// The changed files under the root that an item commit added.
    pub added: BTreeSet<String>,
    /// The files under the root that existed before the item's commits
    /// and that one of them changed a line of, or removed a line from,
    /// or deleted: one entry for each commit that did.
    pub existing_changed: Vec<Edit>,
    /// Lines added and removed by every item commit, inside the root or
    /// not (T7).
    pub lines: u64,
}

/// Works out item's changed files from its commits in the repository
/// that holds `root` (§2.1). Paths are made relative to the root, and a
/// file outside it counts only toward `lines`.
pub fn changes(root: &Path, id: &str, started: Option<&str>) -> Result<Changes> {
    let prefix = git::prefix(root)?;
    let mut shas = git::item_commits(root, id)?;
    shas.reverse();
    // The commits made after `start`. A test file counts as existing for
    // one of them if it exists where the item started, whoever added it.
    let after = match started {
        Some(head) => git::since(root, head)?,
        None => Vec::new(),
    };
    let mut changes = Changes::default();
    for change in git::changes(root, &shas)? {
        if let Some((added, removed)) = change.lines {
            changes.lines += added + removed;
        }
        let Some(path) = change.path.strip_prefix(&prefix) else {
            continue;
        };
        let path = path.to_string();
        let existed = match started {
            Some(head) if after.contains(&change.sha) => git::exists_at(root, head, &change.path),
            _ => !changes.added.contains(&path),
        };
        let edit = |gone| Edit {
            path: path.clone(),
            gone,
            sha: change.sha.chars().take(7).collect(),
            after: after.contains(&change.sha),
        };
        match change.status {
            'A' => {
                changes.added.insert(path.clone());
            }
            'D' if existed => changes.existing_changed.push(edit(true)),
            // A binary file has no line counts, so any change to one counts.
            _ if existed && change.lines.is_none_or(|(_, removed)| removed > 0) => {
                changes.existing_changed.push(edit(false))
            }
            _ => {}
        }
        changes.files.insert(path);
    }
    Ok(changes)
}

/// Runs every trigger on `item` that its project doesn't switch off, in
/// order, and returns each one. `judges` is every judge question G3 asked
/// (T10), and `failed` every request judges failed since start (T11).
/// Exits 3 if git can't say what the item's commits changed.
pub fn run(
    item: &Item,
    config: &Config,
    judges: &[Answered],
    failed: &[FailedRequest],
) -> Result<Vec<Ran>> {
    let project = config.projects.get(&item.project);
    let review = project.map(|project| project.review()).unwrap_or_default();
    let design = project.and_then(|project| project.design.clone());
    let root = crate::gates::root(item, config).map_err(|why| {
        Error::store(format!(
            "Can't work out {}'s changed files: {why}.",
            item.id
        ))
    })?;
    let head = item.started.as_ref().and_then(|s| s.head.as_deref());
    let changes = changes(&root, &item.id, head)?;
    // An item with no `started`, written before slice 7, has no start to
    // count commits from.
    let projects: Vec<String> = config.projects.keys().cloned().collect();
    let unscoped = match &item.started {
        Some(_) => git::unscoped(&root, &item.id, head, &projects)?,
        None => Vec::new(),
    };
    let globs = Globs::new(&review)?;
    let mut ran = Vec::new();
    let mut add = |trigger: &'static str, (hold, files): (Option<String>, Vec<String>)| {
        if review.runs(trigger) {
            ran.push(Ran {
                trigger,
                hold,
                files,
            });
        }
    };
    add("T1", t1(item, &changes, &globs));
    add("T2", t2(&changes, &globs));
    add("T3", t3(&changes, &globs));
    add("T4", t4(&changes, &globs));
    add("T5", (t5(item), Vec::new()));
    add("T6", (t6(item), Vec::new()));
    add("T7", (t7(&changes, review.max_diff_lines()), Vec::new()));
    add("T8", t8(&changes, design.as_deref()));
    add("T9", (t9(item), Vec::new()));
    let auto_verify = project.is_none_or(|project| project.auto_verify());
    add("T10", (t10(judges, auto_verify), Vec::new()));
    let max_failures = project.map_or(config::DEFAULT_MAX_FAILURES, |p| p.max_failures());
    add("T11", (t11(failed, max_failures), Vec::new()));
    add("T13", (t13(item), Vec::new()));
    add("T14", (t14(&item.id, &unscoped), Vec::new()));
    Ok(ran)
}

/// The fingerprint `start` records in `started.criteria_hash` (T5): a
/// SHA-256, in lower-case hex, of the criteria, kind and design. Each
/// criterion is serialized without its evidence, which can only be written
/// once the work exists, so recording it doesn't move the goalposts. Fields
/// a later slice adds to a criterion, such as a judge's, are covered as
/// they're serialized, and leave the fingerprint of a criterion without
/// them unchanged.
pub fn fingerprint(item: &Item) -> String {
    let criteria: Vec<Criterion> = item
        .criteria
        .iter()
        .map(|criterion| Criterion {
            evidence: None,
            ..criterion.clone()
        })
        .collect();
    let value = serde_json::json!({
        "criteria": criteria,
        "kind": item.kind,
        "design": item.design,
    });
    Sha256::digest(value.to_string().as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The holds among `ran`: the triggers that fired, with why.
pub fn holds(ran: &[Ran]) -> Vec<Hold> {
    ran.iter()
        .filter_map(|ran| {
            ran.hold.as_ref().map(|message| Hold {
                trigger: ran.trigger.to_string(),
                message: message.clone(),
                files: ran.files.clone(),
            })
        })
        .collect()
}

/// The project's `review` globs, ready to match.
struct Globs {
    risky: globset::GlobSet,
    source: globset::GlobSet,
    test: globset::GlobSet,
    manifests: globset::GlobSet,
}

impl Globs {
    fn new(review: &Review) -> Result<Globs> {
        let set = |globs: Vec<String>| {
            config::glob_set(&globs).map_err(|why| {
                Error::store(format!(
                    "A review setting in config.json can't be used: {why}."
                ))
            })
        };
        Ok(Globs {
            risky: set(review.risky_paths())?,
            source: set(review.source_paths())?,
            test: set(review.test_paths())?,
            manifests: set(review.manifests())?,
        })
    }

    /// Whether `path` is a source file: it matches `source_paths` and not
    /// `test_paths` (§2.1).
    fn is_source(&self, path: &str) -> bool {
        self.source.is_match(path) && !self.test.is_match(path)
    }
}

/// `files` named in a message, as "a, b and c", with at most
/// `git::LISTED_FILES` of them.
fn file_list(files: &[String]) -> String {
    let mut named: Vec<String> = files.iter().take(git::LISTED_FILES).cloned().collect();
    if files.len() > named.len() {
        named.push(format!("{} more", files.len() - named.len()));
    }
    states::join(&named, "and")
}

/// "file" or "files", for `count` of them.
fn files_word(count: usize) -> &'static str {
    if count == 1 { "file" } else { "files" }
}

/// T1: the item is a capability, or one of its commits added a source
/// file, so new behaviour needs a human look at the design fit.
fn t1(item: &Item, changes: &Changes, globs: &Globs) -> (Option<String>, Vec<String>) {
    let added: Vec<String> = changes
        .added
        .iter()
        .filter(|path| globs.is_source(path))
        .cloned()
        .collect();
    let capability = item.kind == Kind::Capability;
    let message = match (capability, added.is_empty()) {
        (false, true) => None,
        (true, true) => Some("it's a capability, so a human should check the design fit".into()),
        (false, false) => Some(format!(
            "new source {}: {}",
            files_word(added.len()),
            file_list(&added)
        )),
        (true, false) => Some(format!(
            "it's a capability, and adds the source {} {}",
            files_word(added.len()),
            file_list(&added)
        )),
    };
    (message, added)
}

/// T2: a changed file matches the project's `risky_paths`.
fn t2(changes: &Changes, globs: &Globs) -> (Option<String>, Vec<String>) {
    let risky: Vec<String> = changes
        .files
        .iter()
        .filter(|path| globs.risky.is_match(path))
        .cloned()
        .collect();
    let message = (!risky.is_empty()).then(|| {
        format!(
            "risky {} changed: {}",
            files_word(risky.len()),
            file_list(&risky)
        )
    });
    (message, risky)
}

/// T3: a dependency manifest or lockfile changed.
fn t3(changes: &Changes, globs: &Globs) -> (Option<String>, Vec<String>) {
    let manifests: Vec<String> = changes
        .files
        .iter()
        .filter(|path| globs.manifests.is_match(path))
        .cloned()
        .collect();
    let message = (!manifests.is_empty()).then(|| {
        format!(
            "dependency {} changed: {}",
            if manifests.len() == 1 {
                "manifest"
            } else {
                "manifests"
            },
            file_list(&manifests)
        )
    });
    (message, manifests)
}

/// T4: an existing test file had lines changed or removed, or was
/// deleted. Adding a test file, or only adding lines to one, doesn't count.
fn t4(changes: &Changes, globs: &Globs) -> (Option<String>, Vec<String>) {
    let mut modified: Vec<String> = Vec::new();
    let mut deleted: Vec<String> = Vec::new();
    for edit in &changes.existing_changed {
        if !globs.test.is_match(&edit.path) {
            continue;
        }
        let list = if edit.gone {
            &mut deleted
        } else {
            &mut modified
        };
        if !list.contains(&edit.path) {
            list.push(edit.path.clone());
        }
    }
    modified.retain(|path| !deleted.contains(path));
    // A file with the commits that changed it, each marked before or after
    // `start`, such as `tests/a.rs (b7ba4ee before start, 1a2b3c4 after start)`.
    let named = |files: &[String]| -> Vec<String> {
        files
            .iter()
            .map(|path| {
                let commits: Vec<String> = changes
                    .existing_changed
                    .iter()
                    .filter(|edit| edit.path == *path)
                    .map(|edit| {
                        let when = if edit.after { "after" } else { "before" };
                        format!("{} {when} start", edit.sha)
                    })
                    .collect();
                format!("{path} ({})", commits.join(", "))
            })
            .collect()
    };
    let mut parts = Vec::new();
    if !modified.is_empty() {
        parts.push(format!(
            "existing test {} modified: {}",
            files_word(modified.len()),
            file_list(&named(&modified))
        ));
    }
    if !deleted.is_empty() {
        parts.push(format!(
            "existing test {} deleted: {}",
            files_word(deleted.len()),
            file_list(&named(&deleted))
        ));
    }
    let files: Vec<String> = modified.into_iter().chain(deleted).collect();
    ((!parts.is_empty()).then(|| parts.join("; ")), files)
}

/// T5: the criteria, kind or design changed after start, so the
/// fingerprint `start` took no longer matches. Or, at any time, a text in
/// `written` is no longer any criterion's text, which names it (§2.1): a
/// criterion I wrote was reworded or removed. An item started before
/// slice 11 has no fingerprint, so only `written` is compared.
fn t5(item: &Item) -> Option<String> {
    let mut parts = Vec::new();
    let taken = item.started.as_ref().and_then(|s| s.criteria_hash.as_ref());
    if taken.is_some_and(|hash| *hash != fingerprint(item)) {
        parts.push(
            "the criteria, their checks, the kind or the design changed after start".to_string(),
        );
    }
    let gone: Vec<String> = item
        .written
        .iter()
        .filter(|text| !item.criteria.iter().any(|c| c.text == **text))
        .map(|text| format!("\"{text}\""))
        .collect();
    match gone.len() {
        0 => {}
        1 => parts.push(format!(
            "a written criterion is no longer any criterion's text: {}",
            gone[0]
        )),
        _ => parts.push(format!(
            "written criteria are no longer any criterion's text: {}",
            states::join(&gone, "and")
        )),
    }
    (!parts.is_empty()).then(|| parts.join("; "))
}

/// T6: a criterion has neither a script check nor a judge, so only a
/// human can check its evidence.
fn t6(item: &Item) -> Option<String> {
    let manual: Vec<String> = item
        .criteria
        .iter()
        .enumerate()
        .filter(|(_, criterion)| criterion.is_manual())
        .map(|(i, _)| (i + 1).to_string())
        .collect();
    match manual.len() {
        0 => None,
        1 => Some(format!(
            "criterion {} is manual, so only a human can check its evidence",
            manual[0]
        )),
        _ => Some(format!(
            "criteria {} are manual, so only a human can check their evidence",
            states::join(&manual, "and")
        )),
    }
}

/// T7: the item's commits changed more lines than `max_diff_lines`.
fn t7(changes: &Changes, max: u64) -> Option<String> {
    (changes.lines > max).then(|| {
        format!(
            "{} lines changed, more than the project's max_diff_lines of {max}",
            changes.lines
        )
    })
}

/// T8: the project's design doc is among the changed files.
fn t8(changes: &Changes, design: Option<&str>) -> (Option<String>, Vec<String>) {
    let Some(design) = design.map(|design| design.split('#').next().unwrap_or(design)) else {
        return (None, Vec::new());
    };
    let design = design.trim_start_matches("./");
    if !changes.files.contains(design) {
        return (None, Vec::new());
    }
    (
        Some(format!("the project's design doc changed: {design}")),
        vec![design.to_string()],
    )
}

/// T9: the item has the `needs-review` label, which asks for a human.
fn t9(item: &Item) -> Option<String> {
    item.labels
        .iter()
        .any(|label| label == NEEDS_REVIEW)
        .then(|| format!("it has the label {NEEDS_REVIEW}, which asks for a human's review"))
}

/// T10: a judge answered unsure, or the project's judge settings have
/// `auto_verify` false and a judge graded the item (§2.5).
fn t10(judges: &[Answered], auto_verify: bool) -> Option<String> {
    let unsure: Vec<String> = judges
        .iter()
        .filter(|answered| answered.outcome() == Outcome::Unsure)
        .map(|answered| answered.judge_ref.clone())
        .collect();
    if let [one] = unsure.as_slice() {
        let answered = judges
            .iter()
            .find(|answered| &answered.judge_ref == one)
            .expect("found above");
        return Some(format!(
            "a judge answered unsure to {one}, so a human decides: {}",
            answered.reason
        ));
    }
    if !unsure.is_empty() {
        return Some(format!(
            "judges answered unsure to {} questions, so a human decides: {}",
            unsure.len(),
            file_list(&unsure)
        ));
    }
    (!auto_verify && !judges.is_empty()).then(|| {
        "the project's judge settings have auto_verify false, so an item a judge passed waits \
         for a human"
            .to_string()
    })
}

/// T11: judges failed `max` or more different requests since start
/// (§2.5), so the work may have been changed until a judge passed. Each
/// request is counted once, and fails at start don't count.
fn t11(failed: &[FailedRequest], max: u64) -> Option<String> {
    if failed.is_empty() || (failed.len() as u64) < max {
        return None;
    }
    let refs: Vec<String> = failed.iter().map(|f| f.refs.join(", ")).collect();
    Some(format!(
        "judges failed {} different requests since start, and judge.max_failures is {max}, so the \
         work may have been changed until a judge passed: {}",
        failed.len(),
        file_list(&refs)
    ))
}

/// T13: a criterion's script check already passed when it ran before the
/// work, at `start` or when it was given after start (§2.4), or every
/// answer of its judge did at `start`, so it can't be proving the work. A
/// criterion with no entry in `started.checks`, such as one given a check
/// while the item was in review, which isn't run then, or a judge after
/// start, counts as one that didn't pass. The change that gave it the
/// check or judge fired T5, which holds the item anyway.
fn t13(item: &Item) -> Option<String> {
    let started = item.started.as_ref()?;
    let passed: Vec<String> = started
        .checks
        .iter()
        .filter(|check| check.passed)
        .map(|check| {
            let judge = item
                .criteria
                .get(check.criterion.wrapping_sub(1))
                .is_some_and(|criterion| criterion.is_judge());
            let what = if judge { "judge" } else { "check" };
            format!("criterion {}'s {what}", check.criterion)
        })
        .collect();
    match passed.len() {
        0 => None,
        1 => Some(format!(
            "{} already passed before the work, so it can't be proving it",
            passed[0]
        )),
        _ => Some(format!(
            "{} already passed before the work, so they can't be proving it",
            states::join(&passed, "and")
        )),
    }
}

/// T14: commits made since start that touch the project's root aren't
/// scoped to item `id`, so the other triggers can't see them as the item's
/// (§2.5). `unscoped` holds each one's SHA and header, newest first, and
/// the hold names them oldest first.
fn t14(id: &str, unscoped: &[(String, String)]) -> Option<String> {
    if unscoped.is_empty() {
        return None;
    }
    let named: Vec<String> = unscoped
        .iter()
        .rev()
        .map(|(sha, header)| format!("{} \"{header}\"", crate::gates::short(sha)))
        .collect();
    Some(format!(
        "{} since start, not scoped to {id}: {}",
        if named.len() == 1 {
            "an unscoped commit".to_string()
        } else {
            format!("{} unscoped commits", named.len())
        },
        file_list(&named)
    ))
}

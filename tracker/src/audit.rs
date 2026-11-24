//! `audit` (§3): signs that the store was edited by hand. Every change goes
//! through a `tracker` command, which keeps an item's status, history,
//! verification record and git ref in step, and `deleted.json` below its
//! project's counter, so a mismatch was written some other way (§7).
//!
//! It doesn't read `judge/failures.json` or `judge/cache/`, so a T11 count
//! or a cached answer changed by hand isn't reported (§7).

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::config::Config;
use crate::digest::key_of;
use crate::gates;
use crate::git;
use crate::item::{self, Item, Status, VerifiedBy};

/// One thing `audit` found.
#[derive(Debug, Serialize)]
pub struct Problem {
    /// What kind: `status`, `verification`, `ref`, `held`, `id` or `deleted`.
    pub problem: &'static str,
    /// The item or `deleted.json` line it's about, if it names one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub message: String,
}

fn problem(kind: &'static str, id: &str, message: String) -> Problem {
    Problem {
        problem: kind,
        id: Some(id.to_string()),
        message,
    }
}

/// Every problem in `items` and in `deleted`, the text of `deleted.json`,
/// checked against `counters`, items first by project and number, then
/// `deleted.json`'s lines in order.
pub fn run(
    items: &[Item],
    deleted: &str,
    counters: &BTreeMap<String, u64>,
    config: &Config,
) -> Vec<Problem> {
    let mut sorted: Vec<&Item> = items.iter().collect();
    sorted.sort_by(|a, b| a.project.cmp(&b.project).then(a.number().cmp(&b.number())));
    let mut problems = Vec::new();
    for item in sorted {
        check_item(item, config, &mut problems);
    }
    let ids: BTreeSet<&str> = items.iter().map(|item| item.id.as_str()).collect();
    check_deleted(deleted, &ids, counters, &mut problems);
    problems
}

fn check_item(item: &Item, config: &Config, problems: &mut Vec<Problem>) {
    let id = &item.id;
    // Every move adds a history entry, so the last one's `to` is the state.
    // An item with no history has never moved from open.
    let moved_to = item.history.last().map_or(Status::Open, |entry| entry.to);
    if item.status != moved_to {
        problems.push(problem(
            "status",
            id,
            format!(
                "{id} is {}, but the last move in its history left it {}, so its status was \
                 edited by hand.",
                item.status.as_str(),
                moved_to.as_str()
            ),
        ));
    }

    if item.status == Status::Done {
        match &item.verification {
            None => problems.push(problem(
                "verification",
                id,
                format!(
                    "{id} is done but has no verification record, so nothing says how it \
                     reached done."
                ),
            )),
            Some(record) if record.by != VerifiedBy::Unchecked => {
                if let Some(message) = ref_problem(item, record.commit.as_deref(), config) {
                    problems.push(problem("ref", id, message));
                }
            }
            Some(_) => {}
        }
    }

    // Holds and held questions are there only while an item waits in
    // review (§4.3), and a T12 hold comes with its disputed questions.
    let has_holds = !item.holds.is_empty() || !item.held_questions.is_empty();
    if item.status != Status::Review && has_holds {
        problems.push(problem(
            "held",
            id,
            format!(
                "{id} is {} but has holds or held questions, which only an item in review has.",
                item.status.as_str()
            ),
        ));
    }
    let t12_hold = item.holds.iter().any(|hold| hold.trigger == "T12");
    let t12_question = item.held_questions.iter().any(|q| q.trigger == "T12");
    if item.status == Status::Review && t12_hold != t12_question {
        let message = if t12_hold {
            format!("{id} is held under T12, but has no disputed question.")
        } else {
            format!("{id} has a disputed question, but isn't held under T12.")
        };
        problems.push(problem("held", id, message));
    }
}

/// What's wrong with a verified item's `refs/tracker/<ID>` (§2.7), or `None`
/// if it points at the record's `commit`.
fn ref_problem(item: &Item, commit: Option<&str>, config: &Config) -> Option<String> {
    let id = &item.id;
    let name = git::ref_name(id);
    let Some(commit) = commit else {
        return Some(format!(
            "{id}'s verification record names no commit, so {name} can't be checked."
        ));
    };
    let root = match gates::root(item, config) {
        Ok(root) => root,
        Err(why) => return Some(format!("{name} can't be checked: {why}.")),
    };
    match git::ref_target(&root, id) {
        Ok(Some(target)) if target == commit => None,
        Ok(Some(target)) => Some(format!(
            "{name} points at {}, not at {}, the commit {id}'s record says was verified.",
            gates::short(&target),
            gates::short(commit)
        )),
        Ok(None) => Some(format!(
            "{name} is missing, so nothing keeps {}, the commit {id} was verified at, from \
             git's garbage collection.",
            gates::short(commit)
        )),
        Err(err) => Some(format!("{name} can't be checked: {}", err.message)),
    }
}

fn check_deleted(
    deleted: &str,
    ids: &BTreeSet<&str>,
    counters: &BTreeMap<String, u64>,
    problems: &mut Vec<Problem>,
) {
    // Each ID's first line, by line number.
    let mut first_line: BTreeMap<String, usize> = BTreeMap::new();
    for (index, line) in deleted.lines().enumerate() {
        let n = index + 1;
        if line.trim().is_empty() {
            continue;
        }
        let id = serde_json::from_str::<serde_json::Value>(line)
            .ok()
            .and_then(|record| record["id"].as_str().map(str::to_string))
            .filter(|id| item::is_valid_id(id));
        let Some(id) = id else {
            problems.push(Problem {
                problem: "deleted",
                id: None,
                message: format!(
                    "Line {n} of deleted.json can't be read as a delete record with an item ID."
                ),
            });
            continue;
        };

        if ids.contains(id.as_str()) {
            problems.push(problem(
                "id",
                &id,
                format!(
                    "{id} is an item, and line {n} of deleted.json says it was deleted, so the \
                     ID was used twice."
                ),
            ));
        }
        match first_line.get(&id) {
            Some(first) => problems.push(problem(
                "id",
                &id,
                format!(
                    "Lines {first} and {n} of deleted.json both delete {id}. A delete is \
                     recorded once, so one of them was written by hand."
                ),
            )),
            None => {
                first_line.insert(id.clone(), n);
            }
        }

        let key = key_of(&id);
        let number: u64 = id
            .rsplit_once('-')
            .map_or(0, |(_, n)| n.parse().unwrap_or(0));
        let counter = counters.get(key).copied().unwrap_or(1);
        if number >= counter {
            problems.push(problem(
                "deleted",
                &id,
                format!(
                    "Line {n} of deleted.json deletes {id}, but counters.json's next {key} \
                     number is {counter}, so {id} was never handed out and the line was \
                     written by hand. `tracker create` refuses {key} until its counter in \
                     counters.json is above {number}."
                ),
            ));
        }
    }
}

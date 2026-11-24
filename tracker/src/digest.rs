//! Each project's digest (§2.6): every item auto-verified since I last
//! acknowledged it, and every shelve, human action and edit that switches
//! off a trigger, each marked by whether it came from a terminal. `digest
//! ack` records what I've seen in `digests.json` (§2.7).

use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{self, Result};
use crate::item::{HeldQuestion, Item, Kind, Status, Verification, VerifiedBy, Via};
use crate::store::{self, Store};

/// One project's entry in `digests.json`: when I acknowledged its digest,
/// whether that came from a terminal, and how much it listed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ack {
    pub at: Timestamp,
    pub interactive: bool,
    pub verified: usize,
    pub actions: usize,
}

/// One project's digest, as `digest --json` prints it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Digest {
    pub project: String,
    /// When it was last acknowledged, or `None` if it never was. Everything
    /// listed is from after it.
    pub acknowledged: Option<Timestamp>,
    /// The last ack as recorded, so an ack I didn't make stands out.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_ack: Option<Ack>,
    /// Auto-verified items, the ones a judge passed first.
    pub verified: Vec<Verified>,
    /// Shelves, human actions and trigger-switching edits, oldest first.
    pub actions: Vec<Action>,
}

/// An item auto-verified since the ack.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Verified {
    pub id: String,
    pub title: String,
    /// Whether a judge passed it, rather than only scripts.
    pub judged: bool,
    pub criteria: Vec<String>,
    /// Each question a judge was asked, from the item's criteria.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub questions: Vec<Asked>,
    pub verification: Verification,
}

/// A judge question that was asked, by its ref such as `c1:works`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Asked {
    #[serde(rename = "ref")]
    pub judge_ref: String,
    pub ask: String,
}

/// A shelve, a human action or an edit that switched off a trigger.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Action {
    pub id: String,
    pub title: String,
    /// `shelve`, `approve`, `reject`, `criterion-accept`, `delete` or `edit`.
    pub action: String,
    pub at: Timestamp,
    pub interactive: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// For a delete, the state the item was in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// For a shelve of an item with a judge criterion: starting it again
    /// empties its T11 count (§7), so the pair is worth a look.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub judged: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub labels_removed: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind_before: Option<Kind>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub held_questions: Vec<HeldQuestion>,
}

impl Digest {
    /// How many items and actions it lists.
    pub fn entries(&self) -> usize {
        self.verified.len() + self.actions.len()
    }

    /// When its oldest entry happened, or `None` for an empty digest.
    pub fn oldest(&self) -> Option<Timestamp> {
        let verified = self.verified.iter().map(|entry| entry.verification.at);
        let actions = self.actions.iter().map(|action| action.at);
        verified.chain(actions).min()
    }
}

/// The acks in `digests.json`, by project key. Empty before the first ack.
pub fn read_acks(store: &Store) -> Result<BTreeMap<String, Ack>> {
    let path = store.digests_path();
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(e) => return Err(error::io("read", &path, &e)),
    };
    serde_json::from_str(&text).map_err(|e| {
        error::parse(
            &path,
            &e,
            "It maps each project key to its last ack, such as {\"BM\": {\"at\": \
             \"2026-11-20T10:00:00Z\", \"interactive\": true, \"verified\": 2, \"actions\": 1}}. \
             Only `tracker digest ack` writes it, so a human should check how it changed. \
             Removing a project's entry lists everything again.",
        )
    })
}

/// Writes `digests.json`. Call this only while holding the lock.
pub fn write_acks(store: &Store, acks: &BTreeMap<String, Ack>) -> Result<()> {
    let text = serde_json::to_string_pretty(acks).expect("acks always serialize");
    store::write_file(&store.digests_path(), &format!("{text}\n"))
}

/// The lines of `deleted.json` that can be read as delete records, as JSON.
/// The others are left out here, and `audit` reports them.
pub fn deleted_records(store: &Store) -> Result<Vec<Value>> {
    let text = store.read_deleted()?;
    Ok(text
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|record| record.is_object())
        .collect())
}

/// The project key of an item ID, such as `BM` for `BM-12`.
pub fn key_of(id: &str) -> &str {
    id.rsplit_once('-').map_or(id, |(key, _)| key)
}

/// Builds `project`'s digest from every item, `deleted.json`'s records and
/// the project's last ack.
pub fn build(project: &str, items: &[Item], deleted: &[Value], ack: Option<&Ack>) -> Digest {
    let since = |at: Timestamp| ack.is_none_or(|ack| at > ack.at);
    let mut items: Vec<&Item> = items
        .iter()
        .filter(|item| item.project == project)
        .collect();
    items.sort_by_key(|item| item.number());

    let mut verified = Vec::new();
    let mut actions = Vec::new();
    for item in &items {
        if item.status == Status::Done
            && let Some(record) = &item.verification
            && record.by == VerifiedBy::Auto
            && since(record.at)
        {
            verified.push(Verified {
                id: item.id.clone(),
                title: item.title.clone(),
                judged: !record.judges.is_empty(),
                criteria: item.criteria.iter().map(|c| c.text.clone()).collect(),
                questions: asked(item, record),
                verification: record.clone(),
            });
        }
        let judged = item.criteria.iter().any(|c| c.is_judge());
        for entry in item.history.iter().filter(|entry| since(entry.at)) {
            let listed = match entry.via {
                Via::Shelve | Via::Approve | Via::Reject | Via::CriterionAccept => true,
                Via::Edit => {
                    entry
                        .labels_removed
                        .iter()
                        .any(|label| label == "needs-review")
                        || entry.kind_before == Some(Kind::Capability)
                }
                _ => false,
            };
            if !listed {
                continue;
            }
            actions.push(Action {
                id: item.id.clone(),
                title: item.title.clone(),
                action: entry.via.as_str().to_string(),
                at: entry.at,
                interactive: entry.interactive,
                reason: entry.reason.clone(),
                status: None,
                judged: (entry.via == Via::Shelve && judged).then_some(true),
                labels_removed: entry.labels_removed.clone(),
                kind_before: entry.kind_before,
                held_questions: entry.held_questions.clone(),
            });
        }
    }

    for record in deleted {
        let Some(id) = record["id"].as_str() else {
            continue;
        };
        let Some(at) = record["at"]
            .as_str()
            .and_then(|at| at.parse::<Timestamp>().ok())
        else {
            continue;
        };
        if key_of(id) != project || !since(at) {
            continue;
        }
        actions.push(Action {
            id: id.to_string(),
            title: record["title"].as_str().unwrap_or_default().to_string(),
            action: "delete".to_string(),
            at,
            interactive: record["interactive"].as_bool().unwrap_or(false),
            reason: None,
            status: record["status"].as_str().map(str::to_string),
            judged: None,
            labels_removed: Vec::new(),
            kind_before: None,
            held_questions: Vec::new(),
        });
    }

    // Judge-passed first (§2.6), then oldest first. Sorts are stable, so
    // entries at the same second keep the items' order.
    verified.sort_by_key(|entry| (!entry.judged, entry.verification.at));
    actions.sort_by_key(|action| action.at);
    Digest {
        project: project.to_string(),
        acknowledged: ack.map(|ack| ack.at),
        last_ack: ack.cloned(),
        verified,
        actions,
    }
}

/// The question each of the record's judge answers was for, found by its
/// ref `c<n>:<id>`, or `c<n>:<at>:<id>` for an `each` judge, in the item's criteria. An answer whose question is gone
/// from the item is left out.
fn asked(item: &Item, record: &Verification) -> Vec<Asked> {
    let mut out = Vec::new();
    for judge in &record.judges {
        let Some((front, id)) = judge.judge_ref.rsplit_once(':') else {
            continue;
        };
        let criterion = front.split(':').next().unwrap_or(front);
        let Some(n) = criterion
            .strip_prefix('c')
            .and_then(|n| n.parse::<usize>().ok())
        else {
            continue;
        };
        let question = n
            .checked_sub(1)
            .and_then(|n| item.criteria.get(n))
            .and_then(|c| c.ask.iter().find(|q| q.id == id));
        if let Some(question) = question {
            out.push(Asked {
                judge_ref: judge.judge_ref.clone(),
                ask: question.ask.clone(),
            });
        }
    }
    out
}

/// Whether `digest` is overdue: its oldest entry is at least `every_days`
/// old at `now` (§2.6). An empty digest never is.
pub fn is_overdue(digest: &Digest, every_days: u64, now: Timestamp) -> bool {
    let Some(oldest) = digest.oldest() else {
        return false;
    };
    let age = now.as_second() - oldest.as_second();
    age >= i64::try_from(every_days.saturating_mul(86_400)).unwrap_or(i64::MAX)
}

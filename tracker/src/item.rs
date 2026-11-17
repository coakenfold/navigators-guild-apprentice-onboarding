//! The item file (§4.3): the serde structs, in the file's fixed field order.
//!
//! Fields added by later slices get `#[serde(default, skip_serializing_if =
//! …)]`, so files written by an earlier version still load and come back
//! byte-for-byte unchanged.

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub title: String,
    pub project: String,
    pub kind: Kind,
    pub status: Status,
    pub priority: Priority,
    /// Free text, lowercased and de-duplicated (§2.1).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
    /// A reference into a design doc, such as `DESIGN.md#2.4`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub design: Option<String>,
    pub created: Timestamp,
    pub updated: Timestamp,
    /// Filled in when the item reaches done (§2.7), and present only while
    /// it's done: reopening moves it to `previous_verification`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<Verification>,
    /// The record of the item's last verification, from before it was last
    /// reopened. Each reopen from done replaces it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_verification: Option<Verification>,
    /// One entry per move between states (§2.2) or `edit`, oldest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub history: Vec<HistoryEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// How an item reached done (§2.1, §2.7). Later slices add the commit, its
/// ref, and the checks and judges that ran.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Verification {
    pub by: VerifiedBy,
    pub at: Timestamp,
    /// Why the record says what it does, such as that no gates ran.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VerifiedBy {
    /// The gates passed and no trigger fired.
    Auto,
    /// I approved it from the inbox.
    Human,
    /// It reached done before the gates existed, so nothing checked it.
    Unchecked,
}

impl VerifiedBy {
    pub const ALL: [VerifiedBy; 3] = [VerifiedBy::Auto, VerifiedBy::Human, VerifiedBy::Unchecked];

    /// The name used on the command line, in item files and in `--json`.
    pub fn as_str(self) -> &'static str {
        match self {
            VerifiedBy::Auto => "auto",
            VerifiedBy::Human => "human",
            VerifiedBy::Unchecked => "unchecked",
        }
    }

    /// The way named `text`, such as `auto`.
    pub fn parse(text: &str) -> Option<VerifiedBy> {
        VerifiedBy::ALL.into_iter().find(|by| by.as_str() == text)
    }
}

/// One move between states, or one `edit`, which has `from` equal to `to`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub at: Timestamp,
    pub from: Status,
    pub to: Status,
    /// The command that made the move. `status <id> <state>` records the
    /// command it maps onto.
    pub via: Via,
    /// Whether stdin was a terminal, which is how the digest tells my moves
    /// from an agent's (§2.6).
    pub interactive: bool,
    /// Why, for the moves that need a reason: shelve and reopen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// For an entry that doesn't change the state, such as an `edit`, the
    /// names of the fields it changed (§2.1).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<String>,
}

/// The commands that add a history entry: the ones that move an item
/// between states (§2.2, §3), and `edit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Via {
    Start,
    Submit,
    Shelve,
    Reopen,
    /// Changes an item's details without moving it, so no move has it.
    Edit,
    /// A command this version doesn't know, such as `approve`, in a file
    /// written by a newer tracker. The file still loads and lists, but the
    /// command's name is lost, so `Store::update_item` refuses to write the
    /// file back. No move has it.
    #[serde(other)]
    Unknown,
}

impl Via {
    /// The command's name, as typed and as item files record it.
    pub fn as_str(self) -> &'static str {
        match self {
            Via::Start => "start",
            Via::Submit => "submit",
            Via::Shelve => "shelve",
            Via::Reopen => "reopen",
            Via::Edit => "edit",
            Via::Unknown => "unknown",
        }
    }

    /// Whether the move needs `--reason` (§2.2).
    pub fn needs_reason(self) -> bool {
        matches!(self, Via::Shelve | Via::Reopen)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Fix,
    Change,
    /// Adds something the system couldn't do before.
    Capability,
    Chore,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::Fix, Kind::Change, Kind::Capability, Kind::Chore];

    /// The name used on the command line, in item files and in `--json`.
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Fix => "fix",
            Kind::Change => "change",
            Kind::Capability => "capability",
            Kind::Chore => "chore",
        }
    }

    /// The kind named `text`, such as `fix`.
    pub fn parse(text: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|kind| kind.as_str() == text)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Open,
    InProgress,
    Review,
    Done,
    Shelved,
}

impl Status {
    /// Every state, in the order the lifecycle reaches them (§2.2).
    pub const ALL: [Status; 5] = [
        Status::Open,
        Status::InProgress,
        Status::Review,
        Status::Done,
        Status::Shelved,
    ];

    /// The name used in item files and `--json` output.
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Open => "open",
            Status::InProgress => "in-progress",
            Status::Review => "review",
            Status::Done => "done",
            Status::Shelved => "shelved",
        }
    }

    /// The state named `text`, such as `in-progress`.
    pub fn parse(text: &str) -> Option<Status> {
        Status::ALL
            .into_iter()
            .find(|status| status.as_str() == text)
    }

    /// The state as it reads after "is": "open", or "in review".
    pub fn phrase(self) -> &'static str {
        match self {
            Status::Review => "in review",
            other => other.as_str(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    Medium,
    High,
}

impl Priority {
    /// Every priority, highest first, which is the order `list` sorts by.
    pub const ALL: [Priority; 3] = [Priority::High, Priority::Medium, Priority::Low];

    /// The name used on the command line, in item files and in `--json`.
    pub fn as_str(self) -> &'static str {
        match self {
            Priority::Low => "low",
            Priority::Medium => "medium",
            Priority::High => "high",
        }
    }

    /// The priority named `text`, such as `high`.
    pub fn parse(text: &str) -> Option<Priority> {
        Priority::ALL
            .into_iter()
            .find(|priority| priority.as_str() == text)
    }

    /// Where `list` puts it: 0 for high, which comes first.
    pub fn rank(self) -> usize {
        Priority::ALL
            .iter()
            .position(|priority| *priority == self)
            .expect("ALL holds every priority")
    }
}

impl Item {
    /// The number after the dash in the ID: 12 for `BM-12`.
    pub fn number(&self) -> u64 {
        self.id
            .rsplit_once('-')
            .and_then(|(_, n)| n.parse().ok())
            .unwrap_or(0)
    }
}

/// Whether `id` is an item ID: a project key, a dash and a number from 1
/// with no leading zeros, such as `BM-12`. Nothing else may become part of
/// an item file's path.
pub fn is_valid_id(id: &str) -> bool {
    let Some((key, number)) = id.split_once('-') else {
        return false;
    };
    crate::config::is_valid_key(key)
        && number.starts_with(|c: char| ('1'..='9').contains(&c))
        && number.chars().all(|c| c.is_ascii_digit())
        && number.parse::<u64>().is_ok()
}

/// The current time to the second, which is how item files record times.
pub fn now() -> Timestamp {
    Timestamp::from_second(Timestamp::now().as_second()).expect("the current time is in range")
}

/// An item file's text: pretty-printed with two-space indents, the fields in
/// struct order, and a final newline.
pub fn to_json(item: &Item) -> String {
    let text = serde_json::to_string_pretty(item).expect("items always serialize");
    format!("{text}\n")
}

pub fn from_json(text: &str) -> Result<Item, serde_json::Error> {
    serde_json::from_str(text)
}

/// The top-level fields of an item file that this version would lose or
/// change if it read the file and wrote it back: fields it doesn't know,
/// such as ones a newer tracker added, and values it would write
/// differently. Empty when a rewrite keeps everything.
pub fn lost_on_rewrite(text: &str) -> Result<Vec<String>, serde_json::Error> {
    let on_disk: serde_json::Value = serde_json::from_str(text)?;
    let rewritten = serde_json::to_value(from_json(text)?).expect("items always serialize");
    let mut lost = Vec::new();
    if let Some(fields) = on_disk.as_object() {
        for (name, value) in fields {
            if rewritten.get(name) != Some(value) {
                lost.push(name.clone());
            }
        }
    }
    Ok(lost)
}

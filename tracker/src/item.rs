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
    pub created: Timestamp,
    pub updated: Timestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Fix,
    Change,
    Capability,
    Chore,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    Medium,
    High,
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

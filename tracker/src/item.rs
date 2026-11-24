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
    /// The text of each criterion as it was written at `create` (§2.1), in
    /// no particular order. Only a `criterion` command run from an
    /// interactive terminal changes it afterwards, so T5 can
    /// tell when an agent reworded or dropped a criterion it was given. It
    /// keeps the text, not a hash, so `criterion accept` and T5 can say
    /// which.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub written: Vec<String>,
    /// The **Done when** list (§2.3), numbered from 1 in commands.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub criteria: Vec<Criterion>,
    /// What the last `start` found (§4.3). Each `start` replaces it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started: Option<Started>,
    /// Filled in when the item reaches done (§2.7), and present only while
    /// it's done: reopening moves it to `previous_verification`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<Verification>,
    /// The record of the item's last verification, from before it was last
    /// reopened. Each reopen from done replaces it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_verification: Option<Verification>,
    /// The triggers that fired at the last submit, while the item waits in
    /// review for a human to approve or reject it (§2.5). Present only
    /// while it's in review: approve moves them to its verification record,
    /// and reject clears them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub holds: Vec<Hold>,
    /// One entry per move between states (§2.2) or `edit`, oldest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub history: Vec<HistoryEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// One **Done when** criterion (§2.3): what must be true, and how it's
/// verified. A criterion with a `check` is a script criterion, and one with
/// questions in `ask` is a judge criterion (§2.3.1). One with neither is
/// manual, and only it has `evidence`.
///
/// The judge's fields come between `check` and `evidence`, and are left out
/// when they're not set, so a criterion without a judge is written, and
/// fingerprinted (T5), as it was before slice 12.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Criterion {
    pub text: String,
    /// A shell command, run in the project root, whose exit code 0 means
    /// the criterion holds. Slice 7 runs it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check: Option<String>,
    /// For a judge, the files it reads, relative to the project's root, or
    /// `diff` for the item's changes (§2.3.1).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<String>,
    /// For a judge, a regex: the questions are asked once per match, about
    /// the lines around it. Without it, the whole input is the excerpt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub each: Option<String>,
    /// For a judge with `each`, how many lines either side of a match the
    /// excerpt shows. The config's `judge.context_lines` when it's not set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_lines: Option<usize>,
    /// For a judge about comments: the excerpts keep them.
    #[serde(default, skip_serializing_if = "is_false")]
    pub keep_comments: bool,
    /// For a judge, its yes/no questions, each asked in its own call.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ask: Vec<Question>,
    /// For a manual criterion, a note saying where to look (G2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
}

fn is_false(value: &bool) -> bool {
    !value
}

impl Criterion {
    /// Whether nothing can run it, so only a human can verify it (T6).
    pub fn is_manual(&self) -> bool {
        self.check.is_none() && !self.is_judge()
    }

    /// Whether a judge verifies it (§2.3.1).
    pub fn is_judge(&self) -> bool {
        !self.ask.is_empty()
    }
}

/// One of a judge's yes/no questions (§2.3.1).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Question {
    /// Names the question in its ref, such as `suggests_next_step`.
    pub id: String,
    /// The question.
    pub ask: String,
    /// What a yes means, shown to the model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yes: Option<String>,
    /// What a no means, shown to the model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no: Option<String>,
    /// The answer that passes: yes when it's not set, or no for a question
    /// about a defect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pass_when: Option<Answer>,
}

impl Question {
    /// The answer that passes.
    pub fn passing(&self) -> Answer {
        self.pass_when.unwrap_or(Answer::Yes)
    }
}

/// A judge's answer to one question (§2.3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Answer {
    Yes,
    No,
    /// The excerpt doesn't show enough to decide, which holds the item
    /// (T10).
    Unsure,
}

impl Answer {
    pub fn as_str(self) -> &'static str {
        match self {
            Answer::Yes => "yes",
            Answer::No => "no",
            Answer::Unsure => "unsure",
        }
    }
}

/// What `start` recorded (§2.4, §4.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Started {
    pub at: Timestamp,
    /// A SHA-256 fingerprint, in hex, of the criteria, kind and design at
    /// start (T5), as `triggers::fingerprint` takes it. Left out by
    /// trackers before slice 11.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub criteria_hash: Option<String>,
    /// The commit `HEAD` pointed to at start, as a full SHA (T14). Left out
    /// for a repository with no commits yet, and by trackers before slice 8.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head: Option<String>,
    /// Each criterion check, run once at start, where failing is expected
    /// because the work isn't done yet (T13).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub checks: Vec<StartedCheck>,
}

/// Whether one criterion's check passed at start.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StartedCheck {
    /// The criterion, numbered from 1.
    pub criterion: usize,
    pub passed: bool,
}

/// How an item reached done (§2.1, §2.7).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Verification {
    pub by: VerifiedBy,
    pub at: Timestamp,
    /// The commit the checks ran against, as a full SHA: `HEAD` when G5
    /// looked. Left out of `unchecked` records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    /// The git ref that keeps `commit`, `refs/tracker/<ID>`, in the
    /// project's repository (§2.7).
    #[serde(default, rename = "ref", skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,
    /// Every check the submit ran, criterion checks first, then the
    /// project's, in order (§4.3).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub checks: Vec<CheckRecord>,
    /// Every judge question the submit asked, one entry per question and
    /// excerpt, in order (§4.3).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub judges: Vec<JudgeRecord>,
    /// For a `human` record, the triggers that held the item, which I
    /// accepted by approving it (§2.7). Empty for an `auto` one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub triggers: Vec<Hold>,
    /// Why the record says what it does, such as that no gates ran.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// One trigger that fired (§2.5): its name, such as `T6`, the message
/// saying why it holds the item, and the changed files that fired it, if
/// it's about files, which `show --diff` lists first.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hold {
    pub trigger: String,
    pub message: String,
    /// Each file's path relative to the project's root. Left out for a
    /// trigger that isn't about files, and in holds written before TR-6.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
}

/// One check in a verification record: the criterion it came from, its
/// command, whether it passed, how long it took, and its last lines of
/// output, left out if it printed nothing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckRecord {
    /// The criterion whose check it is, numbered from 1. Left out for a
    /// project check, and in records written before TR-16.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub criterion: Option<usize>,
    pub cmd: String,
    pub ok: bool,
    pub secs: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

/// One judge question in a verification record: its ref, the model that
/// answered, the fingerprint of the request it was asked in, which names
/// its cache entry, and the answer with its reason (§4.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JudgeRecord {
    #[serde(rename = "ref")]
    pub judge_ref: String,
    pub model: String,
    pub request: String,
    pub answer: Answer,
    pub reason: String,
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

/// One move between states, or one `edit` or `criterion accept`, which has
/// `from` equal to `to`.
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
    /// Why, for the moves that need a reason: shelve, reopen and reject.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// For an entry that doesn't change the state, such as an `edit`, the
    /// names of the fields it changed (§2.1). `criterion accept` names
    /// `written`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<String>,
    /// For an `edit` that changed `kind`, the kind it replaced, so the
    /// digest can list a move away from `capability`, which T1 relies on
    /// (§2.6).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind_before: Option<Kind>,
    /// For an `edit`, the labels it removed, so the digest can list one
    /// that removed `needs-review`, which T9 relies on (§2.6).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels_removed: Vec<String>,
}

/// The commands that add a history entry: the ones that move an item
/// between states (§2.2, §3), `edit` and `criterion accept`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Via {
    Start,
    Submit,
    /// Moves a held item to done, from my terminal (§2.7).
    Approve,
    /// Sends a held item back to in-progress, from my terminal (§2.7).
    Reject,
    Shelve,
    Reopen,
    /// Changes an item's details without moving it, so no move has it.
    Edit,
    /// Records the criteria's text in `written` (§2.7). It doesn't move
    /// the item, so no move has it.
    #[serde(rename = "criterion-accept")]
    CriterionAccept,
    /// A command this version doesn't know, such as `archive`, in a file
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
            Via::Approve => "approve",
            Via::Reject => "reject",
            Via::Shelve => "shelve",
            Via::Reopen => "reopen",
            Via::Edit => "edit",
            Via::CriterionAccept => "criterion-accept",
            Via::Unknown => "unknown",
        }
    }

    /// Whether the move needs `--reason` (§2.2).
    pub fn needs_reason(self) -> bool {
        matches!(self, Via::Shelve | Via::Reopen | Via::Reject)
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
/// change if it read the file and wrote it back, in two kinds.
#[derive(Debug, Default, PartialEq)]
pub struct Lost {
    /// Fields that hold something this version doesn't know, such as a
    /// field a newer tracker added, at the top level or inside a record, or
    /// a history entry made by a command it doesn't have. A rewrite would
    /// drop it, and `show` can't display it.
    pub unknown: Vec<String>,
    /// Fields whose values this version knows, but would only write
    /// differently: `[]` or `null`, which it leaves out, or a time with an
    /// offset such as `+00:00`, which it writes as the same instant ending
    /// in `Z`. These come from hand edits, and nothing is hidden.
    pub respelled: Vec<String>,
}

impl Lost {
    /// Whether a rewrite keeps the file exactly as it is.
    pub fn is_empty(&self) -> bool {
        self.unknown.is_empty() && self.respelled.is_empty()
    }

    /// Every field a rewrite would lose or change, the unknown ones first.
    pub fn fields(&self) -> Vec<String> {
        let mut fields = self.unknown.clone();
        fields.extend(self.respelled.iter().cloned());
        fields
    }
}

/// What an item file holds that this version would lose or change by
/// reading it and writing it back. Empty when a rewrite keeps everything.
pub fn lost_on_rewrite(text: &str) -> Result<Lost, serde_json::Error> {
    let on_disk: serde_json::Value = serde_json::from_str(text)?;
    let rewritten = serde_json::to_value(from_json(text)?).expect("items always serialize");
    let mut lost = Lost::default();
    if let Some(fields) = on_disk.as_object() {
        for (name, value) in fields {
            match difference(Some(value), rewritten.get(name)) {
                Difference::None => {}
                Difference::Spelling => lost.respelled.push(name.clone()),
                Difference::Meaning => lost.unknown.push(name.clone()),
            }
        }
    }
    Ok(lost)
}

/// How a value on disk differs from what a rewrite puts in its place,
/// least to most.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Difference {
    None,
    /// The same value, written another way.
    Spelling,
    /// Something this version doesn't know, which a rewrite would lose.
    Meaning,
}

/// Compares a value on disk with its rewrite, looking inside records and
/// lists so that one unknown field anywhere counts as `Meaning`. `None` on
/// either side means the field isn't there.
fn difference(
    on_disk: Option<&serde_json::Value>,
    rewritten: Option<&serde_json::Value>,
) -> Difference {
    use serde_json::Value;
    match (on_disk, rewritten) {
        (a, b) if a == b => Difference::None,
        // An empty list or a null, which a rewrite leaves out.
        (Some(value), None) | (None, Some(value)) if is_empty(value) => Difference::Spelling,
        (Some(Value::Object(a)), Some(Value::Object(b))) => a
            .keys()
            .chain(b.keys())
            .map(|key| difference(a.get(key), b.get(key)))
            .max()
            .unwrap_or(Difference::None),
        (Some(Value::Array(a)), Some(Value::Array(b))) if a.len() == b.len() => a
            .iter()
            .zip(b)
            .map(|(a, b)| difference(Some(a), Some(b)))
            .max()
            .unwrap_or(Difference::None),
        (Some(Value::String(a)), Some(Value::String(b))) if same_instant(a, b) => {
            Difference::Spelling
        }
        _ => Difference::Meaning,
    }
}

fn is_empty(value: &serde_json::Value) -> bool {
    value.is_null() || value.as_array().is_some_and(Vec::is_empty)
}

/// Whether two strings are both times, and the same instant.
fn same_instant(a: &str, b: &str) -> bool {
    match (a.parse::<Timestamp>(), b.parse::<Timestamp>()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

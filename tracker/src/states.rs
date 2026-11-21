//! The states and the moves between them (§2.2): which command moves an
//! item from which state to which, and what to say when a move isn't drawn.

use serde_json::{Value, json};

use crate::error::Error;
use crate::item::{Status, Via};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Move {
    pub via: Via,
    pub from: Status,
    pub to: Status,
}

const fn step(via: Via, from: Status, to: Status) -> Move {
    Move { via, from, to }
}

/// Every move in §2.2's diagram that this version has a command for, one
/// row per arrow. `allowed_json` lists one entry per row (§3), so a command
/// with two outcomes gets two rows. `submit`'s move to review, and
/// `approve` and `reject`, arrive with the triggers in slice 9, and `find`
/// must then stop assuming one row per command and state. `delete` isn't a
/// move: the item is gone.
pub const MOVES: [Move; 6] = [
    step(Via::Start, Status::Open, Status::InProgress),
    step(Via::Submit, Status::InProgress, Status::Done),
    step(Via::Shelve, Status::Open, Status::Shelved),
    step(Via::Shelve, Status::InProgress, Status::Shelved),
    step(Via::Reopen, Status::Done, Status::Open),
    step(Via::Reopen, Status::Shelved, Status::Open),
];

/// The move `via` makes from `from`, if the diagram draws one.
pub fn find(via: Via, from: Status) -> Option<Move> {
    MOVES
        .into_iter()
        .find(|step| step.via == via && step.from == from)
}

/// The command that `status <id> <to>` maps onto. `None` for review, which
/// only `submit` reaches, when a trigger holds the item.
pub fn via_to(to: Status) -> Option<Via> {
    match to {
        Status::Open => Some(Via::Reopen),
        Status::InProgress => Some(Via::Start),
        Status::Done => Some(Via::Submit),
        Status::Shelved => Some(Via::Shelve),
        Status::Review => None,
    }
}

/// How to run `via` on `id`, such as `tracker shelve XX-1 --reason "…"`.
pub fn command(id: &str, via: Via) -> String {
    if via.needs_reason() {
        format!("tracker {} {id} --reason \"…\"", via.as_str())
    } else {
        format!("tracker {} {id}", via.as_str())
    }
}

/// The moves allowed from `from`, for the `allowed` field of a refusal:
/// each with its command, the state it leads to, and how to run it on `id`.
/// Empty for review, which only approve and reject leave (slice 9).
pub fn allowed_json(id: &str, from: Status) -> Value {
    let moves: Vec<Value> = MOVES
        .iter()
        .filter(|step| step.from == from)
        .map(|step| {
            json!({
                "via": step.via.as_str(),
                "to": step.to.as_str(),
                "command": command(id, step.via),
            })
        })
        .collect();
    Value::Array(moves)
}

/// One sentence listing the moves allowed from `from`, as commands for `id`.
pub fn allowed(id: &str, from: Status) -> String {
    let moves: Vec<String> = MOVES
        .iter()
        .filter(|step| step.from == from)
        .map(|step| format!("`{}` (to {})", command(id, step.via), step.to.as_str()))
        .collect();
    if moves.is_empty() {
        // Only review has no moves yet.
        return "A held item waits for a human to approve or reject it, and this version of \
                tracker can't do either yet."
            .to_string();
    }
    if moves.len() == 1 {
        return format!("From {}, the only move is {}.", from.as_str(), moves[0]);
    }
    format!(
        "From {}, the allowed moves are {}.",
        from.as_str(),
        join(&moves, "and")
    )
}

/// The refusal for a move the diagram doesn't draw: exit 1, with
/// `message` and the moves allowed from `from`.
fn refusal(id: &str, from: Status, message: String) -> Error {
    let mut error = Error::input(message);
    error
        .fields
        .push(("allowed".to_string(), allowed_json(id, from)));
    error
}

/// The refusal for running `via` on an item in `from`, where the diagram
/// draws no such move.
pub fn not_from(id: &str, from: Status, via: Via) -> Error {
    let starts: Vec<String> = MOVES
        .iter()
        .filter(|step| step.via == via)
        .map(|step| step.from.phrase().to_string())
        .collect();
    let message = format!(
        "{id} is {}, and `{}` only moves an item that is {}. {}",
        from.phrase(),
        via.as_str(),
        join(&starts, "or"),
        allowed(id, from)
    );
    refusal(id, from, message)
}

/// The refusal for `status <id> <to>` on an item in `from`, where the
/// diagram draws no such move.
pub fn not_to(id: &str, from: Status, to: Status) -> Error {
    let what = if from == Status::Review {
        format!("{id} is in review, so `status` can't move it.")
    } else if from == to {
        format!("{id} is already {}.", to.phrase())
    } else if to == Status::Review {
        "No command moves an item straight to review. `submit` puts it there when it needs a \
         human."
            .to_string()
    } else {
        format!(
            "{id} is {}, and there's no move from {} to {}.",
            from.phrase(),
            from.as_str(),
            to.as_str()
        )
    };
    refusal(id, from, format!("{what} {}", allowed(id, from)))
}

/// "a", "a or b", "a, b or c", with `word` as "or" or "and".
pub fn join(parts: &[String], word: &str) -> String {
    match parts.split_last() {
        None => String::new(),
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} {word} {last}", rest.join(", ")),
    }
}

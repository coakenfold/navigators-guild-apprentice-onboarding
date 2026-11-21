//! The review triggers (§2.5): once every gate has passed, they decide
//! whether `submit` moves an item straight to done or holds it in review
//! for a human. A trigger never refuses: it only holds.
//!
//! This version has T6 (a manual criterion) and T9 (the `needs-review`
//! label). Slices 10 and 11 add the triggers that read the item's commits,
//! and with them the project's `review.disabled`, which never switches off
//! T6.

use crate::item::{Hold, Item};
use crate::states;

/// The label that asks for a human's review (T9).
pub const NEEDS_REVIEW: &str = "needs-review";

/// One trigger that ran: its name, such as `T6`, and, if it fired, why.
#[derive(Debug, Clone, PartialEq)]
pub struct Ran {
    pub trigger: &'static str,
    pub hold: Option<String>,
}

/// Runs every trigger on `item`, in order, and returns each one.
pub fn run(item: &Item) -> Vec<Ran> {
    vec![
        Ran {
            trigger: "T6",
            hold: t6(item),
        },
        Ran {
            trigger: "T9",
            hold: t9(item),
        },
    ]
}

/// The holds among `ran`: the triggers that fired, with why.
pub fn holds(ran: &[Ran]) -> Vec<Hold> {
    ran.iter()
        .filter_map(|ran| {
            ran.hold.as_ref().map(|message| Hold {
                trigger: ran.trigger.to_string(),
                message: message.clone(),
            })
        })
        .collect()
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

/// T9: the item has the `needs-review` label, which asks for a human.
fn t9(item: &Item) -> Option<String> {
    item.labels
        .iter()
        .any(|label| label == NEEDS_REVIEW)
        .then(|| format!("it has the label {NEEDS_REVIEW}, which asks for a human's review"))
}

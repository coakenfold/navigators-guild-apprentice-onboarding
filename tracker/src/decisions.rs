//! `tracker judges` (§2.3.2): my decisions on each judge question, read
//! from the `held_questions` of every `approve` and `reject` in every
//! item's history, which a reopen leaves alone. Only T10 and T12 holds
//! record questions, so a T11 hold isn't counted (§2.7).

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::item::{Answer, Item, Via};

/// One judge question within a project, and my decisions on the items it
/// held.
#[derive(Debug, Serialize)]
pub struct Question {
    pub project: String,
    /// The question's text, or `None` for decisions recorded before TR-53
    /// kept it, which are grouped by question id instead.
    pub ask: Option<String>,
    /// The question's id, from the first decision's ref.
    pub question: String,
    /// The counts for each trigger and answer that held an item.
    pub held: Vec<Held>,
}

/// How often I approved and rejected the items a question held with one
/// trigger and answer.
#[derive(Debug, Serialize)]
pub struct Held {
    pub trigger: String,
    pub answer: Answer,
    /// `disputed`, `unsure` or `passed`: what the trigger and answer mean.
    pub held_as: &'static str,
    pub approved: u64,
    pub rejected: u64,
}

/// What a held question's trigger and answer mean: a fail the agent
/// disputed (T12), an unsure answer (T10), or, under `"auto_verify":
/// false`, a passing answer (T10). Approving the first overrules the judge,
/// and approving the last agrees with it.
pub fn held_as(trigger: &str, answer: Answer) -> &'static str {
    if trigger == "T12" {
        "disputed"
    } else if answer == Answer::Unsure {
        "unsure"
    } else {
        "passed"
    }
}

/// Every question that held an item I approved or rejected, by project and
/// then text. Each decision counts once per question, trigger and answer,
/// however many of an `each` judge's matches it held, since §2.3.2 counts
/// the items a question held.
pub fn tally(items: &[Item]) -> Vec<Question> {
    let mut questions: BTreeMap<(String, String), Question> = BTreeMap::new();
    for item in items {
        for entry in &item.history {
            let approved = match entry.via {
                Via::Approve => true,
                Via::Reject => false,
                _ => continue,
            };
            let mut counted = BTreeSet::new();
            for held in &entry.held_questions {
                let id = held
                    .judge_ref
                    .rsplit(':')
                    .next()
                    .unwrap_or(&held.judge_ref)
                    .to_string();
                // A question with no text is grouped by its id, after a
                // character no text starts with, so it can't join one that
                // has its text.
                let group = if held.ask.is_empty() {
                    format!("\u{0}{id}")
                } else {
                    held.ask.clone()
                };
                let answer = held.answer;
                if !counted.insert((group.clone(), held.trigger.clone(), answer.as_str())) {
                    continue;
                }
                let question = questions
                    .entry((item.project.clone(), group))
                    .or_insert_with(|| Question {
                        project: item.project.clone(),
                        ask: (!held.ask.is_empty()).then(|| held.ask.clone()),
                        question: id,
                        held: Vec::new(),
                    });
                let at = match question
                    .held
                    .iter()
                    .position(|split| split.trigger == held.trigger && split.answer == answer)
                {
                    Some(at) => at,
                    None => {
                        question.held.push(Held {
                            trigger: held.trigger.clone(),
                            answer,
                            held_as: held_as(&held.trigger, answer),
                            approved: 0,
                            rejected: 0,
                        });
                        question.held.len() - 1
                    }
                };
                if approved {
                    question.held[at].approved += 1;
                } else {
                    question.held[at].rejected += 1;
                }
            }
        }
    }
    let mut questions: Vec<Question> = questions.into_values().collect();
    for question in &mut questions {
        question
            .held
            .sort_by(|a, b| (&a.trigger, a.answer.as_str()).cmp(&(&b.trigger, b.answer.as_str())));
    }
    questions
}

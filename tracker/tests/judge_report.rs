//! Slice 16 (TR-12): `tracker judges` (DESIGN.md §2.3.2, §3, §6). For each
//! judge question, grouped by its text within a project, how often I
//! approved and how often I rejected the items it held, split by the held
//! question's trigger and answer, so overruling the judge on a dispute is
//! counted apart from agreeing with it.
//!
//! Every decision is an `approve` or `reject`, which need a terminal, so
//! these tests run them under `script` and are skipped where it isn't
//! installed. The judge command is a fake that prints `reply.json`.
//!
//! The tests for each of TR-12's criteria are named after it, such as
//! `tr12_c6_`, so each criterion's check runs only its own.

mod common;

use std::fs;
use std::path::PathBuf;

use common::{Env, git_commit_all};
use serde_json::{Value, json};

const MODEL: &str = "claude-haiku-5-5";

/// The question every judge here asks, whatever its id.
const ASK: &str = "Does this code do the work?";

const FAKE: &str = "#!/bin/sh
cat > /dev/null
cat \"$(dirname \"$0\")/reply.json\"
";

fn judge_dir(env: &Env) -> PathBuf {
    env.work.path().join("judge")
}

fn edit_config(env: &Env, change: impl FnOnce(&mut Value)) {
    let path = env.home.path().join("config.json");
    let mut config: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    change(&mut config);
    fs::write(&path, serde_json::to_string_pretty(&config).unwrap()).unwrap();
}

/// Has the fake answer the question `works` with `answer`.
fn reply(env: &Env, answer: &str) {
    let reply = json!({
        "adapter": "fake 1",
        "model": MODEL,
        "answers": { "works": { "answer": answer, "reason": format!("The fake says {answer}.") } },
        "cost_usd": 0,
    });
    fs::write(judge_dir(env).join("reply.json"), reply.to_string()).unwrap();
}

/// Project XX, whose `src/lib.rs` holds a stub, with the fake as the judge
/// command, or `None` where `script` isn't installed.
fn env_with_judge() -> Option<Env> {
    let env = judge_env();
    env.in_terminal(&["--version"])?;
    Some(env)
}

/// `env_with_judge`, for a test that needs no terminal.
fn judge_env() -> Env {
    use std::os::unix::fs::PermissionsExt;
    let env = Env::new();
    env.add_project("XX", "xx");
    let dir = judge_dir(&env);
    fs::create_dir_all(&dir).unwrap();
    let fake = dir.join("fake");
    fs::write(&fake, FAKE).unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
    let command = fake.to_str().unwrap().to_string();
    edit_config(&env, |config| {
        config["judge"] = json!({ "command": command, "model": MODEL });
    });
    // An `each` judge of `step\(` has a match to grade at start.
    write_lib(&env, "fn run() {\n    step(0);\n}\n");
    git_commit_all(env.work.path(), "chore: the code before the work");
    env
}

fn write_lib(env: &Env, text: &str) {
    let lib = env.work.path().join("xx/src/lib.rs");
    fs::create_dir_all(lib.parent().unwrap()).unwrap();
    fs::write(lib, text).unwrap();
}

/// Creates an item in XX whose only criterion is `judge`, starts it while
/// the fake answers no, then commits version `n` of the work and has the
/// fake answer `answer`. Returns its ID.
fn started_with(env: &Env, judge: &Value, n: usize, answer: &str) -> String {
    let id = env.create_bare("XX", "judged");
    let file = judge_dir(env).join(format!("{id}.json"));
    fs::write(&file, judge.to_string()).unwrap();
    env.json(&["criterion", "add", &id, "--judge", file.to_str().unwrap()]);
    reply(env, "no");
    env.json(&["start", &id]);
    write_lib(
        env,
        &format!("fn run() {{\n    step({n}1);\n    step({n}2);\n}}\n"),
    );
    git_commit_all(env.work.path(), &format!("feat({id}): version {n}"));
    reply(env, answer);
    id
}

/// A judge that reads all of `src/lib.rs` and asks `ASK`.
fn whole_judge() -> Value {
    json!({
        "text": "The code does the work",
        "inputs": ["src/lib.rs"],
        "ask": [{ "id": "works", "ask": ASK }],
    })
}

/// `approve <id>` in a terminal, which must succeed.
fn approve(env: &Env, id: &str) {
    let (shown, ok) = env
        .in_terminal_typing(&["approve", id], "or anything else to cancel: ", id, || {})
        .unwrap();
    assert!(ok, "{shown}");
}

/// `reject <id>` in a terminal, which must succeed.
fn reject(env: &Env, id: &str) {
    let (shown, ok) = env
        .in_terminal_typing(
            &["reject", id, "--reason", "not yet"],
            "or anything else to cancel: ",
            id,
            || {},
        )
        .unwrap();
    assert!(ok, "{shown}");
}

/// `submit <id>`, disputing `c1:works`, which must hold it under T12.
fn dispute(env: &Env, id: &str) {
    let output = env.json(&[
        "submit",
        id,
        "--dispute",
        "c1:works",
        "--reason",
        "lib.rs:2 does the work",
    ]);
    assert_eq!(output["result"], "held", "{output}");
}

/// The one question `judges --json` reports.
fn only_question(env: &Env) -> Value {
    let output = env.json(&["judges"]);
    let questions = output["questions"].as_array().unwrap();
    assert_eq!(questions.len(), 1, "{output}");
    questions[0].clone()
}

/// The counts in `question` for what `trigger` held with `answer`, as
/// `(held_as, approved, rejected)`.
fn counts(question: &Value, trigger: &str, answer: &str) -> (String, u64, u64) {
    let held = question["held"]
        .as_array()
        .unwrap()
        .iter()
        .find(|held| held["trigger"] == trigger && held["answer"] == answer)
        .unwrap_or_else(|| panic!("nothing held under {trigger} {answer}: {question}"));
    (
        held["held_as"].as_str().unwrap().to_string(),
        held["approved"].as_u64().unwrap(),
        held["rejected"].as_u64().unwrap(),
    )
}

fn judges_text(env: &Env) -> String {
    let output = env
        .cmd()
        .arg("judges")
        .assert()
        .success()
        .get_output()
        .clone();
    String::from_utf8(output.stdout).unwrap()
}

mod tr12_c6 {
    //! `judges` shows the approve and reject counts for each question.
    use super::*;

    #[test]
    fn tr12_c6_judges_shows_the_approve_and_reject_counts_for_each_question() {
        let Some(env) = env_with_judge() else {
            return;
        };
        let id = started_with(&env, &whole_judge(), 1, "no");
        dispute(&env, &id);
        reject(&env, &id);
        // The same code needs the same dispute again (§2.3.2).
        dispute(&env, &id);
        approve(&env, &id);

        let question = only_question(&env);
        assert_eq!(question["project"], "XX", "{question}");
        assert_eq!(question["ask"], ASK, "{question}");
        assert_eq!(question["question"], "works", "{question}");
        let (_, approved, rejected) = counts(&question, "T12", "no");
        assert_eq!((approved, rejected), (1, 1), "{question}");

        let shown = judges_text(&env);
        assert!(shown.contains(ASK), "{shown}");
        assert!(shown.contains("approved 1"), "{shown}");
        assert!(shown.contains("rejected 1"), "{shown}");
        assert!(shown.contains("T10 and T12"), "{shown}");
    }
}

mod tr12_c8 {
    //! `judges` splits each question's approve and reject counts by the
    //! held question's trigger and answer, so agreeing with the judge and
    //! overruling it are counted apart, and DESIGN.md §6 says so (slice 15
    //! review 2, design 1).
    use super::*;

    #[test]
    fn tr12_c8_judges_splits_counts_by_the_held_questions_trigger_and_answer() {
        let Some(env) = env_with_judge() else {
            return;
        };
        // XX-1: a disputed fail, approved, which overrules the judge.
        let id = started_with(&env, &whole_judge(), 1, "no");
        dispute(&env, &id);
        approve(&env, &id);
        // XX-2: an unsure answer, approved.
        let id = started_with(&env, &whole_judge(), 2, "unsure");
        assert_eq!(env.json(&["submit", &id])["result"], "held");
        approve(&env, &id);
        // XX-3: a pass held by auto_verify: false, rejected.
        edit_config(&env, |config| {
            config["projects"]["XX"]["judge"] = json!({ "auto_verify": false });
        });
        let id = started_with(&env, &whole_judge(), 3, "yes");
        assert_eq!(env.json(&["submit", &id])["result"], "held");
        reject(&env, &id);

        // One question, since all three ask the same text.
        let question = only_question(&env);
        assert_eq!(question["held"].as_array().unwrap().len(), 3, "{question}");
        assert_eq!(
            counts(&question, "T12", "no"),
            ("disputed".to_string(), 1, 0)
        );
        assert_eq!(
            counts(&question, "T10", "unsure"),
            ("unsure".to_string(), 1, 0)
        );
        assert_eq!(
            counts(&question, "T10", "yes"),
            ("passed".to_string(), 0, 1)
        );
        let shown = judges_text(&env);
        assert!(shown.contains("disputed"), "{shown}");
        assert!(shown.contains("unsure"), "{shown}");
    }

    #[test]
    fn tr12_c8_an_each_question_counts_once_per_decision() {
        let Some(env) = env_with_judge() else {
            return;
        };
        let judge = json!({
            "text": "Each step does the work",
            "inputs": ["src/lib.rs"],
            "each": "step\\(",
            "context_lines": 0,
            "ask": [{ "id": "works", "ask": ASK }],
        });
        // Two matches, both unsure, held under T10, and approved once.
        let id = started_with(&env, &judge, 1, "unsure");
        let output = env.json(&["submit", &id]);
        assert_eq!(output["result"], "held", "{output}");
        assert_eq!(env.item(&id)["held_questions"].as_array().unwrap().len(), 2);
        approve(&env, &id);

        let question = only_question(&env);
        assert_eq!(
            counts(&question, "T10", "unsure"),
            ("unsure".to_string(), 1, 0)
        );
    }
}

mod tr12_c9 {
    //! DESIGN.md §2.3.2 says any G3 failure without a judge answer, such as
    //! a missing input or a failed call, skips the dispute check (slice 15
    //! review 2, design 3). The code already does: this pins it for a
    //! missing input (slice 15 review 2, test gap 2).
    use super::*;

    #[test]
    fn tr12_c9_a_dispute_beside_a_missing_input_is_refused_for_g3() {
        let env = judge_env();
        let id = started_with(&env, &whole_judge(), 1, "no");
        fs::remove_file(env.work.path().join("xx/src/lib.rs")).unwrap();
        git_commit_all(env.work.path(), &format!("feat({id}): remove the input"));
        let error = env.error_json_in(
            &env.outside(),
            &["submit", &id, "--dispute", "c9:nope", "--reason", "x"],
            2,
        );
        let failures = error["failures"].as_array().unwrap();
        assert!(
            failures.iter().any(|failure| failure["gate"] == "G3"),
            "{error}"
        );
        assert!(env.item(&id).get("held_questions").is_none());
    }
}

//! Slice 15 (TR-11): disputes (DESIGN.md §2.3.2, §2.5's T12, §2.7). An
//! agent that believes a judge's fail is wrong submits with `--dispute
//! <ref> --reason "…"`. If the disputed questions are the only failures,
//! T12 holds the item for a human, who approves or rejects it, and that
//! decision is recorded against each held question.
//!
//! The judge command is `fake`, a shell script in `<work>/judge/`, outside
//! the project's root, that saves each request it gets as
//! `request-<n>.json` and prints `reply.json`.
//!
//! The tests for each of TR-11's criteria are named after it, such as
//! `tr11_c1_`, so each criterion's check runs only its own.

mod common;

use std::fs;
use std::path::PathBuf;

use common::{Env, git_commit_all};
use serde_json::{Value, json};

const MODEL: &str = "claude-haiku-5-5";

/// The code before the work: a stub the judge can grade, and should fail.
const STUB: &str = "fn run() {\n    todo!()\n}\n";

/// The agent's reason for its dispute.
const AGENT: &str = "lib.rs:2 calls step, which does the work";

/// What the fake says when it answers no.
const MODEL_NO: &str = "The fake says no.";

const FAKE: &str = "#!/bin/sh
dir=$(dirname \"$0\")
n=$(ls \"$dir\" | grep -c '^request-')
cat > \"$dir/request-$((n + 1)).json\"
cat \"$dir/reply.json\"
";

const PROMPT: &str = "Type XX-1 to confirm";

fn judge_dir(env: &Env) -> PathBuf {
    env.work.path().join("judge")
}

fn write(env: &Env, path: &str, text: &str) {
    let path = env.work.path().join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn edit_config(env: &Env, change: impl FnOnce(&mut Value)) {
    let path = env.home.path().join("config.json");
    let mut config: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    change(&mut config);
    fs::write(&path, serde_json::to_string_pretty(&config).unwrap()).unwrap();
}

/// Makes the fake the store's judge command.
fn install_fake(env: &Env) {
    use std::os::unix::fs::PermissionsExt;
    let dir = judge_dir(env);
    fs::create_dir_all(&dir).unwrap();
    let fake = dir.join("fake");
    fs::write(&fake, FAKE).unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
    let command = fake.to_str().unwrap().to_string();
    edit_config(env, |config| {
        config["judge"] = json!({ "command": command, "model": MODEL });
    });
    reply(env, &[("works", "no"), ("clear", "no")]);
}

/// Has the fake answer each question id with its answer.
fn reply(env: &Env, answers: &[(&str, &str)]) {
    let mut map = serde_json::Map::new();
    for (id, answer) in answers {
        let reason = if *answer == "no" {
            MODEL_NO.to_string()
        } else {
            format!("The fake says {answer}.")
        };
        map.insert(
            id.to_string(),
            json!({ "answer": answer, "reason": reason }),
        );
    }
    let reply = json!({ "adapter": "fake 1", "model": MODEL, "answers": map, "cost_usd": 0 });
    fs::write(judge_dir(env).join("reply.json"), reply.to_string()).unwrap();
}

/// A judge that reads all of `src/lib.rs`, with the questions `ids`.
fn judge_asking(ids: &[&str]) -> Value {
    let ask: Vec<Value> = ids
        .iter()
        .map(|id| json!({ "id": id, "ask": format!("Is this code {id}?") }))
        .collect();
    json!({ "text": "The code does the work", "inputs": ["src/lib.rs"], "ask": ask })
}

/// Adds `judge` to XX-1 as a criterion.
fn add_judge(env: &Env, judge: &Value) {
    let path = judge_dir(env).join("judge.json");
    fs::write(&path, judge.to_string()).unwrap();
    env.json(&[
        "criterion",
        "add",
        "XX-1",
        "--judge",
        path.to_str().unwrap(),
    ]);
}

/// Project XX, whose `src/lib.rs` holds `STUB`, and XX-1 with its test
/// commit and a judge asking `ids`, which the fake answers no. The judges
/// aren't asked yet, and XX-1 is open.
fn open_with(ids: &[&str]) -> Env {
    let env = Env::new();
    env.add_project("XX", "xx");
    write(&env, "xx/src/lib.rs", STUB);
    git_commit_all(env.work.path(), "chore: the code before XX-1");
    env.json(&["create", "a", "--project", "XX"]);
    install_fake(&env);
    env.commit_for("XX-1");
    add_judge(&env, &judge_asking(ids));
    env
}

/// `open_with`, started on the stub, with the work committed after.
fn started_with(ids: &[&str]) -> Env {
    let env = open_with(ids);
    env.json(&["start", "XX-1"]);
    work(&env, 1);
    env
}

/// XX-1, started, with a one-question judge, `works`, that fails the work.
fn started() -> Env {
    started_with(&["works"])
}

/// Changes `src/lib.rs` to version `n` of the work, and commits it.
fn work(env: &Env, n: usize) {
    write(
        env,
        "xx/src/lib.rs",
        &format!("fn run() {{\n    step({n});\n}}\n"),
    );
    git_commit_all(env.work.path(), &format!("feat(XX-1): version {n}"));
}

/// `submit XX-1`, disputing `c1:works` with `AGENT`'s reason.
fn dispute(env: &Env) -> Value {
    env.json(&["submit", "XX-1", "--dispute", "c1:works", "--reason", AGENT])
}

/// `started`, held under T12 by a dispute of `c1:works`.
fn held() -> Env {
    let env = started();
    let output = dispute(&env);
    assert_eq!(output["result"], "held", "{output}");
    env
}

fn fired(output: &Value) -> Vec<String> {
    output["triggers"]
        .as_array()
        .unwrap_or_else(|| panic!("no triggers: {output}"))
        .iter()
        .filter(|ran| ran["fired"] == true)
        .map(|ran| ran["trigger"].as_str().unwrap().to_string())
        .collect()
}

fn hold<'a>(output: &'a Value, trigger: &str) -> &'a Value {
    output["holds"]
        .as_array()
        .unwrap()
        .iter()
        .find(|hold| hold["trigger"] == trigger)
        .unwrap_or_else(|| panic!("no {trigger} hold: {output}"))
}

/// The number of runs `judge/failures.json` counts for XX-1 (T11).
fn counted(env: &Env) -> usize {
    let path = env.home.path().join("judge/failures.json");
    let Ok(text) = fs::read_to_string(&path) else {
        return 0;
    };
    let failures: Value = serde_json::from_str(&text).unwrap();
    failures["XX-1"]["failed"]
        .as_array()
        .map_or(0, |failed| failed.len())
}

/// The refs of a refusal's failures from G3.
fn g3_refs(error: &Value) -> Vec<String> {
    error["failures"]
        .as_array()
        .unwrap_or_else(|| panic!("no failures: {error}"))
        .iter()
        .filter(|failure| failure["gate"] == "G3")
        .filter_map(|failure| failure["ref"].as_str().map(str::to_string))
        .collect()
}

/// The single held question in `questions`, checked against `trigger`.
fn only_question<'a>(questions: &'a Value, trigger: &str) -> &'a Value {
    let questions = questions
        .as_array()
        .unwrap_or_else(|| panic!("no held_questions: {questions}"));
    assert_eq!(questions.len(), 1, "{questions:?}");
    assert_eq!(questions[0]["trigger"], trigger, "{questions:?}");
    &questions[0]
}

mod tr11_c1 {
    //! A dispute holds the item under T12, showing the excerpt, the
    //! model's answer and reason, and the agent's reason.
    use super::*;

    #[test]
    fn tr11_c1_a_dispute_holds_the_item_under_t12() {
        let env = started();
        let output = dispute(&env);
        assert_eq!(output["result"], "held", "{output}");
        assert_eq!(output["status"], "review", "{output}");
        assert_eq!(fired(&output), ["T12"], "{output}");
        let message = hold(&output, "T12")["message"].as_str().unwrap();
        assert!(message.contains("c1:works"), "{message}");
        assert!(message.contains("no"), "{message}");

        let item = env.item("XX-1");
        assert_eq!(item["status"], "review");
        let question = only_question(&item["held_questions"], "T12");
        assert_eq!(question["ref"], "c1:works");
        assert_eq!(question["answer"], "no");
        assert_eq!(question["reason"], MODEL_NO);
        assert_eq!(question["dispute"], AGENT);
        // T12 runs between T11 and T13.
        let names: Vec<&str> = output["triggers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|ran| ran["trigger"].as_str().unwrap())
            .collect();
        let at = |name: &str| names.iter().position(|n| *n == name).unwrap();
        assert!(at("T11") < at("T12") && at("T12") < at("T13"), "{names:?}");
    }

    #[test]
    fn tr11_c1_the_held_submit_shows_the_excerpt_and_both_reasons() {
        let env = started();
        let output = env
            .cmd()
            .args(["submit", "XX-1", "--dispute", "c1:works", "--reason", AGENT])
            .assert()
            .success();
        let text = String::from_utf8(output.get_output().stdout.clone()).unwrap();
        assert!(text.contains("T12"), "{text}");
        assert!(text.contains("c1:works"), "{text}");
        // The excerpt, the model's answer and reason, and the agent's reason.
        assert!(text.contains("step(1);"), "{text}");
        assert!(text.contains(MODEL_NO), "{text}");
        assert!(text.contains(AGENT), "{text}");
    }

    #[test]
    fn tr11_c1_review_shows_the_model_s_and_the_agent_s_reasons() {
        let env = held();
        let output = env.cmd().arg("review").assert().success();
        let text = String::from_utf8(output.get_output().stdout.clone()).unwrap();
        assert!(text.contains("T12"), "{text}");
        assert!(text.contains(MODEL_NO), "{text}");
        assert!(text.contains(AGENT), "{text}");

        let review = env.json(&["review"]);
        let question = only_question(&review["held"][0]["held_questions"], "T12");
        assert_eq!(question["dispute"], AGENT, "{review}");
    }

    #[test]
    fn tr11_c1_submit_help_names_t12_and_dispute() {
        let env = Env::without_store();
        let output = env.cmd().args(["submit", "--help"]).assert().success();
        let help = String::from_utf8(output.get_output().stdout.clone()).unwrap();
        let words: Vec<&str> = help
            .split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
            .collect();
        assert!(words.contains(&"T12"), "{help}");
        assert!(words.contains(&"--dispute"), "{help}");
    }
}

mod tr11_c2 {
    //! Disputing a question that passed or got an unsure answer exits 1.
    use super::*;

    fn refused(env: &Env) -> String {
        let before = fs::read_to_string(env.item_path("XX-1")).unwrap();
        let error = env.error_json_in(
            &env.outside(),
            &["submit", "XX-1", "--dispute", "c1:works", "--reason", AGENT],
            1,
        );
        assert_eq!(
            fs::read_to_string(env.item_path("XX-1")).unwrap(),
            before,
            "the refused submit changed XX-1"
        );
        error["error"].as_str().unwrap().to_string()
    }

    #[test]
    fn tr11_c2_disputing_a_question_that_passed_exits_1() {
        let env = started();
        reply(&env, &[("works", "yes")]);
        let message = refused(&env);
        assert!(message.contains("c1:works"), "{message}");
        assert!(message.contains("passed"), "{message}");
    }

    #[test]
    fn tr11_c2_disputing_an_unsure_answer_exits_1() {
        let env = started();
        reply(&env, &[("works", "unsure")]);
        let message = refused(&env);
        assert!(message.contains("c1:works"), "{message}");
        assert!(message.contains("unsure"), "{message}");
    }

    #[test]
    fn tr11_c2_disputing_a_ref_no_judge_asked_exits_1() {
        let env = started();
        let before = fs::read_to_string(env.item_path("XX-1")).unwrap();
        let error = env.error_json_in(
            &env.outside(),
            &["submit", "XX-1", "--dispute", "c9:nope", "--reason", AGENT],
            1,
        );
        assert!(
            error["error"].as_str().unwrap().contains("c9:nope"),
            "{error}"
        );
        assert_eq!(fs::read_to_string(env.item_path("XX-1")).unwrap(), before);
    }

    #[test]
    fn tr11_c2_a_dispute_refused_with_exit_1_counts_nothing_toward_t11() {
        // An unsure answer is a failed request for T11, but nothing was
        // submitted, so the refused run isn't counted.
        let env = started();
        reply(&env, &[("works", "unsure")]);
        let message = refused(&env);
        assert!(message.contains("unsure"), "{message}");
        assert_eq!(counted(&env), 0);
    }

    #[test]
    fn tr11_c2_each_dispute_needs_its_own_reason() {
        let env = started();
        let error = env.error_json_in(
            &env.outside(),
            &["submit", "XX-1", "--dispute", "c1:works"],
            1,
        );
        assert!(
            error["error"].as_str().unwrap().contains("--reason"),
            "{error}"
        );
        assert_eq!(env.item("XX-1")["status"], "in-progress");
    }
}

mod tr11_c3 {
    //! A dispute doesn't stop another failing gate from refusing the submit.
    use super::*;

    #[test]
    fn tr11_c3_a_failing_check_still_refuses_a_disputed_submit() {
        let env = open_with(&["works"]);
        env.json(&["criterion", "add", "XX-1", "it builds", "--check", "false"]);
        env.json(&["start", "XX-1"]);
        work(&env, 1);
        let error = env.error_json_in(
            &env.outside(),
            &["submit", "XX-1", "--dispute", "c1:works", "--reason", AGENT],
            2,
        );
        assert_eq!(error["result"], "refused", "{error}");
        let check = error["failures"]
            .as_array()
            .unwrap()
            .iter()
            .any(|failure| failure["gate"] == "G3" && failure["criterion"] == 2);
        assert!(check, "{error}");
        assert_eq!(env.item("XX-1")["status"], "in-progress");
    }

    #[test]
    fn tr11_c3_a_failing_project_check_still_refuses_a_disputed_submit() {
        let env = started();
        edit_config(&env, |config| {
            config["projects"]["XX"]["checks"] = json!(["false"]);
        });
        let error = env.error_json_in(
            &env.outside(),
            &["submit", "XX-1", "--dispute", "c1:works", "--reason", AGENT],
            2,
        );
        let g4 = error["failures"]
            .as_array()
            .unwrap()
            .iter()
            .any(|failure| failure["gate"] == "G4");
        assert!(g4, "{error}");
        assert_eq!(env.item("XX-1")["status"], "in-progress");
    }

    #[test]
    fn tr11_c3_an_undisputed_judge_fail_still_refuses_and_is_the_one_listed() {
        let env = started_with(&["works", "clear"]);
        let error = env.error_json_in(
            &env.outside(),
            &["submit", "XX-1", "--dispute", "c1:works", "--reason", AGENT],
            2,
        );
        assert_eq!(g3_refs(&error), ["c1:clear"], "{error}");
        assert_eq!(env.item("XX-1")["status"], "in-progress");
        assert!(env.item("XX-1").get("held_questions").is_none());
    }

    #[test]
    fn tr11_c3_disputing_every_fail_of_two_questions_holds_the_item() {
        let env = started_with(&["works", "clear"]);
        let output = env.json(&[
            "submit",
            "XX-1",
            "--dispute",
            "c1:works",
            "--reason",
            AGENT,
            "--dispute",
            "c1:clear",
            "--reason",
            "it reads plainly",
        ]);
        assert_eq!(output["result"], "held", "{output}");
        assert_eq!(fired(&output), ["T12"], "{output}");
        let questions = env.item("XX-1")["held_questions"].clone();
        let refs: Vec<&str> = questions
            .as_array()
            .unwrap()
            .iter()
            .map(|q| q["ref"].as_str().unwrap())
            .collect();
        assert_eq!(refs, ["c1:works", "c1:clear"], "{questions}");
        assert_eq!(questions[1]["dispute"], "it reads plainly");
    }
}

mod tr11_c4 {
    //! Approving a T12 hold moves the item to done.
    use super::*;

    #[test]
    fn tr11_c4_approving_a_t12_hold_moves_the_item_to_done() {
        let env = held();
        let Some((output, ok)) =
            env.in_terminal_typing(&["approve", "XX-1"], PROMPT, "XX-1", || {})
        else {
            return;
        };
        assert!(ok, "{output}");
        // The question I'm asked shows what I'm deciding.
        assert!(output.contains(AGENT), "{output}");
        assert!(output.contains(MODEL_NO), "{output}");
        assert!(output.contains("step(1);"), "{output}");

        let item = env.item("XX-1");
        assert_eq!(item["status"], "done", "{item}");
        assert_eq!(item["verification"]["by"], "human");
        assert!(item.get("held_questions").is_none(), "{item}");
        // My decision is recorded against the held question.
        let question = only_question(&item["verification"]["held_questions"], "T12");
        assert_eq!(question["ref"], "c1:works");
        let last = item["history"].as_array().unwrap().last().unwrap().clone();
        assert_eq!(last["via"], "approve");
        let question = only_question(&last["held_questions"], "T12");
        assert_eq!(question["dispute"], AGENT);
    }

    #[test]
    fn tr11_c4_a_dispute_covers_only_the_answer_it_disputed() {
        // A change made while it waits gets a new answer, which nobody
        // disputed, so the approve's G3 refuses it.
        let env = held();
        work(&env, 2);
        let Some((output, ok)) =
            env.in_terminal_typing(&["approve", "XX-1"], PROMPT, "XX-1", || {})
        else {
            return;
        };
        assert!(!ok, "{output}");
        assert!(output.contains("G3"), "{output}");
        assert_eq!(env.item("XX-1")["status"], "review");
    }

    #[test]
    fn tr11_c4_approving_a_t10_hold_records_the_unsure_question() {
        let env = started();
        reply(&env, &[("works", "unsure")]);
        let output = env.json(&["submit", "XX-1"]);
        assert_eq!(fired(&output), ["T10"], "{output}");
        only_question(&env.item("XX-1")["held_questions"], "T10");
        let Some((output, ok)) =
            env.in_terminal_typing(&["approve", "XX-1"], PROMPT, "XX-1", || {})
        else {
            return;
        };
        assert!(ok, "{output}");
        let item = env.item("XX-1");
        let question = only_question(&item["verification"]["held_questions"], "T10");
        assert_eq!(question["ref"], "c1:works");
        assert_eq!(question["answer"], "unsure");
    }
}

mod tr11_c5 {
    //! Resubmitting unchanged code after a reject needs the dispute again.
    //! T12 can't be disabled.
    use super::*;

    #[test]
    fn tr11_c5_resubmitting_unchanged_code_after_a_reject_needs_the_dispute_again() {
        let env = held();
        let Some((output, ok)) = env.in_terminal_typing(
            &["reject", "XX-1", "--reason", "not yet"],
            PROMPT,
            "XX-1",
            || {},
        ) else {
            return;
        };
        assert!(ok, "{output}");
        let item = env.item("XX-1");
        assert_eq!(item["status"], "in-progress");
        assert!(item.get("held_questions").is_none(), "{item}");
        // My decision is recorded against the held question.
        let last = item["history"].as_array().unwrap().last().unwrap().clone();
        assert_eq!(last["via"], "reject");
        assert_eq!(
            only_question(&last["held_questions"], "T12")["ref"],
            "c1:works"
        );

        // The cache gives the same no, and nothing carries the dispute over.
        let error = env.error_json_in(&env.outside(), &["submit", "XX-1"], 2);
        assert_eq!(g3_refs(&error), ["c1:works"], "{error}");
        let output = dispute(&env);
        assert_eq!(fired(&output), ["T12"], "{output}");
    }

    #[test]
    fn tr11_c5_t12_cant_be_disabled() {
        let env = started();
        edit_config(&env, |config| {
            config["projects"]["XX"]["review"] = json!({ "disabled": ["T12"] });
        });
        let output = dispute(&env);
        assert_eq!(output["result"], "held", "{output}");
        assert_eq!(fired(&output), ["T12"], "{output}");
    }
}

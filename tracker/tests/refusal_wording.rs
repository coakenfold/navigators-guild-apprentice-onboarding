//! TR-54: a refusal that has counted a T11 run says the item wasn't
//! changed, not that nothing was. A refusal that counts nothing keeps
//! "Nothing was changed." (DESIGN.md §2.3.2).
//!
//! The judge command is `fake`, as in tests/disputes.rs.

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

/// Changes `src/lib.rs` to version `n` of the work, and commits it.
fn work(env: &Env, n: usize) {
    write(
        env,
        "xx/src/lib.rs",
        &format!("fn run() {{\n    step({n});\n}}\n"),
    );
    git_commit_all(env.work.path(), &format!("feat(XX-1): version {n}"));
}

fn started() -> Env {
    started_with(&["works"])
}

mod tr54_c1 {
    use super::*;

    #[test]
    fn tr54_c1_a_refused_dispute_says_the_item_wasnt_changed() {
        let env = started();
        let error = env.error_json_in(
            &env.outside(),
            &["submit", "XX-1", "--dispute", "c9:nope", "--reason", AGENT],
            1,
        );
        let message = error["error"].as_str().unwrap();
        assert!(message.contains("XX-1 wasn't changed."), "{message}");
        assert!(!message.contains("Nothing was changed"), "{message}");
    }
}

mod tr54_c2 {
    use super::*;

    #[test]
    fn tr54_c2_a_submit_refused_for_a_judge_fail_says_the_item_wasnt_changed() {
        let env = started();
        let error = env.error_json_in(&env.outside(), &["submit", "XX-1"], 2);
        let message = error["error"].as_str().unwrap();
        assert!(message.contains("G3 failed"), "{message}");
        assert!(message.contains("XX-1 wasn't changed."), "{message}");
        assert!(!message.contains("Nothing was changed"), "{message}");
    }

    #[test]
    fn tr54_c2_a_submit_refused_before_any_judge_is_asked_says_nothing_was_changed() {
        let env = open_with(&["works"]);
        env.json(&["start", "XX-1"]);
        // Uncommitted change: a static gate refuses before the judges run.
        write(&env, "xx/src/lib.rs", "fn run() {\n    dirty();\n}\n");
        let error = env.error_json_in(&env.outside(), &["submit", "XX-1"], 2);
        let message = error["error"].as_str().unwrap();
        assert!(message.contains("Nothing was changed."), "{message}");
    }
}

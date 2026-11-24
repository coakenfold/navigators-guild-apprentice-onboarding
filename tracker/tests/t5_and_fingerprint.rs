//! TR-42: tests for slice 11's T5 and T13 that its first pass left out
//! (slice 11 review 1, test gaps 1 to 3). The code already works, so each
//! test was seen failing by breaking the code temporarily.
//!
//! Each of TR-42's criteria has a module named after it, so its check runs
//! only its own tests. Every project here is XX, whose root `<work>/xx` is
//! a subfolder of the work folder's repository.

mod common;

use std::fs;
use std::path::Path;

use common::{Env, git_commit_all};
use serde_json::{Value, json};

/// A check that fails until XX-1's work adds `xx/done`.
const DONE: &str = "test -f done";

const ACCEPT_PROMPT: &str = "or anything else to cancel: ";

fn write(env: &Env, path: &str, text: &str) {
    let path = env.work.path().join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// A store with project XX at `<work>/xx`, then XX-1 created with the
/// `--done-when` criteria in `done_when` and committed.
fn created(done_when: &[&str]) -> Env {
    let env = Env::new();
    env.add_project("XX", "xx");
    write(&env, "xx/README.md", "## 1. XX\n\nWhat it is.\n");
    git_commit_all(env.work.path(), "chore: the code before XX-1");
    let mut args = vec!["create", "a", "--project", "XX"];
    for text in done_when {
        args.push("--done-when");
        args.push(text);
    }
    env.json(&args);
    env.commit_for("XX-1");
    env
}

/// XX-1 with one script criterion, started.
fn started() -> Env {
    let env = created(&[]);
    env.json(&["criterion", "add", "XX-1", "it is done", "--check", DONE]);
    env.json(&["start", "XX-1"]);
    env
}

fn do_the_work(env: &Env) {
    write(env, "xx/done", "done\n");
    git_commit_all(env.work.path(), "feat(XX-1): the work");
}

fn fired(output: &Value) -> Vec<String> {
    output["triggers"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|ran| ran["fired"] == true)
        .map(|ran| ran["trigger"].as_str().unwrap().to_string())
        .collect()
}

fn hold(output: &Value, trigger: &str) -> String {
    output["holds"]
        .as_array()
        .unwrap()
        .iter()
        .find(|hold| hold["trigger"] == trigger)
        .unwrap_or_else(|| panic!("no {trigger} hold: {output}"))["message"]
        .as_str()
        .unwrap()
        .to_string()
}

/// Asserts that a submit was held by T5 alone.
fn assert_held_by_t5(output: &Value) {
    assert_eq!(output["status"], "review", "{output}");
    assert_eq!(fired(output), ["T5"], "{output}");
}

mod criterion_1 {
    //! The criteria fingerprint of fixture XX-23, which slice 11's binary
    //! recorded, must not change: a change to `Criterion` or to how it's
    //! serialized would move it, and every item started before would then
    //! fire T5.
    use super::*;

    #[test]
    fn the_fingerprint_of_fixture_xx_23_is_the_one_slice_11_recorded() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/slice-11/XX-23.json");
        let text = fs::read_to_string(path).unwrap();
        let item = tracker::item::from_json(&text).unwrap();
        let recorded = item
            .started
            .as_ref()
            .and_then(|started| started.criteria_hash.clone())
            .unwrap();
        assert_eq!(
            recorded,
            "ed3c8e84e7b07c3926fe287d9a874c0492b91ced9cedb550860561cb9ce3e1da"
        );
        assert_eq!(tracker::triggers::fingerprint(&item), recorded);
    }
}

mod criterion_2 {
    //! A check changed or given after `start`, and `edit --design` after
    //! `start`, each fire T5.
    use super::*;

    #[test]
    fn a_check_changed_after_start_fires_t5() {
        let env = started();
        env.json(&["criterion", "set", "XX-1", "1", "--check", "test -f ./done"]);
        do_the_work(&env);

        let output = env.json(&["submit", "XX-1"]);
        assert_held_by_t5(&output);
        assert!(hold(&output, "T5").contains("checks"), "{output}");
    }

    #[test]
    fn a_check_given_after_start_fires_t5() {
        let env = started();
        env.json(&[
            "criterion",
            "add",
            "XX-1",
            "it is really done",
            "--check",
            "test -s done",
        ]);
        do_the_work(&env);

        assert_held_by_t5(&env.json(&["submit", "XX-1"]));
    }

    #[test]
    fn edit_design_after_start_fires_t5() {
        let env = started();
        let output = env.json_in(&env.dir("xx"), &["edit", "XX-1", "--design", "README.md#1"]);
        assert_eq!(output["item"]["design"], "README.md#1", "{output}");
        do_the_work(&env);

        let output = env.json(&["submit", "XX-1"]);
        assert_held_by_t5(&output);
        assert!(hold(&output, "T5").contains("design"), "{output}");
    }
}

mod criterion_3 {
    //! `criterion accept` after `start` records the new text in `written`
    //! but doesn't take the fingerprint again, so T5 still fires. And a
    //! check given while the item is in review isn't run, so T13 reads it
    //! as one that didn't pass.
    use super::*;

    #[test]
    fn criterion_accept_after_start_still_leaves_t5_firing_for_the_edit() {
        let env = created(&["it is done"]);
        env.json(&["criterion", "set", "XX-1", "1", "--check", DONE]);
        env.json(&["start", "XX-1"]);
        // I reword the criterion by hand in the file.
        let mut item = env.item("XX-1");
        item["criteria"][0]["text"] = json!("it is done well");
        fs::write(
            env.item_path("XX-1"),
            serde_json::to_string_pretty(&item).unwrap() + "\n",
        )
        .unwrap();
        do_the_work(&env);

        // Before accepting, T5 names both halves.
        let output = env.json(&["gates", "XX-1"]);
        assert_eq!(fired(&output), ["T5"], "{output}");

        let Some((output, ok)) = env.in_terminal_typing(
            &["criterion", "accept", "XX-1"],
            ACCEPT_PROMPT,
            "XX-1",
            || {},
        ) else {
            return;
        };
        assert!(ok, "{output}");
        assert_eq!(env.item("XX-1")["written"], json!(["it is done well"]));

        // `written` now agrees, but the fingerprint is the one from start.
        let output = env.json(&["submit", "XX-1"]);
        assert_held_by_t5(&output);
        let message = hold(&output, "T5");
        assert!(message.contains("changed after start"), "{message}");
        assert!(!message.contains("no longer any"), "{message}");
    }

    #[test]
    fn t13_reads_a_check_given_in_review_as_one_that_didnt_pass() {
        // Criterion 1 is manual at start, so `started.checks` has no entry
        // for it. The item is held for T6 and waits in review.
        let env = created(&["it reads well"]);
        env.json(&["criterion", "add", "XX-1", "it is done", "--check", DONE]);
        env.json(&["start", "XX-1"]);
        do_the_work(&env);
        env.json(&["evidence", "XX-1", "1", "the page"]);
        let output = env.json(&["submit", "XX-1"]);
        assert_eq!(output["status"], "review", "{output}");

        // From a terminal I give criterion 1 a check that passes right now.
        let Some(output) = env.in_terminal(&["criterion", "set", "XX-1", "1", "--check", "true"])
        else {
            return;
        };
        assert!(output.status.success(), "{output:?}");

        let output = env.json(&["gates", "XX-1"]);
        let triggers = fired(&output);
        assert!(triggers.contains(&"T5".to_string()), "{output}");
        assert!(!triggers.contains(&"T13".to_string()), "{output}");
    }
}

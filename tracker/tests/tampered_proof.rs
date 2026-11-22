//! Slice 11 (TR-7): tampered proof is held (DESIGN.md §2.1, §2.4, §2.5).
//! T5 holds an item whose goalposts moved: its criteria, kind or design
//! changed after start, or a criterion I wrote was reworded or removed. T13
//! holds one whose check already passed before the work. T14 holds one with
//! unscoped commits since start that touch the project's root.
//!
//! The tests for each of TR-7's criteria are in a module named after it,
//! such as `criterion_1`, so each criterion's check runs only its own.
//! Every project here is XX, whose root `<work>/xx` is a subfolder of the
//! work folder's repository, as TR's root is a folder in guild-portfolio.

mod common;

use std::fs;

use common::{Env, git_commit_all};
use serde_json::Value;

/// A check that fails until XX-1's work adds `xx/done`.
const DONE: &str = "test -f done";

/// A store with project XX at `<work>/xx`, holding one file committed
/// unscoped before any item exists.
fn env_with_project() -> Env {
    let env = Env::new();
    env.add_project("XX", "xx");
    write(&env, "xx/README.md", "XX\n");
    git_commit_all(env.work.path(), "chore: the code before XX-1");
    env
}

/// Creates XX-1 with the `create` arguments in `args`, and commits it as
/// its test commit, as `Env::create_bare` does.
fn create(env: &Env, args: &[&str]) {
    let mut all = vec!["create", "a", "--project", "XX"];
    all.extend_from_slice(args);
    env.json(&all);
    env.commit_for("XX-1");
}

/// XX-1, created with one script criterion whose check is `DONE`, and
/// started.
fn started() -> Env {
    let env = env_with_project();
    create(&env, &[]);
    env.json(&["criterion", "add", "XX-1", "it is done", "--check", DONE]);
    env.json(&["start", "XX-1"]);
    env
}

/// Writes `text` to `path`, from the top of the work folder's repository,
/// making its folders.
fn write(env: &Env, path: &str, text: &str) {
    let path = env.work.path().join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// XX-1's work: adds `xx/done`, so `DONE` passes, in one of its commits.
fn do_the_work(env: &Env) {
    write(env, "xx/done", "done\n");
    git_commit_all(env.work.path(), "feat(XX-1): the work");
}

/// Submits XX-1 and returns the output.
fn submit(env: &Env) -> Value {
    env.json(&["submit", "XX-1"])
}

/// The names of the triggers that fired in a submit's or gates' output.
fn fired(output: &Value) -> Vec<String> {
    output["triggers"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|ran| ran["fired"] == true)
        .map(|ran| ran["trigger"].as_str().unwrap().to_string())
        .collect()
}

/// Whether `trigger` ran in a submit's or gates' output.
fn ran(output: &Value, trigger: &str) -> bool {
    output["triggers"]
        .as_array()
        .unwrap()
        .iter()
        .any(|ran| ran["trigger"] == trigger)
}

/// The message of the hold `trigger` put on the item, from a submit's
/// output.
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

/// Asserts that a submit was held by exactly `triggers`.
fn assert_held_by(output: &Value, triggers: &[&str]) {
    assert_eq!(output["status"], "review", "{output}");
    assert_eq!(fired(output), triggers, "{output}");
}

/// Asserts that `trigger` ran and didn't fire, and that nothing else did
/// either, so the item reached done.
fn assert_quiet(output: &Value, trigger: &str) {
    assert!(ran(output, trigger), "{trigger} didn't run: {output}");
    assert_eq!(output["status"], "done", "{output}");
    assert!(fired(output).is_empty(), "{output}");
}

mod criterion_1 {
    //! T13 fires for a `cargo test` filter that matched no tests at start.
    use super::*;

    /// A store with project XX, whose root holds a small crate with one
    /// test, `search_finds_a_tag`, which passes once `xx/found` exists.
    /// `target/` and `Cargo.lock`, which `cargo test` writes, are ignored,
    /// so the checks leave the tree as they found it (§2.4).
    fn env_with_crate() -> Env {
        let env = Env::new();
        env.add_project("XX", "xx");
        write(
            &env,
            "xx/Cargo.toml",
            "[package]\nname = \"xx\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        );
        write(&env, "xx/.gitignore", "/target\n/Cargo.lock\n");
        write(
            &env,
            "xx/src/lib.rs",
            "#[test]\nfn search_finds_a_tag() {\n    \
             assert!(std::path::Path::new(\"found\").exists());\n}\n",
        );
        git_commit_all(env.work.path(), "chore: the crate before XX-1");
        create(&env, &[]);
        env
    }

    #[test]
    fn t13_fires_for_a_cargo_test_filter_that_matched_no_tests_at_start() {
        let env = env_with_crate();
        // A filter that matches no test, so `cargo test` runs none and
        // exits 0, before the work and after it.
        let check = "cargo test --offline search_finds_a_tga";
        env.json(&[
            "criterion",
            "add",
            "XX-1",
            "search finds a tag",
            "--check",
            check,
        ]);
        let start = env.json(&["start", "XX-1"]);
        assert_eq!(
            start["item"]["started"]["checks"][0]["passed"], true,
            "{start}"
        );

        write(&env, "xx/found", "\n");
        git_commit_all(env.work.path(), "feat(XX-1): the work");
        let output = submit(&env);
        assert_held_by(&output, &["T13"]);
        let message = hold(&output, "T13");
        assert!(message.contains("criterion 1's check"), "{message}");
        assert!(message.contains("already passed"), "{message}");
    }

    #[test]
    fn t13_stays_quiet_for_a_cargo_test_filter_that_failed_at_start() {
        let env = env_with_crate();
        let check = "cargo test --offline search_finds_a_tag";
        env.json(&[
            "criterion",
            "add",
            "XX-1",
            "search finds a tag",
            "--check",
            check,
        ]);
        let start = env.json(&["start", "XX-1"]);
        assert_eq!(
            start["item"]["started"]["checks"][0]["passed"], false,
            "{start}"
        );

        write(&env, "xx/found", "\n");
        git_commit_all(env.work.path(), "feat(XX-1): the work");
        assert_quiet(&submit(&env), "T13");
    }
}

mod criterion_2 {
    //! T5 fires when a non-interactive command changes or removes a
    //! written criterion's text before start, and stays quiet when one adds
    //! a criterion or gives one a check.
    use super::*;

    #[test]
    fn t5_fires_when_a_written_criterion_is_removed_before_start() {
        let env = env_with_project();
        create(
            &env,
            &["--done-when", "tags filter", "--done-when", "search works"],
        );
        env.json(&["criterion", "remove", "XX-1", "2"]);
        env.json(&["criterion", "set", "XX-1", "1", "--check", DONE]);
        env.json(&["start", "XX-1"]);
        do_the_work(&env);

        let output = submit(&env);
        assert_held_by(&output, &["T5"]);
        let message = hold(&output, "T5");
        assert!(message.contains("\"search works\""), "{message}");
        assert!(!message.contains("tags filter"), "{message}");
    }

    #[test]
    fn t5_fires_when_a_written_criterion_is_reworded_before_start() {
        let env = env_with_project();
        create(&env, &["--done-when", "search finds tags"]);
        // No command rewords a criterion, so an agent removes it and adds
        // the new text.
        env.json(&["criterion", "remove", "XX-1", "1"]);
        env.json(&[
            "criterion",
            "add",
            "XX-1",
            "search finds something",
            "--check",
            DONE,
        ]);
        env.json(&["start", "XX-1"]);
        do_the_work(&env);

        let output = submit(&env);
        assert_held_by(&output, &["T5"]);
        let message = hold(&output, "T5");
        assert!(message.contains("\"search finds tags\""), "{message}");
    }

    #[test]
    fn t5_stays_quiet_when_a_criterion_is_added_or_given_a_check_before_start() {
        let env = env_with_project();
        create(&env, &["--done-when", "tags filter"]);
        env.json(&["criterion", "set", "XX-1", "1", "--check", DONE]);
        env.json(&["criterion", "add", "XX-1", "search works", "--check", DONE]);
        env.json(&["start", "XX-1"]);
        do_the_work(&env);

        assert_quiet(&submit(&env), "T5");
    }
}

mod criterion_3 {
    //! `edit --kind` after start fires T5.
    use super::*;

    #[test]
    fn edit_kind_after_start_fires_t5() {
        let env = started();
        env.json(&["edit", "XX-1", "--kind", "fix"]);
        do_the_work(&env);

        let output = submit(&env);
        assert_held_by(&output, &["T5"]);
        let message = hold(&output, "T5");
        assert!(message.contains("changed after start"), "{message}");
    }

    #[test]
    fn t5_stays_quiet_when_nothing_changed_after_start() {
        let env = started();
        do_the_work(&env);
        assert_quiet(&submit(&env), "T5");
    }
}

mod criterion_4 {
    //! T14 fires for an unscoped commit after start, including one headed
    //! `XX-1: …`. It doesn't fire for one made before start, or for an
    //! unscoped commit that only touches files outside the project root.
    use super::*;

    #[test]
    fn t14_fires_for_an_unscoped_commit_after_start() {
        let env = started();
        write(&env, "xx/notes.md", "notes\n");
        let sha = git_commit_all(env.work.path(), "tidy the notes");
        do_the_work(&env);

        let output = submit(&env);
        assert_held_by(&output, &["T14"]);
        let message = hold(&output, "T14");
        assert!(message.contains(&sha[..7]), "{message}");
        assert!(message.contains("tidy the notes"), "{message}");
    }

    #[test]
    fn t14_fires_for_a_commit_headed_with_the_id_alone() {
        let env = started();
        write(&env, "xx/done", "done\n");
        let sha = git_commit_all(env.work.path(), "XX-1: the work");

        let output = submit(&env);
        assert_held_by(&output, &["T14"]);
        let message = hold(&output, "T14");
        assert!(message.contains(&sha[..7]), "{message}");
        assert!(message.contains("XX-1: the work"), "{message}");
    }

    #[test]
    fn t14_doesnt_fire_for_an_unscoped_commit_before_start() {
        let env = env_with_project();
        create(&env, &[]);
        env.json(&["criterion", "add", "XX-1", "it is done", "--check", DONE]);
        write(&env, "xx/notes.md", "notes\n");
        git_commit_all(env.work.path(), "tidy the notes");
        env.json(&["start", "XX-1"]);
        do_the_work(&env);

        assert_quiet(&submit(&env), "T14");
    }

    #[test]
    fn t14_doesnt_fire_for_an_unscoped_commit_only_outside_the_root() {
        let env = started();
        write(&env, "elsewhere/notes.md", "notes\n");
        git_commit_all(env.work.path(), "tidy the notes elsewhere");
        do_the_work(&env);

        assert_quiet(&submit(&env), "T14");
    }

    /// Slice 10 review 2's security 1: an unscoped commit after start
    /// deletes an existing test, and the item's own commit re-adds it
    /// weakened. T4 sees only an added file, so T14 is what holds it.
    #[test]
    fn t14_holds_an_unscoped_delete_of_a_test_that_the_item_re_adds() {
        let env = env_with_project();
        write(
            &env,
            "xx/tests/search.rs",
            "#[test]\nfn finds() { assert!(false); }\n",
        );
        git_commit_all(env.work.path(), "chore: the tests before XX-1");
        create(&env, &[]);
        env.json(&["criterion", "add", "XX-1", "it is done", "--check", DONE]);
        env.json(&["start", "XX-1"]);
        fs::remove_file(env.work.path().join("xx/tests/search.rs")).unwrap();
        let sha = git_commit_all(env.work.path(), "remove a stale test");
        write(&env, "xx/tests/search.rs", "#[test]\nfn finds() {}\n");
        do_the_work(&env);

        let output = submit(&env);
        assert_held_by(&output, &["T14"]);
        assert!(hold(&output, "T14").contains(&sha[..7]), "{output}");
    }
}

mod criterion_5 {
    //! A reopened item resubmitted with unchanged code is held by T13.
    use super::*;

    #[test]
    fn a_reopened_item_resubmitted_with_unchanged_code_is_held_by_t13() {
        let env = started();
        do_the_work(&env);
        assert_quiet(&submit(&env), "T13");

        env.json(&["reopen", "XX-1", "--reason", "The tag list is missing."]);
        let start = env.json(&["start", "XX-1"]);
        assert_eq!(start["warnings"].as_array().unwrap().len(), 1, "{start}");
        let output = submit(&env);
        assert_held_by(&output, &["T13"]);
        assert!(
            hold(&output, "T13").contains("criterion 1's check"),
            "{output}"
        );
    }
}

mod commits_scoped_elsewhere {
    //! A commit scoped to another item in the same project counts as
    //! unscoped, since it gets past every trigger that reads changed files
    //! for this item. One scoped to another project's key isn't this
    //! project's work, so T14 leaves it alone (§2.5).
    use super::*;

    #[test]
    fn t14_fires_for_a_commit_scoped_to_another_item_in_the_project() {
        let env = started();
        write(&env, "xx/notes.md", "notes\n");
        let sha = git_commit_all(env.work.path(), "docs(XX-2): the notes");
        do_the_work(&env);

        let output = submit(&env);
        assert_held_by(&output, &["T14"]);
        let message = hold(&output, "T14");
        assert!(message.contains(&sha[..7]), "{message}");
        assert!(message.contains("docs(XX-2): the notes"), "{message}");
    }

    #[test]
    fn t14_tells_another_project_from_another_item() {
        // A commit scoped to another project's key doesn't fire it.
        let env = started();
        write(&env, "xx/notes.md", "notes\n");
        git_commit_all(env.work.path(), "docs(YY-2): the notes");
        do_the_work(&env);
        assert_quiet(&submit(&env), "T14");

        // And one scoped to another item of this project does.
        let env = started();
        write(&env, "xx/notes.md", "notes\n");
        git_commit_all(env.work.path(), "docs(XX-2): the notes");
        do_the_work(&env);
        assert_held_by(&submit(&env), &["T14"]);
    }
}

mod missing_start_check {
    //! PLAN.md's slice 11 prompt: a check given while the item is in
    //! review isn't run then, so its criterion can have no entry in
    //! `started.checks`. T13 reads a missing entry as a check that didn't
    //! pass.
    use super::*;

    #[test]
    fn t13_reads_a_missing_started_check_as_one_that_didnt_pass() {
        let env = env_with_project();
        create(&env, &[]);
        // A check that passes before the work, whose start entry is then
        // taken out, as for a check given in review.
        env.json(&["criterion", "add", "XX-1", "it is done", "--check", "true"]);
        env.json(&["start", "XX-1"]);
        let path = env.item_path("XX-1");
        let mut item: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        item["started"].as_object_mut().unwrap().remove("checks");
        fs::write(&path, serde_json::to_string_pretty(&item).unwrap()).unwrap();
        do_the_work(&env);

        assert_quiet(&submit(&env), "T13");
    }
}

mod fingerprint {
    //! `start` records the criteria's fingerprint in `started`, before
    //! `head` (§4.3), and evidence recorded after start doesn't change it,
    //! since it can only be written once the work exists.
    use super::*;

    #[test]
    fn start_records_the_criteria_fingerprint_before_head() {
        let env = started();
        let item = env.item("XX-1");
        let started = item["started"].as_object().unwrap();
        let fields: Vec<&String> = started.keys().collect();
        assert_eq!(fields, ["at", "criteria_hash", "head", "checks"], "{item}");
        let hash = started["criteria_hash"].as_str().unwrap();
        assert_eq!(hash.len(), 64, "{hash}");
        assert!(hash.bytes().all(|b| b.is_ascii_hexdigit()), "{hash}");
    }

    #[test]
    fn evidence_recorded_after_start_doesnt_fire_t5() {
        let env = env_with_project();
        create(&env, &["--done-when", "the page reads well"]);
        env.json(&["criterion", "add", "XX-1", "it is done", "--check", DONE]);
        env.json(&["start", "XX-1"]);
        env.json(&["evidence", "XX-1", "1", "the home page"]);
        do_the_work(&env);

        // Held only for the manual criterion.
        let output = submit(&env);
        assert!(ran(&output, "T5"), "{output}");
        assert_held_by(&output, &["T6"]);
    }
}

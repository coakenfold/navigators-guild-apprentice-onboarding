//! Slice 10 (TR-6): risky changes are held (DESIGN.md §2.1, §2.5, §4.2).
//! Once every gate passes, T1, T2, T3, T4, T7 and T8 read the item's
//! changed files, the files its commits touched, with paths made relative
//! to the project's root before they're matched against its `review`
//! globs. Files outside the root still count toward T7.
//!
//! The tests for each of TR-6's criteria are in a module named after it,
//! such as `criterion_1`, so each criterion's check runs only its own.
//! Every project here is XX, whose root `<work>/xx` is a subfolder of the
//! work folder's repository, as TR's root is a folder in guild-portfolio.
//! `approve` and `reject` need a terminal, so their tests run them under
//! `script` against a throwaway store, and skip where it isn't installed.

mod common;

use std::fs;

use common::{Env, git_commit_all, git_ok};
use serde_json::{Value, json};

/// The question every human-only command asks.
const PROMPT: &str = "Type XX-1 to confirm";

/// A store with project XX at `<work>/xx`. Each of `base`'s files, a path
/// from the root and its text, is written and committed unscoped before
/// XX-1 is created, so it exists before the item's work. Then XX-1 is
/// created, with a script criterion, and started.
fn started_with(base: &[(&str, &str)]) -> Env {
    let env = Env::new();
    env.add_project("XX", "xx");
    for (path, text) in base {
        write(&env, &format!("xx/{path}"), text);
    }
    git_commit_all(env.work.path(), "chore: the code before XX-1");
    env.create("XX", "a");
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

/// Removes `path`, from the top of the work folder's repository.
fn remove(env: &Env, path: &str) {
    fs::remove_file(env.work.path().join(path)).unwrap();
}

/// Commits everything as one of XX-1's commits.
fn commit(env: &Env) -> String {
    git_commit_all(env.work.path(), "feat(XX-1): the work")
}

/// Sets XX's `review` settings in `config.json`, as a user would by hand.
fn set_review(env: &Env, review: Value) {
    set_project_field(env, "review", review);
}

/// Sets one of XX's settings in `config.json`.
fn set_project_field(env: &Env, field: &str, value: Value) {
    let path = env.home.path().join("config.json");
    let mut config: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    config["projects"]["XX"][field] = value;
    fs::write(&path, serde_json::to_string_pretty(&config).unwrap()).unwrap();
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

/// The names of the triggers that ran.
fn ran(output: &Value) -> Vec<String> {
    output["triggers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|ran| ran["trigger"].as_str().unwrap().to_string())
        .collect()
}

/// The hold `trigger` put on the item, from a submit's output.
fn hold<'a>(output: &'a Value, trigger: &str) -> &'a Value {
    output["holds"]
        .as_array()
        .unwrap()
        .iter()
        .find(|hold| hold["trigger"] == trigger)
        .unwrap_or_else(|| panic!("no {trigger} hold: {output}"))
}

/// Asserts that a submit was held by exactly `triggers`.
fn assert_held_by(output: &Value, triggers: &[&str]) {
    assert_eq!(output["status"], "review", "{output}");
    assert_eq!(fired(output), triggers, "{output}");
}

/// Asserts that a submit verified the item automatically, with no trigger
/// fired.
fn assert_not_held(output: &Value) {
    assert_eq!(output["status"], "done", "{output}");
    assert!(fired(output).is_empty(), "{output}");
}

/// Whether `refs/tracker/<id>` exists in the work folder's repository.
fn has_ref(env: &Env, id: &str) -> bool {
    git_ok(
        env.work.path(),
        &[
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/tracker/{id}"),
        ],
    )
}

mod criterion_1 {
    //! Each of the six triggers (T1, T2, T3, T4, T7 and T8) has one test
    //! where it fires and one where it stays quiet.
    use super::*;

    #[test]
    fn t1_fires_for_a_new_source_file() {
        let env = started_with(&[("src/lib.rs", "pub fn a() {}\n")]);
        write(&env, "xx/src/search.rs", "pub fn search() {}\n");
        commit(&env);
        let output = submit(&env);
        assert_held_by(&output, &["T1"]);
        let hold = hold(&output, "T1");
        assert!(
            hold["message"].as_str().unwrap().contains("src/search.rs"),
            "{hold}"
        );
        assert_eq!(hold["files"], json!(["src/search.rs"]));
    }

    #[test]
    fn t1_fires_for_a_capability() {
        let env = Env::new();
        env.add_project("XX", "xx");
        env.create("XX", "a");
        env.json(&["edit", "XX-1", "--kind", "capability"]);
        env.json(&["start", "XX-1"]);
        let output = submit(&env);
        assert_held_by(&output, &["T1"]);
        let message = hold(&output, "T1")["message"].as_str().unwrap();
        assert!(message.contains("capability"), "{message}");
    }

    #[test]
    fn t1_stays_quiet_for_a_change_to_an_existing_source_file() {
        let env = started_with(&[("src/lib.rs", "pub fn a() {}\n")]);
        write(&env, "xx/src/lib.rs", "pub fn a() {}\npub fn b() {}\n");
        commit(&env);
        let output = submit(&env);
        assert_not_held(&output);
        assert!(ran(&output).contains(&"T1".to_string()), "{output}");
    }

    #[test]
    fn t1_stays_quiet_for_a_new_test_file() {
        // `tests/**` is excluded from `source_paths`, even one that says so.
        let env = started_with(&[]);
        set_review(&env, json!({ "source_paths": ["src/**", "tests/**"] }));
        write(&env, "xx/tests/search.rs", "#[test]\nfn it() {}\n");
        commit(&env);
        assert_not_held(&submit(&env));
    }

    #[test]
    fn t2_fires_for_a_risky_path() {
        let env = started_with(&[("src/store.rs", "pub fn save() {}\n")]);
        set_review(&env, json!({ "risky_paths": ["src/store.rs"] }));
        write(&env, "xx/src/store.rs", "pub fn save() { todo!() }\n");
        commit(&env);
        let output = submit(&env);
        assert_held_by(&output, &["T2"]);
        assert_eq!(hold(&output, "T2")["files"], json!(["src/store.rs"]));
    }

    #[test]
    fn t2_stays_quiet_for_a_path_that_isnt_risky() {
        let env = started_with(&[("src/store.rs", "a\n"), ("src/list.rs", "a\n")]);
        set_review(&env, json!({ "risky_paths": ["src/store.rs"] }));
        write(&env, "xx/src/list.rs", "b\n");
        commit(&env);
        assert_not_held(&submit(&env));
    }

    #[test]
    fn t3_fires_for_a_changed_manifest() {
        let env = started_with(&[("Cargo.toml", "[package]\nname = \"xx\"\n")]);
        write(
            &env,
            "xx/Cargo.toml",
            "[package]\nname = \"xx\"\n\n[dependencies]\nfuse = \"1\"\n",
        );
        commit(&env);
        let output = submit(&env);
        assert_held_by(&output, &["T3"]);
        let hold = hold(&output, "T3");
        assert!(
            hold["message"].as_str().unwrap().contains("Cargo.toml"),
            "{hold}"
        );
        assert_eq!(hold["files"], json!(["Cargo.toml"]));
    }

    #[test]
    fn t3_stays_quiet_when_no_manifest_changed() {
        let env = started_with(&[("Cargo.toml", "[package]\n"), ("README.md", "a\n")]);
        write(&env, "xx/README.md", "b\n");
        commit(&env);
        assert_not_held(&submit(&env));
    }

    #[test]
    fn t4_fires_for_a_changed_line_in_an_existing_test_file() {
        let env = started_with(&[("tests/list.rs", "fn one() { assert!(true) }\n")]);
        write(&env, "xx/tests/list.rs", "fn one() { assert!(1 == 1) }\n");
        commit(&env);
        let output = submit(&env);
        assert_held_by(&output, &["T4"]);
        let hold = hold(&output, "T4");
        assert!(
            hold["message"].as_str().unwrap().contains("tests/list.rs"),
            "{hold}"
        );
        assert_eq!(hold["files"], json!(["tests/list.rs"]));
    }

    #[test]
    fn t4_stays_quiet_for_a_new_test_file() {
        let env = started_with(&[("tests/list.rs", "fn one() {}\n")]);
        write(&env, "xx/tests/search.rs", "fn two() {}\n");
        commit(&env);
        assert_not_held(&submit(&env));
    }

    #[test]
    fn t7_fires_for_more_changed_lines_than_max_diff_lines() {
        let env = started_with(&[]);
        set_review(&env, json!({ "max_diff_lines": 5 }));
        write(&env, "xx/notes.txt", &"a line\n".repeat(6));
        commit(&env);
        let output = submit(&env);
        assert_held_by(&output, &["T7"]);
        let message = hold(&output, "T7")["message"].as_str().unwrap();
        assert!(message.contains('6') && message.contains('5'), "{message}");
    }

    #[test]
    fn t7_fires_by_default_above_300_lines() {
        let env = started_with(&[]);
        write(&env, "xx/notes.txt", &"a line\n".repeat(301));
        commit(&env);
        assert_held_by(&submit(&env), &["T7"]);
    }

    #[test]
    fn t7_counts_lines_changed_outside_the_root() {
        let env = started_with(&[]);
        set_review(&env, json!({ "max_diff_lines": 5 }));
        write(&env, "elsewhere/notes.txt", &"a line\n".repeat(6));
        commit(&env);
        assert_held_by(&submit(&env), &["T7"]);
    }

    #[test]
    fn t7_stays_quiet_at_max_diff_lines() {
        let env = started_with(&[]);
        set_review(&env, json!({ "max_diff_lines": 5 }));
        write(&env, "xx/notes.txt", &"a line\n".repeat(5));
        commit(&env);
        assert_not_held(&submit(&env));
    }

    #[test]
    fn t8_fires_when_the_design_doc_changes() {
        let env = started_with(&[("docs/DESIGN.md", "# Design\n")]);
        set_project_field(&env, "design", json!("docs/DESIGN.md"));
        write(&env, "xx/docs/DESIGN.md", "# Design\n\nMore.\n");
        commit(&env);
        let output = submit(&env);
        assert_held_by(&output, &["T8"]);
        assert_eq!(hold(&output, "T8")["files"], json!(["docs/DESIGN.md"]));
    }

    #[test]
    fn t8_stays_quiet_when_another_doc_changes() {
        let env = started_with(&[("docs/DESIGN.md", "# Design\n"), ("docs/notes.md", "a\n")]);
        set_project_field(&env, "design", json!("docs/DESIGN.md"));
        write(&env, "xx/docs/notes.md", "b\n");
        commit(&env);
        assert_not_held(&submit(&env));
    }

    #[test]
    fn the_gates_dry_run_lists_the_same_holds() {
        let env = started_with(&[("Cargo.toml", "a\n")]);
        write(&env, "xx/Cargo.toml", "b\n");
        commit(&env);
        let output = env.json(&["gates", "XX-1"]);
        assert_eq!(fired(&output), ["T3"], "{output}");
        assert_eq!(output["holds"][0]["files"], json!(["Cargo.toml"]));
    }
}

mod criterion_2 {
    //! Adding a test case to an existing test file doesn't fire T4.
    //! Changing a line in one does, and so does deleting one.
    use super::*;

    const TEST_FILE: &str = "#[test]\nfn one() {\n    assert_eq!(1 + 1, 2);\n}\n";

    #[test]
    fn adding_a_test_case_to_an_existing_test_file_doesnt_fire_t4() {
        let env = started_with(&[("tests/sums.rs", TEST_FILE)]);
        let more = format!("{TEST_FILE}\n#[test]\nfn two() {{\n    assert_eq!(2 + 2, 4);\n}}\n");
        write(&env, "xx/tests/sums.rs", &more);
        commit(&env);
        assert_not_held(&submit(&env));
    }

    #[test]
    fn changing_a_line_in_an_existing_test_file_fires_t4() {
        let env = started_with(&[("tests/sums.rs", TEST_FILE)]);
        write(
            &env,
            "xx/tests/sums.rs",
            &TEST_FILE.replace("assert_eq!(1 + 1, 2)", "assert!(true)"),
        );
        commit(&env);
        let output = submit(&env);
        assert_held_by(&output, &["T4"]);
        let message = hold(&output, "T4")["message"].as_str().unwrap();
        assert!(message.contains("modified: tests/sums.rs"), "{message}");
    }

    #[test]
    fn deleting_an_existing_test_file_fires_t4() {
        let env = started_with(&[("tests/sums.rs", TEST_FILE)]);
        remove(&env, "xx/tests/sums.rs");
        commit(&env);
        let output = submit(&env);
        assert_held_by(&output, &["T4"]);
        let hold = hold(&output, "T4");
        let message = hold["message"].as_str().unwrap();
        assert!(message.contains("deleted: tests/sums.rs"), "{message}");
        assert_eq!(hold["files"], json!(["tests/sums.rs"]));
    }

    #[test]
    fn renaming_an_existing_test_file_fires_t4() {
        // A rename deletes the old file, so it counts as one.
        let env = started_with(&[("tests/sums.rs", TEST_FILE)]);
        remove(&env, "xx/tests/sums.rs");
        write(&env, "xx/tests/adding.rs", TEST_FILE);
        commit(&env);
        let message = hold(&submit(&env), "T4")["message"].clone();
        assert!(
            message.as_str().unwrap().contains("tests/sums.rs"),
            "{message}"
        );
    }

    #[test]
    fn changing_a_test_file_the_item_added_doesnt_fire_t4() {
        // The item's own test commit added it, so it didn't exist before
        // the item's work.
        let env = started_with(&[]);
        write(&env, "xx/tests/sums.rs", TEST_FILE);
        git_commit_all(env.work.path(), "test(XX-1): add the test");
        write(
            &env,
            "xx/tests/sums.rs",
            &TEST_FILE.replace("1 + 1", "0 + 2"),
        );
        commit(&env);
        assert_not_held(&submit(&env));
    }
}

mod criterion_3 {
    //! In a subfolder project, `source_paths: ["src/**"]` matches
    //! `<root>/src/…` and not a `src/` folder elsewhere in the repo.
    use super::*;

    #[test]
    fn source_paths_match_src_under_the_subfolder_root() {
        let env = started_with(&[]);
        set_review(&env, json!({ "source_paths": ["src/**"] }));
        write(&env, "xx/src/deep/search.rs", "pub fn search() {}\n");
        commit(&env);
        let output = submit(&env);
        assert_held_by(&output, &["T1"]);
        assert_eq!(hold(&output, "T1")["files"], json!(["src/deep/search.rs"]));
    }

    #[test]
    fn source_paths_dont_match_a_src_folder_elsewhere_in_the_repo() {
        let env = started_with(&[]);
        set_review(&env, json!({ "source_paths": ["src/**"] }));
        // At the top of the repository, in another folder, and in a folder
        // inside the root that isn't its `src/`.
        write(&env, "src/top.rs", "pub fn top() {}\n");
        write(&env, "other/src/other.rs", "pub fn other() {}\n");
        write(&env, "xx/lib/src/nested.rs", "pub fn nested() {}\n");
        commit(&env);
        assert_not_held(&submit(&env));
    }

    #[test]
    fn globs_match_from_the_root_not_from_the_top_of_the_repo() {
        // `xx/src/**` would match a path from the top of the repository,
        // so it must match nothing.
        let env = started_with(&[]);
        set_review(&env, json!({ "source_paths": ["xx/src/**"] }));
        write(&env, "xx/src/search.rs", "pub fn search() {}\n");
        commit(&env);
        assert_not_held(&submit(&env));
    }
}

mod criterion_4 {
    //! `"disabled": ["T8"]` silences T8, but `["T6"]` doesn't silence T6.
    use super::*;

    #[test]
    fn disabling_t8_silences_it() {
        let env = started_with(&[("docs/DESIGN.md", "# Design\n")]);
        set_project_field(&env, "design", json!("docs/DESIGN.md"));
        set_review(&env, json!({ "disabled": ["T8"] }));
        write(&env, "xx/docs/DESIGN.md", "# Design\n\nMore.\n");
        commit(&env);
        let output = submit(&env);
        assert_not_held(&output);
        // A trigger switched off doesn't run, so it isn't listed (§3).
        assert!(!ran(&output).contains(&"T8".to_string()), "{output}");
        assert!(ran(&output).contains(&"T1".to_string()), "{output}");
    }

    #[test]
    fn disabling_t6_doesnt_silence_it() {
        let env = Env::new();
        env.add_project("XX", "xx");
        set_review(&env, json!({ "disabled": ["T6", "T10", "T12"] }));
        env.create_bare("XX", "manual");
        env.json(&["criterion", "add", "XX-1", "the page looks right"]);
        env.json(&["evidence", "XX-1", "1", "see the screenshot"]);
        env.json(&["start", "XX-1"]);
        let output = submit(&env);
        assert_held_by(&output, &["T6"]);
        assert!(ran(&output).contains(&"T6".to_string()), "{output}");
    }
}

mod criterion_5 {
    //! After a reject, a resubmit is held again for every trigger that
    //! still applies.
    use super::*;

    #[test]
    fn a_resubmit_after_a_reject_is_held_again_for_every_trigger_that_applies() {
        let env = started_with(&[
            ("Cargo.toml", "[package]\n"),
            ("tests/list.rs", "fn one() { assert!(true) }\n"),
            ("docs/DESIGN.md", "# Design\n"),
        ]);
        set_project_field(&env, "design", json!("docs/DESIGN.md"));
        env.json(&["edit", "XX-1", "--label", "needs-review"]);
        write(&env, "xx/Cargo.toml", "[package]\nname = \"xx\"\n");
        write(&env, "xx/docs/DESIGN.md", "# Design\n\nMore.\n");
        commit(&env);
        let first = submit(&env);
        assert_held_by(&first, &["T3", "T8", "T9"]);

        let Some((output, ok)) = env.in_terminal_typing(
            &[
                "reject",
                "XX-1",
                "--reason",
                "Say why the dependency is needed.",
            ],
            PROMPT,
            "XX-1",
            || {},
        ) else {
            return;
        };
        assert!(ok, "{output}");
        assert_eq!(env.item("XX-1")["status"], "in-progress");

        // The label comes off, and the rework edits an existing test. T3
        // and T8 still apply, from the first commit, and T4 now does too.
        env.json(&["edit", "XX-1", "--unlabel", "needs-review"]);
        write(&env, "xx/tests/list.rs", "fn one() { assert!(1 == 1) }\n");
        commit(&env);
        let second = submit(&env);
        assert_held_by(&second, &["T3", "T4", "T8"]);
        assert_eq!(hold(&second, "T3"), hold(&first, "T3"));
        let item = env.item("XX-1");
        let holds: Vec<&str> = item["holds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|hold| hold["trigger"].as_str().unwrap())
            .collect();
        assert_eq!(holds, ["T3", "T4", "T8"]);
    }
}

mod criterion_6 {
    //! `approve` runs the triggers again before asking, at the `HEAD` it
    //! names, shows every hold that fires there, marking new ones, and
    //! records them. It exits 1, writing nothing, if `HEAD` moves before
    //! its gates run (slice 9 review 1, design 1).
    use super::*;

    /// XX-1, held under T9 by its submit.
    fn held() -> Env {
        let env = started_with(&[("Cargo.toml", "[package]\n")]);
        env.json(&["edit", "XX-1", "--label", "needs-review"]);
        assert_held_by(&submit(&env), &["T9"]);
        env
    }

    #[test]
    fn approve_shows_and_records_a_hold_from_a_commit_made_after_the_submit() {
        let env = held();
        write(&env, "xx/Cargo.toml", "[package]\nname = \"xx\"\n");
        let head = commit(&env);

        let Some((output, ok)) =
            env.in_terminal_typing(&["approve", "XX-1"], PROMPT, "XX-1", || {})
        else {
            return;
        };
        assert!(ok, "{output}");
        let t3 = output
            .lines()
            .find(|line| line.contains("T3"))
            .unwrap_or_else(|| panic!("no T3 line: {output}"));
        assert!(t3.contains("Cargo.toml"), "{t3}");
        assert!(t3.contains("new since the submit"), "{t3}");
        let t9 = output.lines().find(|line| line.contains("T9")).unwrap();
        assert!(!t9.contains("new since the submit"), "{t9}");
        assert!(output.contains(&head[..7]), "{output}");

        let item = env.item("XX-1");
        assert_eq!(item["status"], "done", "{item}");
        let record = &item["verification"];
        assert_eq!(record["commit"], head.as_str(), "{record}");
        let triggers: Vec<&str> = record["triggers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|hold| hold["trigger"].as_str().unwrap())
            .collect();
        assert_eq!(triggers, ["T3", "T9"], "{record}");
        common::check_schema("item", &item);
    }

    #[test]
    fn approve_writes_nothing_when_head_moves_while_it_waits() {
        let env = held();
        let before = fs::read(env.item_path("XX-1")).unwrap();
        let Some((output, ok)) =
            env.in_terminal_typing(&["approve", "XX-1"], PROMPT, "XX-1", || {
                write(&env, "xx/Cargo.toml", "[package]\nname = \"xx\"\n");
                commit(&env);
            })
        else {
            return;
        };
        assert!(!ok, "{output}");
        assert!(output.contains("HEAD moved"), "{output}");
        assert_eq!(fs::read(env.item_path("XX-1")).unwrap(), before);
        assert!(!has_ref(&env, "XX-1"));
    }
}

mod criterion_7 {
    //! `show --diff` lists the files that fired a trigger before the
    //! commits and their diff.
    use super::*;

    #[test]
    fn show_diff_lists_the_files_that_fired_a_trigger_first() {
        let env = started_with(&[
            ("Cargo.toml", "[package]\n"),
            ("tests/list.rs", "fn one() { assert!(true) }\n"),
        ]);
        write(&env, "xx/README.md", "read me\n");
        write(&env, "xx/Cargo.toml", "[package]\nname = \"xx\"\n");
        write(&env, "xx/tests/list.rs", "fn one() {}\n");
        commit(&env);
        assert_held_by(&submit(&env), &["T3", "T4"]);

        let output = env
            .cmd()
            .args(["show", "XX-1", "--diff"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let text = String::from_utf8(output).unwrap();
        let listed = text
            .find("Files that fired a trigger:")
            .unwrap_or_else(|| panic!("{text}"));
        let commits = text.find("Commits,").unwrap();
        let diff = text.find("Diff,").unwrap();
        assert!(listed < commits && commits < diff, "{text}");
        let section: Vec<&str> = text[listed..commits]
            .lines()
            .skip(1)
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();
        assert_eq!(section, ["T3  Cargo.toml", "T4  tests/list.rs"], "{text}");

        let json = env.json(&["show", "XX-1", "--diff"]);
        assert_eq!(json["item"]["holds"][0]["files"], json!(["Cargo.toml"]));
    }
}

/// The folder rule's other half: a file outside the root is a changed file,
/// but it matches none of the project's globs, even one that would match
/// its path from the top of the repository.
#[test]
fn a_file_outside_the_root_matches_no_glob() {
    let env = started_with(&[]);
    set_review(&env, json!({ "risky_paths": ["**"] }));
    write(&env, "elsewhere/Cargo.toml", "[package]\n");
    commit(&env);
    assert_not_held(&submit(&env));
}

/// A `review` glob that can't be read is refused when the config loads,
/// with exit 3, as a bad root is, rather than leaving a trigger unable to
/// run at a submit.
#[test]
fn an_invalid_review_glob_exits_3_naming_it() {
    let env = Env::new();
    env.add_project("XX", "xx");
    set_review(&env, json!({ "risky_paths": ["src/[store.rs"] }));
    let message = env.error(&["list"], 3);
    assert!(message.contains("src/[store.rs"), "{message}");
}

/// A store with project XX whose item XX-1 is created, has each of `tests`
/// (a path from the root and its text) added by a `test(XX-1)` commit, as
/// every TR item's tests are, and is then started.
fn started_after_tests(tests: &[(&str, &str)]) -> Env {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");
    for (path, text) in tests {
        write(&env, &format!("xx/{path}"), text);
    }
    git_commit_all(env.work.path(), "test(XX-1): add the tests");
    env.json(&["start", "XX-1"]);
    env
}

const OWN_TEST: &str = "#[test]\nfn one() {\n    assert_eq!(1 + 1, 2);\n}\n";

/// TR-31, criterion 1: a test the item committed before `start`, and then
/// changed or removed a line of after it, fires T4.
mod tr_31_c1 {
    use super::*;

    #[test]
    fn changing_a_line_of_a_test_committed_before_start_fires_t4() {
        let env = started_after_tests(&[("tests/own.rs", OWN_TEST)]);
        write(
            &env,
            "xx/tests/own.rs",
            &OWN_TEST.replace("assert_eq!(1 + 1, 2)", "assert!(true)"),
        );
        commit(&env);
        let output = submit(&env);
        assert_held_by(&output, &["T4"]);
        let hold = hold(&output, "T4");
        assert_eq!(hold["files"], json!(["tests/own.rs"]));
        assert!(
            hold["message"].as_str().unwrap().contains("tests/own.rs"),
            "{hold}"
        );
    }

    #[test]
    fn removing_a_test_committed_before_start_fires_t4() {
        let env = started_after_tests(&[("tests/own.rs", OWN_TEST)]);
        remove(&env, "xx/tests/own.rs");
        commit(&env);
        let output = submit(&env);
        assert_held_by(&output, &["T4"]);
        assert_eq!(hold(&output, "T4")["files"], json!(["tests/own.rs"]));
    }
}

/// TR-31, criterion 2: adding lines to such a test after `start` doesn't.
mod tr_31_c2 {
    use super::*;

    #[test]
    fn adding_lines_to_a_test_committed_before_start_doesnt_fire_t4() {
        let env = started_after_tests(&[("tests/own.rs", OWN_TEST)]);
        let more = format!("{OWN_TEST}\n#[test]\nfn two() {{\n    assert_eq!(2 + 2, 4);\n}}\n");
        write(&env, "xx/tests/own.rs", &more);
        commit(&env);
        assert_not_held(&submit(&env));
    }
}

/// TR-32, criterion 1: `diff.relative` in the repository's own config
/// doesn't hide changes from the triggers or from `show --diff`.
mod tr_32_c1 {
    use super::*;

    fn set_relative(env: &Env) {
        assert!(git_ok(
            env.work.path(),
            &["config", "diff.relative", "true"]
        ));
    }

    #[test]
    fn the_file_triggers_still_fire_with_diff_relative_set() {
        let env = started_with(&[("src/lib.rs", "pub fn a() {}\n")]);
        set_relative(&env);
        write(&env, "xx/src/search.rs", "pub fn search() {}\n");
        commit(&env);
        let output = submit(&env);
        assert_held_by(&output, &["T1"]);
        assert_eq!(hold(&output, "T1")["files"], json!(["src/search.rs"]));
    }

    #[test]
    fn show_diff_still_lists_files_outside_the_root_with_diff_relative_set() {
        let env = started_with(&[]);
        set_relative(&env);
        write(&env, "xx/inside.txt", "inside the root\n");
        write(&env, "other/outside.txt", "outside the root\n");
        commit(&env);
        let output = env.json(&["show", "XX-1", "--diff"]);
        let diff = output["diff"].as_str().unwrap();
        assert!(diff.contains("inside.txt"), "{diff}");
        assert!(diff.contains("other/outside.txt"), "{diff}");
    }
}

/// TR-33, criterion 1: T4's hold names each file's commits that changed or
/// removed its lines, each marked as before or after `start`.
mod tr_33_c1 {
    use super::*;

    const BASE: &str = "#[test]\nfn one() {\n    assert_eq!(1 + 1, 2);\n}\n";

    /// XX-1 with `tests/old.rs` from an unscoped commit, a sanctioned edit
    /// of it in a `test(XX-1)` commit before `start`, and then a weakening
    /// in a `feat(XX-1)` commit after it. Returns the env and the short
    /// SHAs of the commit before and the commit after.
    fn edited_before_and_after_start() -> (Env, String, String) {
        let env = Env::new();
        env.add_project("XX", "xx");
        write(&env, "xx/tests/old.rs", BASE);
        git_commit_all(env.work.path(), "chore: the code before XX-1");
        env.create("XX", "a");
        write(
            &env,
            "xx/tests/old.rs",
            &BASE.replace("1 + 1, 2", "1 + 2, 3"),
        );
        let before = git_commit_all(env.work.path(), "test(XX-1): update the expected value");
        env.json(&["start", "XX-1"]);
        write(&env, "xx/tests/old.rs", "#[test]\nfn one() {}\n");
        let after = commit(&env);
        (env, before[..7].to_string(), after[..7].to_string())
    }

    #[test]
    fn the_hold_names_each_commit_marked_before_or_after_start() {
        let (env, before, after) = edited_before_and_after_start();
        let output = submit(&env);
        assert_held_by(&output, &["T4"]);
        let hold = hold(&output, "T4");
        let message = hold["message"].as_str().unwrap();
        assert!(message.contains("tests/old.rs"), "{message}");
        assert!(
            message.contains(&format!("{before} before start")),
            "{message}"
        );
        assert!(
            message.contains(&format!("{after} after start")),
            "{message}"
        );
        assert_eq!(hold["files"], json!(["tests/old.rs"]));
    }

    #[test]
    fn the_text_output_names_them_too() {
        let (env, before, after) = edited_before_and_after_start();
        let output = env.cmd().args(["submit", "XX-1"]).output().unwrap();
        let text = String::from_utf8_lossy(&output.stdout).into_owned()
            + &String::from_utf8_lossy(&output.stderr);
        assert!(text.contains(&format!("{before} before start")), "{text}");
        assert!(text.contains(&format!("{after} after start")), "{text}");
    }

    #[test]
    fn a_deleted_file_names_its_commit_too() {
        let env = started_with(&[("tests/old.rs", BASE)]);
        remove(&env, "xx/tests/old.rs");
        let sha = commit(&env);
        let message = hold(&submit(&env), "T4")["message"].clone();
        let message = message.as_str().unwrap();
        assert!(
            message.contains(&format!("{} after start", &sha[..7])),
            "{message}"
        );
    }
}

/// TR-34, criterion 1: an unknown key in a project's `review` settings is
/// refused, naming it.
mod tr_34_c1 {
    use super::*;

    #[test]
    fn a_misspelled_review_key_exits_3_naming_it() {
        let env = Env::new();
        env.add_project("XX", "xx");
        set_review(&env, json!({ "risky_path": ["src/store.rs"] }));
        let message = env.error(&["list"], 3);
        assert!(message.contains("risky_path"), "{message}");
    }
}

/// TR-34, criterion 2: a `review` with every key §4.2 lists still loads.
mod tr_34_c2 {
    use super::*;

    #[test]
    fn a_review_with_every_key_in_4_2_loads() {
        let env = Env::new();
        env.add_project("XX", "xx");
        set_review(
            &env,
            json!({
                "risky_paths": ["src/store.rs"],
                "source_paths": ["src/**"],
                "test_paths": ["tests/**"],
                "manifests": ["**/Cargo.toml"],
                "max_diff_lines": 200,
                "disabled": ["T8"],
                "digest": { "every_days": 7 },
            }),
        );
        env.json(&["list"]);
    }
}

/// TR-35, criterion 1: `approve`'s question lists a hold from the submit
/// that no longer fires.
mod tr_35_c1 {
    use super::*;

    #[test]
    fn approve_lists_a_submit_hold_that_no_longer_fires() {
        let env = started_with(&[("Cargo.toml", "[package]\n")]);
        write(&env, "xx/Cargo.toml", "[package]\nname = \"xx\"\n");
        commit(&env);
        assert_held_by(&submit(&env), &["T3"]);
        // T3 is switched off while the item waits, so it won't fire again.
        set_review(&env, json!({ "disabled": ["T3"] }));

        let Some((output, ok)) =
            env.in_terminal_typing(&["approve", "XX-1"], PROMPT, "XX-1", || {})
        else {
            return;
        };
        assert!(ok, "{output}");
        let t3 = output
            .lines()
            .find(|line| line.contains("T3"))
            .unwrap_or_else(|| panic!("no T3 line: {output}"));
        assert!(t3.contains("no longer fires"), "{t3}");
        assert!(!output.contains("no trigger recorded"), "{output}");
    }
}

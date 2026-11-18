//! Slice 6: gates G1 (at `start`), G2 and G6 (at `submit` and in the
//! `gates` dry run), as DESIGN.md §2.4 describes. A failure lists every
//! reason, changes nothing and exits 2.

mod common;

use std::fs;
use std::path::PathBuf;

use common::Env;
use serde_json::{Value, json};

/// A store with project XX, whose root is `<work>/xx`, and that root.
fn env_with_project() -> (Env, PathBuf) {
    let env = Env::new();
    let root = env.add_project("XX", "xx");
    (env, root)
}

/// Runs a command that a gate must refuse: exit 2, the error shape with
/// `id`, `status`, `result`, `gates` and `failures`, and XX-1's file
/// byte-for-byte unchanged. Returns the error object.
fn refused_by_gate(env: &Env, args: &[&str]) -> Value {
    let id = args[1..]
        .iter()
        .find(|arg| arg.contains('-'))
        .unwrap()
        .to_uppercase();
    let before = fs::read(env.item_path(&id)).unwrap();
    let error = env.error_json_in(&env.outside(), args, 2);
    assert_eq!(
        fs::read(env.item_path(&id)).unwrap(),
        before,
        "{args:?} changed the file"
    );
    assert_eq!(error["id"], id.as_str(), "{error}");
    assert_eq!(error["status"], env.item(&id)["status"], "{error}");
    assert_eq!(error["result"], "refused", "{error}");
    let message = error["error"].as_str().unwrap().to_lowercase();
    assert!(message.contains("nothing was changed"), "{error}");
    error
}

/// Each failure's gate and message.
fn failures(error: &Value) -> Vec<(String, String)> {
    error["failures"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["gate"].as_str().unwrap().to_string(),
                f["message"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

/// Sets an item's status by hand, for states no command reaches yet.
fn set_status(env: &Env, id: &str, status: &str) {
    let mut item = env.item(id);
    item["status"] = json!(status);
    fs::write(
        env.item_path(id),
        serde_json::to_string_pretty(&item).unwrap() + "\n",
    )
    .unwrap();
}

/// Gives XX-1 the design reference `design` by hand, as `edit --design`
/// would store it, relative to the root, without its checks.
fn set_design(env: &Env, design: &str) {
    let mut item = env.item("XX-1");
    item["design"] = json!(design);
    fs::write(
        env.item_path("XX-1"),
        serde_json::to_string_pretty(&item).unwrap() + "\n",
    )
    .unwrap();
}

/// A design doc: sections 1, 2 and 2.4 have something under them, and 2.5
/// and 3 don't. The `## 4.` line is in a code block, so it isn't a heading.
const DESIGN: &str = "# Design\n\n## 1. Overview\n\nWhat it is.\n\n## 2. Concepts\n\n\
                      ### 2.4 Gates\n\nThe gates.\n\n```markdown\n## 4. In a code block\n\n\
                      text\n```\n\n### 2.5 Empty\n\n## 3. Interface\n";

// G1

#[test]
fn start_with_no_criteria_is_refused_by_g1_with_exit_2() {
    let (env, _) = env_with_project();
    env.create_bare("XX", "a");
    for args in [&["start", "XX-1"][..], &["status", "XX-1", "in-progress"]] {
        let error = refused_by_gate(&env, args);
        assert_eq!(error["gates"], json!([{ "gate": "G1", "ok": false }]));
        let failures = failures(&error);
        assert_eq!(failures.len(), 1, "{failures:?}");
        assert_eq!(failures[0].0, "G1");
        assert!(failures[0].1.contains("has no criteria"), "{failures:?}");
        assert!(
            failures[0].1.contains("tracker criterion add XX-1"),
            "{failures:?}"
        );
    }
    // Once it has one, it starts.
    env.add_criterion("XX-1");
    assert_eq!(
        env.json(&["start", "XX-1"])["item"]["status"],
        "in-progress"
    );
}

#[test]
fn start_while_another_item_in_the_project_is_in_progress_is_refused_by_g1() {
    let (env, _) = env_with_project();
    env.add_project("YY", "yy");
    env.create("XX", "a");
    env.create("XX", "b");
    env.create("YY", "c");
    env.json(&["start", "XX-1"]);

    let error = refused_by_gate(&env, &["start", "XX-2"]);
    let failures = failures(&error);
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert!(
        failures[0].1.contains("XX-1 is in progress"),
        "{failures:?}"
    );
    assert!(
        failures[0].1.contains("Submit or shelve XX-1"),
        "{failures:?}"
    );

    // Another project's items don't count.
    env.json(&["start", "YY-1"]);
    // Shelving XX-1 frees the project (§2.8).
    env.json(&["shelve", "XX-1", "--reason", "later"]);
    env.json(&["start", "XX-2"]);
}

#[test]
fn start_while_another_item_is_in_review_is_refused_by_g1() {
    let (env, _) = env_with_project();
    env.create("XX", "a");
    env.create("XX", "b");
    set_status(&env, "XX-1", "review");
    let error = refused_by_gate(&env, &["start", "XX-2"]);
    let failures = failures(&error);
    assert!(failures[0].1.contains("XX-1 is in review"), "{failures:?}");

    // Done, shelved and open items don't count.
    set_status(&env, "XX-1", "done");
    env.json(&["start", "XX-2"]);
}

#[test]
fn g1_fails_while_an_item_file_in_the_project_cant_be_read() {
    // The file may be an item in progress, so G1 fails rather than leave
    // it out, as `list` would (review 1's finding 5).
    let (env, _) = env_with_project();
    env.add_project("YY", "yy");
    env.create("XX", "a");
    env.create("XX", "b");
    env.create("YY", "c");
    env.json(&["start", "XX-1"]);
    let broken = env.item_path("XX-1");
    let text = fs::read_to_string(&broken).unwrap().replacen(',', "", 1);
    fs::write(&broken, text).unwrap();

    for args in [&["start", "XX-2"][..], &["status", "XX-2", "in-progress"]] {
        let error = refused_by_gate(&env, args);
        let failures = failures(&error);
        assert_eq!(failures.len(), 1, "{failures:?}");
        assert_eq!(failures[0].0, "G1");
        let message = &failures[0].1;
        assert!(
            message.contains("Can't tell whether XX-1 is in progress"),
            "{message}"
        );
        assert!(message.contains("XX-1.json, line"), "{message}");
        assert!(message.contains("move it out of items/"), "{message}");
    }

    // Another project's file, and one not named after an item, don't
    // count, since neither can be an item in YY.
    let items = broken.parent().unwrap();
    fs::write(items.join("notes.json"), "{").unwrap();
    env.json(&["start", "YY-1"]);

    // Once the broken file is moved out, XX-2 can start.
    fs::rename(&broken, env.outside().join("XX-1.json")).unwrap();
    env.json(&["start", "XX-2"]);
}

#[test]
fn g1_lists_every_reason_it_fails() {
    let (env, _) = env_with_project();
    env.create("XX", "a");
    env.create("XX", "b");
    env.create_bare("XX", "c");
    env.json(&["start", "XX-1"]);
    set_status(&env, "XX-2", "review");

    let error = refused_by_gate(&env, &["start", "XX-3"]);
    let messages: Vec<String> = failures(&error).into_iter().map(|f| f.1).collect();
    assert_eq!(messages.len(), 3, "{messages:?}");
    assert!(messages[0].contains("has no criteria"), "{messages:?}");
    assert!(messages[1].contains("XX-1 is in progress"), "{messages:?}");
    assert!(messages[2].contains("XX-2 is in review"), "{messages:?}");
    assert_eq!(
        error["error"],
        "XX-3 can't be started: G1 failed. Nothing was changed."
    );

    // The text lists them under the error, one per line.
    let output = env.cmd().args(["start", "XX-3"]).assert().code(2);
    let stderr = String::from_utf8_lossy(&output.get_output().stderr).to_string();
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 4, "{stderr}");
    assert_eq!(
        lines[0],
        "error: XX-3 can't be started: G1 failed. Nothing was changed."
    );
    for (line, message) in lines[1..].iter().zip(&messages) {
        assert_eq!(*line, format!("  G1  {message}"), "{stderr}");
    }
}

#[test]
fn a_move_the_diagram_doesnt_draw_is_still_exit_1_not_a_gate() {
    // The state is checked before the gates: a done item has no start.
    let (env, _) = env_with_project();
    env.create_bare("XX", "a");
    set_status(&env, "XX-1", "done");
    let error = env.error_json_in(&env.outside(), &["start", "XX-1"], 1);
    assert!(error.get("allowed").is_some(), "{error}");
    assert!(error.get("failures").is_none(), "{error}");
}

#[test]
fn a_newer_trackers_file_is_refused_with_exit_3_before_any_gate_runs() {
    // A judge criterion (slice 12) has `ask`, which this version doesn't
    // know. Read without it, the criterion would look manual, and G2 would
    // ask for evidence that `evidence` then refuses.
    let (env, _) = env_with_project();
    env.create("XX", "a");
    let mut item = env.item("XX-1");
    item["criteria"] = json!([{
        "text": "Errors say what to do",
        "ask": ["Does each error message say how to fix the problem?"],
    }]);
    fs::write(
        env.item_path("XX-1"),
        serde_json::to_string_pretty(&item).unwrap() + "\n",
    )
    .unwrap();

    let refused_as_newer = |args: &[&str]| {
        let before = fs::read(env.item_path("XX-1")).unwrap();
        let error = env.error_json_in(&env.outside(), args, 3);
        assert_eq!(fs::read(env.item_path("XX-1")).unwrap(), before, "{args:?}");
        let message = error["error"].as_str().unwrap();
        assert!(message.contains("newer tracker"), "{args:?}: {message}");
        assert!(message.contains("criteria"), "{args:?}: {message}");
        assert!(
            message.contains("Nothing was changed"),
            "{args:?}: {message}"
        );
        assert!(error.get("failures").is_none(), "{args:?}: {error}");
    };
    for args in [
        &["gates", "XX-1"][..],
        &["start", "XX-1"],
        &["status", "XX-1", "in-progress"],
    ] {
        refused_as_newer(args);
    }
    set_status(&env, "XX-1", "in-progress");
    for args in [&["submit", "XX-1"][..], &["status", "XX-1", "done"]] {
        refused_as_newer(args);
    }
}

// G2

#[test]
fn a_manual_criterion_with_no_evidence_fails_g2() {
    let (env, _) = env_with_project();
    env.json(&[
        "create",
        "a",
        "--project",
        "XX",
        "--done-when",
        "Looks right",
    ]);
    env.json(&["start", "XX-1"]);

    for args in [
        &["gates", "XX-1"][..],
        &["submit", "XX-1"],
        &["status", "XX-1", "done"],
    ] {
        let error = refused_by_gate(&env, args);
        assert_eq!(
            error["gates"],
            json!([{ "gate": "G2", "ok": false }, { "gate": "G6", "ok": true }]),
            "{args:?}"
        );
        assert_eq!(error["failures"][0]["gate"], "G2");
        assert_eq!(error["failures"][0]["criterion"], 1);
        let message = error["failures"][0]["message"].as_str().unwrap();
        assert!(message.contains("\"Looks right\""), "{message}");
        assert!(message.contains("tracker evidence XX-1 1"), "{message}");
    }
    assert_eq!(env.item("XX-1")["status"], "in-progress");

    // With evidence it passes, and so does the submit.
    env.json(&["evidence", "XX-1", "1", "the home page"]);
    env.json(&["gates", "XX-1"]);
    assert_eq!(env.json(&["submit", "XX-1"])["status"], "done");
}

#[test]
fn an_item_with_no_criteria_fails_g2() {
    // G1 needs a criterion at start, but it can be removed afterwards, and
    // an item started before slice 6 has none (review 1's finding 6).
    let (env, _) = env_with_project();
    env.create("XX", "a");
    env.json(&["start", "XX-1"]);
    env.json(&["criterion", "remove", "XX-1", "1"]);

    for args in [
        &["gates", "XX-1"][..],
        &["submit", "XX-1"],
        &["status", "XX-1", "done"],
    ] {
        let error = refused_by_gate(&env, args);
        assert_eq!(
            error["gates"],
            json!([{ "gate": "G2", "ok": false }, { "gate": "G6", "ok": true }]),
            "{args:?}"
        );
        let failures = error["failures"].as_array().unwrap();
        assert_eq!(failures.len(), 1, "{error}");
        assert_eq!(failures[0]["gate"], "G2");
        assert!(failures[0].get("criterion").is_none(), "{error}");
        let message = failures[0]["message"].as_str().unwrap();
        assert!(message.contains("XX-1 has no criteria"), "{message}");
        assert!(message.contains("tracker criterion add XX-1"), "{message}");
    }

    // With a criterion again, it's submitted.
    env.add_criterion("XX-1");
    assert_eq!(env.json(&["submit", "XX-1"])["status"], "done");
}

#[test]
fn a_script_criterion_needs_no_evidence() {
    let (env, _) = env_with_project();
    env.create("XX", "a");
    let output = env.json(&["gates", "XX-1"]);
    assert_eq!(
        output,
        json!({
            "id": "XX-1",
            "status": "open",
            "result": "passed",
            "gates": [{ "gate": "G2", "ok": true }, { "gate": "G6", "ok": true }],
        })
    );
}

// G6

#[test]
fn a_numbered_section_with_content_under_it_passes_g6() {
    let (env, root) = env_with_project();
    fs::create_dir(root.join("docs")).unwrap();
    fs::write(root.join("docs/DESIGN.md"), DESIGN).unwrap();
    env.create("XX", "a");
    // Every way a section can be named, and none at all.
    for design in [
        "docs/DESIGN.md#2.4",
        "docs/DESIGN.md#1",
        "docs/DESIGN.md#1.",
        "docs/DESIGN.md#2",
        "docs/DESIGN.md",
    ] {
        set_design(&env, design);
        env.json(&["gates", "XX-1"]);
    }
    // Named through `edit --design`, relative to the current folder.
    env.json_in(
        &root.join("docs"),
        &["edit", "XX-1", "--design", "DESIGN.md#2.4"],
    );
    assert_eq!(env.item("XX-1")["design"], "docs/DESIGN.md#2.4");
    env.json(&["start", "XX-1"]);
    assert_eq!(env.json(&["submit", "XX-1"])["status"], "done");
}

#[test]
fn a_design_naming_a_missing_section_fails_g6() {
    let (env, root) = env_with_project();
    fs::write(root.join("DESIGN.md"), DESIGN).unwrap();
    env.create("XX", "a");
    env.json(&["start", "XX-1"]);
    for (design, says) in [
        ("DESIGN.md#2.9", "no heading numbered 2.9"),
        // In a code block, so not a heading.
        ("DESIGN.md#4", "no heading numbered 4"),
        ("DESIGN.md#2.5", "nothing under it"),
        ("DESIGN.md#3", "nothing under it"),
        ("DESIGN.md#gates", "the section is a number"),
        ("NOTES.md#1", "doesn't exist"),
    ] {
        set_design(&env, design);
        for args in [&["gates", "XX-1"][..], &["submit", "XX-1"]] {
            let error = refused_by_gate(&env, args);
            assert_eq!(
                error["gates"],
                json!([{ "gate": "G2", "ok": true }, { "gate": "G6", "ok": false }]),
                "{design}"
            );
            let message = error["failures"][0]["message"].as_str().unwrap();
            assert!(message.contains(says), "{design}: {message}");
            assert!(message.contains("tracker edit XX-1 --design"), "{message}");
        }
    }
}

#[cfg(unix)]
#[test]
fn a_design_whose_file_is_a_symlink_outside_the_root_fails_g6() {
    // `edit --design` refuses such a link, but a file can become one
    // later, so G6 checks again when it reads it (§7).
    let (env, root) = env_with_project();
    let outside = env.dir("elsewhere");
    fs::write(outside.join("DESIGN.md"), DESIGN).unwrap();
    fs::write(root.join("DESIGN.md"), DESIGN).unwrap();
    env.create("XX", "a");
    env.json(&[
        "edit",
        "XX-1",
        "--design",
        &format!("{}/DESIGN.md#2.4", root.display()),
    ]);
    env.json(&["gates", "XX-1"]);

    fs::remove_file(root.join("DESIGN.md")).unwrap();
    std::os::unix::fs::symlink(outside.join("DESIGN.md"), root.join("DESIGN.md")).unwrap();
    let error = refused_by_gate(&env, &["gates", "XX-1"]);
    let message = error["failures"][0]["message"].as_str().unwrap();
    assert!(message.contains("outside XX's root"), "{message}");

    // And so does a folder on the way that leads outside.
    fs::remove_file(root.join("DESIGN.md")).unwrap();
    std::os::unix::fs::symlink(&outside, root.join("docs")).unwrap();
    set_design(&env, "docs/DESIGN.md#2.4");
    let error = refused_by_gate(&env, &["gates", "XX-1"]);
    let message = error["failures"][0]["message"].as_str().unwrap();
    assert!(message.contains("outside XX's root"), "{message}");

    // A symlink that stays inside the root is fine.
    fs::remove_file(root.join("docs")).unwrap();
    fs::create_dir(root.join("real")).unwrap();
    fs::write(root.join("real/DESIGN.md"), DESIGN).unwrap();
    std::os::unix::fs::symlink(root.join("real"), root.join("docs")).unwrap();
    env.json(&["gates", "XX-1"]);
}

#[test]
fn a_design_written_by_hand_to_leave_the_root_fails_g6() {
    let (env, _) = env_with_project();
    let outside = env.dir("elsewhere");
    fs::write(outside.join("DESIGN.md"), DESIGN).unwrap();
    env.create("XX", "a");
    for design in [
        "../elsewhere/DESIGN.md#2.4".to_string(),
        format!("{}/DESIGN.md#2.4", outside.display()),
    ] {
        set_design(&env, &design);
        let error = refused_by_gate(&env, &["gates", "XX-1"]);
        let message = error["failures"][0]["message"].as_str().unwrap();
        assert!(message.contains("outside XX's root"), "{design}: {message}");
    }
}

#[cfg(unix)]
#[test]
fn g6_never_reads_a_design_that_isnt_a_regular_file() {
    // A pipe would make the read wait forever.
    let (env, root) = env_with_project();
    let status = std::process::Command::new("mkfifo")
        .arg(root.join("DESIGN.md"))
        .status();
    if !status.is_ok_and(|s| s.success()) {
        eprintln!("Skipped: mkfifo isn't available.");
        return;
    }
    env.create("XX", "a");
    set_design(&env, "DESIGN.md#1");
    let error = refused_by_gate(&env, &["gates", "XX-1"]);
    let message = error["failures"][0]["message"].as_str().unwrap();
    assert!(message.contains("isn't a regular file"), "{message}");
}

#[test]
fn g6_fails_when_the_project_isnt_in_config_json() {
    let (env, root) = env_with_project();
    fs::write(root.join("DESIGN.md"), DESIGN).unwrap();
    env.create("XX", "a");
    set_design(&env, "DESIGN.md#1");
    fs::write(
        env.home.path().join("config.json"),
        "{\n  \"projects\": {}\n}\n",
    )
    .unwrap();
    let error = refused_by_gate(&env, &["gates", "XX-1"]);
    let message = error["failures"][0]["message"].as_str().unwrap();
    assert!(message.contains("isn't in config.json"), "{message}");
}

// The dry run and the refused submit

#[test]
fn gates_lists_every_failure_and_leaves_the_item_file_unchanged() {
    let (env, root) = env_with_project();
    fs::write(root.join("DESIGN.md"), DESIGN).unwrap();
    env.json(&[
        "create",
        "a",
        "--project",
        "XX",
        "--done-when",
        "first",
        "--done-when",
        "second",
    ]);
    env.json(&["criterion", "add", "XX-1", "third", "--check", "true"]);
    set_design(&env, "DESIGN.md#9");
    env.json(&["start", "XX-1"]);

    let error = refused_by_gate(&env, &["gates", "XX-1"]);
    assert_eq!(
        error,
        json!({
            "error": "XX-1 would be refused at submit: G2 and G6 failed. This was a dry run, so \
                      nothing was changed.",
            "exit_code": 2,
            "id": "XX-1",
            "status": "in-progress",
            "result": "refused",
            "gates": [{ "gate": "G2", "ok": false }, { "gate": "G6", "ok": false }],
            "failures": [
                { "gate": "G2", "criterion": 1, "message": error["failures"][0]["message"] },
                { "gate": "G2", "criterion": 2, "message": error["failures"][1]["message"] },
                { "gate": "G6", "message": error["failures"][2]["message"] },
            ],
        })
    );
    // The text says the same, on stderr, one reason per line.
    let output = env.cmd().args(["gates", "XX-1"]).assert().code(2);
    let stderr = String::from_utf8_lossy(&output.get_output().stderr).to_string();
    assert_eq!(stderr.lines().count(), 4, "{stderr}");
    assert!(stderr.contains("  G2  Criterion 1, \"first\""), "{stderr}");
    assert!(stderr.contains("  G2  Criterion 2, \"second\""), "{stderr}");
    assert!(
        stderr.contains("  G6  DESIGN.md has no heading numbered 9"),
        "{stderr}"
    );
    assert!(output.get_output().stdout.is_empty());
}

#[test]
fn a_refused_submit_exits_2_and_changes_nothing() {
    let (env, _) = env_with_project();
    env.json(&["create", "a", "--project", "XX", "--done-when", "first"]);
    env.json(&["start", "XX-1"]);
    let error = refused_by_gate(&env, &["submit", "XX-1"]);
    assert_eq!(
        error["error"],
        "XX-1 can't be submitted: G2 failed. Nothing was changed."
    );
    let item = env.item("XX-1");
    assert_eq!(item["status"], "in-progress");
    assert!(item.get("verification").is_none(), "{item}");
    assert_eq!(item["history"].as_array().unwrap().len(), 1);
}

#[test]
fn gates_says_when_the_item_isnt_ready_to_submit() {
    let (env, _) = env_with_project();
    env.create("XX", "a");
    let output = env.cmd().args(["gates", "XX-1"]).assert().success();
    let text = String::from_utf8_lossy(&output.get_output().stdout).to_string();
    assert_eq!(
        text,
        "XX-1 passes the gates submit runs: G2 and G6.\nIt's open, and only an in-progress \
         item can be submitted.\n"
    );
    env.json(&["start", "XX-1"]);
    let output = env.cmd().args(["gates", "XX-1"]).assert().success();
    let text = String::from_utf8_lossy(&output.get_output().stdout).to_string();
    assert_eq!(text, "XX-1 passes the gates submit runs: G2 and G6.\n");
}

#[test]
fn a_failing_gates_says_when_the_item_isnt_ready_to_submit_either() {
    // Fixing every reason listed still wouldn't let an open item be
    // submitted, so the refusal says that too (review 2's finding 5).
    let (env, _) = env_with_project();
    env.json(&["create", "a", "--project", "XX", "--done-when", "first"]);
    let error = refused_by_gate(&env, &["gates", "XX-1"]);
    assert_eq!(error["status"], "open", "{error}");
    let output = env.cmd().args(["gates", "XX-1"]).assert().code(2);
    let stderr = String::from_utf8_lossy(&output.get_output().stderr).to_string();
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 3, "{stderr}");
    assert!(lines[1].starts_with("  G2  "), "{stderr}");
    assert_eq!(
        lines[2],
        "  It's open, and only an in-progress item can be submitted."
    );

    // In progress, there's no such line.
    env.json(&["start", "XX-1"]);
    let error = refused_by_gate(&env, &["gates", "XX-1"]);
    assert_eq!(error["status"], "in-progress", "{error}");
    let output = env.cmd().args(["gates", "XX-1"]).assert().code(2);
    let stderr = String::from_utf8_lossy(&output.get_output().stderr).to_string();
    assert_eq!(stderr.lines().count(), 2, "{stderr}");
}

#[test]
fn gates_needs_an_item_that_exists() {
    let (env, _) = env_with_project();
    let message = env.error(&["gates", "XX-1"], 1);
    assert!(message.contains("no item XX-1"), "{message}");
    env.error(&["gates", "../config"], 1);
}

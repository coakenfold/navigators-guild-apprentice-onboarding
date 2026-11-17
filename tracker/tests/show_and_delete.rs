//! Slice 4: show an item's full record, and delete one from a real terminal
//! (DESIGN.md §2.8, §3).
//!
//! The tests that type an ID run `tracker delete` under `script`, which
//! gives it a terminal, and skip where `script` isn't installed. PLAN.md
//! also asks for each state to be tried once by hand.

mod common;

use std::fs;

use common::{Env, check_schema};
use serde_json::{Value, json};

/// The end of `delete`'s question, which the tests wait for before typing.
const PROMPT: &str = "or anything else to cancel: ";

/// A store with project XX, whose root is `<work>/xx`.
fn env_with_project() -> Env {
    let env = Env::new();
    env.add_project("XX", "xx");
    env
}

/// `tracker <args>` as text, from outside every project root. Expects exit 0.
fn text(env: &Env, args: &[&str]) -> String {
    let output = env.cmd().args(args).assert().success();
    String::from_utf8(output.get_output().stdout.clone()).unwrap()
}

/// Every line of deleted.json, parsed, after checking each against its schema.
fn deleted(env: &Env) -> Vec<Value> {
    let path = env.home.path().join("deleted.json");
    let Ok(text) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    assert!(text.ends_with('\n'), "{text:?}");
    text.lines()
        .map(|line| {
            let record: Value = serde_json::from_str(line).unwrap();
            check_schema("deleted", &record);
            record
        })
        .collect()
}

/// Rewrites XX-<n>'s file with `change` applied, as a hand edit would.
fn edit_file(env: &Env, id: &str, change: impl FnOnce(&mut Value)) {
    let mut item = env.item(id);
    change(&mut item);
    fs::write(
        env.item_path(id),
        serde_json::to_string_pretty(&item).unwrap() + "\n",
    )
    .unwrap();
}

#[test]
fn show_of_a_missing_id_exits_1() {
    let env = env_with_project();
    env.create("XX", "a");
    let message = env.error(&["show", "XX-9"], 1);
    assert!(message.contains("XX-9"), "{message}");
    // So does something that isn't an ID, and the text says so on stderr.
    env.error(&["show", "../config"], 1);
    env.cmd().args(["show", "XX-9"]).assert().code(1);
    // A lower-case ID finds the item, as for every other command.
    assert_eq!(env.json(&["show", "xx-1"])["item"]["id"], "XX-1");
}

#[test]
fn show_json_is_the_item_as_its_file_holds_it_when_this_version_wrote_it() {
    let env = env_with_project();
    env.create("XX", "a");
    env.json(&["edit", "XX-1", "--label", "bug", "--description", "d"]);
    env.json(&["start", "XX-1"]);
    env.json(&["submit", "XX-1"]);
    env.json(&["reopen", "XX-1", "--reason", "not yet"]);
    env.json(&["start", "XX-1"]);
    env.json(&["submit", "XX-1"]);

    let shown = env.json(&["show", "XX-1"]);
    assert_eq!(shown["item"], env.item("XX-1"));
    assert!(shown.get("warning").is_none(), "{shown}");
    assert!(shown["item"]["verification"].is_object());
    assert!(shown["item"]["previous_verification"].is_object());
    assert_eq!(shown["item"]["history"].as_array().unwrap().len(), 6);
}

#[test]
fn show_names_the_fields_of_a_file_from_a_newer_tracker() {
    // A newer tracker's file, with a field this version doesn't know. The
    // item is shown as this version reads it, and the field is named
    // rather than left out unsaid.
    let env = env_with_project();
    env.create("XX", "a");
    edit_file(&env, "XX-1", |item| {
        item["criteria"] = json!([{ "kind": "script", "run": "cargo test" }]);
    });

    let shown = env.json(&["show", "XX-1"]);
    assert!(shown["item"].get("criteria").is_none(), "{shown}");
    let warning = shown["warning"].as_str().unwrap();
    assert!(warning.contains("doesn't know"), "{warning}");
    assert!(warning.contains(": criteria."), "{warning}");
    assert!(warning.contains("newer tracker"), "{warning}");

    let shown = text(&env, &["show", "XX-1"]);
    assert!(shown.ends_with(&format!("\n{warning}\n")), "{shown}");
}

#[test]
fn show_prints_every_field_the_verification_records_and_history_with_times() {
    let env = env_with_project();
    let root = env.work.path().join("xx");
    env.json_in(
        &root,
        &[
            "create",
            "Search filters bookmarks by tag",
            "--kind",
            "capability",
            "--priority",
            "high",
            "--label",
            "feature",
            "--label",
            "needs-review",
            "--design",
            "DESIGN.md#2.5",
            "--description",
            "Tag chips filter the list.\n\nSearch narrows within the tag.",
        ],
    );
    env.json(&[
        "edit",
        "XX-1",
        "--kind",
        "change",
        "--unlabel",
        "needs-review",
    ]);
    env.json(&["start", "XX-1"]);
    env.json(&["submit", "XX-1"]);
    env.json(&["reopen", "XX-1", "--reason", "The tag list is missing."]);
    env.json(&["start", "XX-1"]);
    env.json(&["submit", "XX-1"]);
    let item = env.item("XX-1");

    let shown = text(&env, &["show", "XX-1"]);
    for expected in [
        "XX-1  Search filters bookmarks by tag",
        "XX project",
        "change",
        "done",
        "high",
        "feature",
        "DESIGN.md#2.5",
        item["created"].as_str().unwrap(),
        item["updated"].as_str().unwrap(),
        // The description, as plain text over its lines.
        "  Tag chips filter the list.\n\n  Search narrows within the tag.\n",
        // Both verification records, with their notes.
        "Verification:",
        "Previous verification",
        "unchecked",
        "No gates ran",
        // The history, with what the edit changed and the reopen's reason.
        "History, oldest first (6 entries):",
        "changed: kind, labels",
        "kind before: capability",
        "labels removed: needs-review",
        "reason: The tag list is missing.",
        "open → in-progress",
        "in-progress → done",
        "stays open",
        "not from a terminal",
    ] {
        assert!(shown.contains(expected), "{expected:?} isn't in:\n{shown}");
    }
    // Each entry's time, in order, at the start of its line.
    let mut rest = shown.as_str();
    for entry in item["history"].as_array().unwrap() {
        let line = format!("  {}  {}", entry["at"].as_str().unwrap(), entry["via"]);
        let line = line.replace('"', "");
        let at = rest
            .find(&line)
            .unwrap_or_else(|| panic!("{line:?} isn't in order in:\n{shown}"));
        rest = &rest[at + line.len()..];
    }
}

#[test]
fn show_says_when_an_item_has_no_verification_or_history() {
    let env = env_with_project();
    env.create("XX", "a");
    let shown = text(&env, &["show", "XX-1"]);
    assert!(shown.contains("Verification: none."), "{shown}");
    assert!(shown.contains("History: none yet."), "{shown}");
    // Fields an item doesn't have aren't shown as empty rows.
    assert!(!shown.contains("labels"), "{shown}");
    assert!(!shown.contains("Description"), "{shown}");
}

#[test]
fn show_escapes_text_edited_by_hand() {
    let env = env_with_project();
    env.create("XX", "a");
    env.json(&["shelve", "XX-1", "--reason", "r"]);
    edit_file(&env, "XX-1", |item| {
        item["title"] = json!("clear\u{1b}[2J\nsecond line");
        item["description"] = json!("ok\n\u{1b}]0;title\u{7}\u{202e}reversed");
        item["history"][0]["reason"] = json!("why\u{9b}31m\u{2028}");
    });

    let shown = text(&env, &["show", "XX-1"]);
    for c in ['\u{1b}', '\u{7}', '\u{9b}', '\u{202e}', '\u{2028}'] {
        assert!(!shown.contains(c), "{c:?} is in:\n{shown}");
    }
    // The title stays on its own line, and the description keeps its line break.
    assert!(
        shown.contains("XX-1  clear\\u{1b}[2J\\u{a}second line\n"),
        "{shown}"
    );
    assert!(shown.contains("  ok\n  \\u{1b}]0;title"), "{shown}");
}

#[test]
fn echo_the_id_into_delete_is_refused_and_the_file_remains() {
    let env = env_with_project();
    env.create("XX", "a");
    let before = fs::read(env.item_path("XX-1")).unwrap();

    // `echo XX-1 | tracker delete XX-1`, as text and as JSON.
    let output = env
        .cmd()
        .args(["delete", "XX-1"])
        .write_stdin("XX-1\n")
        .assert()
        .code(1);
    let stderr = String::from_utf8_lossy(&output.get_output().stderr).to_string();
    assert!(stderr.contains("interactive terminal"), "{stderr}");
    // It says what an agent can do instead.
    assert!(stderr.contains("tracker shelve XX-1"), "{stderr}");
    let output = env
        .cmd()
        .args(["delete", "XX-1", "--json"])
        .write_stdin("XX-1\n")
        .assert()
        .code(1);
    let error = common::parse(&output.get_output().stdout);
    check_schema("error", &error);
    assert!(
        error["error"].as_str().unwrap().contains("terminal"),
        "{error}"
    );

    assert_eq!(fs::read(env.item_path("XX-1")).unwrap(), before);
    assert!(!env.home.path().join("deleted.json").exists());
    // Without the ID on stdin too.
    env.error(&["delete", "XX-1"], 1);
    assert_eq!(fs::read(env.item_path("XX-1")).unwrap(), before);
}

#[test]
fn delete_of_a_missing_id_exits_1_and_writes_nothing() {
    let env = env_with_project();
    env.error(&["delete", "XX-1"], 1);
    if let Some((output, ok)) = env.in_terminal_typing(&["delete", "XX-1"], PROMPT, "XX-1", || {}) {
        assert!(!ok, "{output}");
        assert!(output.contains("There's no item XX-1"), "{output}");
    }
    assert!(!env.home.path().join("deleted.json").exists());
}

#[test]
fn in_a_terminal_typing_the_id_deletes_an_item_in_each_state() {
    let env = env_with_project();
    for title in ["open one", "started", "finished", "put away", "held"] {
        env.create("XX", title);
    }
    env.json(&["start", "XX-2"]);
    env.json(&["start", "XX-3"]);
    env.json(&["submit", "XX-3"]);
    env.json(&["shelve", "XX-4", "--reason", "later"]);
    // Nothing reaches review until slice 9's triggers, but by hand.
    edit_file(&env, "XX-5", |item| item["status"] = json!("review"));
    let done_at = env.item("XX-3")["verification"]["at"]
        .as_str()
        .unwrap()
        .to_string();

    let cases = [
        (
            "XX-1",
            "open",
            "XX-1 \"open one\" is open.",
            "removes its file.",
        ),
        (
            "XX-2",
            "in-progress",
            "XX-2 \"started\" is in-progress, so someone may be working on it.",
            "removes its file and its history (1 entry).",
        ),
        (
            "XX-3",
            "done",
            &format!("XX-3 \"finished\" is done, verified unchecked at {done_at}."),
            "removes its file, its history (2 entries) and its verification record.",
        ),
        (
            "XX-4",
            "shelved",
            "XX-4 \"put away\" is shelved.",
            "removes its file and its history (1 entry).",
        ),
        (
            "XX-5",
            "review",
            "XX-5 \"held\" is in review, waiting for a human's decision.",
            "removes its file.",
        ),
    ];
    for (n, (id, state, says_state, says_lost)) in cases.iter().enumerate() {
        let title = env.item(id)["title"].clone();
        let Some((output, ok)) = env.in_terminal_typing(&["delete", id], PROMPT, id, || {}) else {
            return;
        };
        assert!(ok, "{output}");
        assert!(output.contains(says_state), "{id}: {output}");
        assert!(output.contains(says_lost), "{id}: {output}");
        assert!(output.contains("won't be used again"), "{id}: {output}");
        assert!(
            output.contains(&format!("Type {id} to confirm")),
            "{output}"
        );
        assert!(output.contains(&format!("Deleted {id}.")), "{output}");
        assert!(!env.item_path(id).exists(), "{id}");

        // One more line in deleted.json, for this item.
        let records = deleted(&env);
        assert_eq!(records.len(), n + 1);
        let record = records.last().unwrap();
        assert_eq!(record["id"], *id);
        assert_eq!(record["title"], title);
        assert_eq!(record["status"], *state);
        assert_eq!(record["interactive"], true);
    }
}

#[test]
fn in_a_terminal_a_wrong_id_cancels() {
    let env = env_with_project();
    env.create("XX", "a");
    env.create("XX", "b");
    let before = fs::read(env.item_path("XX-1")).unwrap();

    for (answer, says) in [
        ("XX-2", "\"XX-2\" isn't XX-1"),
        ("XX-1 please", "isn't XX-1"),
        ("", "nothing was typed"),
    ] {
        let Some((output, ok)) = env.in_terminal_typing(&["delete", "XX-1"], PROMPT, answer, || {})
        else {
            return;
        };
        assert!(!ok, "{answer:?}: {output}");
        assert!(output.contains("Cancelled"), "{output}");
        assert!(output.contains(says), "{output}");
        assert!(output.contains("Nothing was changed"), "{output}");
    }
    assert_eq!(fs::read(env.item_path("XX-1")).unwrap(), before);
    assert!(env.item_path("XX-2").exists());
    assert!(!env.home.path().join("deleted.json").exists());

    // The ID's key can be typed in lower case, as on the command line.
    let Some((output, ok)) = env.in_terminal_typing(&["delete", "XX-1"], PROMPT, "xx-1", || {})
    else {
        return;
    };
    assert!(ok, "{output}");
    assert!(!env.item_path("XX-1").exists());
}

#[test]
fn deleting_xx_2_and_then_creating_an_item_gives_xx_3() {
    let env = env_with_project();
    env.create("XX", "a");
    env.create("XX", "b");
    let Some((output, ok)) = env.in_terminal_typing(&["delete", "XX-2"], PROMPT, "XX-2", || {})
    else {
        return;
    };
    assert!(ok, "{output}");
    assert_eq!(env.create("XX", "c"), "XX-3");
}

#[test]
fn a_delete_leaves_one_line_with_the_id_title_state_time_and_interactive_only() {
    let env = env_with_project();
    let root = env.work.path().join("xx");
    env.json_in(
        &root,
        &[
            "create",
            "Gone soon",
            "--label",
            "secret-label",
            "--design",
            "DESIGN.md#9",
            "--description",
            "a secret description",
        ],
    );
    env.json(&["shelve", "XX-1", "--reason", "a secret reason"]);
    let Some((output, ok)) = env.in_terminal_typing(&["delete", "XX-1"], PROMPT, "XX-1", || {})
    else {
        return;
    };
    assert!(ok, "{output}");

    let text = fs::read_to_string(env.home.path().join("deleted.json")).unwrap();
    assert_eq!(text.lines().count(), 1, "{text}");
    let record = &deleted(&env)[0];
    let keys: Vec<&String> = record.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["id", "title", "status", "at", "interactive"]);
    assert_eq!(record["id"], "XX-1");
    assert_eq!(record["title"], "Gone soon");
    assert_eq!(record["status"], "shelved");
    assert_eq!(record["interactive"], true);
    // Nothing else of the item: not its description, labels, design or history.
    for secret in ["secret", "DESIGN.md", "created", "history"] {
        assert!(!text.contains(secret), "{secret} is in {text}");
    }
    // And nothing of it is left in items/.
    assert_eq!(
        fs::read_dir(env.home.path().join("items")).unwrap().count(),
        0
    );
}

#[test]
fn delete_json_in_a_terminal_prints_only_the_record_on_stdout() {
    let env = env_with_project();
    env.create("XX", "a");
    let out = env.work.path().join("out.json");
    let line = format!(
        "\"$TRACKER\" delete XX-1 --json > '{}'",
        out.to_str().unwrap()
    );
    let Some((output, ok)) = env.in_terminal_typing_sh(&line, PROMPT, "XX-1") else {
        return;
    };
    assert!(ok, "{output}");
    // The warning and question went to the terminal, through stderr.
    assert!(output.contains("Type XX-1 to confirm"), "{output}");
    let printed = common::parse(&fs::read(&out).unwrap());
    check_schema("delete", &printed);
    assert_eq!(printed["deleted"], deleted(&env)[0]);
}

#[test]
fn an_item_changed_while_deciding_isnt_deleted() {
    let env = env_with_project();
    env.create("XX", "a");
    let Some((output, ok)) = env.in_terminal_typing(&["delete", "XX-1"], PROMPT, "XX-1", || {
        // An agent's edit lands while I read the warning.
        env.json(&["edit", "XX-1", "--priority", "high"]);
    }) else {
        return;
    };
    assert!(!ok, "{output}");
    assert!(
        output.contains("changed while you were deciding"),
        "{output}"
    );
    assert_eq!(env.item("XX-1")["priority"], "high");
    assert!(!env.home.path().join("deleted.json").exists());
}

#[test]
fn a_lost_counter_doesnt_hand_out_a_deleted_id() {
    let env = env_with_project();
    env.create("XX", "a");
    env.create("XX", "b");
    let Some((output, ok)) = env.in_terminal_typing(&["delete", "XX-2"], PROMPT, "XX-2", || {})
    else {
        return;
    };
    assert!(ok, "{output}");
    // counters.json is reset to hand out XX-2 again. Only XX-1 is on disk,
    // but XX-2 is in deleted.json.
    fs::write(env.home.path().join("counters.json"), "{\"XX\": 2}\n").unwrap();
    let message = env.error(&["create", "c", "--project", "XX"], 3);
    assert!(
        message.contains("XX-2 was used by an item since deleted"),
        "{message}"
    );
    assert!(message.contains("at least 3"), "{message}");

    fs::write(env.home.path().join("counters.json"), "{\"XX\": 3}\n").unwrap();
    assert_eq!(env.create("XX", "c"), "XX-3");
}

#[test]
fn delete_appends_to_a_deleted_json_with_no_final_newline() {
    let env = env_with_project();
    env.create("XX", "a");
    // A line written by hand, without its newline.
    let earlier = r#"{"id":"YY-1","title":"t","status":"open","at":"2026-11-15T22:01:56Z","interactive":true}"#;
    fs::write(env.home.path().join("deleted.json"), earlier).unwrap();
    let Some((output, ok)) = env.in_terminal_typing(&["delete", "XX-1"], PROMPT, "XX-1", || {})
    else {
        return;
    };
    assert!(ok, "{output}");
    let records = deleted(&env);
    assert_eq!(records.len(), 2);
    assert_eq!(records[0]["id"], "YY-1");
    assert_eq!(records[1]["id"], "XX-1");
}

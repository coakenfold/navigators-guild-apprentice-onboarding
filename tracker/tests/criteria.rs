//! Slice 6: Done-when criteria (DESIGN.md §2.3, §3): `create --done-when`,
//! `criterion add`, `set`, `remove` and `accept`, `evidence`, and the
//! `written` fingerprints (§2.1).

mod common;

use std::fs;

use common::Env;
use serde_json::{Value, json};

/// The SHA-256 of "a" and of "b", in lower-case hex: what `written` holds
/// for a criterion with that text.
const SHA_A: &str = "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb";
const SHA_B: &str = "3e23e8160039594a33894f6564e1b1348bbd7a0088d42c4acb73eeaed59c009d";

/// The end of a human-only command's question, which the tests wait for.
const PROMPT: &str = "or anything else to cancel: ";

/// A store with project XX and one open item, XX-1, with no criteria.
fn env_with_item() -> Env {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create_bare("XX", "a");
    env
}

/// Runs a criterion command with `--json`, checks that the file holds the
/// item it prints, and returns the output.
fn changed(env: &Env, args: &[&str]) -> Value {
    let output = env.json(args);
    let id = output["item"]["id"].as_str().unwrap();
    assert_eq!(env.item(id), output["item"], "{args:?}");
    output
}

/// Runs a command that must be refused with `code`, checks that XX-1's
/// file is byte-for-byte unchanged, and returns the error message.
fn refused(env: &Env, args: &[&str], code: i32) -> String {
    let before = fs::read(env.item_path("XX-1")).unwrap();
    let message = env.error(args, code);
    assert_eq!(
        fs::read(env.item_path("XX-1")).unwrap(),
        before,
        "{args:?} changed the file"
    );
    message
}

fn criteria(item: &Value) -> Vec<Value> {
    item.get("criteria")
        .map(|c| c.as_array().unwrap().clone())
        .unwrap_or_default()
}

fn written(item: &Value) -> Vec<String> {
    item.get("written")
        .map(|w| {
            w.as_array()
                .unwrap()
                .iter()
                .map(|f| f.as_str().unwrap().to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// Sets XX-1's status by hand.
fn set_status(env: &Env, status: &str) {
    let mut item = env.item("XX-1");
    item["status"] = json!(status);
    fs::write(
        env.item_path("XX-1"),
        serde_json::to_string_pretty(&item).unwrap() + "\n",
    )
    .unwrap();
}

#[test]
fn create_done_when_adds_manual_criteria_and_their_written_fingerprints() {
    let env = Env::new();
    env.add_project("XX", "xx");
    let item = env.json(&[
        "create",
        "t",
        "--project",
        "XX",
        "--done-when",
        "a",
        "--done-when",
        " b ",
    ])["item"]
        .clone();
    assert_eq!(item["criteria"], json!([{ "text": "a" }, { "text": "b" }]));
    // One SHA-256 per criterion's text, taken at create (§2.1).
    assert_eq!(written(&item), [SHA_A, SHA_B]);
    assert_eq!(env.item("XX-1"), item);
    // Creating isn't a move or an edit, so there's no history yet.
    assert!(item.get("history").is_none(), "{item}");

    // Without --done-when there are neither.
    let item = env.json(&["create", "u", "--project", "XX"])["item"].clone();
    assert!(item.get("criteria").is_none(), "{item}");
    assert!(item.get("written").is_none(), "{item}");
}

#[test]
fn written_is_taken_at_create_whoever_creates_the_item() {
    // The criteria given at create are mine, from the plan (§5), even when
    // an agent runs the create for me.
    let env = Env::new();
    env.add_project("XX", "xx");
    let item = env.json(&["create", "t", "--project", "XX", "--done-when", "a"])["item"].clone();
    assert_eq!(written(&item), [SHA_A]);
}

#[test]
fn create_refuses_a_bad_or_repeated_criterion() {
    let env = Env::new();
    env.add_project("XX", "xx");
    for (done_when, says) in [
        (&["", "a"][..], "can't be empty"),
        (&["two\nlines"], "one line"),
        (&["esc\u{1b}[2K"], "control character"),
        (&["a", " a "], "twice"),
    ] {
        let mut args = vec!["create", "t", "--project", "XX"];
        for text in done_when {
            args.extend(["--done-when", text]);
        }
        let message = env.error(&args, 1);
        assert!(message.contains(says), "{done_when:?}: {message}");
    }
    let message = env.error(
        &[
            "create",
            "t",
            "--project",
            "XX",
            "--done-when",
            &"é".repeat(501),
        ],
        1,
    );
    assert!(message.contains("500"), "{message}");
    // Nothing was created, and no ID was used.
    assert_eq!(env.create_bare("XX", "next"), "XX-1");
}

#[test]
fn criterion_add_adds_a_script_or_a_manual_criterion() {
    let env = env_with_item();
    let output = changed(
        &env,
        &[
            "criterion",
            "add",
            "XX-1",
            "Clicking a tag filters",
            "--check",
            " cargo test filter_by_tag ",
        ],
    );
    assert_eq!(output["criterion"], 1);
    let output = changed(&env, &["criterion", "add", "xx-1", "Looks right"]);
    assert_eq!(output["criterion"], 2);
    assert_eq!(
        criteria(&output["item"]),
        [
            json!({ "text": "Clicking a tag filters", "check": "cargo test filter_by_tag" }),
            json!({ "text": "Looks right" }),
        ]
    );
    // An agent's criteria aren't written: only mine are (§2.1).
    assert!(written(&output["item"]).is_empty());
    // A change, so `updated` moves, but it isn't an edit, so no history.
    assert!(output["item"].get("history").is_none());

    // The text says what to do about a manual criterion.
    let out = env
        .cmd()
        .args(["criterion", "add", "XX-1", "Reads well"])
        .assert()
        .success();
    let text = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert!(
        text.starts_with("Added criterion 3 to XX-1: Reads well\n"),
        "{text}"
    );
    assert!(text.contains("tracker evidence XX-1 3"), "{text}");
}

#[test]
fn criterion_add_refuses_bad_input_and_a_repeated_criterion() {
    let env = env_with_item();
    env.json(&["criterion", "add", "XX-1", "a"]);
    for (args, says) in [
        (&["criterion", "add", "XX-1", "  "][..], "can't be empty"),
        (&["criterion", "add", "XX-1", "x\ny"], "one line"),
        (
            &["criterion", "add", "XX-1", "x", "--check", " "],
            "can't be empty",
        ),
        (
            &["criterion", "add", "XX-1", "x", "--check", "a\nb"],
            "one line",
        ),
        (
            &["criterion", "add", "XX-1", " a "],
            "already has that criterion",
        ),
        (&["criterion", "add", "XX-9", "x"], "no item XX-9"),
    ] {
        let message = refused(&env, args, 1);
        assert!(message.contains(says), "{args:?}: {message}");
    }
}

#[test]
fn criterion_set_gives_a_criterion_a_check_and_drops_its_evidence() {
    let env = env_with_item();
    env.json(&["criterion", "add", "XX-1", "a"]);
    env.json(&["evidence", "XX-1", "1", "see the screenshot"]);
    let output = changed(&env, &["criterion", "set", "XX-1", "1", "--check", "true"]);
    assert_eq!(output["criterion"], 1);
    // The text stays, and a check's output is its proof, so the evidence goes.
    assert_eq!(
        criteria(&output["item"]),
        [json!({ "text": "a", "check": "true" })]
    );

    // A new check replaces the old one.
    let output = changed(&env, &["criterion", "set", "XX-1", "1", "--check", "false"]);
    assert_eq!(criteria(&output["item"])[0]["check"], "false");

    // The same check again changes nothing, not even `updated`.
    let before = fs::read(env.item_path("XX-1")).unwrap();
    env.json(&["criterion", "set", "XX-1", "1", "--check", "false"]);
    assert_eq!(fs::read(env.item_path("XX-1")).unwrap(), before);

    for (n, says) in [("0", "no criterion 0"), ("2", "no criterion 2")] {
        let message = refused(&env, &["criterion", "set", "XX-1", n, "--check", "x"], 1);
        assert!(message.contains(says), "{n}: {message}");
    }
    // --check is required, and a number is a number.
    refused(&env, &["criterion", "set", "XX-1", "1"], 1);
    refused(
        &env,
        &["criterion", "set", "XX-1", "one", "--check", "x"],
        1,
    );
}

#[test]
fn criterion_remove_removes_one_and_the_rest_move_up() {
    let env = env_with_item();
    for text in ["a", "b", "c"] {
        env.json(&["criterion", "add", "XX-1", text]);
    }
    let output = changed(&env, &["criterion", "remove", "XX-1", "2"]);
    assert_eq!(output["criterion"], 2);
    assert_eq!(
        criteria(&output["item"]),
        [json!({ "text": "a" }), json!({ "text": "c" })]
    );

    env.json(&["criterion", "remove", "XX-1", "1"]);
    env.json(&["criterion", "remove", "XX-1", "1"]);
    // The last one gone, `criteria` is left out.
    assert!(env.item("XX-1").get("criteria").is_none());
    let message = refused(&env, &["criterion", "remove", "XX-1", "1"], 1);
    assert!(message.contains("no criteria yet"), "{message}");
}

#[test]
fn an_agent_removing_a_written_criterion_leaves_written_alone() {
    // So slice 11's T5 can see that a criterion I wrote is gone.
    let env = Env::new();
    env.add_project("XX", "xx");
    env.json(&[
        "create",
        "t",
        "--project",
        "XX",
        "--done-when",
        "a",
        "--done-when",
        "b",
    ]);
    let item = changed(&env, &["criterion", "remove", "XX-1", "1"])["item"].clone();
    assert_eq!(item["criteria"], json!([{ "text": "b" }]));
    assert_eq!(written(&item), [SHA_A, SHA_B]);
}

#[test]
fn from_a_terminal_add_and_remove_change_written() {
    let env = env_with_item();
    let Some(output) = env.in_terminal(&["criterion", "add", "XX-1", "a"]) else {
        return;
    };
    assert!(output.status.success(), "{output:?}");
    assert_eq!(written(&env.item("XX-1")), [SHA_A]);
    env.json(&["criterion", "add", "XX-1", "b"]);
    assert_eq!(written(&env.item("XX-1")), [SHA_A]);

    let output = env
        .in_terminal(&["criterion", "remove", "XX-1", "1"])
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let item = env.item("XX-1");
    assert_eq!(item["criteria"], json!([{ "text": "b" }]));
    assert!(item.get("written").is_none(), "{item}");
}

#[test]
fn evidence_is_recorded_for_a_manual_criterion_only() {
    let env = env_with_item();
    env.json(&["criterion", "add", "XX-1", "a"]);
    env.json(&["criterion", "add", "XX-1", "b", "--check", "cargo test b"]);

    let output = changed(
        &env,
        &["evidence", "XX-1", "1", " the screenshot in docs/a.png "],
    );
    assert_eq!(output["criterion"], 1);
    assert_eq!(
        criteria(&output["item"])[0],
        json!({ "text": "a", "evidence": "the screenshot in docs/a.png" })
    );
    // Evidence replaces evidence.
    let output = changed(&env, &["evidence", "XX-1", "1", "docs/b.png"]);
    assert_eq!(criteria(&output["item"])[0]["evidence"], "docs/b.png");

    // A script criterion's output is its proof (§2.3).
    let message = refused(&env, &["evidence", "XX-1", "2", "trust me"], 1);
    assert!(message.contains("has a check"), "{message}");
    assert!(message.contains("cargo test b"), "{message}");

    for (args, says) in [
        (&["evidence", "XX-1", "3", "x"][..], "no criterion 3"),
        (&["evidence", "XX-1", "1", " "], "can't be empty"),
        (&["evidence", "XX-1", "1", "a\nb"], "one line"),
    ] {
        let message = refused(&env, args, 1);
        assert!(message.contains(says), "{args:?}: {message}");
    }
    let message = refused(&env, &["evidence", "XX-1", "1", &"é".repeat(501)], 1);
    assert!(message.contains("500"), "{message}");
}

#[test]
fn criteria_can_change_before_and_after_start_but_not_once_done() {
    let env = env_with_item();
    env.add_criterion("XX-1");
    env.json(&["start", "XX-1"]);
    // After start too: slice 11's T5 is what notices (§3).
    env.json(&["criterion", "add", "XX-1", "more"]);
    env.json(&["evidence", "XX-1", "2", "here"]);
    env.json(&["submit", "XX-1"]);

    for args in [
        &["criterion", "add", "XX-1", "x"][..],
        &["criterion", "set", "XX-1", "1", "--check", "x"],
        &["criterion", "remove", "XX-1", "1"],
        &["evidence", "XX-1", "2", "there"],
    ] {
        let message = refused(&env, args, 1);
        assert!(message.contains("is done"), "{args:?}: {message}");
        assert!(
            message.contains("tracker reopen XX-1"),
            "{args:?}: {message}"
        );
    }

    // A shelved item's can change.
    env.json(&["reopen", "XX-1", "--reason", "r"]);
    env.json(&["shelve", "XX-1", "--reason", "r"]);
    env.json(&["criterion", "add", "XX-1", "x"]);
}

#[test]
fn a_held_items_criteria_change_only_from_a_terminal() {
    let env = env_with_item();
    env.json(&["criterion", "add", "XX-1", "a"]);
    set_status(&env, "review");
    for args in [
        &["criterion", "add", "XX-1", "x"][..],
        &["criterion", "set", "XX-1", "1", "--check", "x"],
        &["criterion", "remove", "XX-1", "1"],
        &["evidence", "XX-1", "1", "there"],
    ] {
        let message = refused(&env, args, 1);
        assert!(message.contains("in review"), "{args:?}: {message}");
    }
    let Some(output) = env.in_terminal(&["evidence", "XX-1", "1", "there"]) else {
        return;
    };
    assert!(output.status.success(), "{output:?}");
    assert_eq!(env.item("XX-1")["criteria"][0]["evidence"], "there");
}

#[test]
fn criterion_accept_is_refused_outside_an_interactive_terminal() {
    let env = env_with_item();
    env.json(&["criterion", "add", "XX-1", "a"]);
    // As an agent's shell would run it, typing the ID into a pipe.
    let before = fs::read(env.item_path("XX-1")).unwrap();
    let output = env
        .cmd()
        .args(["criterion", "accept", "XX-1", "--json"])
        .write_stdin("XX-1\n")
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let error = common::parse(&output);
    common::check_schema("error", &error);
    let message = error["error"].as_str().unwrap();
    assert!(message.contains("interactive terminal"), "{message}");
    assert!(
        message.contains("Only a human accepts criteria"),
        "{message}"
    );
    assert_eq!(fs::read(env.item_path("XX-1")).unwrap(), before);

    // Even when there's nothing to accept.
    env.create_bare("XX", "b");
    let message = env.error(&["criterion", "accept", "XX-2"], 1);
    assert!(message.contains("interactive terminal"), "{message}");
}

#[test]
fn in_a_terminal_criterion_accept_records_the_criteria_as_written() {
    // I wrote "a" and "b" at create. An agent added "c", and I reworded
    // "b" by hand, as "B", in the file.
    let env = Env::new();
    env.add_project("XX", "xx");
    env.json(&[
        "create",
        "t",
        "--project",
        "XX",
        "--done-when",
        "a",
        "--done-when",
        "b",
    ]);
    env.json(&["criterion", "add", "XX-1", "c"]);
    let mut item = env.item("XX-1");
    item["criteria"][1]["text"] = json!("B");
    fs::write(
        env.item_path("XX-1"),
        serde_json::to_string_pretty(&item).unwrap() + "\n",
    )
    .unwrap();

    let Some((output, ok)) =
        env.in_terminal_typing(&["criterion", "accept", "XX-1"], PROMPT, "xx-1", || {})
    else {
        return;
    };
    assert!(ok, "{output}");
    // It said what it records before asking.
    assert!(output.contains("1. a  (already written)"), "{output}");
    assert!(output.contains("2. B  (new to written)"), "{output}");
    assert!(output.contains("3. c  (new to written)"), "{output}");
    assert!(
        output.contains("1 written criterion matches none"),
        "{output}"
    );
    assert!(output.contains("Accepted XX-1's criteria"), "{output}");

    let item = env.item("XX-1");
    let expected: Vec<String> = ["a", "B", "c"].iter().map(|text| sha256(text)).collect();
    assert_eq!(written(&item), expected);
    // One history entry, which doesn't move the item, naming `written`.
    let history = item["history"].as_array().unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(
        history[0],
        json!({
            "at": history[0]["at"],
            "from": "open",
            "to": "open",
            "via": "criterion-accept",
            "interactive": true,
            "fields": ["written"],
        })
    );
    assert_eq!(item["updated"], history[0]["at"]);
    common::check_schema("item", &item);

    // Accepting again has nothing to record, so it doesn't ask.
    let output = env.in_terminal(&["criterion", "accept", "XX-1"]).unwrap();
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(output.status.success(), "{text}");
    assert!(text.contains("Nothing changed"), "{text}");
    assert!(!text.contains("Type XX-1"), "{text}");
    assert_eq!(env.item("XX-1")["history"].as_array().unwrap().len(), 1);
}

#[test]
fn criterion_accept_has_nothing_to_record_when_written_matches_in_another_order() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.json(&[
        "create",
        "t",
        "--project",
        "XX",
        "--done-when",
        "a",
        "--done-when",
        "b",
    ]);
    let mut item = env.item("XX-1");
    item["written"] = json!([SHA_B, SHA_A]);
    fs::write(
        env.item_path("XX-1"),
        serde_json::to_string_pretty(&item).unwrap() + "\n",
    )
    .unwrap();
    let before = fs::read(env.item_path("XX-1")).unwrap();
    let Some(output) = env.in_terminal(&["criterion", "accept", "XX-1"]) else {
        return;
    };
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(output.status.success(), "{text}");
    assert!(text.contains("Nothing changed"), "{text}");
    assert_eq!(fs::read(env.item_path("XX-1")).unwrap(), before);
}

#[test]
fn in_a_terminal_a_wrong_id_cancels_criterion_accept() {
    let env = env_with_item();
    env.json(&["criterion", "add", "XX-1", "a"]);
    let before = fs::read(env.item_path("XX-1")).unwrap();
    let Some((output, ok)) =
        env.in_terminal_typing(&["criterion", "accept", "XX-1"], PROMPT, "XX-2", || {})
    else {
        return;
    };
    assert!(!ok, "{output}");
    assert!(output.contains("Cancelled"), "{output}");
    assert_eq!(fs::read(env.item_path("XX-1")).unwrap(), before);
}

#[test]
fn criterion_accept_does_nothing_if_the_item_changed_while_deciding() {
    let env = env_with_item();
    env.json(&["criterion", "add", "XX-1", "a"]);
    let Some((output, ok)) =
        env.in_terminal_typing(&["criterion", "accept", "XX-1"], PROMPT, "XX-1", || {
            env.json(&["criterion", "add", "XX-1", "b"]);
        })
    else {
        return;
    };
    assert!(!ok, "{output}");
    assert!(
        output.contains("changed while you were deciding"),
        "{output}"
    );
    assert!(env.item("XX-1").get("written").is_none());
}

#[test]
fn show_lists_the_criteria_with_how_each_is_verified() {
    let env = env_with_item();
    let shown = |env: &Env| -> String {
        let output = env.cmd().args(["show", "XX-1"]).assert().success();
        String::from_utf8(output.get_output().stdout.clone()).unwrap()
    };
    assert!(shown(&env).contains("Done when: no criteria yet."));

    env.json(&["criterion", "add", "XX-1", "a", "--check", "cargo test a"]);
    env.json(&["criterion", "add", "XX-1", "b"]);
    env.json(&["criterion", "add", "XX-1", "c"]);
    env.json(&["evidence", "XX-1", "3", "docs/c.png"]);
    // A hand edit can put anything in the file, so show escapes it.
    let mut item = env.item("XX-1");
    item["criteria"][2]["text"] = json!("c\u{202e}");
    fs::write(
        env.item_path("XX-1"),
        serde_json::to_string_pretty(&item).unwrap() + "\n",
    )
    .unwrap();
    let text = shown(&env);
    assert!(
        text.contains(
            "Done when:\n  1. a\n       check: cargo test a\n  2. b\n       manual, no evidence \
             yet\n  3. c\\u{202e}\n       manual, evidence: docs/c.png\n"
        ),
        "{text}"
    );
}

/// The SHA-256 of `text`, in lower-case hex, worked out here rather than by
/// the code under test.
fn sha256(text: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn the_fingerprint_is_the_sha256_of_the_text() {
    assert_eq!(sha256("a"), SHA_A);
    assert_eq!(tracker::item::fingerprint("a"), SHA_A);
    assert_eq!(tracker::item::fingerprint("é"), sha256("é"));
}

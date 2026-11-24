//! Slice 6: Done-when criteria (DESIGN.md §2.3, §3): `create --done-when`,
//! `criterion add`, `set`, `remove` and `accept`, `evidence`, and
//! `written`, each criterion's text as I wrote it (§2.1).

mod common;

use std::fs;

use common::Env;
use serde_json::{Value, json};

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
fn create_done_when_adds_manual_criteria_and_their_written_text() {
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
    // Each criterion's text, as given at create (§2.1).
    assert_eq!(written(&item), ["a", "b"]);
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
    assert_eq!(written(&item), ["a"]);
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

    // A check is printed before it runs and in every gate failure, so it's
    // at most 1000 characters, counted as characters, not bytes (review 1's
    // finding 8).
    let long = "é".repeat(1001);
    for args in [
        &["criterion", "add", "XX-1", "x", "--check", &long][..],
        &["criterion", "set", "XX-1", "1", "--check", &long],
    ] {
        let message = refused(&env, args, 1);
        assert!(message.contains("at most 1000 characters"), "{message}");
        assert!(message.contains("has 1001"), "{message}");
        assert!(message.contains("in a script the check runs"), "{message}");
    }
    let longest = "é".repeat(1000);
    env.json(&["criterion", "add", "XX-1", "x", "--check", &longest]);
    env.json(&["criterion", "set", "XX-1", "1", "--check", &longest]);
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
    assert_eq!(written(&item), ["a", "b"]);
}

#[test]
fn from_a_terminal_add_changes_written_but_remove_doesnt() {
    let env = env_with_item();
    let Some(output) = env.in_terminal(&["criterion", "add", "XX-1", "a"]) else {
        return;
    };
    assert!(output.status.success(), "{output:?}");
    assert_eq!(written(&env.item("XX-1")), ["a"]);
    env.json(&["criterion", "add", "XX-1", "b"]);
    assert_eq!(written(&env.item("XX-1")), ["a"]);

    // `script` gets past the terminal check (§7), so a terminal `remove`
    // leaves "a" in `written` too, with no history entry. Only `criterion
    // accept`, which asks for the ID and is recorded, takes it out (review
    // 2's finding 2).
    let output = env
        .in_terminal(&["criterion", "remove", "XX-1", "1"])
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let item = env.item("XX-1");
    assert_eq!(item["criteria"], json!([{ "text": "b" }]));
    assert_eq!(written(&item), ["a"]);
    assert!(item.get("history").is_none(), "{item}");

    let (output, ok) = env
        .in_terminal_typing(&["criterion", "accept", "XX-1"], PROMPT, "XX-1", || {})
        .unwrap();
    assert!(ok, "{output}");
    assert!(
        output.contains("drops this criterion from written"),
        "{output}"
    );
    assert!(output.contains("  - a\r\n"), "{output}");
    let item = env.item("XX-1");
    assert_eq!(written(&item), ["b"]);
    assert_eq!(item["history"][0]["via"], "criterion-accept");
}

#[test]
fn criterion_set_and_evidence_leave_written_alone_from_a_terminal_or_not() {
    // Only a change to a criterion's text reaches `written` (§2.1), which
    // slice 11's T5 depends on (review 1's test gap 4). "c" is an agent's,
    // so it stays out of `written` even when I give it a check.
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
    env.json(&["criterion", "set", "XX-1", "1", "--check", "true"]);
    env.json(&["evidence", "XX-1", "2", "here"]);
    assert_eq!(written(&env.item("XX-1")), ["a", "b"]);

    for args in [
        &["criterion", "set", "XX-1", "3", "--check", "false"][..],
        &["evidence", "XX-1", "2", "there"],
    ] {
        let Some(output) = env.in_terminal(args) else {
            return;
        };
        assert!(output.status.success(), "{args:?}: {output:?}");
        assert_eq!(written(&env.item("XX-1")), ["a", "b"], "{args:?}");
    }
    let item = env.item("XX-1");
    assert_eq!(item["criteria"][1]["evidence"], "there");
    assert_eq!(item["criteria"][2]["check"], "false");
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
    // The manual criterion holds the submit under T6 (slice 9), so a human
    // approves it, which needs a terminal.
    assert_eq!(env.json(&["submit", "XX-1"])["result"], "held");
    let Some((output, ok)) =
        env.in_terminal_typing(&["approve", "XX-1"], "Type XX-1 to confirm", "XX-1", || {})
    else {
        return;
    };
    assert!(ok, "{output}");

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
    // "b" is gone from the criteria, so accepting drops it, and says so.
    assert!(
        output.contains("drops this criterion from written"),
        "{output}"
    );
    assert!(output.contains("  - b\r\n"), "{output}");
    assert!(output.contains("Accepted XX-1's criteria"), "{output}");

    let item = env.item("XX-1");
    assert_eq!(written(&item), ["a", "B", "c"]);
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
fn criterion_accept_names_each_written_criterion_it_drops() {
    // I wrote two criteria. An agent removed one, which leaves `written`
    // alone, and I reworded the other by hand. Accepting my edit drops both
    // from `written`, so it names both, and the agent's removal isn't
    // accepted unseen (review 1's finding 4).
    let env = Env::new();
    env.add_project("XX", "xx");
    env.json(&[
        "create",
        "t",
        "--project",
        "XX",
        "--done-when",
        "Search filters by tag",
        "--done-when",
        "Empty results say so",
    ]);
    env.json(&["criterion", "remove", "XX-1", "2"]);
    let mut item = env.item("XX-1");
    item["criteria"][0]["text"] = json!("Search filters by tag, case-insensitively");
    fs::write(
        env.item_path("XX-1"),
        serde_json::to_string_pretty(&item).unwrap() + "\n",
    )
    .unwrap();

    let Some((output, ok)) =
        env.in_terminal_typing(&["criterion", "accept", "XX-1"], PROMPT, "XX-1", || {})
    else {
        return;
    };
    assert!(ok, "{output}");
    assert!(
        output.contains("1. Search filters by tag, case-insensitively  (new to written)"),
        "{output}"
    );
    assert!(
        output.contains("drops these 2 criteria from written"),
        "{output}"
    );
    assert!(output.contains("  - Search filters by tag\r\n"), "{output}");
    assert!(output.contains("  - Empty results say so\r\n"), "{output}");
    assert_eq!(
        written(&env.item("XX-1")),
        ["Search filters by tag, case-insensitively"]
    );
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
    item["written"] = json!(["b", "a"]);
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
fn criterion_accept_refuses_a_done_item() {
    // A done item's criteria are the ones it was verified against, even
    // after a hand edit, so it's reopened first (review 1's test gap 5).
    let env = env_with_item();
    env.add_criterion("XX-1");
    env.json(&["start", "XX-1"]);
    env.json(&["submit", "XX-1"]);
    let mut item = env.item("XX-1");
    item["criteria"][0]["text"] = json!("reworded by hand");
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
    assert_eq!(output.status.code(), Some(1), "{text}");
    assert!(text.contains("XX-1 is done"), "{text}");
    assert!(text.contains("tracker reopen XX-1"), "{text}");
    // It refused before asking for the ID.
    assert!(!text.contains("Type XX-1"), "{text}");
    assert_eq!(fs::read(env.item_path("XX-1")).unwrap(), before);
}

#[test]
fn criterion_accept_refuses_a_newer_trackers_file_before_it_asks() {
    // I wrote "Errors say what to do", then reworded it by hand as a judge
    // criterion (slice 12), with `ask`, which this version doesn't know.
    // Accepting it would mean rewriting the file without `ask`, so it's
    // refused, and before asking me anything (review 2's bug 1).
    let env = Env::new();
    env.add_project("XX", "xx");
    env.json(&[
        "create",
        "t",
        "--project",
        "XX",
        "--done-when",
        "Errors say what to do",
    ]);
    let mut item = env.item("XX-1");
    item["criteria"] = json!([{
        "text": "Errors say what to do, reworded",
        "ask": ["Does each error message say how to fix the problem?"],
    }]);
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
    assert_eq!(output.status.code(), Some(3), "{text}");
    assert!(text.contains("newer tracker"), "{text}");
    assert!(text.contains("criteria"), "{text}");
    assert!(text.contains("Nothing was changed"), "{text}");
    assert!(!text.contains("Type XX-1"), "{text}");
    assert!(!text.contains("new to written"), "{text}");
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

#[test]
fn text_output_escapes_a_hand_edited_criterion_and_its_evidence() {
    // A hand edit can put a line break in a criterion's text or evidence,
    // which would otherwise print as a line of its own (§4.4).
    let env = env_with_item();
    env.json(&["criterion", "add", "XX-1", "a"]);
    env.json(&["criterion", "add", "XX-1", "b"]);
    let mut item = env.item("XX-1");
    item["criteria"][0]["text"] = json!("ok\nXX-5: in-progress → done");
    item["criteria"][1]["evidence"] = json!("x\nfake line");
    fs::write(
        env.item_path("XX-1"),
        serde_json::to_string_pretty(&item).unwrap() + "\n",
    )
    .unwrap();
    let stdout = |args: &[&str]| -> String {
        let output = env.cmd().args(args).assert().success();
        String::from_utf8(output.get_output().stdout.clone()).unwrap()
    };

    assert_eq!(
        stdout(&["criterion", "set", "XX-1", "2", "--check", "true"]),
        "Criterion 2 of XX-1 now has a check.\nIts evidence, \"x\\u{a}fake line\", was removed: \
         a check's own output is its proof.\n"
    );

    // `criterion accept` lists each criterion before asking. Typing
    // something other than the ID cancels it.
    if let Some((output, ok)) =
        env.in_terminal_typing(&["criterion", "accept", "XX-1"], PROMPT, "no", || {})
    {
        assert!(!ok, "{output}");
        assert!(
            output.contains("1. ok\\u{a}XX-5: in-progress → done  (new to written)"),
            "{output}"
        );
    }

    assert_eq!(
        stdout(&["criterion", "remove", "XX-1", "1"]),
        "Removed criterion 1 from XX-1: ok\\u{a}XX-5: in-progress → done\nThe ones after it \
         moved up one.\n"
    );
}

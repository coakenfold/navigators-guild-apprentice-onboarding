//! Slice 2: start, submit, shelve and reopen items, with history kept
//! (DESIGN.md §2.2).

mod common;

use std::fs;

use common::Env;
use serde_json::{Value, json};

/// A store with project XX and one open item, XX-1, with a script
/// criterion, so that gates G1 and G2 let it move (slice 6).
fn env_with_item() -> Env {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");
    env
}

/// Runs a move with `--json` and returns the item it prints, after checking
/// that the file on disk holds the same item.
fn moved(env: &Env, args: &[&str]) -> Value {
    let item = env.json(args)["item"].clone();
    let id = item["id"].as_str().unwrap();
    assert_eq!(env.item(id), item, "{args:?}");
    item
}

fn history(item: &Value) -> &Vec<Value> {
    item["history"].as_array().unwrap()
}

fn history_len(item: &Value) -> usize {
    item.get("history")
        .map(|h| h.as_array().unwrap().len())
        .unwrap_or(0)
}

/// Checks that the item is now in `to` and that its last history entry
/// records the move.
fn check_last_move(item: &Value, from: &str, to: &str, via: &str, reason: Option<&str>) {
    assert_eq!(item["status"], to);
    let entry = history(item).last().unwrap();
    let mut expected = json!({
        "at": entry["at"],
        "from": from,
        "to": to,
        "via": via,
        // The tests' stdin isn't a terminal, as an agent's isn't.
        "interactive": false,
    });
    if let Some(reason) = reason {
        expected["reason"] = json!(reason);
    }
    assert_eq!(entry, &expected);
    assert_eq!(item["updated"], entry["at"]);
}

/// Checks that the item's verification record is the `auto` one that a
/// passing submit writes, with the commit and its ref (slice 8), made by
/// its last move. Until slice 8 it was `unchecked`, with a note.
fn check_verified(item: &Value) {
    let verification = &item["verification"];
    assert_eq!(verification["by"], "auto");
    assert_eq!(verification["at"], history(item).last().unwrap()["at"]);
    assert!(verification["commit"].is_string(), "{verification}");
    let git_ref = format!("refs/tracker/{}", item["id"].as_str().unwrap());
    assert_eq!(verification["ref"], git_ref.as_str());
    assert!(verification.get("note").is_none(), "{verification}");
}

/// Checks that `record`, the verification record the item had when it was
/// done, has moved to `previous_verification`, and that it has no
/// `verification` now that it isn't done.
fn check_moved_aside(item: &Value, record: &Value) {
    assert!(item.get("verification").is_none(), "{item}");
    assert_eq!(&item["previous_verification"], record);
}

/// Runs a move that must be refused with exit 1, checks that the item file
/// is byte-for-byte unchanged, and returns the error object.
fn refused_json(env: &Env, args: &[&str]) -> Value {
    let path = env.item_path("XX-1");
    let before = fs::read(&path).unwrap();
    let error = env.error_json_in(&env.outside(), args, 1);
    assert_eq!(
        fs::read(&path).unwrap(),
        before,
        "{args:?} changed the file"
    );
    error
}

/// The same as `refused_json`, returning only the error message.
fn refused(env: &Env, args: &[&str]) -> String {
    refused_json(env, args)["error"]
        .as_str()
        .unwrap()
        .to_string()
}

/// Runs a move that the diagram doesn't draw, and returns the commands
/// that its `allowed` field lists, in order.
fn allowed(env: &Env, args: &[&str]) -> Vec<String> {
    let error = refused_json(env, args);
    let Some(allowed) = error["allowed"].as_array() else {
        panic!("{args:?} has no allowed moves: {error}");
    };
    allowed
        .iter()
        .map(|step| step["command"].as_str().unwrap().to_string())
        .collect()
}

const START: &str = "tracker start XX-1";
const SUBMIT: &str = "tracker submit XX-1";
const SHELVE: &str = "tracker shelve XX-1 --reason \"…\"";
const REOPEN: &str = "tracker reopen XX-1 --reason \"…\"";
const APPROVE: &str = "tracker approve XX-1";
const REJECT: &str = "tracker reject XX-1 --reason \"…\"";

/// Sets XX-1's status by hand, for states no command can reach yet.
fn set_status(env: &Env, status: &str) {
    let path = env.item_path("XX-1");
    let text = fs::read_to_string(&path).unwrap();
    let mut item: Value = serde_json::from_str(&text).unwrap();
    item["status"] = json!(status);
    fs::write(&path, serde_json::to_string_pretty(&item).unwrap() + "\n").unwrap();
}

#[test]
fn an_item_goes_open_in_progress_done_open_through_the_commands() {
    let env = env_with_item();
    assert_eq!(history_len(&env.item("XX-1")), 0);

    let item = moved(&env, &["start", "XX-1"]);
    check_last_move(&item, "open", "in-progress", "start", None);
    assert!(item.get("verification").is_none());

    let item = moved(&env, &["submit", "XX-1"]);
    check_last_move(&item, "in-progress", "done", "submit", None);
    check_verified(&item);
    let record = item["verification"].clone();

    let item = moved(&env, &["reopen", "XX-1", "--reason", "  found a bug  "]);
    check_last_move(&item, "done", "open", "reopen", Some("found a bug"));
    assert_eq!(history(&item).len(), 3);
    check_moved_aside(&item, &record);
}

#[test]
fn an_item_goes_open_in_progress_done_open_through_status() {
    let env = env_with_item();

    let item = moved(&env, &["status", "XX-1", "in-progress"]);
    check_last_move(&item, "open", "in-progress", "start", None);

    let item = moved(&env, &["status", "XX-1", "done"]);
    check_last_move(&item, "in-progress", "done", "submit", None);
    check_verified(&item);
    let record = item["verification"].clone();

    let item = moved(&env, &["status", "XX-1", "open", "--reason", "found a bug"]);
    check_last_move(&item, "done", "open", "reopen", Some("found a bug"));
    assert_eq!(history(&item).len(), 3);
    check_moved_aside(&item, &record);
}

#[test]
fn each_reopen_from_done_replaces_the_previous_verification() {
    let env = env_with_item();
    env.json(&["start", "XX-1"]);
    env.json(&["submit", "XX-1"]);
    env.json(&["reopen", "XX-1", "--reason", "r"]);

    // Mark the first record, so the second can be told from it even when
    // both were made in the same second.
    let mut item = env.item("XX-1");
    item["previous_verification"]["at"] = json!("2026-01-01T00:00:00Z");
    fs::write(
        env.item_path("XX-1"),
        serde_json::to_string_pretty(&item).unwrap() + "\n",
    )
    .unwrap();
    let first = item["previous_verification"].clone();

    // Reaching done again keeps the previous record next to the new one.
    env.json(&["start", "XX-1"]);
    let item = moved(&env, &["submit", "XX-1"]);
    check_verified(&item);
    assert_eq!(item["previous_verification"], first);
    let second = item["verification"].clone();

    let item = moved(&env, &["reopen", "XX-1", "--reason", "r"]);
    check_moved_aside(&item, &second);
}

#[test]
fn reopening_a_slice_2_file_moves_its_record_aside_too() {
    // Slice 2 kept the record on a shelved item, as fixture XX-4 shows.
    let env = Env::new();
    env.add_project("XX", "xx");
    let fixture =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/slice-2/XX-4.json");
    fs::copy(&fixture, env.item_path("XX-4")).unwrap();
    let record = env.item("XX-4")["verification"].clone();
    assert!(record.is_object());

    let item = moved(&env, &["reopen", "XX-4", "--reason", "r"]);
    check_last_move(&item, "shelved", "open", "reopen", Some("r"));
    check_moved_aside(&item, &record);
}

#[test]
fn submit_prints_the_shape_section_3_gives_it() {
    // The triggers (slice 9) fill in `triggers` and `holds`, and agents
    // parsing `submit` before them didn't break when they arrived. The
    // gates that passed are listed (slice 6).
    let env = env_with_item();
    for args in [&["submit", "XX-1"][..], &["status", "XX-1", "done"]] {
        env.json(&["start", "XX-1"]);
        let output = env.json(args);
        let item = output["item"].clone();
        assert_eq!(
            output,
            json!({
                "id": "XX-1",
                "result": "done",
                "status": "done",
                "gates": [
                    { "gate": "G2", "ok": true },
                    { "gate": "G5", "ok": true },
                    { "gate": "G6", "ok": true },
                    { "gate": "G3", "ok": true, "secs": 0 },
                    { "gate": "G4", "ok": true, "secs": 0 }
                ],
                // Both of slice 9's triggers ran, and neither fired.
                "triggers": [
                    { "trigger": "T6", "fired": false },
                    { "trigger": "T9", "fired": false }
                ],
                "holds": [],
                "item": item,
            }),
            "{args:?}"
        );
        // In the order §3 gives them, with `item` last.
        let fields: Vec<&String> = output.as_object().unwrap().keys().collect();
        assert_eq!(
            fields,
            [
                "id", "result", "status", "gates", "triggers", "holds", "item"
            ],
            "{args:?}"
        );
        assert_eq!(env.item("XX-1"), item, "{args:?}");
        env.json(&["reopen", "XX-1", "--reason", "again"]);
    }
}

#[test]
fn status_open_and_status_shelved_need_a_reason() {
    let env = env_with_item();
    env.json(&["start", "XX-1"]);
    env.json(&["submit", "XX-1"]);

    for reason in [None, Some(""), Some("   ")] {
        let mut args = vec!["status", "XX-1", "open"];
        if let Some(reason) = reason {
            args.extend(["--reason", reason]);
        }
        let message = refused(&env, &args);
        assert!(message.contains("needs a reason"), "{message}");
        assert!(message.contains("--reason"), "{message}");
    }

    env.json(&["status", "XX-1", "open", "--reason", "again"]);
    let message = refused(&env, &["status", "XX-1", "shelved"]);
    assert!(message.contains("needs a reason"), "{message}");
}

#[test]
fn status_refuses_a_reason_for_a_move_that_takes_none() {
    let env = env_with_item();
    let message = refused(&env, &["status", "XX-1", "in-progress", "--reason", "why"]);
    assert!(message.contains("--reason"), "{message}");
}

#[test]
fn shelve_works_from_open_and_from_in_progress() {
    let env = env_with_item();
    env.create("XX", "b");

    let item = moved(&env, &["shelve", "XX-1", "--reason", "not now"]);
    check_last_move(&item, "open", "shelved", "shelve", Some("not now"));

    env.json(&["start", "XX-2"]);
    let item = moved(&env, &["shelve", "XX-2", "--reason", "blocked"]);
    check_last_move(&item, "in-progress", "shelved", "shelve", Some("blocked"));
    assert_eq!(history(&item).len(), 2);
}

#[test]
fn shelve_without_a_reason_exits_1() {
    let env = env_with_item();
    for args in [
        &["shelve", "XX-1"][..],
        &["shelve", "XX-1", "--reason", ""],
        &["shelve", "XX-1", "--reason", " \t "],
    ] {
        let error = refused_json(&env, args);
        let message = error["error"].as_str().unwrap();
        assert!(message.contains("needs a reason"), "{args:?}: {message}");
        // The move is drawn, so there's no list of other moves.
        assert!(error.get("allowed").is_none(), "{args:?}: {error}");
    }
    assert_eq!(env.item("XX-1")["status"], "open");
}

#[test]
fn a_reason_must_fit_on_one_line() {
    let env = env_with_item();
    for reason in ["two\nlines", "esc\u{1b}[2K", "rtl \u{202e}txt"] {
        refused(&env, &["shelve", "XX-1", "--reason", reason]);
    }
}

#[test]
fn a_reason_can_be_at_most_500_characters() {
    // Counted in characters, not bytes, so a reason in any language gets
    // the same room.
    let env = env_with_item();
    let too_long = "é".repeat(501);
    let message = refused(&env, &["shelve", "XX-1", "--reason", &too_long]);
    assert!(message.contains("500"), "{message}");
    let message = refused(&env, &["status", "XX-1", "shelved", "--reason", &too_long]);
    assert!(message.contains("500"), "{message}");

    // Surrounding whitespace is trimmed before counting.
    let longest = format!("  {}  ", "é".repeat(500));
    let item = moved(&env, &["shelve", "XX-1", "--reason", &longest]);
    check_last_move(&item, "open", "shelved", "shelve", Some(longest.trim()));
}

#[test]
fn help_shows_that_shelve_and_reopen_need_a_reason() {
    // `--reason` is optional to clap, so that a missing one gets tracker's
    // own message, but the usage line shows it as needed.
    let env = Env::new();
    for (command, usage) in [
        (
            "shelve",
            "Usage: tracker shelve [OPTIONS] <ID> --reason <REASON>\n",
        ),
        (
            "reopen",
            "Usage: tracker reopen [OPTIONS] <ID> --reason <REASON>\n",
        ),
        (
            "status",
            "Usage: tracker status [OPTIONS] <ID> <STATE> [--reason <REASON>]\n",
        ),
    ] {
        let output = env.cmd().args([command, "--help"]).assert().success();
        let stdout = String::from_utf8_lossy(&output.get_output().stdout).to_string();
        assert!(stdout.contains(usage), "{command}: {stdout}");
    }
}

#[test]
fn reopen_brings_a_shelved_item_back_to_open() {
    let env = env_with_item();
    env.json(&["shelve", "XX-1", "--reason", "not now"]);

    let message = refused(&env, &["reopen", "XX-1"]);
    assert!(message.contains("needs a reason"), "{message}");

    let item = moved(&env, &["reopen", "XX-1", "--reason", "now"]);
    check_last_move(&item, "shelved", "open", "reopen", Some("now"));
    assert!(item.get("verification").is_none());

    // And through `status`.
    env.json(&["status", "XX-1", "shelved", "--reason", "not now"]);
    let item = moved(&env, &["status", "XX-1", "open", "--reason", "now"]);
    check_last_move(&item, "shelved", "open", "reopen", Some("now"));
    assert_eq!(history(&item).len(), 4);
}

#[test]
fn open_to_done_exits_1_and_lists_the_allowed_moves() {
    let env = env_with_item();
    for args in [&["submit", "XX-1"][..], &["status", "XX-1", "done"]] {
        let error = refused_json(&env, args);
        assert_eq!(
            error["allowed"],
            json!([
                { "via": "start", "to": "in-progress", "command": START },
                { "via": "shelve", "to": "shelved", "command": SHELVE },
            ]),
            "{args:?}"
        );
    }
}

#[test]
fn every_move_the_diagram_doesnt_draw_is_refused() {
    let env = env_with_item();
    // From open: only start and shelve.
    for args in [
        &["reopen", "XX-1", "--reason", "r"][..],
        &["status", "XX-1", "open", "--reason", "r"],
    ] {
        assert_eq!(allowed(&env, args), [START, SHELVE], "{args:?}");
    }

    // From in-progress: only submit, to done or to review (slice 9), and
    // shelve.
    env.json(&["start", "XX-1"]);
    for args in [
        &["start", "XX-1"][..],
        &["status", "XX-1", "in-progress"],
        &["reopen", "XX-1", "--reason", "r"],
        &["status", "XX-1", "open", "--reason", "r"],
    ] {
        assert_eq!(allowed(&env, args), [SUBMIT, SUBMIT, SHELVE], "{args:?}");
    }

    // From done: only reopen.
    env.json(&["submit", "XX-1"]);
    for args in [
        &["start", "XX-1"][..],
        &["submit", "XX-1"],
        &["shelve", "XX-1", "--reason", "r"],
        &["status", "XX-1", "shelved", "--reason", "r"],
        &["status", "XX-1", "in-progress"],
    ] {
        assert_eq!(allowed(&env, args), [REOPEN], "{args:?}");
    }

    // From shelved: only reopen.
    env.json(&["reopen", "XX-1", "--reason", "r"]);
    env.json(&["shelve", "XX-1", "--reason", "r"]);
    for args in [
        &["start", "XX-1"][..],
        &["submit", "XX-1"],
        &["shelve", "XX-1", "--reason", "r"],
        &["status", "XX-1", "done"],
    ] {
        assert_eq!(allowed(&env, args), [REOPEN], "{args:?}");
    }
}

#[test]
fn nothing_moves_an_item_to_review_by_hand() {
    let env = env_with_item();
    env.json(&["start", "XX-1"]);
    assert_eq!(
        allowed(&env, &["status", "XX-1", "review"]),
        [SUBMIT, SUBMIT, SHELVE]
    );
}

#[test]
fn an_item_in_review_is_refused_by_every_move() {
    let env = env_with_item();
    set_status(&env, "review");
    for args in [
        &["start", "XX-1"][..],
        &["submit", "XX-1"],
        &["shelve", "XX-1", "--reason", "r"],
        &["reopen", "XX-1", "--reason", "r"],
        &["status", "XX-1", "done"],
        &["status", "XX-1", "open", "--reason", "r"],
        &["status", "XX-1", "in-progress"],
    ] {
        // Only approve and reject (slice 9) move a held item. Until slice
        // 9, `allowed` was empty.
        assert_eq!(allowed(&env, args), [APPROVE, REJECT], "{args:?}");
    }
}

#[test]
fn a_move_from_a_terminal_records_interactive_true() {
    let env = env_with_item();
    let Some(output) = env.in_terminal(&["start", "XX-1"]) else {
        return;
    };
    assert!(output.status.success(), "{output:?}");

    let item = env.item("XX-1");
    assert_eq!(item["status"], "in-progress");
    assert_eq!(history(&item).last().unwrap()["interactive"], true);
}

#[test]
fn every_move_adds_exactly_one_history_entry() {
    let env = env_with_item();
    let moves: [&[&str]; 8] = [
        &["start", "XX-1"],
        &["shelve", "XX-1", "--reason", "r"],
        &["reopen", "XX-1", "--reason", "r"],
        &["status", "XX-1", "in-progress"],
        &["submit", "XX-1"],
        &["status", "XX-1", "open", "--reason", "r"],
        &["status", "XX-1", "shelved", "--reason", "r"],
        &["status", "XX-1", "open", "--reason", "r"],
    ];
    for (i, args) in moves.iter().enumerate() {
        let before = env.item("XX-1");
        let after = moved(&env, args);
        assert_eq!(history_len(&after), i + 1, "{args:?}");
        // The entries already there are left exactly as they were.
        if i > 0 {
            assert_eq!(history(&after)[..i], history(&before)[..], "{args:?}");
        }
        // Every entry's `to` is where the next one starts.
        let entry = history(&after).last().unwrap();
        assert_eq!(entry["from"], before["status"], "{args:?}");
        assert_eq!(entry["to"], after["status"], "{args:?}");
    }
}

#[test]
fn a_refused_move_adds_no_history_entry() {
    let env = env_with_item();
    env.json(&["start", "XX-1"]);
    refused(&env, &["start", "XX-1"]);
    refused(&env, &["shelve", "XX-1"]);
    refused(&env, &["status", "XX-1", "review"]);
    assert_eq!(history_len(&env.item("XX-1")), 1);
}

#[test]
fn a_move_on_a_missing_or_malformed_id_exits_1() {
    let env = env_with_item();
    for args in [
        &["start", "XX-9"][..],
        &["submit", "YY-1"],
        &["shelve", "XX-9", "--reason", "r"],
        &["status", "XX-9", "done"],
        &["start", "../config"],
        &["start", "XX-01"],
    ] {
        env.error(args, 1);
    }
    // An unknown state is bad input too, and names the states.
    let message = refused(&env, &["status", "XX-1", "closed"]);
    assert!(message.contains("in-progress"), "{message}");
}

#[test]
fn a_move_finds_an_id_typed_in_lower_case() {
    let env = env_with_item();
    let item = moved(&env, &["start", " xx-1 "]);
    assert_eq!(item["id"], "XX-1");
    assert_eq!(item["status"], "in-progress");
}

#[test]
fn a_move_needs_a_store() {
    let env = Env::without_store();
    let message = env.error(&["start", "XX-1"], 1);
    assert!(message.contains("Run `tracker init`"), "{message}");
}

#[test]
fn list_and_project_list_show_the_new_states() {
    let env = env_with_item();
    env.create("XX", "b");
    env.create("XX", "c");
    env.json(&["start", "XX-1"]);
    env.json(&["submit", "XX-1"]);
    env.json(&["start", "XX-2"]);
    env.json(&["shelve", "XX-3", "--reason", "r"]);

    let states = |args: &[&str]| -> Vec<String> {
        env.json(args)["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["status"].as_str().unwrap().to_string())
            .collect()
    };
    let every_state = "open,in-progress,review,done,shelved";
    assert_eq!(
        states(&["list", "--status", every_state]),
        ["done", "in-progress", "shelved"]
    );
    // Plain `list` hides done and shelved items (slice 3).
    assert_eq!(states(&["list"]), ["in-progress"]);

    let projects = env.json(&["project", "list"]);
    assert_eq!(
        projects["projects"][0]["counts"],
        json!({ "open": 0, "in-progress": 1, "review": 0, "done": 1, "shelved": 1 })
    );
}

#[test]
fn the_text_output_names_the_move() {
    let env = env_with_item();
    let output = env.cmd().args(["start", "XX-1"]).assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout).to_string();
    assert_eq!(stdout, "XX-1: open → in-progress\n");

    let output = env.cmd().args(["submit", "XX-1"]).assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout).to_string();
    assert!(stdout.starts_with("XX-1: in-progress → done\n"), "{stdout}");
    let commit = env.item("XX-1")["verification"]["commit"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        stdout.contains(&format!(
            "Verified automatically: every gate passed at {}, and refs/tracker/XX-1 keeps that \
             commit.\n",
            &commit[..7]
        )),
        "{stdout}"
    );

    // A refusal goes to stderr, with exit 1.
    let output = env.cmd().args(["submit", "XX-1"]).assert().code(1);
    let stderr = String::from_utf8_lossy(&output.get_output().stderr).to_string();
    assert!(stderr.starts_with("error: XX-1 is done"), "{stderr}");
}

#[test]
fn a_move_refuses_a_file_it_would_lose_fields_from() {
    // During self-tracking, the installed tracker is a slice behind the
    // code, so it meets files with fields it doesn't know. Rewriting one
    // would drop them without a word.
    let env = env_with_item();
    let path = env.item_path("XX-1");
    let original = fs::read_to_string(&path).unwrap();
    let edited = original.replace("  \"created\"", "  \"owner\": \"me\",\n  \"created\"");
    assert_ne!(edited, original);
    fs::write(&path, &edited).unwrap();

    for args in [
        &["start", "XX-1"][..],
        &["status", "XX-1", "in-progress"],
        &["shelve", "XX-1", "--reason", "later"],
    ] {
        let message = env.error(args, 3);
        assert!(message.contains("owner"), "{args:?}: {message}");
        assert_eq!(fs::read_to_string(&path).unwrap(), edited, "{args:?}");
    }
}

#[test]
fn a_move_changes_updated_but_never_created() {
    // Backdated by hand, so a move that rewrote `created` with the current
    // time would show, even within the second the item was made in.
    let env = env_with_item();
    let path = env.item_path("XX-1");
    let mut item = env.item("XX-1");
    let long_ago = "2026-01-01T00:00:00Z";
    item["created"] = json!(long_ago);
    item["updated"] = json!(long_ago);
    fs::write(&path, serde_json::to_string_pretty(&item).unwrap() + "\n").unwrap();

    for args in [
        &["start", "XX-1"][..],
        &["submit", "XX-1"],
        &["reopen", "XX-1", "--reason", "found a bug"],
        &["status", "XX-1", "shelved", "--reason", "later"],
        &["status", "XX-1", "open", "--reason", "now"],
    ] {
        let item = moved(&env, args);
        assert_eq!(item["created"], long_ago, "{args:?}");
        let last = history(&item).last().unwrap();
        assert_eq!(item["updated"], last["at"], "{args:?}");
        assert_ne!(item["updated"], long_ago, "{args:?}");
    }
}

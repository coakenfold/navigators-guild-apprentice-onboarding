//! Slice 16 (TR-12): `audit` (DESIGN.md §3, §4.4, §7) reports signs that
//! the store was edited by hand: a status with no matching history, a done
//! item with no verification record, an ID used twice, a `deleted.json`
//! line at or above its project's counter, and a done item whose
//! `refs/tracker/<ID>` is missing or moved.
//!
//! The tests for each of TR-12's criteria are named after it, such as
//! `tr12_c4_`, so each criterion's check runs only its own.

mod common;

use std::fs;

use common::{Env, git, git_commit_all};
use serde_json::{Value, json};

/// Project XX, with its root at `<work>/xx`.
fn env_with_project() -> Env {
    let env = Env::new();
    env.add_project("XX", "xx");
    env
}

/// Changes item `id`'s file by hand.
fn edit_file(env: &Env, id: &str, change: impl FnOnce(&mut Value)) {
    let mut item = env.item(id);
    change(&mut item);
    let text = serde_json::to_string_pretty(&item).unwrap();
    fs::write(env.item_path(id), format!("{text}\n")).unwrap();
}

/// Writes `lines` to `deleted.json`, one JSON object per line.
fn write_deleted(env: &Env, lines: &[Value]) {
    let text: Vec<String> = lines.iter().map(Value::to_string).collect();
    fs::write(
        env.home.path().join("deleted.json"),
        format!("{}\n", text.join("\n")),
    )
    .unwrap();
}

/// A `deleted.json` line for `id`.
fn deleted_line(id: &str) -> Value {
    json!({
        "id": id,
        "title": "gone",
        "status": "open",
        "at": "2026-11-20T10:00:00Z",
        "interactive": true,
    })
}

/// `audit --json`'s problems, as `(problem, id)` pairs.
fn problems(env: &Env) -> Vec<(String, String)> {
    let output = env.json(&["audit"]);
    output["problems"]
        .as_array()
        .unwrap()
        .iter()
        .map(|problem| {
            assert!(problem["message"].as_str().is_some_and(|m| !m.is_empty()));
            (
                problem["problem"].as_str().unwrap().to_string(),
                problem["id"].as_str().unwrap_or("").to_string(),
            )
        })
        .collect()
}

fn pair(problem: &str, id: &str) -> (String, String) {
    (problem.to_string(), id.to_string())
}

/// The text of `tracker audit`, which must exit 0.
fn audit_text(env: &Env) -> String {
    let output = env
        .cmd()
        .arg("audit")
        .assert()
        .success()
        .get_output()
        .clone();
    String::from_utf8(output.stdout).unwrap()
}

/// Creates an item in XX with a script criterion, starts it and submits
/// it, which auto-verifies it and sets its ref. Returns its ID.
fn auto_verified(env: &Env, title: &str) -> String {
    let id = env.create("XX", title);
    env.json(&["start", &id]);
    assert_eq!(env.json(&["submit", &id])["result"], "done");
    id
}

mod tr12_c4 {
    //! `audit` reports a hand-edited status, a done item with no
    //! verification record, and an ID used twice.
    use super::*;

    #[test]
    fn tr12_c4_audit_reports_a_hand_edited_status() {
        let env = env_with_project();
        env.create("XX", "edited");
        env.create("XX", "untouched");
        assert_eq!(problems(&env), Vec::<(String, String)>::new());

        edit_file(&env, "XX-1", |item| item["status"] = json!("in-progress"));
        assert_eq!(problems(&env), [pair("status", "XX-1")]);
        let shown = audit_text(&env);
        assert!(shown.contains("XX-1"), "{shown}");
        assert!(!shown.contains("XX-2"), "{shown}");
    }

    #[test]
    fn tr12_c4_audit_reports_a_done_item_with_no_verification_record() {
        let env = env_with_project();
        auto_verified(&env, "verified");
        assert_eq!(problems(&env), Vec::<(String, String)>::new());

        edit_file(&env, "XX-1", |item| {
            item.as_object_mut().unwrap().remove("verification");
        });
        assert_eq!(problems(&env), [pair("verification", "XX-1")]);
    }

    #[test]
    fn tr12_c4_audit_reports_an_id_used_twice() {
        let env = env_with_project();
        env.create("XX", "one");
        env.create("XX", "two");
        env.create("XX", "three");
        // XX-2 is still an item, and deleted.json says it was deleted.
        // XX-3's file is gone, and two lines say it was deleted.
        fs::remove_file(env.item_path("XX-3")).unwrap();
        write_deleted(
            &env,
            &[
                deleted_line("XX-2"),
                deleted_line("XX-3"),
                deleted_line("XX-3"),
            ],
        );
        assert_eq!(problems(&env), [pair("id", "XX-2"), pair("id", "XX-3")]);
    }
}

mod tr12_c5 {
    //! `audit` reports a `deleted.json` line whose ID is at or above its
    //! project's counter in `counters.json`. Such a line can only have been
    //! written by hand, and `create`'s tripwire refuses that project until
    //! the counter is raised (slice 4 review 1, finding 7).
    use super::*;

    #[test]
    fn tr12_c5_audit_reports_a_deleted_line_at_or_above_its_counter() {
        let env = env_with_project();
        env.add_project("YY", "yy");
        env.create("XX", "one");
        // XX's counter is 2. XX-2 is at it, XX-7 above it, and YY has no
        // counter at all.
        write_deleted(
            &env,
            &[
                deleted_line("XX-2"),
                deleted_line("XX-7"),
                deleted_line("YY-1"),
            ],
        );
        assert_eq!(
            problems(&env),
            [
                pair("deleted", "XX-2"),
                pair("deleted", "XX-7"),
                pair("deleted", "YY-1")
            ]
        );
        let shown = audit_text(&env);
        assert!(shown.contains("counters.json"), "{shown}");

        // The tripwire refuses XX until its counter is raised.
        env.error(&["create", "next", "--project", "XX"], 3);
        let path = env.home.path().join("counters.json");
        fs::write(&path, "{\"XX\": 8, \"YY\": 2}\n").unwrap();
        env.json(&["create", "next", "--project", "XX"]);
        assert_eq!(problems(&env), Vec::<(String, String)>::new());
    }

    #[test]
    fn tr12_c5_audit_reports_a_line_that_cant_be_read() {
        let env = env_with_project();
        fs::write(env.home.path().join("deleted.json"), "not json\n").unwrap();
        assert_eq!(problems(&env), [pair("deleted", "")]);
    }
}

mod tr12_c7 {
    //! `audit` reports a done `auto` or `human` item whose
    //! `refs/tracker/<ID>` is missing, or doesn't point at its record's
    //! `commit` (slice 8 review 1, design 1).
    use super::*;

    #[test]
    fn tr12_c7_audit_reports_a_missing_or_moved_ref() {
        let env = env_with_project();
        auto_verified(&env, "ref removed");
        auto_verified(&env, "ref moved");
        auto_verified(&env, "untouched");
        assert_eq!(problems(&env), Vec::<(String, String)>::new());

        let work = env.work.path();
        git(work, &["update-ref", "-d", "refs/tracker/XX-1"]);
        let other = git_commit_all(work, "chore: something else");
        git(work, &["update-ref", "refs/tracker/XX-2", &other]);
        assert_eq!(problems(&env), [pair("ref", "XX-1"), pair("ref", "XX-2")]);
        let shown = audit_text(&env);
        assert!(shown.contains("refs/tracker/XX-1"), "{shown}");
        assert!(shown.contains("refs/tracker/XX-2"), "{shown}");
    }

    #[test]
    fn tr12_c7_audit_reports_a_human_approved_items_missing_ref() {
        let env = env_with_project();
        if env.in_terminal(&["--version"]).is_none() {
            return;
        }
        let id = env.create("XX", "approved");
        env.json(&["edit", &id, "--label", "needs-review"]);
        env.json(&["start", &id]);
        assert_eq!(env.json(&["submit", &id])["result"], "held");
        let (shown, ok) = env
            .in_terminal_typing(
                &["approve", &id],
                "or anything else to cancel: ",
                &id,
                || {},
            )
            .unwrap();
        assert!(ok, "{shown}");
        assert_eq!(env.item(&id)["verification"]["by"], "human");
        assert_eq!(problems(&env), Vec::<(String, String)>::new());

        git(env.work.path(), &["update-ref", "-d", "refs/tracker/XX-1"]);
        assert_eq!(problems(&env), [pair("ref", "XX-1")]);
    }
}

mod tr12_held {
    //! `audit` reports holds and held questions that only an item in
    //! review can have (slice 15's reviews, "Slice 16 must decide").
    use super::*;

    #[test]
    fn tr12_held_audit_reports_holds_on_an_item_not_in_review() {
        let env = env_with_project();
        env.create("XX", "open");
        edit_file(&env, "XX-1", |item| {
            item["holds"] = json!([{ "trigger": "T9", "message": "asked for" }]);
        });
        assert_eq!(problems(&env), [pair("held", "XX-1")]);
    }
}

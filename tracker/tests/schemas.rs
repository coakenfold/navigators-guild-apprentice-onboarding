//! The `--json` output is an interface (DESIGN.md §3), defined by the JSON
//! Schemas in `schemas/`. `Env::json` checks every successful output against
//! its command's schema, and `error_message` checks errors. These tests check
//! the schemas themselves, the item files, and that a wrong output is caught.

mod common;

use std::fs;

use common::{Env, check_schema, schemas};
use serde_json::{Value, json};

#[test]
fn every_schema_is_a_valid_schema_named_after_its_file() {
    let schemas = schemas();
    assert!(!schemas.is_empty());
    for (name, schema) in &schemas {
        assert!(
            jsonschema::meta::is_valid(schema),
            "{name} isn't a valid schema"
        );
        assert_eq!(
            schema["$schema"], "https://json-schema.org/draft/2020-12/schema",
            "{name}"
        );
        let id = format!("https://tracker.invalid/schemas/{name}.schema.json");
        assert_eq!(schema["$id"], id.as_str(), "{name}'s $id");
    }
}

#[test]
fn every_item_file_matches_the_item_schema() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.add_project("YY", "yy");
    env.create("XX", "plain");
    env.json(&[
        "create",
        "Ünïcode “quotes” and \\ a backslash",
        "--project",
        "YY",
        "--description",
        "line one\nline \"two\"\n\ttabbed é 😀",
    ]);

    let mut checked = 0;
    for entry in fs::read_dir(env.home.path().join("items")).unwrap() {
        let text = fs::read_to_string(entry.unwrap().path()).unwrap();
        let item: Value = serde_json::from_str(&text).unwrap();
        check_schema("item", &item);
        checked += 1;
    }
    assert_eq!(checked, 2);
}

/// A real item, for the tests below to break.
fn item() -> Value {
    json!({
        "id": "XX-1",
        "title": "t",
        "project": "XX",
        "kind": "change",
        "status": "open",
        "priority": "medium",
        "created": "2026-11-15T22:01:56Z",
        "updated": "2026-11-15T22:01:56Z"
    })
}

fn fails(name: &str, value: Value) {
    let result = std::panic::catch_unwind(|| check_schema(name, &value));
    assert!(result.is_err(), "{name} accepted {value:#}");
}

#[test]
fn a_wrong_output_is_caught() {
    check_schema("item", &item());
    check_schema("create", &json!({ "item": item() }));
    let mut unicode = item();
    unicode["title"] = json!("Ünïcode “quotes” — and emoji 😀");
    check_schema("item", &unicode);

    // A field the schema doesn't name, or a missing one.
    let mut extra = item();
    extra["owner"] = json!("me");
    fails("item", extra);
    let mut missing = item();
    missing.as_object_mut().unwrap().remove("updated");
    fails("item", missing);

    // Values outside what §2.1 and §4.3 allow.
    for (field, value) in [
        ("id", json!("xx-1")),
        ("id", json!("XX-0")),
        ("kind", json!("feature")),
        ("status", json!("closed")),
        ("created", json!("2026-11-15T22:01:56.123Z")),
        ("created", json!("not a time")),
        ("title", json!("")),
        // One line with no control characters (§2.1), as `create` checks.
        ("title", json!("two\nlines")),
        ("title", json!("tab\there")),
        ("title", json!("esc\u{1b}[2K")),
        ("title", json!("c1\u{9b}31m")),
        ("title", json!("del\u{7f}")),
        ("title", json!("ls\u{2028}ps")),
        ("title", json!("rtl \u{202e}txt")),
        ("title", json!("lri \u{2066}x")),
    ] {
        let mut bad = item();
        bad[field] = value;
        fails("item", bad);
    }

    // Through a `$ref` to another file: a bad item inside create and list.
    let mut bad = item();
    bad["status"] = json!("closed");
    fails("create", json!({ "item": bad.clone() }));
    fails(
        "list",
        json!({ "project": null, "items": [item(), bad], "skipped": [] }),
    );
    // list says which project it covered: a key, or null for every one.
    check_schema(
        "list",
        &json!({ "project": "XX", "items": [item()], "skipped": [] }),
    );
    check_schema(
        "list",
        &json!({ "project": null, "items": [], "skipped": [] }),
    );
    fails("list", json!({ "items": [item()], "skipped": [] }));
    fails(
        "list",
        json!({ "project": "xx", "items": [], "skipped": [] }),
    );
    // And which files it skipped: always there, with a line or null.
    fails("list", json!({ "project": "XX", "items": [] }));
    check_schema(
        "list",
        &json!({ "project": "XX", "items": [], "skipped": [
            { "file": "/s/items/XX-2.json", "line": 3, "column": 5, "reason": "expected `,`" },
            { "file": "/s/items/notes.json", "line": null, "column": null, "reason": "not an ID" },
        ] }),
    );
    fails(
        "list",
        json!({ "project": "XX", "items": [], "skipped": [{ "file": "/s/items/XX-2.json" }] }),
    );
    // create's item is wrapped, never bare.
    fails("create", item());

    // A moved item: its verification record and history (§2.2, §4.3).
    let mut moved = item();
    moved["status"] = json!("done");
    moved["verification"] = json!({
        "by": "unchecked",
        "at": "2026-11-15T22:02:00Z",
        "note": "No gates ran."
    });
    moved["history"] = json!([
        { "at": "2026-11-15T22:01:58Z", "from": "open", "to": "in-progress", "via": "start", "interactive": false },
        { "at": "2026-11-15T22:02:00Z", "from": "in-progress", "to": "done", "via": "submit", "interactive": true }
    ]);
    check_schema("item", &moved);
    check_schema("move", &json!({ "item": moved.clone() }));
    fails("move", moved.clone());
    for (pointer, value) in [
        ("/verification/by", json!("agent")),
        ("/verification/at", json!("yesterday")),
        ("/verification/note", json!("")),
        ("/verification/extra", json!(1)),
        ("/history/0/via", json!("status")),
        ("/history/0/from", json!("closed")),
        ("/history/0/interactive", json!("no")),
        ("/history/0/reason", json!("")),
        ("/history/0/reason", json!("two\nlines")),
        ("/history/0/reason", json!("é".repeat(501))),
        ("/history/0/extra", json!(1)),
    ] {
        let mut bad = moved.clone();
        let (parent, field) = pointer.rsplit_once('/').unwrap();
        bad.pointer_mut(parent).unwrap()[field] = value;
        fails("item", bad);
    }
    for field in ["at", "from", "to", "via", "interactive"] {
        let mut bad = moved.clone();
        bad["history"][0].as_object_mut().unwrap().remove(field);
        fails("item", bad);
    }
    let mut bad = moved.clone();
    bad["history"] = json!([]);
    fails("item", bad);

    // Labels and a design reference (slice 3, §2.1).
    let mut organised = item();
    organised["labels"] = json!(["bug", "needs review", "ünï"]);
    organised["design"] = json!("DESIGN.md#2.4");
    check_schema("item", &organised);
    for (field, value) in [
        ("labels", json!([])),
        ("labels", json!(["bug", "bug"])),
        ("labels", json!(["Bug"])),
        ("labels", json!(["Ünï"])),
        ("labels", json!([""])),
        ("labels", json!([" bug"])),
        ("labels", json!(["two\nlines"])),
        ("labels", json!("bug")),
        ("design", json!("")),
        ("design", json!("a\nb")),
    ] {
        let mut bad = organised.clone();
        bad[field] = value;
        fails("item", bad);
    }

    // An edit's history entry: from equal to to, and the fields it changed.
    let edit = json!({
        "at": "2026-11-15T22:03:00Z", "from": "open", "to": "open", "via": "edit",
        "interactive": false, "fields": ["priority", "labels"]
    });
    let mut edited = organised.clone();
    edited["history"] = json!([edit.clone()]);
    check_schema("item", &edited);
    // An edit can also record the kind it replaced and the labels it removed.
    let mut weakening = edit.clone();
    weakening["fields"] = json!(["kind", "labels"]);
    weakening["kind_before"] = json!("capability");
    weakening["labels_removed"] = json!(["needs-review", "bug"]);
    let mut weakened = organised.clone();
    weakened["history"] = json!([weakening.clone()]);
    check_schema("item", &weakened);
    check_schema(
        "edit",
        &json!({ "fields": ["priority", "labels"], "labels_not_found": [], "item": edited.clone() }),
    );
    check_schema(
        "edit",
        &json!({ "fields": [], "labels_not_found": ["ui"], "item": organised.clone() }),
    );
    fails(
        "edit",
        json!({ "labels_not_found": [], "item": edited.clone() }),
    );
    fails("edit", json!({ "fields": [], "item": organised.clone() }));
    fails(
        "edit",
        json!({ "fields": [], "labels_not_found": ["ui", "ui"], "item": organised.clone() }),
    );
    fails(
        "edit",
        json!({ "fields": ["status"], "labels_not_found": [], "item": edited.clone() }),
    );
    for bad_entry in [
        // An edit names what it changed, and has no reason.
        {
            let mut entry = edit.clone();
            entry.as_object_mut().unwrap().remove("fields");
            entry
        },
        {
            let mut entry = edit.clone();
            entry["reason"] = json!("why");
            entry
        },
        {
            let mut entry = edit.clone();
            entry["fields"] = json!([]);
            entry
        },
        {
            let mut entry = edit.clone();
            entry["fields"] = json!(["status"]);
            entry
        },
        {
            let mut entry = edit.clone();
            entry["fields"] = json!(["title", "title"]);
            entry
        },
        // The labels it removed, each once, and a kind it could have had.
        {
            let mut entry = weakening.clone();
            entry["labels_removed"] = json!(["bug", "bug"]);
            entry
        },
        {
            let mut entry = weakening.clone();
            entry["labels_removed"] = json!([]);
            entry
        },
        {
            let mut entry = weakening.clone();
            entry["labels_removed"] = json!([""]);
            entry
        },
        {
            let mut entry = weakening.clone();
            entry["kind_before"] = json!("feature");
            entry
        },
        // A move has no fields, kind_before or labels_removed.
        json!({
            "at": "2026-11-15T22:03:00Z", "from": "open", "to": "in-progress", "via": "start",
            "interactive": false, "fields": ["title"]
        }),
        json!({
            "at": "2026-11-15T22:03:00Z", "from": "open", "to": "in-progress", "via": "start",
            "interactive": false, "kind_before": "capability"
        }),
        json!({
            "at": "2026-11-15T22:03:00Z", "from": "open", "to": "in-progress", "via": "start",
            "interactive": false, "labels_removed": ["needs-review"]
        }),
    ] {
        let mut bad = organised.clone();
        bad["history"] = json!([bad_entry]);
        fails("item", bad);
    }

    // The error shape.
    check_schema("error", &json!({ "error": "x", "exit_code": 1 }));
    fails("error", json!({ "error": "x", "exit_code": 0 }));
    fails("error", json!({ "error": "x" }));
    fails("error", json!({ "message": "x", "exit_code": 1 }));
    // A refused move lists the moves allowed instead.
    let step = json!({ "via": "start", "to": "in-progress", "command": "tracker start XX-1" });
    check_schema(
        "error",
        &json!({ "error": "x", "exit_code": 1, "allowed": [step] }),
    );
    check_schema(
        "error",
        &json!({ "error": "x", "exit_code": 1, "allowed": [] }),
    );
    fails(
        "error",
        json!({ "error": "x", "exit_code": 1, "allowed": [{ "via": "start" }] }),
    );
}

#[test]
fn a_wrong_show_or_delete_output_is_caught() {
    // show wraps the item, as create does.
    check_schema("show", &json!({ "item": item() }));
    fails("show", item());
    let mut bad = item();
    bad["status"] = json!("closed");
    fails("show", json!({ "item": bad }));

    // A line of deleted.json: the ID, title, state, time and interactive,
    // and nothing else of the item (DESIGN.md §2.8).
    let record = json!({
        "id": "XX-2",
        "title": "Gone",
        "status": "shelved",
        "at": "2026-11-16T12:00:00Z",
        "interactive": true
    });
    check_schema("deleted", &record);
    check_schema("delete", &json!({ "deleted": record.clone() }));
    fails("delete", record.clone());
    for field in ["id", "title", "status", "at", "interactive"] {
        let mut bad = record.clone();
        bad.as_object_mut().unwrap().remove(field);
        fails("deleted", bad);
    }
    for (field, value) in [
        ("id", json!("xx-2")),
        ("title", json!("")),
        ("title", json!("two\nlines")),
        ("status", json!("closed")),
        ("at", json!("yesterday")),
        ("interactive", json!("yes")),
        // Nothing else of the item.
        ("description", json!("d")),
        ("history", json!([])),
    ] {
        let mut bad = record.clone();
        bad[field] = value;
        fails("deleted", bad);
    }
}

#[test]
fn a_wrong_criterion_or_gate_output_is_caught() {
    // Criteria, and the text of those written at create (slice 6, DESIGN.md
    // §2.1, §2.3).
    let mut criteria = item();
    criteria["written"] = json!(["a"]);
    criteria["criteria"] = json!([
        { "text": "a" },
        { "text": "b", "check": "cargo test b" },
        { "text": "c", "evidence": "docs/c.png" }
    ]);
    check_schema("item", &criteria);
    check_schema(
        "criterion",
        &json!({ "criterion": 1, "item": criteria.clone() }),
    );
    fails(
        "criterion",
        json!({ "criterion": 0, "item": criteria.clone() }),
    );
    fails("criterion", json!({ "item": criteria.clone() }));
    for (pointer, value) in [
        ("/written", json!([])),
        ("/written", json!([""])),
        ("/written", json!(["two\nlines"])),
        ("/criteria", json!([])),
        ("/criteria/0/text", json!("")),
        ("/criteria/0/text", json!("two\nlines")),
        ("/criteria/1/check", json!("")),
        ("/criteria/1/check", json!("a\nb")),
        ("/criteria/2/evidence", json!("")),
        ("/criteria/0/kind", json!("script")),
        // Only a manual criterion has evidence.
        ("/criteria/1/evidence", json!("trust me")),
    ] {
        let mut bad = criteria.clone();
        let (parent, field) = pointer.rsplit_once('/').unwrap();
        bad.pointer_mut(parent).unwrap()[field] = value;
        fails("item", bad);
    }
    let mut bad = criteria.clone();
    bad["criteria"][0].as_object_mut().unwrap().remove("text");
    fails("item", bad);

    // A criterion accept's history entry: from equal to to, naming written.
    let accept = json!({
        "at": "2026-11-15T22:03:00Z", "from": "open", "to": "open",
        "via": "criterion-accept", "interactive": true, "fields": ["written"]
    });
    let mut accepted = criteria.clone();
    accepted["history"] = json!([accept.clone()]);
    check_schema("item", &accepted);
    check_schema(
        "criterion-accept",
        &json!({ "accepted": true, "item": accepted.clone() }),
    );
    fails("criterion-accept", json!({ "item": accepted.clone() }));
    for change in [
        ("fields", json!(["title"])),
        ("fields", json!(["written", "title"])),
        ("reason", json!("why")),
        ("kind_before", json!("capability")),
    ] {
        let mut entry = accept.clone();
        entry[change.0] = change.1;
        let mut bad = criteria.clone();
        bad["history"] = json!([entry]);
        fails("item", bad);
    }
    let mut entry = accept.clone();
    entry.as_object_mut().unwrap().remove("fields");
    let mut bad = criteria.clone();
    bad["history"] = json!([entry]);
    fails("item", bad);
    // An edit can't claim to have changed written.
    let mut bad = criteria.clone();
    bad["history"] = json!([{
        "at": "2026-11-15T22:03:00Z", "from": "open", "to": "open", "via": "edit",
        "interactive": false, "fields": ["written"]
    }]);
    fails("item", bad);

    // The gates dry run, when every gate passed.
    let gates = json!({
        "id": "XX-1", "status": "open", "result": "passed",
        "gates": [{ "gate": "G2", "ok": true }, { "gate": "G6", "ok": true }]
    });
    check_schema("gates", &gates);
    for (field, value) in [
        ("result", json!("refused")),
        ("gates", json!([])),
        ("gates", json!([{ "gate": "G2", "ok": false }])),
        ("gates", json!([{ "gate": "T1", "ok": true }])),
        ("status", json!("closed")),
    ] {
        let mut bad = gates.clone();
        bad[field] = value;
        fails("gates", bad);
    }

    // A gate's refusal: the error shape, exit 2, with every failure.
    let refusal = json!({
        "error": "XX-1 can't be submitted: G2 failed. Nothing was changed.",
        "exit_code": 2,
        "id": "XX-1",
        "status": "in-progress",
        "result": "refused",
        "gates": [{ "gate": "G2", "ok": false }, { "gate": "G6", "ok": true }],
        "failures": [
            { "gate": "G2", "criterion": 1, "message": "no evidence" },
            { "gate": "G6", "message": "no section" }
        ]
    });
    check_schema("error", &refusal);
    let mut bad = refusal.clone();
    bad["exit_code"] = json!(1);
    fails("error", bad);
    for field in ["id", "status", "gates", "failures"] {
        let mut bad = refusal.clone();
        bad.as_object_mut().unwrap().remove(field);
        fails("error", bad);
    }
    for (pointer, value) in [
        ("/failures/0/criterion", json!(0)),
        ("/failures/0/message", json!("")),
        ("/failures/0/extra", json!(1)),
        ("/result", json!("held")),
        ("/status", json!("closed")),
    ] {
        let mut bad = refusal.clone();
        let (parent, field) = pointer.rsplit_once('/').unwrap();
        bad.pointer_mut(parent).unwrap()[field] = value;
        fails("error", bad);
    }
    fails(
        "error",
        json!({ "error": "x", "exit_code": 1, "gates": [{ "gate": "G1", "ok": false }] }),
    );
    fails(
        "error",
        json!({ "error": "x", "exit_code": 1, "status": "open" }),
    );
}

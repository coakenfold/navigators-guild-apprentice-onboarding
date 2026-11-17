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
    extra["labels"] = json!(["bug"]);
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
    fails("list", json!({ "items": [item(), bad] }));
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

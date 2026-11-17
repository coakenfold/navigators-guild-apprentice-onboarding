//! Item files are read back only as the item their name says, so a file
//! copied or edited by hand can't pass for another item (§4.4).

mod common;

use std::fs;

use common::Env;
use tracker::store::Store;

fn store(env: &Env) -> Store {
    Store {
        home: env.home.path().to_path_buf(),
    }
}

#[test]
fn a_file_whose_name_and_id_disagree_is_refused() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");
    // A copy made by hand: XX-9.json still says it's XX-1.
    fs::copy(env.item_path("XX-1"), env.item_path("XX-9")).unwrap();

    for args in [&["list"][..], &["project", "list"]] {
        let message = env.error(args, 3);
        assert!(message.contains("XX-9.json"), "{args:?}: {message}");
        assert!(message.contains("XX-1"), "{args:?}: {message}");
    }
    let error = store(&env).load_item("XX-9").unwrap_err();
    assert_eq!(error.code, 3);
}

#[test]
fn a_file_whose_project_and_id_disagree_is_refused() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.add_project("YY", "yy");
    env.create("XX", "a");
    let path = env.item_path("XX-1");
    let text = fs::read_to_string(&path).unwrap();
    fs::write(
        &path,
        text.replace("\"project\": \"XX\"", "\"project\": \"YY\""),
    )
    .unwrap();

    let message = env.error(&["list"], 3);
    assert!(message.contains("XX-1.json"), "{message}");
    assert!(message.contains("\"project\""), "{message}");
}

#[test]
fn a_json_file_not_named_after_an_id_is_refused() {
    let env = Env::new();
    env.add_project("XX", "xx");
    let text = {
        env.create("XX", "a");
        fs::read_to_string(env.item_path("XX-1")).unwrap()
    };
    fs::write(env.home.path().join("items/notes.json"), text).unwrap();

    let message = env.error(&["list"], 3);
    assert!(message.contains("notes.json"), "{message}");
}

#[test]
fn load_item_reads_one_item_and_says_when_there_is_none() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");
    let store = store(&env);

    assert_eq!(store.load_item("XX-1").unwrap().title, "a");
    let error = store.load_item("XX-2").unwrap_err();
    assert_eq!(error.code, 1);
    assert!(error.message.contains("no item XX-2"), "{}", error.message);
}

#[test]
fn update_item_rewrites_a_file_this_version_understands() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "before");
    let store = store(&env);

    let mut item = store.load_item("XX-1").unwrap();
    item.title = "after".to_string();
    store.update_item(&item).unwrap();
    assert_eq!(store.load_item("XX-1").unwrap().title, "after");
    assert_eq!(
        fs::read_to_string(env.item_path("XX-1")).unwrap(),
        tracker::item::to_json(&item)
    );
}

#[test]
fn update_item_refuses_a_file_it_would_lose_fields_from() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");
    let store = store(&env);
    let path = env.item_path("XX-1");
    let item = store.load_item("XX-1").unwrap();
    let original = fs::read_to_string(&path).unwrap();

    // A field from a newer tracker, and a time written by hand that this
    // version would write back as `Z` instead of `+00:00`.
    for (field, edited) in [
        (
            "written",
            original.replace(
                "  \"created\"",
                "  \"written\": [\"71ac3e\"],\n  \"created\"",
            ),
        ),
        ("created", original.replacen("Z\",", "+00:00\",", 1)),
    ] {
        assert_ne!(edited, original);
        fs::write(&path, &edited).unwrap();
        // Still listed: only rewriting it would lose something.
        assert_eq!(store.load_items().unwrap().len(), 1);

        let error = store.update_item(&item).unwrap_err();
        assert_eq!(error.code, 3, "{field}");
        assert!(error.message.contains(field), "{field}: {}", error.message);
        assert_eq!(fs::read_to_string(&path).unwrap(), edited, "{field}");
    }
}

#[test]
fn update_item_of_a_missing_item_exits_1() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");
    let store = store(&env);
    let mut item = store.load_item("XX-1").unwrap();
    item.id = "XX-2".to_string();

    assert_eq!(store.update_item(&item).unwrap_err().code, 1);
    assert!(!env.item_path("XX-2").exists());
}

#[test]
fn save_item_never_overwrites_a_file() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");
    let store = store(&env);
    let mut item = store.load_item("XX-1").unwrap();
    let original = fs::read_to_string(env.item_path("XX-1")).unwrap();
    item.title = "replacement".to_string();

    assert_eq!(store.save_item(&item).unwrap_err().code, 3);
    assert_eq!(fs::read_to_string(env.item_path("XX-1")).unwrap(), original);
}

/// Adds a history entry the way slice 6's `criterion accept` will write
/// one, with a `via` this version doesn't know, and returns the file's new
/// text.
fn add_newer_entry(env: &Env, id: &str) -> String {
    let path = env.item_path(id);
    let mut item: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let status = item["status"].clone();
    let entry = serde_json::json!({
        "at": "2026-11-16T12:00:00Z",
        "from": status,
        "to": status,
        "via": "criterion-accept",
        "interactive": true,
    });
    match item["history"].as_array_mut() {
        Some(history) => history.push(entry),
        None => item["history"] = serde_json::json!([entry]),
    }
    let text = serde_json::to_string_pretty(&item).unwrap() + "\n";
    fs::write(&path, &text).unwrap();
    text
}

#[test]
fn a_history_entry_from_a_newer_tracker_still_lists() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");
    env.create("XX", "b");
    env.json(&["start", "XX-1"]);
    add_newer_entry(&env, "XX-1");

    let items = env.json(&["list"])["items"].clone();
    let ids: Vec<&str> = items
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["XX-1", "XX-2"]);
    let history = items[0]["history"].as_array().unwrap();
    assert_eq!(history[1]["via"], "unknown");
    env.json(&["project", "list"]);
}

#[test]
fn a_move_refuses_an_item_whose_history_has_a_command_it_doesnt_know() {
    // Writing the item back would turn `criterion-accept` into `unknown`.
    // Other items are unaffected.
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");
    env.create("XX", "b");
    env.json(&["start", "XX-1"]);
    let edited = add_newer_entry(&env, "XX-1");
    let path = env.item_path("XX-1");

    for args in [
        &["submit", "XX-1"][..],
        &["status", "XX-1", "done"],
        &["shelve", "XX-1", "--reason", "later"],
        &["edit", "XX-1", "--priority", "high"],
    ] {
        let message = env.error(args, 3);
        assert!(message.contains("history"), "{args:?}: {message}");
        assert_eq!(fs::read_to_string(&path).unwrap(), edited, "{args:?}");
    }

    // The text list shows it too, and the other item still moves.
    let output = env
        .cmd()
        .arg("list")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    assert!(
        text.contains("XX-1  medium  change  in-progress  a"),
        "{text}"
    );
    assert_eq!(
        env.json(&["start", "XX-2"])["item"]["status"],
        "in-progress"
    );
}

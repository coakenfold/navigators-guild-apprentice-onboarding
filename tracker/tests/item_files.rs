//! Item files are read back only as the item their name says, so a file
//! copied or edited by hand can't pass for another item (§4.4).

mod common;

use std::fs;
use std::time::{Duration, Instant};

use common::Env;
use tracker::store::Store;

fn store(env: &Env) -> Store {
    Store {
        home: env.home.path().to_path_buf(),
    }
}

/// The files `list --json` skipped, as (file name, line, reason).
fn skipped(list: &serde_json::Value) -> Vec<(String, serde_json::Value, String)> {
    list["skipped"]
        .as_array()
        .unwrap()
        .iter()
        .map(|file| {
            let path = file["file"].as_str().unwrap();
            let name = path.rsplit('/').next().unwrap().to_string();
            (
                name,
                file["line"].clone(),
                file["reason"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

/// The IDs `list --json` listed.
fn ids(list: &serde_json::Value) -> Vec<String> {
    list["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["id"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn a_file_whose_name_and_id_disagree_is_skipped() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");
    // A copy made by hand: XX-9.json still says it's XX-1.
    fs::copy(env.item_path("XX-1"), env.item_path("XX-9")).unwrap();

    let list = env.json(&["list"]);
    assert_eq!(ids(&list), ["XX-1"]);
    let skipped = skipped(&list);
    assert_eq!(skipped.len(), 1, "{skipped:?}");
    assert_eq!(skipped[0].0, "XX-9.json");
    assert_eq!(skipped[0].1, serde_json::Value::Null);
    assert!(skipped[0].2.contains("XX-1"), "{skipped:?}");
    // project list doesn't count it either.
    let projects = env.json(&["project", "list"]);
    assert_eq!(projects["projects"][0]["counts"]["open"], 1);
    assert_eq!(projects["skipped"].as_array().unwrap().len(), 1);

    // A command that names it still refuses.
    let error = store(&env).load_item("XX-9").unwrap_err();
    assert_eq!(error.code, 3);
    let message = env.error(&["show", "XX-9"], 3);
    assert!(message.contains("XX-9.json"), "{message}");
    assert!(message.contains("Rename the file"), "{message}");
}

#[test]
fn a_file_whose_project_and_id_disagree_is_skipped() {
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

    let list = env.json(&["list"]);
    assert_eq!(ids(&list), Vec::<String>::new());
    let skipped = skipped(&list);
    assert_eq!(skipped.len(), 1, "{skipped:?}");
    assert_eq!(skipped[0].0, "XX-1.json");
    assert!(skipped[0].2.contains("\"YY\""), "{skipped:?}");
}

#[test]
fn a_json_file_not_named_after_an_id_is_skipped() {
    let env = Env::new();
    env.add_project("XX", "xx");
    let text = {
        env.create("XX", "a");
        fs::read_to_string(env.item_path("XX-1")).unwrap()
    };
    fs::write(env.home.path().join("items/notes.json"), text).unwrap();

    let list = env.json(&["list"]);
    assert_eq!(ids(&list), ["XX-1"]);
    let skipped = skipped(&list);
    assert_eq!(skipped.len(), 1, "{skipped:?}");
    assert_eq!(skipped[0].0, "notes.json");
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

    // A field from a newer tracker: rewriting the file would lose it.
    let edited = original.replace(
        "  \"created\"",
        "  \"written\": [\"71ac3e\"],\n  \"created\"",
    );
    assert_ne!(edited, original);
    fs::write(&path, &edited).unwrap();
    // Still listed: only rewriting it would lose something.
    assert_eq!(store.load_items().unwrap().0.len(), 1);

    let error = store.update_item(&item).unwrap_err();
    assert_eq!(error.code, 3);
    assert!(error.message.contains("written"), "{}", error.message);
    assert!(error.message.contains("newer tracker"), "{}", error.message);
    assert_eq!(fs::read_to_string(&path).unwrap(), edited);
}

#[test]
fn update_item_rewrites_a_value_only_written_differently() {
    // A time written by hand with `+00:00`, and an empty list, which this
    // version writes as `Z` and leaves out. Nothing is lost, so slice 5
    // lets a move or an edit rewrite them, rather than blame a newer
    // tracker (slice 2 review 2, item 4).
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");
    let path = env.item_path("XX-1");
    let original = fs::read_to_string(&path).unwrap();
    let edited = original
        .replacen("Z\",", "+00:00\",", 1)
        .replace("  \"created\"", "  \"labels\": [],\n  \"created\"");
    fs::write(&path, &edited).unwrap();

    let message = env
        .json(&["edit", "XX-1", "--priority", "high"])
        .to_string();
    assert!(!message.contains("newer tracker"), "{message}");
    let rewritten = fs::read_to_string(&path).unwrap();
    assert!(!rewritten.contains("+00:00"), "{rewritten}");
    assert!(!rewritten.contains("\"labels\""), "{rewritten}");
    assert_eq!(env.item("XX-1")["priority"], "high");
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
    let row: Vec<&str> = text
        .lines()
        .find(|line| line.contains("XX-1"))
        .unwrap()
        .split_whitespace()
        .collect();
    assert_eq!(
        row,
        ["XX-1", "medium", "change", "in-progress", "a"],
        "{text}"
    );
    assert_eq!(
        env.json(&["start", "XX-2"])["item"]["status"],
        "in-progress"
    );
}

#[test]
fn show_prints_an_item_whose_history_has_a_command_it_doesnt_know() {
    // Showing never writes, so the entry is shown as `unknown` and the file
    // stays as it was.
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");
    env.json(&["start", "XX-1"]);
    let edited = add_newer_entry(&env, "XX-1");

    let shown = env.json(&["show", "XX-1"]);
    assert_eq!(shown["item"]["history"][1]["via"], "unknown");
    // And it says the history isn't shown in full.
    let warning = shown["warning"].as_str().unwrap();
    assert!(warning.contains(": history."), "{warning}");
    let output = env.cmd().args(["show", "XX-1"]).assert().success();
    let text = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(text.contains("unknown"), "{text}");
    assert!(text.contains(warning), "{text}");
    assert_eq!(fs::read_to_string(env.item_path("XX-1")).unwrap(), edited);
}

#[test]
fn lost_on_rewrite_tells_unknown_fields_from_values_written_differently() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");
    env.json(&["start", "XX-1"]);
    env.json(&["submit", "XX-1"]);
    let mut item = env.item("XX-1");
    // Written differently: an empty list, a null, a time with an offset,
    // and a null inside a history entry.
    item["labels"] = serde_json::json!([]);
    item["description"] = serde_json::Value::Null;
    let at = item["updated"].as_str().unwrap().replace('Z', "+00:00");
    item["updated"] = serde_json::json!(at);
    item["history"][0]["reason"] = serde_json::Value::Null;
    // Unknown: a field inside a record, and one at the top level.
    item["verification"]["commit"] = serde_json::json!("abc123");
    item["criteria"] = serde_json::json!([]);
    item["owner"] = serde_json::json!("me");
    let text = serde_json::to_string_pretty(&item).unwrap();

    let lost = tracker::item::lost_on_rewrite(&text).unwrap();
    assert_eq!(lost.unknown, ["verification", "owner"]);
    assert_eq!(
        lost.respelled,
        ["updated", "history", "labels", "description", "criteria"]
    );
    assert_eq!(
        lost.fields(),
        [
            "verification",
            "owner",
            "updated",
            "history",
            "labels",
            "description",
            "criteria"
        ]
    );

    // A history entry made by a command this version doesn't know.
    let mut item = env.item("XX-1");
    item["history"][0]["via"] = serde_json::json!("approve");
    let text = serde_json::to_string_pretty(&item).unwrap();
    let lost = tracker::item::lost_on_rewrite(&text).unwrap();
    assert_eq!(lost.unknown, ["history"]);
    assert!(lost.respelled.is_empty());
}

/// Whether `file` can be locked within `limit`, trying again until then.
/// Each lock is released when its file is closed, but a process that
/// another test in this binary starts meanwhile gets a copy of every file
/// this one has open, the lock's included, until its program runs. So for
/// a moment after the lock is dropped, it can still be held: about 1 drop
/// in 100 while processes are being started, for at most 0.2 ms, measured
/// on macOS. A tracker command has one thread, so this can't happen there.
fn locks_within(file: &fs::File, limit: Duration) -> bool {
    let start = Instant::now();
    loop {
        match file.try_lock() {
            Ok(()) => return true,
            Err(fs::TryLockError::WouldBlock) if start.elapsed() < limit => {
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(fs::TryLockError::WouldBlock) => return false,
            Err(fs::TryLockError::Error(e)) => panic!("Can't lock {file:?}: {e}"),
        }
    }
}

#[test]
fn lock_unchanged_holds_the_lock_only_while_the_file_is_as_it_was() {
    // What every human-only command calls after its question (§2.7).
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");
    let store = store(&env);
    let (_, before) = store.load_item_with_text("XX-1").unwrap();
    let command = "tracker delete XX-1";

    // Unchanged: it returns the lock, which is held until it's dropped. The
    // lock file opened a second time can't be locked meanwhile, as another
    // command's couldn't (review 2's test gap 1). Once it's dropped, it can,
    // though not always at once (`locks_within`).
    let lock = store.lock_unchanged("XX-1", &before, command).unwrap();
    let other = fs::File::open(store.lock_path()).unwrap();
    assert!(
        matches!(other.try_lock(), Err(fs::TryLockError::WouldBlock)),
        "lock_unchanged returned without holding the lock"
    );
    drop(lock);
    assert!(
        locks_within(&other, Duration::from_secs(5)),
        "the lock was still held 5 seconds after it was dropped"
    );
    other.unlock().unwrap();

    // Changed, even by one byte.
    fs::write(env.item_path("XX-1"), format!("{before} ")).unwrap();
    let error = store.lock_unchanged("XX-1", &before, command).unwrap_err();
    assert_eq!(error.code, 1);
    assert!(
        error
            .message
            .contains("XX-1 changed while you were deciding"),
        "{}",
        error.message
    );
    assert!(
        error.message.contains("Run `tracker delete XX-1` again"),
        "{}",
        error.message
    );

    // Removed.
    fs::remove_file(env.item_path("XX-1")).unwrap();
    let error = store.lock_unchanged("XX-1", &before, command).unwrap_err();
    assert_eq!(error.code, 1);
    assert!(
        error
            .message
            .contains("XX-1 was removed while you were deciding"),
        "{}",
        error.message
    );

    // The lock was released each time, so a command can still take it.
    env.create("XX", "b");
}

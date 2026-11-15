//! Slice 1: create items in two projects and list them from one store.

mod common;

use std::collections::BTreeSet;
use std::fs;
use std::process::{Command, Stdio};

use common::{Env, parse};
use serde_json::{Value, json};

fn ids(list: &Value) -> Vec<&str> {
    list["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["id"].as_str().unwrap())
        .collect()
}

fn counts(project_list: &Value, key: &str) -> Value {
    let projects = project_list["projects"].as_array().unwrap();
    let project = projects.iter().find(|p| p["key"] == key).unwrap();
    project["counts"].clone()
}

fn open_counts(open: u64) -> Value {
    json!({ "open": open, "in-progress": 0, "review": 0, "done": 0, "shelved": 0 })
}

/// Runs a command that should fail with exit 1 and `--json`, and returns
/// the error message.
fn error_message(env: &Env, args: &[&str]) -> String {
    let output = env.cmd().args(args).arg("--json").assert().code(1);
    let error = parse(&output.get_output().stdout);
    assert_eq!(error["exit_code"], 1);
    error["error"].as_str().unwrap().to_string()
}

#[test]
fn ids_count_up_per_project_and_list_shows_every_item() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.add_project("YY", "yy");

    assert_eq!(env.create("XX", "first"), "XX-1");
    assert_eq!(env.create("XX", "second"), "XX-2");
    assert_eq!(env.create("YY", "third"), "YY-1");

    let list = env.json(&["list"]);
    assert_eq!(ids(&list), ["XX-1", "XX-2", "YY-1"]);
    assert_eq!(list["items"][2]["title"], "third");

    let projects = env.json(&["project", "list"]);
    assert_eq!(counts(&projects, "XX"), open_counts(2));
    assert_eq!(counts(&projects, "YY"), open_counts(1));
}

#[test]
fn create_makes_an_open_medium_change() {
    let env = Env::new();
    env.add_project("XX", "xx");
    let item = env.json(&["create", "  padded title  ", "--project", "XX"]);

    assert_eq!(item["title"], "padded title");
    assert_eq!(item["project"], "XX");
    assert_eq!(item["kind"], "change");
    assert_eq!(item["status"], "open");
    assert_eq!(item["priority"], "medium");
    assert_eq!(item["created"], item["updated"]);
    let created = item["created"].as_str().unwrap();
    assert!(created.parse::<jiff::Timestamp>().is_ok(), "{created}");
}

#[test]
fn project_list_counts_every_state() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");
    env.create("XX", "b");
    env.create("XX", "c");
    // Only `create` exists so far, so set the other states by hand.
    for (id, status) in [("XX-2", "done"), ("XX-3", "in-progress")] {
        let path = env.item_path(id);
        let text = fs::read_to_string(&path).unwrap();
        let text = text.replace("\"status\": \"open\"", &format!("\"status\": \"{status}\""));
        fs::write(&path, text).unwrap();
    }

    let projects = env.json(&["project", "list"]);
    assert_eq!(
        counts(&projects, "XX"),
        json!({ "open": 1, "in-progress": 1, "review": 0, "done": 1, "shelved": 0 })
    );
}

#[test]
fn create_inside_a_project_root_needs_no_project_flag() {
    let env = Env::new();
    let xx = env.add_project("XX", "xx");
    env.add_project("YY", "yy");

    assert_eq!(env.json_in(&xx, &["create", "from the root"])["id"], "XX-1");
    let deeper = env.dir("xx/src/deep");
    assert_eq!(
        env.json_in(&deeper, &["create", "from below"])["id"],
        "XX-2"
    );
}

#[test]
fn project_flag_overrides_the_current_folder() {
    let env = Env::new();
    let xx = env.add_project("XX", "xx");
    env.add_project("YY", "yy");

    let item = env.json_in(&xx, &["create", "for YY", "--project", "YY"]);
    assert_eq!(item["id"], "YY-1");
}

#[test]
fn nested_roots_pick_the_deepest() {
    let env = Env::new();
    env.add_project("MONO", "mono");
    let package = env.add_project("PKG", "mono/packages/pkg");

    let item = env.json_in(&package, &["create", "in the package"]);
    assert_eq!(item["project"], "PKG");
    let other = env.dir("mono/packages/other");
    let item = env.json_in(&other, &["create", "elsewhere in the repo"]);
    assert_eq!(item["project"], "MONO");
}

#[test]
fn create_outside_every_root_without_project_exits_1() {
    let env = Env::new();
    env.add_project("XX", "xx");

    let message = error_message(&env, &["create", "lost"]);
    assert!(message.contains("--project"), "{message}");
    assert!(message.contains("XX"), "{message}");
    assert!(!env.item_path("XX-1").exists());

    // `xx-other` starts with the same characters as `xx` but isn't inside it.
    let sibling = env.dir("xx-other");
    env.cmd()
        .current_dir(&sibling)
        .args(["create", "lost"])
        .assert()
        .code(1);
}

#[test]
fn unknown_project_exits_1_and_names_the_known_ones() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.add_project("YY", "yy");

    let message = error_message(&env, &["create", "t", "--project", "ZZ"]);
    assert!(message.contains("XX, YY"), "{message}");
}

#[test]
fn empty_or_whitespace_title_exits_1() {
    let env = Env::new();
    env.add_project("XX", "xx");

    for title in ["", "   ", "\t \n"] {
        let message = error_message(&env, &["create", title, "--project", "XX"]);
        assert!(message.contains("empty"), "{message}");
    }
    error_message(&env, &["create", "two\nlines", "--project", "XX"]);

    // A refused create doesn't use up an ID.
    assert!(!env.item_path("XX-1").exists());
    assert_eq!(env.create("XX", "a real title"), "XX-1");
}

#[test]
fn description_is_stored_in_the_description_field() {
    let env = Env::new();
    env.add_project("XX", "xx");
    let description = "Tag chips filter the list.\n\n- one\n- \"two\"";

    let item = env.json(&[
        "create",
        "with notes",
        "--project",
        "XX",
        "--description",
        description,
    ]);
    assert_eq!(item["description"], description);
    let file: Value =
        serde_json::from_str(&fs::read_to_string(env.item_path("XX-1")).unwrap()).unwrap();
    assert_eq!(file["description"], description);

    // An item without a description has no `description` field.
    env.create("XX", "no notes");
    let file: Value =
        serde_json::from_str(&fs::read_to_string(env.item_path("XX-2")).unwrap()).unwrap();
    assert!(file.get("description").is_none());
}

#[test]
fn item_file_read_back_and_written_again_is_unchanged() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "plain");
    env.json(&[
        "create",
        "Ünïcode “quotes” and \\ a backslash",
        "--project",
        "XX",
        "--description",
        "line one\nline \"two\"\n\ttabbed é 😀",
    ]);

    for id in ["XX-1", "XX-2"] {
        let text = fs::read_to_string(env.item_path(id)).unwrap();
        let item = tracker::item::from_json(&text).unwrap();
        assert_eq!(tracker::item::to_json(&item), text, "{id} changed");
    }
}

#[test]
fn item_file_has_two_space_indents_and_a_fixed_field_order() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.json(&["create", "t", "--project", "XX", "--description", "d"]);

    let text = fs::read_to_string(env.item_path("XX-1")).unwrap();
    assert!(text.starts_with("{\n  \"id\": \"XX-1\",\n"), "{text}");
    assert!(text.ends_with("}\n"), "{text}");
    let file: Value = serde_json::from_str(&text).unwrap();
    let keys: Vec<&str> = file
        .as_object()
        .unwrap()
        .keys()
        .map(|k| k.as_str())
        .collect();
    let expected = [
        "id",
        "title",
        "project",
        "kind",
        "status",
        "priority",
        "created",
        "updated",
        "description",
    ];
    assert_eq!(keys, expected);
}

#[test]
fn ten_creates_at_once_get_ten_different_ids() {
    let env = Env::new();
    env.add_project("XX", "xx");

    let children: Vec<_> = (0..10)
        .map(|i| {
            Command::new(env!("CARGO_BIN_EXE_tracker"))
                .env("TRACKER_HOME", env.home.path())
                .current_dir(env.outside())
                .args(["create", &format!("item {i}"), "--project", "XX", "--json"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap()
        })
        .collect();

    let mut ids = BTreeSet::new();
    for child in children {
        let output = child.wait_with_output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{stderr}");
        let id = parse(&output.stdout)["id"].as_str().unwrap().to_string();
        ids.insert(id);
    }

    let expected: BTreeSet<String> = (1..=10).map(|n| format!("XX-{n}")).collect();
    assert_eq!(ids, expected);
    assert_eq!(env.read_store_file("counters.json")["XX"], 11);
    let files = fs::read_dir(env.home.path().join("items")).unwrap().count();
    assert_eq!(files, 10, "one file per item, and no temporary files left");
}

#[test]
fn ids_come_from_counters_json_not_from_the_files() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");
    env.create("XX", "b");
    fs::remove_file(env.item_path("XX-2")).unwrap();

    assert_eq!(env.create("XX", "c"), "XX-3");
}

#[test]
fn every_command_prints_json_that_parses() {
    let env = Env::without_store();
    let xx = env.dir("xx");
    let root = xx.to_str().unwrap();

    assert_eq!(env.json(&["init"])["created"], true);
    assert_eq!(env.json(&["init"])["created"], false);
    let added = env.json(&["project", "add", "XX", "--name", "alpha", "--root", root]);
    assert_eq!(added["key"], "XX");
    assert_eq!(added["name"], "alpha");
    assert_eq!(env.json(&["create", "t", "--project", "XX"])["id"], "XX-1");
    assert_eq!(ids(&env.json(&["list"])), ["XX-1"]);
    assert_eq!(env.json(&["project", "list"])["projects"][0]["key"], "XX");

    // `--json` also works before the command.
    let output = env.cmd().args(["--json", "list"]).assert().success();
    assert_eq!(ids(&parse(&output.get_output().stdout)), ["XX-1"]);

    // Failures print one JSON object too, with the same exit code.
    error_message(&env, &["create", " ", "--project", "XX"]);
    error_message(&env, &["create", "t", "--project", "NOPE"]);
    error_message(
        &env,
        &["project", "add", "XX", "--name", "again", "--root", root],
    );
    let message = error_message(&env, &["create"]);
    assert!(message.contains("<TITLE>"), "{message}");
}

#[test]
fn commands_without_a_store_say_run_tracker_init() {
    let env = Env::without_store();
    let commands: [&[&str]; 4] = [
        &["list"],
        &["project", "list"],
        &["create", "t", "--project", "XX"],
        &["project", "add", "XX", "--name", "x", "--root", "."],
    ];
    for args in commands {
        let output = env.cmd().args(args).assert().code(1);
        let stderr = String::from_utf8_lossy(&output.get_output().stderr);
        assert!(stderr.contains("Run `tracker init`."), "{args:?}: {stderr}");
    }
}

#[test]
fn init_again_keeps_the_store() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");

    assert_eq!(env.json(&["init"])["created"], false);
    assert_eq!(env.create("XX", "b"), "XX-2");
    assert_eq!(ids(&env.json(&["list"])), ["XX-1", "XX-2"]);
}

#[test]
fn project_add_refuses_bad_input() {
    let env = Env::new();
    env.add_project("XX", "xx");
    let root = env.dir("other");
    let root = root.to_str().unwrap();
    let file = env.work.path().join("a-file");
    fs::write(&file, "").unwrap();

    error_message(
        &env,
        &["project", "add", "XX", "--name", "dup", "--root", root],
    );
    for key in ["", "1X", "X-Y", "ABCDEFGHIJK"] {
        error_message(
            &env,
            &["project", "add", key, "--name", "n", "--root", root],
        );
    }
    error_message(
        &env,
        &["project", "add", "ZZ", "--name", " ", "--root", root],
    );
    let missing = env.work.path().join("missing");
    error_message(
        &env,
        &[
            "project",
            "add",
            "ZZ",
            "--name",
            "n",
            "--root",
            missing.to_str().unwrap(),
        ],
    );
    error_message(
        &env,
        &[
            "project",
            "add",
            "ZZ",
            "--name",
            "n",
            "--root",
            file.to_str().unwrap(),
        ],
    );

    let projects = env.json(&["project", "list"]);
    assert_eq!(projects["projects"].as_array().unwrap().len(), 1);
}

#[test]
fn project_keys_are_upper_case_and_roots_absolute() {
    let env = Env::new();
    env.dir("bm");
    let added = env.json_in(
        env.work.path(),
        &[
            "project",
            "add",
            "bm",
            "--name",
            "bookmarks",
            "--root",
            "bm",
            "--design",
            "DESIGN.md",
        ],
    );

    assert_eq!(added["key"], "BM");
    let expected = fs::canonicalize(env.work.path().join("bm")).unwrap();
    assert_eq!(added["root"], expected.to_str().unwrap());
    assert_eq!(added["design"], "DESIGN.md");
    assert_eq!(env.create("bm", "lowercase works too"), "BM-1");
}

#[test]
fn project_add_keeps_settings_written_by_hand() {
    let env = Env::new();
    let xx = env.dir("xx");
    let config = json!({
        "projects": {
            "XX": {
                "name": "alpha",
                "root": xx,
                "checks": ["cargo test"],
                "review": { "disabled": ["T8"] }
            }
        },
        "judge": { "model": "claude-haiku-5-5" }
    });
    let config_path = env.home.path().join("config.json");
    fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();

    env.add_project("YY", "yy");

    let saved = env.read_store_file("config.json");
    assert_eq!(saved["projects"]["XX"], config["projects"]["XX"]);
    assert_eq!(saved["judge"], config["judge"]);
    assert_eq!(saved["projects"]["YY"]["name"], "YY project");
}

#[test]
fn bad_arguments_exit_1_and_help_exits_0() {
    let env = Env::new();
    env.cmd().arg("create").assert().code(1);
    env.cmd().arg("frobnicate").assert().code(1);
    env.cmd().args(["list", "--no-such-flag"]).assert().code(1);
    env.cmd().arg("--help").assert().success();
    env.cmd().args(["create", "--help"]).assert().success();
}

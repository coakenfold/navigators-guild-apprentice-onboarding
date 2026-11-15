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
fn a_relative_tracker_home_is_refused() {
    let env = Env::without_store();
    let here = env.outside();

    for args in [&["init"][..], &["list"], &["project", "list"]] {
        let output = env
            .cmd()
            .env("TRACKER_HOME", "store")
            .args(args)
            .arg("--json")
            .assert()
            .code(1);
        let error = parse(&output.get_output().stdout);
        let message = error["error"].as_str().unwrap();
        assert!(message.contains("absolute path"), "{args:?}: {message}");
    }

    // Nothing was created relative to the current folder.
    assert!(!here.join("store").exists());
}

#[cfg(unix)]
#[test]
fn an_argument_that_isnt_utf8_exits_1_without_panicking() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let env = Env::new();
    env.add_project("XX", "xx");
    let title = OsStr::from_bytes(b"bad \xff title");

    env.cmd()
        .arg("create")
        .arg(title)
        .args(["--project", "XX"])
        .assert()
        .code(1);

    let output = env
        .cmd()
        .arg("create")
        .arg(title)
        .args(["--project", "XX", "--json"])
        .assert()
        .code(1);
    let error = parse(&output.get_output().stdout);
    assert_eq!(error["exit_code"], 1);
    assert!(!env.item_path("XX-1").exists());
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

#[cfg(unix)]
#[test]
fn an_unreadable_store_exits_3_and_isnt_reported_as_missing() {
    use std::os::unix::fs::PermissionsExt;

    let env = Env::new();
    let home = env.home.path();
    fs::set_permissions(home, fs::Permissions::from_mode(0o000)).unwrap();

    let output = env.cmd().args(["list", "--json"]).assert();
    // Put the permissions back before checking, so the folder can be cleaned up.
    fs::set_permissions(home, fs::Permissions::from_mode(0o755)).unwrap();

    let output = output.code(3);
    let error = parse(&output.get_output().stdout);
    let message = error["error"].as_str().unwrap();
    assert_eq!(error["exit_code"], 3);
    assert!(!message.contains("tracker init"), "{message}");
}

#[test]
fn a_reader_that_stops_early_doesnt_make_list_panic() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create(
        "XX",
        "a title long enough that a few thousand of them fill the pipe",
    );
    // Copy the one real item file, as `tracker list` would take minutes to
    // fill a pipe with items made one `create` at a time.
    let template = fs::read_to_string(env.item_path("XX-1")).unwrap();
    for n in 2..=3000 {
        let text = template.replace("\"XX-1\"", &format!("\"XX-{n}\""));
        fs::write(env.item_path(&format!("XX-{n}")), text).unwrap();
    }

    let mut child = Command::new(env!("CARGO_BIN_EXE_tracker"))
        .env("TRACKER_HOME", env.home.path())
        .current_dir(env.outside())
        .arg("list")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // Close the reading end without reading: the output is far bigger than
    // the pipe's buffer, so the write fails with a broken pipe.
    drop(child.stdout.take());
    let output = child.wait_with_output().unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("panicked"), "{stderr}");
    assert_eq!(output.status.code(), Some(0), "{stderr}");
}

#[test]
fn two_projects_cant_share_a_root() {
    let env = Env::new();
    let xx = env.add_project("XX", "xx");
    let root = xx.to_str().unwrap();

    let message = error_message(
        &env,
        &["project", "add", "YY", "--name", "y", "--root", root],
    );
    assert!(message.contains("XX"), "{message}");

    // The same folder by another spelling is still the same root.
    let other_spelling = format!("{root}/../xx");
    error_message(
        &env,
        &[
            "project",
            "add",
            "YY",
            "--name",
            "y",
            "--root",
            &other_spelling,
        ],
    );

    let projects = env.json(&["project", "list"]);
    assert_eq!(projects["projects"].as_array().unwrap().len(), 1);
    assert_eq!(env.json_in(&xx, &["create", "here"])["id"], "XX-1");
}

#[test]
fn a_lost_counter_doesnt_hand_out_an_id_already_on_disk() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.add_project("YY", "yy");
    env.create("XX", "a");
    env.create("XX", "b");
    env.create("YY", "other project");
    // XX-1 is deleted, then counters.json is lost and `init` recreates it.
    fs::remove_file(env.item_path("XX-1")).unwrap();
    fs::write(env.home.path().join("counters.json"), "{}\n").unwrap();

    // Without the counter, XX would start again at XX-1, the deleted ID.
    let output = env
        .cmd()
        .args(["create", "c", "--project", "XX", "--json"])
        .assert()
        .code(3);
    let message = parse(&output.get_output().stdout)["error"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(message.contains("XX-2"), "{message}");
    assert!(message.contains("at least 3"), "{message}");
    assert!(!env.item_path("XX-1").exists());

    // A counter at the highest number on disk is refused too.
    fs::write(env.home.path().join("counters.json"), "{\"XX\": 2}\n").unwrap();
    env.cmd()
        .args(["create", "c", "--project", "XX"])
        .assert()
        .code(3);

    // Set above it, IDs carry on. YY's counter is separate: YY-1 is on disk,
    // so YY is refused until its counter is set as well.
    fs::write(env.home.path().join("counters.json"), "{\"XX\": 3}\n").unwrap();
    assert_eq!(env.create("XX", "c"), "XX-3");
    env.cmd()
        .args(["create", "d", "--project", "YY"])
        .assert()
        .code(3);
}

#[test]
fn a_store_missing_a_part_says_run_tracker_init() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");

    // The items folder goes missing: `list` and `create` both say how to fix it.
    fs::remove_dir_all(env.home.path().join("items")).unwrap();
    for args in [&["list"][..], &["create", "b", "--project", "XX"]] {
        let message = error_message(&env, args);
        assert!(
            message.contains("Run `tracker init`"),
            "{args:?}: {message}"
        );
        assert!(message.contains("items"), "{args:?}: {message}");
    }
    env.json(&["init"]);
    assert_eq!(ids(&env.json(&["list"])), Vec::<&str>::new());

    // So does counters.json.
    fs::remove_file(env.home.path().join("counters.json")).unwrap();
    let message = error_message(&env, &["create", "b", "--project", "XX"]);
    assert!(message.contains("counters.json"), "{message}");
    env.json(&["init"]);
    assert_eq!(env.create("XX", "b"), "XX-1");
}

#[test]
fn titles_and_project_names_cant_hold_terminal_control_characters() {
    let env = Env::new();
    env.add_project("XX", "xx");

    // An escape sequence that clears the line and writes over it.
    let message = error_message(
        &env,
        &[
            "create",
            "evil\u{1b}[2K\u{1b}[1Gfake: all good\u{7}",
            "--project",
            "XX",
        ],
    );
    assert!(message.contains("U+001B"), "{message}");

    // Other control characters, and the marks that reorder text.
    for title in [
        "tab\there",
        "bell\u{7}",
        "c1\u{9b}31m",
        "rtl \u{202e}txt.exe",
        "lri \u{2066}x",
    ] {
        error_message(&env, &["create", title, "--project", "XX"]);
    }
    // Line breaks of every kind, not only \n and \r.
    for title in ["a\u{b}b", "a\u{c}b", "a\u{85}b", "a\u{2028}b", "a\u{2029}b"] {
        let message = error_message(&env, &["create", title, "--project", "XX"]);
        assert!(message.contains("one line"), "{message}");
    }
    assert!(!env.item_path("XX-1").exists());

    // Plain unicode is fine.
    assert_eq!(env.create("XX", "Ünïcode “quotes” — and emoji 😀"), "XX-1");

    // A project name is printed by `list` and `project list` too.
    let root = env.dir("yy");
    let message = error_message(
        &env,
        &[
            "project",
            "add",
            "YY",
            "--name",
            "y\u{1b}[31m",
            "--root",
            root.to_str().unwrap(),
        ],
    );
    assert!(message.contains("U+001B"), "{message}");
}

#[test]
fn a_bad_project_key_in_config_json_is_refused() {
    let env = Env::new();
    let root = env.dir("proj");
    let config_path = env.home.path().join("config.json");

    // `../../pwned` would put its item files two folders above `items/`.
    for key in ["../../pwned", "bm", "X-Y", ""] {
        let config = json!({ "projects": { key: { "name": "p", "root": root } } });
        fs::write(&config_path, config.to_string()).unwrap();

        for args in [&["list"][..], &["project", "list"]] {
            let output = env.cmd().args(args).arg("--json").assert().code(3);
            let message = parse(&output.get_output().stdout)["error"]
                .as_str()
                .unwrap()
                .to_string();
            assert!(
                message.contains("isn't valid"),
                "{key:?} {args:?}: {message}"
            );
        }
        env.cmd()
            .current_dir(&root)
            .args(["create", "escape"])
            .assert()
            .code(3);
    }

    // Nothing was written outside the store, or in it. `items/../../` is
    // the folder that holds the store.
    let above_the_store = env.home.path().parent().unwrap();
    assert!(!above_the_store.join("pwned-1.json").exists());
    assert_eq!(
        fs::read_dir(env.home.path().join("items")).unwrap().count(),
        0
    );
}

#[cfg(unix)]
#[test]
fn a_new_store_is_readable_only_by_its_owner() {
    use std::os::unix::fs::PermissionsExt;

    let mode = |path: &std::path::Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;

    // A store folder that `init` creates, along with a missing folder above it.
    let env = Env::without_store();
    let home = env.home.path().join("parent").join("store");
    let run = |home: &std::path::Path| {
        env.cmd()
            .env("TRACKER_HOME", home)
            .args(["init", "--json"])
            .assert()
            .success();
    };
    run(&home);
    assert_eq!(mode(&home), 0o700);
    assert_eq!(mode(home.parent().unwrap()), 0o700);

    // A folder that already exists keeps its permissions.
    let existing = env.home.path().join("shared");
    fs::create_dir(&existing).unwrap();
    fs::set_permissions(&existing, fs::Permissions::from_mode(0o755)).unwrap();
    run(&existing);
    assert_eq!(mode(&existing), 0o755);
}

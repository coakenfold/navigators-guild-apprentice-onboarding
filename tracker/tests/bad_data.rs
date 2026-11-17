//! Handling bad data (§4.4): a file that can't be parsed costs one item,
//! a missing store says "Run `tracker init`.", an unreadable store exits 3,
//! and no command leaves a temporary file behind.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::Env;
use serde_json::Value;

/// The IDs `list --json` listed.
fn ids(list: &Value) -> Vec<String> {
    list["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["id"].as_str().unwrap().to_string())
        .collect()
}

/// A store with XX-1, XX-2 and XX-3, where XX-2's file has lost the comma
/// after its title, as a hand edit might leave it. The parser stops at the
/// start of the next line, line 4.
fn store_with_a_corrupt_item() -> Env {
    let env = Env::new();
    env.add_project("XX", "xx");
    for title in ["one", "two", "three"] {
        env.create("XX", title);
    }
    let path = env.item_path("XX-2");
    let text = fs::read_to_string(&path).unwrap();
    assert_eq!(text.lines().nth(2), Some("  \"title\": \"two\","));
    fs::write(&path, text.replace("\"two\",", "\"two\"")).unwrap();
    env
}

#[test]
fn a_hand_corrupted_item_file_is_reported_and_the_others_still_list() {
    let env = store_with_a_corrupt_item();
    let before = fs::read_to_string(env.item_path("XX-2")).unwrap();

    // As JSON: the other items, and the file, line and reason.
    let list = env.json(&["list"]);
    assert_eq!(ids(&list), ["XX-1", "XX-3"]);
    let skipped = list["skipped"].as_array().unwrap();
    assert_eq!(skipped.len(), 1, "{list:#}");
    assert!(
        skipped[0]["file"].as_str().unwrap().ends_with("XX-2.json"),
        "{list:#}"
    );
    assert_eq!(skipped[0]["line"], 4);
    assert_eq!(skipped[0]["column"], 3);
    assert_eq!(skipped[0]["reason"], "expected `,` or `}`");

    // As text: the list on stdout, which says a file was left out, and the
    // file, line, reason and fix on stderr.
    let output = env.cmd().arg("list").assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.get_output().stderr.clone()).unwrap();
    assert!(
        stdout.contains("XX-1") && stdout.contains("XX-3"),
        "{stdout}"
    );
    assert!(!stdout.contains("XX-2"), "{stdout}");
    assert!(
        stdout.contains("1 item file can't be read as an item, so it isn't listed"),
        "{stdout}"
    );
    assert!(stderr.starts_with("warning: Skipped "), "{stderr}");
    assert!(
        stderr.contains("XX-2.json, line 4, column 3: expected `,` or `}`."),
        "{stderr}"
    );
    assert!(stderr.contains("fix it by hand"), "{stderr}");
    assert_eq!(stderr.lines().count(), 1, "{stderr}");

    // project list counts the others, and says what it skipped.
    let projects = env.json(&["project", "list"]);
    assert_eq!(projects["projects"][0]["counts"]["open"], 2);
    assert_eq!(projects["skipped"], list["skipped"]);

    // A command that names the item refuses, with the same details.
    for args in [&["show", "XX-2"][..], &["start", "XX-2"]] {
        let message = env.error(args, 3);
        assert!(
            message.contains("XX-2.json, line 4, column 3: expected `,` or `}`."),
            "{args:?}: {message}"
        );
        assert!(message.contains("fix it by hand"), "{args:?}: {message}");
    }
    // The other items still work, and the corrupt file is left as it was.
    env.json(&["start", "XX-1"]);
    assert_eq!(fs::read_to_string(env.item_path("XX-2")).unwrap(), before);
}

#[test]
fn a_file_with_an_unknown_value_is_skipped_with_the_parsers_reason() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");
    env.create("XX", "b");
    let path = env.item_path("XX-1");
    let text = fs::read_to_string(&path).unwrap();
    fs::write(&path, text.replace("\"open\"", "\"closed\"")).unwrap();

    let list = env.json(&["list"]);
    assert_eq!(ids(&list), ["XX-2"]);
    let reason = list["skipped"][0]["reason"].as_str().unwrap();
    assert!(reason.contains("unknown variant `closed`"), "{reason}");
    assert!(list["skipped"][0]["line"].is_u64(), "{list:#}");
}

#[test]
fn a_file_that_isnt_utf8_or_is_empty_or_is_a_folder_is_skipped() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");
    let items = env.home.path().join("items");
    fs::write(items.join("XX-2.json"), b"{\"id\": \"\xff\xfe\"}").unwrap();
    fs::write(items.join("XX-3.json"), "").unwrap();
    fs::create_dir(items.join("XX-4.json")).unwrap();

    let list = env.json(&["list"]);
    assert_eq!(ids(&list), ["XX-1"]);
    let reasons: Vec<&str> = list["skipped"]
        .as_array()
        .unwrap()
        .iter()
        .map(|file| file["reason"].as_str().unwrap())
        .collect();
    assert_eq!(reasons.len(), 3, "{list:#}");
    assert!(reasons[0].contains("UTF-8"), "{reasons:?}");
    assert!(reasons[1].contains("EOF"), "{reasons:?}");
    assert!(reasons[2].contains("folder"), "{reasons:?}");

    let message = env.error(&["show", "XX-2"], 3);
    assert!(message.contains("UTF-8"), "{message}");
}

#[cfg(unix)]
#[test]
fn a_symlink_to_a_missing_file_is_skipped_and_named_without_suggesting_init() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");
    env.create("XX", "b");
    let nowhere = env.work.path().join("nowhere.json");
    std::os::unix::fs::symlink(&nowhere, env.item_path("XX-7")).unwrap();

    // The other items still list, and the link is reported with its fix.
    let list = env.json(&["list"]);
    assert_eq!(ids(&list), ["XX-1", "XX-2"]);
    let skipped = list["skipped"].as_array().unwrap();
    assert_eq!(skipped.len(), 1, "{list:#}");
    assert!(
        skipped[0]["file"].as_str().unwrap().ends_with("XX-7.json"),
        "{list:#}"
    );
    assert_eq!(
        skipped[0]["reason"],
        "it's a symlink to a file that doesn't exist"
    );
    let projects = env.json(&["project", "list"]);
    assert_eq!(projects["skipped"], list["skipped"]);

    let output = env.cmd().arg("list").assert().success();
    let stderr = String::from_utf8(output.get_output().stderr.clone()).unwrap();
    assert!(stderr.contains("XX-7.json: it's a symlink"), "{stderr}");
    assert!(stderr.contains("Remove it from items/."), "{stderr}");
    assert!(!stderr.contains("tracker init"), "{stderr}");

    // A command that names it says what it is, not that there's no item,
    // and not that `init` would help: `init` doesn't create items.
    for args in [
        &["show", "XX-7"][..],
        &["start", "XX-7"],
        &["delete", "XX-7"],
    ] {
        let message = env.error(args, 3);
        assert!(
            message.contains("symlink to a file that doesn't exist"),
            "{args:?}: {message}"
        );
        assert!(
            message.contains("Remove it from items/."),
            "{args:?}: {message}"
        );
        assert!(!message.contains("tracker init"), "{args:?}: {message}");
    }
    // Nothing was written through it.
    assert!(!nowhere.exists());
}

/// One run of every command, for the tests that need each of them.
fn every_command(root: &Path) -> Vec<Vec<String>> {
    let root = root.to_str().unwrap();
    [
        &["project", "add", "XX", "--name", "x", "--root", root][..],
        &["project", "list"],
        &["create", "t", "--project", "XX"],
        &["edit", "XX-1", "--priority", "high"],
        &["list"],
        &["start", "XX-1"],
        &["submit", "XX-1"],
        &["shelve", "XX-1", "--reason", "later"],
        &["reopen", "XX-1", "--reason", "back"],
        &["status", "XX-1", "open", "--reason", "back"],
        &["show", "XX-1"],
        &["delete", "XX-1"],
    ]
    .iter()
    .map(|args| args.iter().map(|arg| arg.to_string()).collect())
    .collect()
}

#[test]
fn without_a_store_every_command_says_run_tracker_init() {
    let env = Env::without_store();
    let root = env.dir("xx");
    for args in every_command(&root) {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let message = env.error(&args, 1);
        assert!(
            message.contains("Run `tracker init`."),
            "{args:?}: {message}"
        );

        let output = env.cmd().args(&args).assert().code(1);
        let stderr = String::from_utf8(output.get_output().stderr.clone()).unwrap();
        assert!(stderr.contains("Run `tracker init`."), "{args:?}: {stderr}");
        assert!(!stderr.contains("panicked"), "{args:?}: {stderr}");
    }
    // Nothing was created.
    assert_eq!(fs::read_dir(env.home.path()).unwrap().count(), 0);
}

#[test]
fn a_store_missing_only_config_json_says_to_put_it_back() {
    let env = Env::new();
    fs::remove_file(env.home.path().join("config.json")).unwrap();

    let message = env.error(&["list"], 1);
    assert!(message.contains("missing"), "{message}");
    assert!(message.contains("config.json"), "{message}");
    assert!(message.contains("Run `tracker init`"), "{message}");
}

/// Sets a folder's permissions, and puts them back to 0700 when dropped,
/// even if the test fails, so the temporary folder can be removed.
#[cfg(unix)]
struct Mode(PathBuf);

#[cfg(unix)]
impl Mode {
    fn set(path: &Path, mode: u32) -> Mode {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
        Mode(path.to_path_buf())
    }
}

#[cfg(unix)]
impl Drop for Mode {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o700));
    }
}

/// Whether permissions stop this process, which they don't for root.
#[cfg(unix)]
fn permissions_apply(locked: &Path) -> bool {
    if fs::read_dir(locked).is_ok() {
        eprintln!("Skipped: permissions don't stop this user, probably root.");
        return false;
    }
    true
}

#[cfg(unix)]
#[test]
fn a_store_with_chmod_000_exits_3() {
    let env = Env::new();
    let root = env.add_project("XX", "xx");
    env.create("XX", "a");
    let _mode = Mode::set(env.home.path(), 0o000);
    if !permissions_apply(env.home.path()) {
        return;
    }

    let mut commands = every_command(&root);
    commands.insert(0, vec!["init".to_string()]);
    for args in commands {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let message = env.error(&args, 3);
        assert!(message.contains("ls -ld"), "{args:?}: {message}");

        let output = env.cmd().args(&args).assert().code(3);
        let stderr = String::from_utf8(output.get_output().stderr.clone()).unwrap();
        assert!(!stderr.contains("panicked"), "{args:?}: {stderr}");
    }
}

#[cfg(unix)]
#[test]
fn an_items_folder_with_chmod_000_exits_3() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");
    let items = env.home.path().join("items");
    let _mode = Mode::set(&items, 0o000);
    if !permissions_apply(&items) {
        return;
    }

    for args in [
        &["list"][..],
        &["project", "list"],
        &["create", "b", "--project", "XX"],
        &["show", "XX-1"],
        &["start", "XX-1"],
    ] {
        let message = env.error(args, 3);
        assert!(message.contains("Permission denied"), "{args:?}: {message}");
        assert!(message.contains("ls -ld"), "{args:?}: {message}");
    }
}

/// Every file in the store whose name says it's a temporary file, such as
/// `XX-1.json.4242.tmp`, in any folder.
fn temporary_files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            found.extend(temporary_files(&path));
        } else if path.to_string_lossy().ends_with(".tmp") {
            found.push(path);
        }
    }
    found
}

#[test]
fn no_temporary_files_are_left_in_the_store_after_any_command() {
    let env = Env::without_store();
    let root = env.dir("xx");
    let check = |what: &str| {
        let left = temporary_files(env.home.path());
        assert!(left.is_empty(), "after {what}: {left:?}");
    };
    let run = |args: &[&str]| {
        // Whatever the outcome, success or a refusal.
        env.cmd().args(args).output().unwrap();
        check(&args.join(" "));
    };

    run(&["init"]);
    for args in every_command(&root) {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        run(&args);
    }
    for args in [
        &["create", "second", "--project", "XX"][..],
        &["create", "", "--project", "XX"],
        &["edit", "XX-2", "--label", "bug", "--title", "renamed"],
        &["start", "XX-2"],
        &["start", "XX-2"],
        &["status", "XX-2", "done"],
        &["reopen", "XX-2", "--reason", "again"],
        &["shelve", "XX-2"],
        &["edit", "XX-9", "--priority", "low"],
        &["init"],
        &["list", "--status", "done"],
    ] {
        run(args);
    }

    // A delete typed at a terminal writes deleted.json and removes a file.
    if let Some((output, ok)) =
        env.in_terminal_typing(&["delete", "XX-2"], "to confirm", "XX-2", || {})
    {
        assert!(ok, "{output}");
        check("delete in a terminal");
    }

    // A write that fails takes its temporary file away with it.
    #[cfg(unix)]
    {
        let items = env.home.path().join("items");
        let _mode = Mode::set(&items, 0o500);
        if permissions_apply_to_writes(&items) {
            env.cmd()
                .args(["create", "third", "--project", "XX"])
                .assert()
                .code(3);
            check("a create that can't write");
        }
    }
}

/// Whether a folder without write permission stops this process.
#[cfg(unix)]
fn permissions_apply_to_writes(dir: &Path) -> bool {
    let probe = dir.join("probe");
    if fs::write(&probe, "").is_ok() {
        let _ = fs::remove_file(probe);
        eprintln!("Skipped: permissions don't stop this user, probably root.");
        return false;
    }
    true
}

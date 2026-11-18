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
    // The news is in `skipped`, and stderr stays empty, so an agent that
    // reads stdout and stderr together still gets one JSON object.
    for args in [&["list", "--json"][..], &["project", "list", "--json"]] {
        let output = env.cmd().args(args).assert().success();
        assert!(
            output.get_output().stderr.is_empty(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.get_output().stderr)
        );
    }

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
    env.json(&["edit", "XX-1", "--priority", "high"]);
    // Except for starting one in XX: the file may be an item in progress,
    // so G1 fails until it's fixed (slice 6 review 1's finding 5).
    let error = env.error_json_in(&env.outside(), &["start", "XX-1"], 2);
    let message = error["failures"][0]["message"].as_str().unwrap();
    assert!(
        message.contains("Can't tell whether XX-2 is in progress"),
        "{message}"
    );
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

/// Runs `tracker <args> --json`, stopped after 10 seconds so a command
/// that hangs fails the test rather than the suite, expects exit `code`,
/// and returns the JSON.
#[cfg(unix)]
fn json_within_10s(env: &Env, args: &[&str], code: i32) -> Value {
    let output = env
        .cmd()
        .args(args)
        .arg("--json")
        .timeout(std::time::Duration::from_secs(10))
        .assert()
        .code(code);
    let value = common::parse(&output.get_output().stdout);
    let stderr = String::from_utf8(output.get_output().stderr.clone()).unwrap();
    assert!(stderr.is_empty(), "{args:?}: {stderr}");
    value
}

// Review 2's bug 1 and finding 4: anything in items/ that isn't a regular
// file once symlinks are followed costs one item, like a file that doesn't
// parse. Before, a symlink loop stopped the whole list, and a pipe made
// every command that read it hang, a writing one while holding the lock.
#[cfg(unix)]
#[test]
fn an_entry_that_isnt_a_regular_file_is_skipped_and_never_read() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");
    env.create("XX", "b");
    std::os::unix::fs::symlink("XX-5.json", env.item_path("XX-5")).unwrap();
    std::os::unix::fs::symlink("XX-7.json", env.item_path("XX-6")).unwrap();
    std::os::unix::fs::symlink("XX-6.json", env.item_path("XX-7")).unwrap();
    std::os::unix::fs::symlink("/dev/null", env.item_path("XX-8")).unwrap();
    let made = std::process::Command::new("mkfifo")
        .arg(env.item_path("XX-9"))
        .status()
        .unwrap();
    assert!(made.success());

    for args in [&["list"][..], &["project", "list"]] {
        let listed = json_within_10s(&env, args, 0);
        let skipped: Vec<(String, String)> = listed["skipped"]
            .as_array()
            .unwrap()
            .iter()
            .map(|file| {
                let name = Path::new(file["file"].as_str().unwrap())
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                (name, file["reason"].as_str().unwrap().to_string())
            })
            .collect();
        let names: Vec<&str> = skipped.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(
            names,
            [
                "XX-5.json",
                "XX-6.json",
                "XX-7.json",
                "XX-8.json",
                "XX-9.json"
            ],
            "{args:?}: {listed:#}"
        );
        for (name, reason) in &skipped[..3] {
            assert!(
                reason.starts_with("it's a symlink that can't be followed"),
                "{name}: {reason}"
            );
        }
        for (name, reason) in &skipped[3..] {
            assert_eq!(reason, "it isn't a regular file", "{name}");
        }
    }
    let list = json_within_10s(&env, &["list"], 0);
    assert_eq!(ids(&list), ["XX-1", "XX-2"]);

    // A command that names one refuses with the same reason and the fix,
    // and one that writes, such as start, doesn't hang holding the lock.
    for id in ["XX-5", "XX-8", "XX-9"] {
        for command in ["show", "start", "edit", "delete"] {
            let mut args = vec![command, id];
            if command == "edit" {
                args.extend(["--priority", "low"]);
            }
            let error = json_within_10s(&env, &args, 3);
            let message = error["error"].as_str().unwrap();
            assert!(
                message.contains("symlink that can't be followed")
                    || message.contains("isn't a regular file"),
                "{args:?}: {message}"
            );
            assert!(
                message.contains("Remove it from items/.")
                    || message.contains("Move it out of items/."),
                "{args:?}: {message}"
            );
        }
    }
    // The other items still work.
    json_within_10s(&env, &["edit", "XX-1", "--priority", "low"], 0);
    // G1 names each of them, since any could be an item in progress, and
    // doesn't open one to find out (slice 6 review 1's finding 5).
    let error = json_within_10s(&env, &["start", "XX-1"], 2);
    let failures = error["failures"].as_array().unwrap();
    assert_eq!(failures.len(), 5, "{error:#}");
    for (failure, id) in failures
        .iter()
        .zip(["XX-5", "XX-6", "XX-7", "XX-8", "XX-9"])
    {
        let message = failure["message"].as_str().unwrap();
        assert!(
            message.contains(&format!("Can't tell whether {id} is in progress")),
            "{message}"
        );
    }
}

/// Runs `tracker <args> --json` with `TRACKER_HOME` set to `home`, or
/// unset for `None`, expects exit 3, and returns the error message.
fn store_error(env: &Env, home: Option<&Path>, args: &[&str]) -> String {
    let mut cmd = env.cmd();
    match home {
        Some(home) => cmd.env("TRACKER_HOME", home),
        None => cmd.env_remove("TRACKER_HOME"),
    };
    let output = cmd.args(args).arg("--json").assert().code(3);
    let error = common::parse(&output.get_output().stdout);
    common::check_schema("error", &error);
    error["error"].as_str().unwrap().to_string()
}

#[test]
fn a_store_folder_that_is_a_file_is_named_with_how_to_fix_it() {
    let env = Env::without_store();
    let file = env.work.path().join("f");
    fs::write(&file, "not a store").unwrap();
    let root = env.dir("xx");

    // TRACKER_HOME naming a file, and naming a folder inside a file.
    let mut commands = every_command(&root);
    commands.insert(0, vec!["init".to_string()]);
    for args in commands {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let message = store_error(&env, Some(&file), &args);
        assert_eq!(
            message,
            format!(
                "TRACKER_HOME names {}, which is a file, not a folder, so it can't hold the \
                 store. Point TRACKER_HOME at a folder, or move the file.",
                file.display()
            ),
            "{args:?}"
        );
    }
    let inside = file.join("store");
    let message = store_error(&env, Some(&inside), &["init"]);
    assert_eq!(
        message,
        format!(
            "TRACKER_HOME names {}, but {} is a file, not a folder, so it can't hold the store. \
             Point TRACKER_HOME at a folder, or move the file.",
            inside.display(),
            file.display()
        )
    );

    // With no TRACKER_HOME, ~/.tracker being a file says the same.
    let default = env.user_home().join(".tracker");
    fs::write(&default, "").unwrap();
    let message = store_error(&env, None, &["list"]);
    assert!(
        message.starts_with(&format!(
            "TRACKER_HOME isn't set, so the store is {}, which is a file",
            default.display()
        )),
        "{message}"
    );
    assert!(
        message.ends_with("Set TRACKER_HOME to a folder, or move the file."),
        "{message}"
    );

    // The file is left as it was.
    assert_eq!(fs::read_to_string(&file).unwrap(), "not a store");
}

// Review 2's bug 2: before, `list` said "Run `tracker init`" and `init`
// then failed with "File exists", with nothing saying the store's folder
// was a symlink to nowhere.
#[cfg(unix)]
#[test]
fn a_store_folder_that_is_a_symlink_to_nothing_is_named_with_how_to_fix_it() {
    let env = Env::without_store();
    let root = env.dir("xx");
    let nowhere = env.work.path().join("nowhere");
    let link = env.work.path().join("store");
    std::os::unix::fs::symlink(&nowhere, &link).unwrap();

    let mut commands = every_command(&root);
    commands.insert(0, vec!["init".to_string()]);
    for args in commands {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let message = store_error(&env, Some(&link), &args);
        assert_eq!(
            message,
            format!(
                "TRACKER_HOME names {}, a symlink to {}, which doesn't exist, so it can't hold \
                 the store. Point TRACKER_HOME at a folder, or create {}.",
                link.display(),
                nowhere.display(),
                nowhere.display()
            ),
            "{args:?}"
        );
    }
    // A link above the store's folder, and a relative link, which is
    // followed from the folder it's in.
    std::os::unix::fs::symlink("nowhere", env.work.path().join("up")).unwrap();
    let inside = env.work.path().join("up").join("store");
    let message = store_error(&env, Some(&inside), &["init"]);
    assert_eq!(
        message,
        format!(
            "TRACKER_HOME names {}, but {} is a symlink to {}, which doesn't exist, so it can't \
             hold the store. Point TRACKER_HOME at a folder, or create {}.",
            inside.display(),
            env.work.path().join("up").display(),
            nowhere.display(),
            nowhere.display()
        )
    );

    // With no TRACKER_HOME, ~/.tracker linking to nothing says the same.
    let default = env.user_home().join(".tracker");
    std::os::unix::fs::symlink(&nowhere, &default).unwrap();
    let message = store_error(&env, None, &["list"]);
    assert!(
        message.starts_with(&format!(
            "TRACKER_HOME isn't set, so the store is {}, a symlink to {}",
            default.display(),
            nowhere.display()
        )),
        "{message}"
    );
    assert!(
        message.ends_with(&format!(
            "Set TRACKER_HOME to a folder, or create {}.",
            nowhere.display()
        )),
        "{message}"
    );

    // Nothing was created where the links point.
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
        &["gates", "XX-1"],
        &["criterion", "add", "XX-1", "works", "--check", "true"],
        &["criterion", "set", "XX-1", "1", "--check", "true"],
        &["evidence", "XX-1", "1", "here"],
        &["criterion", "remove", "XX-1", "1"],
        &["criterion", "accept", "XX-1"],
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

/// The commands in backticks in `message` that start with `ls -ld` or
/// `chmod`, as the message suggests running them.
#[cfg(unix)]
fn suggested_commands(message: &str) -> Vec<String> {
    message
        .split('`')
        .skip(1)
        .step_by(2)
        .filter(|part| part.starts_with("ls -ld ") || part.starts_with("chmod "))
        .map(str::to_string)
        .collect()
}

// Review 2's bug 3: a path with a space, or a quote, in it is quoted in
// the commands the permission messages suggest, so they work pasted.
#[cfg(unix)]
#[test]
fn the_commands_a_permission_message_suggests_work_when_pasted() {
    let env = Env::without_store();
    let home = env.work.path().join("it's my store");
    let run = |args: &[&str]| {
        env.cmd()
            .env("TRACKER_HOME", &home)
            .args(args)
            .arg("--json")
            .output()
            .unwrap()
    };
    assert!(run(&["init"]).status.success());
    let items = home.join("items");

    for (locked, args) in [(&home, &["list"][..]), (&items, &["show", "XX-1"])] {
        let _mode = Mode::set(locked, 0o000);
        if !permissions_apply(locked) {
            return;
        }
        let output = run(args);
        assert_eq!(output.status.code(), Some(3), "{args:?}");
        let error = common::parse(&output.stdout);
        let message = error["error"].as_str().unwrap();
        let commands = suggested_commands(message);
        assert!(!commands.is_empty(), "{args:?}: {message}");
        for command in commands {
            let ran = std::process::Command::new("sh")
                .args(["-c", &command])
                .output()
                .unwrap();
            assert!(
                ran.status.success(),
                "{args:?}: `{command}` failed: {}",
                String::from_utf8_lossy(&ran.stderr)
            );
        }
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

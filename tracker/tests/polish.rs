//! Slice 5's polish: `--help` on every command, colour only on a terminal,
//! empty states, `list`'s heading row, the longest title, and error
//! messages that say how to fix the problem.

mod common;

use std::path::Path;

use common::Env;
use unicode_width::UnicodeWidthStr;

const ESC: char = '\u{1b}';

/// Every command, as typed after `tracker`, with its first line of help.
const COMMANDS: [(&[&str], &str); 15] = [
    (&[], "A verification-gated work tracker"),
    (&["init"], "Create the store"),
    (&["project"], "Register projects and list them"),
    (&["project", "add"], "Register a project"),
    (
        &["project", "list"],
        "List projects with item counts by state",
    ),
    (&["create"], "Create an open item"),
    (&["edit"], "Change an item's details"),
    (&["list"], "List items, grouped by project"),
    (&["start"], "Start work on an open item"),
    (&["submit"], "Submit an in-progress item"),
    (
        &["shelve"],
        "Put an open or in-progress item back in the backlog",
    ),
    (&["reopen"], "Bring a done or shelved item back"),
    (&["status"], "Move an item to a state"),
    (&["show"], "Show everything about one item"),
    (&["delete"], "Delete an item for good"),
];

#[test]
fn every_command_has_help_text() {
    let env = Env::without_store();
    for (command, about) in COMMANDS {
        for flag in ["--help", "-h"] {
            let output = env.cmd().args(command).arg(flag).assert().success();
            let help = String::from_utf8(output.get_output().stdout.clone()).unwrap();
            let what = format!("tracker {} {flag}", command.join(" "));
            assert!(help.starts_with(about), "{what}:\n{help}");
            assert!(help.contains("Usage: tracker"), "{what}:\n{help}");
            assert!(help.contains("--json"), "{what}:\n{help}");
            // Every option and argument is described: on the same line,
            // after two spaces, or, under `--help` for a command with long
            // help, on the next line, indented further.
            let lines: Vec<&str> = help.lines().collect();
            for (i, line) in lines.iter().enumerate() {
                let text = line.trim_start();
                if !text.starts_with('-') && !text.starts_with('<') {
                    continue;
                }
                let beside = text
                    .trim_end()
                    .split("  ")
                    .filter(|part| !part.trim().is_empty())
                    .count()
                    > 1;
                let indent = line.len() - text.len();
                let below = flag == "--help"
                    && lines.get(i + 1).is_some_and(|next| {
                        let next_text = next.trim_start();
                        !next_text.is_empty() && next.len() - next_text.len() > indent
                    });
                let described = beside || below;
                assert!(described, "{what}: {line}");
            }
        }
    }
    // The top level says what the exit codes mean.
    let output = env.cmd().arg("--help").assert().success();
    let help = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(help.contains("Exit codes: 0 success"), "{help}");
}

/// A store with an item in every state, in two projects, with labels.
fn busy_store() -> Env {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.add_project("YY", "yy");
    for title in ["open one", "started", "finished", "put away"] {
        env.json(&[
            "create",
            title,
            "--project",
            "XX",
            "--priority",
            "high",
            "--label",
            "bug",
        ]);
    }
    env.create("YY", "other project");
    env.json(&["start", "XX-2"]);
    env.json(&["start", "XX-3"]);
    env.json(&["submit", "XX-3"]);
    env.json(&["shelve", "XX-4", "--reason", "later"]);
    env
}

/// What `tracker <args>` prints, stdout then stderr, with `vars` set.
fn printed(env: &Env, args: &[&str], vars: &[(&str, &str)]) -> (String, String) {
    let mut cmd = env.cmd();
    for (name, value) in vars {
        cmd.env(name, value);
    }
    let output = cmd.args(args).output().unwrap();
    (
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

/// Commands whose output has colour on a terminal: tables, headings,
/// states, a move, and an error.
const COLOURED: [&[&str]; 5] = [
    &["list", "--all", "--status", "open,in-progress,done,shelved"],
    &["project", "list"],
    &["show", "XX-3"],
    &["start", "XX-1"],
    &["show", "XX-99"],
];

#[test]
fn output_that_isnt_a_terminal_has_no_colour_codes() {
    let env = busy_store();
    for args in COLOURED {
        let (stdout, stderr) = printed(&env, args, &[]);
        assert!(!stdout.contains(ESC), "{args:?}: {stdout:?}");
        assert!(!stderr.contains(ESC), "{args:?}: {stderr:?}");
    }

    // Literally `tracker list | cat`, through a shell.
    let output = std::process::Command::new("sh")
        .args(["-c", "\"$TRACKER\" list --all | cat"])
        .env("TRACKER", env!("CARGO_BIN_EXE_tracker"))
        .env("TRACKER_HOME", env.home.path())
        .env_remove("CLICOLOR_FORCE")
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("XX-1"), "{stdout}");
    assert!(!stdout.contains(ESC), "{stdout:?}");
}

#[test]
fn colour_is_there_when_forced_and_turns_off_with_no_color() {
    let env = busy_store();
    for args in COLOURED {
        // CLICOLOR_FORCE stands in for a terminal: anstream decides the same way.
        let (stdout, stderr) = printed(&env, args, &[("CLICOLOR_FORCE", "1")]);
        let all = format!("{stdout}{stderr}");
        assert!(all.contains(&format!("{ESC}[")), "{args:?}: {all:?}");
    }
    // The heading row is bold, and an in-progress state cyan.
    let (stdout, _) = printed(&env, &["list"], &[("CLICOLOR_FORCE", "1")]);
    assert!(stdout.starts_with(&format!("{ESC}[1m  ID")), "{stdout:?}");
    assert!(
        stdout.contains(&format!("{ESC}[36min-progress")),
        "{stdout:?}"
    );
    // Without the codes, the text is exactly what a pipe gets.
    let (plain, _) = printed(&env, &["list"], &[]);
    assert_eq!(strip_colour(&stdout), plain);

    let (stdout, _) = printed(&env, &["list"], &[("NO_COLOR", "1")]);
    assert!(!stdout.contains(ESC), "{stdout:?}");
}

#[test]
fn colour_doesnt_stop_a_title_edited_by_hand_being_escaped() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "plain");
    let path = env.item_path("XX-1");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replace("\"plain\"", "\"evil\\u001b[31mred\"")).unwrap();

    let (stdout, _) = printed(&env, &["list"], &[("CLICOLOR_FORCE", "1")]);
    assert!(stdout.contains("evil\\u{1b}[31mred"), "{stdout:?}");
    assert!(!stdout.contains(&format!("{ESC}[31mred")), "{stdout:?}");
}

#[test]
fn a_terminal_gets_colour() {
    let env = busy_store();
    let Some(output) = env.in_terminal(&["list"]) else {
        return;
    };
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains(&format!("{ESC}[1m")), "{text:?}");
}

/// Removes every `ESC [ … m` colour code.
fn strip_colour(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == ESC {
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn list_text(env: &Env, dir: &Path, args: &[&str]) -> String {
    let output = env
        .cmd()
        .current_dir(dir)
        .arg("list")
        .args(args)
        .assert()
        .success();
    String::from_utf8(output.get_output().stdout.clone()).unwrap()
}

#[test]
fn an_empty_list_prints_its_empty_state() {
    let env = Env::new();
    let outside = env.outside();
    let first = |text: &str| text.lines().next().unwrap_or_default().to_string();

    // No projects.
    let text = list_text(&env, &outside, &[]);
    assert!(first(&text).starts_with("No projects yet."), "{text}");
    assert!(text.contains("tracker project add"), "{text}");

    // A project with no items, from outside it and from inside it.
    let xx = env.add_project("XX", "xx");
    let text = list_text(&env, &outside, &[]);
    assert!(first(&text).starts_with("No items yet."), "{text}");
    let text = list_text(&env, &xx, &[]);
    assert!(first(&text).starts_with("No XX items yet."), "{text}");
    assert!(
        text.contains("tracker create \"title\" --project XX"),
        "{text}"
    );

    // Items, but none open, in progress or in review.
    env.create("XX", "a");
    env.json(&["shelve", "XX-1", "--reason", "later"]);
    let text = list_text(&env, &xx, &[]);
    assert_eq!(first(&text), "No open items. Nice work!", "{text}");

    // Filters that match nothing.
    let text = list_text(&env, &xx, &["--status", "shelved", "--priority", "high"]);
    assert_eq!(first(&text), "No items match these filters.", "{text}");
    assert!(
        text.contains("Filters: status shelved, priority high."),
        "{text}"
    );

    // Each empty state is the only thing before the footer, with no
    // heading row, and the JSON has no items.
    assert!(!text.contains("PRIORITY"), "{text}");
    assert_eq!(env.json_in(&xx, &["list"])["items"], serde_json::json!([]));
}

#[test]
fn list_starts_with_a_heading_row_lined_up_with_the_rows_below_it() {
    let env = busy_store();
    env.json(&[
        "create",
        "中文标题 and 🎉",
        "--project",
        "YY",
        "--label",
        "wide",
    ]);
    let text = list_text(&env, &env.outside(), &["--status", "open,in-progress,done"]);
    let lines: Vec<&str> = text.lines().collect();
    let heading = lines[0];
    let names = ["ID", "PRIORITY", "KIND", "STATUS", "TITLE", "LABELS"];
    let words: Vec<&str> = heading.split_whitespace().collect();
    assert_eq!(words, names, "{text}");

    // The terminal column each name starts at.
    let column = |line: &str, at: usize| line[..at].width();
    let starts: Vec<usize> = names
        .iter()
        .map(|name| column(heading, heading.find(name).unwrap()))
        .collect();

    let list = env.json(&["list", "--status", "open,in-progress,done"]);
    let mut checked = 0;
    for item in list["items"].as_array().unwrap() {
        let id = item["id"].as_str().unwrap();
        let line = lines
            .iter()
            .find(|line| line.trim_start().starts_with(&format!("{id} ")))
            .unwrap();
        let mut from = 0;
        for (i, field) in ["id", "priority", "kind", "status", "title"]
            .iter()
            .enumerate()
        {
            let value = item[field].as_str().unwrap();
            let at = from + line[from..].find(value).unwrap();
            assert_eq!(column(line, at), starts[i], "{field} of {id}:\n{text}");
            from = at + value.len();
        }
        if let Some(at) = line.find('[') {
            assert_eq!(column(line, at), starts[5], "labels of {id}:\n{text}");
        }
        checked += 1;
    }
    assert_eq!(checked, 5, "{text}");
}

#[test]
fn a_title_can_be_at_most_200_characters() {
    let env = Env::new();
    env.add_project("XX", "xx");
    let longest = "标".repeat(200);
    let created = env.json(&["create", &longest, "--project", "XX"]);
    assert_eq!(created["item"]["title"], longest.as_str());

    let too_long = "a".repeat(201);
    for args in [
        &["create", &too_long, "--project", "XX"][..],
        &["edit", "XX-1", "--title", &too_long],
    ] {
        let message = env.error(args, 1);
        assert!(message.contains("at most 200 characters"), "{message}");
        assert!(message.contains("201"), "{message}");
        assert!(message.contains("--description"), "{message}");
    }
    assert_eq!(env.item("XX-1")["title"], longest.as_str());
}

#[test]
fn common_errors_say_how_to_fix_the_problem() {
    let env = Env::new();
    env.add_project("XX", "xx");

    // An ID that names no item, from every command that takes one.
    for args in [
        &["show", "XX-9"][..],
        &["edit", "XX-9", "--priority", "low"],
        &["start", "XX-9"],
        &["submit", "XX-9"],
        &["shelve", "XX-9", "--reason", "later"],
        &["reopen", "XX-9", "--reason", "back"],
        &["status", "XX-9", "open", "--reason", "back"],
        &["delete", "XX-9"],
    ] {
        assert_eq!(
            env.error(args, 1),
            "There's no item XX-9. `tracker list --all --status \
             open,in-progress,review,done,shelved` lists every item.",
            "{args:?}"
        );
    }

    let root = env.dir("yy");
    let root = root.to_str().unwrap();
    assert_eq!(
        env.error(&["project", "add", "YY", "--name", " ", "--root", root], 1),
        "A project's --name can't be empty. Give it a readable name, such as --name bookmarks."
    );

    let nope = env.work.path().join("nope");
    let message = env.error(
        &[
            "project",
            "add",
            "YY",
            "--name",
            "y",
            "--root",
            nope.to_str().unwrap(),
        ],
        1,
    );
    assert!(
        message.starts_with(&format!("Can't use {} as a project root: ", nope.display())),
        "{message}"
    );
    assert!(
        message.ends_with(". Check the path, or create the folder first."),
        "{message}"
    );

    let output = env
        .cmd()
        .env_remove("TRACKER_HOME")
        .env_remove("HOME")
        .args(["list", "--json"])
        .assert()
        .code(3);
    let error = common::parse(&output.get_output().stdout);
    assert_eq!(
        error["error"],
        "Neither TRACKER_HOME nor HOME is set, so there's no store to use. Set TRACKER_HOME to \
         the store's folder, an absolute path such as /Users/me/.tracker."
    );
}

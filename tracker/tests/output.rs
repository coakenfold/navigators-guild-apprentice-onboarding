//! Nothing the tool prints can change what the terminal shows: text from
//! files edited by hand, folder names and repeated input are all escaped,
//! as text and as JSON.

mod common;

use std::fs;

use common::{Env, check_schema, parse};
use serde_json::json;
use tracker::text;

const ESC: char = '\u{1b}';
const RLO: char = '\u{202e}';

fn has_raw(output: &[u8], c: char) -> bool {
    String::from_utf8_lossy(output).contains(c)
}

#[test]
fn a_title_edited_by_hand_is_escaped_in_text_and_json() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "plain");
    let title = format!("evil{ESC}[2K{ESC}[1Gfake{RLO}txt\u{85}c1\nline");
    let path = env.item_path("XX-1");
    let mut file: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    file["title"] = json!(title);
    fs::write(&path, serde_json::to_string_pretty(&file).unwrap()).unwrap();

    let output = env.cmd().arg("list").assert().success();
    let stdout = &output.get_output().stdout;
    for c in [ESC, RLO, '\u{85}'] {
        assert!(!has_raw(stdout, c), "U+{:04X} printed as is", c as u32);
    }
    let stdout = String::from_utf8_lossy(stdout);
    assert!(stdout.contains("evil\\u{1b}[2K"), "{stdout}");
    // A line break in the title can't start a line of its own.
    assert!(stdout.contains("txt\\u{85}c1\\u{a}line"), "{stdout}");
    assert!(
        !stdout.lines().any(|line| line.starts_with("line")),
        "{stdout}"
    );
    assert_eq!(
        stdout.lines().filter(|line| line.contains("evil")).count(),
        1,
        "{stdout}"
    );

    // The JSON escapes it too, and still parses to the same title.
    let output = env.cmd().args(["list", "--json"]).assert().success();
    let stdout = &output.get_output().stdout;
    for c in [ESC, RLO, '\u{85}'] {
        assert!(!has_raw(stdout, c), "U+{:04X} in the JSON as is", c as u32);
    }
    assert_eq!(parse(stdout)["items"][0]["title"], title.as_str());
}

#[test]
fn names_and_roots_written_by_hand_are_escaped() {
    let env = Env::new();
    let root = env.dir("proj");
    let config = json!({ "projects": { "PK": {
        "name": format!("n{ESC}[31m"),
        "root": format!("{}{ESC}[2J", root.display()),
    } } });
    fs::write(env.home.path().join("config.json"), config.to_string()).unwrap();
    // An item in PK, so `list` prints PK's name too.
    let item = json!({
        "id": "PK-1", "title": "t", "project": "PK", "kind": "change",
        "status": "open", "priority": "medium",
        "created": "2026-11-15T22:01:56Z", "updated": "2026-11-15T22:01:56Z"
    });
    fs::write(env.item_path("PK-1"), item.to_string()).unwrap();

    for args in [&["project", "list"][..], &["list"]] {
        let output = env.cmd().args(args).assert().success();
        let stdout = &output.get_output().stdout;
        assert!(!has_raw(stdout, ESC), "{args:?}");
        assert!(
            String::from_utf8_lossy(stdout).contains("n\\u{1b}[31m"),
            "{args:?}"
        );
    }
}

#[cfg(unix)]
#[test]
fn a_root_whose_path_holds_a_control_character_is_refused() {
    let env = Env::new();
    let root = env.dir(&format!("r{ESC}[31mRED"));

    let message = env.error(
        &[
            "project",
            "add",
            "RR",
            "--name",
            "r",
            "--root",
            root.to_str().unwrap(),
        ],
        1,
    );
    assert!(message.contains("U+001B"), "{message}");
    assert_eq!(env.json(&["project", "list"])["projects"], json!([]));
}

#[test]
fn errors_that_repeat_input_escape_it() {
    let env = Env::new();
    env.add_project("XX", "xx");
    let key = format!("{ESC}[2J{ESC}[HOK");

    // Our own message, which names the unknown key.
    let output = env
        .cmd()
        .args(["create", "t", "--project", &key])
        .assert()
        .code(1);
    let stderr = &output.get_output().stderr;
    assert!(!has_raw(stderr, ESC));
    assert!(String::from_utf8_lossy(stderr).contains("\\u{1b}[2J"));

    // clap's message, which names the unknown command. clap removes
    // colour codes from it by itself, but not a mark that reorders text.
    let output = env.cmd().arg(format!("frob{RLO}x")).assert().code(1);
    let stderr = &output.get_output().stderr;
    assert!(!has_raw(stderr, RLO));
    assert!(String::from_utf8_lossy(stderr).contains("frob\\u{202e}x"));

    // Both again as JSON.
    for args in [
        vec![
            "create".to_string(),
            "t".into(),
            "--project".into(),
            key.clone(),
        ],
        vec![format!("frob{RLO}x")],
    ] {
        let output = env.cmd().args(&args).arg("--json").assert().code(1);
        let stdout = &output.get_output().stdout;
        check_schema("error", &parse(stdout));
        assert!(!has_raw(stdout, ESC) && !has_raw(stdout, RLO), "{args:?}");
        // Escaped as JSON, so the message itself is unchanged.
        let message = parse(stdout)["error"].as_str().unwrap().to_string();
        assert!(message.contains(ESC) || message.contains(RLO), "{args:?}");
    }
}

#[test]
fn for_json_keeps_the_value_and_plain_text() {
    let value = json!({ "s": format!("a{RLO}b\u{2028}c\u{85}d\u{7f}e{ESC}f é 😀 “q”\\") });
    let pretty = serde_json::to_string_pretty(&value).unwrap();
    let escaped = text::for_json(&pretty);
    assert!(
        !escaped
            .chars()
            .any(|c| text::changes_display(c) && c != '\n')
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&escaped).unwrap(),
        value
    );
    assert!(escaped.contains("é 😀 “q”"));
}

#[test]
fn for_terminal_writes_out_what_would_change_the_display() {
    assert_eq!(text::for_terminal("plain é 😀", false), "plain é 😀");
    assert_eq!(text::for_terminal("a\tb\nc", false), "a\\u{9}b\\u{a}c");
    assert_eq!(text::for_terminal("a\tb\nc", true), "a\\u{9}b\nc");
    assert_eq!(text::for_terminal("x\u{2066}y", true), "x\\u{2066}y");
}

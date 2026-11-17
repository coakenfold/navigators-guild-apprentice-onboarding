//! Slice 3: set priority, kind and labels, edit items, and filter a list
//! sorted by priority (DESIGN.md §2.1, §3).

mod common;

use std::fs;
use std::path::Path;

use common::Env;
use serde_json::{Value, json};

/// The IDs in a `list --json` output, in order.
fn ids(list: &Value) -> Vec<String> {
    list["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["id"].as_str().unwrap().to_string())
        .collect()
}

/// The IDs `tracker list <args>` prints, run from outside every root.
fn listed(env: &Env, args: &[&str]) -> Vec<String> {
    let mut all = vec!["list"];
    all.extend_from_slice(args);
    ids(&env.json(&all))
}

/// `tracker list <args>` as text, run from `dir`.
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

/// Creates an item with extra `create` flags and returns its ID.
fn create(env: &Env, project: &str, title: &str, flags: &[&str]) -> String {
    let mut args = vec!["create", title, "--project", project];
    args.extend_from_slice(flags);
    env.json(&args)["item"]["id"].as_str().unwrap().to_string()
}

/// Runs `edit` with `--json` and returns its output, after checking that
/// the file on disk holds the item it prints.
fn edit(env: &Env, args: &[&str]) -> Value {
    let mut all = vec!["edit"];
    all.extend_from_slice(args);
    let output = env.json(&all);
    let id = output["item"]["id"].as_str().unwrap();
    assert_eq!(env.item(id), output["item"], "{args:?}");
    output
}

fn history(item: &Value) -> Vec<Value> {
    item.get("history")
        .map(|h| h.as_array().unwrap().clone())
        .unwrap_or_default()
}

#[test]
fn create_defaults_to_a_medium_change_with_no_labels_or_design() {
    let env = Env::new();
    env.add_project("XX", "xx");
    let item = env.json(&["create", "a", "--project", "XX"])["item"].clone();
    assert_eq!(item["kind"], "change");
    assert_eq!(item["priority"], "medium");
    assert!(item.get("labels").is_none(), "{item}");
    assert!(item.get("design").is_none(), "{item}");
}

#[test]
fn create_takes_kind_priority_labels_and_design() {
    let env = Env::new();
    env.add_project("XX", "xx");
    let item = env.json(&[
        "create",
        "a",
        "--project",
        "XX",
        "--kind",
        "capability",
        "--priority",
        "high",
        "--label",
        "search",
        "--label",
        "ui",
        "--design",
        " DESIGN.md#2.5 ",
    ])["item"]
        .clone();
    assert_eq!(item["kind"], "capability");
    assert_eq!(item["priority"], "high");
    assert_eq!(item["labels"], json!(["search", "ui"]));
    assert_eq!(item["design"], "DESIGN.md#2.5");
    assert_eq!(env.item("XX-1"), item);

    // The file keeps §4.3's order: labels and design after priority.
    let text = fs::read_to_string(env.item_path("XX-1")).unwrap();
    let file: Value = serde_json::from_str(&text).unwrap();
    let keys: Vec<&str> = file
        .as_object()
        .unwrap()
        .keys()
        .map(|k| k.as_str())
        .collect();
    assert_eq!(
        keys,
        [
            "id", "title", "project", "kind", "status", "priority", "labels", "design", "created",
            "updated"
        ]
    );
}

#[test]
fn labels_are_lowercased_and_stored_once() {
    let env = Env::new();
    env.add_project("XX", "xx");
    let id = create(
        &env,
        "XX",
        "a",
        &[
            "--label", "Bug", "--label", "bug", "--label", " BUG ", "--label", "Ünï",
        ],
    );
    assert_eq!(env.item(&id)["labels"], json!(["bug", "ünï"]));

    // An empty label, or one that doesn't fit on a line, is refused.
    for label in ["", "  ", "two\nlines", "esc\u{1b}[2K"] {
        let message = env.error(&["create", "b", "--project", "XX", "--label", label], 1);
        assert!(message.contains("label"), "{label:?}: {message}");
    }
    assert!(!env.item_path("XX-2").exists());
}

#[test]
fn a_label_is_at_most_50_characters() {
    // A label is printed in every list row, so a long one widens the table.
    let env = Env::new();
    env.add_project("XX", "xx");
    let longest = "é".repeat(50);
    let id = create(&env, "XX", "a", &["--label", &longest]);
    assert_eq!(env.item(&id)["labels"], json!([longest]));

    let too_long = "é".repeat(51);
    for args in [
        vec!["create", "b", "--project", "XX", "--label", &too_long],
        vec!["edit", &id, "--label", &too_long],
    ] {
        let message = env.error(&args, 1);
        assert!(message.contains("50"), "{message}");
        assert!(message.contains("51"), "{message}");
    }
    assert!(!env.item_path("XX-2").exists());
    assert_eq!(env.item(&id)["labels"], json!([longest]));
}

#[test]
fn an_unknown_priority_or_kind_exits_1_and_names_the_allowed_values() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");
    let before = fs::read(env.item_path("XX-1")).unwrap();

    for args in [
        &["create", "b", "--project", "XX", "--priority", "urgent"][..],
        &["edit", "XX-1", "--priority", "urgent"],
        &["list", "--priority", "urgent"],
    ] {
        let message = env.error(args, 1);
        for value in ["low", "medium", "high"] {
            assert!(message.contains(value), "{args:?}: {message}");
        }
    }
    for args in [
        &["create", "b", "--project", "XX", "--kind", "feature"][..],
        &["edit", "XX-1", "--kind", "feature"],
        &["list", "--kind", "feature"],
    ] {
        let message = env.error(args, 1);
        for value in ["fix", "change", "capability", "chore"] {
            assert!(message.contains(value), "{args:?}: {message}");
        }
    }

    // Nothing was created or changed.
    assert!(!env.item_path("XX-2").exists());
    assert_eq!(fs::read(env.item_path("XX-1")).unwrap(), before);
}

#[test]
fn edit_priority_and_unlabel_changes_the_item_and_adds_one_history_entry() {
    let env = Env::new();
    env.add_project("XX", "xx");
    let id = create(
        &env,
        "XX",
        "a",
        &["--priority", "high", "--label", "bug", "--label", "ui"],
    );

    let output = edit(&env, &[&id, "--priority", "low", "--unlabel", "bug"]);
    assert_eq!(output["fields"], json!(["priority", "labels"]));
    let item = &output["item"];
    assert_eq!(item["priority"], "low");
    assert_eq!(item["labels"], json!(["ui"]));

    let entries = history(item);
    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    assert_eq!(
        entry,
        &json!({
            "at": entry["at"],
            "from": "open",
            "to": "open",
            "via": "edit",
            "interactive": false,
            "fields": ["priority", "labels"],
        })
    );
    assert_eq!(item["updated"], entry["at"]);

    // Removing the last label leaves no `labels` field.
    let item = edit(&env, &[&id, "--unlabel", "UI"])["item"].clone();
    assert!(item.get("labels").is_none(), "{item}");
    assert_eq!(history(&item).len(), 2);
}

#[test]
fn edit_changes_every_detail_and_names_each_field() {
    let env = Env::new();
    env.add_project("XX", "xx");
    let id = create(&env, "XX", "a", &["--description", "old"]);

    let output = edit(
        &env,
        &[
            &id,
            "--description",
            "new\nnotes",
            "--title",
            "  renamed  ",
            "--design",
            "DESIGN.md#2.1",
            "--label",
            "Needs-Review",
            "--kind",
            "fix",
            "--priority",
            "high",
        ],
    );
    // In the item file's order, whatever order the flags came in.
    assert_eq!(
        output["fields"],
        json!([
            "title",
            "kind",
            "priority",
            "labels",
            "design",
            "description"
        ])
    );
    let item = &output["item"];
    assert_eq!(item["title"], "renamed");
    assert_eq!(item["kind"], "fix");
    assert_eq!(item["priority"], "high");
    assert_eq!(item["labels"], json!(["needs-review"]));
    assert_eq!(item["design"], "DESIGN.md#2.1");
    assert_eq!(item["description"], "new\nnotes");

    // An empty --design or --description removes it.
    let output = edit(&env, &[&id, "--design", "", "--description", " "]);
    assert_eq!(output["fields"], json!(["design", "description"]));
    assert!(output["item"].get("design").is_none());
    assert!(output["item"].get("description").is_none());
    assert_eq!(history(&output["item"]).len(), 2);
}

#[test]
fn an_edit_that_changes_nothing_writes_nothing() {
    let env = Env::new();
    env.add_project("XX", "xx");
    let id = create(&env, "XX", "a", &["--label", "bug"]);
    let before = fs::read(env.item_path(&id)).unwrap();

    // Values the item already has, and a label it doesn't have to remove.
    let output = edit(
        &env,
        &[
            &id,
            "--priority",
            "medium",
            "--label",
            "BUG",
            "--unlabel",
            "ui",
            "--title",
            "a",
        ],
    );
    assert_eq!(output["fields"], json!([]));
    assert_eq!(fs::read(env.item_path(&id)).unwrap(), before);
}

#[test]
fn edit_refuses_bad_input_and_changes_nothing() {
    let env = Env::new();
    env.add_project("XX", "xx");
    let id = create(&env, "XX", "a", &["--label", "bug"]);
    let before = fs::read(env.item_path(&id)).unwrap();

    for (args, says) in [
        (&[id.as_str()][..], "--priority"),
        (&[&id, "--title", " "], "title"),
        (&[&id, "--title", "two\nlines"], "title"),
        (&[&id, "--label", ""], "label"),
        (&[&id, "--label", "Bug", "--unlabel", "bug"], "both"),
        (&[&id, "--design", "a\nb"], "design"),
    ] {
        let mut all = vec!["edit"];
        all.extend_from_slice(args);
        let message = env.error(&all, 1);
        assert!(message.contains(says), "{args:?}: {message}");
    }
    assert_eq!(fs::read(env.item_path(&id)).unwrap(), before);

    // A missing or malformed ID.
    assert!(
        env.error(&["edit", "XX-9", "--priority", "high"], 1)
            .contains("XX-9")
    );
    env.error(&["edit", "../config", "--priority", "high"], 1);
}

#[test]
fn edit_finds_an_id_typed_in_lower_case() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");
    assert_eq!(
        edit(&env, &["xx-1", "--priority", "high"])["item"]["priority"],
        "high"
    );
}

#[test]
fn edit_works_in_every_state_but_done_and_review() {
    let env = env_with_every_state();

    for (id, state) in [
        ("XX-1", "open"),
        ("XX-2", "in-progress"),
        ("XX-5", "shelved"),
    ] {
        let item = edit(&env, &[id, "--priority", "high"])["item"].clone();
        assert_eq!(item["status"], state);
        let last = history(&item).last().unwrap().clone();
        assert_eq!(last["from"], state, "{id}");
        assert_eq!(last["to"], state, "{id}");
        assert_eq!(last["via"], "edit", "{id}");
    }

    let before = fs::read(env.item_path("XX-4")).unwrap();
    let message = env.error(&["edit", "XX-4", "--priority", "high"], 1);
    assert!(message.contains("done"), "{message}");
    assert!(message.contains("tracker reopen XX-4"), "{message}");
    assert_eq!(fs::read(env.item_path("XX-4")).unwrap(), before);

    // Once reopened, it can be edited again.
    env.json(&["reopen", "XX-4", "--reason", "found a bug"]);
    edit(&env, &["XX-4", "--priority", "high"]);

    // A held item is mine to decide (§2.2), so an agent can't change it
    // while I do.
    let before = fs::read(env.item_path("XX-3")).unwrap();
    let message = env.error(&["edit", "XX-3", "--label", "bug"], 1);
    assert!(message.contains("review"), "{message}");
    assert!(message.contains("terminal"), "{message}");
    assert_eq!(fs::read(env.item_path("XX-3")).unwrap(), before);
}

#[test]
fn i_can_edit_an_item_in_review_from_a_terminal() {
    let env = env_with_every_state();
    let Some(output) = env.in_terminal(&["edit", "XX-3", "--label", "bug"]) else {
        return;
    };
    assert!(output.status.success(), "{output:?}");

    let item = env.item("XX-3");
    assert_eq!(item["status"], "review");
    assert_eq!(item["labels"], json!(["bug"]));
    let last = history(&item).last().unwrap().clone();
    assert_eq!(last["via"], "edit");
    assert_eq!(last["interactive"], true);
}

#[test]
fn an_edit_from_a_terminal_records_interactive_true() {
    // The digest marks each edit by whether it came from a terminal (§2.6).
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");
    edit(&env, &["XX-1", "--priority", "high"]);
    let Some(output) = env.in_terminal(&["edit", "XX-1", "--label", "bug"]) else {
        return;
    };
    assert!(output.status.success(), "{output:?}");

    let entries = history(&env.item("XX-1"));
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["interactive"], false);
    assert_eq!(entries[1]["interactive"], true);
    assert_eq!(entries[1]["fields"], json!(["labels"]));
}

#[test]
fn concurrent_edits_all_land() {
    // `edit` reads the item after taking the lock, so no edit is lost to
    // another that read the file first.
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");
    let labels: Vec<String> = (1..=10).map(|n| format!("l{n}")).collect();
    let children: Vec<_> = labels
        .iter()
        .map(|label| {
            env.std_cmd()
                .args(["edit", "XX-1", "--label", label, "--json"])
                .stdout(std::process::Stdio::null())
                .spawn()
                .unwrap()
        })
        .collect();
    for mut child in children {
        assert!(child.wait().unwrap().success());
    }

    let item = env.item("XX-1");
    let mut stored: Vec<String> = item["labels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|label| label.as_str().unwrap().to_string())
        .collect();
    stored.sort();
    let mut expected = labels.clone();
    expected.sort();
    assert_eq!(stored, expected);
    assert_eq!(history(&item).len(), 10);

    let left: Vec<_> = fs::read_dir(env.home.path().join("items"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(left, ["XX-1.json"]);
}

#[test]
fn a_move_after_an_edit_still_works() {
    // The edit entry's `fields` must survive the move's rewrite.
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");
    edit(&env, &["XX-1", "--label", "bug"]);
    let item = env.json(&["start", "XX-1"])["item"].clone();
    let entries = history(&item);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["fields"], json!(["labels"]));
    assert_eq!(entries[1]["via"], "start");
}

#[test]
fn list_shows_high_before_medium_before_low_within_each_project() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.add_project("YY", "yy");
    create(&env, "XX", "low", &["--priority", "low"]);
    create(&env, "XX", "medium", &[]);
    create(&env, "XX", "high", &["--priority", "high"]);
    create(&env, "YY", "low", &["--priority", "low"]);
    create(&env, "XX", "high again", &["--priority", "high"]);
    create(&env, "YY", "high", &["--priority", "high"]);

    // Grouped by project, then by priority, then by number.
    assert_eq!(
        listed(&env, &[]),
        ["XX-3", "XX-4", "XX-2", "XX-1", "YY-2", "YY-1"]
    );

    // An edit moves an item in the order.
    edit(&env, &["XX-1", "--priority", "high"]);
    assert_eq!(
        listed(&env, &["--project", "XX"]),
        ["XX-1", "XX-3", "XX-4", "XX-2"]
    );
}

#[test]
fn the_text_list_marks_priority_kind_and_labels() {
    let env = Env::new();
    env.add_project("XX", "xx");
    create(
        &env,
        "XX",
        "Fix the crash",
        &["--priority", "high", "--kind", "fix", "--label", "bug"],
    );
    create(
        &env,
        "XX",
        "Tidy up",
        &["--priority", "low", "--kind", "chore"],
    );

    let text = list_text(&env, &env.outside(), &[]);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[0], "XX  XX project");
    assert_eq!(lines[1], "  XX-1  high  fix    open  Fix the crash  [bug]");
    assert_eq!(lines[2], "  XX-2  low   chore  open  Tidy up");
}

#[test]
fn list_filters_combine() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.add_project("YY", "yy");
    // XX-1 matches all three. Each of the others misses one.
    create(
        &env,
        "XX",
        "match",
        &["--priority", "high", "--label", "bug"],
    );
    create(&env, "XX", "medium", &["--label", "bug"]);
    create(&env, "XX", "no label", &["--priority", "high"]);
    create(
        &env,
        "XX",
        "other label",
        &["--priority", "high", "--label", "ui"],
    );
    create(
        &env,
        "XX",
        "started",
        &["--priority", "high", "--label", "bug"],
    );
    env.json(&["start", "XX-5"]);
    create(
        &env,
        "YY",
        "match too",
        &["--priority", "high", "--label", "Bug"],
    );

    assert_eq!(
        listed(
            &env,
            &["--status", "open", "--priority", "high", "--label", "bug"]
        ),
        ["XX-1", "YY-1"]
    );
    // A label filter is lowercased like the labels it matches.
    assert_eq!(
        listed(&env, &["--label", "BUG", "--project", "XX"]),
        ["XX-1", "XX-5", "XX-2"]
    );
    assert_eq!(
        listed(&env, &["--kind", "change", "--priority", "medium"]),
        ["XX-2"]
    );
    assert!(listed(&env, &["--kind", "fix"]).is_empty());
    assert_eq!(listed(&env, &["--status", "in-progress"]), ["XX-5"]);

    let text = list_text(
        &env,
        &env.outside(),
        &["--priority", "high", "--label", "bug"],
    );
    assert!(text.contains("priority high"), "{text}");
    assert!(text.contains("label \"bug\""), "{text}");
}

/// A store with one XX item in each state: XX-1 open, XX-2 in-progress,
/// XX-3 in review (set by hand, since only slice 9's triggers reach it),
/// XX-4 done and XX-5 shelved.
fn env_with_every_state() -> Env {
    let env = Env::new();
    env.add_project("XX", "xx");
    for title in ["open", "in-progress", "review", "done", "shelved"] {
        env.create("XX", title);
    }
    env.json(&["start", "XX-2"]);
    let path = env.item_path("XX-3");
    let mut item = env.item("XX-3");
    item["status"] = json!("review");
    fs::write(&path, serde_json::to_string_pretty(&item).unwrap() + "\n").unwrap();
    env.json(&["start", "XX-4"]);
    env.json(&["submit", "XX-4"]);
    env.json(&["shelve", "XX-5", "--reason", "later"]);
    env
}

#[test]
fn plain_list_hides_done_and_shelved_items() {
    let env = env_with_every_state();
    assert_eq!(listed(&env, &[]), ["XX-1", "XX-2", "XX-3"]);

    let text = list_text(&env, &env.outside(), &[]);
    assert!(!text.contains("XX-5"), "{text}");
    assert!(text.contains("--status"), "{text}");
}

#[test]
fn status_shows_exactly_the_states_named() {
    let env = env_with_every_state();
    assert_eq!(listed(&env, &["--status", "done"]), ["XX-4"]);
    assert_eq!(listed(&env, &["--status", "shelved"]), ["XX-5"]);
    assert_eq!(
        listed(&env, &["--status", "open,shelved"]),
        ["XX-1", "XX-5"]
    );
    assert_eq!(
        listed(&env, &["--status", "shelved,open,open"]),
        ["XX-1", "XX-5"]
    );
    // Repeating the flag adds states too.
    assert_eq!(
        listed(&env, &["--status", "review", "--status", "done"]),
        ["XX-3", "XX-4"]
    );
    assert_eq!(
        listed(&env, &["--status", "open,in-progress,review,done,shelved"]),
        ["XX-1", "XX-2", "XX-3", "XX-4", "XX-5"]
    );

    // An unknown state exits 1 and names the states.
    for value in ["closed", "open,closed", "open,"] {
        let message = env.error(&["list", "--status", value], 1);
        assert!(message.contains("in-progress"), "{value}: {message}");
    }
}

#[test]
fn by_keeps_done_items_verified_that_way() {
    let env = env_with_every_state();
    assert_eq!(listed(&env, &["--by", "unchecked"]), ["XX-4"]);
    assert!(listed(&env, &["--by", "auto"]).is_empty());
    assert!(listed(&env, &["--by", "human"]).is_empty());
    // --by combines with --status: only done items have a record.
    assert_eq!(
        listed(&env, &["--status", "done", "--by", "unchecked"]),
        ["XX-4"]
    );
    assert!(listed(&env, &["--status", "open", "--by", "unchecked"]).is_empty());

    // A reopened item's record is in previous_verification, so it doesn't count.
    env.json(&["reopen", "XX-4", "--reason", "found a bug"]);
    assert!(listed(&env, &["--by", "unchecked"]).is_empty());

    let message = env.error(&["list", "--by", "agent"], 1);
    assert!(message.contains("unchecked"), "{message}");
}

#[test]
fn by_ignores_a_record_left_on_a_shelved_item_by_slice_2() {
    // Slice 2's XX-4 fixture is shelved but still holds `verification`.
    let env = Env::new();
    env.add_project("XX", "xx");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/slice-2/XX-4.json");
    fs::copy(&fixture, env.item_path("XX-4")).unwrap();
    assert_eq!(env.item("XX-4")["verification"]["by"], "unchecked");
    assert_eq!(env.item("XX-4")["status"], "shelved");

    assert!(listed(&env, &["--by", "unchecked"]).is_empty());
    assert!(listed(&env, &["--status", "shelved", "--by", "unchecked"]).is_empty());
    assert_eq!(listed(&env, &["--status", "shelved"]), ["XX-4"]);
}

#[test]
fn inside_a_project_root_list_shows_only_that_project() {
    let env = Env::new();
    let xx = env.add_project("XX", "xx");
    let yy = env.add_project("YY", "yy");
    env.create("XX", "a");
    env.create("YY", "b");
    let sub = env.dir("xx/src/deep");

    for dir in [&xx, &sub] {
        let list = env.json_in(dir, &["list"]);
        assert_eq!(list["project"], "XX");
        assert_eq!(ids(&list), ["XX-1"]);
    }
    let text = list_text(&env, &xx, &[]);
    assert!(
        text.contains("XX only. `--all` for every project."),
        "{text}"
    );
    assert!(!text.contains("YY"), "{text}");

    // --all there, or plain list outside every root, shows every project.
    for list in [env.json_in(&xx, &["list", "--all"]), env.json(&["list"])] {
        assert_eq!(list["project"], Value::Null);
        assert_eq!(ids(&list), ["XX-1", "YY-1"]);
    }
    for (dir, args) in [(&xx, &["--all"][..]), (&env.outside(), &[])] {
        let text = list_text(&env, dir, args);
        assert!(text.contains("Every project."), "{text}");
    }

    // --project picks another project from inside a root, or one from
    // outside them all. A lower-case key works too.
    for (dir, key) in [(&xx, "YY"), (&yy, "xx"), (&env.outside(), "YY")] {
        let list = env.json_in(dir, &["list", "--project", key]);
        let key = key.to_uppercase();
        assert_eq!(list["project"], key.as_str());
        assert_eq!(ids(&list), [format!("{key}-1")]);
    }
}

#[test]
fn the_scope_is_reported_even_when_nothing_matches() {
    let env = Env::new();
    let xx = env.add_project("XX", "xx");
    env.add_project("YY", "yy");
    env.create("YY", "b");

    let list = env.json_in(&xx, &["list"]);
    assert_eq!(list["project"], "XX");
    assert_eq!(ids(&list), Vec::<String>::new());
    let text = list_text(&env, &xx, &[]);
    assert!(text.contains("No items match."), "{text}");
    assert!(text.contains("XX only."), "{text}");
}

#[test]
fn list_refuses_an_unknown_project_and_project_with_all() {
    let env = Env::new();
    let xx = env.add_project("XX", "xx");
    env.create("XX", "a");

    let message = env.error(&["list", "--project", "NOPE"], 1);
    assert!(message.contains("NOPE"), "{message}");
    assert!(message.contains("XX"), "{message}");

    let message = env.error_in(&xx, &["list", "--project", "XX", "--all"], 1);
    assert!(message.contains("--all"), "{message}");
}

#[test]
fn project_lists_the_items_of_a_project_no_longer_in_config_json() {
    // `project list` counts such items, so `list --project` finds them too.
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");
    fs::write(
        env.home.path().join("config.json"),
        "{\n  \"projects\": {}\n}\n",
    )
    .unwrap();

    let list = env.json(&["list", "--project", "XX"]);
    assert_eq!(list["project"], "XX");
    assert_eq!(ids(&list), ["XX-1"]);
}

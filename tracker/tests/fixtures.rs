//! Item files written by earlier slices must still load in this one, and come
//! back byte-for-byte unchanged when written again (PLAN.md, integration
//! risks). `tests/fixtures/slice-<n>/` holds files that slice n's binary
//! wrote. Never edit them: a slice that changes the item file adds its own
//! folder instead, and every folder stays tested.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::{Env, check_schema};
use serde_json::Value;

/// Every fixture file, sorted, across every slice's folder.
fn fixture_files() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut files = Vec::new();
    for slice in fs::read_dir(&root).unwrap() {
        for file in fs::read_dir(slice.unwrap().path()).unwrap() {
            files.push(file.unwrap().path());
        }
    }
    files.sort();
    files
}

/// The names of the fixture files in `tests/fixtures/<slice>/`.
fn names_in(slice: &str) -> Vec<String> {
    fixture_files()
        .iter()
        .filter(|path| path.parent().unwrap().ends_with(slice))
        .map(|path| path.file_name().unwrap().to_str().unwrap().to_string())
        .collect()
}

#[test]
fn slice_1_wrote_two_fixtures() {
    assert_eq!(names_in("slice-1"), ["XX-1.json", "XX-2.json"]);
}

/// XX-3 is done and unchecked. XX-4 was done, then reopened and shelved, so
/// it has a verification record and history entries with reasons: slice 2
/// kept the record on reopen until review 1. XX-5, written after review 1,
/// was done and then reopened, so its record is in `previous_verification`.
#[test]
fn slice_2_wrote_three_fixtures_with_history() {
    assert_eq!(names_in("slice-2"), ["XX-3.json", "XX-4.json", "XX-5.json"]);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/slice-2");
    let read = |name: &str| -> Value {
        serde_json::from_str(&fs::read_to_string(root.join(name)).unwrap()).unwrap()
    };

    let item = read("XX-5.json");
    assert!(item.get("verification").is_none());
    assert_eq!(item["previous_verification"]["by"], "unchecked");

    let item = read("XX-4.json");
    assert_eq!(item["verification"]["by"], "unchecked");
    let vias: Vec<&str> = item["history"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["via"].as_str().unwrap())
        .collect();
    assert_eq!(vias, ["start", "submit", "reopen", "shelve"]);
}

/// XX-6 has every field slice 3 adds: a kind and priority other than the
/// defaults, labels, a design reference and an edit entry. XX-7 was edited
/// while in progress, which records `from` equal to `to`, and then shelved.
/// XX-8's edit, written after review 1, records the kind it replaced and
/// the labels it removed.
#[test]
fn slice_3_wrote_three_fixtures_with_edits() {
    assert_eq!(names_in("slice-3"), ["XX-6.json", "XX-7.json", "XX-8.json"]);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/slice-3");
    let read = |name: &str| -> Value {
        serde_json::from_str(&fs::read_to_string(root.join(name)).unwrap()).unwrap()
    };

    let item = read("XX-6.json");
    assert_eq!(item["kind"], "capability");
    assert_eq!(item["priority"], "high");
    assert_eq!(
        item["labels"],
        serde_json::json!(["feature", "needs-review"])
    );
    assert_eq!(item["design"], "DESIGN.md#2.5");
    assert_eq!(item["history"][0]["via"], "edit");

    let item = read("XX-7.json");
    let edit = &item["history"][1];
    assert_eq!(edit["via"], "edit");
    assert_eq!(edit["from"], "in-progress");
    assert_eq!(edit["to"], "in-progress");
    assert_eq!(
        edit["fields"],
        serde_json::json!(["title", "kind", "labels"])
    );

    let item = read("XX-8.json");
    let edit = &item["history"][0];
    assert_eq!(edit["kind_before"], "capability");
    assert_eq!(edit["labels_removed"], serde_json::json!(["needs-review"]));
}

/// The item file's last shape before slice 6 adds `criteria` (PLAN.md,
/// integration risks), one item in each state slice 5 can reach: review
/// waits for slice 9. XX-9 is open, with every field and an edit that
/// records `fields`, `kind_before` and `labels_removed`. XX-10 is in
/// progress, and XX-11 done and unchecked. XX-12 was done, then reopened,
/// so its record is in `previous_verification`, and then shelved.
#[test]
fn slice_5_wrote_four_fixtures_one_in_each_state() {
    assert_eq!(
        names_in("slice-5"),
        ["XX-10.json", "XX-11.json", "XX-12.json", "XX-9.json"]
    );
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/slice-5");
    let read = |name: &str| -> Value {
        serde_json::from_str(&fs::read_to_string(root.join(name)).unwrap()).unwrap()
    };

    let states: Vec<Value> = ["XX-9", "XX-10", "XX-11", "XX-12"]
        .iter()
        .map(|id| read(&format!("{id}.json"))["status"].clone())
        .collect();
    assert_eq!(
        states,
        ["open", "in-progress", "done", "shelved"].map(Value::from)
    );

    let item = read("XX-9.json");
    assert_eq!(item["design"], "DESIGN.md#2.5");
    assert!(item["description"].is_string());
    let edit = &item["history"][0];
    assert_eq!(edit["via"], "edit");
    assert_eq!(
        edit["fields"],
        serde_json::json!(["title", "kind", "labels"])
    );
    assert_eq!(edit["kind_before"], "capability");
    assert_eq!(edit["labels_removed"], serde_json::json!(["needs-review"]));

    assert_eq!(read("XX-11.json")["verification"]["by"], "unchecked");

    let item = read("XX-12.json");
    assert!(item.get("verification").is_none());
    assert_eq!(item["previous_verification"]["by"], "unchecked");
    let vias: Vec<&str> = item["history"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["via"].as_str().unwrap())
        .collect();
    assert_eq!(vias, ["start", "submit", "reopen", "shelve"]);
}

/// The item file after slice 6 adds `written`, `criteria` and the
/// `criterion-accept` history entry. XX-13 is open, with every field: two
/// criteria written at create, one with evidence, and a script criterion
/// an agent added, so `written` has two texts for three criteria.
/// XX-14's criteria were accepted from a terminal, then it was started and
/// shelved. XX-15 is done, unchecked, after gates G2 and G6 passed.
#[test]
fn slice_6_wrote_three_fixtures_with_criteria() {
    assert_eq!(
        names_in("slice-6"),
        ["XX-13.json", "XX-14.json", "XX-15.json"]
    );
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/slice-6");
    let read = |name: &str| -> Value {
        serde_json::from_str(&fs::read_to_string(root.join(name)).unwrap()).unwrap()
    };

    let item = read("XX-13.json");
    assert_eq!(item["status"], "open");
    assert_eq!(item["written"].as_array().unwrap().len(), 2);
    let criteria = item["criteria"].as_array().unwrap();
    assert_eq!(criteria.len(), 3);
    assert!(criteria[1]["evidence"].is_string());
    assert!(criteria[2]["check"].is_string());
    assert_eq!(item["written"][0], criteria[0]["text"]);
    assert_eq!(item["written"][1], criteria[1]["text"]);

    let item = read("XX-14.json");
    assert_eq!(item["status"], "shelved");
    assert_eq!(item["written"].as_array().unwrap().len(), 2);
    let accept = &item["history"][0];
    assert_eq!(accept["via"], "criterion-accept");
    assert_eq!(accept["interactive"], true);
    assert_eq!(accept["fields"], serde_json::json!(["written"]));

    let item = read("XX-15.json");
    assert_eq!(item["status"], "done");
    assert_eq!(item["verification"]["by"], "unchecked");
    let note = item["verification"]["note"].as_str().unwrap();
    assert!(note.starts_with("No checks ran"), "{note}");
}

#[test]
fn every_fixture_loads_and_comes_back_byte_for_byte() {
    for path in fixture_files() {
        let text = fs::read_to_string(&path).unwrap();
        let item = tracker::item::from_json(&text)
            .unwrap_or_else(|e| panic!("{} no longer loads: {e}", path.display()));
        assert_eq!(
            tracker::item::to_json(&item),
            text,
            "{} changed when written again",
            path.display()
        );
        check_schema("item", &serde_json::from_str(&text).unwrap());
        // So a later slice's binary can rewrite it without losing anything.
        let lost = tracker::item::lost_on_rewrite(&text).unwrap();
        assert!(lost.is_empty(), "{} would change {lost:?}", path.display());
    }
}

#[test]
fn the_binary_lists_every_fixture_as_it_is_on_disk() {
    let env = Env::new();
    env.add_project("XX", "xx");
    let files = fixture_files();
    for path in &files {
        let to = env
            .home
            .path()
            .join("items")
            .join(path.file_name().unwrap());
        fs::copy(path, to).unwrap();
    }

    // Every state, since plain `list` hides done and shelved items. `list`
    // sorts by priority, so each item is found by its ID.
    let list = env.json(&["list", "--status", "open,in-progress,review,done,shelved"]);
    let listed = list["items"].as_array().unwrap();
    assert_eq!(listed.len(), files.len());
    for path in &files {
        let file: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        let item = listed.iter().find(|item| item["id"] == file["id"]);
        assert_eq!(item, Some(&file), "{}", path.display());
    }
}

#[test]
fn the_binary_shows_every_fixture_with_each_history_time() {
    let env = Env::new();
    env.add_project("XX", "xx");
    for path in fixture_files() {
        let to = env
            .home
            .path()
            .join("items")
            .join(path.file_name().unwrap());
        fs::copy(&path, to).unwrap();
        let file: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let id = file["id"].as_str().unwrap();

        assert_eq!(env.json(&["show", id])["item"], file, "{}", path.display());
        let output = env.cmd().args(["show", id]).assert().success();
        let text = String::from_utf8(output.get_output().stdout.clone()).unwrap();
        assert!(text.contains(file["title"].as_str().unwrap()), "{text}");
        for entry in file["history"].as_array().into_iter().flatten() {
            let at = entry["at"].as_str().unwrap();
            assert!(text.contains(at), "{at} isn't in:\n{text}");
        }
    }
}

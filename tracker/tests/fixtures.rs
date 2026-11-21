//! Item files written by earlier slices must still load in this one, and come
//! back byte-for-byte unchanged when written again (PLAN.md, integration
//! risks). `tests/fixtures/slice-<n>/` holds files that slice n's binary
//! wrote. Never edit them: a slice that changes the item file adds its own
//! folder instead, and every folder stays tested.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::{Env, check_schema};
use serde_json::{Value, json};

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

/// The item file after slice 7 adds `started` and the checks in a
/// verification record. XX-16 was started with one check failing and one
/// already passing, then shelved and reopened, so it's open and still has
/// what its start recorded. XX-17 is done, unchecked, after its criterion
/// check, which printed 25 lines, and a project check passed. XX-18, written
/// after TR-16, is checked below.
#[test]
fn slice_7_wrote_three_fixtures_with_check_results() {
    assert_eq!(
        names_in("slice-7"),
        ["XX-16.json", "XX-17.json", "XX-18.json"]
    );
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/slice-7");
    let read = |name: &str| -> Value {
        serde_json::from_str(&fs::read_to_string(root.join(name)).unwrap()).unwrap()
    };

    let item = read("XX-16.json");
    assert_eq!(item["status"], "open");
    assert_eq!(
        item["started"]["checks"],
        serde_json::json!([
            { "criterion": 1, "passed": false },
            { "criterion": 2, "passed": true },
        ])
    );

    let item = read("XX-17.json");
    assert_eq!(item["status"], "done");
    assert_eq!(item["started"]["checks"][0]["passed"], false);
    let checks = item["verification"]["checks"].as_array().unwrap();
    assert_eq!(checks.len(), 2);
    let output = checks[0]["output"].as_str().unwrap();
    assert_eq!(output.lines().count(), 20);
    assert!(output.starts_with("line 6\n"), "{output}");
    assert_eq!(checks[1]["output"], "all 3 tests passed");
}

/// TR-20: XX-18 was written after TR-16 gave each criterion's check record
/// its `criterion`, so it keeps that shape for later slices to read. It's
/// done, unchecked, after the checks of criteria 1 and 3, with a manual
/// criterion 2 between them, and a project check, which names none.
#[test]
fn slice_7s_xx_18_names_each_checks_criterion_and_comes_back_byte_for_byte() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/slice-7/XX-18.json");
    let text = fs::read_to_string(&path).unwrap();
    let file: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(file["status"], "done");
    let criteria: Vec<Value> = file["verification"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|check| check["criterion"].clone())
        .collect();
    assert_eq!(criteria, [json!(1), json!(3), Value::Null]);
    assert_eq!(
        file["verification"]["checks"][1]["output"],
        "parsed 3 lines"
    );

    let item = tracker::item::from_json(&text).unwrap();
    assert_eq!(tracker::item::to_json(&item), text);
    let lost = tracker::item::lost_on_rewrite(&text).unwrap();
    assert!(lost.is_empty(), "{lost:?}");
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

/// TR-4: XX-19 was written by slice 8's binary, the first to prove work
/// against a commit. It's done, `auto`, with the `HEAD` its start recorded,
/// and the commit and ref its submit recorded, before its checks: a
/// criterion's, which names criterion 1, and a project check.
#[test]
fn slice_8s_xx_19_has_a_commit_and_a_ref_and_comes_back_byte_for_byte() {
    assert_eq!(names_in("slice-8"), ["XX-19.json"]);
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/slice-8/XX-19.json");
    let text = fs::read_to_string(&path).unwrap();
    let file: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(file["status"], "done");
    assert_eq!(
        file["started"]["head"],
        "79ccc98a447884d2f6360687ef17fce9eae9428f"
    );
    let record = &file["verification"];
    assert_eq!(record["by"], "auto");
    assert_eq!(record["commit"], "0e949586dff8e8424f1fbdf9cfaea3035cdf0cf7");
    assert_eq!(record["ref"], "refs/tracker/XX-19");
    assert!(record.get("note").is_none(), "{record}");
    // The fields come in §4.3's order: `head` after `at`, and `commit`
    // and `ref` before `checks`.
    let started: Vec<&String> = file["started"].as_object().unwrap().keys().collect();
    assert_eq!(started, ["at", "head", "checks"]);
    let fields: Vec<&String> = record.as_object().unwrap().keys().collect();
    assert_eq!(fields, ["by", "at", "commit", "ref", "checks"]);
    assert_eq!(record["checks"][0]["criterion"], 1);
    assert_eq!(record["checks"][0]["output"], "test filters_by_tag ... ok");
    assert!(record["checks"][1].get("criterion").is_none());

    let item = tracker::item::from_json(&text).unwrap();
    assert_eq!(tracker::item::to_json(&item), text);
    let lost = tracker::item::lost_on_rewrite(&text).unwrap();
    assert!(lost.is_empty(), "{lost:?}");
}

/// XX-20 is held in review by T6 and T9, with its `holds`, after
/// `verification`'s place and before `history`. XX-21 was held, rejected
/// with a reason, resubmitted and approved, so its record is a human's,
/// with the triggers it approved after its checks.
#[test]
fn slice_9s_held_and_approved_fixtures_come_back_byte_for_byte() {
    assert_eq!(names_in("slice-9"), ["XX-20.json", "XX-21.json"]);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/slice-9");
    let read = |name: &str| -> (String, Value) {
        let text = fs::read_to_string(root.join(name)).unwrap();
        let file = serde_json::from_str(&text).unwrap();
        (text, file)
    };

    let (held_text, held) = read("XX-20.json");
    assert_eq!(held["status"], "review");
    assert!(held.get("verification").is_none(), "{held}");
    let triggers: Vec<&str> = held["holds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|hold| hold["trigger"].as_str().unwrap())
        .collect();
    assert_eq!(triggers, ["T6", "T9"]);
    let fields: Vec<&String> = held.as_object().unwrap().keys().collect();
    let at = |name: &str| fields.iter().position(|f| *f == name).unwrap();
    assert_eq!(at("holds") + 1, at("history"), "{fields:?}");
    assert_eq!(held["history"][1]["to"], "review");

    let (approved_text, approved) = read("XX-21.json");
    assert_eq!(approved["status"], "done");
    assert!(approved.get("holds").is_none(), "{approved}");
    let record = &approved["verification"];
    assert_eq!(record["by"], "human");
    assert_eq!(record["triggers"][0]["trigger"], "T6");
    let fields: Vec<&String> = record.as_object().unwrap().keys().collect();
    assert_eq!(fields, ["by", "at", "commit", "ref", "checks", "triggers"]);
    let vias: Vec<&str> = approved["history"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["via"].as_str().unwrap())
        .collect();
    assert_eq!(vias, ["start", "submit", "reject", "submit", "approve"]);
    assert_eq!(
        approved["history"][2]["reason"],
        "The empty state is missing."
    );
    assert_eq!(approved["history"][4]["interactive"], true);

    for text in [held_text, approved_text] {
        let item = tracker::item::from_json(&text).unwrap();
        assert_eq!(tracker::item::to_json(&item), text);
        let lost = tracker::item::lost_on_rewrite(&text).unwrap();
        assert!(lost.is_empty(), "{lost:?}");
        check_schema("item", &serde_json::from_str(&text).unwrap());
    }
}

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
/// it has a verification record and history entries with reasons.
#[test]
fn slice_2_wrote_two_fixtures_with_history() {
    assert_eq!(names_in("slice-2"), ["XX-3.json", "XX-4.json"]);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/slice-2");
    let item: Value =
        serde_json::from_str(&fs::read_to_string(root.join("XX-4.json")).unwrap()).unwrap();
    assert_eq!(item["verification"]["by"], "unchecked");
    let vias: Vec<&str> = item["history"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["via"].as_str().unwrap())
        .collect();
    assert_eq!(vias, ["start", "submit", "reopen", "shelve"]);
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
        assert!(lost.is_empty(), "{} would lose {lost:?}", path.display());
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

    let list = env.json(&["list"]);
    let listed = list["items"].as_array().unwrap();
    assert_eq!(listed.len(), files.len());
    for (item, path) in listed.iter().zip(&files) {
        let file: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(item, &file, "{}", path.display());
    }
}

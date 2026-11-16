//! `write_file` writes atomically (§4.4): a temporary file, flushed, then
//! renamed over the original, with no temporary file left either way.

use std::fs;

use tempfile::TempDir;
use tracker::store::write_file;

fn names(dir: &TempDir) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

#[test]
fn a_write_replaces_the_file_and_leaves_no_temporary_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("counters.json");
    write_file(&path, "{}\n").unwrap();
    write_file(&path, "{\"XX\": 2}\n").unwrap();

    assert_eq!(fs::read_to_string(&path).unwrap(), "{\"XX\": 2}\n");
    assert_eq!(names(&dir), ["counters.json"]);
}

#[test]
fn a_failed_write_leaves_no_temporary_file() {
    let dir = TempDir::new().unwrap();
    // A folder that isn't empty can't be renamed over, so the write fails
    // after the temporary file is written.
    let path = dir.path().join("XX-1.json");
    fs::create_dir(&path).unwrap();
    fs::write(path.join("inside"), "").unwrap();

    let error = write_file(&path, "{}\n").unwrap_err();
    assert_eq!(error.code, 3);
    assert_eq!(names(&dir), ["XX-1.json"]);
    assert!(path.join("inside").exists());
}

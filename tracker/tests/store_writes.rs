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

/// Someone with write access to the store plants a symlink where the
/// temporary file would go, pointing at a file outside it.
#[cfg(unix)]
fn plant_symlink(dir: &TempDir, name: &str) -> std::path::PathBuf {
    let victim = dir.path().join("victim");
    fs::write(&victim, "mine\n").unwrap();
    std::os::unix::fs::symlink(&victim, dir.path().join(name)).unwrap();
    victim
}

#[cfg(unix)]
#[test]
fn a_planted_tmp_symlink_is_not_written_through() {
    let dir = TempDir::new().unwrap();
    let victim = plant_symlink(&dir, "XX-1.json.tmp");
    let path = dir.path().join("XX-1.json");

    write_file(&path, "{}\n").unwrap();

    assert_eq!(fs::read_to_string(&victim).unwrap(), "mine\n");
    assert_eq!(fs::read_to_string(&path).unwrap(), "{}\n");
}

#[cfg(unix)]
#[test]
fn a_symlink_planted_at_this_process_tmp_name_is_refused() {
    let dir = TempDir::new().unwrap();
    let name = format!("XX-1.json.{}.tmp", std::process::id());
    let victim = plant_symlink(&dir, &name);
    let path = dir.path().join("XX-1.json");

    let error = write_file(&path, "{}\n").unwrap_err();

    assert_eq!(error.code, 3);
    assert!(error.message.contains(&name), "{}", error.message);
    assert_eq!(fs::read_to_string(&victim).unwrap(), "mine\n");
    assert!(!path.exists());
}

//! `config.json` (§4.2): the registered projects.
//!
//! So far the tool reads only each project's name, root and design. serde
//! skips the other settings when reading, and `add_project` edits the file as
//! plain JSON, so settings added by hand survive.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::store::{self, Store};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub projects: BTreeMap<String, Project>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub root: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub design: Option<String>,
}

impl Project {
    /// The root as a path, with a leading `~/` expanded to the home folder,
    /// as in §4.2's example.
    pub fn root_path(&self) -> PathBuf {
        if let Some(rest) = self.root.strip_prefix("~/")
            && let Some(home) = std::env::var_os("HOME")
        {
            return PathBuf::from(home).join(rest);
        }
        PathBuf::from(&self.root)
    }
}

pub fn load(store: &Store) -> Result<Config> {
    let path = store.config_path();
    let text = store::read_file(&path)?;
    let config: Config = serde_json::from_str(&text)
        .map_err(|e| Error::store(format!("Can't parse {}: {e}", path.display())))?;
    // Keys written by hand are checked too: a key becomes part of every item
    // ID, and so of a file path, and a key such as `../x` would put item
    // files outside the store.
    for key in config.projects.keys() {
        if !is_valid_key(key) {
            return Err(Error::store(format!(
                "{} has a project key \"{key}\", which isn't valid. A key is up to 10 \
                 capital letters and digits, starting with a letter, such as BM. Rename it there.",
                path.display()
            )));
        }
    }
    // A relative root would be read from the current folder, so it would
    // name a different folder wherever a command runs: `.` would put every
    // folder inside that project. `project add` stores absolute roots, so
    // only a root written by hand can be relative.
    for (key, project) in &config.projects {
        if !project.root_path().is_absolute() {
            return Err(Error::store(format!(
                "{} gives {key} the root \"{}\", which isn't an absolute path, so it would \
                 mean a different folder wherever tracker runs. Write the full path there, or \
                 start it with ~/.",
                path.display(),
                project.root
            )));
        }
    }
    Ok(config)
}

/// Adds a project to `config.json`. Call this only while holding the lock.
pub fn add_project(store: &Store, key: &str, project: &Project) -> Result<()> {
    let path = store.config_path();
    let text = store::read_file(&path)?;
    // Edited as plain JSON, not as `Config`, so that settings this version
    // doesn't know about yet are kept, in the order they were written.
    let mut config: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| Error::store(format!("Can't parse {}: {e}", path.display())))?;
    let Some(projects) = config
        .as_object_mut()
        .map(|top| top.entry("projects").or_insert(serde_json::json!({})))
        .and_then(|projects| projects.as_object_mut())
    else {
        return Err(Error::store(format!(
            "{} should be a JSON object with a \"projects\" object in it.",
            path.display()
        )));
    };

    if projects.contains_key(key) {
        return Err(Error::input(format!(
            "There's already a project {key}. Pick another key, or edit {} to change it.",
            path.display()
        )));
    }
    let value = serde_json::to_value(project).expect("projects always serialize");
    projects.insert(key.to_string(), value);

    let text = serde_json::to_string_pretty(&config).expect("JSON values always serialize");
    store::write_file(&path, &format!("{text}\n"))
}

/// Checks a project key and returns it in upper case. A key is what comes
/// before the dash in an item ID, such as `BM` in `BM-12`.
pub fn normalize_key(key: &str) -> Result<String> {
    let key = key.trim().to_ascii_uppercase();
    if !is_valid_key(&key) {
        return Err(Error::input(format!(
            "\"{key}\" isn't a valid project key. Use up to 10 letters and digits, \
             starting with a letter, such as BM or TR."
        )));
    }
    Ok(key)
}

/// Whether `key` is a project key exactly as stored: up to 10 capital
/// letters and digits, starting with a letter, such as `BM` or `TR2`.
pub fn is_valid_key(key: &str) -> bool {
    key.len() <= 10
        && key.starts_with(|c: char| c.is_ascii_uppercase())
        && key
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
}

/// The key of the project whose root contains `dir`. When roots are nested,
/// as with a monorepo and one of its packages, the deepest root wins.
pub fn project_for_dir(config: &Config, dir: &Path) -> Option<String> {
    let dir = fs::canonicalize(dir).ok()?;
    let mut best: Option<(String, usize)> = None;
    for (key, project) in &config.projects {
        // A root that no longer exists can't contain anything.
        let Ok(root) = fs::canonicalize(project.root_path()) else {
            continue;
        };
        let depth = root.components().count();
        let deeper = best
            .as_ref()
            .is_none_or(|(_, best_depth)| depth > *best_depth);
        if dir.starts_with(&root) && deeper {
            best = Some((key.clone(), depth));
        }
    }
    best.map(|(key, _)| key)
}

/// The key of the project whose root is exactly `root`, a canonical path.
pub fn project_with_root(config: &Config, root: &Path) -> Option<String> {
    config.projects.iter().find_map(|(key, project)| {
        let theirs = fs::canonicalize(project.root_path()).ok()?;
        (theirs == root).then(|| key.clone())
    })
}

/// The registered keys, for error messages, such as "BM, TR".
pub fn key_list(config: &Config) -> String {
    let keys: Vec<&str> = config.projects.keys().map(|k| k.as_str()).collect();
    keys.join(", ")
}

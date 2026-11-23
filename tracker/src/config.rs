//! `config.json` (§4.2): the registered projects.
//!
//! The tool reads each project's name, root, design, checks, check
//! timeout, `review` and `judge` settings, and the global `judge` settings. A key under a project that isn't
//! one of these is refused, and `add_project` edits the file as plain JSON,
//! so settings added by hand survive.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{self, Error, Result};
use crate::store::{self, Store};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub projects: BTreeMap<String, Project>,
    /// How to call the judge, for every project (§4.2). Each setting left
    /// out takes its default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub judge: Option<Judge>,
}

/// The global `judge` settings (§2.3.1, §4.2).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Judge {
    /// A line of `sh` that reads a judge request on stdin and writes the
    /// answers on stdout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// A full model name, never an alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// The largest excerpt a question is asked about, in bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_input_bytes: Option<usize>,
    /// The lines shown either side of an `each` match.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_lines: Option<usize>,
}

/// The judge command when the config doesn't name one (§4.2).
pub const DEFAULT_JUDGE_COMMAND: &str = "tracker-judge";
/// The judge model when the config doesn't name one (§4.2).
pub const DEFAULT_JUDGE_MODEL: &str = "claude-haiku-5-5";
/// `max_input_bytes` when the config doesn't set it (§4.2).
pub const DEFAULT_MAX_INPUT_BYTES: usize = 40000;
/// `context_lines` when neither the config nor the judge sets it (§4.2).
pub const DEFAULT_CONTEXT_LINES: usize = 12;
/// A project's `judge.timeout_secs` when it isn't set (§4.2).
pub const DEFAULT_JUDGE_TIMEOUT_SECS: u64 = 60;

/// How many different failed judge requests hold an item by default (T11).
pub const DEFAULT_MAX_FAILURES: u64 = 3;

impl Config {
    fn judge(&self) -> Judge {
        self.judge.clone().unwrap_or_default()
    }

    pub fn judge_command(&self) -> String {
        self.judge()
            .command
            .unwrap_or_else(|| DEFAULT_JUDGE_COMMAND.to_string())
    }

    pub fn judge_model(&self) -> String {
        self.judge()
            .model
            .unwrap_or_else(|| DEFAULT_JUDGE_MODEL.to_string())
    }

    pub fn max_input_bytes(&self) -> usize {
        self.judge()
            .max_input_bytes
            .unwrap_or(DEFAULT_MAX_INPUT_BYTES)
    }

    pub fn context_lines(&self) -> usize {
        self.judge().context_lines.unwrap_or(DEFAULT_CONTEXT_LINES)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub name: String,
    pub root: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub design: Option<String>,
    /// The project checks, each a line of `sh` run in the root, that G4
    /// runs on every submit (§2.4). Set by hand in `config.json`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub checks: Vec<String>,
    /// How long a check may run before it's stopped and fails, in
    /// seconds. `DEFAULT_CHECK_TIMEOUT_SECS` when it isn't set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check_timeout_secs: Option<u64>,
    /// What the triggers look for (§2.5, §4.2). Set by hand in
    /// `config.json`; each setting left out takes its default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review: Option<Review>,
    /// The project's judge settings (§4.2). Each one left out takes its
    /// default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub judge: Option<ProjectJudge>,
}

/// A project's `judge` settings (§4.2).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectJudge {
    /// How many different failed judge requests hold an item (T11).
    /// `DEFAULT_MAX_FAILURES` when it isn't set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_failures: Option<u64>,
    /// `false` holds every item a judge passed (T10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_verify: Option<bool>,
    /// How long the judge may take per question, in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
}

/// A project's `review` settings (§4.2). Each glob is matched against a
/// changed file's path relative to the project's root, and `*` doesn't
/// match `/`, so `*.js` names only files at the top of the root.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Review {
    /// Files whose change holds an item (T2). None by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risky_paths: Option<Vec<String>>,
    /// What counts as a source file (§2.1), minus `test_paths`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_paths: Option<Vec<String>>,
    /// What counts as a test file (T4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_paths: Option<Vec<String>>,
    /// Dependency manifests and lockfiles (T3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifests: Option<Vec<String>>,
    /// More changed lines than this hold an item (T7).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_diff_lines: Option<u64>,
    /// Triggers switched off for this project, such as `["T8"]`. T6, T10
    /// and T12 stay on whatever it says.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disabled: Vec<String>,
    /// How often the digest is due, such as `{"every_days": 7}` (§4.2).
    /// Slice 16 reads it, and it's kept as written until then.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<serde_json::Value>,
}

/// `source_paths` when the project doesn't set it (§4.2).
pub const DEFAULT_SOURCE_PATHS: &[&str] = &["src/**"];
/// `test_paths` when the project doesn't set it.
pub const DEFAULT_TEST_PATHS: &[&str] = &["tests/**"];
/// `manifests` when the project doesn't set it: the usual dependency
/// manifests and lockfiles, at any depth under the root (§2.5).
pub const DEFAULT_MANIFESTS: &[&str] = &[
    "**/Cargo.toml",
    "**/Cargo.lock",
    "**/package.json",
    "**/package-lock.json",
    "**/npm-shrinkwrap.json",
    "**/yarn.lock",
    "**/pnpm-lock.yaml",
    "**/go.mod",
    "**/go.sum",
    "**/pyproject.toml",
    "**/requirements.txt",
    "**/Gemfile",
    "**/Gemfile.lock",
];
/// `max_diff_lines` when the project doesn't set it, as in §4.2.
pub const DEFAULT_MAX_DIFF_LINES: u64 = 300;
/// The triggers `review.disabled` can't switch off, since each would
/// otherwise let unchecked work reach done (§2.5).
pub const ALWAYS_ON: &[&str] = &["T6", "T10", "T12"];

impl Review {
    pub fn risky_paths(&self) -> Vec<String> {
        self.risky_paths.clone().unwrap_or_default()
    }

    pub fn source_paths(&self) -> Vec<String> {
        or_default(&self.source_paths, DEFAULT_SOURCE_PATHS)
    }

    pub fn test_paths(&self) -> Vec<String> {
        or_default(&self.test_paths, DEFAULT_TEST_PATHS)
    }

    pub fn manifests(&self) -> Vec<String> {
        or_default(&self.manifests, DEFAULT_MANIFESTS)
    }

    pub fn max_diff_lines(&self) -> u64 {
        self.max_diff_lines.unwrap_or(DEFAULT_MAX_DIFF_LINES)
    }

    /// Whether `trigger` runs for this project: it isn't in `disabled`,
    /// or it's one that can't be switched off.
    pub fn runs(&self, trigger: &str) -> bool {
        ALWAYS_ON.contains(&trigger) || !self.disabled.iter().any(|name| name == trigger)
    }
}

fn or_default(globs: &Option<Vec<String>>, default: &[&str]) -> Vec<String> {
    match globs {
        Some(globs) => globs.clone(),
        None => default.iter().map(|glob| glob.to_string()).collect(),
    }
}

/// The globs `globs`, as one set, matched as `Review` says. Fails with
/// the glob that isn't valid.
pub fn glob_set(globs: &[String]) -> std::result::Result<globset::GlobSet, String> {
    let mut set = globset::GlobSetBuilder::new();
    for glob in globs {
        let glob = globset::GlobBuilder::new(glob)
            .literal_separator(true)
            .build()
            .map_err(|e| format!("\"{glob}\" isn't a valid glob: {e}"))?;
        set.add(glob);
    }
    set.build().map_err(|e| e.to_string())
}

/// How long a check may run when the project doesn't say, as in §4.2.
pub const DEFAULT_CHECK_TIMEOUT_SECS: u64 = 300;

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

    /// How long one of the project's checks may run (§4.2).
    pub fn check_timeout(&self) -> std::time::Duration {
        let secs = self
            .check_timeout_secs
            .unwrap_or(DEFAULT_CHECK_TIMEOUT_SECS);
        std::time::Duration::from_secs(secs)
    }

    /// The project's `review` settings, with a default for each one left
    /// out (§4.2).
    pub fn review(&self) -> Review {
        self.review.clone().unwrap_or_default()
    }

    /// Whether an item a judge passed can reach done without a human
    /// (T10).
    pub fn auto_verify(&self) -> bool {
        self.judge
            .as_ref()
            .and_then(|judge| judge.auto_verify)
            .unwrap_or(true)
    }

    /// How many different failed judge requests since start hold an item
    /// (T11, §4.2).
    pub fn max_failures(&self) -> u64 {
        self.judge
            .as_ref()
            .and_then(|judge| judge.max_failures)
            .unwrap_or(DEFAULT_MAX_FAILURES)
    }

    /// How long the judge may take per question (§4.2).
    pub fn judge_timeout(&self) -> std::time::Duration {
        let secs = self
            .judge
            .as_ref()
            .and_then(|judge| judge.timeout_secs)
            .unwrap_or(DEFAULT_JUDGE_TIMEOUT_SECS);
        std::time::Duration::from_secs(secs)
    }
}

/// What to do about a `config.json` that can't be parsed.
const CONFIG_FIX: &str = "It's JSON, laid out as in DESIGN.md §4.2, so fix it by hand. Every \
                          command refuses until it parses, so a broken config is never \
                          mistaken for one with no projects.";

pub fn load(store: &Store) -> Result<Config> {
    let path = store.config_path();
    let text = store::read_file(&path)?;
    let config: Config =
        serde_json::from_str(&text).map_err(|e| error::parse(&path, &e, CONFIG_FIX))?;
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
    // A glob that can't be read would leave a trigger unable to run, so
    // it's refused here, as a key or root is, rather than at a submit.
    for (key, project) in &config.projects {
        let review = project.review();
        for globs in [
            review.risky_paths(),
            review.source_paths(),
            review.test_paths(),
            review.manifests(),
        ] {
            if let Err(why) = glob_set(&globs) {
                return Err(Error::store(format!(
                    "{} gives {key} a review setting that can't be used: {why}. Fix it there.",
                    path.display()
                )));
            }
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
    let mut config: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| error::parse(&path, &e, CONFIG_FIX))?;
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

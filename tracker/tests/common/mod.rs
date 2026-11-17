//! Shared test setup. Every test gets its own temporary store (`TRACKER_HOME`)
//! and a separate temporary folder for project roots, so no test touches
//! `~/.tracker` or sees another test's items.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

pub struct Env {
    /// The store, which `TRACKER_HOME` points to.
    pub home: TempDir,
    /// Project roots, a folder outside all of them, and `HOME`.
    pub work: TempDir,
}

impl Env {
    /// A fresh store that `tracker init` has already set up.
    pub fn new() -> Env {
        let env = Env::without_store();
        env.json(&["init"]);
        env
    }

    /// An empty `TRACKER_HOME`, with no store in it yet.
    pub fn without_store() -> Env {
        let env = Env {
            home: TempDir::new().unwrap(),
            work: TempDir::new().unwrap(),
        };
        fs::create_dir_all(env.outside()).unwrap();
        fs::create_dir_all(env.user_home()).unwrap();
        env
    }

    /// What `HOME` points to, so that a test which unsets `TRACKER_HOME`, or
    /// forgets it, uses `<work>/home/.tracker` rather than the real store.
    pub fn user_home(&self) -> PathBuf {
        self.work.path().join("home")
    }

    /// A folder that's inside no project root.
    pub fn outside(&self) -> PathBuf {
        self.work.path().join("outside")
    }

    /// A folder under the work folder, created if it doesn't exist.
    pub fn dir(&self, relative: &str) -> PathBuf {
        let dir = self.work.path().join(relative);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// `tracker` with this test's store and `HOME`, run from a folder
    /// outside every project root unless the test changes `current_dir`.
    pub fn cmd(&self) -> Command {
        Command::from_std(self.std_cmd())
    }

    /// The same as `cmd`, as a `std::process::Command`, for tests that
    /// need to spawn the process themselves. Every test command starts here.
    pub fn std_cmd(&self) -> std::process::Command {
        let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_tracker"));
        cmd.env("TRACKER_HOME", self.home.path())
            .env("HOME", self.user_home())
            .current_dir(self.outside());
        cmd
    }

    /// Runs `tracker <args> --json` from `dir`, expects exit 0, checks the
    /// output against its command's schema, and returns it.
    pub fn json_in(&self, dir: &Path, args: &[&str]) -> Value {
        let output = self
            .cmd()
            .current_dir(dir)
            .args(args)
            .arg("--json")
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let value = parse(&output);
        check_schema(schema_for(args), &value);
        value
    }

    /// Runs `tracker <args> --json` from outside every project root.
    pub fn json(&self, args: &[&str]) -> Value {
        self.json_in(&self.outside(), args)
    }

    /// Runs `tracker <args> --json` from `dir`, expects it to fail with
    /// `code`, checks the output against the error schema, and returns the
    /// error message.
    pub fn error_in(&self, dir: &Path, args: &[&str], code: i32) -> String {
        let error = self.error_json_in(dir, args, code);
        error["error"].as_str().unwrap().to_string()
    }

    /// The same as `error_in`, returning the whole error object, for the
    /// fields some errors add, such as `allowed`.
    pub fn error_json_in(&self, dir: &Path, args: &[&str], code: i32) -> Value {
        let output = self
            .cmd()
            .current_dir(dir)
            .args(args)
            .arg("--json")
            .assert()
            .code(code)
            .get_output()
            .stdout
            .clone();
        let error = parse(&output);
        check_schema("error", &error);
        assert_eq!(error["exit_code"], code, "{error}");
        error
    }

    /// Runs `tracker <args> --json` from outside every project root and
    /// expects it to fail with `code`. Returns the error message.
    pub fn error(&self, args: &[&str], code: i32) -> String {
        self.error_in(&self.outside(), args, code)
    }

    /// Runs `tracker <args>` under `script`, on a pseudo-terminal as if I'd
    /// typed it, with no pty crate (PLAN.md). Its options differ between
    /// macOS and BSD, and util-linux. Returns `None` where `script` isn't
    /// installed, so the test can skip.
    pub fn in_terminal(&self, args: &[&str]) -> Option<std::process::Output> {
        let tracker = env!("CARGO_BIN_EXE_tracker");
        let mut cmd = std::process::Command::new("script");
        if cfg!(target_os = "linux") {
            let mut line = format!("'{tracker}'");
            for arg in args {
                line.push_str(&format!(" '{}'", arg.replace('\'', "'\\''")));
            }
            cmd.args(["-qec", &line, "/dev/null"]);
        } else {
            cmd.args(["-q", "/dev/null", tracker]).args(args);
        }
        cmd.env("TRACKER_HOME", self.home.path())
            .env("HOME", self.user_home())
            .current_dir(self.outside())
            .stdin(std::process::Stdio::null());
        match cmd.output() {
            Ok(output) => Some(output),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                eprintln!("Skipped: `script` isn't installed.");
                None
            }
            Err(e) => panic!("Can't run script: {e}"),
        }
    }

    /// Registers project `key` with the root `<work>/<root>`, and returns
    /// that root.
    pub fn add_project(&self, key: &str, root: &str) -> PathBuf {
        let root = self.dir(root);
        let name = format!("{key} project");
        self.json(&[
            "project",
            "add",
            key,
            "--name",
            &name,
            "--root",
            root.to_str().unwrap(),
        ]);
        root
    }

    /// Creates an item in `project` and returns its ID.
    pub fn create(&self, project: &str, title: &str) -> String {
        let output = self.json(&["create", title, "--project", project]);
        output["item"]["id"].as_str().unwrap().to_string()
    }

    /// Reads an item's file as JSON.
    pub fn item(&self, id: &str) -> Value {
        let text = fs::read_to_string(self.item_path(id)).unwrap();
        serde_json::from_str(&text).unwrap()
    }

    pub fn item_path(&self, id: &str) -> PathBuf {
        self.home.path().join("items").join(format!("{id}.json"))
    }

    pub fn read_store_file(&self, name: &str) -> Value {
        let text = fs::read_to_string(self.home.path().join(name)).unwrap();
        serde_json::from_str(&text).unwrap()
    }
}

/// Parses a command's stdout as JSON, showing the output if it doesn't parse.
pub fn parse(stdout: &[u8]) -> Value {
    let text = String::from_utf8_lossy(stdout);
    match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(e) => panic!("stdout isn't JSON ({e}):\n{text}"),
    }
}

/// The schema in `schemas/` that a command's `--json` output must match.
/// A new command needs a schema before its tests can run.
pub fn schema_for(args: &[&str]) -> &'static str {
    match args {
        ["init", ..] => "init",
        ["project", "add", ..] => "project-add",
        ["project", "list", ..] => "project-list",
        ["create", ..] => "create",
        ["edit", ..] => "edit",
        ["list", ..] => "list",
        ["submit", ..] | ["status", _, "done", ..] => "submit",
        ["start" | "shelve" | "reopen" | "status", ..] => "move",
        _ => panic!(
            "No schema for `tracker {}`. Add one to schemas/.",
            args.join(" ")
        ),
    }
}

/// Every schema in `schemas/`, by file name without `.schema.json`.
pub fn schemas() -> Vec<(String, Value)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas");
    let mut schemas = Vec::new();
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let file = path.file_name().unwrap().to_str().unwrap().to_string();
        let Some(name) = file.strip_suffix(".schema.json") else {
            panic!("{} isn't a schema file", path.display());
        };
        let text = fs::read_to_string(&path).unwrap();
        let schema: Value =
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("{file} isn't valid JSON: {e}"));
        schemas.push((name.to_string(), schema));
    }
    schemas.sort_by(|a, b| a.0.cmp(&b.0));
    schemas
}

/// Checks `value` against `schemas/<name>.schema.json`, and panics listing
/// every way it doesn't match. The schemas are loaded into a registry under
/// their `$id`s, so a `$ref` to another schema file resolves without any
/// network access.
pub fn check_schema(name: &str, value: &Value) {
    let schemas = schemas();
    let mut registry = jsonschema::Registry::new();
    for (_, schema) in &schemas {
        let id = schema["$id"].as_str().unwrap();
        registry = registry
            .add(id, jsonschema::Resource::from_contents(schema.clone()))
            .unwrap();
    }
    let registry = registry.prepare().unwrap();

    let Some((_, schema)) = schemas.iter().find(|(n, _)| n == name) else {
        panic!("There's no schemas/{name}.schema.json");
    };
    let validator = jsonschema::options()
        .with_registry(&registry)
        .should_validate_formats(true)
        .build(schema)
        .unwrap_or_else(|e| panic!("schemas/{name}.schema.json can't be used: {e}"));
    let errors: Vec<String> = validator
        .iter_errors(value)
        .map(|e| format!("  at {}: {e}", e.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "This doesn't match schemas/{name}.schema.json:\n{}\n{}",
        errors.join("\n"),
        serde_json::to_string_pretty(value).unwrap()
    );
}

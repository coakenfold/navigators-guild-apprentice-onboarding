//! Shared test setup. Every test gets its own temporary store (`TRACKER_HOME`)
//! and a separate temporary folder for project roots, so no test touches
//! `~/.tracker` or sees another test's items.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

/// `Env::add_criterion`'s check. `$PPID` is the tracker that runs it.
pub const BEFORE_THE_WORK_FAILS: &str =
    "! ps -ww -o args= -p $PPID | grep -qwE 'start|in-progress|criterion'";

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

    /// An empty `TRACKER_HOME`, with no store in it yet. The work folder is
    /// a git repository with one commit, so every project root under it is
    /// a subfolder of one, as TR's is (§2.1). From slice 8, `start`,
    /// `submit`, `gates` and shelving an in-progress item need that.
    pub fn without_store() -> Env {
        let env = Env {
            home: TempDir::new().unwrap(),
            work: TempDir::new().unwrap(),
        };
        fs::create_dir_all(env.outside()).unwrap();
        fs::create_dir_all(env.user_home()).unwrap();
        git_init(env.work.path());
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
        without_colour_settings(&mut cmd);
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
    /// typed it, with no pty crate (PLAN.md). Returns `None` where `script`
    /// isn't installed, so the test can skip.
    pub fn in_terminal(&self, args: &[&str]) -> Option<std::process::Output> {
        let mut cmd = self.script_cmd(&tracker_argv(args));
        cmd.stdin(std::process::Stdio::null());
        match cmd.output() {
            Ok(output) => Some(output),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                eprintln!("Skipped: `script` isn't installed.");
                None
            }
            Err(e) => panic!("Can't run script: {e}"),
        }
    }

    /// Runs `tracker <args>` under `script`, as `in_terminal` does, and
    /// types `answer` and Enter once its output contains `prompt`. Calls
    /// `meanwhile` just before typing, while the command waits. Returns
    /// the terminal's output, which mixes stdout, stderr and the echoed
    /// answer, and whether the command succeeded. `None` where `script`
    /// isn't installed.
    ///
    /// The answer is typed only once the prompt is up, and stdin stays open
    /// until the command ends: on macOS, `script` sends end-of-input as soon
    /// as its stdin closes, and the terminal drops input typed before the
    /// command reads it.
    ///
    /// Panics, after killing the command, if the prompt hasn't appeared or
    /// the command hasn't ended within `TERMINAL_TIMEOUT`. With stdin held
    /// open, a command waiting for an answer to a question worded other than
    /// `prompt` would otherwise never end, and the test would hang.
    pub fn in_terminal_typing(
        &self,
        args: &[&str],
        prompt: &str,
        answer: &str,
        meanwhile: impl FnOnce(),
    ) -> Option<(String, bool)> {
        self.typing(
            &tracker_argv(args),
            prompt,
            answer,
            meanwhile,
            TERMINAL_TIMEOUT,
        )
    }

    /// The same as `in_terminal_typing`, giving up after `timeout`, for
    /// testing the timeout itself.
    pub fn in_terminal_typing_within(
        &self,
        args: &[&str],
        prompt: &str,
        answer: &str,
        timeout: Duration,
    ) -> Option<(String, bool)> {
        self.typing(&tracker_argv(args), prompt, answer, || {}, timeout)
    }

    /// The same as `in_terminal_typing`, for a line of `sh`, in which
    /// `$TRACKER` is the binary. For redirecting stdout to a file, which
    /// leaves stdin on the terminal.
    pub fn in_terminal_typing_sh(
        &self,
        line: &str,
        prompt: &str,
        answer: &str,
    ) -> Option<(String, bool)> {
        let argv = vec!["sh".to_string(), "-c".to_string(), line.to_string()];
        self.typing(&argv, prompt, answer, || {}, TERMINAL_TIMEOUT)
    }

    fn typing(
        &self,
        argv: &[String],
        prompt: &str,
        answer: &str,
        meanwhile: impl FnOnce(),
        timeout: Duration,
    ) -> Option<(String, bool)> {
        use std::io::{Read, Write};
        let mut cmd = self.script_cmd(argv);
        cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit());
        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                eprintln!("Skipped: `script` isn't installed.");
                return None;
            }
            Err(e) => panic!("Can't run script: {e}"),
        };
        let mut stdin = child.stdin.take().unwrap();
        let mut stdout = child.stdout.take().unwrap();

        // A read blocks until there's output, so it happens on its own
        // thread, and this one waits on the channel with a deadline. The
        // channel closes when the output ends.
        let (sender, output) = mpsc::channel();
        std::thread::spawn(move || {
            let mut buffer = [0; 4096];
            while let Ok(n) = stdout.read(&mut buffer) {
                if n == 0 || sender.send(buffer[..n].to_vec()).is_err() {
                    break;
                }
            }
        });
        let deadline = Instant::now() + timeout;

        // Read until the prompt appears, or the command ends without one.
        let mut seen = Vec::new();
        let waiting = format!("the prompt {prompt:?}");
        if read_until(
            &output,
            &mut seen,
            Some(prompt),
            deadline,
            &mut child,
            &waiting,
        ) {
            meanwhile();
            stdin.write_all(format!("{answer}\n").as_bytes()).unwrap();
            stdin.flush().unwrap();
        }
        let waiting = format!("the command to end after {answer:?} was typed");
        read_until(&output, &mut seen, None, deadline, &mut child, &waiting);
        let status = child.wait().unwrap();
        drop(stdin);
        Some((
            String::from_utf8_lossy(&seen).into_owned(),
            status.success(),
        ))
    }

    /// `script` running `argv` with this test's store and `HOME`, and
    /// `TRACKER` set to the binary. Its options differ between macOS and
    /// BSD, and util-linux.
    fn script_cmd(&self, argv: &[String]) -> std::process::Command {
        let mut cmd = std::process::Command::new("script");
        if cfg!(target_os = "linux") {
            let quoted: Vec<String> = argv
                .iter()
                .map(|arg| format!("'{}'", arg.replace('\'', "'\\''")))
                .collect();
            cmd.args(["-qec", &quoted.join(" "), "/dev/null"]);
        } else {
            cmd.args(["-q", "/dev/null"]).args(argv);
        }
        cmd.env("TRACKER", env!("CARGO_BIN_EXE_tracker"))
            .env("TRACKER_HOME", self.home.path())
            .env("HOME", self.user_home())
            // A terminal that shows colour, wherever the tests run.
            .env("TERM", "xterm-256color")
            .current_dir(self.outside());
        without_colour_settings(&mut cmd);
        cmd
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

    /// Creates an item in `project` and returns its ID. It gets one script
    /// criterion, as `add_criterion` adds, so that gate G1 lets it start
    /// and G2 lets it be submitted (slice 6), and the tests of moves still
    /// test moves.
    pub fn create(&self, project: &str, title: &str) -> String {
        let id = self.create_bare(project, title);
        self.add_criterion(&id);
        id
    }

    /// Creates an item in `project` with no criteria, as `tracker create`
    /// does, and returns its ID. G1 won't let it start. Then it commits
    /// everything in the work folder's repository, as `commit_for` does, so
    /// the item has a commit and G5 lets it be submitted (slice 8), as its
    /// tests would be committed before `start`. A project root in a
    /// repository of its own, such as `tests/commits.rs` makes, isn't in
    /// that one, so its items have no commits until the test makes them.
    pub fn create_bare(&self, project: &str, title: &str) -> String {
        let output = self.json(&["create", title, "--project", project]);
        let id = output["item"]["id"].as_str().unwrap().to_string();
        self.commit_for(&id);
        id
    }

    /// Commits everything in the work folder's repository, which holds
    /// every project root under it, as `test(<id>): …`, so item `id` has a
    /// commit and the tree is clean for G5 (slice 8). For a test that wrote
    /// a file in a root after the item was created.
    pub fn commit_for(&self, id: &str) {
        git_commit_all(self.work.path(), &format!("test({id}): the work"));
    }

    /// Gives item `id` a script criterion whose check fails when it's run
    /// before the work, as a check should (§2.4), and passes everywhere
    /// else, so that G1 lets the item start, G2 needs no evidence for it,
    /// G3 passes, and T13 doesn't hold it. The check runs before the work
    /// when `start`, `status <id> in-progress` or a `criterion` command
    /// runs it, so it fails when the tracker that runs it was started with
    /// one of those words.
    pub fn add_criterion(&self, id: &str) {
        self.json(&[
            "criterion",
            "add",
            id,
            "it works",
            "--check",
            BEFORE_THE_WORK_FAILS,
        ]);
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

/// Removes the settings that turn colour on or off whatever the output is
/// (anstream reads them), so a developer's own `NO_COLOR` or
/// `CLICOLOR_FORCE` can't change what a test sees. A test that wants one
/// sets it after this.
fn without_colour_settings(cmd: &mut std::process::Command) {
    for name in ["NO_COLOR", "CLICOLOR", "CLICOLOR_FORCE"] {
        cmd.env_remove(name);
    }
}

/// How long `in_terminal_typing` waits for its prompt and then for the
/// command to end, before it kills the command and fails the test. A
/// command takes well under a second, so this allows for a busy machine.
pub const TERMINAL_TIMEOUT: Duration = Duration::from_secs(20);

/// Adds what `script` prints, as chunks from `output`, to `seen`, until it
/// contains `prompt`, or until the output ends when `prompt` is `None`.
/// Returns whether the prompt appeared: false if the output ended first.
/// Kills `child` and panics, saying what it was `waiting` for and showing
/// the output so far, if neither happens by `deadline`.
fn read_until(
    output: &mpsc::Receiver<Vec<u8>>,
    seen: &mut Vec<u8>,
    prompt: Option<&str>,
    deadline: Instant,
    child: &mut std::process::Child,
    waiting: &str,
) -> bool {
    loop {
        if let Some(prompt) = prompt
            && String::from_utf8_lossy(seen).contains(prompt)
        {
            return true;
        }
        let left = deadline.saturating_duration_since(Instant::now());
        match output.recv_timeout(left) {
            Ok(chunk) => seen.extend_from_slice(&chunk),
            Err(mpsc::RecvTimeoutError::Disconnected) => return false,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!(
                    "Gave up on the terminal, still waiting for {waiting}. Output so far:\n{}",
                    String::from_utf8_lossy(seen)
                );
            }
        }
    }
}

/// Runs `git <args>` in `dir`, expects it to succeed, and returns its
/// stdout without the final line break. It reads no global or system git
/// config, and commits under a test identity without signing, so a
/// developer's own settings can't change what a test does.
pub fn git(dir: &Path, args: &[&str]) -> String {
    let output = git_cmd(dir, args).output().unwrap();
    assert!(
        output.status.success(),
        "`git {}` failed in {}:\n{}",
        args.join(" "),
        dir.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .trim_end()
        .to_string()
}

/// Whether `git <args>` succeeds in `dir`.
pub fn git_ok(dir: &Path, args: &[&str]) -> bool {
    git_cmd(dir, args).output().unwrap().status.success()
}

fn git_cmd(dir: &Path, args: &[&str]) -> std::process::Command {
    let mut cmd = std::process::Command::new("git");
    cmd.current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args([
            "-c",
            "user.name=Tracker Test",
            "-c",
            "user.email=test@tracker.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args);
    cmd
}

/// Makes `dir` a git repository with one commit, so it has a `HEAD`.
pub fn git_init(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    git(dir, &["init", "--quiet"]);
    git(
        dir,
        &["commit", "--quiet", "--allow-empty", "-m", "chore: start"],
    );
}

/// Commits everything under `dir`, in the repository that holds it, with
/// `message`, even if nothing changed. Returns the new commit's SHA.
pub fn git_commit_all(dir: &Path, message: &str) -> String {
    git(dir, &["add", "--all", "."]);
    git(dir, &["commit", "--quiet", "--allow-empty", "-m", message]);
    git(dir, &["rev-parse", "HEAD"])
}

/// The binary followed by `args`.
fn tracker_argv(args: &[&str]) -> Vec<String> {
    let mut argv = vec![env!("CARGO_BIN_EXE_tracker").to_string()];
    argv.extend(args.iter().map(|arg| arg.to_string()));
    argv
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
        ["start" | "shelve" | "reopen" | "reject" | "status", ..] => "move",
        ["approve", ..] => "approve",
        ["review", ..] => "review",
        ["show", ..] => "show",
        ["delete", ..] => "delete",
        ["gates", ..] => "gates",
        ["criterion", "accept", ..] => "criterion-accept",
        ["criterion", ..] | ["evidence", ..] => "criterion",
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

//! Slice 7 (TR-1): running script checks, as DESIGN.md §2.4 and §4.2
//! describe. G3 runs each criterion's check and G4 the project's checks,
//! each in the project's root, stopped after `check_timeout_secs`. `start`
//! runs each criterion check once and records the results.

mod common;

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use common::Env;
use serde_json::{Value, json};

/// A store with project XX, whose root is `<work>/xx`, and that root.
fn env_with_project() -> (Env, PathBuf) {
    let env = Env::new();
    let root = env.add_project("XX", "xx");
    (env, root)
}

/// Sets `field` of project `key` in `config.json` by hand, as §4.2 says
/// settings other than name, root and design are set.
fn set_project(env: &Env, key: &str, field: &str, value: Value) {
    let path = env.home.path().join("config.json");
    let mut config: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    config["projects"][key][field] = value;
    fs::write(&path, serde_json::to_string_pretty(&config).unwrap() + "\n").unwrap();
}

/// Adds a criterion to `id` with the check `check`.
fn add_check(env: &Env, id: &str, text: &str, check: &str) {
    env.json(&["criterion", "add", id, text, "--check", check]);
}

/// A check that prints `line 1` to `line 30`, one per line.
const THIRTY_LINES: &str = "for i in $(seq 1 30); do echo line $i; done";

/// `line 11` to `line 30`: the last 20 lines of `THIRTY_LINES`.
fn last_twenty() -> String {
    (11..=30)
        .map(|i| format!("line {i}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Runs `tracker <args> --json`, expects exit 2, and returns the error
/// object and stderr.
fn refused(env: &Env, args: &[&str]) -> (Value, String) {
    let output = env
        .cmd()
        .args(args)
        .arg("--json")
        .assert()
        .code(2)
        .get_output()
        .clone();
    let error = common::parse(&output.stdout);
    common::check_schema("error", &error);
    (error, String::from_utf8_lossy(&output.stderr).into_owned())
}

/// The failures from gate `gate`.
fn failures_from(error: &Value, gate: &str) -> Vec<Value> {
    error["failures"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["gate"] == gate)
        .cloned()
        .collect()
}

/// Whether gate `gate` is listed in `gates` with `ok`.
fn gate_ok(gates: &Value, gate: &str) -> Option<bool> {
    gates
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["gate"] == gate)
        .map(|g| g["ok"].as_bool().unwrap())
}

#[test]
fn a_failing_check_shows_its_command_exit_code_and_last_lines_of_output() {
    let (env, _) = env_with_project();
    env.create_bare("XX", "a");
    let check = format!("{THIRTY_LINES}; exit 3");
    add_check(&env, "XX-1", "it works", &check);

    // As JSON: the failure names the criterion, and has the command, its
    // exit code and its last 20 lines of output as fields of their own.
    let (error, stderr) = refused(&env, &["gates", "XX-1"]);
    assert_eq!(gate_ok(&error["gates"], "G3"), Some(false), "{error}");
    let g3 = failures_from(&error, "G3");
    assert_eq!(g3.len(), 1, "{error}");
    let failure = &g3[0];
    assert_eq!(failure["criterion"], 1, "{failure}");
    assert_eq!(failure["cmd"], check.as_str(), "{failure}");
    assert_eq!(failure["exit_code"], 3, "{failure}");
    assert_eq!(failure["timed_out"], false, "{failure}");
    assert_eq!(failure["output"], last_twenty().as_str(), "{failure}");
    let message = failure["message"].as_str().unwrap();
    assert!(message.contains(&check), "{message}");
    assert!(message.contains("exit 3"), "{message}");
    // The command is printed before it runs, on stderr, so the JSON on
    // stdout stays one object.
    assert!(stderr.contains(&check), "{stderr}");

    // As text, from a submit: the command, the exit code, and the last
    // lines, each on a line of its own.
    env.json(&["start", "XX-1"]);
    let output = env
        .cmd()
        .args(["submit", "XX-1"])
        .assert()
        .code(2)
        .get_output()
        .clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(&check), "{stderr}");
    assert!(stderr.contains("exit 3"), "{stderr}");
    let lines: Vec<&str> = stderr.lines().map(str::trim).collect();
    for i in 11..=30 {
        let line = format!("line {i}");
        assert!(lines.contains(&line.as_str()), "no {line:?} in:\n{stderr}");
    }
    assert!(!lines.contains(&"line 10"), "more than 20 lines:\n{stderr}");
    assert_eq!(env.item("XX-1")["status"], "in-progress");
}

#[test]
fn a_check_running_longer_than_its_timeout_is_stopped_and_fails() {
    let (env, _) = env_with_project();
    set_project(&env, "XX", "check_timeout_secs", json!(1));
    env.create_bare("XX", "a");
    // `sleep` runs as a child of the shell, so stopping only the shell
    // would leave it holding the output open for 30 seconds.
    let check = "echo before; sleep 30; echo after";
    add_check(&env, "XX-1", "it works", check);

    let began = Instant::now();
    let (error, _) = refused(&env, &["gates", "XX-1"]);
    let took = began.elapsed();
    assert!(took < Duration::from_secs(15), "took {took:?}");

    let g3 = failures_from(&error, "G3");
    assert_eq!(g3.len(), 1, "{error}");
    let failure = &g3[0];
    assert_eq!(failure["criterion"], 1, "{failure}");
    assert_eq!(failure["cmd"], check, "{failure}");
    assert_eq!(failure["timed_out"], true, "{failure}");
    assert_eq!(failure["exit_code"], Value::Null, "{failure}");
    assert_eq!(failure["output"], "before", "{failure}");
    let message = failure["message"].as_str().unwrap();
    assert!(message.contains("stopped"), "{message}");
    assert!(message.contains("check_timeout_secs"), "{message}");
}

#[test]
fn a_broken_project_check_fails_g4_even_when_every_criterion_check_passes() {
    let (env, root) = env_with_project();
    // The criterion check passes only in the project's root.
    fs::write(root.join("marker"), "").unwrap();
    let broken = "no-such-command-for-tracker-tests";
    set_project(&env, "XX", "checks", json!(["test -f marker", broken]));
    env.create_bare("XX", "a");
    add_check(&env, "XX-1", "it works", "test -f marker");

    let (error, stderr) = refused(&env, &["gates", "XX-1"]);
    assert_eq!(gate_ok(&error["gates"], "G3"), Some(true), "{error}");
    assert_eq!(gate_ok(&error["gates"], "G4"), Some(false), "{error}");
    let failures = error["failures"].as_array().unwrap();
    assert_eq!(failures.len(), 1, "{error}");
    let failure = &failures[0];
    assert_eq!(failure["gate"], "G4", "{failure}");
    assert_eq!(failure.get("criterion"), None, "{failure}");
    assert_eq!(failure["cmd"], broken, "{failure}");
    assert_eq!(failure["exit_code"], 127, "{failure}");
    assert!(stderr.contains(broken), "{stderr}");

    // Fixed, every gate passes.
    set_project(&env, "XX", "checks", json!(["test -f marker"]));
    let passed = env.json(&["gates", "XX-1"]);
    assert_eq!(gate_ok(&passed["gates"], "G3"), Some(true), "{passed}");
    assert_eq!(gate_ok(&passed["gates"], "G4"), Some(true), "{passed}");
}

#[test]
fn start_records_a_check_that_fails_and_warns_about_one_that_already_passes() {
    let (env, _) = env_with_project();
    env.add_project("YY", "yy");
    for key in ["XX", "YY"] {
        let id = env.create_bare(key, "a");
        add_check(&env, &id, "not yet", "false");
        add_check(&env, &id, "already", "true");
        env.json(&["criterion", "add", &id, "looks right"]);
    }
    let expected = json!([
        { "criterion": 1, "passed": false },
        { "criterion": 2, "passed": true },
    ]);

    // As JSON: the start goes ahead, records both, and warns about 2.
    let started = env.json(&["start", "XX-1"]);
    assert_eq!(started["item"]["status"], "in-progress", "{started}");
    assert_eq!(started["item"]["started"]["checks"], expected, "{started}");
    assert_eq!(env.item("XX-1")["started"]["checks"], expected);
    let warnings = started["warnings"].as_array().unwrap();
    assert_eq!(warnings.len(), 1, "{started}");
    let warning = warnings[0].as_str().unwrap();
    assert!(warning.contains("Criterion 2"), "{warning}");
    assert!(warning.contains("already passes"), "{warning}");

    // As text, a warning on stderr about criterion 2 alone.
    let output = env
        .cmd()
        .args(["start", "YY-1"])
        .assert()
        .success()
        .get_output()
        .clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let warnings: Vec<&str> = stderr
        .lines()
        .filter(|line| line.starts_with("warning:"))
        .collect();
    assert_eq!(warnings.len(), 1, "{stderr}");
    assert!(warnings[0].contains("Criterion 2"), "{stderr}");
    assert_eq!(env.item("YY-1")["started"]["checks"], expected);
}

#[test]
fn a_passing_checks_last_20_lines_are_kept() {
    let (env, _) = env_with_project();
    set_project(&env, "XX", "checks", json!(["echo project check"]));
    env.create_bare("XX", "a");
    add_check(&env, "XX-1", "it works", THIRTY_LINES);
    env.json(&["start", "XX-1"]);

    let submitted = env.json(&["submit", "XX-1"]);
    let checks = submitted["item"]["verification"]["checks"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(checks.len(), 2, "{submitted}");
    assert_eq!(checks[0]["cmd"], THIRTY_LINES, "{submitted}");
    assert_eq!(checks[0]["ok"], true, "{submitted}");
    assert!(checks[0]["secs"].is_u64(), "{submitted}");
    assert_eq!(checks[0]["output"], last_twenty().as_str(), "{submitted}");
    assert_eq!(checks[1]["cmd"], "echo project check", "{submitted}");
    assert_eq!(checks[1]["output"], "project check", "{submitted}");
    // The file keeps them too, for `show`.
    assert_eq!(env.item("XX-1")["verification"]["checks"], json!(checks));
}

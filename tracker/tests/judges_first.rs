//! Slice 14 (TR-10): judges must fail first, as checks do (DESIGN.md
//! §2.4, §2.5's T11 and T13). G1 refuses `start` while a judge has nothing
//! to grade, `start` runs every judge once, T13 holds an item whose judge
//! already passed at start, and T11 holds one after `max_failures`
//! different failed requests since start.
//!
//! The judge command is `fake`, a shell script in `<work>/judge/`, outside
//! the project's root, that saves each request it gets as
//! `request-<n>.json`, waits if `sleep` exists, and prints `reply.json`.
//!
//! The tests for each of TR-10's criteria are named after it, such as
//! `tr10_c1_`, so each criterion's check runs only its own.

mod common;

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use common::{Env, GUARD_MESSAGE, git_commit_all};
use serde_json::{Value, json};

const MODEL: &str = "claude-haiku-5-5";

/// The code before the work: a stub the judge can grade, and should fail.
const STUB: &str = "fn run() {\n    todo!()\n}\n";

const FAKE: &str = "#!/bin/sh
dir=$(dirname \"$0\")
n=$(ls \"$dir\" | grep -c '^request-')
cat > \"$dir/request-$((n + 1)).json\"
if [ -f \"$dir/sleep\" ]; then sleep \"$(cat \"$dir/sleep\")\"; fi
cat \"$dir/reply.json\"
";

fn judge_dir(env: &Env) -> PathBuf {
    env.work.path().join("judge")
}

fn write(env: &Env, path: &str, text: &str) {
    let path = env.work.path().join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn edit_config(env: &Env, change: impl FnOnce(&mut Value)) {
    let path = env.home.path().join("config.json");
    let mut config: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    change(&mut config);
    fs::write(&path, serde_json::to_string_pretty(&config).unwrap()).unwrap();
}

/// Makes the fake the store's judge command.
fn install_fake(env: &Env) {
    use std::os::unix::fs::PermissionsExt;
    let dir = judge_dir(env);
    fs::create_dir_all(&dir).unwrap();
    let fake = dir.join("fake");
    fs::write(&fake, FAKE).unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
    let command = fake.to_str().unwrap().to_string();
    edit_config(env, |config| {
        config["judge"] = json!({ "command": command, "model": MODEL });
    });
    reply(env, "no");
}

/// Has the fake answer the question `works` with `answer`.
fn reply(env: &Env, answer: &str) {
    let reply = json!({
        "adapter": "fake 1",
        "model": MODEL,
        "answers": { "works": { "answer": answer, "reason": format!("The fake says {answer}.") } },
        "cost_usd": 0
    });
    fs::write(judge_dir(env).join("reply.json"), reply.to_string()).unwrap();
}

/// Has the fake print `text`, which isn't a judge's reply.
fn reply_text(env: &Env, text: &str) {
    fs::write(judge_dir(env).join("reply.json"), text).unwrap();
}

/// How many requests the fake was sent.
fn asked(env: &Env) -> usize {
    match fs::read_dir(judge_dir(env)) {
        Ok(entries) => entries
            .filter(|entry| {
                let name = entry.as_ref().unwrap().file_name();
                name.to_string_lossy().starts_with("request-")
            })
            .count(),
        Err(_) => 0,
    }
}

/// Sets XX's `judge.max_failures` (§4.2).
fn max_failures(env: &Env, max: u64) {
    edit_config(env, |config| {
        config["projects"]["XX"]["judge"] = json!({ "max_failures": max });
    });
}

/// A judge that reads all of `src/lib.rs`.
fn whole_judge() -> Value {
    json!({
        "text": "The code does the work",
        "inputs": ["src/lib.rs"],
        "ask": [{ "id": "works", "ask": "Does this code do the work?" }]
    })
}

/// Adds `judge` to XX-1 as a criterion.
fn add_judge(env: &Env, judge: &Value) {
    let path = judge_dir(env).join("judge.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, judge.to_string()).unwrap();
    env.json(&[
        "criterion",
        "add",
        "XX-1",
        "--judge",
        path.to_str().unwrap(),
    ]);
}

/// Project XX, whose `src/lib.rs` holds `STUB`, and XX-1, created with no
/// commit of its own yet. The fake is installed, and answers no.
fn env_without_commit() -> Env {
    let env = Env::new();
    env.add_project("XX", "xx");
    write(&env, "xx/src/lib.rs", STUB);
    git_commit_all(env.work.path(), "chore: the code before XX-1");
    env.json(&["create", "a", "--project", "XX"]);
    install_fake(&env);
    env
}

/// `env_without_commit`, with XX-1's test commit made and `whole_judge` as
/// its only criterion.
fn with_judge() -> Env {
    let env = env_without_commit();
    env.commit_for("XX-1");
    add_judge(&env, &whole_judge());
    env
}

/// Changes `src/lib.rs` to version `n` of the work, and commits it.
fn work(env: &Env, n: usize) {
    write(
        env,
        "xx/src/lib.rs",
        &format!("fn run() {{\n    step({n});\n}}\n"),
    );
    git_commit_all(env.work.path(), &format!("feat(XX-1): version {n}"));
}

/// The names of the triggers in a submit's or `gates`'s output, with
/// whether each fired.
fn triggers(output: &Value) -> Vec<(String, bool)> {
    output["triggers"]
        .as_array()
        .unwrap_or_else(|| panic!("no triggers: {output}"))
        .iter()
        .map(|ran| {
            (
                ran["trigger"].as_str().unwrap().to_string(),
                ran["fired"] == true,
            )
        })
        .collect()
}

fn fired(output: &Value) -> Vec<String> {
    triggers(output)
        .into_iter()
        .filter(|(_, fired)| *fired)
        .map(|(name, _)| name)
        .collect()
}

/// Asserts that T11 ran on `output` and didn't fire.
fn t11_quiet(output: &Value) {
    assert!(
        triggers(output).contains(&("T11".to_string(), false)),
        "T11 didn't run, or fired: {output}"
    );
}

/// The `started.checks` entries of XX-1, as (criterion, passed).
fn started(env: &Env) -> Vec<(u64, bool)> {
    env.item("XX-1")["started"]["checks"]
        .as_array()
        .map(|checks| {
            checks
                .iter()
                .map(|c| (c["criterion"].as_u64().unwrap(), c["passed"] == true))
                .collect()
        })
        .unwrap_or_default()
}

/// The failed requests `judge/failures.json` counts for XX-1 (T11).
fn counted(env: &Env) -> usize {
    let path = env.home.path().join("judge/failures.json");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let failures: Value = serde_json::from_str(&text).unwrap();
    failures["XX-1"]["failed"]
        .as_array()
        .map_or(0, |failed| failed.len())
}

/// The failures of a refused command, from `gate`.
fn failures_from<'a>(error: &'a Value, gate: &str) -> Vec<&'a Value> {
    error["failures"]
        .as_array()
        .unwrap_or_else(|| panic!("no failures: {error}"))
        .iter()
        .filter(|failure| failure["gate"] == gate)
        .collect()
}

mod tr10_c1 {
    //! `start` refuses a `diff` judge before the item has a commit, and G1
    //! refuses any judge with nothing to grade.
    use super::*;

    fn diff_judge() -> Value {
        json!({
            "text": "The change is small",
            "inputs": ["diff"],
            "ask": [{ "id": "works", "ask": "Is this change small?" }]
        })
    }

    #[test]
    fn tr10_c1_start_refuses_a_diff_judge_before_the_item_has_a_commit() {
        let env = env_without_commit();
        add_judge(&env, &diff_judge());
        let error = env.error_json_in(&env.outside(), &["start", "XX-1"], 2);
        let g1 = failures_from(&error, "G1");
        assert_eq!(g1.len(), 1, "{error}");
        assert_eq!(g1[0]["criterion"], 1, "{error}");
        let message = g1[0]["message"].as_str().unwrap();
        assert!(message.contains("has no commits"), "{message}");
        assert_eq!(env.item("XX-1")["status"], "open");
        assert_eq!(asked(&env), 0, "the judge was asked");
    }

    #[test]
    fn tr10_c1_a_diff_judge_starts_once_the_item_has_a_commit() {
        let env = env_without_commit();
        add_judge(&env, &diff_judge());
        env.commit_for("XX-1");
        env.json(&["start", "XX-1"]);
        assert_eq!(env.item("XX-1")["status"], "in-progress");
    }

    #[test]
    fn tr10_c1_start_refuses_a_judge_whose_each_matches_nothing_now() {
        let env = env_without_commit();
        env.commit_for("XX-1");
        let mut judge = whole_judge();
        judge["each"] = json!("todo!\\(");
        add_judge(&env, &judge);
        write(&env, "xx/src/lib.rs", "fn run() {}\n");
        env.commit_for("XX-1");
        let error = env.error_json_in(&env.outside(), &["start", "XX-1"], 2);
        let g1 = failures_from(&error, "G1");
        assert_eq!(g1.len(), 1, "{error}");
        let message = g1[0]["message"].as_str().unwrap();
        assert!(message.contains("matches nothing"), "{message}");
    }

    #[test]
    fn tr10_c1_start_refuses_a_judge_whose_input_is_gone() {
        let env = with_judge();
        fs::remove_file(env.work.path().join("xx/src/lib.rs")).unwrap();
        env.commit_for("XX-1");
        let error = env.error_json_in(&env.outside(), &["start", "XX-1"], 2);
        let g1 = failures_from(&error, "G1");
        assert_eq!(g1.len(), 1, "{error}");
        let message = g1[0]["message"].as_str().unwrap();
        assert!(message.contains("src/lib.rs"), "{message}");
    }
}

mod tr10_c2 {
    //! A judge whose answers all pass at start is recorded as already
    //! passing and held by T13.
    use super::*;

    #[test]
    fn tr10_c2_a_judge_that_passes_at_start_is_recorded_and_held_by_t13() {
        let env = with_judge();
        reply(&env, "yes");
        let output = env.json(&["start", "XX-1"]);
        assert_eq!(started(&env), [(1, true)]);
        let warnings = output["warnings"].to_string();
        assert!(warnings.contains("Criterion 1's judge"), "{warnings}");
        work(&env, 1);
        let output = env.json(&["submit", "XX-1"]);
        assert_eq!(output["result"], "held", "{output}");
        assert!(fired(&output).contains(&"T13".to_string()), "{output}");
        let hold = output["holds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|hold| hold["trigger"] == "T13")
            .unwrap();
        let message = hold["message"].as_str().unwrap();
        assert!(message.contains("criterion 1's judge"), "{message}");
    }

    #[test]
    fn tr10_c2_a_judge_that_fails_at_start_is_recorded_as_not_passing() {
        let env = with_judge();
        let output = env.json(&["start", "XX-1"]);
        assert_eq!(asked(&env), 1, "start asks the judge once");
        assert_eq!(started(&env), [(1, false)]);
        assert_eq!(output["warnings"], json!([]), "{output}");
        work(&env, 1);
        reply(&env, "yes");
        let output = env.json(&["submit", "XX-1"]);
        assert_eq!(output["result"], "done", "{output}");
    }

    #[test]
    fn tr10_c2_an_unsure_answer_at_start_doesnt_pass() {
        let env = with_judge();
        reply(&env, "unsure");
        env.json(&["start", "XX-1"]);
        assert_eq!(started(&env), [(1, false)]);
    }

    #[test]
    fn tr10_c2_a_failed_call_at_start_lets_the_item_start_as_not_passing() {
        let env = with_judge();
        reply_text(&env, "Sure! The answer is yes.");
        let output = env.json(&["start", "XX-1"]);
        assert_eq!(env.item("XX-1")["status"], "in-progress");
        assert_eq!(started(&env), [(1, false)]);
        let warnings = output["warnings"].to_string();
        assert!(warnings.contains("couldn't be graded"), "{warnings}");
    }

    #[test]
    fn tr10_c2_start_asks_its_judges_without_holding_the_lock() {
        let env = with_judge();
        fs::write(judge_dir(&env).join("sleep"), "4").unwrap();
        let mut start = env
            .std_cmd()
            .args(["start", "XX-1", "--json"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while asked(&env) == 0 {
            if Instant::now() > deadline {
                let _ = start.kill();
                let _ = start.wait();
                panic!("start never asked the judge");
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let began = Instant::now();
        env.json(&["list"]);
        let took = began.elapsed();
        assert!(start.wait().unwrap().success());
        assert!(
            took < Duration::from_secs(3),
            "list waited {took:?} for start's lock"
        );
    }
}

mod tr10_c3 {
    //! T11 holds an item after `max_failures` different failed requests.
    use super::*;

    #[test]
    fn tr10_c3_t11_holds_an_item_after_max_failures_different_failed_requests() {
        let env = with_judge();
        env.json(&["start", "XX-1"]);
        for n in 1..=3 {
            work(&env, n);
            env.error_json_in(&env.outside(), &["gates", "XX-1"], 2);
        }
        work(&env, 4);
        reply(&env, "yes");
        let output = env.json(&["submit", "XX-1"]);
        assert_eq!(output["result"], "held", "{output}");
        assert_eq!(fired(&output), ["T11"], "{output}");
        let message = output["holds"][0]["message"].as_str().unwrap();
        assert!(message.contains('3'), "{message}");
        assert!(message.contains("c1:works"), "{message}");
    }

    #[test]
    fn tr10_c3_fewer_failed_requests_than_max_failures_dont_hold() {
        let env = with_judge();
        env.json(&["start", "XX-1"]);
        for n in 1..=2 {
            work(&env, n);
            env.error_json_in(&env.outside(), &["gates", "XX-1"], 2);
        }
        work(&env, 3);
        reply(&env, "yes");
        let output = env.json(&["submit", "XX-1"]);
        assert_eq!(output["result"], "done", "{output}");
        t11_quiet(&output);
    }

    #[test]
    fn tr10_c3_max_failures_comes_from_the_projects_judge_settings() {
        let env = with_judge();
        max_failures(&env, 1);
        env.json(&["start", "XX-1"]);
        work(&env, 1);
        env.error_json_in(&env.outside(), &["submit", "XX-1"], 2);
        work(&env, 2);
        reply(&env, "yes");
        let output = env.json(&["submit", "XX-1"]);
        assert_eq!(fired(&output), ["T11"], "{output}");
    }

    #[test]
    fn tr10_c3_an_unsure_answer_counts_as_a_failed_request() {
        let env = with_judge();
        max_failures(&env, 1);
        env.json(&["start", "XX-1"]);
        work(&env, 1);
        reply(&env, "unsure");
        env.json(&["gates", "XX-1"]);
        work(&env, 2);
        reply(&env, "yes");
        let output = env.json(&["submit", "XX-1"]);
        assert_eq!(fired(&output), ["T11"], "{output}");
    }

    #[test]
    fn tr10_c3_a_failed_call_doesnt_count() {
        let env = with_judge();
        max_failures(&env, 1);
        env.json(&["start", "XX-1"]);
        work(&env, 1);
        reply_text(&env, "Sure! The answer is yes.");
        env.error_json_in(&env.outside(), &["gates", "XX-1"], 2);
        reply(&env, "yes");
        let output = env.json(&["submit", "XX-1"]);
        assert_eq!(output["result"], "done", "{output}");
        t11_quiet(&output);
    }

    #[test]
    fn tr10_c3_submit_help_names_t11() {
        let env = Env::without_store();
        let output = env.cmd().args(["submit", "--help"]).assert().success();
        let help = String::from_utf8(output.get_output().stdout.clone()).unwrap();
        let named = help
            .split(|c: char| !c.is_ascii_alphanumeric())
            .any(|word| word == "T11");
        assert!(named, "submit --help doesn't name T11:\n{help}");
    }
}

mod tr10_c4 {
    //! An unchanged failure that `gates` repeats counts once, and fails at
    //! start don't count.
    use super::*;

    #[test]
    fn tr10_c4_an_unchanged_failure_that_gates_repeats_counts_once() {
        let env = with_judge();
        max_failures(&env, 2);
        env.json(&["start", "XX-1"]);
        work(&env, 1);
        for _ in 0..3 {
            env.error_json_in(&env.outside(), &["gates", "XX-1"], 2);
        }
        assert_eq!(counted(&env), 1);
        work(&env, 2);
        reply(&env, "yes");
        let output = env.json(&["submit", "XX-1"]);
        assert_eq!(output["result"], "done", "{output}");
        t11_quiet(&output);
    }

    #[test]
    fn tr10_c4_fails_at_and_before_start_dont_count() {
        let env = with_judge();
        max_failures(&env, 1);
        // Before start, on the open item, and at start itself.
        env.error_json_in(&env.outside(), &["gates", "XX-1"], 2);
        env.json(&["start", "XX-1"]);
        // After start, the same request as at start, with the work unchanged.
        env.error_json_in(&env.outside(), &["gates", "XX-1"], 2);
        assert_eq!(counted(&env), 0);
        work(&env, 1);
        reply(&env, "yes");
        let output = env.json(&["submit", "XX-1"]);
        assert_eq!(output["result"], "done", "{output}");
        t11_quiet(&output);
    }
}

mod tr10_c5 {
    //! The test harness puts a `tracker-judge` and a `claude` that fail
    //! first on `PATH`, so no test can reach the installed judge or the
    //! real `claude`.
    use super::*;

    #[test]
    fn tr10_c5_the_harness_puts_failing_judge_commands_first_on_path() {
        let env = Env::new();
        env.add_project("XX", "xx");
        env.create_bare("XX", "a");
        let found = env.outside().join("found");
        let check = format!(
            "command -v tracker-judge > '{0}'; command -v claude >> '{0}'; false",
            found.display()
        );
        env.json(&["criterion", "add", "XX-1", "it works", "--check", &check]);
        env.json(&["start", "XX-1"]);
        let found = fs::read_to_string(&found).unwrap();
        let lines: Vec<&str> = found.lines().collect();
        assert_eq!(lines.len(), 2, "{found}");
        for line in lines {
            let path = PathBuf::from(line);
            assert!(path.starts_with(env.guard_dir()), "{line}");
            let output = Command::new(&path).stdin(Stdio::null()).output().unwrap();
            assert_eq!(output.status.code(), Some(1), "{line}");
            let said = String::from_utf8_lossy(&output.stderr);
            assert!(said.contains(GUARD_MESSAGE), "{said}");
        }
    }

    #[test]
    fn tr10_c5_a_judge_left_at_the_default_command_reaches_only_the_guard() {
        let env = Env::new();
        env.add_project("XX", "xx");
        write(&env, "xx/src/lib.rs", STUB);
        git_commit_all(env.work.path(), "chore: the code before XX-1");
        env.create_bare("XX", "a");
        add_judge(&env, &whole_judge());
        let output = env.json(&["start", "XX-1"]);
        let warnings = output["warnings"].to_string();
        assert!(warnings.contains(GUARD_MESSAGE), "{warnings}");
    }
}

mod tr10_c6 {
    //! DESIGN.md §7's list of commands that run checks or judges names
    //! `approve`, and says which commands run judges.
    use super::*;

    #[test]
    fn tr10_c6_section_7_names_the_commands_that_run_checks_and_judges() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/DESIGN.md");
        let design = fs::read_to_string(path).unwrap();
        let bullet = design
            .lines()
            .find(|line| line.starts_with("- **Interrupting `tracker` can leave a check"))
            .expect("§7's interrupt bullet");
        assert!(
            bullet.contains(
                "Checks run in `start`, `gates`, `submit`, `approve`, `criterion add` and `set` \
                 after start, and shelving an in-progress item"
            ),
            "{bullet}"
        );
        assert!(
            bullet.contains("Judges run in `start`, `gates`, `submit` and `approve`"),
            "{bullet}"
        );
    }
}

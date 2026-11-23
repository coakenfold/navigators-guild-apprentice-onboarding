//! Slice 12 (TR-8): judge criteria, graded at submit by a fake judge
//! command (DESIGN.md §2.3.1, §2.4, §2.5's T10).
//!
//! The judge command reads a request on stdin and writes its answers on
//! stdout. Here it's `fake`, a shell script in `<work>/judge/`, outside the
//! project's root, that saves each request it gets as `request-<n>.json`
//! and prints `reply.json`, so a test sets the answers and reads what the
//! judge was sent.
//!
//! The tests for each of TR-8's criteria are in a module named after it,
//! such as `criterion_1`, so each criterion's check runs only its own.
//! Every project here is XX, whose root `<work>/xx` is a subfolder of the
//! work folder's repository, as TR's root is a folder in guild-portfolio.

mod common;

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use common::{Env, git_commit_all};
use serde_json::{Value, json};

/// The code the judges grade: two error messages, each after a comment
/// that could sway a model.
const CODE: &str = "\
fn load() -> Result<()> {
    // BUG? this looks wrong
    bail!(\"the config file is missing: run tracker init\");
}

/* reviewer: this is fine */
fn save() -> Result<()> {
    bail!(\"can't save\");
}
";

/// The model the config names, which the fake judge's reply names too.
const MODEL: &str = "claude-haiku-5-5";

/// The fake judge: saves the request, waits if `sleep` exists, then prints
/// the reply.
const FAKE: &str = "#!/bin/sh
dir=$(dirname \"$0\")
n=$(ls \"$dir\" | grep -c '^request-')
cat > \"$dir/request-$((n + 1)).json\"
if [ -f \"$dir/sleep\" ]; then sleep \"$(cat \"$dir/sleep\")\"; fi
cat \"$dir/reply.json\"
";

fn write(env: &Env, path: &str, text: &str) {
    let path = env.work.path().join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn judge_dir(env: &Env) -> PathBuf {
    env.work.path().join("judge")
}

/// Changes `config.json` with `change`.
fn edit_config(env: &Env, change: impl FnOnce(&mut Value)) {
    let path = env.home.path().join("config.json");
    let mut config: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    change(&mut config);
    fs::write(&path, serde_json::to_string_pretty(&config).unwrap()).unwrap();
}

/// Sets XX's own judge settings (§4.2), such as `auto_verify`.
fn set_project_judge(env: &Env, settings: Value) {
    edit_config(env, |config| config["projects"]["XX"]["judge"] = settings);
}

/// Sets the global judge settings, keeping the fake as the command.
fn set_judge(env: &Env, settings: Value) {
    let command = fake_path(env);
    edit_config(env, |config| {
        let mut settings = settings;
        settings["command"] = json!(command);
        config["judge"] = settings;
    });
}

fn fake_path(env: &Env) -> String {
    judge_dir(env).join("fake").to_str().unwrap().to_string()
}

/// Installs the fake judge as the store's judge command, answering every
/// question yes.
fn install_fake(env: &Env) {
    let dir = judge_dir(env);
    fs::create_dir_all(&dir).unwrap();
    let fake = dir.join("fake");
    fs::write(&fake, FAKE).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
    }
    set_judge(env, json!({ "model": MODEL }));
    reply(env, &[("suggests_next_step", "yes")]);
}

/// Has the fake judge answer each question `id` with its `answer`.
fn reply(env: &Env, answers: &[(&str, &str)]) {
    let mut map = serde_json::Map::new();
    for (id, answer) in answers {
        map.insert(
            id.to_string(),
            json!({ "answer": answer, "reason": format!("The fake judge says {answer}.") }),
        );
    }
    let reply = json!({ "adapter": "fake 1", "model": MODEL, "answers": map, "cost_usd": 0 });
    reply_text(env, &reply.to_string());
}

/// Has the fake judge print `text`, whatever it is.
fn reply_text(env: &Env, text: &str) {
    fs::write(judge_dir(env).join("reply.json"), text).unwrap();
}

/// Every request the fake judge was sent, oldest first.
fn requests(env: &Env) -> Vec<Value> {
    let mut found = Vec::new();
    for n in 1.. {
        let path = judge_dir(env).join(format!("request-{n}.json"));
        let Ok(text) = fs::read_to_string(path) else {
            break;
        };
        found.push(serde_json::from_str(&text).unwrap());
    }
    found
}

/// Writes a judge file, outside the project's root, and returns its path.
fn judge_file(env: &Env, name: &str, judge: &Value) -> String {
    let path = judge_dir(env).join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, serde_json::to_string_pretty(judge).unwrap()).unwrap();
    path.to_str().unwrap().to_string()
}

/// The error-message judge from §2.3.1, cut to one question, graded once
/// per `bail!` with a line of context either side.
fn each_judge() -> Value {
    json!({
        "text": "Every error message says what to do next",
        "inputs": ["src/errors.rs"],
        "each": "bail!\\(",
        "context_lines": 1,
        "ask": [{
            "id": "suggests_next_step",
            "ask": "Does this error message tell the user what to do next?",
            "yes": "the message gives a concrete next step",
            "no": "the message only reports the problem"
        }]
    })
}

/// A judge that reads all of `src/errors.rs` as one excerpt.
fn whole_judge(questions: Value) -> Value {
    json!({
        "text": "The error messages are helpful",
        "inputs": ["src/errors.rs"],
        "ask": questions
    })
}

/// A store with project XX, whose `src/errors.rs` holds `CODE`, committed
/// unscoped before XX-1 is created and given its test commit. The fake
/// judge is installed and answers yes.
fn env_with_item() -> Env {
    let env = Env::new();
    env.add_project("XX", "xx");
    write(&env, "xx/src/errors.rs", CODE);
    git_commit_all(env.work.path(), "chore: the code before XX-1");
    env.json(&["create", "a", "--project", "XX"]);
    env.commit_for("XX-1");
    install_fake(&env);
    env
}

/// XX-1 with `judge` as its only criterion.
fn with_judge(judge: &Value) -> Env {
    let env = env_with_item();
    let file = judge_file(&env, "judge.json", judge);
    env.json(&["criterion", "add", "XX-1", "--judge", &file]);
    env
}

/// Starts XX-1 and commits its work.
fn start_and_work(env: &Env) {
    env.json(&["start", "XX-1"]);
    git_commit_all(env.work.path(), "feat(XX-1): the work");
}

/// The names of the triggers that fired.
fn fired(output: &Value) -> Vec<String> {
    output["triggers"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|ran| ran["fired"] == true)
        .map(|ran| ran["trigger"].as_str().unwrap().to_string())
        .collect()
}

/// The failures of a refused command's `--json`, from G3.
fn g3_failures(error: &Value) -> Vec<Value> {
    error["failures"]
        .as_array()
        .unwrap_or_else(|| panic!("no failures: {error}"))
        .iter()
        .filter(|failure| failure["gate"] == "G3")
        .cloned()
        .collect()
}

fn refused(env: &Env, args: &[&str]) -> Value {
    env.error_json_in(&env.outside(), args, 2)
}

mod criterion_1 {
    //! Tests cover a pass, a fail and an unsure answer.
    use super::*;

    #[test]
    fn a_yes_passes_and_the_item_is_verified_automatically() {
        let env = with_judge(&each_judge());
        start_and_work(&env);
        let output = env.json(&["submit", "XX-1"]);
        assert_eq!(output["result"], "done", "{output}");
        assert_eq!(requests(&env).len(), 2, "one request per bail!");
        let answers: Vec<&Value> = output["judges"]
            .as_array()
            .unwrap()
            .iter()
            .map(|judge| &judge["answer"])
            .collect();
        assert_eq!(answers, [&json!("yes"), &json!("yes")], "{output}");
    }

    #[test]
    fn a_no_fails_g3_and_refuses_the_submit() {
        let env = with_judge(&each_judge());
        start_and_work(&env);
        reply(&env, &[("suggests_next_step", "no")]);
        let before = fs::read(env.item_path("XX-1")).unwrap();
        let error = refused(&env, &["submit", "XX-1"]);
        let failures = g3_failures(&error);
        assert_eq!(failures.len(), 2, "{error}");
        assert_eq!(failures[0]["criterion"], 1, "{error}");
        assert_eq!(fs::read(env.item_path("XX-1")).unwrap(), before);
    }

    #[test]
    fn an_unsure_answer_holds_the_item_under_t10() {
        let env = with_judge(&each_judge());
        start_and_work(&env);
        reply(&env, &[("suggests_next_step", "unsure")]);
        let output = env.json(&["submit", "XX-1"]);
        assert_eq!(output["result"], "held", "{output}");
        assert_eq!(fired(&output), ["T10"], "{output}");
        let message = output["holds"][0]["message"].as_str().unwrap();
        assert!(message.contains("unsure"), "{message}");
        assert!(
            message.contains("c1:src/errors.rs:3:suggests_next_step"),
            "{message}"
        );
    }
}

mod criterion_2 {
    //! Tests cover a `"pass_when": "no"` question.
    use super::*;

    fn defect_judge() -> Value {
        whole_judge(json!([{
            "id": "hard_codes_a_secret",
            "ask": "Does this code hard-code a secret?",
            "pass_when": "no"
        }]))
    }

    #[test]
    fn a_no_passes_a_pass_when_no_question() {
        let env = with_judge(&defect_judge());
        reply(&env, &[("hard_codes_a_secret", "no")]);
        start_and_work(&env);
        let output = env.json(&["submit", "XX-1"]);
        assert_eq!(output["result"], "done", "{output}");
        assert_eq!(env.item("XX-1")["criteria"][0]["ask"][0]["pass_when"], "no");
    }

    #[test]
    fn a_yes_fails_a_pass_when_no_question() {
        let env = with_judge(&defect_judge());
        reply(&env, &[("hard_codes_a_secret", "yes")]);
        start_and_work(&env);
        let error = refused(&env, &["submit", "XX-1"]);
        let failures = g3_failures(&error);
        assert_eq!(failures.len(), 1, "{error}");
        assert_eq!(failures[0]["answer"], "yes", "{error}");
    }
}

mod criterion_3 {
    //! Tests cover malformed output and a timeout.
    use super::*;

    #[test]
    fn a_reply_that_isnt_json_fails_g3() {
        let env = with_judge(&each_judge());
        reply_text(&env, "Sure! The answer is yes.");
        let error = refused(&env, &["gates", "XX-1"]);
        let failures = g3_failures(&error);
        assert!(!failures.is_empty(), "{error}");
        assert_eq!(failures[0]["criterion"], 1, "{error}");
        let message = failures[0]["message"].as_str().unwrap();
        assert!(message.contains("judge"), "{message}");
    }

    #[test]
    fn an_answer_that_isnt_yes_no_or_unsure_fails_g3() {
        let env = with_judge(&each_judge());
        reply(&env, &[("suggests_next_step", "maybe")]);
        let error = refused(&env, &["gates", "XX-1"]);
        assert!(!g3_failures(&error).is_empty(), "{error}");
    }

    #[test]
    fn a_reply_missing_a_question_fails_g3() {
        let env = with_judge(&each_judge());
        reply(&env, &[("another_question", "yes")]);
        let error = refused(&env, &["gates", "XX-1"]);
        assert!(!g3_failures(&error).is_empty(), "{error}");
    }

    #[test]
    fn a_judge_that_runs_past_its_timeout_is_stopped_and_fails_g3() {
        let env = with_judge(&each_judge());
        set_project_judge(&env, json!({ "timeout_secs": 1 }));
        fs::write(judge_dir(&env).join("sleep"), "20").unwrap();
        let began = Instant::now();
        let error = refused(&env, &["gates", "XX-1"]);
        assert!(began.elapsed() < Duration::from_secs(15), "it waited");
        let failures = g3_failures(&error);
        assert!(!failures.is_empty(), "{error}");
        let message = failures[0]["message"].as_str().unwrap();
        assert!(message.contains("timeout_secs"), "{message}");
    }
}

mod criterion_4 {
    //! Tests cover an oversized excerpt, refused before the judge is
    //! called.
    use super::*;

    #[test]
    fn an_excerpt_over_max_input_bytes_fails_g3_without_calling_the_judge() {
        let env = with_judge(&each_judge());
        set_judge(&env, json!({ "model": MODEL, "max_input_bytes": 20 }));
        let error = refused(&env, &["gates", "XX-1"]);
        let failures = g3_failures(&error);
        assert!(!failures.is_empty(), "{error}");
        let message = failures[0]["message"].as_str().unwrap();
        assert!(message.contains("max_input_bytes"), "{message}");
        assert!(requests(&env).is_empty(), "the judge was called");
    }
}

mod criterion_5 {
    //! Tests cover a cache hit, where the fake judge isn't called again.
    use super::*;

    #[test]
    fn unchanged_work_is_answered_from_the_cache() {
        let env = with_judge(&whole_judge(json!([{
            "id": "suggests_next_step",
            "ask": "Does each error message tell the user what to do next?"
        }])));
        env.json(&["gates", "XX-1"]);
        assert_eq!(requests(&env).len(), 1);
        // A new answer would fail it, but the cached one is used.
        reply(&env, &[("suggests_next_step", "no")]);
        let output = env.json(&["gates", "XX-1"]);
        assert_eq!(requests(&env).len(), 1, "the judge was called again");
        assert_eq!(output["judges"][0]["answer"], "yes", "{output}");
        let cached: Vec<_> = fs::read_dir(env.home.path().join("judge/cache"))
            .unwrap()
            .collect();
        assert_eq!(cached.len(), 1);
    }

    #[test]
    fn changed_work_asks_the_judge_again() {
        let env = with_judge(&whole_judge(json!([{
            "id": "suggests_next_step",
            "ask": "Does each error message tell the user what to do next?"
        }])));
        env.json(&["gates", "XX-1"]);
        write(
            &env,
            "xx/src/errors.rs",
            &CODE.replace("can't save", "nope"),
        );
        git_commit_all(env.work.path(), "feat(XX-1): change a message");
        env.json(&["gates", "XX-1"]);
        assert_eq!(requests(&env).len(), 2);
    }
}

mod criterion_6 {
    //! `--judge` refuses invalid JSON, an invalid regex and an `each` that
    //! matches nothing. `set` refuses a file whose `text` differs.
    use super::*;

    /// Runs `criterion add XX-1 --judge` on `text`, expects exit 1 and an
    /// unchanged item, and returns the error message.
    fn refused_add(env: &Env, text: &str) -> String {
        let path = judge_dir(env).join("bad.json");
        fs::write(&path, text).unwrap();
        let before = fs::read(env.item_path("XX-1")).unwrap();
        let message = env.error(
            &[
                "criterion",
                "add",
                "XX-1",
                "--judge",
                path.to_str().unwrap(),
            ],
            1,
        );
        assert_eq!(fs::read(env.item_path("XX-1")).unwrap(), before);
        message
    }

    #[test]
    fn a_judge_file_that_isnt_json_is_refused() {
        let env = env_with_item();
        let message = refused_add(&env, "{ \"text\": \"a\", ");
        assert!(message.contains("JSON"), "{message}");
    }

    #[test]
    fn an_each_that_isnt_a_regex_is_refused() {
        let env = env_with_item();
        let mut judge = each_judge();
        judge["each"] = json!("bail!(");
        let message = refused_add(&env, &judge.to_string());
        assert!(message.contains("regex"), "{message}");
    }

    #[test]
    fn an_each_that_matches_nothing_is_refused() {
        let env = env_with_item();
        let mut judge = each_judge();
        judge["each"] = json!("panic!\\(");
        let message = refused_add(&env, &judge.to_string());
        assert!(message.contains("matches nothing"), "{message}");
    }

    #[test]
    fn an_each_that_matches_only_a_comment_is_refused() {
        let env = env_with_item();
        let mut judge = each_judge();
        judge["each"] = json!("BUG\\?");
        let message = refused_add(&env, &judge.to_string());
        assert!(message.contains("matches nothing"), "{message}");
    }

    #[test]
    fn an_input_that_doesnt_exist_is_refused() {
        let env = env_with_item();
        let mut judge = each_judge();
        judge["inputs"] = json!(["src/missing.rs"]);
        let message = refused_add(&env, &judge.to_string());
        assert!(message.contains("src/missing.rs"), "{message}");
    }

    #[test]
    fn a_valid_judge_is_copied_into_the_item() {
        let env = env_with_item();
        let file = judge_file(&env, "judge.json", &each_judge());
        env.json(&["criterion", "add", "XX-1", "--judge", &file]);
        let criterion = &env.item("XX-1")["criteria"][0];
        assert_eq!(
            criterion["text"],
            "Every error message says what to do next"
        );
        assert_eq!(criterion["inputs"], json!(["src/errors.rs"]));
        assert_eq!(criterion["each"], "bail!\\(");
        assert_eq!(criterion["ask"][0]["id"], "suggests_next_step");
        assert!(criterion.get("check").is_none(), "{criterion}");
    }

    #[test]
    fn set_refuses_a_judge_whose_text_differs() {
        let env = env_with_item();
        env.json(&["criterion", "add", "XX-1", "Errors say what to do next"]);
        let file = judge_file(&env, "judge.json", &each_judge());
        let before = fs::read(env.item_path("XX-1")).unwrap();
        let message = env.error(&["criterion", "set", "XX-1", "1", "--judge", &file], 1);
        assert!(message.contains("text"), "{message}");
        assert_eq!(fs::read(env.item_path("XX-1")).unwrap(), before);
    }

    #[test]
    fn set_gives_a_criterion_with_the_same_text_a_judge() {
        let env = env_with_item();
        env.json(&[
            "criterion",
            "add",
            "XX-1",
            "Every error message says what to do next",
            "--check",
            "true",
        ]);
        let file = judge_file(&env, "judge.json", &each_judge());
        env.json(&["criterion", "set", "XX-1", "1", "--judge", &file]);
        let criterion = &env.item("XX-1")["criteria"][0];
        assert!(criterion.get("check").is_none(), "{criterion}");
        assert_eq!(criterion["ask"][0]["id"], "suggests_next_step");
    }
}

mod criterion_7 {
    //! G3 fails a judge after a later commit removes every match. Judges
    //! don't run when a script check failed.
    use super::*;

    #[test]
    fn g3_fails_a_judge_once_a_commit_removes_every_match() {
        let env = with_judge(&each_judge());
        env.json(&["start", "XX-1"]);
        write(&env, "xx/src/errors.rs", "fn load() {}\n");
        git_commit_all(env.work.path(), "feat(XX-1): no more bail");
        let error = refused(&env, &["submit", "XX-1"]);
        let failures = g3_failures(&error);
        assert_eq!(failures.len(), 1, "{error}");
        assert_eq!(failures[0]["criterion"], 1, "{error}");
        let message = failures[0]["message"].as_str().unwrap();
        assert!(message.contains("matches nothing"), "{message}");
        assert!(requests(&env).is_empty(), "the judge was called");
    }

    #[test]
    fn judges_dont_run_when_a_script_check_fails() {
        let env = with_judge(&each_judge());
        env.json(&["criterion", "add", "XX-1", "it fails", "--check", "false"]);
        let error = refused(&env, &["gates", "XX-1"]);
        assert!(!g3_failures(&error).is_empty(), "{error}");
        assert!(requests(&env).is_empty(), "the judge was called");
    }

    #[test]
    fn judges_dont_run_when_a_project_check_fails() {
        let env = with_judge(&each_judge());
        edit_config(&env, |config| {
            config["projects"]["XX"]["checks"] = json!(["false"]);
        });
        refused(&env, &["gates", "XX-1"]);
        assert!(requests(&env).is_empty(), "the judge was called");
    }
}

mod criterion_8 {
    //! The fake judge receives the excerpts with comments stripped, and
    //! refs look like `c2:file:line:id`.
    use super::*;

    /// XX-1 with a script criterion first, as `Env::create` gives it, and
    /// `judge` as criterion 2.
    fn second_judge(judge: &Value) -> Env {
        let env = Env::new();
        env.add_project("XX", "xx");
        write(&env, "xx/src/errors.rs", CODE);
        git_commit_all(env.work.path(), "chore: the code before XX-1");
        env.create("XX", "a");
        install_fake(&env);
        let file = judge_file(&env, "judge.json", judge);
        env.json(&["criterion", "add", "XX-1", "--judge", &file]);
        env
    }

    /// Every text the judge was sent in `state`, joined.
    fn sent(env: &Env) -> String {
        requests(env)
            .iter()
            .flat_map(|request| {
                request["state"]
                    .as_object()
                    .unwrap()
                    .values()
                    .map(|value| value.as_str().unwrap().to_string())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn the_judge_gets_each_excerpt_without_its_comments() {
        let env = second_judge(&each_judge());
        env.json(&["gates", "XX-1"]);
        let requests = requests(&env);
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0]["model"], MODEL);
        assert_eq!(
            requests[0]["questions"]["suggests_next_step"]["instructions"],
            "Does this error message tell the user what to do next?"
        );
        let sent = sent(&env);
        assert!(sent.contains("run tracker init"), "{sent}");
        assert!(sent.contains("can't save"), "{sent}");
        assert!(!sent.contains("BUG?"), "{sent}");
        assert!(!sent.contains("reviewer"), "{sent}");
    }

    #[test]
    fn keep_comments_keeps_them() {
        let mut judge = whole_judge(json!([{ "id": "q", "ask": "Are the comments accurate?" }]));
        judge["keep_comments"] = json!(true);
        let env = second_judge(&judge);
        reply(&env, &[("q", "yes")]);
        env.json(&["gates", "XX-1"]);
        let sent = sent(&env);
        assert!(sent.contains("BUG?"), "{sent}");
        assert!(sent.contains("reviewer"), "{sent}");
    }

    #[test]
    fn refs_name_the_criterion_the_match_and_the_question() {
        let env = second_judge(&each_judge());
        let output = env.json(&["gates", "XX-1"]);
        let refs: Vec<&str> = output["judges"]
            .as_array()
            .unwrap()
            .iter()
            .map(|judge| judge["ref"].as_str().unwrap())
            .collect();
        assert_eq!(
            refs,
            [
                "c2:src/errors.rs:3:suggests_next_step",
                "c2:src/errors.rs:8:suggests_next_step"
            ]
        );
    }

    #[test]
    fn a_ref_for_a_whole_input_leaves_out_where() {
        let env = second_judge(&whole_judge(json!([{ "id": "q", "ask": "Is it clear?" }])));
        reply(&env, &[("q", "yes")]);
        let output = env.json(&["gates", "XX-1"]);
        assert_eq!(output["judges"][0]["ref"], "c2:q", "{output}");
    }
}

mod criterion_9 {
    //! `--json` failures carry `ref`, `answer` and `reason`. A done item's
    //! verification record has one judge entry per question.
    use super::*;

    #[test]
    fn a_failed_question_carries_its_ref_answer_and_reason() {
        let env = with_judge(&each_judge());
        reply(&env, &[("suggests_next_step", "no")]);
        let error = refused(&env, &["gates", "XX-1"]);
        let failure = &g3_failures(&error)[0];
        assert_eq!(failure["ref"], "c1:src/errors.rs:3:suggests_next_step");
        assert_eq!(failure["answer"], "no");
        assert_eq!(failure["reason"], "The fake judge says no.");
    }

    #[test]
    fn a_done_items_record_has_one_judge_entry_per_question() {
        let env = with_judge(&whole_judge(json!([
            { "id": "names_problem", "ask": "Does each message name the problem?" },
            { "id": "suggests_next_step", "ask": "Does each message say what to do?" }
        ])));
        reply(
            &env,
            &[("names_problem", "yes"), ("suggests_next_step", "yes")],
        );
        start_and_work(&env);
        env.json(&["submit", "XX-1"]);
        let item = env.item("XX-1");
        let judges = item["verification"]["judges"].as_array().unwrap();
        assert_eq!(judges.len(), 2, "{item}");
        assert_eq!(judges[0]["ref"], "c1:names_problem");
        assert_eq!(judges[1]["ref"], "c1:suggests_next_step");
        for judge in judges {
            assert_eq!(judge["model"], MODEL);
            assert_eq!(judge["answer"], "yes");
            assert_eq!(judge["reason"], "The fake judge says yes.");
            assert_eq!(judge["request"].as_str().unwrap().len(), 64, "{judge}");
        }
    }
}

mod criterion_10 {
    //! `"auto_verify": false` holds an item a judge passed under T10,
    //! which runs on every submit and can't be switched off.
    use super::*;

    #[test]
    fn auto_verify_false_holds_a_judge_passed_item_under_t10() {
        let env = with_judge(&each_judge());
        set_project_judge(&env, json!({ "auto_verify": false }));
        start_and_work(&env);
        let output = env.json(&["submit", "XX-1"]);
        assert_eq!(fired(&output), ["T10"], "{output}");
        let message = output["holds"][0]["message"].as_str().unwrap();
        assert!(message.contains("auto_verify"), "{message}");
    }

    #[test]
    fn t10_cant_be_switched_off() {
        let env = with_judge(&each_judge());
        reply(&env, &[("suggests_next_step", "unsure")]);
        edit_config(&env, |config| {
            config["projects"]["XX"]["review"] = json!({ "disabled": ["T10"] });
        });
        start_and_work(&env);
        let output = env.json(&["submit", "XX-1"]);
        assert_eq!(fired(&output), ["T10"], "{output}");
    }

    #[test]
    fn t10_stays_quiet_for_an_item_with_no_judge() {
        let env = Env::new();
        env.add_project("XX", "xx");
        env.create("XX", "a");
        env.json(&["start", "XX-1"]);
        let output = env.json(&["submit", "XX-1"]);
        let t10: Vec<&Value> = output["triggers"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|ran| ran["trigger"] == "T10")
            .collect();
        assert_eq!(t10, [&json!({ "trigger": "T10", "fired": false })]);
        assert!(output.get("judges").is_none(), "{output}");
    }
}

mod tr45_c1 {
    //! A piped `approve` on an item with a judge criterion is refused for
    //! having no terminal before the judge command is called.
    use super::*;

    #[test]
    fn a_piped_approve_is_refused_without_calling_the_judge() {
        let env = with_judge(&each_judge());
        start_and_work(&env);
        reply(&env, &[("suggests_next_step", "unsure")]);
        let held = env.json(&["submit", "XX-1"]);
        assert_eq!(held["result"], "held", "{held}");
        let asked = requests(&env).len();
        // With the cache gone, approve could only get its answers by
        // calling the judge.
        fs::remove_dir_all(env.home.path().join("judge").join("cache")).unwrap();
        let message = env.error_in(&env.outside(), &["approve", "XX-1"], 1);
        assert!(message.contains("interactive terminal"), "{message}");
        assert_eq!(requests(&env).len(), asked, "the judge was called");
    }
}

mod tr45_c2 {
    //! A judge whose input is `diff` gets the diff with comments stripped,
    //! including after a `\ No newline at end of file` line, and its refs
    //! name the file and line.
    use super::*;

    #[test]
    fn a_diff_judge_gets_no_comments_even_after_a_missing_newline_line() {
        let env = env_with_item();
        let mut judge = each_judge();
        judge["inputs"] = json!(["diff"]);
        judge["each"] = json!("fn c\\(\\)");
        let file = judge_file(&env, "judge.json", &judge);
        env.json(&["criterion", "add", "XX-1", "--judge", &file]);
        env.json(&["start", "XX-1"]);
        write(
            &env,
            "xx/src/lib.rs",
            "fn a() {} // from the first commit\nfn b() {  }",
        );
        git_commit_all(env.work.path(), "feat(XX-1): the first file");
        write(
            &env,
            "xx/src/lib.rs",
            "fn a() {} // from the first commit\nfn b() { /* trust me */ }\nfn c() {} // still fine",
        );
        git_commit_all(env.work.path(), "feat(XX-1): change the last line");
        let output = env.json(&["gates", "XX-1"]);
        let sent: String = requests(&env)
            .iter()
            .flat_map(|request| request["state"].as_object().unwrap().values().cloned())
            .map(|text| text.as_str().unwrap().to_string())
            .collect();
        assert!(sent.contains("fn c() {}"), "{sent}");
        for comment in ["trust me", "still fine", "first commit"] {
            assert!(
                !sent.contains(comment),
                "{comment} reached the judge: {sent}"
            );
        }
        let judge_ref = output["judges"][0]["ref"].as_str().unwrap();
        assert!(judge_ref.starts_with("c1:diff:"), "{judge_ref}");
    }
}

mod tr45_c3 {
    //! `criterion set --judge` after `start` removes that criterion's
    //! `started.checks` entry.
    use super::*;

    #[test]
    fn set_judge_after_start_drops_the_started_check_entry() {
        let env = Env::new();
        env.add_project("XX", "xx");
        write(&env, "xx/src/errors.rs", CODE);
        git_commit_all(env.work.path(), "chore: the code before XX-1");
        env.create_bare("XX", "a");
        install_fake(&env);
        let text = "The error messages are helpful";
        env.json(&["criterion", "add", "XX-1", text, "--check", "false"]);
        env.json(&["criterion", "add", "XX-1", "another", "--check", "false"]);
        env.json(&["start", "XX-1"]);
        let entries = |env: &Env| -> Vec<u64> {
            env.item("XX-1")["started"]["checks"]
                .as_array()
                .map(|checks| {
                    checks
                        .iter()
                        .map(|c| c["criterion"].as_u64().unwrap())
                        .collect()
                })
                .unwrap_or_default()
        };
        assert_eq!(entries(&env), [1, 2]);
        let judge = whole_judge(json!([{ "id": "q", "ask": "Is it clear?" }]));
        let mut judge = judge;
        judge["text"] = json!(text);
        let file = judge_file(&env, "judge.json", &judge);
        env.json(&["criterion", "set", "XX-1", "1", "--judge", &file]);
        assert_eq!(entries(&env), [2]);
    }
}

mod tr45_c4 {
    //! `gates` and `submit` text show an unsure answer's question and
    //! excerpt beside its ref and reason.
    use super::*;

    fn text_of(env: &Env, args: &[&str]) -> String {
        let output = env
            .cmd()
            .current_dir(env.outside())
            .args(args)
            .output()
            .unwrap();
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }

    #[test]
    fn submit_and_gates_text_show_the_question_and_excerpt_of_an_unsure_answer() {
        let env = with_judge(&each_judge());
        start_and_work(&env);
        reply(&env, &[("suggests_next_step", "unsure")]);
        let submitted = text_of(&env, &["submit", "XX-1"]);
        let gates = text_of(&env, &["gates", "XX-1"]);
        for shown in [&submitted, &gates] {
            assert!(
                shown.contains("c1:src/errors.rs:3:suggests_next_step"),
                "{shown}"
            );
            assert!(
                shown.contains("Does this error message tell the user what to do next?"),
                "{shown}"
            );
            assert!(shown.contains("the config file is missing"), "{shown}");
        }
        let review = text_of(&env, &["review"]);
        assert!(review.contains("tracker gates XX-1"), "{review}");
    }
}

mod tr45_c5 {
    //! A judge reply naming a different model, and a judge command that
    //! exits non-zero, each fail G3 with a reason that says so.
    use super::*;

    #[test]
    fn a_reply_naming_another_model_fails_g3() {
        let env = with_judge(&each_judge());
        let reply = json!({
            "adapter": "fake 1", "model": "claude-other-1", "cost_usd": 0,
            "answers": { "suggests_next_step": { "answer": "yes", "reason": "fine" } }
        });
        reply_text(&env, &reply.to_string());
        let error = refused(&env, &["gates", "XX-1"]);
        let failures = g3_failures(&error);
        assert!(!failures.is_empty(), "{error}");
        let message = failures[0]["message"].as_str().unwrap();
        assert!(message.contains("asked for claude-haiku-5-5"), "{message}");
        assert!(message.contains("claude-other-1"), "{message}");
    }

    #[test]
    fn a_judge_command_that_exits_non_zero_fails_g3_with_its_message() {
        let env = with_judge(&each_judge());
        edit_config(&env, |config| {
            config["judge"]["command"] = json!("echo the model is down >&2; exit 1");
        });
        let error = refused(&env, &["gates", "XX-1"]);
        let failures = g3_failures(&error);
        assert!(!failures.is_empty(), "{error}");
        let message = failures[0]["message"].as_str().unwrap();
        assert!(message.contains("exited with 1"), "{message}");
        assert!(message.contains("the model is down"), "{message}");
    }
}

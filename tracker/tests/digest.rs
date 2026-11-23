//! Slice 16 (TR-12): the digest (DESIGN.md §2.6, §2.7). `digest` lists,
//! for each project, every item auto-verified since its digest was last
//! acknowledged, judge-passed ones first, and every shelve, human action and
//! trigger-switching edit, each marked by whether it came from a terminal.
//! `digest ack` records that I've seen it, from a terminal only, once I type
//! `ack`. `review` flags a digest older than `every_days` as overdue.
//!
//! The tests for each of TR-12's criteria are named after it, such as
//! `tr12_c1_`, so each criterion's check runs only its own.

mod common;

use std::fs;
use std::path::PathBuf;

use common::{Env, git_commit_all};
use serde_json::{Value, json};

const MODEL: &str = "claude-haiku-5-5";

/// What `digest ack` asks.
const ACK_PROMPT: &str = "Type ack to confirm";

/// What `delete` and `approve` ask, after the item's ID.
const CONFIRM: &str = "or anything else to cancel: ";

const FAKE: &str = "#!/bin/sh
cat > /dev/null
cat \"$(dirname \"$0\")/reply.json\"
";

fn judge_dir(env: &Env) -> PathBuf {
    env.work.path().join("judge")
}

fn edit_config(env: &Env, change: impl FnOnce(&mut Value)) {
    let path = env.home.path().join("config.json");
    let mut config: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    change(&mut config);
    fs::write(&path, serde_json::to_string_pretty(&config).unwrap()).unwrap();
}

/// Makes a fake judge, which prints `reply.json`, the store's judge command.
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
}

/// Has the fake answer the question `works` with `answer`.
fn reply(env: &Env, answer: &str) {
    let reply = json!({
        "adapter": "fake 1",
        "model": MODEL,
        "answers": { "works": { "answer": answer, "reason": format!("The fake says {answer}.") } },
        "cost_usd": 0,
    });
    fs::write(judge_dir(env).join("reply.json"), reply.to_string()).unwrap();
}

/// Projects XX and YY, with roots `<work>/xx` and `<work>/yy`.
fn two_projects() -> Env {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.add_project("YY", "yy");
    env
}

/// Creates an item in `project` with a script criterion, starts it and
/// submits it, which auto-verifies it. Returns its ID.
fn auto_verified(env: &Env, project: &str, title: &str) -> String {
    let id = env.create(project, title);
    env.json(&["start", &id]);
    let output = env.json(&["submit", &id]);
    assert_eq!(output["result"], "done", "{output}");
    id
}

/// Creates an item in XX whose only criterion is a judge of `src/lib.rs`,
/// which fails at start and passes the work, and auto-verifies it.
fn judge_verified(env: &Env, title: &str) -> String {
    install_fake(env);
    let lib = env.work.path().join("xx/src/lib.rs");
    fs::create_dir_all(lib.parent().unwrap()).unwrap();
    fs::write(&lib, "fn run() {\n    todo!()\n}\n").unwrap();
    git_commit_all(env.work.path(), "chore: the code before the judged item");
    let id = env.create_bare("XX", title);
    let judge = json!({
        "text": "The code does the work",
        "inputs": ["src/lib.rs"],
        "ask": [{ "id": "works", "ask": "Does this code do the work?" }],
    });
    let file = judge_dir(env).join("judge.json");
    fs::write(&file, judge.to_string()).unwrap();
    env.json(&["criterion", "add", &id, "--judge", file.to_str().unwrap()]);
    reply(env, "no");
    env.json(&["start", &id]);
    fs::write(&lib, "fn run() {\n    step();\n}\n").unwrap();
    git_commit_all(env.work.path(), &format!("feat({id}): the work"));
    reply(env, "yes");
    let output = env.json(&["submit", &id]);
    assert_eq!(output["result"], "done", "{output}");
    id
}

/// The one digest in `output`, which must be `project`'s.
fn only_digest<'a>(output: &'a Value, project: &str) -> &'a Value {
    let digests = output["digests"].as_array().unwrap();
    assert_eq!(digests.len(), 1, "{output}");
    assert_eq!(digests[0]["project"], project, "{output}");
    &digests[0]
}

/// The digest of `project`, from `digest --project <project> --json`.
fn digest_of(env: &Env, project: &str) -> Value {
    let output = env.json(&["digest", "--project", project]);
    assert_eq!(output["project"], project, "{output}");
    only_digest(&output, project).clone()
}

/// The IDs of the items a digest lists as auto-verified, in order.
fn verified_ids(digest: &Value) -> Vec<String> {
    digest["verified"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["id"].as_str().unwrap().to_string())
        .collect()
}

/// `digest ack --project <project>` in a terminal, answering `answer`.
/// Returns the terminal's output and whether it succeeded, or `None` where
/// `script` isn't installed.
fn ack(env: &Env, project: &str, answer: &str) -> Option<(String, bool)> {
    env.in_terminal_typing(
        &["digest", "ack", "--project", project],
        ACK_PROMPT,
        answer,
        || {},
    )
}

/// The text of `tracker <args>`, which must exit 0.
fn text(env: &Env, args: &[&str]) -> String {
    let output = env.cmd().args(args).assert().success().get_output().clone();
    String::from_utf8(output.stdout).unwrap()
}

mod tr12_c1 {
    //! `digest` lists every item auto-verified since that project's last
    //! ack, judge-passed first. `digest ack --project XX` empties only XX's
    //! digest.
    use super::*;

    #[test]
    fn tr12_c1_digest_lists_every_auto_verified_item_judge_passed_first() {
        let env = two_projects();
        auto_verified(&env, "XX", "scripts only");
        judge_verified(&env, "judged");
        auto_verified(&env, "YY", "other project");
        // An item that isn't done isn't listed.
        env.create("XX", "still open");

        let digest = digest_of(&env, "XX");
        assert_eq!(digest["acknowledged"], Value::Null, "{digest}");
        assert_eq!(verified_ids(&digest), ["XX-2", "XX-1"], "{digest}");
        let judged = &digest["verified"][0];
        assert_eq!(judged["judged"], true, "{digest}");
        assert_eq!(judged["title"], "judged", "{digest}");
        assert_eq!(judged["criteria"], json!(["The code does the work"]));
        let judges = judged["verification"]["judges"].as_array().unwrap();
        assert_eq!(judges[0]["ref"], "c1:works", "{digest}");
        assert_eq!(judges[0]["answer"], "yes", "{digest}");
        assert_eq!(digest["verified"][1]["judged"], false, "{digest}");
        assert_eq!(digest["verified"][1]["verification"]["by"], "auto");

        // Inside XX's root it covers XX, and outside every root, every
        // project.
        let root = env.work.path().join("xx");
        let inside = env.json_in(&root, &["digest"]);
        assert_eq!(inside["project"], "XX", "{inside}");
        assert_eq!(verified_ids(only_digest(&inside, "XX")), ["XX-2", "XX-1"]);
        let everywhere = env.json(&["digest"]);
        assert_eq!(everywhere["project"], Value::Null, "{everywhere}");
        let projects: Vec<&str> = everywhere["digests"]
            .as_array()
            .unwrap()
            .iter()
            .map(|digest| digest["project"].as_str().unwrap())
            .collect();
        assert_eq!(projects, ["XX", "YY"], "{everywhere}");
        let all = env.json_in(&root, &["digest", "--all"]);
        assert_eq!(all["project"], Value::Null, "{all}");

        // The text lists the judged item first, with its ref and answer.
        let shown = text(&env, &["digest", "--project", "XX"]);
        let judged_at = shown.find("XX-2").unwrap();
        let scripts_at = shown.find("XX-1").unwrap();
        assert!(judged_at < scripts_at, "{shown}");
        assert!(shown.contains("c1:works"), "{shown}");
        assert!(shown.contains("The fake says yes."), "{shown}");
        assert!(shown.contains("never acknowledged"), "{shown}");
    }

    #[test]
    fn tr12_c1_digest_ack_project_empties_only_that_projects_digest() {
        let env = two_projects();
        auto_verified(&env, "XX", "first");
        auto_verified(&env, "YY", "other project");
        let Some((shown, ok)) = ack(&env, "XX", "ack") else {
            return;
        };
        assert!(ok, "{shown}");
        // It showed the digest before asking.
        assert!(shown.contains("XX-1"), "{shown}");

        let xx = digest_of(&env, "XX");
        assert_eq!(verified_ids(&xx), Vec::<String>::new(), "{xx}");
        assert!(xx["acknowledged"].is_string(), "{xx}");
        let yy = digest_of(&env, "YY");
        assert_eq!(verified_ids(&yy), ["YY-1"], "{yy}");
        assert_eq!(yy["acknowledged"], Value::Null, "{yy}");
        let acks = env.read_store_file("digests.json");
        assert!(acks.get("XX").is_some(), "{acks}");
        assert!(acks.get("YY").is_none(), "{acks}");

        // An item auto-verified after the ack is in the next digest.
        auto_verified(&env, "XX", "after the ack");
        assert_eq!(verified_ids(&digest_of(&env, "XX")), ["XX-2"]);
    }
}

mod tr12_c2 {
    //! From a terminal, `digest ack` asks for `ack`. Typing the project
    //! key, or anything else, cancels with exit 1. An item auto-verified in
    //! that project while the question waits makes it exit 1, record
    //! nothing, and leave that item in the next digest.
    use super::*;

    #[test]
    fn tr12_c2_digest_ack_asks_for_ack_and_anything_else_cancels() {
        let env = two_projects();
        auto_verified(&env, "XX", "first");

        // Without a terminal it's refused.
        let error = env.error(&["digest", "ack", "--project", "XX"], 1);
        assert!(error.contains("ack"), "{error}");

        for answer in ["XX", "xx", "XX-1", "nope", ""] {
            let Some((shown, ok)) = ack(&env, "XX", answer) else {
                return;
            };
            assert!(!ok, "{answer:?} acknowledged it:\n{shown}");
            assert!(shown.contains("Cancelled"), "{shown}");
            assert!(!env.home.path().join("digests.json").exists(), "{answer:?}");
            assert_eq!(verified_ids(&digest_of(&env, "XX")), ["XX-1"]);
        }

        // `ack` may be typed in either case.
        let (shown, ok) = ack(&env, "XX", "ACK").unwrap();
        assert!(ok, "{shown}");
        assert_eq!(verified_ids(&digest_of(&env, "XX")), Vec::<String>::new());
    }

    #[test]
    fn tr12_c2_outside_every_root_digest_ack_needs_a_project() {
        let env = two_projects();
        let error = env.error(&["digest", "ack"], 1);
        assert!(error.contains("--project"), "{error}");
        let error = env.error(&["digest", "ack", "--project", "QQ"], 1);
        assert!(error.contains("QQ"), "{error}");
    }

    #[test]
    fn tr12_c2_an_item_verified_while_the_question_waits_stops_the_ack() {
        let env = two_projects();
        auto_verified(&env, "XX", "first");
        let Some((shown, ok)) = env.in_terminal_typing(
            &["digest", "ack", "--project", "XX"],
            ACK_PROMPT,
            "ack",
            || {
                auto_verified(&env, "XX", "while it asked");
            },
        ) else {
            return;
        };
        assert!(!ok, "{shown}");
        assert!(shown.contains("changed"), "{shown}");
        assert!(!env.home.path().join("digests.json").exists());
        let digest = digest_of(&env, "XX");
        assert_eq!(digest["acknowledged"], Value::Null, "{digest}");
        assert_eq!(verified_ids(&digest), ["XX-1", "XX-2"], "{digest}");
    }
}

mod tr12_c3 {
    //! A shelve from piped stdin and one run under `script` (skipped where
    //! `script` isn't installed) are both listed, marked not from a terminal
    //! and from a terminal. So are a delete and an approve made since the
    //! last ack, an edit that removes `needs-review`, and one that changes
    //! kind from `capability` to `change`. An edit that only adds a label
    //! isn't listed.
    use super::*;

    /// The digest's actions about item `id`.
    fn actions_on<'a>(digest: &'a Value, id: &str) -> Vec<&'a Value> {
        digest["actions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|action| action["id"] == id)
            .collect()
    }

    /// The one action about item `id`, which must be `action`.
    fn only_action<'a>(digest: &'a Value, id: &str, action: &str) -> &'a Value {
        let actions = actions_on(digest, id);
        assert_eq!(actions.len(), 1, "{id}: {digest}");
        assert_eq!(actions[0]["action"], action, "{id}: {digest}");
        actions[0]
    }

    #[test]
    fn tr12_c3_a_piped_shelve_is_listed_as_not_from_a_terminal() {
        let env = two_projects();
        env.create("XX", "put away");
        env.json(&["shelve", "XX-1", "--reason", "not now"]);
        let digest = digest_of(&env, "XX");
        let shelve = only_action(&digest, "XX-1", "shelve");
        assert_eq!(shelve["interactive"], false, "{digest}");
        assert_eq!(shelve["reason"], "not now", "{digest}");
        assert_eq!(shelve["title"], "put away", "{digest}");
        let shown = text(&env, &["digest", "--project", "XX"]);
        assert!(shown.contains("not from a terminal"), "{shown}");
    }

    #[test]
    fn tr12_c3_shelves_human_actions_and_trigger_edits_are_listed_marked_by_terminal() {
        let env = two_projects();
        // Skipped where `script` isn't installed.
        if env.in_terminal(&["--version"]).is_none() {
            return;
        }
        // XX-1: shelved from piped stdin. XX-2: shelved from a terminal.
        env.create("XX", "piped shelve");
        env.json(&["shelve", "XX-1", "--reason", "piped"]);
        env.create("XX", "terminal shelve");
        let output = env
            .in_terminal(&["shelve", "XX-2", "--reason", "typed"])
            .unwrap();
        assert!(output.status.success(), "{output:?}");

        // XX-3: deleted from a terminal.
        env.create("XX", "to delete");
        let (shown, ok) = env
            .in_terminal_typing(&["delete", "XX-3"], CONFIRM, "XX-3", || {})
            .unwrap();
        assert!(ok, "{shown}");

        // XX-4: asked for review, held under T9, and approved.
        let id = env.create("XX", "approved");
        env.json(&["edit", &id, "--label", "needs-review"]);
        env.json(&["start", &id]);
        assert_eq!(env.json(&["submit", &id])["result"], "held");
        let (shown, ok) = env
            .in_terminal_typing(&["approve", &id], CONFIRM, &id, || {})
            .unwrap();
        assert!(ok, "{shown}");

        // XX-5: needs-review added, which isn't listed, then removed.
        env.create("XX", "label removed");
        env.json(&["edit", "XX-5", "--label", "needs-review"]);
        env.json(&["edit", "XX-5", "--unlabel", "needs-review"]);
        // XX-6: a capability recorded as a change.
        env.json(&[
            "create",
            "kind changed",
            "--project",
            "XX",
            "--kind",
            "capability",
        ]);
        env.json(&["edit", "XX-6", "--kind", "change"]);
        // XX-7: only a label added, which isn't listed.
        env.create("XX", "label added");
        env.json(&["edit", "XX-7", "--label", "later"]);

        let digest = digest_of(&env, "XX");
        let piped = only_action(&digest, "XX-1", "shelve");
        assert_eq!(piped["interactive"], false, "{digest}");
        let typed = only_action(&digest, "XX-2", "shelve");
        assert_eq!(typed["interactive"], true, "{digest}");
        assert_eq!(typed["reason"], "typed", "{digest}");
        let delete = only_action(&digest, "XX-3", "delete");
        assert_eq!(delete["interactive"], true, "{digest}");
        assert_eq!(delete["status"], "open", "{digest}");
        assert_eq!(delete["title"], "to delete", "{digest}");
        let approve = only_action(&digest, "XX-4", "approve");
        assert_eq!(approve["interactive"], true, "{digest}");
        let unlabel = only_action(&digest, "XX-5", "edit");
        assert_eq!(unlabel["labels_removed"], json!(["needs-review"]));
        assert_eq!(unlabel["interactive"], false, "{digest}");
        let kind = only_action(&digest, "XX-6", "edit");
        assert_eq!(kind["kind_before"], "capability", "{digest}");
        assert!(actions_on(&digest, "XX-7").is_empty(), "{digest}");

        // Oldest first.
        let times: Vec<&str> = digest["actions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|action| action["at"].as_str().unwrap())
            .collect();
        let mut sorted = times.clone();
        sorted.sort();
        assert_eq!(times, sorted, "{digest}");

        let shown = text(&env, &["digest", "--project", "XX"]);
        for id in ["XX-1", "XX-2", "XX-3", "XX-4", "XX-5", "XX-6"] {
            assert!(shown.contains(id), "{id}:\n{shown}");
        }
        assert!(!shown.contains("XX-7"), "{shown}");
        assert!(shown.contains("from a terminal"), "{shown}");
        assert!(shown.contains("needs-review"), "{shown}");
        assert!(shown.contains("capability"), "{shown}");
    }
}

mod tr12_c10 {
    //! `review` flags a project's digest as overdue when its oldest entry
    //! is older than `review.digest.every_days`, and says nothing of a
    //! digest that isn't.
    use super::*;

    #[test]
    fn tr12_c10_review_flags_a_digest_older_than_every_days_as_overdue() {
        let env = two_projects();
        edit_config(&env, |config| {
            config["projects"]["XX"]["review"] = json!({ "digest": { "every_days": 0 } });
        });
        auto_verified(&env, "XX", "first");
        // YY's digest has an entry too, but the default is 7 days.
        auto_verified(&env, "YY", "other project");
        let at = env.item("XX-1")["verification"]["at"].clone();

        let output = env.json(&["review"]);
        assert_eq!(output["held"], json!([]), "{output}");
        assert_eq!(
            output["overdue_digests"],
            json!([{ "project": "XX", "entries": 1, "oldest": at, "acknowledged": null }]),
            "{output}"
        );
        let shown = text(&env, &["review"]);
        assert!(shown.contains("XX's digest is overdue"), "{shown}");
        assert!(shown.contains("tracker digest --project XX"), "{shown}");
        assert!(!shown.contains("YY's digest"), "{shown}");
        assert!(!shown.contains("Nothing needs you."), "{shown}");

        // Once it's acknowledged, it's empty, so not overdue.
        let Some((shown, ok)) = ack(&env, "XX", "ack") else {
            return;
        };
        assert!(ok, "{shown}");
        let output = env.json(&["review"]);
        assert!(output.get("overdue_digests").is_none(), "{output}");
        assert!(text(&env, &["review"]).contains("Nothing needs you."));
    }
}

mod tr12_c11 {
    //! `review`, `approve`, `reject`, `digest`, `digest ack`, `audit` and
    //! `judges` have `--help` text to slice 5's standard, and `digest`,
    //! `audit` and `judges` print an empty state when they've nothing to
    //! report.
    use super::*;

    /// Each command added since slice 5, with its first line of help.
    const COMMANDS: [(&[&str], &str); 7] = [
        (&["review"], "The inbox"),
        (&["approve"], "Approve a held item"),
        (&["reject"], "Reject a held item"),
        (
            &["digest"],
            "What was auto-verified, shelved or done by hand since the last ack",
        ),
        (
            &["digest", "ack"],
            "Record that you've read a project's digest",
        ),
        (&["audit"], "Check the store for signs of hand edits"),
        (
            &["judges"],
            "For each judge question, how often you approved and rejected",
        ),
    ];

    #[test]
    fn tr12_c11_every_command_added_since_slice_5_has_help_text() {
        let env = Env::without_store();
        for (command, about) in COMMANDS {
            for flag in ["--help", "-h"] {
                let output = env.cmd().args(command).arg(flag).assert().success();
                let help = String::from_utf8(output.get_output().stdout.clone()).unwrap();
                let what = format!("tracker {} {flag}", command.join(" "));
                assert!(help.starts_with(about), "{what}:\n{help}");
                assert!(help.contains("Usage: tracker"), "{what}:\n{help}");
                assert!(help.contains("--json"), "{what}:\n{help}");
                // Every option and argument is described, as in
                // tests/polish.rs.
                let lines: Vec<&str> = help.lines().collect();
                for (i, line) in lines.iter().enumerate() {
                    let text = line.trim_start();
                    if !text.starts_with('-') && !text.starts_with('<') {
                        continue;
                    }
                    let beside = text
                        .trim_end()
                        .split("  ")
                        .filter(|part| !part.trim().is_empty())
                        .count()
                        > 1;
                    let indent = line.len() - text.len();
                    let below = flag == "--help"
                        && lines.get(i + 1).is_some_and(|next| {
                            let next_text = next.trim_start();
                            !next_text.is_empty() && next.len() - next_text.len() > indent
                        });
                    assert!(beside || below, "{what}: {line}");
                }
            }
        }
        // The top level names the new commands.
        let output = env.cmd().arg("--help").assert().success();
        let help = String::from_utf8(output.get_output().stdout.clone()).unwrap();
        for name in ["digest", "audit", "judges"] {
            assert!(help.contains(name), "{help}");
        }
    }

    #[test]
    fn tr12_c11_digest_audit_and_judges_print_an_empty_state() {
        let env = two_projects();
        let shown = text(&env, &["digest", "--project", "XX"]);
        assert!(shown.contains("XX's digest is empty"), "{shown}");
        let shown = text(&env, &["digest"]);
        assert!(shown.contains("XX's digest is empty"), "{shown}");
        assert!(shown.contains("YY's digest is empty"), "{shown}");
        assert!(text(&env, &["audit"]).contains("No problems found"));
        assert!(text(&env, &["judges"]).contains("No decisions on judge questions yet"));

        let output = env.json(&["audit"]);
        assert_eq!(output["problems"], json!([]), "{output}");
        let output = env.json(&["judges"]);
        assert_eq!(output["questions"], json!([]), "{output}");
    }
}

//! Slice 9 (TR-5): the inbox, `approve` and `reject` (DESIGN.md §2.5, §2.7,
//! §3). Once every gate passes, the triggers run: T6 holds an item with a
//! manual criterion, and T9 one with the `needs-review` label. A held item
//! waits in review, listed by `tracker review`, until a human approves it,
//! which re-runs the gates, or rejects it, which sends it back to
//! in-progress. `show --diff` prints the item's commits and their diff.
//!
//! The tests for each of TR-5's criteria are in a module named after it,
//! such as `criterion_1`, so each criterion's check runs only its own.
//! `approve` and `reject` need a terminal, so their tests run them under
//! `script` against a throwaway store, as the `delete` tests do. Typing the
//! ID in a real terminal is still checked by hand (PLAN.md).

mod common;

use std::fs;

use common::{Env, git, git_commit_all, git_ok};
use serde_json::{Value, json};

/// The question every human-only command asks.
const PROMPT: &str = "Type XX-1 to confirm";

const START: &str = "tracker start XX-1";
const SUBMIT: &str = "tracker submit XX-1";
const SHELVE: &str = "tracker shelve XX-1 --reason \"…\"";
const APPROVE: &str = "tracker approve XX-1";
const REJECT: &str = "tracker reject XX-1 --reason \"…\"";

/// A store with project XX, whose root is `<work>/xx`, a subfolder of the
/// work folder's repository.
fn env_with_project() -> Env {
    let env = Env::new();
    env.add_project("XX", "xx");
    env
}

/// Item `id`, just created in `project`, with a manual criterion that has
/// evidence, and started, so that its submit passes every gate and T6
/// holds it.
fn start_manual(env: &Env, project: &str) -> String {
    let id = env.create_bare(project, "manual");
    env.json(&["criterion", "add", &id, "the page looks right"]);
    env.json(&["evidence", &id, "1", "see the screenshot in the PR"]);
    env.json(&["start", &id]);
    id
}

/// Item `id`, just created in `project` with a script criterion and the
/// `needs-review` label, and started, so that T9 holds its submit.
fn start_asked_for(env: &Env, project: &str) -> String {
    let id = env.create(project, "asked for");
    env.json(&["edit", &id, "--label", "needs-review"]);
    env.json(&["start", &id]);
    id
}

/// XX-1, held under T9 by its submit. Returns the submit's output.
fn held_xx_1(env: &Env) -> Value {
    let id = start_asked_for(env, "XX");
    assert_eq!(id, "XX-1");
    let output = env.json(&["submit", "XX-1"]);
    assert_eq!(output["result"], "held", "{output}");
    output
}

/// The triggers in a submit's output that fired, in order.
fn fired(output: &Value) -> Vec<String> {
    output["triggers"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|t| t["fired"] == true)
        .map(|t| t["trigger"].as_str().unwrap().to_string())
        .collect()
}

/// Whether `refs/tracker/<id>` exists in the work folder's repository.
fn has_ref(env: &Env, id: &str) -> bool {
    git_ok(
        env.work.path(),
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("refs/tracker/{id}"),
        ],
    )
}

fn history(item: &Value) -> &Vec<Value> {
    item["history"].as_array().unwrap()
}

/// Checks that `item` is held in review by its last move, a submit, with
/// no verification record and no ref, since only reaching done creates
/// them (§2.7).
fn check_held(env: &Env, item: &Value) {
    assert_eq!(item["status"], "review", "{item}");
    assert!(item.get("verification").is_none(), "{item}");
    let entry = history(item).last().unwrap();
    assert_eq!(entry["via"], "submit", "{item}");
    assert_eq!(entry["from"], "in-progress", "{item}");
    assert_eq!(entry["to"], "review", "{item}");
    assert!(
        !has_ref(env, item["id"].as_str().unwrap()),
        "a held item got a ref"
    );
}

/// The commands listed in a refusal's `allowed`, as `(via, to, command)`.
fn allowed(error: &Value) -> Vec<(String, String, String)> {
    let Some(allowed) = error["allowed"].as_array() else {
        panic!("No allowed moves: {error}");
    };
    allowed
        .iter()
        .map(|step| {
            (
                step["via"].as_str().unwrap().to_string(),
                step["to"].as_str().unwrap().to_string(),
                step["command"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

fn step(via: &str, to: &str, command: &str) -> (String, String, String) {
    (via.to_string(), to.to_string(), command.to_string())
}

/// `tracker <args>` with stdin piped, holding `input`, as an agent's shell
/// would run it. Returns the exit code and stdout.
fn piped(env: &Env, args: &[&str], input: &str) -> (i32, String) {
    let output = env.cmd().args(args).write_stdin(input).output().unwrap();
    (
        output.status.code().unwrap(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    )
}

/// `tracker <args>`'s text output, which must exit 0.
fn text(env: &Env, args: &[&str]) -> String {
    let output = env.cmd().args(args).assert().success().get_output().clone();
    String::from_utf8(output.stdout).unwrap()
}

mod criterion_1 {
    //! A manual criterion with evidence is held under T6. A `needs-review`
    //! item is held under T9. An item with neither still auto-verifies.
    use super::*;

    #[test]
    fn a_manual_criterion_with_evidence_is_held_under_t6() {
        let env = env_with_project();
        start_manual(&env, "XX");
        let output = env.json(&["submit", "XX-1"]);
        assert_eq!(output["result"], "held", "{output}");
        assert_eq!(output["status"], "review", "{output}");
        assert_eq!(fired(&output), ["T6"], "{output}");
        let holds = output["holds"].as_array().unwrap();
        assert_eq!(holds.len(), 1, "{output}");
        assert_eq!(holds[0]["trigger"], "T6");
        let message = holds[0]["message"].as_str().unwrap();
        assert!(message.contains("criterion 1"), "{message}");

        let item = env.item("XX-1");
        assert_eq!(item, output["item"]);
        assert_eq!(item["holds"], output["holds"]);
        check_held(&env, &item);
    }

    #[test]
    fn a_needs_review_item_is_held_under_t9() {
        let env = env_with_project();
        start_asked_for(&env, "XX");
        let output = env.json(&["submit", "XX-1"]);
        assert_eq!(output["result"], "held", "{output}");
        assert_eq!(fired(&output), ["T9"], "{output}");
        let holds = output["holds"].as_array().unwrap();
        assert_eq!(holds.len(), 1, "{output}");
        assert_eq!(holds[0]["trigger"], "T9");
        let message = holds[0]["message"].as_str().unwrap();
        assert!(message.contains("needs-review"), "{message}");

        let item = env.item("XX-1");
        assert_eq!(item["holds"], output["holds"]);
        check_held(&env, &item);
    }

    #[test]
    fn an_item_with_neither_still_auto_verifies() {
        let env = env_with_project();
        env.create("XX", "plain");
        env.json(&["start", "XX-1"]);
        let output = env.json(&["submit", "XX-1"]);
        assert_eq!(output["result"], "done", "{output}");
        assert_eq!(output["status"], "done", "{output}");
        // Both triggers ran, and neither fired.
        assert_eq!(
            output["triggers"],
            json!([
                { "trigger": "T1", "fired": false },
                { "trigger": "T2", "fired": false },
                { "trigger": "T3", "fired": false },
                { "trigger": "T4", "fired": false },
                { "trigger": "T5", "fired": false },
                { "trigger": "T6", "fired": false },
                { "trigger": "T7", "fired": false },
                { "trigger": "T8", "fired": false },
                { "trigger": "T9", "fired": false },
                { "trigger": "T10", "fired": false },
                { "trigger": "T13", "fired": false },
                { "trigger": "T14", "fired": false }
            ])
        );
        assert_eq!(output["holds"], json!([]));

        let item = env.item("XX-1");
        assert_eq!(item["status"], "done");
        assert!(item.get("holds").is_none(), "{item}");
        assert_eq!(item["verification"]["by"], "auto");
        assert!(has_ref(&env, "XX-1"));
    }
}

mod criterion_2 {
    //! `submit --json` matches the held shape in §3.
    use super::*;

    #[test]
    fn submit_json_matches_the_held_shape_in_section_3() {
        for args in [&["submit", "XX-1"][..], &["status", "XX-1", "done"]] {
            // Held by both triggers: a manual criterion and the label.
            let env = env_with_project();
            start_manual(&env, "XX");
            env.json(&["edit", "XX-1", "--label", "needs-review"]);
            let output = env.json(args);
            let item = env.item("XX-1");
            let holds = output["holds"].clone();
            assert_eq!(
                output,
                json!({
                    "id": "XX-1",
                    "result": "held",
                    "status": "review",
                    "gates": [
                        { "gate": "G2", "ok": true },
                        { "gate": "G5", "ok": true },
                        { "gate": "G6", "ok": true },
                        { "gate": "G3", "ok": true, "secs": 0 },
                        { "gate": "G4", "ok": true, "secs": 0 }
                    ],
                    "triggers": [
                        { "trigger": "T1", "fired": false },
                        { "trigger": "T2", "fired": false },
                        { "trigger": "T3", "fired": false },
                        { "trigger": "T4", "fired": false },
                        { "trigger": "T5", "fired": false },
                        { "trigger": "T6", "fired": true },
                        { "trigger": "T7", "fired": false },
                        { "trigger": "T8", "fired": false },
                        { "trigger": "T9", "fired": true },
                        { "trigger": "T10", "fired": false },
                        { "trigger": "T13", "fired": false },
                        { "trigger": "T14", "fired": false }
                    ],
                    "holds": holds,
                    "item": item,
                }),
                "{args:?}"
            );
            // In the order §3 gives them, with `item` last.
            let fields: Vec<&String> = output.as_object().unwrap().keys().collect();
            assert_eq!(
                fields,
                [
                    "id", "result", "status", "gates", "triggers", "holds", "item"
                ],
                "{args:?}"
            );
            // Each hold names its trigger and says why.
            let holds = holds.as_array().unwrap();
            let names: Vec<&str> = holds
                .iter()
                .map(|h| h["trigger"].as_str().unwrap())
                .collect();
            assert_eq!(names, ["T6", "T9"], "{args:?}");
            for hold in holds {
                let fields: Vec<&String> = hold.as_object().unwrap().keys().collect();
                assert_eq!(fields, ["trigger", "message"], "{hold}");
                assert!(!hold["message"].as_str().unwrap().is_empty(), "{hold}");
            }
            assert_eq!(item["holds"], output["holds"], "{args:?}");
        }
    }
}

mod criterion_3 {
    //! `review` lists held items with their reasons and the `show --diff`
    //! hint, and prints "Nothing needs you." when it's empty.
    use super::*;

    #[test]
    fn the_inbox_lists_held_items_with_their_reasons_and_the_diff_hint() {
        let env = env_with_project();
        env.add_project("YY", "yy");
        held_xx_1(&env);
        start_manual(&env, "YY");
        env.json(&["submit", "YY-1"]);
        // Neither an open item nor a done one is listed.
        env.create("XX", "open");
        let xx = env.item("XX-1");
        let yy = env.item("YY-1");

        // It covers every project, even inside one project's root.
        let root = env.work.path().join("xx");
        for output in [env.json(&["review"]), env.json_in(&root, &["review"])] {
            assert_eq!(
                output,
                json!({
                    "held": [
                        {
                            "id": "XX-1",
                            "project": "XX",
                            "title": "asked for",
                            "holds": xx["holds"],
                            "diff": "tracker show XX-1 --diff",
                        },
                        {
                            "id": "YY-1",
                            "project": "YY",
                            "title": "manual",
                            "holds": yy["holds"],
                            "diff": "tracker show YY-1 --diff",
                        }
                    ],
                    "skipped": [],
                })
            );
        }

        let shown = text(&env, &["review"]);
        for id in ["XX-1", "YY-1"] {
            assert!(shown.contains(id), "{shown}");
            assert!(
                shown.contains(&format!("tracker show {id} --diff")),
                "{shown}"
            );
        }
        for item in [&xx, &yy] {
            let hold = &item["holds"][0];
            assert!(shown.contains(hold["trigger"].as_str().unwrap()), "{shown}");
            assert!(shown.contains(hold["message"].as_str().unwrap()), "{shown}");
        }
        assert!(!shown.contains("XX-2"), "{shown}");
        assert!(!shown.contains("Nothing needs you."), "{shown}");
    }

    #[test]
    fn the_inbox_says_nothing_needs_you_when_its_empty() {
        let env = env_with_project();
        assert_eq!(env.json(&["review"]), json!({ "held": [], "skipped": [] }));
        assert!(text(&env, &["review"]).contains("Nothing needs you."));

        // Items that aren't held don't count.
        env.create("XX", "open");
        env.create("XX", "done");
        env.json(&["start", "XX-2"]);
        env.json(&["submit", "XX-2"]);
        assert_eq!(env.json(&["review"]), json!({ "held": [], "skipped": [] }));
        assert!(text(&env, &["review"]).contains("Nothing needs you."));
    }
}

mod criterion_4 {
    //! `echo XX-1 | tracker approve XX-1` is refused. Typing the ID in a
    //! real terminal moves the item to done (checked by hand).
    //!
    //! The real terminal is the user's to try. Here `script` stands in for
    //! it, against a throwaway store.
    use super::*;

    #[test]
    fn approve_from_piped_stdin_is_refused() {
        let env = env_with_project();
        held_xx_1(&env);
        let before = fs::read(env.item_path("XX-1")).unwrap();

        let (code, stdout) = piped(&env, &["approve", "XX-1", "--json"], "XX-1\n");
        assert_eq!(code, 1, "{stdout}");
        let error = common::parse(stdout.as_bytes());
        common::check_schema("error", &error);
        let message = error["error"].as_str().unwrap();
        assert!(message.contains("interactive terminal"), "{message}");

        let (code, _) = piped(&env, &["approve", "XX-1"], "XX-1\n");
        assert_eq!(code, 1);
        assert_eq!(fs::read(env.item_path("XX-1")).unwrap(), before);
        assert!(!has_ref(&env, "XX-1"));
    }

    #[test]
    fn approve_typed_in_a_terminal_moves_the_item_to_done() {
        let env = env_with_project();
        let submitted = held_xx_1(&env);
        // A commit made while it waits: approve records the HEAD it saw,
        // not the submit's.
        fs::write(env.work.path().join("xx/notes.txt"), "notes\n").unwrap();
        let head = git_commit_all(env.work.path(), "docs(XX-1): add notes");

        let Some((output, ok)) =
            env.in_terminal_typing(&["approve", "XX-1"], PROMPT, "XX-1", || {})
        else {
            return;
        };
        assert!(ok, "{output}");
        // The question says what holds it.
        assert!(output.contains("T9"), "{output}");

        let item = env.item("XX-1");
        assert_eq!(item["status"], "done", "{item}");
        assert!(item.get("holds").is_none(), "{item}");
        let record = &item["verification"];
        assert_eq!(record["by"], "human", "{record}");
        assert_eq!(record["commit"], head.as_str(), "{record}");
        assert_eq!(record["ref"], "refs/tracker/XX-1", "{record}");
        assert_eq!(record["triggers"], submitted["holds"], "{record}");
        assert_eq!(record["checks"][0]["criterion"], 1, "{record}");
        assert_eq!(
            git(env.work.path(), &["rev-parse", "refs/tracker/XX-1"]),
            head
        );
        let entry = history(&item).last().unwrap();
        assert_eq!(entry["via"], "approve", "{item}");
        assert_eq!(entry["from"], "review", "{item}");
        assert_eq!(entry["to"], "done", "{item}");
        assert_eq!(entry["interactive"], true, "{item}");
        common::check_schema("item", &item);
    }

    #[test]
    fn approve_typed_with_another_id_changes_nothing() {
        let env = env_with_project();
        held_xx_1(&env);
        let before = fs::read(env.item_path("XX-1")).unwrap();
        let Some((output, ok)) =
            env.in_terminal_typing(&["approve", "XX-1"], PROMPT, "XX-2", || {})
        else {
            return;
        };
        assert!(!ok, "{output}");
        assert_eq!(fs::read(env.item_path("XX-1")).unwrap(), before);
        assert!(!has_ref(&env, "XX-1"));
    }
}

mod criterion_5 {
    //! An approve whose gate re-run fails leaves the item in review. A
    //! reject puts its reason in history.
    use super::*;

    #[test]
    fn an_approve_whose_gate_rerun_fails_leaves_the_item_in_review() {
        let env = env_with_project();
        held_xx_1(&env);
        // The project now has an uncommitted file, so G5 fails.
        fs::write(env.work.path().join("xx/stray.txt"), "stray\n").unwrap();
        let before = fs::read(env.item_path("XX-1")).unwrap();

        let Some((output, ok)) =
            env.in_terminal_typing(&["approve", "XX-1"], PROMPT, "XX-1", || {})
        else {
            return;
        };
        assert!(!ok, "{output}");
        assert!(output.contains("G5"), "{output}");
        assert!(output.contains("stays in review"), "{output}");
        assert_eq!(fs::read(env.item_path("XX-1")).unwrap(), before);
        assert_eq!(env.item("XX-1")["status"], "review");
        assert!(!has_ref(&env, "XX-1"));
    }

    #[test]
    fn a_reject_puts_its_reason_in_history() {
        let env = env_with_project();
        held_xx_1(&env);
        let reason = "The tests miss the empty case.";
        let Some((output, ok)) = env.in_terminal_typing(
            &["reject", "XX-1", "--reason", reason],
            PROMPT,
            "XX-1",
            || {},
        ) else {
            return;
        };
        assert!(ok, "{output}");

        let item = env.item("XX-1");
        assert_eq!(item["status"], "in-progress", "{item}");
        assert!(item.get("holds").is_none(), "{item}");
        assert!(item.get("verification").is_none(), "{item}");
        let entry = history(&item).last().unwrap();
        assert_eq!(
            entry,
            &json!({
                "at": entry["at"],
                "from": "review",
                "to": "in-progress",
                "via": "reject",
                "interactive": true,
                "reason": reason,
            })
        );
        common::check_schema("item", &item);

        // The agent can submit again, and the label still holds it.
        let output = env.json(&["submit", "XX-1"]);
        assert_eq!(output["result"], "held", "{output}");
    }

    #[test]
    fn reject_from_piped_stdin_is_refused() {
        let env = env_with_project();
        held_xx_1(&env);
        let before = fs::read(env.item_path("XX-1")).unwrap();
        let message = env.error(&["reject", "XX-1", "--reason", "no"], 1);
        assert!(message.contains("interactive terminal"), "{message}");
        // A missing reason is refused before anything is asked.
        let message = env.error(&["reject", "XX-1"], 1);
        assert!(message.contains("--reason"), "{message}");
        assert_eq!(fs::read(env.item_path("XX-1")).unwrap(), before);
    }
}

mod criterion_6 {
    //! Every move refused for an item in review, through a command or
    //! through `status`, names `approve` and `reject`, and its `allowed`
    //! lists both.
    use super::*;

    #[test]
    fn every_move_refused_for_an_item_in_review_names_approve_and_reject() {
        let env = env_with_project();
        held_xx_1(&env);
        let before = fs::read(env.item_path("XX-1")).unwrap();
        for args in [
            &["start", "XX-1"][..],
            &["submit", "XX-1"],
            &["shelve", "XX-1", "--reason", "r"],
            &["reopen", "XX-1", "--reason", "r"],
            &["status", "XX-1", "done"],
            &["status", "XX-1", "open", "--reason", "r"],
            &["status", "XX-1", "in-progress"],
            &["status", "XX-1", "shelved", "--reason", "r"],
            &["status", "XX-1", "review"],
        ] {
            let error = env.error_json_in(&env.outside(), args, 1);
            let message = error["error"].as_str().unwrap();
            assert!(message.contains(APPROVE), "{args:?}: {message}");
            assert!(
                message.contains("tracker reject XX-1"),
                "{args:?}: {message}"
            );
            assert_eq!(
                allowed(&error),
                [
                    step("approve", "done", APPROVE),
                    step("reject", "in-progress", REJECT)
                ],
                "{args:?}"
            );
            assert_eq!(fs::read(env.item_path("XX-1")).unwrap(), before, "{args:?}");
        }
    }
}

mod criterion_7 {
    //! `show XX-1 --diff` includes XX-1's commits and none of XX-12's.
    use super::*;

    #[test]
    fn show_diff_includes_xx_1s_commits_and_none_of_xx_12s() {
        let env = env_with_project();
        env.create_bare("XX", "a");
        let work = env.work.path();
        let first = git(work, &["rev-parse", "HEAD"]);
        fs::write(work.join("xx/one.txt"), "the first one\n").unwrap();
        let second = git_commit_all(work, "feat(XX-1): add one");
        fs::write(work.join("xx/twelve.txt"), "the twelfth\n").unwrap();
        git_commit_all(work, "feat(XX-12): add twelve");
        // A file outside the root is still one of its changed files (§2.1).
        fs::create_dir_all(work.join("other")).unwrap();
        fs::write(work.join("other/outside.txt"), "outside the root\n").unwrap();
        let third = git_commit_all(work, "fix(XX-1)!: add a note outside the root");

        let output = env.json(&["show", "XX-1", "--diff"]);
        assert_eq!(output["item"], env.item("XX-1"));
        assert_eq!(
            output["commits"],
            json!([
                { "sha": third, "header": "fix(XX-1)!: add a note outside the root" },
                { "sha": second, "header": "feat(XX-1): add one" },
                { "sha": first, "header": "test(XX-1): the work" },
            ])
        );
        let diff = output["diff"].as_str().unwrap();
        assert!(diff.contains("xx/one.txt"), "{diff}");
        assert!(diff.contains("+the first one"), "{diff}");
        assert!(diff.contains("other/outside.txt"), "{diff}");
        assert!(!diff.contains("twelve"), "{diff}");
        assert!(!diff.contains("XX-12"), "{diff}");

        let shown = text(&env, &["show", "XX-1", "--diff"]);
        assert!(shown.contains("feat(XX-1): add one"), "{shown}");
        assert!(shown.contains("+the first one"), "{shown}");
        assert!(!shown.contains("XX-12"), "{shown}");
        assert!(!shown.contains("twelve"), "{shown}");

        // Without --diff, neither is printed.
        let output = env.json(&["show", "XX-1"]);
        assert!(output.get("commits").is_none(), "{output}");
        assert!(output.get("diff").is_none(), "{output}");
    }
}

mod criterion_8 {
    //! A refused move from in-progress lists `submit` twice in `allowed`,
    //! once to done and once to review, with the same command.
    use super::*;

    #[test]
    fn a_refused_move_from_in_progress_lists_submit_to_done_and_to_review() {
        let env = env_with_project();
        env.create("XX", "a");
        env.json(&["start", "XX-1"]);
        for args in [
            &["start", "XX-1"][..],
            &["status", "XX-1", "in-progress"],
            &["reopen", "XX-1", "--reason", "r"],
            &["status", "XX-1", "review"],
            &["approve", "XX-1"],
            &["reject", "XX-1", "--reason", "r"],
        ] {
            let error = env.error_json_in(&env.outside(), args, 1);
            assert_eq!(
                allowed(&error),
                [
                    step("submit", "done", SUBMIT),
                    step("submit", "review", SUBMIT),
                    step("shelve", "shelved", SHELVE),
                ],
                "{args:?}"
            );
        }
    }
}

// Beyond the criteria: what else slice 9's prompt asks for, and how it
// settles slice 8 review 2's "Slice 9 must decide" list.

#[test]
fn approve_on_an_item_that_isnt_in_review_is_refused_with_its_allowed_moves() {
    let env = env_with_project();
    env.create("XX", "a");
    let error = env.error_json_in(&env.outside(), &["approve", "XX-1"], 1);
    assert_eq!(
        allowed(&error),
        [
            step("start", "in-progress", START),
            step("shelve", "shelved", SHELVE)
        ]
    );
}

#[test]
fn approve_writes_nothing_when_a_check_edits_the_item_file() {
    // §4.4: approve lets the lock go while the gates' checks run, then
    // reads the item again, as submit does.
    let env = env_with_project();
    let go = env.outside().join("go");
    let check = format!(
        "if [ -f '{}' ]; then printf ' ' >> '{}'; fi",
        go.display(),
        env.item_path("XX-1").display()
    );
    env.create_bare("XX", "a");
    env.json(&["criterion", "add", "XX-1", "it works", "--check", &check]);
    env.json(&["edit", "XX-1", "--label", "needs-review"]);
    env.json(&["start", "XX-1"]);
    let output = env.json(&["submit", "XX-1"]);
    assert_eq!(output["result"], "held", "{output}");
    fs::write(&go, "").unwrap();

    let Some((output, ok)) = env.in_terminal_typing(&["approve", "XX-1"], PROMPT, "XX-1", || {})
    else {
        return;
    };
    assert!(!ok, "{output}");
    assert!(output.contains("changed while its checks ran"), "{output}");
    let item = env.item("XX-1");
    assert_eq!(item["status"], "review", "{item}");
    assert!(item.get("verification").is_none(), "{item}");
    assert!(!has_ref(&env, "XX-1"));
}

#[test]
fn a_held_item_pauses_its_project_until_its_decided() {
    let env = env_with_project();
    held_xx_1(&env);
    env.create("XX", "next");
    let error = env.error_json_in(&env.outside(), &["start", "XX-2"], 2);
    let message = error["failures"][0]["message"].as_str().unwrap();
    assert!(message.contains("XX-1 is in review"), "{message}");
}

mod tr_27 {
    //! Slice 9 review 1, bugs 1 and 2: the inbox's heading reads for any
    //! number of items, and `show --diff` outside a git work tree says it
    //! can't show the item's commits.
    use super::*;

    #[test]
    fn the_inbox_heading_doesnt_call_two_items_it() {
        let env = env_with_project();
        env.add_project("YY", "yy");
        held_xx_1(&env);
        start_manual(&env, "YY");
        env.json(&["submit", "YY-1"]);

        let shown = text(&env, &["review"]);
        let heading = shown.lines().next().unwrap();
        assert!(heading.starts_with("2 items are held"), "{shown}");
        assert!(!heading.contains(" it:"), "{heading}");
        assert!(!heading.ends_with(" it"), "{heading}");
        assert!(heading.contains("approve or reject each one"), "{heading}");
    }

    #[test]
    fn show_diff_outside_a_git_work_tree_says_it_cant_show_the_commits() {
        // The work folder is a repository (`Env`), so this root is elsewhere.
        let env = Env::new();
        let elsewhere = tempfile::TempDir::new().unwrap();
        let root = elsewhere.path().join("xx");
        fs::create_dir_all(&root).unwrap();
        env.json(&[
            "project",
            "add",
            "XX",
            "--name",
            "XX project",
            "--root",
            root.to_str().unwrap(),
        ]);
        env.create_bare("XX", "a");
        let before = fs::read(env.item_path("XX-1")).unwrap();

        let message = env.error(&["show", "XX-1", "--diff"], 3);
        assert!(message.contains("Can't show XX-1's commits"), "{message}");
        assert!(
            message.contains("isn't inside a git work tree"),
            "{message}"
        );
        assert!(!message.contains("record or prove"), "{message}");
        assert_eq!(fs::read(env.item_path("XX-1")).unwrap(), before);
    }
}

mod tr_28 {
    //! Slice 9 review 1, security 1: `show --diff` prints a file's real
    //! changed lines, even with a textconv filter set for it in
    //! `.git/config`.
    use super::*;

    #[test]
    fn show_diff_ignores_a_textconv_filter() {
        let env = env_with_project();
        env.create_bare("XX", "a");
        let work = env.work.path();
        fs::write(work.join("xx/.gitattributes"), "*.sh diff=quiet\n").unwrap();
        fs::write(
            work.join("xx/work.sh"),
            "ok line\nrm -rf the_tests # hidden\n",
        )
        .unwrap();
        git_commit_all(work, "feat(XX-1): add work.sh");
        git(work, &["config", "diff.quiet.textconv", "grep -v hidden"]);

        let output = env.json(&["show", "XX-1", "--diff"]);
        let diff = output["diff"].as_str().unwrap();
        assert!(diff.contains("+ok line"), "{diff}");
        assert!(diff.contains("+rm -rf the_tests # hidden"), "{diff}");

        let shown = text(&env, &["show", "XX-1", "--diff"]);
        assert!(shown.contains("+rm -rf the_tests # hidden"), "{shown}");
    }
}

mod tr_29 {
    //! Slice 9 review 1, design 2: once every gate passes, the `gates` dry
    //! run runs the triggers and lists them and their holds, as §3 says, so
    //! an agent can tell before it submits that the submit will be held.
    use super::*;

    #[test]
    fn gates_lists_the_holds_a_submit_would_get() {
        let env = env_with_project();
        start_manual(&env, "XX");
        let before = fs::read(env.item_path("XX-1")).unwrap();

        let output = env.json(&["gates", "XX-1"]);
        assert_eq!(output["result"], "passed", "{output}");
        assert_eq!(
            output["triggers"],
            json!([
                { "trigger": "T1", "fired": false },
                { "trigger": "T2", "fired": false },
                { "trigger": "T3", "fired": false },
                { "trigger": "T4", "fired": false },
                { "trigger": "T5", "fired": false },
                { "trigger": "T6", "fired": true },
                { "trigger": "T7", "fired": false },
                { "trigger": "T8", "fired": false },
                { "trigger": "T9", "fired": false },
                { "trigger": "T10", "fired": false },
                { "trigger": "T13", "fired": false },
                { "trigger": "T14", "fired": false }
            ]),
            "{output}"
        );
        let holds = output["holds"].as_array().unwrap();
        assert_eq!(holds.len(), 1, "{output}");
        assert_eq!(holds[0]["trigger"], "T6", "{output}");
        let message = holds[0]["message"].as_str().unwrap();
        assert!(message.contains("criterion 1"), "{message}");
        // In the order §3 gives them.
        let fields: Vec<&String> = output.as_object().unwrap().keys().collect();
        assert_eq!(
            fields,
            ["id", "status", "result", "gates", "triggers", "holds"],
            "{output}"
        );

        let shown = text(&env, &["gates", "XX-1"]);
        assert!(shown.contains("held for review"), "{shown}");
        assert!(shown.contains("T6"), "{shown}");
        assert!(shown.contains(message), "{shown}");

        // A dry run changes nothing, and the submit gets the same holds.
        assert_eq!(fs::read(env.item_path("XX-1")).unwrap(), before);
        let submitted = env.json(&["submit", "XX-1"]);
        assert_eq!(submitted["holds"], output["holds"], "{submitted}");
        assert_eq!(submitted["triggers"], output["triggers"], "{submitted}");
    }

    #[test]
    fn gates_says_no_holds_when_no_trigger_fires() {
        let env = env_with_project();
        env.create("XX", "plain");
        env.json(&["start", "XX-1"]);
        let before = fs::read(env.item_path("XX-1")).unwrap();

        let output = env.json(&["gates", "XX-1"]);
        assert_eq!(output["result"], "passed", "{output}");
        assert_eq!(
            output["triggers"],
            json!([
                { "trigger": "T1", "fired": false },
                { "trigger": "T2", "fired": false },
                { "trigger": "T3", "fired": false },
                { "trigger": "T4", "fired": false },
                { "trigger": "T5", "fired": false },
                { "trigger": "T6", "fired": false },
                { "trigger": "T7", "fired": false },
                { "trigger": "T8", "fired": false },
                { "trigger": "T9", "fired": false },
                { "trigger": "T10", "fired": false },
                { "trigger": "T13", "fired": false },
                { "trigger": "T14", "fired": false }
            ]),
            "{output}"
        );
        assert_eq!(output["holds"], json!([]), "{output}");

        let shown = text(&env, &["gates", "XX-1"]);
        assert!(!shown.contains("held for review"), "{shown}");
        assert_eq!(fs::read(env.item_path("XX-1")).unwrap(), before);
    }
}

mod tr_30 {
    //! Slice 9 review 1, test gaps 1 to 3: `approve`'s second look at the
    //! tree, `reject`'s re-read under the lock and a wrong ID typed into
    //! it, and `approve --json`'s output against its schema.
    use super::*;

    /// `tracker <line>` under `script` with stdout redirected to a file,
    /// typing `answer`, followed by a line saying how it exited. Returns
    /// the terminal's output and stdout's JSON.
    fn json_in_terminal(env: &Env, args: &str, answer: &str) -> Option<(String, Value)> {
        let out = env.work.path().join("out.json");
        let line = format!(
            "\"$TRACKER\" {args} --json > '{}'; echo \"tracker exited with $?\"",
            out.to_str().unwrap()
        );
        let (output, _) = env.in_terminal_typing_sh(&line, PROMPT, answer)?;
        Some((output, common::parse(&fs::read(&out).unwrap())))
    }

    // Criterion 1.

    #[test]
    fn approve_fails_g5_for_a_file_its_checks_leave() {
        // The check leaves `xx/left.txt` once `go` exists, which it does
        // only after the held submit.
        let env = env_with_project();
        let go = env.outside().join("go");
        let left = env.work.path().join("xx/left.txt");
        let check = format!(
            "if [ -f '{}' ]; then touch '{}'; fi",
            go.display(),
            left.display()
        );
        env.create_bare("XX", "a");
        env.json(&["criterion", "add", "XX-1", "it works", "--check", &check]);
        env.json(&["edit", "XX-1", "--label", "needs-review"]);
        env.json(&["start", "XX-1"]);
        let output = env.json(&["submit", "XX-1"]);
        assert_eq!(output["result"], "held", "{output}");
        fs::write(&go, "").unwrap();
        let before = fs::read(env.item_path("XX-1")).unwrap();

        let Some((output, error)) = json_in_terminal(&env, "approve XX-1", "XX-1") else {
            return;
        };
        assert!(output.contains("tracker exited with 2"), "{output}");
        common::check_schema("error", &error);
        assert_eq!(error["exit_code"], 2, "{error}");
        let g5: Vec<&Value> = error["failures"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["gate"] == "G5")
            .collect();
        assert_eq!(g5.len(), 1, "{error}");
        let message = g5[0]["message"].as_str().unwrap();
        assert!(message.contains("xx/left.txt"), "{message}");
        assert!(left.exists());

        assert_eq!(fs::read(env.item_path("XX-1")).unwrap(), before);
        let item = env.item("XX-1");
        assert_eq!(item["status"], "review", "{item}");
        assert!(item.get("verification").is_none(), "{item}");
        assert!(!has_ref(&env, "XX-1"));
    }

    // Criterion 2.

    #[test]
    fn reject_writes_nothing_when_the_item_changes_while_deciding() {
        let env = env_with_project();
        held_xx_1(&env);
        let path = env.item_path("XX-1");
        let Some((output, ok)) = env.in_terminal_typing(
            &["reject", "XX-1", "--reason", "not yet"],
            PROMPT,
            "XX-1",
            || {
                // Something else writes the item while I read the warning.
                let mut text = fs::read(&path).unwrap();
                text.push(b' ');
                fs::write(&path, text).unwrap();
            },
        ) else {
            return;
        };
        let changed = fs::read(&path).unwrap();
        assert!(!ok, "{output}");
        assert!(
            output.contains("changed while you were deciding"),
            "{output}"
        );
        assert_eq!(fs::read(&path).unwrap(), changed);
        assert_eq!(changed.last(), Some(&b' '));
        assert_eq!(env.item("XX-1")["status"], "review");
    }

    #[test]
    fn reject_typed_with_another_id_cancels_and_changes_nothing() {
        let env = env_with_project();
        held_xx_1(&env);
        let before = fs::read(env.item_path("XX-1")).unwrap();
        let Some((output, error)) =
            json_in_terminal(&env, "reject XX-1 --reason 'not yet'", "XX-2")
        else {
            return;
        };
        assert!(output.contains("tracker exited with 1"), "{output}");
        common::check_schema("error", &error);
        assert_eq!(error["exit_code"], 1, "{error}");
        let message = error["error"].as_str().unwrap();
        assert!(
            message.contains("Cancelled: \"XX-2\" isn't XX-1"),
            "{message}"
        );
        assert_eq!(fs::read(env.item_path("XX-1")).unwrap(), before);
    }

    // Criterion 3.

    #[test]
    fn a_successful_approve_json_matches_its_schema() {
        let env = env_with_project();
        held_xx_1(&env);
        let Some((output, approved)) = json_in_terminal(&env, "approve XX-1", "XX-1") else {
            return;
        };
        assert!(output.contains("tracker exited with 0"), "{output}");
        common::check_schema("approve", &approved);
        assert_eq!(approved["id"], "XX-1", "{approved}");
        assert_eq!(approved["result"], "approved", "{approved}");
        assert_eq!(approved["status"], "done", "{approved}");
        assert_eq!(approved["item"], env.item("XX-1"), "{approved}");
        let fields: Vec<&String> = approved.as_object().unwrap().keys().collect();
        assert_eq!(fields, ["id", "result", "status", "gates", "item"]);
    }

    #[test]
    fn an_approve_json_refused_by_a_gate_matches_the_refusal_shape() {
        // approve.schema.json says a refused approve prints the error shape,
        // which for a gate's refusal adds id, status, result, gates and
        // failures, so that's the schema it's checked against.
        let env = env_with_project();
        held_xx_1(&env);
        fs::write(env.work.path().join("xx/stray.txt"), "stray\n").unwrap();
        let before = fs::read(env.item_path("XX-1")).unwrap();
        let Some((output, refused)) = json_in_terminal(&env, "approve XX-1", "XX-1") else {
            return;
        };
        assert!(output.contains("tracker exited with 2"), "{output}");
        common::check_schema("error", &refused);
        assert_eq!(refused["exit_code"], 2, "{refused}");
        assert_eq!(refused["id"], "XX-1", "{refused}");
        assert_eq!(refused["status"], "review", "{refused}");
        assert_eq!(refused["result"], "refused", "{refused}");
        assert!(refused["gates"].is_array(), "{refused}");
        assert_eq!(refused["failures"][0]["gate"], "G5", "{refused}");
        assert_eq!(fs::read(env.item_path("XX-1")).unwrap(), before);
    }
}

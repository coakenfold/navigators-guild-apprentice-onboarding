//! TR-44: when git no longer has an item's `started.head`, `gates`,
//! `submit` and `approve` still exit 3, since a lost base can't show every
//! commit since start, but the message names the lost commit and says how
//! to start the item over, not git's own error.

mod common;

use std::fs;

use common::{Env, git, git_commit_all};

const PROMPT: &str = "Type XX-1 to confirm";

/// The first seven characters of XX-1's `started.head`, after making git
/// lose that commit: the branch is reset past it, another commit is made,
/// and the reflog and unreachable objects are pruned.
fn lose_started_head(env: &Env) -> String {
    let head = env.item("XX-1")["started"]["head"]
        .as_str()
        .unwrap()
        .to_string();
    let work = env.work.path();
    git(work, &["reset", "--quiet", "--hard", "HEAD~1"]);
    git_commit_all(work, "test(XX-1): the work again");
    git(work, &["reflog", "expire", "--expire=now", "--all"]);
    git(work, &["gc", "--quiet", "--prune=now"]);
    assert!(
        !common::git_ok(work, &["cat-file", "-e", &head]),
        "git still has {head}"
    );
    head[..7].to_string()
}

fn check_message(message: &str, short: &str) {
    assert!(message.contains(short), "{message}");
    assert!(!message.contains("failed in"), "{message}");
    assert!(!message.contains("git rev-list"), "{message}");
    assert!(!message.contains("git log"), "{message}");
    for word in ["shelve", "reopen", "start"] {
        assert!(message.contains(word), "{message}");
    }
}

mod criterion_1 {
    use super::*;

    #[test]
    fn gates_and_submit_name_the_lost_commit_and_how_to_start_over() {
        let env = Env::new();
        env.add_project("XX", "xx");
        env.create("XX", "lost");
        env.json(&["start", "XX-1"]);
        let short = lose_started_head(&env);
        for command in ["gates", "submit"] {
            let message = env.error(&[command, "XX-1"], 3);
            check_message(&message, &short);
        }
    }
}

mod criterion_2 {
    use super::*;

    #[test]
    fn approve_names_the_lost_commit() {
        let env = Env::new();
        env.add_project("XX", "xx");
        env.create("XX", "asked for");
        env.json(&["edit", "XX-1", "--label", "needs-review"]);
        env.json(&["start", "XX-1"]);
        let held = env.json(&["submit", "XX-1"]);
        assert_eq!(held["result"], "held", "{held}");
        let short = lose_started_head(&env);

        let out = env.work.path().join("out.json");
        let line = format!(
            "\"$TRACKER\" approve XX-1 --json > '{}'; echo \"tracker exited with $?\"",
            out.to_str().unwrap()
        );
        let Some((output, _)) = env.in_terminal_typing_sh(&line, PROMPT, "XX-1") else {
            return;
        };
        assert!(output.contains("tracker exited with 3"), "{output}");
        let error = common::parse(&fs::read(&out).unwrap());
        assert_eq!(error["exit_code"], 3, "{error}");
        check_message(error["error"].as_str().unwrap(), &short);
        assert_eq!(env.item("XX-1")["status"], "review");
    }
}

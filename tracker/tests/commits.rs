//! Slice 8 (TR-4): `submit` proves committed work, as DESIGN.md §2.1, G5
//! (§2.4) and §2.7 describe. An item's commits are those reachable from
//! `HEAD` whose header is scoped to its ID. G5 needs a clean tree under the
//! project root and at least one item commit. A passing submit records the
//! commit and creates `refs/tracker/<ID>`, which keeps it, and `delete`
//! removes. Shelving an in-progress item needs a clean tree and passing
//! project checks.
//!
//! Every project here is a subfolder, `xx`, of a throwaway repository, so
//! the folder rule (PLAN.md's assumptions, §2.1) is tested as TR uses it.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::{Env, git, git_commit_all, git_init, git_ok};
use serde_json::{Value, json};

/// A store with project XX, whose root is the subfolder `xx` of a
/// throwaway repository at `<work>/repo`. The repository also holds
/// `other/notes.txt`, outside the root, and everything is committed.
/// Returns the store, the repository and the root.
fn env_with_repo() -> (Env, PathBuf, PathBuf) {
    let env = Env::new();
    let repo = env.dir("repo");
    git_init(&repo);
    let root = env.dir("repo/xx");
    fs::write(root.join("a.txt"), "a\n").unwrap();
    fs::write(root.join(".gitignore"), "*.log\n").unwrap();
    fs::create_dir_all(repo.join("other")).unwrap();
    fs::write(repo.join("other/notes.txt"), "notes\n").unwrap();
    git_commit_all(&repo, "chore: set up");
    env.json(&[
        "project",
        "add",
        "XX",
        "--name",
        "XX project",
        "--root",
        root.to_str().unwrap(),
    ]);
    (env, repo, root)
}

/// The same, with item XX-1, which has one criterion whose check is `true`,
/// so that G2 and G3 pass and G5 is what's tested.
fn env_with_item() -> (Env, PathBuf, PathBuf) {
    let (env, repo, root) = env_with_repo();
    env.create_bare("XX", "a");
    env.add_criterion("XX-1");
    (env, repo, root)
}

/// Sets `field` of project `key` in `config.json` by hand, as §4.2 says
/// settings other than name, root and design are set.
fn set_project(env: &Env, key: &str, field: &str, value: Value) {
    let path = env.home.path().join("config.json");
    let mut config: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    config["projects"][key][field] = value;
    fs::write(&path, serde_json::to_string_pretty(&config).unwrap() + "\n").unwrap();
}

/// Runs `tracker <args> --json`, expects exit 2, a gate's refusal, and
/// returns the error object.
fn refused(env: &Env, args: &[&str]) -> Value {
    env.error_json_in(&env.outside(), args, 2)
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

/// The messages of G5's failures in a dry run of XX-1's gates, which must
/// refuse.
fn g5_failures(env: &Env) -> Vec<String> {
    let error = refused(env, &["gates", "XX-1"]);
    assert_eq!(gate_ok(&error["gates"], "G5"), Some(false), "{error}");
    failures_from(&error, "G5")
        .iter()
        .map(|f| f["message"].as_str().unwrap().to_string())
        .collect()
}

/// Asserts that a dry run of XX-1's gates passes, with G5 among them.
fn g5_passes(env: &Env) {
    let output = env.json(&["gates", "XX-1"]);
    assert_eq!(output["result"], "passed", "{output}");
    assert_eq!(gate_ok(&output["gates"], "G5"), Some(true), "{output}");
}

/// What G5 says when the item has no commit.
const NO_COMMITS: &str = "XX-1 has no commits";

// Criterion 1: an uncommitted change fails G5, and so does an untracked
// file.

#[test]
fn g5_fails_for_an_uncommitted_change() {
    let (env, _, root) = env_with_item();
    git_commit_all(&root, "feat(XX-1): add a");
    g5_passes(&env);

    // A change to a committed file, not staged.
    fs::write(root.join("a.txt"), "changed\n").unwrap();
    let messages = g5_failures(&env);
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert!(messages[0].contains("xx/a.txt"), "{messages:?}");

    // Staged, it's still uncommitted.
    git(&root, &["add", "a.txt"]);
    let messages = g5_failures(&env);
    assert!(messages[0].contains("xx/a.txt"), "{messages:?}");

    git_commit_all(&root, "feat(XX-1): change a");
    g5_passes(&env);
}

#[test]
fn g5_fails_for_an_untracked_file() {
    let (env, _, root) = env_with_item();
    git_commit_all(&root, "feat(XX-1): add a");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/helper.rs"), "// new\n").unwrap();

    let messages = g5_failures(&env);
    assert_eq!(messages.len(), 1, "{messages:?}");
    // Every untracked file is named, not only its folder.
    assert!(messages[0].contains("xx/src/helper.rs"), "{messages:?}");

    git_commit_all(&root, "feat(XX-1): add the helper");
    g5_passes(&env);
}

// Criterion 2: an ignored file doesn't fail G5, and neither does a dirty
// file outside a subfolder project's root.

#[test]
fn g5_ignores_an_ignored_file() {
    let (env, _, root) = env_with_item();
    git_commit_all(&root, "feat(XX-1): add a");
    fs::write(root.join("debug.log"), "noise\n").unwrap();
    g5_passes(&env);
}

#[test]
fn g5_ignores_a_dirty_file_outside_a_subfolder_projects_root() {
    let (env, repo, root) = env_with_item();
    git_commit_all(&root, "feat(XX-1): add a");
    fs::write(repo.join("other/notes.txt"), "changed\n").unwrap();
    fs::write(repo.join("other/new.txt"), "new\n").unwrap();
    fs::write(repo.join("top.txt"), "new\n").unwrap();
    g5_passes(&env);

    // The same change inside the root does fail it.
    fs::write(root.join("new.txt"), "new\n").unwrap();
    let messages = g5_failures(&env);
    assert!(messages[0].contains("xx/new.txt"), "{messages:?}");
    assert!(!messages[0].contains("other/"), "{messages:?}");
    assert!(!messages[0].contains("top.txt"), "{messages:?}");
}

// Criterion 3: `feat(XX-1): …` and `fix(XX-1)!: …` count as XX-1's
// commits.

#[test]
fn a_feat_commit_scoped_to_the_item_counts_as_its_commit() {
    let (env, _, root) = env_with_item();
    let messages = g5_failures(&env);
    assert!(messages[0].contains(NO_COMMITS), "{messages:?}");

    git_commit_all(&root, "feat(XX-1): add search");
    g5_passes(&env);
}

#[test]
fn a_breaking_fix_commit_scoped_to_the_item_counts_as_its_commit() {
    let (env, _, root) = env_with_item();
    let messages = g5_failures(&env);
    assert!(messages[0].contains(NO_COMMITS), "{messages:?}");

    git_commit_all(&root, "fix(XX-1)!: drop the old sort flag");
    g5_passes(&env);
}

// Criterion 4: a commit scoped to `XX-12`, one headed `XX-1: …`, one that
// names XX-1 only in its body, and a `feat(XX-1)` commit on another branch
// don't count as XX-1's commits.

/// Asserts that G5 fails XX-1 for having no commits, and for nothing else.
fn has_no_commits(env: &Env) {
    let messages = g5_failures(env);
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert!(messages[0].contains(NO_COMMITS), "{messages:?}");
}

#[test]
fn a_commit_scoped_to_another_item_doesnt_count_as_its_commit() {
    let (env, _, root) = env_with_item();
    git_commit_all(&root, "feat(XX-12): sort by date");
    has_no_commits(&env);
}

#[test]
fn a_commit_headed_with_the_bare_id_doesnt_count_as_its_commit() {
    let (env, _, root) = env_with_item();
    git_commit_all(&root, "XX-1: add search");
    has_no_commits(&env);
}

#[test]
fn a_commit_naming_the_item_only_in_its_body_doesnt_count_as_its_commit() {
    let (env, _, root) = env_with_item();
    git_commit_all(
        &root,
        "chore: tidy the list\n\nfeat(XX-1): add search\nPart of XX-1.",
    );
    has_no_commits(&env);
}

#[test]
fn a_commit_on_another_branch_doesnt_count_as_its_commit() {
    let (env, repo, root) = env_with_item();
    git(&repo, &["switch", "--quiet", "-c", "elsewhere"]);
    git_commit_all(&root, "feat(XX-1): add search");
    git(&repo, &["switch", "--quiet", "main"]);
    has_no_commits(&env);

    // The same commit, reachable from HEAD, counts.
    git(&repo, &["merge", "--quiet", "--ff-only", "elsewhere"]);
    g5_passes(&env);
}

// Criterion 5: a passing submit records the SHA and each check's result,
// duration and output, and creates the ref. Deleting the item removes the
// ref.

/// A store with XX-1 started, whose criterion check reads `done.txt`, so
/// it fails at start, and a project check that prints a line. The work,
/// `done.txt`, is then committed as `feat(XX-1): …`, so a submit passes.
/// Returns the store, the repository, the root and the SHA of the work.
fn env_ready_to_submit() -> (Env, PathBuf, PathBuf, String) {
    let (env, repo, root) = env_with_repo();
    set_project(&env, "XX", "checks", json!(["echo project checks pass"]));
    env.create_bare("XX", "a");
    env.json(&[
        "criterion",
        "add",
        "XX-1",
        "done.txt says done",
        "--check",
        "cat done.txt",
    ]);
    git_commit_all(&root, "test(XX-1): nothing yet");
    env.json(&["start", "XX-1"]);
    fs::write(root.join("done.txt"), "done\n").unwrap();
    let sha = git_commit_all(&root, "feat(XX-1): write done.txt");
    (env, repo, root, sha)
}

#[test]
fn a_passing_submit_records_the_commit_and_each_checks_result_and_creates_the_ref() {
    let (env, repo, _, sha) = env_ready_to_submit();
    let output = env.json(&["submit", "XX-1"]);
    assert_eq!(output["result"], "done", "{output}");
    assert_eq!(gate_ok(&output["gates"], "G5"), Some(true), "{output}");

    let item = env.item("XX-1");
    assert_eq!(output["item"], item);
    let record = &item["verification"];
    assert_eq!(record["by"], "auto", "{record}");
    assert_eq!(record["commit"], sha.as_str(), "{record}");
    assert_eq!(record["ref"], "refs/tracker/XX-1", "{record}");
    assert_eq!(record.get("note"), None, "{record}");
    let checks = record["checks"].as_array().unwrap();
    assert_eq!(checks.len(), 2, "{record}");
    assert_eq!(checks[0]["criterion"], 1);
    assert_eq!(checks[0]["cmd"], "cat done.txt");
    assert_eq!(checks[0]["ok"], true);
    assert!(checks[0]["secs"].is_u64(), "{record}");
    assert_eq!(checks[0]["output"], "done");
    assert_eq!(checks[1].get("criterion"), None);
    assert_eq!(checks[1]["cmd"], "echo project checks pass");
    assert_eq!(checks[1]["ok"], true);
    assert!(checks[1]["secs"].is_u64(), "{record}");
    assert_eq!(checks[1]["output"], "project checks pass");

    // The ref is in the project's repository, at the verified commit.
    assert_eq!(git(&repo, &["rev-parse", "refs/tracker/XX-1"]), sha);
}

#[test]
fn a_passing_submits_ref_is_removed_when_the_item_is_deleted() {
    let (env, repo, _, sha) = env_ready_to_submit();
    env.json(&["submit", "XX-1"]);
    assert_eq!(git(&repo, &["rev-parse", "refs/tracker/XX-1"]), sha);

    let Some((output, ok)) =
        env.in_terminal_typing(&["delete", "XX-1"], "Type XX-1 to confirm", "XX-1", || {})
    else {
        return;
    };
    assert!(ok, "{output}");
    // The prompt says the ref goes too (§2.8).
    assert!(output.contains("refs/tracker/XX-1"), "{output}");
    assert!(!env.item_path("XX-1").exists());
    assert!(
        !git_ok(
            &repo,
            &["rev-parse", "--verify", "--quiet", "refs/tracker/XX-1"]
        ),
        "the ref is still there"
    );
    // The commit itself stays in git, on its branch.
    assert_eq!(git(&repo, &["rev-parse", "HEAD"]), sha);
}

// Criterion 6: the commit is still readable after its branch is rebased
// away and `git gc --prune=now` runs. After the test is renamed, `show`
// still prints the saved output.

#[test]
fn the_verified_commit_outlives_a_rebase_and_gc() {
    let (env, repo, root) = env_with_repo();
    env.create_bare("XX", "a");
    env.json(&[
        "criterion",
        "add",
        "XX-1",
        "done.txt says done",
        "--check",
        "cat done.txt",
    ]);
    env.json(&["start", "XX-1"]);
    git(&repo, &["switch", "--quiet", "-c", "work"]);
    fs::write(root.join("done.txt"), "done\n").unwrap();
    let verified = git_commit_all(&root, "feat(XX-1): write done.txt");
    env.json(&["submit", "XX-1"]);
    assert_eq!(
        env.item("XX-1")["verification"]["commit"],
        verified.as_str()
    );

    // `main` moves on, and `work` is rebased onto it, so no branch points
    // to the verified commit any more.
    git(&repo, &["switch", "--quiet", "main"]);
    fs::write(repo.join("other/notes.txt"), "more notes\n").unwrap();
    git_commit_all(&repo, "docs: more notes");
    git(&repo, &["rebase", "--quiet", "main", "work"]);
    let rebased = git(&repo, &["rev-parse", "work"]);
    assert_ne!(rebased, verified);

    // A commit nothing points to, to show that the gc below does prune.
    git(
        &repo,
        &["commit", "--quiet", "--allow-empty", "-m", "scratch"],
    );
    let scratch = git(&repo, &["rev-parse", "HEAD"]);
    git(&repo, &["reset", "--quiet", "--hard", "HEAD~1"]);

    git(&repo, &["reflog", "expire", "--expire=now", "--all"]);
    git(&repo, &["gc", "--quiet", "--prune=now"]);
    let exists = |sha: &str| git_ok(&repo, &["cat-file", "-e", &format!("{sha}^{{commit}}")]);
    assert!(!exists(&scratch), "gc didn't prune the scratch commit");
    assert!(exists(&verified), "gc pruned the verified commit");
    assert_eq!(git(&repo, &["rev-parse", "refs/tracker/XX-1"]), verified);
    // And the tree as it was verified is still there.
    assert_eq!(
        git(&repo, &["show", &format!("{verified}:xx/done.txt")]),
        "done"
    );
}

#[test]
fn the_saved_output_shows_after_the_test_is_renamed() {
    let (env, repo, root) = env_with_repo();
    fs::create_dir_all(root.join("tests")).unwrap();
    fs::write(
        root.join("tests/filters_by_tag.sh"),
        "test -f done.txt && echo 'test filters_by_tag ... ok'\n",
    )
    .unwrap();
    git_commit_all(&root, "test(XX-1): add filters_by_tag");
    env.create_bare("XX", "a");
    env.json(&[
        "criterion",
        "add",
        "XX-1",
        "it filters by tag",
        "--check",
        "sh tests/filters_by_tag.sh",
    ]);
    env.json(&["start", "XX-1"]);
    fs::write(root.join("done.txt"), "done\n").unwrap();
    git_commit_all(&root, "feat(XX-1): filter by tag");
    env.json(&["submit", "XX-1"]);

    // The test is renamed, and now prints its new name.
    git(
        &root,
        &["mv", "tests/filters_by_tag.sh", "tests/tag_filter.sh"],
    );
    fs::write(
        root.join("tests/tag_filter.sh"),
        "test -f done.txt && echo 'test tag_filter ... ok'\n",
    )
    .unwrap();
    git_commit_all(&repo, "refactor: rename the tag test");

    let output = env.cmd().args(["show", "XX-1"]).assert().success();
    let text = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(text.contains("test filters_by_tag ... ok"), "{text}");
    assert!(text.contains("sh tests/filters_by_tag.sh"), "{text}");
    assert!(!text.contains("tag_filter"), "{text}");
}

// Criterion 7: shelving an in-progress item is refused while the tree is
// dirty or a project check fails, and allowed once both are fixed.

#[test]
fn shelving_an_in_progress_item_is_refused_while_the_tree_is_dirty_or_a_project_check_fails() {
    let (env, _, root) = env_with_item();
    set_project(&env, "XX", "checks", json!(["test ! -f broken"]));
    env.json(&["start", "XX-1"]);
    let before = fs::read_to_string(env.item_path("XX-1")).unwrap();
    let shelve = ["shelve", "XX-1", "--reason", "Blocked on XX-2."];

    // Both: an untracked file in the root, which also breaks the check.
    fs::write(root.join("broken"), "").unwrap();
    let error = refused(&env, &shelve);
    assert_eq!(error["result"], "refused", "{error}");
    assert_eq!(gate_ok(&error["gates"], "G5"), Some(false), "{error}");
    assert_eq!(gate_ok(&error["gates"], "G4"), Some(false), "{error}");
    assert!(
        error["error"]
            .as_str()
            .unwrap()
            .contains("can't be shelved"),
        "{error}"
    );
    assert_eq!(fs::read_to_string(env.item_path("XX-1")).unwrap(), before);

    // Committed, the tree is clean, but the check still fails.
    git_commit_all(&root, "chore: break the build");
    let error = refused(&env, &shelve);
    assert_eq!(gate_ok(&error["gates"], "G5"), Some(true), "{error}");
    assert_eq!(gate_ok(&error["gates"], "G4"), Some(false), "{error}");
    assert_eq!(fs::read_to_string(env.item_path("XX-1")).unwrap(), before);

    // The check is fixed, but the fix isn't committed.
    fs::remove_file(root.join("broken")).unwrap();
    let error = refused(&env, &shelve);
    assert_eq!(gate_ok(&error["gates"], "G5"), Some(false), "{error}");
    assert_eq!(gate_ok(&error["gates"], "G4"), Some(true), "{error}");
    let g5 = failures_from(&error, "G5");
    assert!(
        g5[0]["message"].as_str().unwrap().contains("xx/broken"),
        "{error}"
    );
    assert_eq!(fs::read_to_string(env.item_path("XX-1")).unwrap(), before);

    // Both fixed: the shelve goes ahead.
    git_commit_all(&root, "revert(XX-1): unbreak the build");
    let output = env.json(&shelve);
    assert_eq!(output["item"]["status"], "shelved", "{output}");
}

// Criterion 8: if a check edits the item file while the gates run, `submit`
// writes nothing and exits 1.

/// A check that passes, and once `go` exists, changes XX-1's priority
/// through `tracker edit`. `go` is outside the repository, so making it
/// leaves the tree clean.
fn check_that_edits(go: &Path) -> String {
    format!(
        "if [ -f '{}' ]; then '{}' edit XX-1 --priority high; fi",
        go.display(),
        env!("CARGO_BIN_EXE_tracker")
    )
}

/// Asserts that XX-1's submit exits 1 for a change made while its checks
/// ran, keeps that change, and writes nothing: no verification record, no
/// move and no ref.
fn submit_writes_nothing(env: &Env, repo: &Path) {
    let history = env.item("XX-1")["history"].clone();
    let message = env.error(&["submit", "XX-1"], 1);
    assert!(
        message.contains("changed while its checks ran"),
        "{message}"
    );
    let item = env.item("XX-1");
    assert_eq!(item["status"], "in-progress", "{item}");
    assert_eq!(item["priority"], "high", "{item}");
    assert_eq!(item.get("verification"), None, "{item}");
    // The edit's entry, and nothing after it.
    let entries = item["history"].as_array().unwrap();
    assert_eq!(entries.len(), history.as_array().unwrap().len() + 1);
    assert_eq!(entries.last().unwrap()["via"], "edit", "{item}");
    assert!(
        !git_ok(
            repo,
            &["rev-parse", "--verify", "--quiet", "refs/tracker/XX-1"]
        ),
        "the ref was created"
    );
}

#[test]
fn submit_writes_nothing_when_a_criterion_check_edits_the_item_file() {
    let (env, repo, root) = env_with_repo();
    let go = env.outside().join("go");
    env.create_bare("XX", "a");
    env.json(&[
        "criterion",
        "add",
        "XX-1",
        "it works",
        "--check",
        &check_that_edits(&go),
    ]);
    env.json(&["start", "XX-1"]);
    git_commit_all(&root, "feat(XX-1): the work");
    fs::write(&go, "").unwrap();
    submit_writes_nothing(&env, &repo);
}

#[test]
fn submit_writes_nothing_when_a_project_check_edits_the_item_file() {
    let (env, repo, root) = env_with_item();
    let go = env.outside().join("go");
    set_project(&env, "XX", "checks", json!([check_that_edits(&go)]));
    env.json(&["start", "XX-1"]);
    git_commit_all(&root, "feat(XX-1): the work");
    fs::write(&go, "").unwrap();
    submit_writes_nothing(&env, &repo);
}

// Beyond the criteria: what else slice 8's prompt asks for, and how it
// settles slice 7 review 2's "Slice 8 must decide" list.

#[test]
fn start_records_head() {
    let (env, repo, _) = env_with_item();
    let head = git(&repo, &["rev-parse", "HEAD"]);
    let output = env.json(&["start", "XX-1"]);
    assert_eq!(output["item"]["started"]["head"], head.as_str(), "{output}");
}

#[test]
fn a_root_outside_any_git_work_tree_exits_3() {
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
    env.add_criterion("XX-1");
    let before = fs::read_to_string(env.item_path("XX-1")).unwrap();

    for args in [&["gates", "XX-1"][..], &["start", "XX-1"]] {
        let message = env.error(args, 3);
        assert!(
            message.contains("isn't inside a git work tree"),
            "{message}"
        );
        assert_eq!(fs::read_to_string(env.item_path("XX-1")).unwrap(), before);
    }

    // Started while the root was in a repository, which is then removed.
    git_init(&root);
    env.json(&["start", "XX-1"]);
    fs::remove_dir_all(root.join(".git")).unwrap();
    let before = fs::read_to_string(env.item_path("XX-1")).unwrap();
    for args in [
        &["submit", "XX-1"][..],
        &["shelve", "XX-1", "--reason", "later"],
    ] {
        let message = env.error(args, 3);
        assert!(
            message.contains("isn't inside a git work tree"),
            "{message}"
        );
        assert_eq!(fs::read_to_string(env.item_path("XX-1")).unwrap(), before);
    }
}

#[test]
fn a_check_that_leaves_a_file_in_the_root_fails_g5() {
    // Review 1's "must decide": G5 looks at the tree again once the checks
    // have run, so a check that leaves a file fails this submit, not the
    // next one or the next item's.
    let (env, repo, root) = env_with_repo();
    env.create_bare("XX", "a");
    env.json(&[
        "criterion",
        "add",
        "XX-1",
        "it works",
        "--check",
        "test -f done.txt && touch left-behind.txt",
    ]);
    env.json(&["start", "XX-1"]);
    fs::write(root.join("done.txt"), "done\n").unwrap();
    git_commit_all(&root, "feat(XX-1): the work");

    let error = refused(&env, &["submit", "XX-1"]);
    assert_eq!(gate_ok(&error["gates"], "G3"), Some(true), "{error}");
    assert_eq!(gate_ok(&error["gates"], "G5"), Some(false), "{error}");
    let g5 = failures_from(&error, "G5");
    assert_eq!(g5.len(), 1, "{error}");
    let message = g5[0]["message"].as_str().unwrap();
    assert!(message.contains("The checks left"), "{message}");
    assert!(message.contains("xx/left-behind.txt"), "{message}");
    assert_eq!(env.item("XX-1")["status"], "in-progress");
    assert!(!git_ok(
        &repo,
        &["rev-parse", "--verify", "--quiet", "refs/tracker/XX-1"]
    ));

    // A file that was already there before the checks is listed once,
    // under the first G5 failure, not again as left by them.
    let error = refused(&env, &["submit", "XX-1"]);
    let g5 = failures_from(&error, "G5");
    assert_eq!(g5.len(), 1, "{error}");
    assert!(
        !g5[0]["message"]
            .as_str()
            .unwrap()
            .contains("The checks left"),
        "{error}"
    );
}

#[test]
fn a_check_that_moves_head_fails_g5() {
    let (env, _, root) = env_with_item();
    git_commit_all(&root, "feat(XX-1): the work");
    set_project(
        &env,
        "XX",
        "checks",
        json!([
            "git -c user.name=t -c user.email=t@t.invalid -c commit.gpgsign=false commit \
             --quiet --allow-empty -m 'feat(XX-1): sneaked in'"
        ]),
    );
    env.json(&["start", "XX-1"]);
    let error = refused(&env, &["submit", "XX-1"]);
    let g5 = failures_from(&error, "G5");
    assert_eq!(g5.len(), 1, "{error}");
    assert!(
        g5[0]["message"]
            .as_str()
            .unwrap()
            .contains("HEAD moved from"),
        "{error}"
    );
}

#[test]
fn a_shelve_writes_nothing_when_a_project_check_edits_the_item_file() {
    // Review 2's "must decide": shelving runs project checks without the
    // lock, so it reads the item again afterwards, as TR-18's tests show
    // for `start`.
    let (env, _, _) = env_with_item();
    let go = env.outside().join("go");
    set_project(&env, "XX", "checks", json!([check_that_edits(&go)]));
    env.json(&["start", "XX-1"]);
    fs::write(&go, "").unwrap();
    let message = env.error(&["shelve", "XX-1", "--reason", "later"], 1);
    assert!(
        message.contains("changed while its checks ran"),
        "{message}"
    );
    let item = env.item("XX-1");
    assert_eq!(item["status"], "in-progress", "{item}");
    assert_eq!(item["priority"], "high", "{item}");
}

#[test]
fn an_open_item_shelves_whatever_the_tree_and_checks_say() {
    // Only an in-progress item frees its project (§2.8).
    let (env, _, root) = env_with_item();
    set_project(&env, "XX", "checks", json!(["false"]));
    fs::write(root.join("scratch.txt"), "").unwrap();
    let output = env.json(&["shelve", "XX-1", "--reason", "later"]);
    assert_eq!(output["item"]["status"], "shelved", "{output}");
}

#[test]
fn show_prints_started_and_each_saved_output_escaped() {
    // Review 2's "must decide": `show` prints what start recorded, and
    // each check's saved output with its criterion, through `Styled`, so a
    // control character in the output is written out.
    let (env, _, root) = env_with_repo();
    set_project(&env, "XX", "checks", json!(["echo two; echo lines"]));
    env.create_bare("XX", "a");
    env.json(&[
        "criterion",
        "add",
        "XX-1",
        "it works",
        "--check",
        "test -f done.txt && printf 'a\\033[2Kb\\n'",
    ]);
    env.json(&["start", "XX-1"]);
    fs::write(root.join("done.txt"), "done\n").unwrap();
    git_commit_all(&root, "feat(XX-1): the work");
    env.json(&["submit", "XX-1"]);
    let item = env.item("XX-1");

    let output = env.cmd().args(["show", "XX-1"]).assert().success();
    let text = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(!text.contains('\u{1b}'), "{text:?}");
    for expected in [
        "Started:".to_string(),
        format!("  head    {}", item["started"]["head"].as_str().unwrap()),
        "  checks  criterion 1 failed".to_string(),
        format!(
            "  commit  {}",
            item["verification"]["commit"].as_str().unwrap()
        ),
        "  ref     refs/tracker/XX-1".to_string(),
        "    criterion 1, passed in".to_string(),
        "      a\\u{1b}[2Kb\n".to_string(),
        "    project, passed in".to_string(),
        ": `echo two; echo lines`\n      two\n      lines\n".to_string(),
    ] {
        assert!(text.contains(&expected), "{expected:?} isn't in:\n{text}");
    }
}

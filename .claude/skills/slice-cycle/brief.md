# Brief for agents working on the `tracker` crate

You run unattended, as a subagent: never ask questions, make sensible calls and say what you decided in your final report.

Repo root: /Users/chadoakenfold/Documents/Code/guild-projects/guild-portfolio. The crate is `tracker/`. Work from `tracker/` unless told otherwise. Read `tracker/CLAUDE.md` first: its conventions, test command and "never read src/main.rs whole" rule apply.

## Hard rules

- Stay on the current git branch. Never switch branches, rebase, merge, push, or run `gh` commands. The orchestrator handles branches and the stack.
- Demo and scratch scripts: start with `set -eu`, `cd` into a temp folder (never run them from `tracker/` or the repo), and export `TRACKER_HOME` before any `tracker` command. Give every heredoc a quoted, unique delimiter such as `<<'DEMO_END'`, never `EOF`, since demo files often contain an `EOF` line of their own.
- Never run `tracker` against the real store except through the tracker workflow below. For any demo or experiment, set `TRACKER_HOME` to a temporary folder and use `target/debug/tracker`.
- Track items with the **installed** `tracker` (from `~/.cargo/bin`), never `target/debug/tracker` or `cargo run`. Never run `cargo install`: the orchestrator installs new binaries between slices.
- Never edit files under `~/.tracker`. Change items only through `tracker` commands. Never delete an item. Never try to approve, or to fake an interactive terminal (`script`, `expect`, a pty) for a real item: `approve`, `reject`, `delete` and `criterion accept` are the user's. Using `script` inside a test against a throwaway `TRACKER_HOME` is fine, as the plan allows.
- If an item is held for review, stop and report the hold reasons.
- Disk space is limited. Delete any scratch repo copies, store copies and `target` folders you create before you finish, and never copy `tracker/target`.
- Never make an unscoped commit, or move a branch, under `tracker/` while a TR item is in progress or in review: from slice 11's binary on, T14 holds the item for it.
- Docs commits: `tests/docs_links.rs` checks each review's links as of the review's own last commit, and the readme's at `HEAD`. So when a readme update and a review's Outcome link to each other, commit the readme update first and the Outcome second, never the reverse, and run `cargo test --test docs_links` after both.
- Commit messages carry no attribution or trailer lines.
- Don't over-engineer. Prefer the plain fix for the likely case over handling unlikely scenarios.

## The tracker workflow (from the user's ~/.claude/CLAUDE.md and DESIGN.md §5)

- Pick up work with `tracker show <ID>`. One item at a time; don't write the next item's tests until the current one is done. An item held in review isn't done: a commit scoped to the next item, made while one waits in review, makes T14 hold the waiting item at `approve`.
- Before `tracker start <ID>`: give every criterion its check (`tracker criterion set <ID> <n> --check "<cmd>"`), write and commit the tests those checks run, and run them to see them fail.
- Never reword or remove a criterion you were given. If one is wrong or can't be checked, add a criterion or say so in your report.
- Every criterion needs a check that proves it. Use a script check whenever possible. Make each check run just that criterion's test, such as `cargo test --test checks some_test_name`, not the whole suite. Check that the filter matches the test(s) you mean (a filter that matches nothing passes with "0 passed").
- From slice 11's binary on, T13 holds an item whose check already passes at start. So each criterion's check should include a test that fails before the work, even for a fix to code that already mostly works.
- If the change will foreseeably break an existing test (a pinned output shape, a gate list), update that test in the test commit before `tracker start`, so it's seen failing first. From slice 11, editing an existing test fires T4 and holds the item, whichever commit does it.
- A slice that adds a trigger adds it to every test that pins the trigger list (`grep -rn '"T14"' tests/` finds them) in its first test commit, before `start`, so they're seen failing.
- A check that asserts text is absent must fail when the file can't be read: write `test -f docs/DESIGN.md && ! grep -qF '…' docs/DESIGN.md`, never a bare `! grep`.
- Run `cargo fmt` before every commit, tests included, so no later commit has to reformat a test.
- Stage only the files you changed, by name (`git add tracker/src/x.rs …`). Never `git add -A`, `git add .` or `git commit -a`, and never commit a file you didn't change: the orchestrator may have its own changes in the repo.
- Commits are Conventional Commits scoped to the item: `test(TR-n): …`, `feat(TR-n): …`, `fix(TR-n): …`, `docs(TR-n): …`. Only the scope links a commit to the item. Never make an unscoped commit while an item is in progress.
- Add `--json` when you need to read a result. Run `tracker gates <ID>` to see what's left, then `tracker submit <ID>`.
- Before submitting, run the full test command yourself: `cargo fmt --check && cargo clippy -- -D warnings && cargo test` (the installed binary may not run project checks yet).
- If a gate refuses, fix the cause. Never weaken a check or edit an existing test just to pass. If a test is genuinely wrong, fix it and say so.
- Out-of-scope work you find: `tracker create` it (with `--project TR`, a description saying where it came from) instead of doing it. The exception is a project check failing for a reason outside the item (a flaky test, a new lint): fix it there and say so.
- If an item can't be done now: revert its unfinished work in a commit scoped to it, then `tracker shelve <ID> --reason "…"`.

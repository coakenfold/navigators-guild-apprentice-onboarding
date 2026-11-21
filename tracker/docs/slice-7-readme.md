# Slice 7: Running checks

`tracker` is a command-line work tracker, written in Rust, that keeps one central list of work items across many projects and decides when an item is really done. This document covers **slice 7** of the 16 in [PLAN.md](PLAN.md): what it built, how it works, and how to test it. The earlier readmes cover the rest, ending with [slice 6](slice-6-readme.md), criteria and the first gates.

After slice 7 you can **see checks run at start and submit, and fail with their output**. The spec is [DESIGN.md](DESIGN.md). This slice implements gates G3 and G4 from [§2.4](DESIGN.md#24-gates-is-it-finished), the project `checks` and `check_timeout_secs` from [§4.2](DESIGN.md#42-config), and `started.checks` and `verification.checks` from [§4.3](DESIGN.md#43-item-file). It's the first slice tracked in `tracker` itself, as TR-1.

| | |
| --- | --- |
| Status | Every _Done when_ check has a test, and every test passes, as does the test command. PLAN.md has slice 7 as `in review`. The branch isn't merged into `main`. |
| Item | TR-1, whose five criteria each run one test in `tests/checks.rs` |
| Branch | `feat/tracker/slice-7`, from `main` at `37abcc3`: `6ecf0e6` (the criteria's tests, seen failing before `tracker start TR-1`), `fa65c9a` (the code and schemas), `0ac6b10` (slice 6 tests whose expected gate lists and note changed), `b59d88d` (fixtures), then this readme and PLAN.md's status |
| Test command | `cargo fmt --check && cargo clippy -- -D warnings && cargo test`, run in `tracker/` |
| Tests | 255: 5 new in `tests/checks.rs`, 1 new in `tests/fixtures.rs`, and 1 new unit test in `src/checks.rs` |

## Contents

- [What's in this slice](#whats-in-this-slice)
- [Try it](#try-it)
- [How it works](#how-it-works)
- [JSON output](#json-output)
- [Code tour](#code-tour)
- [Testing](#testing)
- [Decisions worth knowing](#decisions-worth-knowing)
- [Limits, and what later slices change](#limits-and-what-later-slices-change)

## What's in this slice

**Built:**

- **G3** runs each criterion's `check`, and **G4** each of the project's `checks`, on `submit` and in the `gates` dry run. They run after G2 and G6, and every check runs, so a refusal lists every one that fails.
- **Each check** is a line of `sh`, run in the project's root with stdin closed. Its command is printed on stderr before it runs. It's stopped once it runs longer than the project's `check_timeout_secs` (300 by default), and then it fails.
- **A failed check** shows its command, its exit code (or that it was stopped) and its last 20 lines of output, under the gate's reason in the text, and as `cmd`, `exit_code`, `timed_out` and `output` in `--json`.
- **`start`** runs each criterion check once, after G1, and records whether each passed in `started.checks`. A check that already passes gets a warning, and the start goes ahead.
- **A passing submit** keeps each check's command, result, time and last 20 lines of output in `verification.checks`. The record is still `unchecked`, with a new note, until slice 8 ties it to a commit.

**Not built yet:**

- A criterion given a check after `start` isn't run then (§2.4). That's TR-2.
- `show` doesn't print `started` or `verification.checks` yet. Slice 8's _Done when_ has `show` print the saved output.
- G5, the commit, `started.head` and auto-verification (slice 8), and T13, which holds an item whose check passed at start (slice 11).

## Try it

Always set `TRACKER_HOME` while developing. Without it, the tool uses your real store at `~/.tracker`.

```sh
cd guild-portfolio/tracker
cargo build

export DEMO=$(mktemp -d)
export TRACKER_HOME=$DEMO/store
export PATH="$PWD/target/debug:$PATH"
mkdir $DEMO/bm && cd $DEMO/bm
tracker init
tracker project add BM --name bookmarks --root .
tracker create "Search filters bookmarks by tag"
tracker criterion add BM-1 "filters by tag" --check 'echo "expected 12 items, received 3" >&2; exit 1'
tracker criterion add BM-1 "old searches still work" --check true
```

Start it. Criterion 1 fails, as it should before the work is done, and criterion 2 already passes, so it gets a warning:

```
$ tracker start BM-1
Running criterion 1's check: `echo "expected 12 items, received 3" >&2; exit 1`
Running criterion 2's check: `true`
warning: Criterion 2's check, `true`, already passes before the work is done, so it can't be proving the new work. Make sure it tests what criterion 2 says, such as with a filter that matches a test that fails now. BM-1 started anyway.
BM-1: open → in-progress
```

A submit runs the checks again, and refuses with the failing one's output:

```
$ tracker submit BM-1
Running criterion 1's check: `echo "expected 12 items, received 3" >&2; exit 1`
Running criterion 2's check: `true`
error: BM-1 can't be submitted: G3 failed. Nothing was changed.
  G3  Criterion 1's check failed: `echo "expected 12 items, received 3" >&2; exit 1` (exit 1). Its last lines of output follow.
        expected 12 items, received 3
```

Project checks and the timeout are set by hand in `$TRACKER_HOME/config.json`, as §4.2 shows:

```json
"BM": { "name": "bookmarks", "root": "…", "checks": ["cargo test"], "check_timeout_secs": 60 }
```

## How it works

### Running one check

`src/checks.rs` runs `sh -c <check>` in the root, which is followed through symlinks first. Stdout and stderr go to one pipe, so the output reads in the order it was printed, and a thread reads it, holding at most the last 64 KiB. Once the check ends, its last 20 lines are kept, without a final line break.

The shell starts in a process group of its own. A check that runs out of time is stopped with `SIGKILL` sent to the whole group, so a test runner the shell started stops too, and the pipe closes. The standard library can't signal a group, so it runs `kill -KILL -- -<group>`. The group is stopped after every check, not only at a timeout, since anything a check left running in the background would hold the pipe open.

A check that can't be run, because the project's root is gone or isn't in `config.json`, fails with `exit_code: null` and a message saying why. A project that isn't in `config.json` fails G4 too, since its project checks aren't known.

### The gates and the lock

`gates::statics` runs G2 and G6, and `gates::scripts` runs G3 and G4. The `gates` dry run runs both without the lock.

`start` and `submit` read the item under the lock, as before, and run the gates that only read files. Then they let the lock go while checks run (§4.4), since a test suite can take minutes and every other command would wait. Afterwards they take it again with `Store::lock_unchanged_after_checks`, which refuses with exit 1 if the item's file changed meanwhile. `start` runs G1 again then, since another item in the project may have started while its checks ran.

### What's recorded

```json
"started": {
  "at": "2026-11-21T05:29:15Z",
  "checks": [{ "criterion": 1, "passed": false }, { "criterion": 2, "passed": true }]
},
"verification": {
  "by": "unchecked",
  "at": "…",
  "checks": [{ "cmd": "cargo test", "ok": true, "secs": 6, "output": "…last 20 lines…" }],
  "note": "Submitted before tracker ran checks against a recorded commit. Gates G2, G6, G3 and G4 passed, but no commit ties the checks to the work."
}
```

`started` sits between `criteria` and `verification`, as §4.3 orders them, and each `start` replaces it. `verification.checks` sits before `note`, so slice 8 can put `commit` and `ref` before it as §4.3 shows. An empty `checks` or `output` is left out, so files from earlier slices load and come back byte-for-byte unchanged.

## JSON output

| Command | Schema | What changed |
| --- | --- | --- |
| `start` | [move.schema.json](../schemas/move.schema.json) | adds `warnings`, always present for `start`, one message per check that already passed. The item has `started`. |
| `gates`, `submit` | [gates.schema.json](../schemas/gates.schema.json), [submit.schema.json](../schemas/submit.schema.json) | `gates` lists G2, G6, G3 and G4. G3 and G4 have `secs`, how long their checks took together. |
| refusals | [error.schema.json](../schemas/error.schema.json) | a failure from a check adds `cmd`, `exit_code` (an integer or `null`), `timed_out` and `output`, all four together |
| every item | [item.schema.json](../schemas/item.schema.json) | `started`, and `verification.checks` |

## Code tour

- `src/checks.rs`: new. `run`, `not_run`, `last_lines`, and the process-group stop.
- `src/gates.rs`: `statics` (G2, G6, renamed from `submit`), `scripts` (G3, G4), `criterion_runs`, which `start` also uses, `Report::checks` and `extend`, and the failure messages and details for a check.
- `src/main.rs`: `apply` takes the lock and lets it go while checks run. `already_passing` writes `start`'s warnings, and `check_record` turns a run into the verification record's entry.
- `src/config.rs`: `checks`, `check_timeout_secs` and `Project::check_timeout`.
- `src/item.rs`: `Started`, `StartedCheck` and `CheckRecord`.
- `src/store.rs`: `lock_unchanged_after_checks`.
- `src/states.rs`: the new `UNCHECKED_NOTE`.

## Testing

### Coverage of the _Done when_ checks

Each of TR-1's criteria runs one test, such as `cargo test --test checks a_passing_checks_last_20_lines_are_kept`.

| Criterion | Test in `tests/checks.rs` |
| --- | --- |
| 1. A failing check shows its command, exit code and last lines of output | `a_failing_check_shows_its_command_exit_code_and_last_lines_of_output`: a check printing 30 lines and exiting 3, through `gates --json` and a text `submit` |
| 2. A check running longer than its timeout is stopped and fails | `a_check_running_longer_than_its_timeout_is_stopped_and_fails`: `sleep 30` under a 1-second timeout ends within 15 seconds, with `timed_out` |
| 3. A broken project check fails G4 even when every criterion check passes | `a_broken_project_check_fails_g4_even_when_every_criterion_check_passes`: a command that doesn't exist exits 127. The criterion check passes only in the root, which tests where checks run |
| 4. `start` records a check that fails, as expected, and warns about one that already passes | `start_records_a_check_that_fails_and_warns_about_one_that_already_passes`, as JSON and as text |
| 5. A passing check's last 20 lines are kept | `a_passing_checks_last_20_lines_are_kept`: lines 11 to 30 of 30, in the submit's verification record |

`tests/fixtures/slice-7/` holds two item files this slice's binary wrote: XX-16, open after a start with one check failing and one passing, and XX-17, done with its checks' output.

### Slice 6 tests that changed

Six tests in `tests/gates.rs` and one in `tests/states.rs` expected the gates to be G2 and G6, as JSON or as text, where they're now G2, G6, G3 and G4, and one in `tests/states.rs` expected the old note's exact text. Their expected values were updated, and nothing they check was removed. `gates_lists_every_failure_and_leaves_the_item_file_unchanged` counted exactly 4 lines on stderr. It now checks that the line printed before the check runs is there, and that there are 4 reasons besides it.

## Decisions worth knowing

- **Every check runs, even after a failure.** §2.4 holds back only the judges until everything before them has passed, so a refusal lists every failing check, as it lists every other reason.
- **G3 and G4 are always listed**, with `secs` of 0 when there was nothing to run, since `gates` lists every gate that ran.
- **The command is printed on stderr**, even with `--json`, so stdout stays one JSON object.
- **The note keeps slice 6's opening words.** "Submitted before tracker ran checks" is still true of a check run against no recorded commit, so the note reads "Submitted before tracker ran checks against a recorded commit."
- **`start` with a check that can't run still starts.** The check counts as not passing, which is what `start` expects, and the submit's G3 says why.

## Limits, and what later slices change

- Checks need `sh` and `kill`, so the tool runs on Unix-like systems only, as the rest of it already assumes.
- A check's processes that leave its process group, such as a daemon that calls `setsid`, aren't stopped.
- `TR` has no project `checks` in its config yet, so G4 runs nothing for this project until they're added by hand, as in §4.2's example.
- Slice 8 adds G5, `started.head`, auto-verification and the commit in the record, and `show` prints the saved output. TR-2 runs a check given after start.

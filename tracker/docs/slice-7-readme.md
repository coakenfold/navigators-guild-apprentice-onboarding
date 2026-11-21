# Slice 7: Running checks

`tracker` is a command-line work tracker, written in Rust, that keeps one central list of work items across many projects and decides when an item is really done. This document covers **slice 7** of the 16 in [PLAN.md](PLAN.md): what it built, how it works, and how to test it. The earlier readmes cover the rest: [slice 1](slice-1-readme.md) the store, `create` and `list`, [slice 2](slice-2-readme.md) the states and `history`, [slice 3](slice-3-readme.md) priority, kind, labels, `edit` and `list`'s filters, [slice 4](slice-4-readme.md) `show` and `delete`, [slice 5](slice-5-readme.md) bad data and polish, and [slice 6](slice-6-readme.md) criteria and gates G1, G2 and G6.

After slice 7 you can **see checks run at start and submit, and fail with their output**. The spec is [DESIGN.md](DESIGN.md). This slice implements gates G3 and G4 and "Checks must fail first" from [§2.4](DESIGN.md#24-gates-is-it-finished), a project's `checks` and `check_timeout_secs` from [§4.2](DESIGN.md#42-config), `started.checks` and `verification.checks` from [§4.3](DESIGN.md#43-item-file), and [§4.4](DESIGN.md#44-handling-bad-data)'s rule that no command holds the lock while checks run. It's the first slice tracked in `tracker` itself, as item TR-1, and a second item, TR-2, adds §2.4's rule that a check given to a criterion after `start` runs once then too.

| | |
| --- | --- |
| Status | Every _Done when_ check has a test, and so does each of TR-2's two criteria. Every test passes, as does the test command. TR-1 is `done`, verified `unchecked` with 8 checks recorded. TR-2 is `done`, verified `unchecked` by the installed slice 6 binary, so G3 never ran its checks and it has no `started` record ([Tracking TR-2](#tracking-tr-2)). PLAN.md has slice 7 as `in review`. There are no reviews yet. The branch isn't merged into `main`. |
| Branch | `feat/tracker/slice-7`, from `main` at `37abcc3`: `6ecf0e6` (TR-1's tests, seen failing before `tracker start TR-1`), `fa65c9a` (the code and schemas), `0ac6b10` (slice 6 tests whose expected gate lists and note changed), `b59d88d` (fixtures), `57118fb` (this readme's first version and PLAN.md's status), `6e6ceda` (this readme's second version), `072bc5d` (two walkthroughs in `docs/`), `6f4d859` (CLAUDE.md: track items with the installed `tracker`), `b2b9c09` (TR-2's tests, committed before `tracker start TR-2`), `e595783` (TR-2's code and schema) |
| Test command | `cargo fmt --check && cargo clippy -- -D warnings && cargo test`, run in `tracker/`. All three pass. |
| Tests | 258: 39 in `tests/create_and_list.rs`, 34 in `tests/organise.rs`, 27 in `tests/states.rs`, 24 in `tests/show_and_delete.rs`, 23 in `tests/criteria.rs`, 22 in `tests/gates.rs`, 14 in `tests/item_files.rs`, 13 in `tests/bad_data.rs`, 11 in `tests/docs_links.rs`, 9 in `tests/fixtures.rs`, 9 in `tests/polish.rs`, 8 in `tests/checks.rs`, 6 in `tests/output.rs`, 5 in `tests/schemas.rs`, 4 in `tests/store_writes.rs`, 2 in `tests/item_ids.rs`, 6 unit tests in the library (4 in `src/gates.rs`, 1 in `src/store.rs`, 1 in `src/checks.rs`) and 2 in `src/main.rs` |

## Contents

- [What's in this slice](#whats-in-this-slice)
- [Try it](#try-it)
- [Commands](#commands)
- [Project checks in config.json](#project-checks-in-configjson)
- [How it works](#how-it-works)
- [JSON output](#json-output)
- [Code tour](#code-tour)
- [Testing](#testing)
- [Decisions worth knowing](#decisions-worth-knowing)
- [Limits, and what later slices change](#limits-and-what-later-slices-change)

## What's in this slice

**Built:**

- **G3** runs each criterion's `check`, and **G4** each of the project's `checks`, on `submit`, `status <id> done` and the `gates` dry run, after G2 and G6. Every check runs, so a refusal lists every one that fails.
- **Each check** is a line of `sh`, run in the project's root with stdin closed. Its command is printed on stderr before it runs. Once it runs longer than the project's `check_timeout_secs`, 300 by default, it's stopped, along with everything it started, and it fails.
- **A failed check** shows its command, its exit code, or that it was stopped, and its last 20 lines of output: under the gate's reason in the text, and as `cmd`, `exit_code`, `timed_out` and `output` in `--json`.
- **`start`**, and `status <id> in-progress`, run each criterion check once after G1 and record whether each passed in `started.checks`. Each check that already passes gets a warning, and the start goes ahead.
- **A check given after `start`** (TR-2), by `criterion add --check` or `criterion set --check` on an in-progress item, runs once then, as `start` runs them. Its result goes into `started.checks`, replacing any earlier one for that criterion, and if it already passes it gets the same warning `start` gives.
- **A passing submit** keeps each check's command, result, time and last 20 lines of output in `verification.checks`. The record is still `unchecked`, with a new note.
- **No command holds the lock while checks run.** `start`, `submit`, and `criterion add` and `criterion set` after start, take it again afterwards and refuse with exit 1 if the item changed meanwhile.

**Not built yet:**

- **`show` doesn't print `started` or `verification.checks`.** [Slice 4's readme](slice-4-readme.md#limits-and-what-later-slices-change) expected slice 7 to add them to `show`. Slice 8's _Done when_ has `show` print the saved output, so they wait for it. `show --json` already has both, since it returns the item.
- G5, the commit, `started.head`, `refs/tracker/<ID>` and auto-verification (slice 8). Until then every submit that passes is `unchecked`.
- T13, which holds an item whose check passed at start (slice 11). Until then the warning is the only sign.
- Judges in G3 and at `start` (slices 12 and 14).

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
```

`project add` takes no checks, so give BM one, and a 2-second timeout, in `config.json`:

```sh
cat > $TRACKER_HOME/config.json <<EOF
{
  "projects": {
    "BM": {
      "name": "bookmarks",
      "root": "$DEMO/bm",
      "checks": ["test -f README.md"],
      "check_timeout_secs": 2
    }
  }
}
EOF
```

Create an item whose first check fails until `tags.txt` exists, and whose second, `true`, passes already:

```sh
tracker create "Search filters bookmarks by tag"
tracker criterion add BM-1 "Filtering by tag shows only tagged bookmarks" --check 'test -f tags.txt || { echo "expected 12 items, received 3" >&2; exit 1; }'
tracker criterion add BM-1 "Old searches still work" --check true
tracker start BM-1
tracker submit BM-1        # exit 2: G3 and G4
tracker criterion add BM-1 "Search answers within 2 seconds" --check 'sleep 10'
tracker gates BM-1         # exit 2: criterion 3 is stopped after 2 seconds
tracker criterion remove BM-1 3
touch tags.txt README.md
tracker submit BM-1
tracker list --status done
cat $TRACKER_HOME/items/BM-1.json
```

The output, with stdout and stderr together:

```
$ tracker init
Created the store at $DEMO/store.
$ tracker project add BM --name bookmarks --root .
Registered BM (bookmarks) at $DEMO/bm.
$ tracker create "Search filters bookmarks by tag"
Created BM-1: Search filters bookmarks by tag
$ tracker criterion add BM-1 "Filtering by tag shows only tagged bookmarks" --check 'test -f tags.txt || { echo "expected 12 items, received 3" >&2; exit 1; }'
Added criterion 1 to BM-1: Filtering by tag shows only tagged bookmarks
$ tracker criterion add BM-1 "Old searches still work" --check true
Added criterion 2 to BM-1: Old searches still work
$ tracker start BM-1
Running criterion 1's check: `test -f tags.txt || { echo "expected 12 items, received 3" >&2; exit 1; }`
Running criterion 2's check: `true`
warning: Criterion 2's check, `true`, already passes before the work is done, so it can't be proving the new work. Make sure it tests what criterion 2 says, such as with a filter that matches a test that fails now. BM-1 started anyway.
BM-1: open → in-progress
$ tracker submit BM-1        # exit 2: G3 and G4
Running criterion 1's check: `test -f tags.txt || { echo "expected 12 items, received 3" >&2; exit 1; }`
Running criterion 2's check: `true`
Running project check: `test -f README.md`
error: BM-1 can't be submitted: G3 and G4 failed. Nothing was changed.
  G3  Criterion 1's check failed: `test -f tags.txt || { echo "expected 12 items, received 3" >&2; exit 1; }` (exit 1). Its last lines of output follow.
        expected 12 items, received 3
  G4  A project check failed: `test -f README.md` (exit 1). It printed nothing.
$ tracker criterion add BM-1 "Search answers within 2 seconds" --check 'sleep 10'
Running criterion 3's check: `sleep 10`
Added criterion 3 to BM-1: Search answers within 2 seconds
BM-1 is in progress, so its check ran once, as `start` runs them, and fails, as it should before the work is done.
$ tracker gates BM-1         # exit 2: criterion 3 is stopped after 2 seconds
Running criterion 1's check: `test -f tags.txt || { echo "expected 12 items, received 3" >&2; exit 1; }`
Running criterion 2's check: `true`
Running criterion 3's check: `sleep 10`
Running project check: `test -f README.md`
error: BM-1 would be refused at submit: G3 and G4 failed. This was a dry run, so nothing was changed.
  G3  Criterion 1's check failed: `test -f tags.txt || { echo "expected 12 items, received 3" >&2; exit 1; }` (exit 1). Its last lines of output follow.
        expected 12 items, received 3
  G3  Criterion 3's check, `sleep 10`, ran longer than 2s, so it was stopped and fails. Make it faster, or raise check_timeout_secs for BM in config.json. It printed nothing.
  G4  A project check failed: `test -f README.md` (exit 1). It printed nothing.
$ tracker criterion remove BM-1 3
Removed criterion 3 from BM-1: Search answers within 2 seconds
$ touch tags.txt README.md
$ tracker submit BM-1
Running criterion 1's check: `test -f tags.txt || { echo "expected 12 items, received 3" >&2; exit 1; }`
Running criterion 2's check: `true`
Running project check: `test -f README.md`
BM-1: in-progress → done
Unchecked. Submitted before tracker ran checks against a recorded commit. Gates G2, G6, G3 and G4 passed, but no commit ties the checks to the work.
$ tracker list --status done
  ID    PRIORITY  KIND    STATUS  TITLE
BM  bookmarks
  BM-1  medium    change  done    Search filters bookmarks by tag

BM only. `--all` for every project.
Filters: status done.
$ cat $TRACKER_HOME/items/BM-1.json
{
  "id": "BM-1",
  "title": "Search filters bookmarks by tag",
  "project": "BM",
  "kind": "change",
  "status": "done",
  "priority": "medium",
  "created": "2026-11-21T06:46:03Z",
  "updated": "2026-11-21T06:46:07Z",
  "criteria": [
    {
      "text": "Filtering by tag shows only tagged bookmarks",
      "check": "test -f tags.txt || { echo \"expected 12 items, received 3\" >&2; exit 1; }"
    },
    {
      "text": "Old searches still work",
      "check": "true"
    }
  ],
  "started": {
    "at": "2026-11-21T06:46:03Z",
    "checks": [
      {
        "criterion": 1,
        "passed": false
      },
      {
        "criterion": 2,
        "passed": true
      },
      {
        "criterion": 3,
        "passed": false
      }
    ]
  },
  "verification": {
    "by": "unchecked",
    "at": "2026-11-21T06:46:07Z",
    "checks": [
      {
        "cmd": "test -f tags.txt || { echo \"expected 12 items, received 3\" >&2; exit 1; }",
        "ok": true,
        "secs": 0
      },
      {
        "cmd": "true",
        "ok": true,
        "secs": 0
      },
      {
        "cmd": "test -f README.md",
        "ok": true,
        "secs": 0
      }
    ],
    "note": "Submitted before tracker ran checks against a recorded commit. Gates G2, G6, G3 and G4 passed, but no commit ties the checks to the work."
  },
  "history": [
    {
      "at": "2026-11-21T06:46:03Z",
      "from": "open",
      "to": "in-progress",
      "via": "start",
      "interactive": false
    },
    {
      "at": "2026-11-21T06:46:07Z",
      "from": "in-progress",
      "to": "done",
      "via": "submit",
      "interactive": false
    }
  ]
}
```

Notes on the output:

- On macOS, `project add` prints the root with `/private` in front of `$DEMO`, since it stores the path with symlinks followed.
- `criterion add` for criterion 3 ran `sleep 10` once, since BM-1 was in progress, and it was stopped after 2 seconds and recorded as `passed: false` (TR-2). The `gates` after it ran it again.
- `started.checks` still has criterion 3 after `criterion remove BM-1 3`. `criterion remove` doesn't touch `started`, so the entry names a criterion that's gone ([Limits](#limits-and-what-later-slices-change)).
- No check printed anything on the passing submit, so `verification.checks` has no `output`. A check that prints, such as `cargo test`, keeps its last 20 lines there, as `tests/fixtures/slice-7/XX-17.json` shows.
- History says `"interactive": false` because the demo ran from a script. Typed at a terminal, it's `true`.

## Commands

No command or flag is new. These change:

| Command | What it does now |
| --- | --- |
| `tracker start <id>`, `status <id> in-progress` | G1, then each criterion check once, without the lock. Then G1 again under the lock, and open → in-progress with `started` recorded. A warning for each check that already passes. |
| `tracker submit <id>`, `status <id> done` | G2 and G6, then G3 and G4 without the lock. Under the lock again, in-progress → done with `verification.checks`, or a refusal listing every failure. |
| `tracker gates <id>` | G2, G6, G3 and G4, without the lock. It changes nothing. |
| `tracker criterion add <id> "text" --check "cmd"`, `criterion set <id> <n> --check "cmd"` | On an in-progress item, the new check runs once, without the lock, and its result goes into `started.checks`. A warning if it already passes. In any other state, or for a manual criterion, nothing runs, as before. `criterion set` with the check the criterion already has changes nothing and runs nothing. |

Exit codes in this slice, as [§3](DESIGN.md#3-interface) defines them:

- `0`: the move was made. A `start` whose check already passes still exits 0, with a warning, and so does a `criterion add` or `criterion set` whose new check already passes.
- `1`: as before, plus the item's file changed while its checks ran, or was removed, for `start`, `submit` and now `criterion add` and `criterion set`. Nothing is written.
- `2`: a gate refused, now including G3 and G4.
- `3`: as before. A check that can't be run is a G3 or G4 failure, not exit 3.

## Project checks in config.json

§4.2's two settings, which `config::Project` now reads. `project add` doesn't set them, so they're written into `config.json` by hand.

| Setting | Default | Meaning |
| --- | --- | --- |
| `checks` | none | Lines of `sh` that G4 runs, in order, in the root, on every submit |
| `check_timeout_secs` | 300 (`config::DEFAULT_CHECK_TIMEOUT_SECS`) | How long any one check, criterion or project, may run before it's stopped |

Both are left out of the file when unset, so `project add`'s output and earlier configs are unchanged.

## How it works

### Running one check

[src/checks.rs](../src/checks.rs) `run`:

1. Prints `Running <what>: `<cmd>`` on stderr, escaped by `text::for_terminal`, so a check can't rewrite the terminal.
2. Starts `sh -c <cmd>` in the root, with stdin closed and stdout and stderr on one pipe from `std::io::pipe`, so the output keeps the order it was printed in. The shell gets a process group of its own with `process_group(0)`.
3. A thread reads the pipe, holding at most the last 64 KiB (`TAIL_BYTES`).
4. `wait` polls every 20 ms until the shell ends or the timeout passes. At the timeout, `stop_group` sends `SIGKILL` to the whole group by running `kill -KILL -- -<group>`, since the standard library can only signal one process.
5. `stop_group` runs again once the shell has ended, so anything the check left in the background stops too, and the pipe closes.
6. `last_lines` keeps the last 20 lines (`KEPT_LINES`), read as UTF-8 with anything else replaced, without trailing blank lines.

A `Run` has `ok` (exit 0 within the time), `exit_code` (`None` when stopped by a signal, at the timeout, or never started), `signal`, `timed_out`, `error`, `secs` (rounded to the nearest second) and `output`. `not_run` makes the `Run` of a check that never started, with `error` saying why.

### G3 and G4

[src/gates.rs](../src/gates.rs):

- `statics` runs G2 then G6. It's slice 6's `submit`, renamed.
- `scripts` runs G3 then G4, and always lists both, with `secs` of 0 when there's nothing to run.
- `criterion_runs` runs each criterion that has a check, numbered from 1, for G3 and for `start`.
- `root` finds the project's root, followed through symlinks. When it can't, each check becomes a failed `not_run` with the reason: a project missing from `config.json`, or a root that's gone or isn't a folder. A project missing from `config.json` also fails G4, since its checks aren't known.
- `check_failure` writes the message: "Criterion 2's check failed: `…` (exit 1)", "…ran longer than 2s, so it was stopped and fails…", or "…couldn't be run: …". It ends with "Its last lines of output follow." or "It printed nothing.", and keeps the `Run` on the `Failure` as `check`.
- `Report::refusal` puts each output line under its failure, indented six spaces. `fail` in [src/main.rs](../src/main.rs) escapes each line, as it does every detail.
- `Report::checks` holds every check that ran, as `Ran`, with its gate, for `secs` and the verification record.

### Checks without the lock

`move_item` and `status` in [src/main.rs](../src/main.rs) take the lock, read the item and pass both to `apply`, which:

- **For `start`:** runs G1 under the lock, refusing with exit 2 if it fails, and loads the config. Then it lets the lock go, runs `gates::criterion_runs`, and takes the lock again with [src/store.rs](../src/store.rs) `Store::lock_unchanged_after_checks`, which refuses with exit 1 if the item's file isn't exactly what was read. Then it runs G1 again, since another item in the project may have started while the checks ran.
- **For `submit`:** runs `statics` under the lock, lets it go, runs `scripts`, and takes it again with `lock_unchanged_after_checks`. Only then does it refuse if any gate failed, so a refusal describes the item as it is.
- **For `gates`:** `gates_dry_run` never takes the lock.

Every start and submit let the lock go, even with no checks to run, so there's one path.

### A check given after start

TR-2. `criterion_add` and `criterion_set` in [src/main.rs](../src/main.rs) read the item and its file's text under the lock with `Store::load_item_with_text`, make the change in memory, then call `run_check_given_after_start`:

1. If the criterion has no check, or the item isn't in progress, it returns at once and nothing runs. An open item's checks run at `start`, and an item in review can be changed only from a terminal.
2. Otherwise it loads the config, lets the lock go, and runs the one check with [src/gates.rs](../src/gates.rs) `criterion_run`, which uses the same `run_in`, `root` and `timeout` as `criterion_runs`, so it prints `Running criterion 3's check: …` and is stopped at `check_timeout_secs`.
3. It takes the lock again with `Store::lock_unchanged_after_checks`, which refuses with exit 1, writing nothing, if the file changed meanwhile, and names the command to run again, such as `tracker criterion add GO-1 "…" --check "…"`.
4. If the item has a `started` record, it removes any entry for that criterion, adds the new one, and sorts by criterion. An item started before slice 7, such as TR-1, has no `started`, so the check runs and warns but nothing is recorded.

The command then writes the item as before. `criterion_output` builds the text and JSON for both commands: a check that already passes gets `already_passing`'s warning, the same one `start` writes, and one that fails adds the line "BM-1 is in progress, so its check ran once, as `start` runs them, and fails, as it should before the work is done."

### What's recorded

- **`started`:** `item::Started`, with `at` and `checks`, a list of `StartedCheck` with `criterion` and `passed`, in criterion order. It sits between `criteria` and `verification`, and each `start` replaces it. A check given after start adds or replaces its criterion's entry ([above](#a-check-given-after-start)). It stays through shelve and reopen, as fixture XX-16 shows. `already_passing` in [src/main.rs](../src/main.rs) writes one warning per passing check.
- **`verification.checks`:** `item::CheckRecord`, with `cmd`, `ok`, `secs` and `output`, made by `check_record`. Criterion checks come first, then the project's, in order. `output` is left out when a check printed nothing. The list sits before `note`, so slice 8 can put `commit` and `ref` before it, as §4.3 shows.
- **`verification.note`:** [src/states.rs](../src/states.rs) `UNCHECKED_NOTE`.

Empty fields are left out, so item files from slices 1–6 load and come back byte-for-byte unchanged.

## JSON output

| Command | Schema | What changed |
| --- | --- | --- |
| `start`, `status <id> in-progress` | [move.schema.json](../schemas/move.schema.json) | `warnings` beside `item`, always present for `start`: one message per check that already passed. `item.started` has every result. Other moves don't have `warnings`. |
| `gates` | [gates.schema.json](../schemas/gates.schema.json) | `gates` lists G2, G6, G3 and G4. G3 and G4 have `secs`, how long their checks took together. |
| `submit`, `status <id> done` | [submit.schema.json](../schemas/submit.schema.json) | the same `gates`, and `item.verification.checks` |
| `start`, `submit` or `gates`, refused | [error.schema.json](../schemas/error.schema.json) | `gates` entries can have `secs`. A failure from a check adds `cmd`, `exit_code` (an integer or `null`), `timed_out` and `output`. The schema requires all four together. |
| every item | [item.schema.json](../schemas/item.schema.json) | `started`, with `at` and `checks`, and `verification.checks` |
| `criterion add`, `criterion set` | [criterion.schema.json](../schemas/criterion.schema.json) | `warnings` beside `criterion` and `item`, always present for these two: start's warning for a check given after start that already passes, and otherwise empty. `criterion remove` and `evidence` don't have it. |

Slice 8 adds `head` to `started` and `commit` and `ref` to `verification`. Slice 11 adds the criteria's fingerprint to `started`, and slice 12 adds `ref`, `answer` and `reason` to judge failures. None of these needs a field here to change.

## Code tour

| File | What's new or changed |
| --- | --- |
| [src/checks.rs](../src/checks.rs) | New. `Run`, `run`, `not_run`, `wait`, `stop_group`, `last_lines`, `KEPT_LINES`, and a unit test for `last_lines` |
| [src/gates.rs](../src/gates.rs) | `Failure::check`, `Report::checks` and `extend`, `Ran`, `statics` (renamed from `submit`), `scripts`, `criterion_checks`, `criterion_runs`, `criterion_run` (TR-2), `run_in`, `root`, `timeout`, `check_failure`. `gates_json` adds `secs`, `failures_json` the check fields, and `refusal` the output lines. |
| [src/main.rs](../src/main.rs) | `apply` takes the lock from `move_item` and `status`, lets it go while checks run, and records `started` and `verification.checks`. New `already_passing` and `check_record`. `gates_dry_run` runs `scripts` too. Help text for `start` and `submit`. For TR-2, `criterion_add` and `criterion_set` read the file's text with the item and call the new `run_check_given_after_start` and `criterion_output`. |
| [src/config.rs](../src/config.rs) | `Project::checks`, `check_timeout_secs`, `check_timeout`, `DEFAULT_CHECK_TIMEOUT_SECS` |
| [src/item.rs](../src/item.rs) | `Item::started`, `Started`, `StartedCheck`, `Verification::checks`, `CheckRecord` |
| [src/store.rs](../src/store.rs) | `Store::lock_unchanged_after_checks` |
| [src/states.rs](../src/states.rs) | `UNCHECKED_NOTE`'s new text |
| [src/lib.rs](../src/lib.rs) | `pub mod checks` |
| [schemas/](../schemas/) | `error`, `gates`, `move`, `submit`, `item` and, for TR-2, `criterion`, as [above](#json-output) |
| [tests/checks.rs](../tests/checks.rs) | New: one test per _Done when_ check, and three for TR-2's criteria, with `COUNTED`, a check that counts its runs in a file, and `runs` |
| [tests/fixtures.rs](../tests/fixtures.rs), `tests/fixtures/slice-7/` | `slice_7_wrote_two_fixtures_with_check_results`, and XX-16 and XX-17 |

To run checks from another command, such as slice 8's shelve or slice 9's `approve`: call `gates::scripts`, `gates::criterion_runs` or `gates::criterion_run` after dropping the lock, then take it again with `Store::lock_unchanged_after_checks` before writing. `run_check_given_after_start` is the smallest example.

## Testing

Run the test command in `tracker/`. TR-1's criteria each run one test, for example `cargo test --test checks a_passing_checks_last_20_lines_are_kept`. The harness in [tests/common/mod.rs](../tests/common/mod.rs) didn't change. Its `Env::create` gives each item the check `true`, so every test that starts an item now runs it, gets the already-passing warning on stderr, and goes ahead.

### Coverage of the _Done when_ checks

| _Done when_ | Test in `tests/checks.rs` |
| --- | --- |
| A failing check shows its command, exit code and last lines of output. | `a_failing_check_shows_its_command_exit_code_and_last_lines_of_output`: a check that prints 30 lines and exits 3. `gates --json` has `cmd`, `exit_code` 3 and exactly lines 11–30, and stderr has the command. A text `submit` prints lines 11–30 and not line 10. |
| A check running longer than its timeout is stopped and fails. | `a_check_running_longer_than_its_timeout_is_stopped_and_fails`: `echo before; sleep 30; echo after` under a 1-second timeout. `sleep` is the shell's child, so only stopping the group ends it within the test's 15 seconds. `timed_out` is true, `exit_code` null, and `output` is `before`. |
| A broken project check fails G4 even when every criterion check passes. | `a_broken_project_check_fails_g4_even_when_every_criterion_check_passes`: a command that doesn't exist exits 127 in G4, while G3 passes. The criterion check, `test -f marker`, passes only in the root, so it also tests where checks run. Fixed, `gates` passes. |
| `start` records a check that fails, as expected, and warns about one that already passes. | `start_records_a_check_that_fails_and_warns_about_one_that_already_passes`: `false`, `true` and a manual criterion. `started.checks` is `[{1, false}, {2, true}]`, and there's one warning, about criterion 2, in `--json` and as text. |
| A passing check's last 20 lines are kept. | `a_passing_checks_last_20_lines_are_kept`: a check printing 30 lines, and a project check. `verification.checks` keeps lines 11–30 and the project check's line, in the submit's JSON and the file. |

All five were committed in `6ecf0e6` and seen failing, each with exit 101, before `tracker start TR-1`. They pass from `fa65c9a`.

### Coverage of TR-2's criteria

TR-2 comes from §2.4's "A criterion that gets a new check after `start` is run once at that moment as well", not from PLAN.md. Its criteria, and the tests their checks run:

| TR-2's criterion | Check | Tests in `tests/checks.rs` |
| --- | --- | --- |
| 1. A check added or set on an in-progress item with `criterion add` or `criterion set` runs once, and its result is recorded in `started.checks`. | `cargo test --test checks check_given_after_start_by` | `check_given_after_start_by_add_runs_once_and_is_recorded`: a check added before start isn't run until `start`, one added after start runs once and joins `started.checks` as criterion 3 with `warnings` empty, and a manual criterion runs nothing. `check_given_after_start_by_set_runs_once_and_is_recorded`: a manual criterion given a check runs it and joins the record, a new check for criterion 1 replaces its entry rather than adding a second, and the same check again runs nothing. Both count runs with `COUNTED`, in the item file and the JSON. |
| 2. A check added that way which already passes gets the same warning `start` gives. | `cargo test --test checks check_given_after_start_that_already_passes` | `check_given_after_start_that_already_passes_is_warned_about`: `criterion add --check true` after start gives exactly the warning `start` gave for the same check on another item, and `criterion set` as text prints one `warning:` line naming criterion 3. |

The filters match two tests and one, as `cargo test` shows. All three were committed in `b2b9c09`, before `tracker start TR-2`. Nothing records that they were run then, so for this readme they were run against `b2b9c09`'s files: all three fail, with exit 101, on their run counts and the warning. They pass from `e595783`.

### Other tests

- `src/checks.rs` `last_lines_keeps_the_last_twenty_without_a_final_break`: 25 lines keep the last 20, `\r\n` and trailing blank lines are dropped, and empty output stays empty.
- `tests/gates.rs` `gates_lists_every_failure_and_leaves_the_item_file_unchanged` now checks that the only extra line on stderr is `Running criterion 3's check: `true``.
- `tests/gates.rs` `ten_starts_at_once_in_one_project_let_exactly_one_through`, from slice 6, now goes through the unlocked checks and the second G1 on every start, and still lets exactly one through.
- `tests/fixtures/slice-7/` holds two files this slice's binary wrote: XX-16, started with one check failing and one passing, then shelved and reopened, and XX-17, done with a criterion check's 20 kept lines and a project check's output. `slice_7_wrote_two_fixtures_with_check_results` checks them, and the existing fixture tests load and rewrite them byte-for-byte.

### Slice 6 tests that changed

`0ac6b10` updated the expected values of tests whose old values slice 7 makes untrue. Nothing they check was removed:

| Test | Was | Now |
| --- | --- | --- |
| `tests/gates.rs`: `a_manual_criterion_with_no_evidence_fails_g2`, `an_item_with_no_criteria_fails_g2`, `a_script_criterion_needs_no_evidence`, `a_design_naming_a_missing_section_fails_g6`, `gates_lists_every_failure_and_leaves_the_item_file_unchanged` | `gates` of G2 and G6 | G2, G6, G3 and G4, with `secs` |
| `tests/gates.rs`: `gates_says_when_the_item_isnt_ready_to_submit` | "passes the gates submit runs: G2 and G6." | "…: G2, G6, G3 and G4." |
| `tests/gates.rs`: `gates_lists_every_failure_and_leaves_the_item_file_unchanged` | exactly 4 lines on stderr | the `Running …` line and 4 reasons |
| `tests/states.rs`: `submit_prints_the_shape_section_3_gives_it` | `gates` of G2 and G6 | G2, G6, G3 and G4 |
| `tests/states.rs`: `the_text_output_names_the_move` | slice 6's note | the new note |

Each failed after `fa65c9a`, before its update. DESIGN.md §5 says not to edit an existing test just to pass, but to fix a test that's wrong and say so. Slice 9's T4 will hold an item for exactly this kind of change.

### Checks done by hand

Each was run with this branch's binary in a temporary `TRACKER_HOME`. None has an automated test:

| What | How | Result |
| --- | --- | --- |
| Check output is escaped | A failing check printing `a\033[2Kb` | The text shows `a\u{1b}[2Kb`. The JSON `output` keeps the raw escape. |
| An item changed while its checks ran | `start` on a check `sleep 2; false`, with `edit --priority high` run after 0.7 seconds | Exit 1, "BM-1 changed while its checks ran, so nothing was changed. Run `tracker start BM-1` again.". The item stays open with no `started`. |
| A root that's gone | `rmdir` the root, then `start` and `gates` | `start` goes ahead and records `passed: false` with no warning. `gates` fails G3: "couldn't be run: GO's root, …, can't be read: No such file or directory". |
| A project removed from `config.json` | Delete GO from the config, then `gates` and `shelve` | G3 and G4 both fail, saying GO isn't in `config.json`. `shelve` still works. |
| An item changed while a check given after start ran | `criterion add GO-1 two --check 'sleep 2; false'` on an in-progress item, with `edit --priority high` run after 0.7 seconds | Exit 1, "GO-1 changed while its checks ran, so nothing was changed. Run `tracker criterion add GO-1 "…" --check "…"` again.". The item still has one criterion and its `started` as it was. |
| A check given after start to an item with no `started` | `started` taken out of an in-progress item's file, as for one started before slice 7, then `criterion add GO-1 three --check true` | Exit 0. The check ran and got start's warning, which ends "GO-1 started anyway.". The criterion was added, and the file still has no `started`. |

### Tracking TR-1

TR-1 was started with the installed slice 6 binary, which doesn't run checks, so its file has no `started`. Its criterion 4 had lost the word `start` when it was created, and was fixed by hand in the item file and recorded with `criterion accept` before the submit. `gates` and `submit` were run with this branch's `target/debug/tracker`, so G3 ran the five tests and G4 ran TR's `cargo fmt --check`, `cargo clippy -- -D warnings` and `cargo test`. TR-1 reached done `unchecked` at 2026-11-21T05:34:16Z with those 8 checks recorded. That breaks PLAN.md's bootstrap rule ([integration risks](PLAN.md#integration-risks)), which tracks each slice with the binary installed from the slice before it: the installed one would have run no check and passed TR-1 on G2 and G6 alone.

### Tracking TR-2

TR-2 was created while TR-1 was being built, as out-of-scope work. It was tracked with the installed slice 6 binary throughout, as the line `6f4d859` adds to [CLAUDE.md](../CLAUDE.md) asks: started at 2026-11-21T06:28:22Z, 11 seconds after `b2b9c09` committed its tests, and submitted at 2026-11-21T06:43:30Z. So:

- Its file has no `started`, since slice 6 doesn't write one.
- It reached done `unchecked` with slice 6's note, "Submitted before tracker ran checks. Gates G2 and G6 passed, but nothing tested the work.", and no `verification.checks`. G3 never ran its two checks, and G4 never ran TR's project checks. Both checks, and the test command, pass when run by hand on this branch ([above](#coverage-of-tr-2s-criteria)), but nothing in its record says so.
- The slice 6 binary can't show TR-1 in full: `tracker show TR-1` ends by saying TR-1's file has fields it doesn't know, in `verification`, since TR-1 was submitted by this branch's binary.

`6f4d859`'s header is `git commit -m 'docs: track items with the installed tracker'`, the command typed into the message by mistake, so it isn't a Conventional Commit. It and the other two docs commits, `6e6ceda` and `072bc5d`, are unscoped, but were made between TR-1's done and TR-2's start, while no item was in progress, so slice 11's T14 wouldn't count them against either.

## Decisions worth knowing

- **Every check runs, even after one fails.** §2.4 holds back only the judges until everything before them passes, so a refusal lists every failing check, as it lists every other reason.
- **G3 and G4 are always listed**, with `secs` 0 when there was nothing to run, since `gates` lists every gate that ran.
- **The command goes to stderr, even with `--json`**, so stdout stays one JSON object, and an agent still sees what's running.
- **A check's whole process group is stopped**, at the timeout and again once the shell ends. A test runner the shell started would otherwise outlive the timeout and hold the output open.
- **Stdout and stderr are kept together, in order**, since a failure message often goes to stderr after the progress that led to it on stdout.
- **`start`'s warnings are also `warnings` in `--json`**, since an `Output`'s warnings are printed only as text.
- **A check that can't run counts as failing at start.** `start` expects failures, so it goes ahead, and the submit's G3 says why. This settles [slice 2's open question](slice-2-readme.md#still-open) for checks: a move on an item whose project was removed from `config.json` still works, but G3 and G4 fail it at submit.
- **`verification.by` stays `unchecked`.** `auto` means proven against a commit, which needs slice 8. The note keeps slice 6's opening words, since checks have now run but not against a recorded commit: "Submitted before tracker ran checks against a recorded commit. Gates G2, G6, G3 and G4 passed, but no commit ties the checks to the work."
- **The re-read after checks is already here.** Slice 8's prompt lists it, but §4.4 requires it as soon as checks run without the lock. Its _Done when_ test, a check that edits the item file, is still slice 8's.
- **`started.checks` records only criterion checks.** Project checks aren't expected to fail at start, so nothing reads their result then.
- **A check given after start runs only on an in-progress item.** An open item's check runs at `start` anyway. An item in review takes criterion changes only from a terminal, and T5 fires on them (§2.4 notes it fires for a new check after start too). A done item refuses them.
- **A check given after start replaces its criterion's entry in `started.checks`**, rather than adding a second, so each criterion has at most one entry and slice 11's T13 reads one result for it: the latest check's, run before any work on it.
- **The warning is `start`'s, word for word**, as TR-2's criterion 2 asks, so it ends "BM-1 started anyway." even though the start was earlier.
- **`criterion set` with the check a criterion already has runs nothing**, since nothing changed, and its `warnings` is empty.

## Limits, and what later slices change

- **Slice 8** adds G5 between G2 and G6 (§2.4's cheapest-first order), `started.head` and `criteria_hash` before `started.checks` (§4.3's order), and `commit` and `ref` before `verification.checks`. It replaces `unchecked` with `auto` and drops `UNCHECKED_NOTE` from new records, but files with it must still load. Shelving an in-progress item needs G4, so it needs the project-check half of `gates::scripts` on its own. Its test that a check editing the item file makes `submit` exit 1 covers `lock_unchanged_after_checks`, which has no automated test yet. `show` prints `started` and each check's saved output, which must go through `Styled::push`, escaped with its line breaks kept, as [slice 5's readme](slice-5-readme.md#limits-and-what-later-slices-change) asks.
- **Slice 9**'s triggers run after `scripts` passes. `approve` re-runs the gates and must let the lock go and re-read the item in the same way.
- **Slice 11**'s T13 reads `started.checks[].passed`. Each entry is numbered by criterion position when its check ran, at start or when it was given after start. `criterion remove` doesn't touch `started`, so a criterion removed after start leaves its entry behind, naming a criterion that's gone or the one that moved up into its place, as the demo's criterion 3 shows. T5 fires on that change anyway. T13 must treat an item started before slice 7, with no `started`, such as TR-1 and TR-2, as having nothing to compare.
- **Slice 12** runs judges in G3 only after every check in G3 and G4 passed, and adds their failures, with `ref`, `answer` and `reason`, beside the check failures. `criterion_checks` must keep skipping judge criteria.
- **Slice 14** runs judges at `start` too, alongside `criterion_runs`, and records them in `started`.
- **Slice 12** adds `criterion add --judge` and `criterion set --judge`. A judge criterion has no `check`, so `run_check_given_after_start` returns early for one, and a judge given after start won't run then unless a later slice adds it. §2.4 says a new check after start runs once, and says nothing of a new judge, so slice 14, which runs judges at `start`, should decide whether one runs then too.
- **Slice 16**'s digest lists the checks that ran for each auto-verified item, from `verification.checks`.
- **Checks need `sh` and `kill`**, so the tool runs on Unix-like systems only. A process that leaves the check's group, such as a daemon that calls `setsid`, isn't stopped.
- **The last 20 lines of a whole suite may not show its totals.** TR-1's `cargo test` record ends with the doc-tests' "0 passed", since `cargo test` prints one summary per test target. A criterion check that runs one test, as §5 asks, doesn't have this problem.
- **`check_timeout_secs` applies to each check, not to a gate**, so a submit can take as long as the timeout times the number of checks. Checks run one at a time.
- **Times are rounded to the nearest second**, so a quick check records `secs: 0`.
- **`start` doesn't warn when a check can't run**, such as when the root is gone. It records `passed: false`, the same as an expected failure, and only the submit's G3 says why.

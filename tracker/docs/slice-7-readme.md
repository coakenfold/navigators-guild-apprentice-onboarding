# Slice 7: Running checks

`tracker` is a command-line work tracker, written in Rust, that keeps one central list of work items across many projects and decides when an item is really done. This document covers **slice 7** of the 16 in [PLAN.md](PLAN.md): what it built, how it works, and how to test it. The earlier readmes cover the rest: [slice 1](slice-1-readme.md) the store, `create` and `list`, [slice 2](slice-2-readme.md) the states and `history`, [slice 3](slice-3-readme.md) priority, kind, labels, `edit` and `list`'s filters, [slice 4](slice-4-readme.md) `show` and `delete`, [slice 5](slice-5-readme.md) bad data and polish, and [slice 6](slice-6-readme.md) criteria and gates G1, G2 and G6.

After slice 7 you can **see checks run at start and submit, and fail with their output**. The spec is [DESIGN.md](DESIGN.md). This slice implements gates G3 and G4 and "Checks must fail first" from [§2.4](DESIGN.md#24-gates-is-it-finished), a project's `checks` and `check_timeout_secs` from [§4.2](DESIGN.md#42-config), `started.checks` and `verification.checks` from [§4.3](DESIGN.md#43-item-file), and [§4.4](DESIGN.md#44-handling-bad-data)'s rule that no command holds the lock while checks run. It's the first slice tracked in `tracker` itself, as item TR-1. A second item, TR-2, adds §2.4's rule that a check given to a criterion after `start` runs once then too, and a third, TR-3, keeps `started.checks` on the right criteria after `criterion remove`. [Review 1](reviews/slice-7-review-1.md)'s fixes are items TR-13 to TR-18 ([below](#fixes-after-review-1)), and [review 2](reviews/slice-7-review-2.md)'s are TR-19 and TR-20 ([below](#fixes-after-review-2)).

| | |
| --- | --- |
| Status | Every _Done when_ check has a test, and so does each criterion of TR-2, TR-3, TR-13 to TR-18 and TR-20 that has a check. TR-17's three criteria and TR-19's four are manual, with evidence naming the passages. Every test passes, as does the test command. TR-1 is `done`, verified `unchecked` with 8 checks recorded. TR-2, TR-3 and TR-13 to TR-20 are `done`, verified `unchecked` by the installed slice 6 binary, so G3 never ran their checks and none has a `started` record ([Tracking TR-2](#tracking-tr-2), [Tracking TR-3](#tracking-tr-3), [Tracking review 1's fixes](#tracking-review-1s-fixes), [Tracking review 2's fixes](#tracking-review-2s-fixes)). From [review 1](reviews/slice-7-review-1.md), bugs 1, 4 and 5, findings 7 and 8, finding 6's docs half and both test gaps are fixed, bugs 2 and 3 won't be fixed, and process 1, 2 and 7 are done ([below](#fixes-after-review-1)). From [review 2](reviews/slice-7-review-2.md), findings 1 and 2, both test gaps and process 1 and 2 are fixed, and so is review 1's open schema wording for bug 5. Bug 1's missing message won't be fixed ([below](#fixes-after-review-2)). Still open: review 1's finding 6's test-helper half (slice 11), and installing this slice's binary before TR-4 starts ([Still open](#still-open)). Slices 8–16 have items, TR-4 to TR-12. PLAN.md has slice 7 as `in review`. The branch isn't merged into `main`. |
| Branch | `feat/tracker/slice-7`, from `main` at `37abcc3`: `6ecf0e6` (TR-1's tests, seen failing before `tracker start TR-1`), `fa65c9a` (the code and schemas), `0ac6b10` (slice 6 tests whose expected gate lists and note changed), `b59d88d` (fixtures), `57118fb` (this readme's first version and PLAN.md's status), `6e6ceda` (this readme's second version), `072bc5d` (two walkthroughs in `docs/`), `a270b4d` (CLAUDE.md: track items with the installed `tracker`), `64ec7f7` (TR-2's tests, committed before `tracker start TR-2`), `513da4d` (TR-2's code and schema), `769909d` (this readme, for TR-2), `d7f8b60` (TR-3's test), `ffb8255` (TR-3's fix), `9ed6f65` (TR-3's notes in this readme), `40a7c7a` (this readme's hashes, after a rebase), `3ec8698` (review 1), `065a308` (the creating-items walkthrough's note that slices 8–16 are TR-4 to TR-12), then review 1's fixes, `f7ef9dd` to `02ef286` ([below](#fixes-after-review-1)), `464c239` (this readme's record of them), `30d2a4d` (review 1's Outcome), `83740e4` (review 2), then review 2's fixes, `61cb404`, `8be2f4b` and `eb80e0f` (TR-19) and `1549d7d` (TR-20) ([below](#fixes-after-review-2)), and this readme's record of them |
| Test command | `cargo fmt --check && cargo clippy -- -D warnings && cargo test`, run in `tracker/`. All three pass. |
| Tests | 269: 39 in `tests/create_and_list.rs`, 34 in `tests/organise.rs`, 27 in `tests/states.rs`, 24 in `tests/show_and_delete.rs`, 23 in `tests/criteria.rs`, 22 in `tests/gates.rs`, 18 in `tests/checks.rs`, 14 in `tests/item_files.rs`, 13 in `tests/bad_data.rs`, 11 in `tests/docs_links.rs`, 10 in `tests/fixtures.rs`, 9 in `tests/polish.rs`, 6 in `tests/output.rs`, 5 in `tests/schemas.rs`, 4 in `tests/store_writes.rs`, 2 in `tests/item_ids.rs`, 6 unit tests in the library (4 in `src/gates.rs`, 1 in `src/store.rs`, 1 in `src/checks.rs`) and 2 in `src/main.rs` |

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
- [Fixes after review 1](#fixes-after-review-1)
- [Fixes after review 2](#fixes-after-review-2)
- [Still open](#still-open)
- [Limits, and what later slices change](#limits-and-what-later-slices-change)

## What's in this slice

**Built:**

- **G3** runs each criterion's `check`, and **G4** each of the project's `checks`, on `submit`, `status <id> done` and the `gates` dry run, after G2 and G6. Every check runs, so a refusal lists every one that fails.
- **Each check** is a line of `sh`, run in the project's root with stdin closed. Its command is printed on stderr before it runs. Once it runs longer than the project's `check_timeout_secs`, 300 by default, it's stopped, along with everything it started, and it fails. Something it started that left its process group isn't stopped, but `tracker` waits at most a second more for its output, whether the check timed out or ended (TR-13, and TR-20's test for one that passes).
- **A failed check** shows its command, its exit code, or that it was stopped, and its last 20 lines of output, each cut to 500 characters (TR-15): under the gate's reason in the text, and as `cmd`, `exit_code`, `timed_out` and `output` in `--json`.
- **`start`**, and `status <id> in-progress`, run each criterion check once after G1 and record whether each passed in `started.checks`. Each check that already passes gets a warning, and the start goes ahead.
- **A check given after `start`** (TR-2), by `criterion add --check` or `criterion set --check` on an in-progress item, runs once then, as `start` runs them. Only an in-progress item runs it, as §2.4 now says (TR-19). Its result goes into `started.checks`, replacing any earlier one for that criterion, and if it already passes it gets the same warning `start` gives. A newer tracker's file is refused with exit 3 before the check runs (TR-14).
- **`criterion remove`** after start drops the removed criterion's entry from `started.checks` and renumbers the later ones (TR-3).
- **A passing submit** keeps each check's criterion, command, result, time and last 20 lines of output in `verification.checks`. A project check names no criterion (TR-16). The numbers are as the criteria were at that submit, and nothing renumbers them later, not even in `previous_verification` after a reopen and a `criterion remove`, as §4.3 now says (TR-19). The record is still `unchecked`, with a new note.
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
  "created": "2026-11-21T07:44:24Z",
  "updated": "2026-11-21T07:44:29Z",
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
    "at": "2026-11-21T07:44:24Z",
    "checks": [
      {
        "criterion": 1,
        "passed": false
      },
      {
        "criterion": 2,
        "passed": true
      }
    ]
  },
  "verification": {
    "by": "unchecked",
    "at": "2026-11-21T07:44:29Z",
    "checks": [
      {
        "criterion": 1,
        "cmd": "test -f tags.txt || { echo \"expected 12 items, received 3\" >&2; exit 1; }",
        "ok": true,
        "secs": 0
      },
      {
        "criterion": 2,
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
      "at": "2026-11-21T07:44:24Z",
      "from": "open",
      "to": "in-progress",
      "via": "start",
      "interactive": false
    },
    {
      "at": "2026-11-21T07:44:29Z",
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
- `started.checks` has no criterion 3 after `criterion remove BM-1 3`: removing a criterion drops its entry and moves the later ones up with their criteria (TR-3).
- In `verification.checks`, the two criterion checks name their `criterion`, and the project check names none (TR-16). Until TR-16, no entry had it.
- No check printed anything on the passing submit, so `verification.checks` has no `output`. A check that prints, such as `cargo test`, keeps its last 20 lines there, as `tests/fixtures/slice-7/XX-17.json` shows.
- History says `"interactive": false` because the demo ran from a script. Typed at a terminal, it's `true`.

## Commands

No command or flag is new. These change:

| Command | What it does now |
| --- | --- |
| `tracker start <id>`, `status <id> in-progress` | G1, then each criterion check once, without the lock. Then G1 again under the lock, and open → in-progress with `started` recorded. A warning for each check that already passes. |
| `tracker submit <id>`, `status <id> done` | G2 and G6, then G3 and G4 without the lock. Under the lock again, in-progress → done with `verification.checks`, or a refusal listing every failure. |
| `tracker gates <id>` | G2, G6, G3 and G4, without the lock. It changes nothing. |
| `tracker criterion add <id> "text" --check "cmd"`, `criterion set <id> <n> --check "cmd"` | On an in-progress item, the new check runs once, without the lock, and its result goes into `started.checks`. A warning if it already passes. In any other state, or for a manual criterion, nothing runs, as before. `criterion set` with the check the criterion already has changes nothing and runs nothing. A newer tracker's file is refused before anything runs (TR-14). |

Exit codes in this slice, as [§3](DESIGN.md#3-interface) defines them:

- `0`: the move was made. A `start` whose check already passes still exits 0, with a warning, and so does a `criterion add` or `criterion set` whose new check already passes.
- `1`: as before, plus the item's file changed while its checks ran, or was removed, for `start`, `submit` and now `criterion add` and `criterion set`. Nothing is written.
- `2`: a gate refused, now including G3 and G4.
- `3`: as before, and now also `criterion add` and `criterion set` on a file with fields this version doesn't know, before any check runs (TR-14). A check that can't be run is a G3 or G4 failure, not exit 3.

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
3. A thread reads the pipe into a buffer it shares with `run`, holding at most the last 64 KiB (`TAIL_BYTES`). It ends an `mpsc` channel when the pipe closes.
4. `wait` polls every 20 ms until the shell ends or the timeout passes. At the timeout, `stop_group` sends `SIGKILL` to the whole group by running `kill -KILL -- -<group>`, since the standard library can only signal one process.
5. `stop_group` runs again once the shell has ended, so anything the check left in the background stops too, and the pipe closes. `run` then waits for the channel to end with `recv_timeout`, for at most 1 second (`LEFTOVER_OUTPUT`), and goes on with what the buffer holds. Only a process that left the group, such as one that called `setsid`, can hold the pipe open past then (TR-13). This happens whether the shell timed out or ended by itself, so a passing check that leaves a detached server running still passes about a second after it ends (TR-20's test).
6. `last_lines` keeps the last 20 lines (`KEPT_LINES`), read as UTF-8 with anything else replaced, without trailing blank lines. `cut` shortens each line longer than 500 characters (`KEPT_LINE_CHARS`) to 499 and `…` (TR-15).

A `Run` has `ok` (exit 0 within the time), `exit_code` (`None` when stopped by a signal, at the timeout, or never started), `signal`, `timed_out`, `error`, `secs` (rounded to the nearest second) and `output`. `not_run` makes the `Run` of a check that never started, with `error` saying why.

### G3 and G4

[src/gates.rs](../src/gates.rs):

- `statics` runs G2 then G6. It's slice 6's `submit`, renamed.
- `scripts` runs G3 then G4, and always lists both, with `secs` of 0 when there's nothing to run.
- `criterion_runs` runs each criterion that has a check, numbered from 1, for G3 and for `start`.
- `root` finds the project's root, followed through symlinks. When it can't, each check becomes a failed `not_run` with the reason: a project missing from `config.json`, or a root that's gone or isn't a folder. A project missing from `config.json` also fails G4, since its checks aren't known.
- `check_failure` writes the message: "Criterion 2's check failed: `…` (exit 1)", "…ran longer than 2s, so it was stopped and fails…", or "…couldn't be run: …". It ends with "Its last lines of output follow." or "It printed nothing.", and keeps the `Run` on the `Failure` as `check`.
- `Report::refusal` puts each output line under its failure, indented six spaces. `fail` in [src/main.rs](../src/main.rs) escapes each line, as it does every detail.
- `Report::checks` holds every check that ran, as `Ran`, with its gate and, for G3, its criterion (TR-16), for `secs` and the verification record.

### Checks without the lock

`move_item` and `status` in [src/main.rs](../src/main.rs) take the lock, read the item and pass both to `apply`, which:

- **For `start`:** runs G1 under the lock, refusing with exit 2 if it fails, and loads the config. Then it lets the lock go, runs `gates::criterion_runs`, and takes the lock again with [src/store.rs](../src/store.rs) `Store::lock_unchanged_after_checks`, which refuses with exit 1 if the item's file isn't exactly what was read. Then it runs G1 again, since another item in the project may have started while the checks ran.
- **For `submit`:** runs `statics` under the lock, lets it go, runs `scripts`, and takes it again with `lock_unchanged_after_checks`. Only then does it refuse if any gate failed, so a refusal describes the item as it is.
- **For `gates`:** `gates_dry_run` never takes the lock.

Every start and submit let the lock go, even with no checks to run, so there's one path.

### A check given after start

TR-2. `criterion_add` and `criterion_set` in [src/main.rs](../src/main.rs) read the item and its file's text under the lock with `Store::load_item_with_text`, and refuse a newer tracker's file at once with `refuse_newer_file`, as `gates` and every move do, so no check runs for a change that couldn't be written (TR-14). They make the change in memory, then call `run_check_given_after_start`:

1. If the criterion has no check, or the item isn't in progress, it returns at once and nothing runs. An open item's checks run at `start`, and an item in review can be changed only from a terminal. §2.4 says so, and that `started.checks` then keeps the old check's result, or none for a criterion that was manual (TR-19).
2. Otherwise it loads the config, lets the lock go, and runs the one check with [src/gates.rs](../src/gates.rs) `criterion_run`, which uses the same `run_in`, `root` and `timeout` as `criterion_runs`, so it prints `Running criterion 3's check: …` and is stopped at `check_timeout_secs`.
3. It takes the lock again with `Store::lock_unchanged_after_checks`, which refuses with exit 1, writing nothing, if the file changed meanwhile, and names the command to run again, such as `tracker criterion add GO-1 "…" --check "…"`.
4. If the item has a `started` record, it removes any entry for that criterion, adds the new one, and sorts by criterion. An item started before slice 7, such as TR-1, has no `started`, so the check runs and warns but nothing is recorded.

The command then writes the item as before. `criterion_output` builds the text and JSON for both commands: a check that already passes gets `already_passing`'s warning, the same one `start` writes, and one that fails adds the line "BM-1 is in progress, so its check ran once, as `start` runs them, and fails, as it should before the work is done."

### What's recorded

- **`started`:** `item::Started`, with `at` and `checks`, a list of `StartedCheck` with `criterion` and `passed`, in criterion order. It sits between `criteria` and `verification`, and each `start` replaces it. A check given after start adds or replaces its criterion's entry ([above](#a-check-given-after-start)), and `criterion remove` drops the removed criterion's entry and renumbers the later ones (TR-3). It stays through shelve and reopen, as fixture XX-16 shows. `already_passing` in [src/main.rs](../src/main.rs) writes one warning per passing check.
- **`verification.checks`:** `item::CheckRecord`, with `criterion`, `cmd`, `ok`, `secs` and `output`, made by `check_record`. Criterion checks come first, each naming its criterion, numbered from 1, then the project's, in order, which name none (TR-16). `criterion` is left out for a project check, and in records written before TR-16, and `output` when a check printed nothing. A reopen moves the record to `previous_verification`, and `criterion_remove` never renumbers it there, so its numbers are as at that submit, and a reader matches its checks to today's criteria by `cmd` (§4.3, TR-19). The list sits before `note`, so slice 8 can put `commit` and `ref` before it, as §4.3 shows.
- **`verification.note`:** [src/states.rs](../src/states.rs) `UNCHECKED_NOTE`.

Empty fields are left out, so item files from slices 1–6 load and come back byte-for-byte unchanged.

## JSON output

| Command | Schema | What changed |
| --- | --- | --- |
| `start`, `status <id> in-progress` | [move.schema.json](../schemas/move.schema.json) | `warnings` beside `item`, always present for `start`: one message per check that already passed. `item.started` has every result. Other moves don't have `warnings`. |
| `gates` | [gates.schema.json](../schemas/gates.schema.json) | `gates` lists G2, G6, G3 and G4. G3 and G4 have `secs`, how long their checks took together. |
| `submit`, `status <id> done` | [submit.schema.json](../schemas/submit.schema.json) | the same `gates`, and `item.verification.checks` |
| `start`, `submit` or `gates`, refused | [error.schema.json](../schemas/error.schema.json) | `gates` entries can have `secs`. A failure from a check adds `cmd`, `exit_code` (an integer or `null`), `timed_out` and `output`. The schema requires all four together. |
| every item | [item.schema.json](../schemas/item.schema.json) | `started`, with `at` and `checks`, and `verification.checks`, whose entries for criterion checks have `criterion` (TR-16). Its `output` description, like `error.schema.json`'s, says each kept line is cut at 500 characters (TR-19) |
| `criterion add`, `criterion set` | [criterion.schema.json](../schemas/criterion.schema.json) | `warnings` beside `criterion` and `item`, always present for these two: start's warning for a check given after start that already passes, and otherwise empty. `criterion remove` and `evidence` don't have it. |

Slice 8 adds `head` to `started` and `commit` and `ref` to `verification`. Slice 11 adds the criteria's fingerprint to `started`, and slice 12 adds `ref`, `answer` and `reason` to judge failures. None of these needs a field here to change.

## Code tour

| File | What's new or changed |
| --- | --- |
| [src/checks.rs](../src/checks.rs) | New. `Run`, `run`, `not_run`, `wait`, `stop_group`, `last_lines`, `KEPT_LINES`, and a unit test for `last_lines`. For TR-13, `LEFTOVER_OUTPUT` and the reader's shared buffer and channel in `run`. For TR-15, `KEPT_LINE_CHARS` and `cut` |
| [src/gates.rs](../src/gates.rs) | `Failure::check`, `Report::checks` and `extend`, `Ran`, `statics` (renamed from `submit`), `scripts`, `criterion_checks`, `criterion_runs`, `criterion_run` (TR-2), `run_in`, `root`, `timeout`, `check_failure`. `gates_json` adds `secs`, `failures_json` the check fields, and `refusal` the output lines. For TR-16, `Ran::criterion`. |
| [src/main.rs](../src/main.rs) | `apply` takes the lock from `move_item` and `status`, lets it go while checks run, and records `started` and `verification.checks`. New `already_passing` and `check_record`. `gates_dry_run` runs `scripts` too. Help text for `start` and `submit`. For TR-2, `criterion_add` and `criterion_set` read the file's text with the item and call the new `run_check_given_after_start` and `criterion_output`. For TR-3, `criterion_remove` drops and renumbers `started.checks` entries. For TR-14, `criterion_add` and `criterion_set` call `refuse_newer_file` after the load. For TR-16, `check_record` copies `Ran::criterion`. |
| [src/config.rs](../src/config.rs) | `Project::checks`, `check_timeout_secs`, `check_timeout`, `DEFAULT_CHECK_TIMEOUT_SECS` |
| [src/item.rs](../src/item.rs) | `Item::started`, `Started`, `StartedCheck`, `Verification::checks`, `CheckRecord`, and, for TR-16, `CheckRecord::criterion` |
| [src/store.rs](../src/store.rs) | `Store::lock_unchanged_after_checks` |
| [src/states.rs](../src/states.rs) | `UNCHECKED_NOTE`'s new text |
| [src/lib.rs](../src/lib.rs) | `pub mod checks` |
| [schemas/](../schemas/) | `error`, `gates`, `move`, `submit`, `item` and, for TR-2, `criterion`, as [above](#json-output). For TR-16, `item`'s `verification.checks[].criterion`. For TR-19, `item`'s and `error`'s descriptions of `output` |
| [tests/checks.rs](../tests/checks.rs) | New: one test per _Done when_ check, three for TR-2's criteria, one for TR-3's, eight for review 1's fixes (TR-13 to TR-16 and TR-18) and one for review 2's (TR-20), with `COUNTED`, a check that counts its runs in a file, `runs`, and, for TR-18, `check_that_edits`, a check that edits the item while it runs |
| [docs/DESIGN.md](DESIGN.md) | For TR-16 and TR-17: §2.4's "Checks must fail first" and its paragraph on `gates`, §3's `gates` row, §4.3's example and its `started.checks` and `verification.checks` bullets, and a [§7](DESIGN.md#7-honest-limits) limit on interrupting `tracker` ([below](#design-decisions)). For TR-19: §2.4's "Checks must fail first" again, on a check given in review, and §4.3's `verification.checks` bullet, on `previous_verification`'s numbers ([below](#design-decisions-1)) |
| [docs/PLAN.md](PLAN.md) | For TR-19: a note in slice 11's prompt, under T13, on how T13 reads `started.checks` after a check given in review |
| [CLAUDE.md](../CLAUDE.md) | For TR-19: a Conventions bullet saying an item whose work needs a `feat` commit is created with `--kind capability` |
| [tests/fixtures.rs](../tests/fixtures.rs), `tests/fixtures/slice-7/` | `slice_7_wrote_three_fixtures_with_check_results` (named `slice_7_wrote_two_fixtures_with_check_results` until TR-20), and XX-16 and XX-17. For TR-20, XX-18 and `slice_7s_xx_18_names_each_checks_criterion_and_comes_back_byte_for_byte` |

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

The filters match two tests and one, as `cargo test` shows. All three were committed in `64ec7f7`, before `tracker start TR-2`. Nothing records that they were run then, so for this readme they were run against `64ec7f7`'s files: all three fail, with exit 101, on their run counts and the warning. They pass from `513da4d`.

### Coverage of TR-3's criterion

TR-3 was found while building TR-2: `criterion remove` renumbered the criteria but left `started.checks` alone, so its entries named the wrong criteria, which slice 11's T13 would read. Its criterion, and the test its check runs:

| TR-3's criterion | Check | Test in `tests/checks.rs` |
| --- | --- | --- |
| 1. After `criterion remove` on an in-progress item, the removed criterion's entry is gone from `started.checks`, and every other entry still names the criterion it was recorded for. | `cargo test --test checks criterion_remove_after_start` | `criterion_remove_after_start_keeps_started_checks_on_their_criteria`: a remove before start makes no `started`. After start, with checks on criteria 1, 2 and 4 and a manual criterion 3, removing criterion 2 leaves `[{1, true}, {3, true}]` in the JSON and the file, and removing the manual criterion, which has no entry, still moves the later entry up to 2. |

The filter matches one test. It was committed in `d7f8b60` at 06:51:07Z, three seconds before `tracker start TR-3`. Nothing records that it was run then, so for this readme it was run against `d7f8b60`'s files: it fails, with exit 101, at `tests/checks.rs:419`, with `started.checks` still `[{1, true}, {2, false}, {4, true}]` after criterion 2 was removed. It passes from `ffb8255`.

### Other tests

- `src/checks.rs` `last_lines_keeps_the_last_twenty_without_a_final_break`: 25 lines keep the last 20, `\r\n` and trailing blank lines are dropped, and empty output stays empty.
- `tests/gates.rs` `gates_lists_every_failure_and_leaves_the_item_file_unchanged` now checks that the only extra line on stderr is `Running criterion 3's check: `true``.
- `tests/gates.rs` `ten_starts_at_once_in_one_project_let_exactly_one_through`, from slice 6, now goes through the unlocked checks and the second G1 on every start, and still lets exactly one through.
- `tests/fixtures/slice-7/` holds three files this slice's binary wrote: XX-16, started with one check failing and one passing, then shelved and reopened, XX-17, done with a criterion check's 20 kept lines and a project check's output, written before TR-16, so its check records have no `criterion`, and XX-18, from TR-20, done with checks on criteria 1 and 3, a manual criterion 2 between them, and a project check, so its records name criteria 1, 3 and none. `slice_7_wrote_three_fixtures_with_check_results` checks the folder holds exactly these three, and XX-16's and XX-17's contents. `slice_7s_xx_18_names_each_checks_criterion_and_comes_back_byte_for_byte` checks XX-18's `criterion` values and that it comes back byte for byte with nothing lost. The existing fixture tests load and rewrite all three byte-for-byte.

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

Each was run with this branch's binary in a temporary `TRACKER_HOME`, before review 1. Three now have automated tests, from TR-18 ([Test gaps](#test-gaps)): check output is escaped, an item changed while its checks ran, and an item changed while a check given after start ran. The other three still don't:

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

TR-2 was created while TR-1 was being built, as out-of-scope work. It was tracked with the installed slice 6 binary throughout, as the line `a270b4d` adds to [CLAUDE.md](../CLAUDE.md) asks: started at 2026-11-21T06:28:22Z, 11 seconds after `64ec7f7` committed its tests, and submitted at 2026-11-21T06:43:30Z. So:

- Its file has no `started`, since slice 6 doesn't write one.
- It reached done `unchecked` with slice 6's note, "Submitted before tracker ran checks. Gates G2 and G6 passed, but nothing tested the work.", and no `verification.checks`. G3 never ran its two checks, and G4 never ran TR's project checks. Both checks, and the test command, pass when run by hand on this branch ([above](#coverage-of-tr-2s-criteria)), but nothing in its record says so.
- The slice 6 binary can't show TR-1 in full: `tracker show TR-1` ends by saying TR-1's file has fields it doesn't know, in `verification`, since TR-1 was submitted by this branch's binary.

`a270b4d` (`docs: track items with the installed tracker`) was first committed with the whole command, `git commit -m '…'`, as its header. It was reworded, and the commits after it rebased with their trees unchanged, so the hashes here are the rebased ones. It and the other two docs commits, `6e6ceda` and `072bc5d`, are unscoped, but were made between TR-1's done and TR-2's start, while no item was in progress, so slice 11's T14 wouldn't count them against either.

### Tracking TR-3

TR-3, a `fix`, was created at 2026-11-21T06:47:48Z, after TR-2's submit, as out-of-scope work found while building TR-2. Like TR-2, it was tracked with the installed slice 6 binary throughout: started at 06:51:10Z, three seconds after `d7f8b60` committed its test, and submitted at 06:52:36Z, after `ffb8255` (the fix) and `9ed6f65` (its notes in this readme). So:

- Its file has no `started`, since slice 6 doesn't write one.
- It reached done `unchecked` with slice 6's note and no `verification.checks`. G3 never ran its check, and G4 never ran TR's project checks. The check, and the test command, pass when run by hand on this branch ([above](#coverage-of-tr-3s-criterion)), but nothing in its record says so.
- `769909d`, before its start, and `40a7c7a`, after its submit, are unscoped, and were made while no item was in progress.

### Tracking review 1's fixes

Slices 8–16's items were created first, as TR-4 to TR-12, at 2026-11-21T07:07:43Z, so that fixes couldn't shift their numbers again ([review 1's process 7](#process)). Review 1's fixes followed at 07:08:13Z, as TR-13 to TR-18, each naming the review and its finding in its description. Each was tracked with the installed slice 6 binary, one at a time:

| Item | Kind | For | Tests committed | Started | Fix | Submitted |
| --- | --- | --- | --- | --- | --- | --- |
| TR-13 | `fix` | bug 1 | `f7ef9dd`, 07:10:29Z | 07:10:34Z | `5e239bb` | 07:13:12Z |
| TR-14 | `fix` | bug 4 | `fd61aac`, 07:13:33Z | 07:13:33Z | `f4dbd44` | 07:29:56Z |
| TR-15 | `fix` | bug 5 | `06375a8`, 07:30:19Z | 07:30:20Z | `33ee754` | 07:31:51Z |
| TR-16 | `change` | finding 8 | `88bf66f`, 07:32:31Z | 07:32:31Z | `0af662d`, `b3ca602` | 07:33:49Z |
| TR-17 | `change` | findings 6 and 7, bug 2 | none: docs only | 07:34:22Z | `99625be` | 07:35:34Z |
| TR-18 | `change` | test gaps 1 and 2 | `02ef286`, 07:36:31Z | 07:37:34Z | none: the code already worked | 07:38:47Z |

So:

- Each reached done `unchecked` on G2 and G6 alone, with slice 6's note, no `verification.checks` and no `started`. The full test command was run by hand before each submit, and passed, with 267 tests after the last.
- Each item's tests were committed before its `tracker start`. TR-14's and TR-16's starts came in the same second as their test commits, so the times don't show whether the tests were run before the start. For this readme, TR-13's, TR-14's, TR-15's and TR-16's tests were run again against their test commits' files, and each failed as the [tables below](#fix-before-slice-8) say. TR-18's tests cover code that already worked, so each was seen failing against code broken on purpose, then restored ([Test gaps](#test-gaps)).
- TR-17's three criteria are manual, each with evidence quoting the DESIGN.md passage. Slice 9's T6 would hold such an item. Until then it reaches done like any other.
- `065a308`, the one unscoped commit since review 1, came after TR-3's submit and before TR-13's start, while no item was in progress.

### Tracking review 2's fixes

Review 2 was committed in `83740e4` at 2026-11-21T13:50:05Z, and its fixes were created at once, as TR-19 at 13:50:05Z and TR-20 at 13:50:06Z, each naming the review and its findings in its description. Each was tracked with the installed slice 6 binary, one at a time:

| Item | Kind | For | Tests committed | Started | Fix | Submitted |
| --- | --- | --- | --- | --- | --- | --- |
| TR-19 | `change` | findings 1 and 2, process 2, and review 1's bug 5's schema wording | none: docs only | 13:52:14Z | `61cb404`, `8be2f4b`, `eb80e0f`, all at 13:52:20Z | 13:53:20Z |
| TR-20 | `change` | test gaps 1 and 2 | `1549d7d`, 13:55:31Z | 13:56:33Z | none: the code already worked | 13:57:39Z |

So:

- Each reached done `unchecked` on G2 and G6 alone, with slice 6's note, no `verification.checks` and no `started`. The full test command was run by hand, and passed, with 269 tests, as it does for this readme.
- TR-19's four criteria are manual, each with evidence quoting the passage and naming its commit. It changes docs only, so there was nothing to see fail, and its commits came after its start.
- TR-20's two checks, `cargo test --test checks a_passing_check_with_an_escaped_process_finishes_soon_and_passes` and `cargo test --test fixtures slice_7s_xx_18_names_each_checks_criterion`, each match one test. Its tests were committed a minute before its `tracker start`. They cover code that already worked, so each was seen failing against code broken on purpose, then restored ([Test gaps](#test-gaps-1)). For this readme, the unbounded wait and the moved field were made again in a copy of `1549d7d`'s files outside the repo, and both tests failed as that table says: the first after 30.03s, the second at `tests/fixtures.rs:263`.
- TR-20 also renamed an existing test, `slice_7_wrote_two_fixtures_with_check_results`, to `slice_7_wrote_three_fixtures_with_check_results`, and added XX-18 to the file names it expects, since it lists the folder's files exactly. Nothing it checked was removed. That's DESIGN.md §5's "fix it and say so" case, which slice 10's T4 will hold.
- Both are `change`s, and neither has a `feat` commit, which fits the CLAUDE.md line TR-19 added.
- The three unscoped commits since review 1's fixes, `464c239` and `30d2a4d` at 07:52Z and `83740e4` at 13:50:05Z, came after TR-18's submit and before TR-19's start, while no item was in progress.

## Decisions worth knowing

- **Every check runs, even after one fails.** §2.4 holds back only the judges until everything before them passes, so a refusal lists every failing check, as it lists every other reason.
- **G3 and G4 are always listed**, with `secs` 0 when there was nothing to run, since `gates` lists every gate that ran.
- **The command goes to stderr, even with `--json`**, so stdout stays one JSON object, and an agent still sees what's running.
- **A check's whole process group is stopped**, at the timeout and again once the shell ends. A test runner the shell started would otherwise outlive the timeout and hold the output open.
- **Stdout and stderr are kept together, in order**, since a failure message often goes to stderr after the progress that led to it on stdout.
- **`start`'s warnings are also `warnings` in `--json`**, since an `Output`'s warnings are printed only as text.
- **A check that can't run counts as failing at start.** `start` expects failures, so it goes ahead, and the submit's G3 says why. This settles [slice 2's open question](slice-2-readme.md#still-open) for checks: a move on an item whose project was removed from `config.json` still works, but G3 and G4 fail it at submit.
- **`verification.by` stays `unchecked`.** `auto` means proven against a commit, which needs slice 8. The note keeps slice 6's opening words, since checks have now run but not against a recorded commit: "Submitted before tracker ran checks against a recorded commit. Gates G2, G6, G3 and G4 passed, but no commit ties the checks to the work."
- **The re-read after checks is already here.** Slice 8's prompt lists it, but §4.4 requires it as soon as checks run without the lock. TR-18 tests it for `start` and for a check given after start, with a check that edits the item file. Slice 8's _Done when_ test does the same for `submit`.
- **`gates` runs the criterion and project checks for an item in any state**, not only one in progress, so an agent can see its checks fail before `start`. For an item that can't be submitted, it says so under the reasons. A done item's `gates` runs every check for nothing, which is the price. §2.4 and §3 say so (TR-17, review 1's finding 6).
- **Once a check's group is stopped, its output is waited for at most 1 second** (TR-13). The timeout is meant to bound how long a submit takes, and only a process that left the group can hold the output open longer. The failure reports the timeout as usual, with the output read so far.
- **A kept line is cut at 500 characters**, the length of a criterion's text or a reason, ending in `…` (TR-15), so one long line, such as minified test output, can't make the item file hundreds of kilobytes long.
- **Each criterion check in `verification.checks` names its `criterion`** (TR-16), so two criteria with one command, or a criterion check that's also a project check, can be told apart, and slice 16's digest can match each check to its criterion. It was added before slice 8 writes the first `auto` records. Records written before TR-16 have no `criterion`, and still load unchanged.
- **Ctrl-C on `tracker` doesn't stop the check it's running** (review 1's bug 2, won't fix). Catching the signals needs a crate such as `signal-hook`, which [§8](DESIGN.md#8-technology) doesn't list, and the harm is small: a check that runs on to its end. §7 says so (TR-17).
- **A `check_timeout_secs` near `u64::MAX` panics** (review 1's bug 3, won't fix). Only an absurd hand-edited `config.json` reaches it: values up to `10000000000000` work.
- **`started.checks` records only criterion checks.** Project checks aren't expected to fail at start, so nothing reads their result then.
- **A check given after start runs only on an in-progress item.** An open item's check runs at `start` anyway. An item in review takes criterion changes only from a terminal, and T5 fires on them (§2.4 notes it fires for a new check after start too). A done item refuses them. §2.4 says so since TR-19 (review 2's finding 1): a check given in review isn't run then, and `started.checks` keeps the old check's result for that criterion, or none, until the item is started again. PLAN.md's slice 11 prompt says how T13 reads such an entry.
- **A verification record is never renumbered** (TR-19, review 2's finding 2). Its `criterion` numbers are as the criteria were at that submit. After a reopen, a `criterion remove` renumbers `started.checks` but not `previous_verification`, since rewriting a past record would change what it says was verified. A reader matches a past record's checks to today's criteria by `cmd`, as §4.3 says.
- **A timed-out check whose escaped process held the output doesn't say so** (review 1's bug 1, the message half, won't fix). The failure reports the timeout, which is what matters, and the note was an optional part of review 1's option (a). The limits [below](#limits-and-what-later-slices-change) say the escaped process is left running.
- **A check given after start replaces its criterion's entry in `started.checks`**, rather than adding a second, so each criterion has at most one entry and slice 11's T13 reads one result for it: the latest check's, run before any work on it.
- **The warning is `start`'s, word for word**, as TR-2's criterion 2 asks, so it ends "BM-1 started anyway." even though the start was earlier.
- **`criterion set` with the check a criterion already has runs nothing**, since nothing changed, and its `warnings` is empty.

## Fixes after review 1

[Review 1](reviews/slice-7-review-1.md) found five bugs, three places where the code and DESIGN.md disagreed or the design was unclear, two test gaps and seven process notes. Bugs 1, 4 and 5, findings 6 to 8 and both test gaps became tracker items, TR-13 to TR-18 ([Tracking review 1's fixes](#tracking-review-1s-fixes)), each with its tests committed before `tracker start`, and the review's option (a) was taken wherever there was a choice. Bugs 2 and 3 won't be fixed, and finding 6's test-helper half is left for slice 11. The review's [Outcome](reviews/slice-7-review-1.md#outcome) records the same decisions.

### Fix before slice 8

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Bug 1: a check that starts a process outside its group kept `tracker` waiting past the timeout, until that process ended | `5e239bb` (TR-13) | `run`'s reader fills a buffer it shares with `run` and ends an `mpsc` channel when the pipe closes. Once the group is stopped, `run` waits at most 1 second for that (`LEFTOVER_OUTPUT`), then goes on with what was read. The failure doesn't say that something the check started kept running, which the review's option (a) also suggested. That half won't be fixed ([Decisions](#decisions-worth-knowing)) | `a_timed_out_check_with_an_escaped_process_fails_soon_after_its_timeout`: a `perl` process that calls `setsid` and sleeps 30 seconds, under a 1-second timeout. It must fail within 10 seconds, with `timed_out` and the output `before`. Before the fix it took 30.07s. It now takes about 2.5s |
| Bug 4: `criterion add` and `criterion set` ran a check before refusing a newer tracker's file | `f4dbd44` (TR-14) | Both call `refuse_newer_file` right after the load, as `gates` and every move do | `criterion_add_and_set_refuse_a_newer_trackers_file_before_running_a_check`: the review's case, with `started.head` added by hand. Both exit 3, the file is unchanged, and the check, `touch ran`, never runs. Before the fix, `criterion add` ran its check and then exited 3, and the `criterion set` half was also seen failing on its own |
| Bug 5: a kept line had no length limit, so one check could add hundreds of kilobytes to the item file | `33ee754` (TR-15) | `last_lines` cuts each kept line to `KEPT_LINE_CHARS`, 500 characters, with `cut`: 499 of them and `…` | `a_kept_line_longer_than_the_limit_is_cut_and_marked`: a 2000-character line, then `short`, kept as 499 `x`s and `…`, then `short`. Before the fix the long line was kept whole |

### Low severity

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Bug 2: Ctrl-C on `tracker` leaves the running check going | `99625be` (TR-17) | Won't fix in code. Catching the signals needs a crate such as `signal-hook`, which §8 doesn't list, and the harm is small: a check that runs on to its end. Instead, §7 has a limit, "Interrupting `tracker` can leave a check running", the review's option (b) | None: docs only |
| Bug 3: a very large `check_timeout_secs` panics | none | Won't fix. Only an absurd hand-edited `config.json` reaches it. `run` still adds the timeout to `Instant::now()` with `+` | |

### Design decisions

| Question | Decision | Commits |
| --- | --- | --- |
| Finding 6: what `gates` runs for an item that isn't in progress | Option (a): every criterion and project check, in any state, so an agent can see its checks fail before `start`. §2.4's paragraph on `tracker gates` and §3's `gates` row say so, and so does [Decisions](#decisions-worth-knowing) | `99625be` (TR-17) |
| Finding 6: the shared test helper's check is still `true` | Left for slice 11. `Env::add_criterion` still adds `--check true`. Slice 11's T13 will hold every item started with a check that passes, so slice 11 has to change the helper anyway | none |
| Finding 7: DESIGN.md didn't say what the slice decided about `started.checks` | As suggested. §2.4's "Checks must fail first" says a check given after start is recorded in `started.checks` in place of any earlier result for its criterion, and warned about as at start, and that `criterion remove` drops the removed criterion's entry and renumbers the later ones. §4.3 says `started.checks` holds the latest run of each criterion's check before the work, numbered as the criteria are now | `99625be` (TR-17) |
| Finding 8: a verification record's checks didn't say which criterion each came from | Option (a). `CheckRecord::criterion`, an `Option<usize>` and its first field, with a serde default and `skip_serializing_if`, so it's left out for a project check and older records load unchanged. `gates::Ran` carries it to `check_record`. `item.schema.json` has it, and §4.3's example and a new bullet show it. Test: `verification_checks_name_the_criterion_each_came_from`, with one command as criterion 1's check and as a project check, a manual criterion 2 and a check on criterion 3, recorded as 1, 3 and none, in the JSON and the file. Before the fix, no entry had a criterion | `88bf66f`, `0af662d`, `b3ca602` (TR-16) |

### Test gaps

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Test gap 1: no test failed if the re-read after checks was taken out | `02ef286` (TR-18) | Two tests whose check edits the item while it runs, with `check_that_edits`: `tracker edit XX-1 --priority high; false`. The check inherits `TRACKER_HOME` | `start_refuses_when_the_item_changed_while_its_checks_ran`: exit 1, and the item is still open, with no `started` and the edit kept. `criterion_add_after_start_refuses_when_the_item_changed_while_its_check_ran`: exit 1, with one criterion, `started.checks` as it was, and the edit kept. Seen failing with `lock_unchanged_after_checks` swapped for `store.lock()` in `start`, then in `run_check_given_after_start` |
| Test gap 2: no test that a check's output is escaped, or that a check's leftover background process is stopped | `02ef286` (TR-18) | Two tests | `control_characters_in_a_checks_output_are_escaped_in_the_text_output`: output with an escape and U+202E, which `gates` prints on stderr as `a\u{1b}[2Kb\u{202e}c`. Seen failing with the output printed without `text::for_terminal`. `a_background_process_a_timed_out_check_started_is_stopped`: the criterion check `(sleep 3; touch late) & sleep 30` under a 1-second timeout, and the project check `(sleep 3; touch later) & true`, and neither file appears. Seen failing with `stop_group` taken out in two ways. The `Running` line's escaping isn't tested with a real control character: `criterion add` refuses one, so only a hand-edited file can hold it |

### Process

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Process 1: the readme stopped at TR-2 | this update | TR-3 in the opening paragraph and the Status row, the counts, [Coverage of TR-3's criterion](#coverage-of-tr-3s-criterion), whose check matches one test, and [Tracking TR-3](#tracking-tr-3) | |
| Process 2: nothing recorded that TR-3's test was seen failing before `tracker start TR-3` | this update | Run against `d7f8b60`'s files for this readme: it fails at `tests/checks.rs:419` with the old entries left in place ([above](#coverage-of-tr-3s-criterion)) | |
| Process 3: no slice 7 check was run by a binary other than the code it checks | none yet | This slice's binary will be installed before TR-4 (slice 8) starts, so slice 8's checks are the first the installed `tracker` runs. Review 1's fixes were verified by the slice 6 binary, as TR-2 and TR-3 were | |
| Process 4: TR-2's kind is `change`, though it adds behaviour | none | TR-2 stays a `change`, since a done item can't be edited. Later out-of-scope items that add behaviour get `capability` | |
| Process 5: the unscoped commits are all between items | none | No action needed. `065a308`, the one unscoped commit since, was made between items too | |
| Process 6: `0ac6b10` changed the expected values of eight slice 6 tests | none | No action needed | |
| Process 7: slices 8–16 had no items, and the walkthrough numbered them from TR-2 | `065a308` | Option (a). The nine items were created in order as TR-4 to TR-12, with the walkthrough's commands unchanged, and the walkthrough has a note that in the real store slice N is TR-(N−4). Its table, step 2 and examples still show the throwaway store's numbers | |

The review's [Slice 8 must decide](reviews/slice-7-review-1.md#slice-8-must-decide) list is carried into [Limits](#limits-and-what-later-slices-change). TR-13 takes bug 1 out of what shelving inherits, TR-14 settles bug 4 before `started.head` exists, TR-15 settles how long a kept line can be, TR-16 settles the shape of `verification.checks`, and slice 8's item is TR-4. Whether G5 runs again after the checks is still slice 8's to decide.

A few consequences:

- `verification.checks` entries for criterion checks have `criterion` from TR-16 on. TR-1's record, written before it, has none.
- A check whose escaped process holds the output open now fails about a second after its timeout, and that process is left running.
- A kept line of output is at most 500 characters, in the item file, in `--json` and in a refusal's text.
- `criterion add --check` or `criterion set --check` on a newer tracker's file exits 3 at once, without printing `Running …`.

## Fixes after review 2

[Review 2](reviews/slice-7-review-2.md) found no new bug and no security issue: two places where DESIGN.md didn't say what the code does, two test gaps and three process notes. Findings 1 and 2, process 2 and review 1's open schema wording became TR-19, and both test gaps became TR-20 ([Tracking review 2's fixes](#tracking-review-2s-fixes)), with TR-20's tests committed before `tracker start`. The review's option (a) was taken wherever there was a choice. Process 1 is fixed in this update of the readme, and process 3 needed no action. The review's [Outcome](reviews/slice-7-review-2.md#outcome) records the same decisions.

### Low severity

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Carried forward: review 1's bug 1, the failure doesn't say that something the check started kept running | none | Won't fix. The failure reports the timeout, which is what matters, and the note was an optional part of review 1's option (a) ([Decisions](#decisions-worth-knowing)) | |
| Carried forward: review 1's bug 5, the schemas' descriptions of `output` don't mention the 500-character cut | `8be2f4b` (TR-19) | `item.schema.json`'s and `error.schema.json`'s `output` descriptions add "Each kept line is cut at 500 characters, the last of them `…`, if it's longer." | None: a description. `cargo test --test schemas` still passes |

Review 1's bugs 2 and 3, also carried forward, still won't be fixed, as [above](#low-severity) says.

### Design decisions

| Question | Decision | Commits |
| --- | --- | --- |
| Finding 1: §2.4 says a check given "after `start`" runs then, but the code runs it only for an item in progress | Option (a), in docs only. §2.4's "Checks must fail first" says it happens only while the item is in progress: an item in review takes criterion changes only from a terminal, and T5 holds it anyway, so a check given then isn't run, and `started.checks` keeps the old check's result for that criterion, or none for one that was manual, until the item is started again. PLAN.md's slice 11 prompt, under T13, says T13 reads a missing entry as a check that didn't pass, and an old one as it reads any other, since `started.checks` doesn't record the command | `61cb404` (TR-19) |
| Finding 2: after a reopen and a `criterion remove`, `previous_verification` names the wrong criterion for each check | Option (a), in docs only. §4.3's `verification.checks` bullet says each `criterion` is numbered as the criteria were at that submit, and that a record is history, so unlike `started.checks` it's never renumbered, and a reader matches its checks to today's criteria by `cmd`. The code already leaves `previous_verification` alone | `61cb404` (TR-19) |

### Test gaps

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Test gap 1: only the timed-out half of TR-13's fix has a test | `1549d7d` (TR-20) | A test for a check that passes and leaves a detached process holding the output. The code already worked | `a_passing_check_with_an_escaped_process_finishes_soon_and_passes`: the check is a `perl` process that calls `setsid`, writes its pid to `escaped.pid` and sleeps 30 seconds, then `& echo ok`, under the default timeout. `gates --json` must pass within 10 seconds with G3 `ok`, and the test kills the escaped process. Seen failing with `run`'s wait for the output made unbounded (`ended.recv()`): it took 30.03s. It now takes about 1.5s |
| Test gap 2: no fixture holds a check record with `criterion` | `1549d7d` (TR-20) | Option (a): XX-18, written by this branch's binary, done with checks on criteria 1 and 3, a manual criterion 2 and a project check. XX-17 stays as the shape before TR-16, which TR-1's record also has. `slice_7_wrote_two_fixtures_with_check_results` lists the folder's files exactly, so it's renamed `slice_7_wrote_three_fixtures_with_check_results` and expects XX-18 too | `slice_7s_xx_18_names_each_checks_criterion_and_comes_back_byte_for_byte`: the records' `criterion` values are 1, 3 and none, criterion 3's `output` is `parsed 3 lines`, and the file comes back byte for byte with nothing lost on rewrite. Seen failing with `criterion` moved after `cmd` in `CheckRecord`, at the byte-for-byte check, and with the field renamed |

### Process

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Process 1: the Status row said its update wasn't committed, and the Branch row didn't name `464c239` or `30d2a4d` | this update | The sentence is gone, and the Branch row names both, then `83740e4` and review 2's fixes | |
| Process 2: TR-16 is a `change`, though its code commit is `feat(TR-16)` | `eb80e0f` (TR-19) | Option (a). [CLAUDE.md](../CLAUDE.md)'s Conventions say an item whose work adds behaviour or a field, and so needs a `feat` commit, is created with `--kind capability`. TR-16 stays a `change`, since a done item can't be edited | |
| Process 3: the order of the fix items' work checks out | none | No action needed | |

The review's [Still open, carried forward](reviews/slice-7-review-2.md#still-open-carried-forward) list is covered above and in [Still open](#still-open): review 1's finding 6's test-helper half stays for slice 11, and installing this slice's binary is still to come. Its [Slice 8 must decide](reviews/slice-7-review-2.md#slice-8-must-decide) list is left for slice 8, whose item is TR-4, and is carried into [Limits](#limits-and-what-later-slices-change).

No behaviour changed: review 2's fixes are docs, one test and one fixture. The Try it output is unchanged.

## Still open

- **Test gaps** (review 1's finding 6, second half, carried forward by review 2): `Env::add_criterion` in [tests/common/mod.rs](../tests/common/mod.rs) still adds `--check true`, left for slice 11.
- **Process** (review 1's process 3, carried forward by review 2): this slice's binary is to be installed with `cargo install --path .` after review 2's Outcome is committed, and before TR-4 (slice 8) starts.

## Limits, and what later slices change

- **Slice 8** adds G5 between G2 and G6 (§2.4's cheapest-first order), `started.head` and `criteria_hash` before `started.checks` (§4.3's order), and `commit` and `ref` before `verification.checks`. It replaces `unchecked` with `auto` and drops `UNCHECKED_NOTE` from new records, but files with it must still load. Shelving an in-progress item needs G4, so it needs the project-check half of `gates::scripts` on its own, and its re-read after those checks needs a test like TR-18's two. Shelving also inherits §7's limit on interrupting `tracker`, and leaves running, without a word, any escaped process a project check starts (TR-13). Its test that a check editing the item file makes `submit` exit 1 covers `lock_unchanged_after_checks` for `submit`, as TR-18's tests do for `start` and for a check given after start. It must decide whether G5's clean-tree check runs again after the checks, since a check that leaves an untracked file in the root passes this submit and fails G5 on the next one, or say in §2.4 that a check must leave the tree as it found it (review 1's [Slice 8 must decide](reviews/slice-7-review-1.md#slice-8-must-decide), carried forward by review 2's). Its first records carry `commit` and `ref` beside `criterion`, and need a fixture of their own once the shape lands, as XX-18 keeps TR-16's. Its item is TR-4, and this slice's binary is to be installed before it starts, so the installed `tracker` runs slice 8's checks. `show` prints `started` and each check's saved output, with its `criterion`, which must go through `Styled::push`, escaped with its line breaks kept, as [slice 5's readme](slice-5-readme.md#limits-and-what-later-slices-change) asks. It should say that a `previous_verification`'s numbers are as at that submit (review 2's finding 2).
- **Slice 9**'s triggers run after `scripts` passes. `approve` re-runs the gates and must let the lock go and re-read the item in the same way.
- **Slice 11**'s T13 reads `started.checks[].passed`. Each entry is numbered by criterion position when its check ran, at start or when it was given after start. `criterion remove` drops the removed criterion's entry and renumbers the later ones (TR-3), so each entry still names the criterion it was recorded for. T5 fires on a removal anyway. A check given while the item is in review isn't run (§2.4), so its entry is the old check's, or missing, and PLAN.md's slice 11 prompt says T13 reads a missing entry as a check that didn't pass (TR-19). T13 must treat an item started before slice 7, with no `started`, such as TR-1 to TR-3 and TR-13 to TR-20, as having nothing to compare. T13 also holds every item that `Env::create` starts in the tests, since `Env::add_criterion`'s check is `true`, so slice 11 changes the helper to a check a test controls (review 1's finding 6).
- **Slice 12** runs judges in G3 only after every check in G3 and G4 passed, and adds their failures, with `ref`, `answer` and `reason`, beside the check failures. `criterion_checks` must keep skipping judge criteria.
- **Slice 14** runs judges at `start` too, alongside `criterion_runs`, and records them in `started`.
- **Slice 12** adds `criterion add --judge` and `criterion set --judge`. A judge criterion has no `check`, so `run_check_given_after_start` returns early for one, and a judge given after start won't run then unless a later slice adds it. §2.4 says a new check after start runs once, and says nothing of a new judge, so slice 14, which runs judges at `start`, should decide whether one runs then too.
- **Slice 16**'s digest lists the checks that ran for each auto-verified item, from `verification.checks`, and can match each to its criterion by `criterion` (TR-16). A record written before TR-16, such as TR-1's, has none. A `previous_verification`'s numbers are as at that submit, never renumbered, so `audit` matches its checks to today's criteria by `cmd` (§4.3, TR-19).
- **Checks need `sh` and `kill`**, so the tool runs on Unix-like systems only. A process that leaves the check's group, such as a daemon that calls `setsid`, isn't stopped. `tracker` waits at most 1 second more for the output it holds, then goes on and leaves it running (TR-13).
- **Interrupting `tracker` leaves its check running**, as §7 says. Ctrl-C, or a signal to `tracker`'s process group, stops `tracker` but not the check's group, which runs on in the root. Fixing it needs a signal crate that §8 doesn't list (review 1's bug 2, won't fix).
- **A `check_timeout_secs` near `u64::MAX` panics** with "overflow when adding duration to instant", at `start` and `gates`. Values up to `10000000000000` work (review 1's bug 3, won't fix).
- **The last 20 lines of a whole suite may not show its totals.** TR-1's `cargo test` record ends with the doc-tests' "0 passed", since `cargo test` prints one summary per test target. A criterion check that runs one test, as §5 asks, doesn't have this problem.
- **`check_timeout_secs` applies to each check, not to a gate**, so a submit can take as long as the timeout times the number of checks. Checks run one at a time.
- **Times are rounded to the nearest second**, so a quick check records `secs: 0`.
- **`start` doesn't warn when a check can't run**, such as when the root is gone. It records `passed: false`, the same as an expected failure, and only the submit's G3 says why.

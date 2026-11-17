# Slice 2: States

`tracker` is a command-line work tracker, written in Rust, that keeps one central list of work items across many projects and, in later slices, decides when an item is really done. This document covers **slice 2** of the 16 in [PLAN.md](./PLAN.md): what it built, how it works, and how to test it. [Slice 1's readme](slice-1-readme.md) covers the store, `create` and `list`.

After slice 2 you can **move an item through its lifecycle**: start it, submit it, shelve it and reopen it, with every move kept in its history. The spec is [DESIGN.md](./DESIGN.md). This slice implements §2.2 (states and moves) and the `status` alias from §3, and adds `history` and `verification` to §4.3's item file.

|              |                                                                                                                                                                                                                                                                 |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Status       | Done. Every _Done when_ check passes, and so does the test command. From [review 1](./reviews/slice-2-review-1.md), its three fixes before slice 3 and two low-severity items are done, and its five questions about DESIGN.md are settled ([below](#fixes-after-review-1)). From [review 2](./reviews/slice-2-review-2.md), its fix before slice 3, three low-severity items, two DESIGN.md questions and four test gaps are done ([below](#fixes-after-review-2)). The rest is listed under [Still open](#still-open). |
| Branch       | `feat/tracker/slice-2`                                                                                                                                                                                                                                          |
| Test command | `cargo fmt --check && cargo clippy -- -D warnings && cargo test`, run in `tracker/`                                                                                                                                                                             |
| Tests        | 98: 38 in `tests/create_and_list.rs`, 27 in `tests/states.rs`, 10 in `tests/item_files.rs`, 6 each in `tests/output.rs` and `tests/docs_links.rs`, 4 in `tests/fixtures.rs`, 3 in `tests/schemas.rs`, and 2 each in `tests/item_ids.rs` and `tests/store_writes.rs` |

## Contents

- [What's in this slice](#whats-in-this-slice)
- [Try it](#try-it)
- [Commands](#commands)
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

- Four commands that move an item: `start`, `submit`, `shelve --reason` and `reopen --reason`.
- `status <id> <state>`, the alias from the course's starting doc, which runs the command that makes the move.
- A move table in [src/states.rs](../src/states.rs) with every edge of §2.2's diagram that has a command so far. A move the diagram doesn't draw exits 1 and lists the allowed moves, as text and as JSON.
- `history`: one entry per move, with when, from, to, the command, whether it came from a terminal, and the reason for shelve and reopen.
- `verification`: the record `submit` writes. Until the gates exist it says `unchecked`, with a note that nothing checked the work. Reopening moves it to `previous_verification`.
- Item files written by this slice, in `tests/fixtures/slice-2/`.

**Not built yet:** review and the moves into and out of it (`approve` and `reject` in slice 9, with the triggers that hold an item), kind, priority, labels, `edit` and filters (slice 3), `show` and `delete` (slice 4), the gates (slice 6), which will decide whether a submit reaches done, and §2.8's rules for shelving an in-progress item (slice 8). Until slice 8, `shelve` moves an in-progress item without checking that the working tree is clean or that the project checks pass.

## Try it

Always set `TRACKER_HOME` while developing. Without it, the tool uses your real store at `~/.tracker`.

```sh
cd guild-portfolio/tracker
cargo build

export DEMO=$(mktemp -d)
export TRACKER_HOME=$DEMO/store
export PATH="$PWD/target/debug:$PATH"
mkdir -p $DEMO/bookmarks && cd $DEMO/bookmarks

tracker init
tracker project add bm --name bookmarks --root $DEMO/bookmarks
tracker create "Search filters bookmarks by tag"
tracker submit BM-1                          # exit 1: it isn't started
tracker start BM-1
tracker submit BM-1
tracker shelve BM-1 --reason "later"         # exit 1: a done item can only be reopened
tracker reopen BM-1 --reason "found a bug"
tracker status BM-1 in-progress              # runs start
tracker shelve BM-1                          # exit 1: needs a reason
tracker shelve BM-1 --reason "Blocked on BM-2"
tracker status BM-1 review                   # exit 1: only submit reaches review
tracker start BM-1 --json                    # exit 1, with the allowed moves
tracker list
cat $TRACKER_HOME/items/BM-1.json
```

Output, with the temporary folder shown as `$DEMO`:

```
Created the store at $DEMO/store.
Registered BM (bookmarks) at $DEMO/bookmarks.
Created BM-1: Search filters bookmarks by tag
error: BM-1 is open, and `submit` only moves an item that is in-progress. From open, the allowed moves are `tracker start BM-1` (to in-progress) and `tracker shelve BM-1 --reason "…"` (to shelved).
BM-1: open → in-progress
BM-1: in-progress → done
Unchecked. No gates ran: this version of tracker has none yet, so nothing checked the work.
error: BM-1 is done, and `shelve` only moves an item that is open or in-progress. From done, the only move is `tracker reopen BM-1 --reason "…"` (to open).
BM-1: done → open
BM-1: open → in-progress
error: Shelving BM-1 needs a reason, which is kept in its history: `tracker shelve BM-1 --reason "…"`.
BM-1: in-progress → shelved
error: No command moves an item straight to review. `submit` puts it there when it needs a human. From shelved, the only move is `tracker reopen BM-1 --reason "…"` (to open).
{
  "error": "BM-1 is shelved, and `start` only moves an item that is open. From shelved, the only move is `tracker reopen BM-1 --reason \"…\"` (to open).",
  "exit_code": 1,
  "allowed": [
    {
      "via": "reopen",
      "to": "open",
      "command": "tracker reopen BM-1 --reason \"…\""
    }
  ]
}
BM  bookmarks
  BM-1  shelved  Search filters bookmarks by tag
```

The item file then holds its last verification in `previous_verification`, because it was reopened after reaching done, and five history entries:

```json
{
  "id": "BM-1",
  "title": "Search filters bookmarks by tag",
  "project": "BM",
  "kind": "change",
  "status": "shelved",
  "priority": "medium",
  "created": "2026-11-17T00:24:57Z",
  "updated": "2026-11-17T00:24:57Z",
  "previous_verification": {
    "by": "unchecked",
    "at": "2026-11-17T00:24:57Z",
    "note": "No gates ran: this version of tracker has none yet, so nothing checked the work."
  },
  "history": [
    {
      "at": "2026-11-17T00:24:57Z",
      "from": "open",
      "to": "in-progress",
      "via": "start",
      "interactive": false
    },
    {
      "at": "2026-11-17T00:24:57Z",
      "from": "in-progress",
      "to": "done",
      "via": "submit",
      "interactive": false
    },
    {
      "at": "2026-11-17T00:24:57Z",
      "from": "done",
      "to": "open",
      "via": "reopen",
      "interactive": false,
      "reason": "found a bug"
    },
    {
      "at": "2026-11-17T00:24:57Z",
      "from": "open",
      "to": "in-progress",
      "via": "start",
      "interactive": false
    },
    {
      "at": "2026-11-17T00:24:57Z",
      "from": "in-progress",
      "to": "shelved",
      "via": "shelve",
      "interactive": false,
      "reason": "Blocked on BM-2"
    }
  ]
}
```

The file itself puts each history field on its own line. It's shortened here. `interactive` is `false` throughout because the demo ran from a script. Typed at a terminal, it would be `true`.

## Commands

| Command                                      | Move                                                                                                                                                                                                        |
| -------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `tracker start <id>`                         | open → in-progress                                                                                                                                                                                          |
| `tracker submit <id>`                        | in-progress → done, with an `unchecked` verification record                                                                                                                                                 |
| `tracker shelve <id> --reason "…"`           | open or in-progress → shelved                                                                                                                                                                               |
| `tracker reopen <id> --reason "…"`           | done or shelved → open. A verification record moves to `previous_verification`                                                                                                                              |
| `tracker status <id> <state> [--reason "…"]` | Runs the command that moves the item to `<state>`: `in-progress` runs start, `done` runs submit, `shelved` runs shelve and `open` runs reopen. `review` is refused, and so is any move of an item in review |

Every command takes `--json`. An ID can be typed in lower case, so `xx-1` finds XX-1.

Exit codes are as in §3: `0` the move was made, `1` the move isn't allowed, the reason is missing or bad, or the item doesn't exist, and `3` the store or the item file can't be read or rewritten.

## How it works

### The move table

[`states::MOVES`](../src/states.rs) lists each move as a command (`via`), the state it starts from and the state it leads to:

| via    | from        | to          |
| ------ | ----------- | ----------- |
| start  | open        | in-progress |
| submit | in-progress | done        |
| shelve | open        | shelved     |
| shelve | in-progress | shelved     |
| reopen | done        | open        |
| reopen | shelved     | open        |

`states::find(via, from)` looks a move up. Nothing leads into or out of review yet: `submit`'s move to review, `approve` and `reject` arrive with the triggers in slice 9. Until then, an item reaches review only if its file is edited by hand, and every move refuses it.

### A move, step by step

`move_item` in [src/main.rs](../src/main.rs) runs `start`, `submit`, `shelve` and `reopen`:

1. Check that the store exists, and put the ID's key in capitals.
2. Take the store's exclusive lock, and read the item under it.
3. Look the move up. If the diagram doesn't draw it, exit 1 with the moves allowed from the item's state (`states::not_from`).
4. Check the reason, for shelve and reopen.
5. `apply` the move: set `status` and `updated`, move a verification record aside on reopen, write the `unchecked` record on submit, and add one history entry.
6. Write the item with `Store::update_item`. It refuses with exit 3, writing nothing, if the file holds something this version would lose by rewriting it, such as a field a newer tracker added.

The item is read and written under one lock, so moves on the same item happen one after another. When five `submit`s and five `shelve`s run at once on an in-progress item, one wins, and the other nine find it already moved and exit 1.

`status` takes the same path. `states::via_to` maps the state onto its command first, and `states::not_to` words the refusal for a state rather than a command. A `--reason` for a move that takes none, such as `status XX-1 in-progress --reason x`, is refused rather than ignored.

### Reasons

Shelve and reopen need a reason, which goes in the history entry. A reason is trimmed, then must be:

- not empty,
- one line, with no control characters or marks that reorder text, as for titles,
- at most 500 characters, counted as characters rather than bytes. It's printed on one line among others, so details belong in the item's description.

### History

Each move adds exactly one entry to `history`, oldest first. A refused move adds none.

| Field         |                                                                                                            |
| ------------- | ---------------------------------------------------------------------------------------------------------- |
| `at`          | When, to the second. Several moves in one second share a time, so the order of the entries is what counts. |
| `from`, `to`  | The states. Each entry's `to` is the next entry's `from`.                                                  |
| `via`         | The command that made the move. `status <id> done` records `submit`, not `status`.                         |
| `interactive` | Whether stdin was a terminal. See below.                                                                   |
| `reason`      | For shelve and reopen. Left out for other moves.                                                           |

From slice 3, `edit` and `criterion accept` add entries of the same shape, with `from` equal to `to` and a `fields` list (DESIGN.md §2.1).

### Verification

`submit` writes `verification` with `by: "unchecked"`, the time, and a `note` saying no gates ran. Slice 6's gates will write `auto`, and slice 9's `approve` will write `human`.

`verification` is present only while the item is done. Reopening moves it to `previous_verification`, so the last record isn't lost, and each reopen from done replaces the one there. When the item reaches done again, it gets a new `verification`, and `previous_verification` stays until the next reopen. Files written before review 1's fix can hold a `verification` on a shelved item, like fixture `XX-4`. Reopening one moves its record aside too.

### `interactive`

`tty::is_interactive()` in [src/tty.rs](../src/tty.rs) asks whether stdin is a terminal. Typing a command in a shell gives `true`. An agent's shell calls, CI and anything with piped input give `false`. Slice 4's check for human-only commands will call the same function, so the history flag and that check can't disagree about what counts as you.

It's a convention, not security. A command run under `script` gets a terminal and records `true`. DESIGN.md §2.6 therefore has slice 16's digest list every shelve and human action, yours included, so one you didn't make stands out, and §7 records the limit.

## JSON output

| Command                                                         | Output                                                                                                       | Schema                                              |
| --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ | --------------------------------------------------- |
| `start`, `shelve`, `reopen`, and `status` to any state but done | `{"item": {…}}`, the item after the move, as its file holds it                                               | [move.schema.json](../schemas/move.schema.json)     |
| `submit`, `status <id> done`                                    | `{"id", "result": "done", "status": "done", "gates": [], "triggers": [], "holds": [], "item": {…}}`, §3's shape | [submit.schema.json](../schemas/submit.schema.json) |
| A move the diagram doesn't draw                                 | `{"error", "exit_code": 1, "allowed": [{"via", "to", "command"}]}`. `allowed` is empty for an item in review | [error.schema.json](../schemas/error.schema.json)   |
| Any other failure                                               | `{"error", "exit_code"}`                                                                                     | [error.schema.json](../schemas/error.schema.json)   |

`submit`'s `gates`, `triggers` and `holds` are always empty for now, and `result` is always `done`. Slice 6 fills in `gates`, and slice 9 fills in `triggers` and `holds` and adds `"result": "held"` with `"status": "review"`. Agents that parse `submit` now won't break when they do, because no field is renamed or removed.

`item.schema.json` gains `verification`, `previous_verification` and `history`, with a `move` definition for each entry.

## Code tour

| File                                  | What it holds                                                                                                                                                                                                                                                                                |
| ------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [src/states.rs](../src/states.rs)     | The move table, `find`, `via_to`, and the refusals: `not_from` and `not_to` build the message and `allowed`, and `allowed` words the moves as one sentence. `UNCHECKED_NOTE` is the note `submit` writes.                                                                                    |
| [src/item.rs](../src/item.rs)         | `HistoryEntry`, `Via` (with `needs_reason`), `Verification` and `VerifiedBy`, and the `verification`, `previous_verification` and `history` fields of `Item`, each with a serde default so slice 1's files still load. `Status::phrase` gives "in review" rather than "review" for messages. |
| [src/main.rs](../src/main.rs)         | The clap definitions of the five commands, and `move_item`, `status`, `check_reason` and `apply`. `MAX_REASON` is the 500-character limit.                                                                                                                                                   |
| [src/tty.rs](../src/tty.rs)           | `is_interactive`.                                                                                                                                                                                                                                                                            |
| [src/error.rs](../src/error.rs)       | `Error` gains `allowed`, which `fail` adds to the JSON.                                                                                                                                                                                                                                      |
| [tests/states.rs](../tests/states.rs) | Every test for this slice. `moved` runs a move and checks the file matches the output. `refused` and `refused_json` check that a refused move leaves the file byte for byte unchanged. `allowed` returns the commands a refusal lists.                                                       |

## Testing

Every _Done when_ check in PLAN.md's slice 2 has a test in [tests/states.rs](../tests/states.rs):

| _Done when_                                                                 | Tests                                                                                                                                                |
| --------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| open → in-progress → done → open, through the commands and through `status` | `an_item_goes_open_in_progress_done_open_through_the_commands`, `an_item_goes_open_in_progress_done_open_through_status`                             |
| `status <id> open` without `--reason` exits 1                               | `status_open_and_status_shelved_need_a_reason`, which also covers an empty and a blank reason, and `status … shelved`                                |
| `shelve` works from open and from in-progress, and exits 1 without a reason | `shelve_works_from_open_and_from_in_progress`, `shelve_without_a_reason_exits_1`                                                                     |
| `reopen` brings a shelved item back to open                                 | `reopen_brings_a_shelved_item_back_to_open`                                                                                                          |
| open → done exits 1 and lists the allowed moves                             | `open_to_done_exits_1_and_lists_the_allowed_moves`, and every other move the diagram doesn't draw in `every_move_the_diagram_doesnt_draw_is_refused` |
| Every move adds exactly one history entry                                   | `every_move_adds_exactly_one_history_entry` (eight moves, with the earlier entries unchanged), `a_refused_move_adds_no_history_entry`                |
| _Prompt:_ `submit` writes `by: "unchecked"`, the time and a note            | `check_unchecked`, called from the tests above                                                                                                       |
| _Prompt:_ each entry has `interactive`                                      | `check_last_move` checks `false` on every move, and `a_move_from_a_terminal_records_interactive_true` checks `true`                                  |

Further tests cover the edges: `nothing_moves_an_item_to_review_by_hand`, `an_item_in_review_is_refused_by_every_move`, `status_refuses_a_reason_for_a_move_that_takes_none`, `a_reason_must_fit_on_one_line`, `a_reason_can_be_at_most_500_characters`, `a_move_on_a_missing_or_malformed_id_exits_1`, `a_move_finds_an_id_typed_in_lower_case`, `a_move_needs_a_store`, `list_and_project_list_show_the_new_states`, `the_text_output_names_the_move` and `help_shows_that_shelve_and_reopen_need_a_reason`. The tests added after each review are listed with their fixes: [review 1](#fixes-after-review-1) and [review 2](#fixes-after-review-2).

**Tests read the JSON, not the wording.** Refusals are checked through `allowed`, so the messages can be reworded freely, as §3 intends. The exceptions are the reason checks, which look for "needs a reason" or "500" because those refusals have no field of their own, and the text-output and `--help` tests, whose subject is the text.

**`interactive: true` is tested with `script`.** `a_move_from_a_terminal_records_interactive_true` runs `start` under `script -q /dev/null …` on macOS and BSD, or `script -qec "…" /dev/null` on Linux, which gives the command a pseudo-terminal without a pty crate (PLAN.md). It's skipped where `script` isn't installed. With `is_interactive` made to return `false`, it fails.

**The fixtures.** `tests/fixtures/slice-2/` holds three files written by this slice's binary:

- `XX-3`: done and unchecked.
- `XX-4`: done, reopened and shelved, written before review 1, so it still has `verification` on a shelved item.
- `XX-5`: done and reopened, written after review 1, so its record is in `previous_verification`.

[tests/fixtures.rs](../tests/fixtures.rs) checks that each loads, comes back byte for byte, matches `item.schema.json`, loses nothing on a rewrite, and lists exactly as it is on disk. **Never edit a fixture.** `reopening_a_slice_2_file_moves_its_record_aside_too` copies `XX-4` into a store and reopens it.

### Checks done by hand

Review 1 checked these against the debug binary in a temporary store:

- Ten `start XX-3` commands run at once: one exits 0, nine exit 1, and the file has one history entry.
- Five `submit` and five `shelve` commands run at once on one in-progress item: exactly one move wins.
- A file with a field that a newer tracker would write is refused with exit 3, and nothing is written.

The third is now automated as `a_move_refuses_a_file_it_would_lose_fields_from`. The first two aren't automated yet ([test gap 2](#still-open)).

## Decisions worth knowing

- **`via` records the command, not `status`.** `status <id> done` and `submit` leave the same history entry, so slice 16's digest and `audit` read one vocabulary.
- **`--reason` is optional to clap.** A missing reason then gets tracker's own message, which says how to give one, rather than clap's. The usage line shows it as needed for `shelve` and `reopen`.
- **A move that isn't allowed exits 1, not 2.** Exit 2 means a gate refused, and none exist yet. A move the diagram doesn't draw is bad input.
- **`updated` changes on every move**, to the move's time, which is also the history entry's `at`.
- **`submit` returns the item next to §3's fields**, as `item`, following CLAUDE.md's rule that a command returning an item wraps it. §3's example shows it.
- **A move on an item whose project was removed from `config.json` succeeds.** §3 says "a command that takes an item ID needs no project". Slice 7 will need the project's root, so whether that stays allowed is still to decide ([still open](#still-open)).

## Fixes after review 1

[slice-2-review-1.md](./reviews/slice-2-review-1.md) found no bugs in the slice's own behaviour. Its findings were about the shape of the JSON, the `interactive` flag, and decisions DESIGN.md didn't record. Each fix below is in its own commit, and each new test was seen failing first.

The first two commits save work from before the fixes: the review itself (`f97d253`), and a DESIGN.md change that came out of discussing finding 5 (`ee14706`, described under [Security items](#security-items)).

### Fix before slice 3

| Finding                                                      | Commit    | Fix                                                                                                                                                               | Tests                                                                                                                               |
| ------------------------------------------------------------ | --------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| 1. `submit --json` doesn't have §3's shape                   | `d8a9564` | `submit` and `status <id> done` print `id`, `result`, `status`, `gates`, `holds` and `item`, from the new `submit.schema.json`. §3's example now includes `item`. | `submit_prints_the_shape_section_3_gives_it`                                                                                        |
| 2. The allowed moves are only in the message                 | `32ef9f7` | A refused move adds `allowed` to its JSON error, and `error.schema.json` defines it. The tests check the field instead of the wording.                            | `every_move_the_diagram_doesnt_draw_is_refused` and the tests before and after it; `a_wrong_output_is_caught` in `tests/schemas.rs` |
| 3. `interactive` is never tested as `true`, and isn't shared | `a370118` | `tty::is_interactive()` replaces the inline check, and slice 4's prompt says to reuse it.                                                                         | `a_move_from_a_terminal_records_interactive_true`                                                                                   |

### Low severity

| Finding                                  | Commit    | Fix                                                                                                            | Tests                                                                              |
| ---------------------------------------- | --------- | -------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| Reasons have no length limit             | `6855ce9` | At most 500 characters, after trimming. `item.schema.json` and DESIGN.md §3 say so.                            | `a_reason_can_be_at_most_500_characters`, and a case in `a_wrong_output_is_caught` |
| `--reason` shows as optional in `--help` | `529fa36` | The usage lines of `shelve` and `reopen` show `--reason <REASON>`, and `status`'s shows `[--reason <REASON>]`. | `help_shows_that_shelve_and_reopen_need_a_reason`                                  |

### Design decisions

The review listed five places where the code and DESIGN.md disagreed or DESIGN.md was silent. Each was settled after discussion.

| Question                                                             | Decision                                                                                                                                                                                                                                                                                                  | Commits                                 |
| -------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------- |
| 1. What happens to the verification record on reopen?                | It moves to `previous_verification`, which keeps the latest only, so `verification` exists only while the item is done. Slice 3's `list --by` and slice 16's digest and `audit` can read it on its own, though `--by` also checks the state because of files like `XX-4`. DESIGN.md §2.1 and §2.7 say so. | `78c7d51`, and the fixture in `8ad15c1` |
| 2. `verification.note` and `history[].reason` aren't in DESIGN.md    | §4.3's example item now goes through a shelve and a reopen with reasons, and a bullet explains `note`.                                                                                                                                                                                                    | `f817e58`                               |
| 3. How is an `edit` recorded in history?                             | In the same shape as a move, with `from` equal to `to` and a `fields` list. `criterion accept` does the same with `via: "criterion-accept"`. DESIGN.md §2.1 and PLAN.md slices 3 and 6 say so.                                                                                                            | `602cba8`                               |
| 4. Refusals for an item in review say approve and reject don't exist | Right for now. Slice 9's _Done when_ now checks that every such refusal names them and that `allowed` lists them.                                                                                                                                                                                         | `a568739`                               |
| 5. Two items in progress at once                                     | The review was wrong that the tests do this: they submit XX-1 before starting XX-2. The real problem is that G1 refuses `start` for an item with no criteria, which is nearly every test here. Slice 6's prompt now says to give those items a criterion through the test helper.                         | `e405dad`                               |

### Security items

These weren't fixed in this round.

- **Finding 4, the symlinked `.tmp` file**, carried over from slice 1's review 2: a planted `<id>.json.tmp` symlink makes a move overwrite the file it points to. It needs write access to the store, which is mode 0700.
- **Finding 5, faking `interactive`** with `script`: only DESIGN.md changed (`ee14706`). The digest will list every shelve and human action, yours included, each marked by whether it came from a terminal, and a delete will leave a line in `deleted.json` so it can be listed. §6 and §7 record the gap that's left: an agent that also edits the store to cover its tracks. The review's suggestion of having `audit` flag such entries wasn't taken, since the digest covers it.

## Fixes after review 2

[slice-2-review-2.md](./reviews/slice-2-review-2.md) reviewed the branch at `44abdb8`, after review 1's fixes and the move into `docs/`. It found that the move had broken the test suite, and it raised a few things for slices 3 and 9 to rely on. As after review 1, each fix is in its own commit, and each new test was seen failing first.

`cf1f600` saves the review itself.

### Fix before slice 3

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| 1. The move into `docs/` broke the test suite, the links, and what the docs say about themselves | `c7dd093` | Every relative link in `docs/` and CLAUDE.md is rewritten for the new layout. The docs tests point at `docs/DESIGN.md` and check `docs/` and its subfolders. DESIGN.md §4.2's TR example and PLAN.md's self-tracking checkpoint give TR's design as `docs/DESIGN.md`. CLAUDE.md and DESIGN.md §8 describe the layout. | The `docs_links` tests, which failed at `44abdb8` |

### Low severity

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| 3. One file with a `via` this version doesn't know makes `list` exit 3 for the whole store | `b892d73` | `Via::Unknown` reads any other command. The file lists, `--json` prints the command as `unknown`, and `Store::update_item` still refuses to write it back. | `a_history_entry_from_a_newer_tracker_still_lists` |
| 6. The move table has one outcome per command and state | `dfb9579` | Settled for slice 9, nothing to change yet: `allowed` keeps a single `to` per entry and lists `submit` once for each outcome, with the same command (option A). DESIGN.md §3, `error.schema.json` and PLAN.md's slice 9 say so. | None yet. Slice 9's _Done when_ checks the two entries. |
| 8. Shelving from in-progress skips §2.8's rules, and the readme doesn't say so | `477b1ce` | [What's in this slice](#whats-in-this-slice) and [Limits](#limits-and-what-later-slices-change) say so. | None, since it's documentation |

### Design decisions

| Question | Decision | Commit |
| --- | --- | --- |
| 1. `holds` lists only the triggers that fired, so an agent can't tell a quiet trigger from one that didn't run | `submit --json` gains `triggers`, every trigger that ran with whether it fired, between `gates` and `holds`. It's empty until slice 9, and a disabled trigger isn't listed. §3 and its example say so. | `21db5f1`, tested by `submit_prints_the_shape_section_3_gives_it` |
| 2. §2.1 says every history entry has the same fields, but `reason` and `fields` appear only on some | §2.1 names the five fields every entry has, and the moves that add `reason`. | `2bf238c` |

### Test gaps

| Gap | Commit | Test |
| --- | --- | --- |
| 1. The docs tests don't look in `docs/reviews/` | `c7dd093`, `205c0e2` | `docs/reviews/` is checked. A review's line links are checked against the commit in its _Line references_ row, not against today's files, and a review that names no commit has its line links skipped: `a_review_names_the_commit_its_lines_point_at` |
| 3. The "would lose fields" refusal through a move command | `087fe4e` | `a_move_refuses_a_file_it_would_lose_fields_from`: start, status and shelve on a file with a field a newer tracker writes exit 3 and leave it unchanged |
| 4. `created` unchanged by a move | `d68d2a8` | `a_move_changes_updated_but_never_created` |
| 6. A `via` from a later slice | `7386240` | `a_move_refuses_an_item_whose_history_has_a_command_it_doesnt_know`: submit, status and shelve exit 3 without writing, the item still lists, and other items still move |

### Process

- **`44abdb8` was committed with a failing test suite.** It stays in the history. `c7dd093` makes the suite pass again, so skip `44abdb8` when bisecting.
- **The readme's status line and file references** are updated: the links by `c7dd093`, and the status line here.

## Still open

Neither review's remaining items are fixed yet. Where both reviews raised the same thing, it's listed once.

**Security**

- **The symlinked `.tmp` file** (slice 1 review 2, slice 2 review 1 finding 4, slice 2 review 2 finding 2): a planted `<id>.json.tmp` symlink makes a move overwrite the file it points to. It needs write access to the store, which is mode 0700.
- **Faking `interactive`** (review 1 finding 5) is answered by the digest rather than by a check, as described under [Security items](#security-items).

**Low severity** (review 2 numbering)

- **4.** A hand-edited timestamp is blamed on "a newer tracker". Refusing is right, but the message should name the field that would change rather than guess at a newer binary. Also raised by review 1.
- **5.** `status <id> closed` is refused, though §1 calls done "the course's 'closed' state". Accepting `closed` as an alias, or adding a hint, is a design call. Also raised by review 1.
- **7.** A move on an item whose project isn't in `config.json` succeeds. §3 allows it today, but slices 7 and 8 need the project's root, so which commands still work without a project has to be decided by then.
- **9.** The `unchecked` note, "No gates ran: this version of tracker has none yet…", is written into item files in the present tense, and stays there after slice 6 adds gates. A tense-free wording such as "Submitted before tracker had gates, so nothing checked the work" stays true. It's cheapest to change before slice 3, while only the test fixtures hold the old text, and those fixtures stay as they are.

**DESIGN.md**

- **3.** `error.schema.json` has `additionalProperties: false`, and its `allowed[].via` enum lists only today's four commands. Slices 6 and 9 must extend the schema in the same commit as the code. Its description says so for slice 6's fields, but not for the `via` enum.

**Test gaps**

- Concurrent moves on one item (gap 2 in both reviews). It's correct when checked by hand, but a ten-way `start` test, like slice 1's ten-way `create`, would guard the lock-then-read order in `move_item`.
- A move on an item whose project isn't registered (gap 5 in both reviews). No test pins today's behaviour, which item 7 above may change.
- The text output of `shelve` and `reopen` (from review 1's gap 4). `the_text_output_names_the_move` covers only `start`, `submit` and a refusal.

**Process**

- Slice 2 landed as one commit (`54d8f66`) rather than one per step. That's in the history and stays.

## Limits, and what later slices change

- **Slice 3** adds `edit`, whose history entries take the shape above, and `list --by`, which must check the state as well as `verification`.
- **Slice 4**'s helper for human-only commands calls `tty::is_interactive`, and `delete` writes its line to `deleted.json`.
- **Slice 5** skips files it can't parse. A history entry whose `via` this version doesn't know, such as slice 3's `edit` or slice 9's `approve`, doesn't stop the file from loading: `Via::Unknown` reads it, `--json` prints it as `unknown`, and `Store::update_item` refuses to write the file back, because that would lose the command's name.
- **Slice 6**'s G1 refuses `start` without criteria, so the move tests need items with one. Its gates fill in `submit`'s `gates`, and a refused submit exits 2 with `id`, `result`, `gates` and `failures` added to the error.
- **Slice 8** makes shelving an in-progress item follow §2.8: it's refused with exit 2 unless the project's working tree is clean and its project checks pass. Shelving an open item stays as it is.
- **Slice 9** adds review, `approve` and `reject`. `states::find` returns the first move for a command and state, but `submit` from in-progress will lead to done or review depending on the triggers, so the table will need to name both outcomes and let the command pick one. `allowed` will then list `submit` twice, once to done and once to review, with the same command: one entry per arrow in the diagram, as decided after review 2 (DESIGN.md §3). Refusals for an item in review must then name approve and reject.
- **Slice 16**'s digest reads `interactive` and lists every shelve and human action.
- **Item files change shape again later.** Each new field needs a serde default and `skip_serializing_if`, its place in §4.3's order, an entry in `item.schema.json`, and a `tests/fixtures/slice-<n>/` folder of files written by that slice's binary.

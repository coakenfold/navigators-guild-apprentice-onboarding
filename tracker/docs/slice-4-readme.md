# Slice 4: Show and delete

`tracker` is a command-line work tracker, written in Rust, that keeps one central list of work items across many projects and, in later slices, decides when an item is really done. This document covers **slice 4** of the 16 in [PLAN.md](PLAN.md): what it built, how it works, and how to test it. [Slice 1's readme](slice-1-readme.md) covers the store, `create` and `list`, [slice 2's readme](slice-2-readme.md) covers the states, the moves between them and `history`, and [slice 3's readme](slice-3-readme.md) covers priority, kind, labels, `edit` and `list`'s filters.

After slice 4 you can **see an item's full record, and delete one from a real terminal**. The spec is [DESIGN.md](DESIGN.md). This slice implements the `show` and `delete` rows of [§3](DESIGN.md#3-interface), the delete half of [§2.8](DESIGN.md#28-shelve-vs-delete) with its `deleted.json` from [§4.1](DESIGN.md#41-layout), and the confirmation that [§2.7](DESIGN.md#27-human-actions)'s human-only commands share. It also makes the two fixes [slice 3's readme](slice-3-readme.md#fixed-on-slice-4s-branch) left for before slice 4.

| | |
| --- | --- |
| Status | Every _Done when_ check has a test, and every test passes, as does the test command. The by-hand check is done too: a person deleted an item in each state by typing its ID in a real terminal, and a wrong ID cancelled ([Checks done by hand](#checks-done-by-hand)). Slice 3's two fixes before slice 4 are done, each in its own commit ([below](#slice-3s-fixes-before-slice-4)). From [review 1](reviews/slice-4-review-1.md), four bugs, both security items, all four DESIGN.md items and five test gaps are done ([below](#fixes-after-review-1)). The branch isn't merged into `main`. What's left is listed under [Still open](#still-open). |
| Branch | `feat/tracker/slice-4`: `c980317` (`feat: slice 4 first pass`), then `bf229d0` and `1d2f64e` (slice 3's fixes) and `c1a08b2` (slice 3's docs pointed at them) |
| Test command | `cargo fmt --check && cargo clippy -- -D warnings && cargo test`, run in `tracker/` |
| Tests | 166: 38 in `tests/create_and_list.rs`, 34 in `tests/organise.rs`, 27 in `tests/states.rs`, 22 in `tests/show_and_delete.rs`, 12 in `tests/item_files.rs`, 11 in `tests/docs_links.rs`, 6 each in `tests/fixtures.rs` and `tests/output.rs`, 4 each in `tests/schemas.rs` and `tests/store_writes.rs`, and 2 in `tests/item_ids.rs` |

## Contents

- [What's in this slice](#whats-in-this-slice)
- [Try it](#try-it)
- [Commands](#commands)
- [The delete record](#the-delete-record)
- [Slice 3's fixes before slice 4](#slice-3s-fixes-before-slice-4)
- [How it works](#how-it-works)
- [JSON output](#json-output)
- [Code tour](#code-tour)
- [Testing](#testing)
- [Decisions worth knowing](#decisions-worth-knowing)
- [Fixes after review 1](#fixes-after-review-1)
- [Still open](#still-open)
- [Limits, and what later slices change](#limits-and-what-later-slices-change)

## What's in this slice

**Built:**

- `show <id>`, which prints every field the item has, its description as plain text, its verification record and the one from before it was last reopened, and its history, oldest first. Each history entry shows its time, command, move, whether it came from a terminal, and its reason or what an edit changed. Free text is escaped. `--json` prints the item as this version reads it. If its file holds fields this version doesn't know, such as ones a newer tracker added, `show` names them: at the end of the text, and as `warning` beside `item` in the JSON.
- `delete <id>`, which works on an item in any state, only from an interactive terminal. It says what state the item is in and what deleting it removes, asks for the ID to be typed back, and then adds one line to `deleted.json` and removes the item file.
- `tty::confirm`, the one helper for human-only commands: it refuses without a terminal, prints what will be lost, and requires the ID. `delete` is its first user.
- `deleted.json`, one JSON object per line, holding the ID, title, state, time and `interactive` of each deleted item, and nothing else of it.
- `create`'s check against a lost or reset `counters.json` now reads `deleted.json` too, so deleting a project's newest item doesn't remove the last trace of its number.
- `show.schema.json`, `delete.schema.json` and `deleted.schema.json`.
- `Env::in_terminal_typing` in the test harness, which types an answer into a command running under `script`, so the confirmation is tested automatically. If the prompt never appears, or the command never ends, it kills the command and fails after `TERMINAL_TIMEOUT` (20 seconds), showing the output so far, rather than hang.
- Slice 3's two fixes before slice 4: `create` and `edit --label` refuse a capital with no lower-case form, such as `ℂ`, and `edit` keeps and removes a label written twice by hand once ([below](#slice-3s-fixes-before-slice-4)).

**Not built yet:**

- `show --diff`, which prints the item's commits and their combined diff (slice 9, with the files that fired a trigger listed first from slice 10).
- What `show` will print once it exists: criteria and `written` (slice 6), `started` with each check's result at start (slice 7), the verification commit, its ref and each check's saved output (slice 8), `holds` (slice 9), and judge answers with their refs (slice 12).
- Removing `refs/tracker/<ID>` on delete (slice 8, which creates the ref). The warning doesn't mention a ref yet, since none exists.
- The digest that lists each line of `deleted.json` (slice 16).
- The other human-only commands that use `tty::confirm`: `criterion accept` (slice 6), `approve` and `reject` (slice 9) and `digest ack` (slice 16).
- Skipping a file that can't be parsed (slice 5). Today `show` and `delete` of such a file exit 3, so `delete` can't remove a corrupt item file.
- Colour, and `--help` and messages polished to slice 5's standard.

## Try it

Always set `TRACKER_HOME` while developing. Without it, the tool uses your real store at `~/.tracker`.

`delete` needs a terminal. Typed at one, `tracker delete BM-2` asks for the ID and waits. So that the demo can be pasted and run as it is, it types the answer through `script`, as the tests do: `script -q /dev/null …` on macOS, where the demo was run, or `script -qec "…" /dev/null` with util-linux. The `sleep`s let the question appear before the answer arrives, and keep stdin open until the command ends, since macOS's `script` sends end-of-input as soon as its stdin closes.

That recipe is the way round the terminal check that [§7](DESIGN.md#7-honest-limits) describes, and an agent working in this folder can read it here. A delete made through it records `"interactive": true` and looks exactly like one typed by a person, so only a person checking the digest (slice 16) tells them apart.

```sh
cd guild-portfolio/tracker
cargo build

export DEMO=$(mktemp -d)
export TRACKER_HOME=$DEMO/store
export PATH="$PWD/target/debug:$PATH"
mkdir -p $DEMO/bookmarks && cd $DEMO/bookmarks

tracker init
tracker project add bm --name bookmarks --root $DEMO/bookmarks
tracker create "Search filters bookmarks by tag" --kind capability --label needs-review \
    --design DESIGN.md#2.5 --description "Tag chips filter the list."
tracker create "Sort by date"
tracker create "Import from Pocket"
tracker create "Export" --label ℂ                 # exit 1: no lower-case form
tracker edit BM-1 --kind change --unlabel needs-review
tracker start BM-1
tracker submit BM-1
tracker reopen BM-1 --reason "The tag list is missing."
tracker show bm-1
tracker show BM-9                                 # exit 1: no such item
tracker show BM-2 --json
echo BM-2 | tracker delete BM-2                   # exit 1: stdin isn't a terminal
(sleep 1; echo BM-3; sleep 1) | script -q /dev/null tracker delete BM-2   # exit 1: wrong ID
(sleep 1; echo BM-2; sleep 1) | script -q /dev/null tracker delete BM-2
cat $TRACKER_HOME/deleted.json
tracker create "Export"
tracker list
```

Output from `tracker init` on, with stderr included, the temporary folder shown as `$DEMO`, and the carriage returns that `script`'s terminal adds to its lines taken out:

```
Created the store at $DEMO/store.
Registered BM (bookmarks) at $DEMO/bookmarks.
Created BM-1: Search filters bookmarks by tag
Created BM-2: Sort by date
Created BM-3: Import from Pocket
error: A label is stored in lower case, and "ℂ" holds ℂ (U+2102), a capital with no lower-case form. Spell it with letters that have one, such as --label c.
Edited BM-1: changed kind and labels.
BM-1: open → in-progress
BM-1: in-progress → done
Unchecked. No gates ran: this version of tracker has none yet, so nothing checked the work.
BM-1: done → open
BM-1  Search filters bookmarks by tag
  project   BM (bookmarks)
  kind      change
  status    open
  priority  medium
  design    DESIGN.md#2.5
  created   2026-11-17T11:53:10Z
  updated   2026-11-17T11:53:10Z

Description:
  Tag chips filter the list.

Verification: none.

Previous verification, from before it was last reopened:
  by    unchecked
  at    2026-11-17T11:53:10Z
  note  No gates ran: this version of tracker has none yet, so nothing checked the work.

History, oldest first (4 entries):
  2026-11-17T11:53:10Z  edit    stays open          not from a terminal
      changed: kind, labels
      kind before: capability
      labels removed: needs-review
  2026-11-17T11:53:10Z  start   open → in-progress  not from a terminal
  2026-11-17T11:53:10Z  submit  in-progress → done  not from a terminal
  2026-11-17T11:53:10Z  reopen  done → open         not from a terminal
      reason: The tag list is missing.
error: There's no item BM-9.
{
  "item": {
    "id": "BM-2",
    "title": "Sort by date",
    "project": "BM",
    "kind": "change",
    "status": "open",
    "priority": "medium",
    "created": "2026-11-17T11:53:10Z",
    "updated": "2026-11-17T11:53:10Z"
  }
}
error: `tracker delete BM-2` needs a human at an interactive terminal, who types BM-2 to confirm it, and stdin here isn't a terminal. Nothing was changed. Only a human deletes items. An item that shouldn't be done is shelved instead, with `tracker shelve BM-2 --reason "…"`, and a human decides whether to delete it.
BM-2 "Sort by date" is open.
Deleting it removes its file.
Any commits scoped to it stay in git, the ID won't be used again, and deleted.json records the delete for the digest.
Type BM-2 to confirm, or anything else to cancel: BM-3
error: Cancelled: "BM-3" isn't BM-2. Nothing was changed.
BM-2 "Sort by date" is open.
Deleting it removes its file.
Any commits scoped to it stay in git, the ID won't be used again, and deleted.json records the delete for the digest.
Type BM-2 to confirm, or anything else to cancel: BM-2
Deleted BM-2. Its line in deleted.json records the delete, and BM-2 won't be used again.
{"id":"BM-2","title":"Sort by date","status":"open","at":"2026-11-17T11:53:13Z","interactive":true}
Created BM-4: Export
BM  bookmarks
  BM-1  medium  change  open  Search filters bookmarks by tag
  BM-3  medium  change  open  Import from Pocket
  BM-4  medium  change  open  Export

BM only. `--all` for every project.
Open, in-progress and review items. `--status done,shelved` for the others.
```

Notes on the output:

- `--label ℂ` was refused, so that `create "Export"` used no ID.
- `show bm-1` found BM-1, because an ID's key is put in capitals, as for every command.
- BM-1 was reopened, so `show` says it has no verification and prints the record from before the reopen. The edit's entry "stays open", since an edit doesn't move the item, and shows the kind and label it took away.
- The labels row is left out because BM-1 has no labels left. `show` leaves out a field the item doesn't have rather than print it empty.
- `BM-3` typed for `BM-2` cancelled the delete, and the file stayed. The answer after each question is the terminal echoing what was typed.
- The record's `interactive` is `true` because `script` gave `delete` a terminal. `delete` refuses without one, so a record that says `false` was written some other way.
- `create "Export"` got BM-4, not BM-2: a deleted item's ID is never used again.
- Every history entry says "not from a terminal" because the demo ran from a script. Typed at a terminal, those commands would record `true`.

## Commands

| Command | What it does |
| --- | --- |
| `tracker show <id>` | Prints everything about the item. Reads only, takes no lock, and works for an item in any state, including one whose project isn't in `config.json`, which it shows as "(not in config.json)". |
| `tracker delete <id>` | From an interactive terminal only: says what will be lost, asks for the ID, then records the delete in `deleted.json` and removes the item file. Works for an item in any state. |

Both take `--json`. An ID can be typed in lower case. Neither needs a project, since the ID names it (§3).

**Input rules:**

- **The ID** follows slice 1's rule ([slice 1's readme](slice-1-readme.md#decisions-worth-knowing)): anything that isn't `KEY-n` is refused with exit 1, so no ID can name a file outside `items/`.
- **The typed answer** is trimmed and compared with the ID ignoring the case of its letters, so `bm-2` confirms BM-2. Anything else cancels, including an empty line, the end of input (Ctrl-D) and an answer with extra words, such as `BM-2 please`.
- **A label** given to `create` or `edit --label` that still holds a capital after lowercasing, such as `ℂ`, is refused with exit 1 ([below](#slice-3s-fixes-before-slice-4)).

**Exit codes** (§3), as this slice uses them:

| Code | Meaning |
| --- | --- |
| `0` | `show` printed the item, or `delete` deleted it |
| `1` | No such item, or not an ID. For `delete`, also: stdin isn't a terminal, the answer wasn't the ID, or the item changed or was removed while the question waited. For `create` and `edit`, a label with a capital that has no lower-case form |
| `2` | Not used yet. It's reserved for "refused by a gate" (slice 6) |
| `3` | The store, the item file or `deleted.json` can't be read, or `deleted.json` can't be written or the item file removed. An item file that can't be parsed also exits 3, until slice 5 |

## The delete record

`deleted.json`, in the store beside `counters.json`, holds one JSON object per line, one per deleted item, oldest first (§2.8, §4.1):

```
{"id":"BM-2","title":"Sort by date","status":"open","at":"2026-11-17T11:53:13Z","interactive":true}
```

| Field | |
| --- | --- |
| `id` | The deleted item's ID. It's never handed out again. |
| `title` | Its title when it was deleted. |
| `status` | The state it was in. §2.8 calls this the state. The field is named `status`, as in the item file. |
| `at` | When it was deleted, in UTC to the second. |
| `interactive` | Whether stdin was a terminal. `delete` refuses otherwise, so it writes `true`. |

Nothing else of the item is kept: not its description, labels, design or history (§2.8). [deleted.schema.json](../schemas/deleted.schema.json) defines a line, with `additionalProperties: false`, and `a_delete_leaves_one_line_with_the_id_title_state_time_and_interactive_only` checks the keys and that none of the item's other text is in the file.

The file is created by the first delete. `init` doesn't create it, and `require_exists` doesn't need it, so a store with no deletes has no `deleted.json`. Each line is serialized compactly, so a title's line breaks are written as `\n` and every record is one line. It goes through `text::for_json`, which also escapes U+2028 and U+2029, the line and paragraph separators serde_json leaves as they are.

**It's also a tripwire for counters.** [Slice 1's readme](slice-1-readme.md#countersjson) explains why `create` refuses when a project's counter is at or below a number already in `items/`: `counters.json` was lost or reset, and the next number could be a deleted item's. Before this slice, deleting a project's newest item removed the only evidence of its number. `Store::highest_number_used` in [src/store.rs](../src/store.rs) now also reads the IDs in `deleted.json`, and the refusal says which it found, such as "XX-2 was used by an item since deleted". A line that doesn't parse is skipped there, since the check is a tripwire and `create` shouldn't stop over a line it doesn't need. [§4.4](DESIGN.md#44-handling-bad-data) says so.

## Slice 3's fixes before slice 4

[Slice 3's readme](slice-3-readme.md#fixed-on-slice-4s-branch) left two fixes for before slice 4, raised by its [review 1](reviews/slice-3-review-1.md#fix-before-slice-4) and [review 2](reviews/slice-3-review-2.md#fix-before-slice-4). Each let `create`, `edit`, `list --json` and now `show --json` print an item that breaks `item.schema.json`. Each fix is in its own commit with its test, and each test was seen failing first, against `c980317`.

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Review 1's finding 1, and review 2's finding 1: labels like `ℂ` have no lower-case form | `bf229d0` | `check_new_labels` in [src/main.rs](../src/main.rs) refuses a label that still holds a capital after lowercasing, naming the character and its code point, with exit 1. `--unlabel` still takes one, so a hand-written one can be removed. [§2.1](DESIGN.md#21-items), `create --help` and `edit --help` say so. | `a_label_with_no_lower_case_form_is_refused`: `ℂ`, `ϒ`, `𝐀` and `set-ℂ` are refused by `create` and `edit --label`, with nothing written. `ǅ` and `Ⓐ`, which have lower-case forms, are still taken, as `ǆ` and `ⓐ`. A hand-written `ℂ` is removed by `--unlabel`. Before the fix, `create --label ℂ` exited 0 |
| Review 2's finding 2: a label written twice by hand is recorded twice in `labels_removed` | `1d2f64e` | `edit` rebuilds the labels from an empty list through `add_labels`, the kept ones and then the new ones, and builds `labels_removed` the same way, so each label appears once in both. | `a_label_written_twice_by_hand_is_kept_and_removed_once`: `--unlabel bug` on `["bug", "ui", "bug"]` leaves `["ui"]` and records `["bug"]`, and `--label x` on `["bug", "bug"]` gives `["bug", "x"]` with nothing recorded as removed, both checked against `edit.schema.json` through `Env::json`. Before the fix, the first edit's output didn't match the schema |

`c1a08b2` points slice 3's readme and the two reviews' outcome tables at these commits.

A few consequences:

- **A new label with a capital that has no lower-case form is refused**, with exit 1. Labels already stored keep working.
- **`edit --label` or `--unlabel` on a file with a duplicate label tidies it**, and names `labels` in `fields`, even when dropping the copy was the only change.

## How it works

### `show`

`show` in [src/main.rs](../src/main.rs):

1. Checks that the store exists and puts the ID's key in capitals.
2. Reads the item with `Store::load_item`, which exits 1 for a missing item and 3 for a file it can't read or parse. It takes no lock, as `list` doesn't: every write is a rename (slice 1's [atomic writes](slice-1-readme.md#atomic-writes)), so a reader sees the whole old file or the whole new one.
3. Reads `config.json` for the project's name. A project that isn't there is shown as "(not in config.json)".
4. Prints the heading line, ID and title, then the fields as a `table` of two columns: project, kind, status, priority, labels and design if the item has them, created and updated.
5. Prints the description, if there is one, under "Description:", indented, with its line breaks kept.
6. Prints "Verification:" with `verification_rows`, or "Verification: none.", then "Previous verification, from before it was last reopened:" if there is one.
7. Prints the history: one `table` row per entry, with the time, command, `from → to` or "stays <state>" for an entry that doesn't move the item, and whether it came from a terminal. Each row is followed by indented lines for its `reason`, `fields` ("changed:"), `kind_before` ("kind before:") and `labels_removed` ("labels removed:"). An item with no entries says "History: none yet.". `entries` gives "1 entry" or "6 entries".

**Escaping.** A file edited by hand can hold anything, so every piece of free text is escaped before it's placed. `table` escapes each cell. The title and each history detail go through `text::for_terminal` with line breaks escaped too, so they can't start a line of their own. The description goes through it with `\n` kept, so its paragraphs survive. `main` then escapes the whole output once more, as for every command. `show_escapes_text_edited_by_hand` checks that an escape sequence, a bell, a C1 control, a line separator and a mark that reorders text, in the title, description or a reason, never reach the terminal.

A history entry whose command this version doesn't know, from a file a newer tracker wrote, is shown with the command `unknown`, as `list --json` prints it ([slice 2's readme](slice-2-readme.md#fixes-after-review-2)). `show` never writes, so the file stays as it was.

### The confirmation

`tty::confirm` in [src/tty.rs](../src/tty.rs) takes the command as typed (`tracker delete BM-2`), the ID, the warning, and a sentence saying what an agent can do instead, which may be empty:

1. **Refuse without a terminal.** If `tty::is_interactive()` says stdin isn't a terminal, it returns exit 1, saying that a human must type the ID and that nothing changed, followed by the caller's sentence. It's the same function that sets history's `interactive`, so the two can't disagree about what counts as a human. It's a convention, not security: a command run under `script` gets a terminal ([§7](DESIGN.md#7-honest-limits)).
2. **Warn and ask.** It writes the warning, escaped, then "Type <ID> to confirm, or anything else to cancel: " to **stderr**, so `--json` output on stdout stays one object.
3. **Read one line** from stdin. The trimmed answer must be the ID, ignoring case. Otherwise it returns exit 1, with "Cancelled: \"BM-3\" isn't BM-2.", "Cancelled: nothing was typed…" or "Cancelled: the input ended…". Every message ends "Nothing was changed."

It doesn't take the lock and doesn't know what the command will change. That's the caller's job, after it returns.

### A delete, step by step

`delete` in [src/main.rs](../src/main.rs):

1. Checks that the store exists and puts the ID's key in capitals.
2. Reads the item and its file's text in one read, with `Store::load_item_with_text`. A missing item exits 1 here, before the terminal check, so an agent asking to delete an ID that doesn't exist is told it doesn't exist. A file holding fields this version doesn't know, such as one a newer tracker wrote, exits 3 here, naming them, and nothing is asked. A newer tracker may have more to remove than this version knows about, such as slice 8's git ref, and a delete can't be undone, so it's left to the tracker that wrote the file. `Store::update_item` refuses to rewrite such a file for the same reason.
3. Builds the warning with `deletion_warning` and calls `tty::confirm`, without the lock. Holding the lock while a person reads and decides would stop every other command, an agent's `create` included, for as long as that takes.
4. Takes the store's lock with `Store::lock_unchanged`, which reads the file's text again. If the file is gone, it exits 1 ("was removed while you were deciding"). If the text differs at all from what was read in step 2, it exits 1 ("changed while you were deciding") and deletes nothing, so what's deleted is exactly what the warning described (§4.4). The item in step 2 was parsed from that same text, so the warning and this check describe the same file.
5. Builds the record (ID, title, state, the current time and `tty::is_interactive()`) and hands it, as one line of JSON, to `Store::delete_item` in [src/store.rs](../src/store.rs).
6. `delete_item` appends the line to `deleted.json` through `write_file`, then removes the item file and flushes `items/`. The record is written first, so a crash between the two leaves a delete that's listed but didn't happen, which you'd notice, rather than one that happened unlisted. If the file can't be removed, it puts `deleted.json` back as it was, or removes it if this was the first delete, and exits 3. If even that fails, the message says to remove the last line by hand. `a_delete_whose_file_cant_be_removed_takes_its_record_back_out` tests both cases with `items/` set to `chmod 500`, on Unix, and skips where permissions aren't enforced, as for root.

Appending reads the whole file, adds a newline if the last line has none, adds the record and writes the result through `write_file`, so `deleted.json` is never half-written, like every other file in the store.

### What the warning says

`deletion_warning` in [src/main.rs](../src/main.rs) writes three lines: the item's state, what deleting it removes, and what stays.

| State | First line |
| --- | --- |
| open | `BM-1 "title" is open.` |
| in-progress | `BM-1 "title" is in-progress, so someone may be working on it.` |
| review | `BM-1 "title" is in review, waiting for a human's decision.` |
| done | `BM-1 "title" is done at 2026-11-17T11:53:10Z, unchecked: no gates ran.`, or "done, with no verification record" for a file edited by hand |
| shelved | `BM-1 "title" is shelved.` |

The second line lists, as they apply: "its file", "its history (N entries)", "its verification record", and "the verification record from before it was reopened". The third says that commits scoped to the item stay in git, that the ID won't be used again, and that `deleted.json` records the delete for the digest. The title is escaped on its own, so a line break in a hand-edited title can't add a line to the warning.

This is §2.8's example, which review 1 brought into line with the code. It names the title, so you can tell which item you're about to delete. It says "deleted.json records the delete for the digest", since the digest arrives in slice 16. And the question is the shared helper's "Type BM-2 to confirm, or anything else to cancel:". The example's commit, "at a41be07", and `refs/tracker/BM-12` arrive with slice 8's commit and ref.

### Labels being added

`check_new_labels` in [src/main.rs](../src/main.rs) checks each label `create` and `edit --label` add, after `check_label` has trimmed and lowercased it. It now refuses one that still holds a character `char::is_uppercase` counts as a capital, before the comma and 50-character rules. `--unlabel` and `list --label` go through `check_label` alone, so they still take a label a hand edit put in a file.

`edit` builds the new labels only when it's given `--label` or `--unlabel`: the item's labels minus those named by `--unlabel`, then those named by `--label`, each added through `add_labels`, which skips one already there. `labels_removed` is the item's labels that aren't in the result, through `add_labels` too. If the result differs from the file's labels, `labels` goes into `fields`.

## JSON output

| Command | Output | Schema |
| --- | --- | --- |
| `show` | `{"item": {…}}`: the item as this version reads it, with `warning` beside it naming any field the file holds that this version doesn't know | [show.schema.json](../schemas/show.schema.json) |
| `delete` | `{"deleted": {"id", "title", "status", "at", "interactive"}}`: the line just added to `deleted.json`. The warning and question go to stderr | [delete.schema.json](../schemas/delete.schema.json) |
| A line of `deleted.json` | `{"id", "title", "status", "at", "interactive"}`, and nothing else | [deleted.schema.json](../schemas/deleted.schema.json) |
| Any failure, including a refused or cancelled delete | `{"error", "exit_code"}`, as before | [error.schema.json](../schemas/error.schema.json) |

`deleted.schema.json` reuses `item.schema.json`'s rules for the ID, title, state and time through `$ref`s, so a title in the record follows the same one-line rule as a title in an item.

`show` returns the item as every other command does, so the fields later slices add to the item (`written` and `criteria` in slice 6, `started` in slice 7, and the rest) appear in `show --json` with no change to `show.schema.json`. `show --diff` (slice 9) adds its commits and diff beside `item`, which needs a new property in `show.schema.json`, since it allows no field it doesn't name. Slice 8 adds no field to the delete record: removing the ref is part of the delete, not something the record holds.

## Code tour

| File | What changed |
| --- | --- |
| [src/tty.rs](../src/tty.rs) | `confirm`, the shared confirmation for human-only commands, beside `is_interactive`. |
| [src/main.rs](../src/main.rs) | The `Show` and `Delete` variants of `Command`, with their `--help` text. `show`, with `verification_rows` and `entries`. `delete` and `deletion_warning`. `init`'s note about a lost `counters.json` now says `create` checks `deleted.json` too. For slice 3's fixes, `check_new_labels` refuses a capital with no lower-case form, the `--label` help of `create` and `edit` says so, and `edit` rebuilds the labels through `add_labels` only when given `--label` or `--unlabel`. |
| [src/store.rs](../src/store.rs) | `deleted_path`, `item_text`, which reads an item file's text exactly as it is on disk, `load_item_with_text`, which reads the item and that text in one read, `lock_unchanged`, which takes the lock and refuses if that text has changed, and `delete_item`. `next_id`'s tripwire becomes `highest_number_used`, which reads `items/` and, through `read_deleted`, `deleted.json`, and says which it found. |
| [schemas/show.schema.json](../schemas/show.schema.json), [schemas/delete.schema.json](../schemas/delete.schema.json), [schemas/deleted.schema.json](../schemas/deleted.schema.json) | New, as described under [JSON output](#json-output). |
| [tests/show_and_delete.rs](../tests/show_and_delete.rs) | Every test for `show` and `delete`. `deleted` reads `deleted.json` and checks each line against its schema. `edit_file` changes an item's file as a hand edit would. `PROMPT` is the end of the question the tests wait for. |
| [tests/common/mod.rs](../tests/common/mod.rs) | `Env::in_terminal_typing` and `Env::in_terminal_typing_sh`, through `typing`, and `script_cmd`, which `in_terminal` now shares, takes any command, and sets `TRACKER` to the binary. `typing` reads `script`'s output on its own thread, and `read_until` waits for it with a deadline. `in_terminal_typing_within` takes a shorter deadline, for the test of the deadline itself. `schema_for` maps `show` and `delete` to their schemas. |
| [tests/organise.rs](../tests/organise.rs) | `a_label_with_no_lower_case_form_is_refused` and `a_label_written_twice_by_hand_is_kept_and_removed_once`, for slice 3's fixes. |
| [tests/fixtures.rs](../tests/fixtures.rs) | `the_binary_shows_every_fixture_with_each_history_time`. |
| [tests/item_files.rs](../tests/item_files.rs) | `show_prints_an_item_whose_history_has_a_command_it_doesnt_know`. |
| [tests/schemas.rs](../tests/schemas.rs) | `a_wrong_show_or_delete_output_is_caught`. |
| [tests/create_and_list.rs](../tests/create_and_list.rs) | `commands_without_a_store_say_run_tracker_init` covers `show` and `delete` too. |
| [DESIGN.md](DESIGN.md) | §2.7 describes the shared confirmation: the refusal without a terminal, the question on stderr, cancelling with exit 1, the ID in either case, and the re-read under the lock. §4.1 says `deleted.json` holds one JSON object per line. §4.4 says the item files and `deleted.json` are a tripwire for `counters.json`. §2.1 says `create` and `edit --label` refuse a capital with no lower-case form. |
| [PLAN.md](PLAN.md) | Slice 4's Status is `done`. |
| [slice-3-readme.md](slice-3-readme.md), [slice-3-review-1.md](reviews/slice-3-review-1.md), [slice-3-review-2.md](reviews/slice-3-review-2.md) | Point at slice 3's fixes, made on this branch (`c1a08b2`). |

**To add a human-only command:** check its input and read what it will change, build a warning that says what will be lost, and call `tty::confirm` with the command as typed, the ID, the warning, and what an agent can do instead (or `""`). Then take the lock with `Store::lock_unchanged`, passing the text the warning was built from, which refuses with exit 1 if the item changed since, and only then write. Record the action where §2.7 says the digest will find it. Don't apply it to `edit` of an item in review, which needs a terminal but not a typed ID ([slice 3's readme](slice-3-readme.md#an-edit-to-an-item-in-review)).

## Testing

```sh
cd guild-portfolio/tracker
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

All three pass on the branch, and so does `cargo clippy --all-targets -- -D warnings`. To run this slice's tests alone: `cargo test --test show_and_delete`.

**The harness.** PLAN.md asks for no pty crate, and says interactive commands are checked by hand, with only the refusal of piped stdin tested automatically. This slice goes further with `script`. [tests/common/mod.rs](../tests/common/mod.rs) gains `Env::in_terminal_typing(args, prompt, answer, meanwhile)`. It runs `tracker` under `script`, reads its output until `prompt` appears, calls `meanwhile`, types `answer` and Enter, and returns the terminal's output, which mixes stdout, stderr and the echoed answer, with whether the command succeeded. Two details make it work on macOS: the answer is typed only after the question appears, because input typed before the command reads it is lost, and stdin stays open until the command ends, because macOS's `script` sends end-of-input as soon as its stdin closes. `Env::in_terminal_typing_sh` does the same for a line of `sh` in which `$TRACKER` is the binary, so a test can redirect stdout to a file and still leave stdin on the terminal. Like `Env::in_terminal`, both return `None` where `script` isn't installed, and the test returns early. On macOS, where the tests were run, none was skipped.

### Coverage of the _Done when_ checks

Every _Done when_ check in [PLAN.md's slice 4](PLAN.md#slice-4-show-and-delete) has a test in [tests/show_and_delete.rs](../tests/show_and_delete.rs), unless noted:

| _Done when_ | Tests |
| --- | --- |
| `show` of a missing ID exits 1 | `show_of_a_missing_id_exits_1`, which also covers something that isn't an ID, the text form, and an ID in lower case |
| `echo XX-1 \| tracker delete XX-1` is refused and the file remains | `echo_the_id_into_delete_is_refused_and_the_file_remains`, as text and as JSON, and with nothing on stdin. The file stays byte for byte, and no `deleted.json` is written |
| By hand in a terminal, for an item in each state: the prompt names the state and what will be lost, typing the ID deletes it, and a wrong ID cancels | By a person in a real terminal ([Checks done by hand](#checks-done-by-hand)), and automatically under `script` by `in_a_terminal_typing_the_id_deletes_an_item_in_each_state` (open, in-progress, done, shelved, and review by a hand edit, each with its exact state line and list of what's lost) and `in_a_terminal_a_wrong_id_cancels` (another ID, extra words and an empty line, then the ID in lower case) |
| Deleting `XX-2` and then creating an item gives `XX-3` | `deleting_xx_2_and_then_creating_an_item_gives_xx_3` |
| A delete leaves one line in `deleted.json` with the ID, title, state, time and `interactive`, and nothing else of the item | `a_delete_leaves_one_line_with_the_id_title_state_time_and_interactive_only`, which also checks that the description, labels, design and history aren't in the file, and that `items/` is empty |
| _Prompt:_ `show` prints every field, the verification record, and history with timestamps | `show_prints_every_field_the_verification_records_and_history_with_times`, `show_json_is_the_item_exactly_as_its_file_holds_it`, `show_says_when_an_item_has_no_verification_or_history` |
| _Prompt:_ one shared helper that refuses without `tty::is_interactive()`, prints what will be lost, and requires the ID | `tty::confirm`, through every `delete` test above |
| _Slice 3's readme:_ `show` escapes the control characters in labels, design, description and history | `show_escapes_text_edited_by_hand`, for the title, description and a reason. Labels and design go through the same `table` cell escaping as the other fields |
| _Slice 3's readme:_ `delete` works on items with any of slice 3's fields | `a_delete_leaves_one_line_with_the_id_title_state_time_and_interactive_only` deletes one with labels, a design and a description |
| _Slice 3's readme:_ its two fixes before slice 4 | `a_label_with_no_lower_case_form_is_refused` and `a_label_written_twice_by_hand_is_kept_and_removed_once` in `tests/organise.rs` |
| _Integration risk:_ files from earlier slices still load | `the_binary_shows_every_fixture_with_each_history_time` in `tests/fixtures.rs` shows every fixture from slices 1–3 and checks its JSON and each history time |

Further tests cover the edges: `delete_of_a_missing_id_exits_1_and_writes_nothing`, `delete_json_in_a_terminal_prints_only_the_record_on_stdout`, `an_item_changed_while_deciding_isnt_deleted`, `a_lost_counter_doesnt_hand_out_a_deleted_id`, `delete_appends_to_a_deleted_json_with_no_final_newline`, `show_prints_an_item_whose_history_has_a_command_it_doesnt_know` in `tests/item_files.rs`, `a_wrong_show_or_delete_output_is_caught` in `tests/schemas.rs`, and `commands_without_a_store_say_run_tracker_init` in `tests/create_and_list.rs`.

**`an_item_changed_while_deciding_isnt_deleted`** uses `meanwhile` to run an agent's `edit` while the question waits, then types the ID. The delete exits 1, the edit is kept, and no record is written. It's the only end-to-end test of the re-read under the lock, which slice 9's `approve` will need too. `lock_unchanged_holds_the_lock_only_while_the_file_is_as_it_was` in [tests/item_files.rs](../tests/item_files.rs) tests the helper itself, including an item removed while the question waits.

**`delete_json_in_a_terminal_prints_only_the_record_on_stdout`** redirects stdout to a file while stdin stays on the terminal. The file parses as one object that matches `delete.schema.json` and equals the line in `deleted.json`, and the question still appears on the terminal, so it went to stderr.

**Tests read the JSON and the files**, except where the text is the subject: `show`'s layout, the warning's lines, and the cancel messages, which have no field of their own.

**Seen failing.** The first pass landed as one commit with its tests (`c980317`), so there's no record of its tests failing before the code. Three were checked by breaking the code on purpose at `c1a08b2`, and each broken version failed exactly its test and none of the other 14 in `tests/show_and_delete.rs`: `tty::confirm` accepting any answer failed `in_a_terminal_a_wrong_id_cancels`, `delete` skipping the re-read failed `an_item_changed_while_deciding_isnt_deleted`, and `highest_number_used` ignoring `deleted.json` failed `a_lost_counter_doesnt_hand_out_a_deleted_id`. The two tests for slice 3's fixes were written first, and seen failing against `c980317`.

**No new fixtures.** This slice doesn't change the item file's shape, so it adds no `tests/fixtures/slice-4/` folder. `deleted.json` is a new file, but no earlier slice wrote one, so there's no older form to keep loading.

### Checks done by hand

The demo under [Try it](#try-it) was run against the debug binary, and its output is pasted unchanged apart from the temporary folder and the carriage returns. Two more checks were run against the debug binary in a temporary store:

- With XX's entry removed from `config.json`, `tracker show XX-1` exits 0 and prints "XX (not in config.json)".
- With XX-1's file cut to `{`, `tracker show XX-1` and `echo XX-1 | tracker delete XX-1` both exit 3, naming the file and the parser's line and column.

**Typed by a person.** PLAN.md's _Done when_ and its integration risks ask for each interactive command to be tried once by a person in a real terminal. On 2026-11-17 a person typed the demo up to `tracker show BM-2 --json` at a terminal, against the debug binary, and then deleted an item in each state by typing its ID at the question: BM-2 and BM-3 open (BM-3's ID typed as `bm-3`), BM-4 in-progress, BM-5 done, BM-6 shelved, and BM-7 in review after its file's `status` was edited by hand.

- Each warning's first line matched the table under [What the warning says](#what-the-warning-says). The second listed what applied: the file alone for the open items and the one in review, which had no history, the file and its history for the in-progress and shelved ones, and the file, its history and its verification record for the done one.
- A wrong ID, an empty line, and a whole command typed at the question each cancelled with "Nothing was changed." and left the item in place.
- `deleted.json` then held six lines, one per delete, with the states open, open, in-progress, done, shelved and review, each with `"interactive":true`.
- Typed at a terminal, every history entry `show` printed said "from a terminal", where the demo's say "not from a terminal".

## Decisions worth knowing

- **The question goes to stderr**, with the warning, so `delete --json` at a terminal still prints exactly one object on stdout. §2.7 says so.
- **The lock isn't held while you decide.** It would stop every other command, an agent's included, for as long as the question waits. Instead `delete` reads the file's text again under the lock, through `Store::lock_unchanged`, and refuses with exit 1 if it changed at all. Every later human-only command calls the same helper (§2.7, §4.4).
- **Cancelling exits 1**, like other bad input. A wrong answer, an empty line and the end of input all cancel. The ID can be typed in either case, as on the command line.
- **The terminal check comes after the item is read.** An agent that asks to delete a missing ID is told it doesn't exist, and one that asks to delete a real item is told it needs a terminal and to shelve instead. Neither changes anything.
- **The delete record is written before the file is removed**, and taken back out if the removal fails, so the store never holds a delete that happened without its line.
- **The record's state field is `status`**, as in the item file, though §2.8 calls it the state.
- **`deleted.json` is one JSON object per line**, so a delete appends a line rather than rewriting a list, and slice 16 can read it line by line. It's created by the first delete, not by `init`.
- **`deleted.json` is also a tripwire for counters.** Without it, deleting a project's newest item would leave a reset counter free to hand that ID out again.
- **The warning names the title**, so you can tell which item you're about to delete. §2.8's example now does too.
- **`show` leaves out the fields an item doesn't have** rather than print empty rows, but always says whether there's a verification record and whether there's any history.
- **`show` works without the project in `config.json`**, like the moves and `edit`, and says "(not in config.json)". It still needs `config.json` to parse, as every command does: review 1's finding 11 was settled that way, and §3 says so.
- **The confirmation isn't applied to `edit` of an item in review.** That edit needs a terminal but not a typed ID: it isn't one of §2.7's human actions ([slice 3's readme](slice-3-readme.md#an-edit-to-an-item-in-review)).
- **A label with a capital that has no lower-case form is refused, not changed.** There's no lower-case letter to change `ℂ` into, and `item.schema.json` forbids capitals, so refusing it keeps the two in step. Slice 3 review 2's other option, ASCII-only labels, would also have closed this, but it's a wider decision, about lookalike labels, and is still open ([below](#still-open)).
- **The capital check uses `char::is_uppercase`**, with no new crate. Run over every Unicode character after lowercasing, it refuses all 471 that the schema's `\p{Lu}\p{Lt}` pattern forbids. It also refuses 78 enclosed capitals, such as `🄰` and `🅰`, which the schema allows because they aren't letters. They read as capitals, so refusing them too costs nothing.
- **`edit` rebuilds the labels only when it's given `--label` or `--unlabel`.** Then a label written twice by hand is kept once and recorded once. An edit of other fields leaves the labels as the file holds them, so `edit --priority low` doesn't report a labels change it wasn't asked for. The cost is that its `--json` still prints the duplicate, which breaks `item.schema.json` until a label edit tidies it (slice 3 review 1's finding 4, still open).

## Fixes after review 1

[slice-4-review-1.md](reviews/slice-4-review-1.md) reviewed the branch at `75ba405`. It found five bugs, two security notes, four places where the code and DESIGN.md disagree or the design is unclear, six test gaps and three process points. Each fix below is in its own commit. Finding 3 and test gap 5 are under [Still open](#still-open).

### Bugs

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| 1. `show --json` isn't "the item exactly as its file holds it" | `6c1a831` | The review's option (a). `show` calls `item::lost_on_rewrite` and names the fields this version doesn't know, or can't show in full, at the end of the text and as `warning` beside `item`. `show.schema.json`, this readme and the test name no longer claim exactness. `Store::load_item_with_text` reads the item and its text in one read. | `show_names_the_fields_of_a_file_from_a_newer_tracker`, `show_prints_an_item_whose_history_has_a_command_it_doesnt_know` |
| 2. `delete`'s refusal tells an agent to shelve items that can't be shelved | `ca0d1b3` | The shelve advice is given only where §2.2 draws a shelve, for open and in-progress items. Review, done and shelved items are told only that a human decides. | `delete_refused_without_a_terminal_suggests_shelve_only_where_it_works` |
| 4. The warning calls an unchecked done item "verified" | `9ed9576` | "done at <time>, unchecked: no gates ran". An `auto` record reads "verified by its gates" and a `human` one "approved by a human". | `in_a_terminal_typing_the_id_deletes_an_item_in_each_state` |
| 5. `delete`'s warning understates what's lost in a file from a newer tracker | `bef1a78` | The review's second option: `delete` exits 3 before asking, naming the fields, as `update_item` does. A newer tracker may have more to remove, such as slice 8's ref, and a delete can't be undone. | `delete_refuses_a_file_from_a_newer_tracker_before_asking` |

### Security items

| Finding | Commit | Fix |
| --- | --- | --- |
| 6. The readme carries a recipe for getting past the terminal check | `41a1197` | No code change. §7 and [Try it](#try-it) say that the `script` recipe is the bypass §7 describes, and that only the digest tells a delete made through it from mine. |
| 7. A forged `deleted.json` line blocks `create` for its project | `f7dd832` | No code change. §3's `audit` row and PLAN.md's slice 16 _Done when_ have `audit` report a `deleted.json` ID at or above its project's counter. |

### Design

| Finding | Commit | Fix |
| --- | --- | --- |
| 8. §2.8's example is out of date | `fc24732` | The example now shows what `delete` prints: the title, "deleted.json records the delete for the digest" and the shared question. Its commit and `refs/tracker/BM-12` stay, for slice 8. |
| 9. `tty::confirm` doesn't own the re-read under the lock | `5a51de2` | `Store::lock_unchanged(id, before, command)` takes the lock and refuses with exit 1 if the item was removed or changed. `delete` calls it, and `criterion accept`, `approve` and `reject` will. `lock_unchanged_holds_the_lock_only_while_the_file_is_as_it_was` tests it, including the removal, which had no test. |
| 10. What `digest ack` asks me to type | `44b2d11` | I type `ack`, since a digest belongs to a project, not an item. It has no item file for `Store::lock_unchanged` to read again, so under the lock it builds the digest again, and if that differs from the one it showed, it records nothing and exits 1. §2.7, `tty::confirm`'s comment and PLAN.md's slice 16 say so. |
| 11. `show` needs a `config.json` that parses | `1fcdeec` | Kept. `show` exits 3 when `config.json` can't be parsed, like every other command, rather than print the item with "(config.json can't be read)" for the project's name. §3 says so. |

### Test gaps

| Gap | Commit | Test |
| --- | --- | --- |
| 1. The terminal harness hangs rather than fails | `ef82659` | `typing` reads `script`'s output on a thread and gives up after `TERMINAL_TIMEOUT`, killing the command and panicking with the output so far. `the_terminal_harness_fails_rather_than_hangs_when_the_prompt_never_comes` |
| 2. `delete_item`'s rollback isn't tested | `b44b843` | `a_delete_whose_file_cant_be_removed_takes_its_record_back_out`, seen failing with the rollback removed |
| 3. No test of a newer tracker's file in `show --json` or `delete` | `6c1a831`, `bef1a78` | The tests for findings 1 and 5 |
| 4. A cancelled delete with `--json` | `ce27825` | `delete_json_in_a_terminal_cancelled_prints_only_the_error_on_stdout` |
| 6. `show` of an item whose project isn't in `config.json` | `96ee241` | `show_and_delete_work_on_an_item_whose_project_isnt_in_config_json` |

## Still open

Nothing from this slice's own _Done when_ checks is uncovered, and the one that needs a person has been done. Everything below but the first group is carried over from [slice 3's readme](slice-3-readme.md#still-open), which carries slice 2's open items as well.

**This slice** (review 1)

- **3. A title holding a line break breaks `deleted.schema.json`**, in `delete --json` and in the line kept in `deleted.json`.
- **Test gap 5. The terminal tests have only run on macOS.** The util-linux branch of `script_cmd` is untried with piped stdin.
- **The slice landed as one commit** (`c980317`, 1197 lines added across 14 files), as slices 2 and 3 did. The earlier slices' reviews asked for one commit per step rather than one for the whole slice. That's in the history and stays.

**Security** (slice 3 review 2)

- **3. Lookalike labels**, such as one with a Unicode hyphen that looks like `needs-review`. `show` prints them as they are, so they look the same there too. Making labels ASCII is still one of the two choices. `bf229d0` refuses `ℂ` without it, so the choice is now only about lookalikes.
- **4. Whether T9 keeps holding an item whose `needs-review` an edit removed**, for slice 9 to decide.

**Low severity** (slice 3 review 1)

- **4. Hand-edited labels aren't matched case-insensitively.** A capital such as `Bug` isn't found by `list --label bug` or removed by `--unlabel bug`, and the item's `--json` output, `show --json` included, breaks the schema. A duplicate is now kept once by an edit that names a label (`1d2f64e`), but other edits leave it.
- **5. Invisible characters are accepted in labels and titles.** `show` doesn't escape zero-width characters, since `text::changes_display` doesn't count them.

**Test gaps** (slice 3)

- Two projects in the text list, and hand-edited labels.

**Process** (slices 2 and 3)

- **The `unchecked` note is still in the present tense**, and `show` now prints it under both verification records. It's cheapest to change before self-tracking starts after slice 6.
- **PLAN.md marks this slice `done` before any review**, in `c980317`, as slices 2 and 3 did, so the Status column still means "built", not "built and reviewed".

**Carried over from slice 2**, untouched by this slice:

- **A hand-edited timestamp is blamed on "a newer tracker"** (slice 2 review 2, item 4).
- **`status <id> closed` is refused** (slice 2 review 2, item 5).
- **Commands on an item whose project isn't in `config.json`** (slice 2 review 2, item 7). `show` and `delete` now work on one too, which `show_and_delete_work_on_an_item_whose_project_isnt_in_config_json` pins. The moves and `edit` still have no such test.
- **`error.schema.json`'s `allowed[].via` enum lists only today's four moves** (slice 2 review 2, item 3). `delete` isn't a move, so it doesn't appear there.
- **Concurrent moves on one item aren't tested**, and neither are a delete and a move on one item at the same moment, beyond the re-read test.
- **The text output of `shelve` and `reopen` isn't tested.**

## Limits, and what later slices change

- **Slice 5** skips files that can't be parsed in listings. It must decide what `show` and `delete` do with one: today both exit 3, so a corrupt item file can only be removed by hand, which leaves no line in `deleted.json`. It adds colour, which must stay out of `show`'s output when it isn't a terminal, as for `list`, and must not reach the question on stderr when stderr isn't a terminal. Its "No temporary files are left in the store after any command" check covers `delete`'s write of `deleted.json`. It's also the natural place for slice 3 review 1's finding 5, invisible characters, which `show` would then escape.
- **Slice 6** adds `criteria`, `written` and `evidence`, which `show` must print, and which the warning should name ("its criteria (N)"). Its `criterion accept` is the second user of `tty::confirm`, and adds a history entry with `via: "criterion-accept"`, which `show` prints as `criterion-accept` once `Via` has it, and as `unknown` until then. Its _Done when_ needs `criterion accept` refused outside a terminal. G1 means most move tests need items with a criterion. The deletion tests in `tests/show_and_delete.rs` start and submit items, so they need that too.
- **Slice 7** adds `started` and each check's last 20 lines of output, which `show` must print with the verification record. Long output should be indented under its check like the description, and escaped with its line breaks kept.
- **Slice 8** adds the verification commit and `refs/tracker/<ID>`. `show` prints both, and the warning for a done item says "verified by its gates at <commit>" and lists the ref among what's lost, as §2.8's example does. `delete` must remove the ref, and its _Done when_ checks that. Keep the record first. A ref that can't be removed should leave the item file in place, or be named in the error so it can be removed by hand. After the test is renamed, `show` must still print the saved output.
- **Slice 9** adds `approve`, `reject` and `show --diff`. `approve` and `reject` use `tty::confirm`, and `approve` re-runs the gates after the question, so it must re-read the item under the lock, through `Store::lock_unchanged` as `delete` does, which PLAN.md's integration risks also ask. `reject --reason` checks its reason before asking. `show` prints `holds` for an item in review, and the warning for an item in review could name its holds. `show --diff` adds a property to `show.schema.json`. The tests that put an item in review by editing its file, `in_a_terminal_typing_the_id_deletes_an_item_in_each_state` among them, can then use a real hold. T9 matches `needs-review` exactly, so slice 9 also decides slice 3 review 2's finding 4.
- **Slice 12** adds judge answers with their refs to the verification record, which `show` prints with each question's ref, answer and reason ([§2.3.2](DESIGN.md#232-when-a-judge-is-wrong)).
- **Slice 16**'s digest lists every line of `deleted.json` since that project's last ack, marked by `interactive`, yours included. It finds the project from the ID's key, since the record has no `project` field. A line that doesn't parse should be reported rather than skipped, unlike the counter tripwire. `digest ack` uses `tty::confirm`. `audit`'s "IDs used twice" should count `deleted.json` as well as `items/`, since an ID both deleted and on disk means the counter was reset. Slice 16 also brings `--help` and empty states up to slice 5's standard for `show` and `delete`.
- **Item files change shape again later.** `show --json` prints whatever the item holds, so each new field needs only what the earlier readmes list: a serde default and `skip_serializing_if`, its place in [§4.3](DESIGN.md#43-item-file)'s order, an entry in `item.schema.json`, and a fixtures folder. `show`'s text needs a line for it, or the field is left out of the text, though not of the JSON. A new label rule belongs in `check_new_labels`, so `--unlabel` can still remove a label written by hand.

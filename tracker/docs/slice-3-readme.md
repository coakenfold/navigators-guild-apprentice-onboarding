# Slice 3: Organise and filter

`tracker` is a command-line work tracker, written in Rust, that keeps one central list of work items across many projects and, in later slices, decides when an item is really done. This document covers **slice 3** of the 16 in [PLAN.md](PLAN.md): what it built, how it works, and how to test it. [Slice 1's readme](slice-1-readme.md) covers the store, `create` and `list`, and [slice 2's readme](slice-2-readme.md) covers the states, the moves between them and `history`.

After slice 3 you can **set an item's priority, kind and labels, edit its details, and filter a list sorted by priority**. The spec is [DESIGN.md](DESIGN.md). This slice implements the `kind`, `priority`, `labels`, `design` and `history` fields of [§2.1](DESIGN.md#21-items), the `create`, `edit` and `list` rows of [§3](DESIGN.md#3-interface) with its rules for which project a listing covers, and their place in [§4.3](DESIGN.md#43-item-file)'s item file.

| | |
| --- | --- |
| Status | Every _Done when_ check passes, and so does the test command. From [review 1](reviews/slice-3-review-1.md), two security items, three low-severity items, all five DESIGN.md questions and three test gaps are done ([below](#fixes-after-review-1)). **Its one fix before slice 4 is still open:** a label such as `ℂ`, which has no lower-case form, is stored as it is, so `create`, `edit` and `list --json` can print output that breaks `item.schema.json`. That and the rest are listed under [Still open](#still-open). The branch isn't merged into `main` yet. |
| Branch | `feat/tracker/slice-3` |
| Test command | `cargo fmt --check && cargo clippy -- -D warnings && cargo test`, run in `tracker/` |
| Tests | 135: 38 in `tests/create_and_list.rs`, 29 in `tests/organise.rs`, 27 in `tests/states.rs`, 11 in `tests/docs_links.rs`, 10 in `tests/item_files.rs`, 6 in `tests/output.rs`, 5 in `tests/fixtures.rs`, 4 in `tests/store_writes.rs`, 3 in `tests/schemas.rs` and 2 in `tests/item_ids.rs` |

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
- [Still open](#still-open)
- [Limits, and what later slices change](#limits-and-what-later-slices-change)

## What's in this slice

**Built:**

- `create` takes `--kind`, `--priority`, repeatable `--label` and `--design`, with §2.1's defaults: `change` and `medium`. Labels are trimmed, lowercased, kept once, and at most 50 characters.
- `edit <id>`, which changes the title, kind, priority, labels (`--label` and `--unlabel`), design and description. It adds one history entry with `via: "edit"`, `from` equal to `to`, and `fields` naming what changed. It's refused for done items, and for items in review unless it comes from an interactive terminal.
- An edit's history entry also records `kind_before` when it changes the kind, and `labels_removed` when it removes labels, so slice 16's digest can list the edits that weaken T1 and T9.
- `list` sorts by priority within each project, and marks each item's priority, kind and status. Its filters combine: `--project`, `--all`, `--status` (comma-separated), `--priority`, repeatable `--label`, `--kind` and `--by`.
- Plain `list` shows open, in-progress and review items. Inside a project root it shows only that project, and both its text and its JSON (`project`) say which it covered.
- The temporary file a write goes through is named with the process ID and opened only if it doesn't exist yet, so a planted symlink can't redirect a write. This closes an item slice 1 and slice 2 left open.
- `edit.schema.json`, and `labels`, `design` and the edit entry in `item.schema.json`.
- Item files written by this slice, in `tests/fixtures/slice-3/`.

**Not built yet:**

- `show` (slice 4). Until then, a description and an item's history can only be read in its file or in `--json` output.
- Nothing reaches review but a hand edit until slice 9's triggers. The refusal of an agent's edit in review is real, but only a hand-edited file can test it today.
- Nothing reads labels, kind or design yet. T9 reads `needs-review` in slice 9, T1 reads `capability` in slice 10, T5 fingerprints kind and design at `start` in slice 11, and G6 checks that `design` names a real section in slice 6.
- Slice 16's digest, which reads `kind_before` and `labels_removed`.
- `list`'s column heading row, colour, and empty states such as "No open items. Nice work!" (slice 5). So is skipping an item file that can't be parsed: today one bad file stops `list` with exit 3, whatever the filters.

## Try it

Always set `TRACKER_HOME` while developing. Without it, the tool uses your real store at `~/.tracker`.

```sh
cd guild-portfolio/tracker
cargo build

export DEMO=$(mktemp -d)
export TRACKER_HOME=$DEMO/store
export PATH="$PWD/target/debug:$PATH"
mkdir -p $DEMO/bookmarks $DEMO/notes && cd $DEMO/bookmarks

tracker init
tracker project add bm --name bookmarks --root $DEMO/bookmarks
tracker project add nt --name notes --root $DEMO/notes
tracker create "Search filters bookmarks by tag" --kind capability --priority high \
    --label Search --label search --label needs-review --design DESIGN.md#2.5
tracker create "Sort by date" --label ui
tracker create "Fix the crash on an empty import" --kind fix --priority high --label bug
tracker create "Tidy the README" --kind chore --priority low
tracker create "Sync notes" --project nt --priority high --label bug
tracker create "Export" --priority urgent                 # exit 1: names the priorities
tracker create "Export" --label a-label-much-too-long-to-print-in-every-single-list-row   # exit 1
tracker edit BM-2 --priority low --description "Newest first."
tracker edit BM-1 --kind change --unlabel needs-review
tracker edit bm-2 --priority low                          # nothing to change
tracker edit BM-2                                         # exit 1: no flag given
tracker edit BM-2 --label UI --unlabel ui                 # exit 1: both
tracker start BM-4
tracker submit BM-4
tracker edit BM-4 --title "Tidy the docs"                 # exit 1: done
tracker shelve BM-2 --reason "After search"
tracker list --all
tracker list --status open,shelved --priority low
tracker list --by unchecked
tracker list --label BUG --kind fix --all --json
tracker list --project NOPE                               # exit 1
tracker list
cat $TRACKER_HOME/items/BM-1.json
```

Output from `tracker init` on, with stderr included and the temporary folder shown as `$DEMO`:

```
Created the store at $DEMO/store.
Registered BM (bookmarks) at $DEMO/bookmarks.
Registered NT (notes) at $DEMO/notes.
Created BM-1: Search filters bookmarks by tag
Created BM-2: Sort by date
Created BM-3: Fix the crash on an empty import
Created BM-4: Tidy the README
Created NT-1: Sync notes
error: invalid value 'urgent' for '--priority <PRIORITY>'
  [possible values: high, medium, low]

For more information, try '--help'.
error: A label can be at most 50 characters, and "a-label-much-too-long-to-print-in-every-single-list-row" has 55. Use a word or two, and put the details in --description.
Edited BM-2: changed priority and description.
Edited BM-1: changed kind and labels.
BM-2 already has those details. Nothing changed.
error: Say what to change, with at least one of --title, --kind, --priority, --label, --unlabel, --design and --description, such as `tracker edit BM-2 --priority high`.
error: "ui" is given to both --label and --unlabel. Keep the one you mean.
BM-4: open → in-progress
BM-4: in-progress → done
Unchecked. No gates ran: this version of tracker has none yet, so nothing checked the work.
error: BM-4 is done, so it can't be edited: its verification record is about the item as it was verified. Reopen it first with `tracker reopen BM-4 --reason "…"`.
BM-2: open → shelved
BM  bookmarks
  BM-1  high  change  open  Search filters bookmarks by tag   [search]
  BM-3  high  fix     open  Fix the crash on an empty import  [bug]
NT  notes
  NT-1  high  change  open  Sync notes                        [bug]

Every project.
Open, in-progress and review items. `--status done,shelved` for the others.
BM  bookmarks
  BM-2  low  change  shelved  Sort by date  [ui]

BM only. `--all` for every project.
Filters: status open or shelved, priority low.
BM  bookmarks
  BM-4  low  chore  done  Tidy the README

BM only. `--all` for every project.
Filters: verified by unchecked (done items only).
{
  "project": null,
  "items": [
    {
      "id": "BM-3",
      "title": "Fix the crash on an empty import",
      "project": "BM",
      "kind": "fix",
      "status": "open",
      "priority": "high",
      "labels": [
        "bug"
      ],
      "created": "2026-11-17T03:01:53Z",
      "updated": "2026-11-17T03:01:53Z"
    }
  ]
}
error: There's no project NOPE. Registered projects: BM, NT.
BM  bookmarks
  BM-1  high  change  open  Search filters bookmarks by tag   [search]
  BM-3  high  fix     open  Fix the crash on an empty import  [bug]

BM only. `--all` for every project.
Open, in-progress and review items. `--status done,shelved` for the others.
{
  "id": "BM-1",
  "title": "Search filters bookmarks by tag",
  "project": "BM",
  "kind": "change",
  "status": "open",
  "priority": "high",
  "labels": [
    "search"
  ],
  "design": "DESIGN.md#2.5",
  "created": "2026-11-17T03:01:53Z",
  "updated": "2026-11-17T03:01:53Z",
  "history": [
    {
      "at": "2026-11-17T03:01:53Z",
      "from": "open",
      "to": "open",
      "via": "edit",
      "interactive": false,
      "fields": [
        "kind",
        "labels"
      ],
      "kind_before": "capability",
      "labels_removed": [
        "needs-review"
      ]
    }
  ]
}
```

Notes on the output:

- `Search` and `search` became one label, `search`. `edit bm-2` found BM-2, because an ID's key is put in capitals.
- The second `create "Export"` was refused, so no BM-5 exists. A refused `create` uses no ID.
- Every `list` but `--all` and `--json` ran inside BM's root, so it covered BM only. The plain `list` at the end hides BM-2 (shelved) and BM-4 (done).
- `--by unchecked` needs no `--status done`: `--by` lists done items on its own.
- BM-1's edit records `kind_before` and `labels_removed`, because it moved the item away from `capability` and took off `needs-review`, the two edits slice 16's digest will list.
- `interactive` is `false` because the demo ran from a script. Typed at a terminal, it would be `true`.
- On macOS, `mktemp -d` gives a path under `/var`, and `project add` stores the canonical path under `/private/var`. Both are shown as `$DEMO`.

## Commands

| Command | What it does |
| --- | --- |
| `tracker create "title" [--kind K] [--priority P] [--label L]… [--design REF] [--description TEXT] [--project KEY]` | Creates an open item. The kind defaults to `change` and the priority to `medium`. |
| `tracker edit <id> [--title T] [--kind K] [--priority P] [--label L]… [--unlabel L]… [--design REF] [--description TEXT]` | Changes the details given and adds one history entry naming the fields that changed. An edit that changes nothing writes nothing and exits 0. `--design ""` and `--description ""` remove the field. Refused for done items, and for items in review unless it comes from an interactive terminal. |
| `tracker list [--project KEY \| --all] [--status S,…] [--priority P] [--label L]… [--kind K] [--by auto\|human\|unchecked]` | Lists items grouped by project, sorted by priority from high to low, then by number. Without `--status` it shows open, in-progress and review items, or done items when `--by` is given. Inside a project root it covers that project unless `--project` or `--all` says otherwise. |

Every command takes `--json`. The kinds are `fix`, `change`, `capability` and `chore`, and the priorities `low`, `medium` and `high`. An unknown one is refused by clap, which names the allowed values.

**Input rules** (§2.1, [§4.4](DESIGN.md#44-handling-bad-data)):

- **Label:** trimmed and lowercased, then refused if it's empty or holds a line break, a control character or a mark that reorders text, as for titles. `create` and `edit --label` refuse one over 50 characters, counted as characters. `--unlabel` has no limit, so a long label written into a file by hand can still be removed. Repeats are kept once, in the order first given. The same label given to `--label` and `--unlabel` in one edit is refused.
- **`list --label`:** trimmed and lowercased like a label, with no length limit. Repeating it keeps items with every label named.
- **Design:** trimmed, and on one line. An empty one is left out by `create`, and removes the field in `edit`. The part before `#` names a file relative to the current folder, and is refused if it's empty or isn't inside the project's root, whether by an absolute path, `..`, or a symlink in a folder that exists. It's stored relative to the root, with `/` between folders, so `DESIGN.md#2.4` typed in `docs/` is stored as `docs/DESIGN.md#2.4`. Whether the file and section exist isn't checked until slice 6's G6.
- **Description:** stored as given. A blank one is left out by `create`, and removes the field in `edit`.
- **Title:** as in slice 1, for `edit --title` too.
- **`--status`:** one or more states, comma-separated or with the flag repeated. Spaces around each state are ignored, and a state named twice counts once. An unknown or empty state is refused, naming the states.
- **`--project`:** trimmed and put in capitals. It can't be combined with `--all`. `list --project` also takes a key that's no longer in `config.json` while items still use it, as `project list` counts them (§3).

**Exit codes** (§3), as this slice uses them:

| Code | Meaning |
| --- | --- |
| `0` | Success, including an edit that changes nothing and a list that matches nothing |
| `1` | Bad input: an unknown kind, priority, state or way of verifying, a bad label, title or design, an `edit` with no flag, a label given to both `--label` and `--unlabel`, a missing or malformed ID, an unknown project, `--project` with `--all`, an edit of a done item, or an edit of an item in review from outside a terminal |
| `2` | Not used yet. It's reserved for "refused by a gate" (slice 6) |
| `3` | The store or an item file can't be read or rewritten: for `edit`, a file it would lose fields from, such as one a newer tracker wrote, and a temporary file name that's already taken |

## How it works

### Labels

`check_label` in [src/main.rs](../src/main.rs) trims and lowercases a label with Rust's `to_lowercase`, and refuses an empty one or one that `check_printable` refuses. `check_new_labels` adds the 50-character limit (`MAX_LABEL`) for labels being added, and `add_labels` appends each label that isn't already there, so the order is the order labels were first given.

`item.schema.json` describes a stored label as one line, trimmed, with no character in Unicode's uppercase (`Lu`) or titlecase (`Lt`) categories. Lowercasing doesn't guarantee that: a few `Lu` characters, such as `ℂ` (U+2102), have no lower-case form, and come through unchanged. That's [review 1's finding 1](reviews/slice-3-review-1.md#fix-before-slice-4), still open ([below](#still-open)).

Labels are matched exactly. A label written into a file by hand with a capital, such as `Bug`, isn't found by `list --label bug` and can't be removed by `--unlabel bug` (review 1's finding 4, also open).

### An edit, step by step

`edit` in [src/main.rs](../src/main.rs):

1. Checks that the store exists and puts the ID's key in capitals.
2. Checks every value given: title, labels to add and remove, design and description. A label given to both `--label` and `--unlabel`, or no flag at all, is refused here. All of this happens before the lock, so bad input never waits for another command.
3. Takes the store's exclusive lock, and reads the item under it with `Store::load_item`.
4. Refuses a done item, pointing to `reopen`, and an item in review unless `tty::is_interactive()` says stdin is a terminal.
5. For `--design`, reads `config.json` and stores the reference relative to the item's project root, refusing one outside it.
6. Compares each value with the item's, in the item file's field order: title, kind, priority, labels, design, description. Each that differs is changed and its name added to `fields`. The labels are rebuilt as the old ones minus `--unlabel`, plus `--label`.
7. If `fields` is empty, stops there: the file isn't touched, and the output says nothing changed.
8. Otherwise sets `updated`, adds one history entry, and writes the item with `Store::update_item`, which refuses with exit 3 if the file holds something this version would lose.

Because the item is read and written under one lock, edits on one item happen one after another. Ten `edit --label` commands started at once all land, each with its own history entry (`concurrent_edits_all_land`).

`edit` reads `config.json` only for `--design`, which needs the project's root, so it works on an item whose project is no longer registered, as the moves do (§3: "a command that takes an item ID needs no project"), unless it's given a design reference.

### What an edit records

An edit adds one entry to `history`, in the same shape as a move ([slice 2's readme](slice-2-readme.md#history)):

| Field | |
| --- | --- |
| `at` | When, to the second. `updated` takes the same time. |
| `from`, `to` | Both the item's state, since an edit doesn't move it. |
| `via` | `edit` |
| `interactive` | Whether stdin was a terminal. |
| `fields` | The fields that changed, in the file's order, whatever order the flags came in. Never empty: an edit that changes nothing adds no entry. |
| `kind_before` | Only when the kind changed: the kind it replaced. |
| `labels_removed` | Only when labels were removed: the ones that were on the item and aren't now. `--unlabel` of a label the item didn't have isn't recorded. |

`kind_before` is written for every change of kind, and `labels_removed` for every removed label. [§2.6](DESIGN.md#26-digest-auditing-the-auto-verifications) has slice 16's digest pick out the two that weaken a trigger: removing `needs-review`, which switches off T9, and moving away from `capability`, which leaves T1 firing only on a new file. `item.schema.json` allows `fields`, `kind_before` and `labels_removed` only on an edit entry, requires `fields` there, and forbids `reason`.

### An edit to an item in review

[§2.2](DESIGN.md#22-states) calls a held item mine to decide, so an agent can't change it while I do: `edit` on an item in review exits 1 unless stdin is a terminal. It calls the same `tty::is_interactive()` that sets history's `interactive`, so the two can't disagree. It doesn't ask for the ID to be typed back, as slice 4's helper for human-only commands will: an edit isn't one of [§2.7](DESIGN.md#27-human-actions)'s human actions, only one that waits for the human to finish deciding. Like those actions, it's a convention, not security ([§7](DESIGN.md#7-honest-limits)): a command run under `script` gets a terminal.

### Listing

`list` in [src/main.rs](../src/main.rs) reads only, so it takes no lock.

1. **Scope.** `--all` covers every project. `--project` covers the key named, which must be in `config.json` or used by an item. Otherwise `config::project_for_dir` picks the project whose root holds the current folder, the deepest if roots are nested, and no project there means every project.
2. **States.** The states named in `--status`, or `done` alone when `--by` is given without `--status`, or else `ACTIVE`: open, in-progress and review.
3. **Filters.** An item is kept only if it matches every filter given: scope, states, priority, kind, every label named, and `--by`. `--by` checks that the item is done as well as what its `verification` says, because a file written by slice 2 before its review 1 can hold a record on a shelved item, like fixture `slice-2/XX-4.json`.
4. **Order.** By project key, then by priority (`Priority::rank`, high first), then by number.
5. **Text.** One `table` for every row, so the columns line up across projects, with each project's heading line (key and name, or the key alone if it isn't registered) before its first item. A row is the ID, priority, kind, status, title and labels in brackets. After a blank line come the scope ("BM only. `--all` for every project." or "Every project."), a hint about the hidden states when neither `--status` nor `--by` was given, and the filters used. An empty store says "No items yet.", and a list that matches nothing says "No items match.", so a filtered list is never mistaken for an empty store.
6. **JSON.** `{"project": <key or null>, "items": [...]}`, with the items in the same order.

`Store::load_items` reads every item file before the filters run. So, until slice 5 skips files it can't parse, one bad file in any project stops `list` with exit 3, even `list --project` for another project.

### The temporary file

`write_file` in [src/store.rs](../src/store.rs) used to write `XX-1.json.tmp`, and would follow a symlink planted at that name to a file outside the store. It now writes `XX-1.json.<pid>.tmp`, opened with `create_new`, so a file or symlink already at that name is refused with exit 3, and left where it is, rather than followed. The rest is as in [slice 1's readme](slice-1-readme.md#atomic-writes): flush, rename over the target, flush the folder. `Store::load_items` reads only `*.json` files, so it skips a leftover `.tmp` file.

## JSON output

| Command | Output | Schema |
| --- | --- | --- |
| `create` | `{"item": {…}}`, now with `labels` and `design` when they're set | [create.schema.json](../schemas/create.schema.json) |
| `edit` | `{"fields": [...], "item": {…}}`: the fields the edit changed, in the file's order, and the item as its file holds it. `fields` is empty when nothing changed, and then nothing was written | [edit.schema.json](../schemas/edit.schema.json) |
| `list` | `{"project": "BM" or null, "items": [...]}`: the project the list covered, or `null` for every project, and the items that match, in the list's order | [list.schema.json](../schemas/list.schema.json) |
| Any failure | `{"error", "exit_code"}`, as before | [error.schema.json](../schemas/error.schema.json) |

[item.schema.json](../schemas/item.schema.json) gains `labels` (at least one, unique, each trimmed, one line and with no capitals) and `design` (one line), and its history entry gains `via: "edit"`, `fields`, `kind_before` and `labels_removed`, with an `if`/`then`/`else` that allows the last three only on an edit and requires `fields` there.

`list`'s `project` is new and required. An agent that read `items` before isn't affected. Later slices add item fields such as `written` and `criteria` (slice 6), which `create`, `edit` and `list` will then print, without renaming or removing any field here. A new field `edit` can change must be added to the `fields` enum in both `edit.schema.json` and `item.schema.json`.

## Code tour

| File | What changed |
| --- | --- |
| [src/main.rs](../src/main.rs) | `CreateArgs`, `EditArgs` and `ListArgs`, with `kinds`, `priorities` and `state_name` as their value parsers. `create` sets kind, priority, labels and design. `edit`, with `check_label`, `check_labels`, `check_new_labels`, `MAX_LABEL`, `add_labels`, `check_design`, and `resolve_design` with `follow_symlinks`, which store a design reference relative to the root. `list`, with `ACTIVE`. `apply` writes the new entry fields as empty for a move. |
| [src/item.rs](../src/item.rs) | `Item` gains `labels` and `design`, between `priority` and `created`. `HistoryEntry` gains `fields`, `kind_before` and `labels_removed`, each with a serde default and `skip_serializing_if`. `Via::Edit`. `Kind`, `Priority` and `VerifiedBy` gain `ALL`, `as_str` and `parse`, which give clap its allowed values, and `Priority::rank` gives the sort order. |
| [src/states.rs](../src/states.rs) | `join` ("a, b or c") is now public, for `edit`'s and `list`'s messages. |
| [src/store.rs](../src/store.rs) | `write_file` names its temporary file with the process ID and opens it with `create_new`. |
| [schemas/edit.schema.json](../schemas/edit.schema.json) | New: `edit --json`. |
| [schemas/item.schema.json](../schemas/item.schema.json), [schemas/list.schema.json](../schemas/list.schema.json) | As described under [JSON output](#json-output). |
| [tests/organise.rs](../tests/organise.rs) | Every test for `create`'s new flags, `edit` and `list`. `env_with_every_state` builds a store with one XX item in each state, putting XX-3 in review by hand. `edit` checks that the file on disk holds the item the command printed. |
| [tests/common/mod.rs](../tests/common/mod.rs) | `Env::in_terminal`, and `schema_for` maps `edit` to its schema. |
| [tests/fixtures/slice-3/](../tests/fixtures/slice-3/), [tests/fixtures.rs](../tests/fixtures.rs) | Three item files written by this slice's binary, and `slice_3_wrote_three_fixtures_with_edits`. `the_binary_lists_every_fixture_as_it_is_on_disk` now lists every state and finds each item by ID, since `list` hides done and shelved items and sorts by priority. |
| [tests/store_writes.rs](../tests/store_writes.rs) | `a_planted_tmp_symlink_is_not_written_through`, `a_symlink_planted_at_this_process_tmp_name_is_refused`. |
| [tests/docs_links.rs](../tests/docs_links.rs) | Checks that each heading anchor in a link names a heading in its target, slugged as GitHub does, including links within one doc. Skips links in code. Checks a review's paths and anchors as of the last commit that changed it, so a later change to DESIGN.md can't break an old review. |
| [tests/item_files.rs](../tests/item_files.rs), [tests/states.rs](../tests/states.rs), [tests/schemas.rs](../tests/schemas.rs), [tests/output.rs](../tests/output.rs) | Tests that used `labels` as a field this version didn't know now use `written`, and ones that used an `edit` entry as an unknown command now use `criterion-accept`. `list` tests allow for the new sort and the hidden states. `a_wrong_output_is_caught` covers labels, design, the edit entry and `list`'s `project`. |

**To add a field `edit` can change:** add the flag to `EditArgs`, check its value in `edit` before the lock, compare it in the file's field order and push its name onto `fields`, and add the name to the `fields` enum in `edit.schema.json` and in `item.schema.json`'s history entry. **To add a `list` filter:** add it to `ListArgs`, add a condition to the `items.retain` call in `list`, and name it in the `Filters:` line.

## Testing

```sh
cd guild-portfolio/tracker
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

All three pass on the branch, and so does `cargo clippy --all-targets -- -D warnings`. To run this slice's tests alone: `cargo test --test organise`.

**The harness.** [tests/common/mod.rs](../tests/common/mod.rs) gains `Env::in_terminal(args)`, which runs `tracker` under `script` on a pseudo-terminal, as if typed, with no pty crate (PLAN.md). Its options differ between macOS and BSD (`script -q /dev/null …`) and util-linux (`script -qec "…" /dev/null`). It returns `None` where `script` isn't installed, and the test skips. Slice 2's `a_move_from_a_terminal_records_interactive_true` now uses it too.

### Coverage of the _Done when_ checks

Every _Done when_ check in [PLAN.md's slice 3](PLAN.md#slice-3-organise-and-filter) has a test in [tests/organise.rs](../tests/organise.rs), unless noted:

| _Done when_ | Tests |
| --- | --- |
| `list` shows high before medium before low | `list_shows_high_before_medium_before_low_within_each_project`, which also checks grouping by project and that an edit re-sorts |
| An unknown priority or kind exits 1 and names the allowed values | `an_unknown_priority_or_kind_exits_1_and_names_the_allowed_values`, for `create`, `edit` and `list` |
| `--label Bug --label bug` is stored once, as `bug` | `labels_are_lowercased_and_stored_once`, which also covers `" BUG "`, a non-ASCII label, and empty and multi-line labels |
| `edit --priority low --unlabel bug` changes the item and adds one history entry | `edit_priority_and_unlabel_changes_the_item_and_adds_one_history_entry` |
| `edit` on a done item exits 1 | `edit_works_in_every_state_but_done_and_review`, which also checks open, in-progress, shelved, reopened and review items |
| `list --status open --priority high --label bug` shows only items that match all three | `list_filters_combine` |
| `--status done` shows only done items, and `--status open,shelved` exactly those two states | `status_shows_exactly_the_states_named` |
| Plain `list` hides shelved items | `plain_list_hides_done_and_shelved_items`, and `list_and_project_list_show_the_new_states` in `tests/states.rs` |
| Inside XX's root, `list` shows only XX's items with `"project": "XX"`. `--all` there, or plain `list` outside every root, shows every project with `"project": null` | `inside_a_project_root_list_shows_only_that_project`, `the_scope_is_reported_even_when_nothing_matches` |
| _Prompt:_ `create` takes `--kind`, `--priority` and `--label` with §2.1's defaults | `create_defaults_to_a_medium_change_with_no_labels_or_design`, `create_takes_kind_priority_labels_and_design` |
| _Prompt:_ `edit` covers priority, labels, title, kind, design and description, with one entry, `via: "edit"`, `from` equal to `to`, and `fields` | `edit_changes_every_detail_and_names_each_field`, and the `edit --priority low --unlabel bug` test above |
| _Prompt:_ `list` marks priority and kind | `the_text_list_marks_priority_kind_and_labels` |
| _Prompt:_ `--kind` and `--project` filters, and `--project` with `--all` | `list_filters_combine`, `inside_a_project_root_list_shows_only_that_project`, `list_refuses_an_unknown_project_and_project_with_all` |
| _Prompt:_ `--by` checks that the item is done as well as its `verification` | `by_keeps_done_items_verified_that_way`, `by_ignores_a_record_left_on_a_shelved_item_by_slice_2` |
| _Integration risk:_ files saved by slice 3 load and come back byte for byte | `tests/fixtures.rs`: `slice_3_wrote_three_fixtures_with_edits`, `every_fixture_loads_and_comes_back_byte_for_byte`, `the_binary_lists_every_fixture_as_it_is_on_disk` |

Further tests cover the edges: `an_edit_that_changes_nothing_writes_nothing`, `edit_refuses_bad_input_and_changes_nothing`, `edit_finds_an_id_typed_in_lower_case`, `a_move_after_an_edit_still_works`, `two_label_filters_keep_items_with_both`, `project_lists_the_items_of_a_project_no_longer_in_config_json`, and in `tests/item_files.rs`, `a_move_refuses_an_item_whose_history_has_a_command_it_doesnt_know`, which now includes `edit`. The tests added after review 1 are listed with their fixes [below](#fixes-after-review-1).

**Tests read the JSON, not the wording**, except where the text is the subject: `the_text_list_marks_priority_kind_and_labels` checks the rows line for line, and the scope and filter tests look for "XX only.", "Every project." and "Filters:" phrases. Refusals with no field of their own are checked for a word, such as "review" and "terminal" for an edit in review, or "50" and "51" for a long label.

**`edit` checks the file against its output.** The `edit` helper in `tests/organise.rs` asserts that the file on disk equals the item the command printed, so every edit test also checks that `--json` reports what was written.

**Seen failing.** The first pass landed as one commit with its tests (`1296661`), so there's no record of its tests failing before the code. Review 1's [outcome](reviews/slice-3-review-1.md#outcome) records that each test added with a fix was seen failing first, and that `concurrent_edits_all_land` and `an_edit_from_a_terminal_records_interactive_true`, which cover behaviour that already worked, were each checked by breaking the code on purpose.

**The fixtures.** `tests/fixtures/slice-3/` holds three files written by this slice's binary:

- `XX-6`: a `capability` at `high`, with the labels `feature` and `needs-review`, a design reference and one edit entry.
- `XX-7`: edited while in progress, so the entry has `from` and `to` both `in-progress` and `fields` of `title`, `kind` and `labels`, then shelved.
- `XX-8`: written after review 1, with an edit entry that records `kind_before: "capability"` and `labels_removed: ["needs-review"]`.

**Never edit a fixture.** [tests/fixtures.rs](../tests/fixtures.rs) checks that each one, from every slice, loads, comes back byte for byte, matches `item.schema.json`, loses nothing on a rewrite, and lists exactly as it is on disk.

### Checks done by hand

Review 1 checked two things by hand that are now automated: ten concurrent edits on one item (`concurrent_edits_all_land`), and `edit` on a file with a history command this version doesn't know (`a_move_refuses_an_item_whose_history_has_a_command_it_doesnt_know`). The Try it demo above was run against the debug binary, and its output is pasted unchanged apart from the temporary folder. Finding 1 was reproduced on the branch the same way: `tracker create a --project XX --label 'ℂ' --json` exits 0 and stores `"labels": ["ℂ"]`.

## Decisions worth knowing

- **An edit that changes nothing writes nothing.** It exits 0, adds no history entry, leaves the file byte for byte, and returns `"fields": []`. §3 says so since review 1.
- **`fields` follows the file's order**, not the order of the flags, so two edits that change the same fields read the same.
- **`kind_before` and `labels_removed` are recorded for every such edit**, not only the two that weaken a trigger. The entry stays a plain record of what changed, and slice 16's digest picks out removals of `needs-review` and moves away from `capability`.
- **An agent's edit to an item in review is refused**, and mine from a terminal goes through. §2.2, §3 and `edit --help` say so since review 1. It needs only a terminal, not the typed ID that slice 4's helper will ask for.
- **A done item can't be edited.** Its verification record describes the item as it was verified. The refusal points to `reopen`.
- **The label limit applies only to input.** A label already in a file, however long, still loads and lists, and `--unlabel` can remove it. The 50 matches the reason's rule of a short limit for text printed among other lines. §2.1 says so.
- **Labels are compared exactly**, so a capital written into a file by hand defeats `list --label` and `--unlabel` (review 1's finding 4, open).
- **`--by` without `--status` lists done items**, since only done items have been verified, and with `--status` the two combine, so `--status open --by auto` lists nothing. §3 says so since review 1.
- **Repeated `--label` filters mean every label named**, like the other filters, which all combine. §3's `list` row shows `[--label]…`.
- **`list --project` takes an unregistered key while items still use it**, so `list` and `project list` agree. §3 says so since review 1.
- **`list` reads every item before filtering**, and takes no lock. One bad file anywhere stops it until slice 5.
- **`edit` doesn't need the item's project to be registered**, like the moves. Slice 2's still-open item 7 asks which commands should keep working without one.
- **`env_with_every_state` puts an item in review by editing its file**, since nothing reaches review until slice 9. Its tests of an item in review will need a real hold then.
- **clap refuses an unknown kind, priority, state or `--by` value** through `PossibleValuesParser`, which names the allowed values. Slice 1's error path makes that exit 1, not clap's 2.

## Fixes after review 1

[slice-3-review-1.md](reviews/slice-3-review-1.md) reviewed the branch at `9351296`, where the slice was the single commit `1296661`. It found one real bug (finding 1), a design gap that `edit` opens (finding 2), the symlinked temporary file still open from slices 1 and 2 (finding 3), five low-severity items, five decisions DESIGN.md didn't record, six test gaps, and four process points. Each fix below is in its own commit, and each new test that guards a change was seen failing first. `eb9c922` saves the review and its outcome.

Two commits between the first pass and the review don't change the slice's code: `8216c23` adds the heading-anchor checks to `tests/docs_links.rs`, and `9351296` adds the `slice-readme` skill outside `tracker/`.

### Low severity

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| 6. Labels have no length limit | `47d4671` | `create` and `edit --label` refuse a label over 50 characters. `--unlabel` has no limit. §2.1 says so. Whether titles need a limit is now in slice 5's prompt (`5634992`). | `a_label_is_at_most_50_characters` |
| 7. The list filters are stricter than they need to be | `9daacb3`, `8f85fa4` | `--status` trims each state. `list --label` repeats, keeping items with every label named, and §3 says so. | `status_shows_exactly_the_states_named`, `two_label_filters_keep_items_with_both` |
| 8. `list` text has no column headings | `5634992` | Moved to slice 5's prompt and _Done when_ in PLAN.md. Nothing in the code yet. | None yet. Slice 5's _Done when_ checks it. |

### Security items

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| 2. An agent can quietly take off `needs-review`, or move an item away from `capability`, and nothing reports it | `f2f07de` | The review's option (b), widened. An edit's history entry records `kind_before` and `labels_removed`, and DESIGN.md §2.1, §2.6, §3 and [§6](DESIGN.md#6-what-im-trusting-and-what-checks-it), and PLAN.md's slice 16, have the digest list every edit that removes `needs-review` or changes kind away from `capability`. It lists every such edit marked by whether it came from a terminal, not only an agent's, because §2.6 lists my own actions too: `script` can fake a terminal. Adds fixture `slice-3/XX-8.json`. | `an_edit_records_the_labels_it_removed_and_the_kind_it_replaced`, `slice_3_wrote_three_fixtures_with_edits` |
| 3. The symlinked `.tmp` file, still open from slices 1 and 2 | `4a40b00` | The temporary file is named `<file>.<pid>.tmp` and opened with `create_new`, so a planted symlink is refused rather than followed. | `a_planted_tmp_symlink_is_not_written_through`, `a_symlink_planted_at_this_process_tmp_name_is_refused` |

### Design decisions

The review listed five things the code does that DESIGN.md §3 didn't record. Each was settled and written down.

| Question | Decision | Commits |
| --- | --- | --- |
| 1. Does an edit that changes nothing add a history entry? | No. It writes nothing and returns `"fields": []`. | `e7e3071` |
| 2. What does `--by` without `--status` list? | Done items. | `40b2979` |
| 3. How is `design` or `description` cleared? | `edit --design ""` or `--description ""` removes the field. | `b238b17` |
| 4. Does `list --project` take a key that's no longer in `config.json`? | Yes, while items still use it, as `project list` counts them. | `d30b2ba` |
| 5. Can an item in review be edited? | Only from an interactive terminal, so an agent can't change a held item while I decide it. §2.2, §3 and `edit --help` say so. | `6fb2e70`, tested by `edit_works_in_every_state_but_done_and_review` and `i_can_edit_an_item_in_review_from_a_terminal` |

### Test gaps

| Gap | Commit | Test |
| --- | --- | --- |
| 2. `edit` on an item in review | `6fb2e70` | `edit_works_in_every_state_but_done_and_review` now uses `env_with_every_state` and checks that a piped edit is refused, and `i_can_edit_an_item_in_review_from_a_terminal` that one from a terminal works |
| 3. Concurrent edits | `4321762` | `concurrent_edits_all_land`: ten `edit --label` commands at once all land, with ten history entries and no `.tmp` file left. Slice 2's concurrent moves are still untested. |
| 4. An edit from a terminal records `interactive: true` | `213c9e0`, `e8b3ac4` | `213c9e0` moves the `script` runner into `Env::in_terminal`, then `an_edit_from_a_terminal_records_interactive_true` uses it |

### Process

- **No `slice-3-readme.md`:** this document. It records the decisions above, and carries slice 2's open items forward under [Still open](#still-open).
- The other three process points are still open.

A few consequences:

- **An agent can no longer edit an item in review.** Today that only matters for a hand-edited file. From slice 9, an agent that wants to change a held item has to wait for `approve` or `reject`.
- **A new label over 50 characters is refused**, with exit 1. Labels already stored keep working.
- **A leftover `<file>.<pid>.tmp` can block a write.** If a process with the same ID left one behind, the write exits 3 and names the file to remove. Process IDs are rarely reused that soon, and a write that fails removes its own temporary file.

## Still open

Where review 1 and slice 2's readme raised the same thing, it's listed once.

**Fix before slice 4** (review 1)

- **1. Labels with no lower-case form break the schema.** `check_label` stores `label.trim().to_lowercase()`, which leaves characters such as `ℂ` (U+2102), `ϒ` (U+03D2) and `𝐀` unchanged, though `item.schema.json` forbids `Lu` and `Lt` characters. `create … --label 'ℂ' --json` exits 0, and from then on `create`, `edit` and `list --json` print output that breaks its schema. The review suggests refusing such a label after lowercasing, and adding `ℂ` to `labels_are_lowercased_and_stored_once` (test gap 1).

**Low severity** (review 1)

- **4. Hand-edited labels aren't matched case-insensitively.** A file holding `"Bug"` isn't found by `list --label bug`, can't be changed by `--unlabel bug`, and breaks the schema in `list --json`. A hand-written `["bug", "bug"]` keeps both through `edit`, since the rebuild filters but doesn't de-duplicate.
- **5. Invisible characters are accepted in labels and titles.** `check_printable` doesn't refuse zero-width characters (U+200B–U+200D, U+2060, U+FEFF), so a label can look exactly like `needs-review` and not be it, and T9 won't fire on it.

**Test gaps** (review 1)

- **1.** `ℂ` among the refused labels, with finding 1.
- **5.** Two projects in the text list. `the_text_list_marks_priority_kind_and_labels` has one project, so nothing checks the heading line before each project or that the columns line up across them.
- **6.** Hand-edited labels, once finding 4 is decided.

**Process** (review 1)

- **The `unchecked` note is still in the present tense.** `states::UNCHECKED_NOTE` reads "No gates ran: this version of tracker has none yet…" and stays in item files after slice 6 adds gates. It's cheapest to change before self-tracking starts after slice 6, while only the test fixtures hold the old text. Also slice 2 review 2's item 9.
- **PLAN.md marks slice 3 `done` in its first-pass commit**, before any review, as slice 2's did. So the Status column means "built", not "built and reviewed". PLAN.md's intro should say so, or the status should change only after the review's fixes.
- **The slice landed as one commit** (`1296661`, 1518 lines added across 16 files), as slice 2 did. That's in the history and stays.

**Carried over from slice 2** ([slice 2's readme](slice-2-readme.md#still-open)), untouched by this slice:

- **A hand-edited timestamp is blamed on "a newer tracker"** (slice 2 review 2, item 4). The message should name the field that would change.
- **`status <id> closed` is refused** (slice 2 review 2, item 5), though §1 calls done the course's "closed" state.
- **Commands on an item whose project isn't in `config.json`** (slice 2 review 2, item 7). Moves and now `edit` succeed. Slices 7 and 8 need the project's root, so which commands keep working has to be decided by then, and no test pins today's behaviour.
- **`error.schema.json`'s `allowed[].via` enum lists only today's four moves** (slice 2 review 2, item 3). Slices 6 and 9 must extend it in the same commit as their code. `edit` isn't a move, so it doesn't appear there.
- **Concurrent moves on one item aren't tested**, unlike concurrent edits now.
- **The text output of `shelve` and `reopen` isn't tested.**

## Limits, and what later slices change

- **Slice 4**'s `show` prints `labels`, `design`, `description` and every history entry, including an edit's `fields`, `kind_before` and `labels_removed`, escaping their control characters. Its helper for human-only commands must not be applied to `edit` in review, which needs a terminal but not a typed ID. `delete` must work on items with any of slice 3's fields.
- **Slice 5** adds `list`'s column heading row (now in its _Done when_), lined up with the rows across projects, and empty states such as "No open items. Nice work!" in place of today's "No items match.", which must still say what the list covered. Skipping unparseable files changes `list`, which today stops at the first bad file in any project. It also decides whether titles need a length limit, and is the natural place for review 1's finding 5, invisible characters.
- **Slice 6** adds `written` and `criteria`, which go between `updated` and `verification` in §4.3's order. Several tests use `written` as the field "a newer tracker added": `update_item_refuses_a_file_it_would_lose_fields_from` in `tests/item_files.rs`, `a_move_refuses_a_file_it_would_lose_fields_from` in `tests/states.rs`, and `a_wrong_output_is_caught` in `tests/schemas.rs`. And `add_newer_entry` in `tests/item_files.rs` uses `criterion-accept` as the command this version doesn't know. Slice 6 must move each to a field and a command it still doesn't know. G1 refuses `start` without criteria, so `env_with_every_state`, which starts XX-2 and XX-4, needs items with one. G6 reads the `design` that `create` and `edit` set, which they check only for a file inside the root, so G6 must check again after following symlinks.
- **Slice 9** makes review reachable through the triggers, and T9 holds an item labelled `needs-review`, matched exactly after lowercasing. The tests that put XX-3 in review by hand can then use a real hold. An agent's edit to a held item stays refused.
- **Slice 10**'s T1 fires for `kind: capability`, which `create --kind` and `edit --kind` set.
- **Slice 11**'s T5 fingerprints kind and design at `start`, so `edit --kind` or `--design` after start fires T5 (§2.5). An edit of title, priority, labels or description must not change the fingerprint.
- **Slice 16**'s digest lists every edit whose entry has `labels_removed` containing `needs-review`, or `kind_before: "capability"` with a new kind, marked by `interactive`, and not an edit that only adds a label. It reads these fields from files written since `f2f07de`: an edit made by the first pass (`1296661`) has neither, as fixtures `XX-6` and `XX-7` show. `audit` could also report labels that aren't in their stored form (review 1's finding 4). Slice 16 brings `--help` and empty states up to slice 5's standard for every command added since.
- **Item files change shape again later.** Each new field needs a serde default and `skip_serializing_if`, its place in §4.3's order, an entry in `item.schema.json`, and a `tests/fixtures/slice-<n>/` folder of files written by that slice's binary. A new field `edit` can change also goes in both `fields` enums.

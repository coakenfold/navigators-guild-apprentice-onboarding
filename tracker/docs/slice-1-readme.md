# Slice 1: Create and list

`tracker` is a command-line work tracker, written in Rust, that keeps one central list of work items across many projects and, in later slices, decides when an item is really done. This document covers **slice 1**, the first of the 16 slices in [PLAN.md](../PLAN.md): what it built, how it works, and how to test it.

After slice 1 you can **register projects, create items in them, and list everything from one store**. The spec is [DESIGN.md](../DESIGN.md). This slice implements §3 (interface) for `init`, `project add`, `project list`, `create` and `list`, and §4.1–4.3 (store layout, config and item file).

| | |
| --- | --- |
| Status | Done. Every _Done when_ check passes, and bugs 1–8 from [review 1](slice-1-review-1.md) are fixed ([below](#fixes-after-review-1)). |
| Branch | `feat/tracker/slice-1` |
| Test command | `cargo fmt --check && cargo clippy -- -D warnings && cargo test`, run in `tracker/` |
| Tests | 32: 28 in `tests/create_and_list.rs`, 2 in `tests/docs_links.rs` and 2 in `tests/store_writes.rs` |

## Contents

- [What's in this slice](#whats-in-this-slice)
- [Try it](#try-it)
- [Commands](#commands)
- [The store](#the-store)
- [How it works](#how-it-works)
- [JSON output](#json-output)
- [Code tour](#code-tour)
- [Testing](#testing)
- [Decisions worth knowing](#decisions-worth-knowing)
- [Fixes after review 1](#fixes-after-review-1)
- [Limits, and what later slices change](#limits-and-what-later-slices-change)

## What's in this slice

**Built:**

- A Rust crate at `guild-portfolio/tracker/`, made with `cargo init --vcs none`. It's an ordinary folder in the guild-portfolio repo, not a nested repo. `tracker/.gitignore` ignores `/target`.
- `DESIGN.md`, `PLAN.md`, `walkthrough.html` and `judge-eval/` were moved here with `git mv` from `chapters/02-the-methodology/02-tracking-your-work/`, so their history follows them and their relative links still work.
- Five commands: `init`, `project add`, `project list`, `create` and `list`. Every one takes `--json`.
- A central store, with one JSON file per item and IDs handed out from `counters.json` under an exclusive file lock.
- A default project: `create` picks the project whose root contains the current folder.
- An integration test harness (assert_cmd and tempfile) in which every test gets its own temporary `TRACKER_HOME`.

**Not built yet:** states and moves (slice 2), kind, priority, labels, `edit` and filters (slice 3), `show` and `delete` (slice 4), skipping corrupt files, colour and help polish (slice 5), and everything about criteria, gates, triggers and judges (slice 6 on).

## Try it

Always set `TRACKER_HOME` while developing. Without it, the tool uses your real store at `~/.tracker`. It must be an absolute path; a relative one is refused.

```sh
cd guild-portfolio/tracker
cargo build

export DEMO=$(mktemp -d)
export TRACKER_HOME=$DEMO/store
export PATH="$PWD/target/debug:$PATH"
mkdir -p $DEMO/bookmarks/src $DEMO/elsewhere

tracker init
tracker project add bm --name bookmarks --root $DEMO/bookmarks --design DESIGN.md

cd $DEMO/bookmarks/src
tracker create "Search filters bookmarks by tag" --description "Tag chips above the list."

cd $DEMO/elsewhere
tracker create "Lost"                      # exit 1: not inside any project root
tracker create "Sort by date" --project BM

tracker list
tracker project list
tracker list --json
cat $TRACKER_HOME/items/BM-1.json
```

Output, with the temporary folder shown as `$DEMO`:

```
Created the store at $DEMO/store.
Registered BM (bookmarks) at $DEMO/bookmarks.
Created BM-1: Search filters bookmarks by tag
error: The current folder isn't inside any project's root. Run this from inside one, or pass --project with one of: BM.
Created BM-2: Sort by date

BM  bookmarks
  BM-1  open  Search filters bookmarks by tag
  BM-2  open  Sort by date

KEY  NAME       OPEN  IN-PROGRESS  REVIEW  DONE  SHELVED  ROOT
BM   bookmarks  2     0            0       0     0        $DEMO/bookmarks
```

`cargo run -- <args>` works too, but its build messages go to stderr alongside the tool's own.

## Commands

| Command | What it does |
| --- | --- |
| `tracker init` | Creates whatever parts of the store are missing, so it also repairs a store that has lost `counters.json` or `items/`. Running it on a complete store changes nothing and exits 0. |
| `tracker project add KEY --name NAME --root PATH [--design FILE]` | Registers a project. The key is uppercased (`bm` becomes `BM`). The root must be an existing folder that no other project uses, and is stored as an absolute, canonical path. |
| `tracker project list` | Lists projects with their item counts for all five states. |
| `tracker create "title" [--description TEXT] [--project KEY]` | Creates an open item and prints its ID. Without `--project`, it uses the project whose root contains the current folder. |
| `tracker list` | Lists every item, grouped by project and sorted by number. |

`--json` is a global flag, so `tracker list --json` and `tracker --json list` both work.

**Input rules** (§2.1, §4.4):

- **Title:** surrounding whitespace is trimmed. An empty or whitespace-only title exits 1, and so does a title with a line break.
- **Description:** stored as given. An empty or whitespace-only one is left out.
- **Project key:** up to 10 ASCII letters and digits, starting with a letter. A dash isn't allowed, since the dash separates the key from the number in `BM-12`. `--project` is also uppercased before lookup.
- **Project name:** can't be empty.
- **Root:** must exist and be a folder. A relative path is resolved against the current folder. Two projects can't share a root, even when it's spelled differently (`xx/../xx`), but one root can be nested inside another.
- **`TRACKER_HOME`:** must be absolute. A relative path would name a different store in every folder, so it's refused with exit 1.

**Exit codes** (§3):

| Code | Meaning in this slice |
| --- | --- |
| `0` | Success, including `init` on an existing store, and output cut short because its reader stopped early (`tracker list \| head -1`). |
| `1` | Bad input: a bad title, key, name or root, a root another project already uses, an unknown project, no project for this folder, a duplicate key, a relative `TRACKER_HOME`, or a bad argument, including one that isn't valid UTF-8 (see below). Also a store that doesn't exist yet, or is missing `counters.json` or `items/`: both say "Run `tracker init`." |
| `2` | Not used yet. It's reserved for "refused by a gate" (slice 6). |
| `3` | The store can't be read or written: one that exists but can't be read (such as `chmod 000`), an unparseable `config.json`, `counters.json` or item file, a lock that can't be taken, a counter at or below a number already on disk, or output that can't be written. |

clap normally exits with 2 when it can't parse the arguments. Here 2 means a gate refused, so bad arguments exit 1 instead. `--help` and `--version` exit 0.

## The store

The store is `$TRACKER_HOME`, which must be an absolute path, or `~/.tracker` when that's unset or empty (§4.1). `init` creates:

```
$TRACKER_HOME/
├── config.json     # registered projects
├── counters.json   # the next item number for each project
├── lock            # an empty file; commands lock it while they write
└── items/
    └── BM-1.json   # one file per item
```

`digests.json` and `judge/` from §4.1 aren't created yet. The slices that use them add them.

Every other command checks for `config.json`, `counters.json` and `items/` before doing anything ([store.rs](../src/store.rs) `require_exists`). With no `config.json` there's no store, and the message is "There's no tracker store at … Run `tracker init`." If only `counters.json` or `items/` is missing, the message names it and says to run `init` to put it back. Both exit 1. A store that's there but can't be read exits 3 instead, so a permissions problem isn't mistaken for a missing store.

### `config.json`

```json
{
  "projects": {
    "BM": {
      "name": "bookmarks",
      "root": "/Users/me/code/bookmarks",
      "design": "DESIGN.md"
    }
  }
}
```

So far the tool reads only `name`, `root` and `design`. Any other setting from §4.2 (`checks`, `review`, `judge`, …) is ignored when reading, and **kept** when `project add` rewrites the file. A root written by hand can start with `~/`, as in §4.2's example, and the tool expands it.

### `counters.json`

```json
{
  "BM": 3
}
```

Each value is the **next** number to hand out. A project with no entry starts at 1. IDs come only from this file, never from the item files that happen to exist, so a deleted item's ID is never reused (§4.4).

The item files are a tripwire, though. If a project's counter is at or below the highest number among its item files, `counters.json` was lost or reset, and its next number could be a deleted item's ID, whose scoped commits would then count as the new item's work (§2.8). So `create` refuses with exit 3 and says what to set the counter to. Each project is checked separately: after the file is lost, every project with items needs its counter set again. A project that lost its counter and has no item files left can't be checked, because nothing is left on disk to compare against.

### Item files

`items/BM-1.json`:

```json
{
  "id": "BM-1",
  "title": "Search filters bookmarks by tag",
  "project": "BM",
  "kind": "change",
  "status": "open",
  "priority": "medium",
  "created": "2026-11-15T22:01:56Z",
  "updated": "2026-11-15T22:01:56Z",
  "description": "Tag chips above the list."
}
```

- The file is pretty-printed with two-space indents and a final newline, and the fields are always in this order (§4.3).
- `kind` is always `change` and `priority` always `medium` for now. Slice 3 adds the flags that set them.
- `status` can hold any of the five §2.2 states: `open`, `in-progress`, `review`, `done` or `shelved`. `create` makes only `open` items, but a file with any of the five loads, and `project list` counts it.
- Timestamps are RFC 3339 in UTC, to the second.
- `description` is left out when there isn't one.

Slice 1 writes only the fields it sets. The later fields in §4.3 (`labels`, `design`, `written`, `criteria`, `started`, `verification`, `history`) are added by the slices that need them.

## How it works

### Handing out IDs

`create` does this:

1. Checks the title. A bad title fails here, before any ID is used.
2. Takes an **exclusive lock** on `$TRACKER_HOME/lock` with `std::fs::File::lock`, waiting while another command holds it.
3. Reads `config.json` and works out the project.
4. Reads `counters.json`, takes the number `n`, and checks that `n` is above the highest `KEY-<number>.json` in `items/`. If it isn't, the counter was lost or reset, so it exits 3 rather than overwrite a file or reuse a deleted item's ID.
5. Writes `counters.json` with `n + 1`, **then** writes the item file. With this order, a crash between the two writes can only skip an ID, never hand one out twice.
6. Releases the lock when the file handle is dropped. If the process dies, the operating system releases it, so the lock can never go stale.

`project add` and `init` take the same lock. `list` and `project list` only read, so they don't lock.

### Atomic writes

Every write goes through [store.rs](../src/store.rs) `write_file`:

1. Writes `<file>.tmp` beside the target and flushes it to disk (`sync_all`), so a crash can't keep the rename and lose the data, leaving an empty file in place of the old one.
2. Renames it over the target, so a reader never sees a half-written file.
3. Flushes the folder too, so the rename itself survives a crash. This step is best effort, because not every system lets a folder be flushed.

If writing or renaming fails, the `.tmp` file is removed and the command exits 3, so no temporary file is left behind. `list` reads only `*.json` files in `items/`, so it would skip a leftover `.tmp` file anyway.

The flushes make writes slower, which is noticeable only in the tests: `tests/create_and_list.rs` takes about 2 seconds instead of 1.

### Printing

Output goes through `print_out` in [main.rs](../src/main.rs) instead of `println!`. When the reader stops early, as in `tracker list | head -1`, `println!` panics on the broken pipe. `print_out` stops writing and exits with the command's usual code. Any other write error is reported on stderr and exits 3. Errors on stderr are written with `writeln!` and ignored if stderr is closed, since there's nowhere left to report them.

### The default project

When `create` has no `--project` flag, it canonicalizes the current folder and each project's root, and picks the project whose root contains the current folder ([config.rs](../src/config.rs) `project_for_dir`):

- The comparison is by path component, so `/work/xx-other` is not inside `/work/xx`.
- When roots are nested, as with a monorepo and one of its packages, the **deepest** root wins.
- A root that no longer exists is skipped.
- If no root contains the current folder, `create` exits 1 and names the registered keys.
- Two projects can't have the same root, so there's never a tie to break. `project add` refuses one, under the lock, and names the project that already has it ([config.rs](../src/config.rs) `project_with_root`).

### Keeping hand-edited config

`project add` loads `config.json` as a plain `serde_json::Value` rather than into the `Config` struct, inserts the new project, and writes the file back. serde_json's `preserve_order` feature keeps keys in the order they were written, so settings this version doesn't know about survive in place.

## JSON output

With `--json`, every command prints exactly one JSON object on stdout, pretty-printed, and keeps the same exit code. Agents rely on these field names (§3), so renaming or removing one is a breaking change.

| Command | Prints |
| --- | --- |
| `init` | `{"home": "<path>", "created": true}`, where `created` is false if the store already existed |
| `project add` | `{"key", "name", "root", "design"}`, where `design` is `null` if it wasn't given |
| `project list` | `{"projects": [{"key", "name", "root", "design", "counts": {"open", "in-progress", "review", "done", "shelved"}}]}`, with all five states always present and sorted by key |
| `create` | The new item, with exactly the fields of its item file |
| `list` | `{"items": [<item>, …]}`, sorted by project and then by number |
| any failure | `{"error": "<message>", "exit_code": <n>}` |

Without `--json`, success text goes to stdout and errors go to stderr as `error: <message>`. The text is for people and its wording can change freely.

## Code tour

The crate is a library plus a binary. The library holds the store and file formats, and the binary holds the command line. The split lets integration tests call the real serialization code.

| File | What's in it |
| --- | --- |
| [src/main.rs](../src/main.rs) | clap definitions (`Cli`, `Command`, `ProjectCommand`), one function per command (`init`, `project_add`, `project_list`, `create`, `list`), the `Output { text, json }` each one returns, error printing and exit codes (`fail`), `print_out`, which writes to stdout without panicking on a closed pipe, and a small `table` helper for aligned text. |
| [src/lib.rs](../src/lib.rs) | Declares the library modules. |
| [src/error.rs](../src/error.rs) | `Error { code, message }`, with `Error::input` (exit 1) and `Error::store` (exit 3), and the `Result<T>` alias. |
| [src/store.rs](../src/store.rs) | `Store`: finding the store (`locate`, which refuses a relative `TRACKER_HOME`), its paths, `init`, `require_exists`, `lock`, `next_id` with its tripwire `highest_number_on_disk`, `save_item` and `load_items`. Also the free functions `read_file` and `write_file`, which writes atomically and flushes to disk. |
| [src/config.rs](../src/config.rs) | `Config` and `Project`, `load`, `add_project`, `normalize_key`, `project_for_dir`, `project_with_root` and `key_list`. |
| [src/item.rs](../src/item.rs) | `Item` and the `Kind`, `Status` and `Priority` enums with their serde names, `Status::ALL`, `now()` (to the second), and `to_json`/`from_json`, the only code that turns an item into file text and back. |

To add a command: add a variant to `Command` in `main.rs`, write a function that returns `Result<Output>` with both `text` and `json` filled in, and call it from the `match` in `main`. Write commands should take `store.lock()` before reading the files they're going to change.

## Testing

### Running the tests

```sh
cd guild-portfolio/tracker
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

`cargo clippy --all-targets -- -D warnings` lints the tests too, and is clean as well. To run one test: `cargo test --test create_and_list ten_creates`.

### The harness

[tests/common/mod.rs](../tests/common/mod.rs) gives each test an `Env` with two temporary folders that are deleted when the test ends:

- `home`: the store. Every command gets `TRACKER_HOME` pointing here, so no test touches `~/.tracker` or sees another test's items.
- `work`: project roots, plus `work/outside`, a folder inside no root. Commands run from `outside` unless a test picks another folder.

| Helper | Use |
| --- | --- |
| `Env::new()` | A store that `tracker init` has already set up |
| `Env::without_store()` | An empty `TRACKER_HOME`, for testing `init` and "Run `tracker init`." |
| `env.cmd()` | An `assert_cmd::Command` for the built binary, with the store and folder set |
| `env.json(args)` / `env.json_in(dir, args)` | Runs with `--json`, expects exit 0, returns the parsed `serde_json::Value` |
| `env.add_project(key, root)` | Registers a project with its root at `work/<root>` and returns that path |
| `env.create(project, title)` | Creates an item and returns its ID |
| `env.dir(rel)`, `env.item_path(id)`, `env.read_store_file(name)` | Paths and raw store files |
| `parse(stdout)` | Parses stdout as JSON, showing the output if it doesn't parse |

The binary is found with `env!("CARGO_BIN_EXE_tracker")`, which Cargo sets for integration tests. Tests assert on the `--json` output, not on the text, because the text can change.

### Coverage of the _Done when_ checks

| _Done when_ (PLAN.md) | Tests (`tests/create_and_list.rs` unless noted) |
| --- | --- |
| Three creates in two projects get `XX-1`, `XX-2` and `YY-1`. `list` shows all three, and `project list` shows the counts. | `ids_count_up_per_project_and_list_shows_every_item`, `project_list_counts_every_state` |
| Inside XX's root, `create` needs no `--project`. Outside every project root, without it, `create` exits 1. | `create_inside_a_project_root_needs_no_project_flag`, `create_outside_every_root_without_project_exits_1`, `nested_roots_pick_the_deepest`, `project_flag_overrides_the_current_folder` |
| An empty or whitespace-only title exits 1. A description is stored in `description`. | `empty_or_whitespace_title_exits_1`, `description_is_stored_in_the_description_field` |
| An item file read back and written again is byte-for-byte unchanged. | `item_file_read_back_and_written_again_is_unchanged`, `item_file_has_two_space_indents_and_a_fixed_field_order` |
| Ten `create` commands started at once get ten different IDs. | `ten_creates_at_once_get_ten_different_ids` |
| Every command's `--json` output parses. | `every_command_prints_json_that_parses` |
| DESIGN.md's links to `judge-eval/`, PLAN.md and walkthrough.html still work. | `tests/docs_links.rs`: `design_doc_still_links_to_the_files_moved_with_it`, `relative_links_in_the_docs_resolve` |
| After `cargo build`, `git status -- tracker` shows nothing from `target/`. | Checked by hand (below) |

Further tests cover the edges: `create_makes_an_open_medium_change`, `unknown_project_exits_1_and_names_the_known_ones`, `ids_come_from_counters_json_not_from_the_files`, `commands_without_a_store_say_run_tracker_init`, `init_again_keeps_the_store`, `project_add_refuses_bad_input`, `project_keys_are_upper_case_and_roots_absolute`, `project_add_keeps_settings_written_by_hand` and `bad_arguments_exit_1_and_help_exits_0`.

The fixes after review 1 each brought a regression test, listed with its bug [below](#fixes-after-review-1).

**The lock test really tests the lock.** With `store.lock()` removed from `create`, `ten_creates_at_once_get_ten_different_ids` failed in 5 out of 5 runs, with duplicate IDs. With the lock in place it passes. That test spawns ten real processes at once with `std::process::Command`. It also checks that `counters.json` ends at 11 and that `items/` holds exactly ten files, so no `.tmp` file is left over.

**The round-trip test** creates items through the binary, including a title and description with unicode, quotes, a backslash, tabs and line breaks. It then parses each file with `tracker::item::from_json`, writes it again with `tracker::item::to_json`, and compares the text. Keep this test as fields are added: PLAN.md's integration risks rely on it.

### Checks done by hand

```sh
cd guild-portfolio/tracker
cargo build
git status --short -- .                     # lists no target/ files
git status --short --ignored -- . | grep target   # target/ appears only as ignored (!!)
ls -a | grep -x .git                        # prints nothing: no nested repo
```

## Decisions worth knowing

- **Bad arguments exit 1, not 2.** Under `--json`, clap's message is reduced to one line in the usual error object.
- **Errors under `--json` go to stdout**, as one object, so an agent always parses exactly one object from stdout.
- **`counters.json` stores the next number**, not the last one used, as §4.1 describes it.
- **`list` shows every project's items wherever you run it, for now.** After review 1, DESIGN.md §3 decided that inside a project root `list` shows only that project, with `--all` for every project and the scope named in its output. Slice 3 builds that, along with the other filters, priority sorting and hiding shelved items.
- **An unparseable item file stops `list` and `project list` with exit 3**, naming the file and the parser's line and column. Slice 5 changes this to report the file and skip it (§4.4).
- **Times are kept to the second**, to match §4.3's example and keep files easy to read.
- **Titles must be one line**, since `list` prints one item per line. Details go in `--description`.
- **The design path isn't checked** by `project add`. G6, in slice 6, is what reads it.
- **Plain Rust, as CLAUDE.md asks:** the code uses the standard library and plain functions. The only generics are serde's derives and the `Result<T>` alias.
- **A relative `TRACKER_HOME` is refused rather than made absolute**, because making it absolute would still give a different store in each folder. A relative `HOME` has the same problem for the `~/.tracker` fallback, and isn't checked.

## Fixes after review 1

[slice-1-review-1.md](slice-1-review-1.md) found eight bugs, each reproduced against the binary. Each is fixed in its own commit, with a regression test. For bugs 1–7, the test was seen failing with the fix taken out.

| Bug | Commit | Fix | Test |
| --- | --- | --- | --- |
| 1. A non-UTF-8 argument panics | `beaf786` | The clap error path looks for `--json` with `args_os()`, since `args()` panics on non-Unicode input. | `an_argument_that_isnt_utf8_exits_1_without_panicking` (Unix only) |
| 2. A relative `TRACKER_HOME` gives a different store in each folder | `f7f0be9` | `locate` refuses it with exit 1, suggesting the absolute path. | `a_relative_tracker_home_is_refused` |
| 3. An unreadable store is reported as missing | `2b05305` | `require_exists` reads metadata instead of using `is_file()`, which hid permission errors. Only "not found" says "Run `tracker init`."; anything else exits 3. | `an_unreadable_store_exits_3_and_isnt_reported_as_missing` (Unix only) |
| 4. A closed pipe panics | `21e42d6` | `print_out` replaces `print!` and `println!`. | `a_reader_that_stops_early_doesnt_make_list_panic` |
| 5. Two projects can share one root | `f0d6da1` | `project add` refuses a root that's already registered. | `two_projects_cant_share_a_root` |
| 6. A lost counter can reuse a deleted item's ID | `439ff19` | `next_id` refuses a counter at or below the highest number on disk. | `a_lost_counter_doesnt_hand_out_an_id_already_on_disk` |
| 7. A partly deleted store gives a raw error | `96be208` | `require_exists` also checks `counters.json` and `items/`. | `a_store_missing_a_part_says_run_tracker_init` |
| 8. Writes aren't flushed to disk | `e79f279` | `write_file` flushes before the rename, flushes the folder after it, and removes the `.tmp` file on any failure. | `tests/store_writes.rs` |

Bug 8's tests would have passed before the fix too. A crash or a half-finished write can't be reproduced in a test, so they only guard that a write, successful or failed, leaves no `.tmp` file behind.

Still open from the review: security items S1–S3, the design questions, and the test gaps.

## Limits, and what later slices change

- **Item files change shape in later slices.** Each new field needs `#[serde(default, skip_serializing_if = …)]` so that files written by this slice still load and still round-trip byte for byte. Put each field where §4.3 places it: for example, `labels` between `priority` and `created`, and `history` before `description`.
- **This version drops fields it doesn't know** if it rewrites an item. It never rewrites an existing item, so nothing is lost now. From slice 2 on, commands rewrite items, so install a newer binary only between items (PLAN.md, integration risks).
- **`Status` already has all five states**, but no command moves between them. Slice 2 adds the moves and `history`.
- **Slice 5's store checks are partly here already.** A store with `chmod 000` exits 3 (`an_unreadable_store_exits_3_and_isnt_reported_as_missing`), and `commands_without_a_store_say_run_tracker_init` covers the "Run `tracker init`." wording for all four commands that need a store. Slice 5 still has to skip unparseable item files and test that no temporary files are left after every command.
- **The tool isn't installed yet.** `cargo install --path .` and self-tracking with a `TR` project start after slice 6.

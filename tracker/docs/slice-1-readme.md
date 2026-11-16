# Slice 1: Create and list

`tracker` is a command-line work tracker, written in Rust, that keeps one central list of work items across many projects and, in later slices, decides when an item is really done. This document covers **slice 1**, the first of the 16 slices in [PLAN.md](../PLAN.md): what it built, how it works, and how to test it.

After slice 1 you can **register projects, create items in them, and list everything from one store**. The spec is [DESIGN.md](../DESIGN.md). This slice implements §3 (interface) for `init`, `project add`, `project list`, `create` and `list`, and §4.1–4.3 (store layout, config and item file).

| | |
| --- | --- |
| Status | Done. Every _Done when_ check passes. From [review 1](slice-1-review-1.md), bugs 1–8 and security items S1–S3 are fixed, its design questions are settled, and its test gaps are closed ([below](#fixes-after-review-1)). From [review 2](slice-1-review-2.md), its five fixes before slice 2, two low-severity items, the schema mismatch, its test gaps and the `rust-version` nit are done ([below](#fixes-after-review-2)). |
| Branch | `feat/tracker/slice-1` |
| Test command | `cargo fmt --check && cargo clippy -- -D warnings && cargo test`, run in `tracker/` |
| Tests | 67: 38 in `tests/create_and_list.rs`, 8 in `tests/item_files.rs`, 6 in `tests/output.rs`, 5 in `tests/docs_links.rs`, 3 each in `tests/fixtures.rs` and `tests/schemas.rs`, and 2 each in `tests/item_ids.rs` and `tests/store_writes.rs` |

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
- [Fixes after review 2](#fixes-after-review-2)
- [Limits, and what later slices change](#limits-and-what-later-slices-change)

## What's in this slice

**Built:**

- A Rust crate at `guild-portfolio/tracker/`, made with `cargo init --vcs none`. It's an ordinary folder in the guild-portfolio repo, not a nested repo. `tracker/.gitignore` ignores `/target`.
- `DESIGN.md`, `PLAN.md`, `walkthrough.html` and `judge-eval/` were moved here with `git mv` from `chapters/02-the-methodology/02-tracking-your-work/`, so their history follows them and their relative links still work.
- Five commands: `init`, `project add`, `project list`, `create` and `list`. Every one takes `--json`.
- A central store, with one JSON file per item and IDs handed out from `counters.json` under an exclusive file lock.
- A default project: `create` picks the project whose root contains the current folder.
- An integration test harness (assert_cmd and tempfile) in which every test gets its own temporary `TRACKER_HOME` and `HOME`.

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
| `tracker init` | Creates whatever parts of the store are missing, so it also repairs a store that has lost `config.json`, `counters.json` or `items/`, and says which parts it put back and what that means. It checks the files already there first, and exits 3, writing nothing, if `config.json` or `counters.json` can't be used. Running it on a complete store changes nothing and exits 0. |
| `tracker project add KEY --name NAME --root PATH [--design FILE]` | Registers a project. The key is uppercased (`bm` becomes `BM`). The root must be an existing folder that no other project uses, and is stored as an absolute, canonical path. |
| `tracker project list` | Lists projects with their item counts for all five states, followed by any project that has items but isn't in `config.json`. |
| `tracker create "title" [--description TEXT] [--project KEY]` | Creates an open item and prints its ID. Without `--project`, it uses the project whose root contains the current folder. |
| `tracker list` | Lists every item, grouped by project and sorted by number. |

`--json` is a global flag, so `tracker list --json` and `tracker --json list` both work.

**Input rules** (§2.1, §4.4):

- **Title:** surrounding whitespace is trimmed. An empty or whitespace-only title exits 1. So does a title with a line break of any kind, a control character such as a tab or the escape that starts a terminal colour code, or a mark that reorders text (U+200E/F, U+202A–202E, U+2066–2069).
- **Description:** stored as given. An empty or whitespace-only one is left out.
- **Project key:** up to 10 ASCII letters and digits, starting with a letter. A dash isn't allowed, since the dash separates the key from the number in `BM-12`. `--project` is also uppercased before lookup. Keys in `config.json` are checked too, including ones written by hand: they must already be in capitals.
- **Project name:** can't be empty, and follows the title's rules for line breaks and control characters, since `list` and `project list` print it.
- **Root:** must exist and be a folder. A relative path is resolved against the current folder. Two projects can't share a root, even when it's spelled differently (`xx/../xx`), but one root can be nested inside another. Its path follows the title's rules for control characters, since `project list` prints it. A root written by hand in `config.json` must be absolute, or start with `~/`.
- **`TRACKER_HOME`:** must be absolute. A relative path would name a different store in every folder, so it's refused with exit 1.

**Exit codes** (§3):

| Code | Meaning in this slice |
| --- | --- |
| `0` | Success, including `init` on an existing store, and output cut short because its reader stopped early (`tracker list \| head -1`). |
| `1` | Bad input: a bad title, key, name or root, a root another project already uses, an unknown project, no project for this folder, a duplicate key, a relative `TRACKER_HOME`, or a bad argument, including one that isn't valid UTF-8 (see below). Also a store that doesn't exist yet, or is missing `counters.json` or `items/`: both say "Run `tracker init`." |
| `2` | Not used yet. It's reserved for "refused by a gate" (slice 6). |
| `3` | The store can't be read or written: one that exists but can't be read (such as `chmod 000`), an unparseable `config.json`, `counters.json` or item file, an item file whose name, `id` and `project` disagree, a project key or relative root in `config.json`, a lock that can't be taken, a counter of 0, at its largest, or at or below a number already on disk, an item file a rewrite would lose fields from, or output that can't be written. |

clap normally exits with 2 when it can't parse the arguments. Here 2 means a gate refused, so bad arguments exit 1 instead. `--help` and `--version` exit 0.

## The store

The store is `$TRACKER_HOME`, which must be an absolute path, or `~/.tracker` when that's unset or empty (§4.1). When `init` creates the store's folder, it makes it readable only by you (0700), along with any missing folders above it. A folder that already exists keeps its permissions, since `TRACKER_HOME` might name a folder with other uses. `init` creates:

```
$TRACKER_HOME/
├── config.json     # registered projects
├── counters.json   # the next item number for each project
├── lock            # an empty file; commands lock it while they write
└── items/
    └── BM-1.json   # one file per item
```

`digests.json` and `judge/` from §4.1 aren't created yet. The slices that use them add them.

Every other command checks for `config.json`, `counters.json` and `items/` before doing anything ([store.rs](../src/store.rs) `require_exists`). With no `config.json` there's no store, and the message is "There's no tracker store at … Run `tracker init`." If only `counters.json` or `items/` is missing, the message names it and says to run `init` to put it back. `init` then says what it put back: `counters.json` comes back empty, so it says to set the counters again, and `config.json` comes back with no projects. Both exit 1. A store that's there but can't be read exits 3 instead, so a permissions problem isn't mistaken for a missing store.

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

So far the tool reads only `name`, `root` and `design`. Any other setting from §4.2 (`checks`, `review`, `judge`, …) is ignored when reading, and **kept** when `project add` rewrites the file. A root written by hand can start with `~/`, as in §4.2's example, and the tool expands it (tested by `a_root_written_by_hand_with_a_tilde_is_inside_home`).

### `counters.json`

```json
{
  "BM": 3
}
```

Each value is the **next** number to hand out. A project with no entry starts at 1. A value of 0 is refused, since IDs start at 1, and so is the largest number a `u64` holds, since no number is left after it. IDs come only from this file, never from the item files that happen to exist, so a deleted item's ID is never reused (§4.4).

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
- A file is read only as the item its name says: `BM-1.json` must hold `"id": "BM-1"` in project `BM` ([store.rs](../src/store.rs) `read_item`). Otherwise a copied or renamed file would list one ID twice, and a later rewrite would overwrite the other item's file. A mismatch exits 3 and names the file.

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

Everything printed is escaped first ([text.rs](../src/text.rs)). Input is checked too, but a file edited by hand, or a folder's name, can still hold an escape sequence or a mark that reorders text. So text output writes each character that would change the display as `\u{1b}`, in table cells, headings and error messages alike, and a cell's line breaks too, so a title can't start a line of its own. JSON output writes them as JSON escapes such as `\u202e`, since serde_json escapes only those below U+0020 by itself. The JSON still parses to the same value.

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

Each output is defined by a JSON Schema in [schemas/](../schemas/), which is the reference. In short:

| Command | Prints | Schema |
| --- | --- | --- |
| `init` | `{"home": "<path>", "created": true, "repaired": []}`, where `created` is false if the store already existed, and `repaired` lists the parts of an existing store it put back: `config.json`, `counters.json` or `items/` | [init](../schemas/init.schema.json) |
| `project add` | `{"key", "name", "root", "design"}`, where `design` is `null` if it wasn't given | [project-add](../schemas/project-add.schema.json) |
| `project list` | `{"projects": [{"key", "name", "root", "design", "counts": {"open", "in-progress", "review", "done", "shelved"}}]}`, with all five states always present and sorted by key, and `"unregistered": [{"key", "counts"}]` for projects that have items but aren't in `config.json` | [project-list](../schemas/project-list.schema.json) |
| `create` | `{"item": <item>}`: the new item, exactly as its file holds it | [create](../schemas/create.schema.json) |
| `list` | `{"items": [<item>, …]}`, sorted by project and then by number | [list](../schemas/list.schema.json) |
| any failure | `{"error": "<message>", "exit_code": <n>}`, the base shape that slice 6's gate refusals add fields to | [error](../schemas/error.schema.json) |

An item, in an item file or in `create` and `list`, matches [item.schema.json](../schemas/item.schema.json). Its schema allows no field it doesn't name, so a slice that adds an item field adds it there too.

`create` wraps its item in `item` (since review 1), so that fields about the command, such as the warnings `start` will print, never sit among the item's own.

Without `--json`, success text goes to stdout and errors go to stderr as `error: <message>`. The text is for people and its wording can change freely.

## Code tour

The crate is a library plus a binary. The library holds the store and file formats, and the binary holds the command line. The split lets integration tests call the real serialization code.

| File | What's in it |
| --- | --- |
| [src/main.rs](../src/main.rs) | clap definitions (`Cli`, `Command`, `ProjectCommand`), one function per command (`init`, `project_add`, `project_list`, `create`, `list`), the `Output { text, json }` each one returns, error printing and exit codes (`fail`), `print_out`, which writes to stdout without panicking on a closed pipe, and a small `table` helper for aligned text. |
| [src/lib.rs](../src/lib.rs) | Declares the library modules. |
| [src/error.rs](../src/error.rs) | `Error { code, message }`, with `Error::input` (exit 1) and `Error::store` (exit 3), and the `Result<T>` alias. |
| [src/store.rs](../src/store.rs) | `Store`: finding the store (`locate`, which refuses a relative `TRACKER_HOME`), its paths, `init` and the `Init` it returns, `require_exists`, `lock`, `next_id` with `read_counters` and its tripwire `highest_number_on_disk`, `save_item` (new items only), `update_item` (rewrites, refused when they'd lose fields), `load_item` and `load_items`. Also the free functions `read_item`, which checks a file is the item its name says, `read_file` and `write_file`, which writes atomically and flushes to disk. |
| [src/config.rs](../src/config.rs) | `Config` and `Project`, `load`, `add_project`, `normalize_key`, `project_for_dir`, `project_with_root` and `key_list`. |
| [src/item.rs](../src/item.rs) | `Item` and the `Kind`, `Status` and `Priority` enums with their serde names, `Status::ALL`, `now()` (to the second), `to_json`/`from_json`, the only code that turns an item into file text and back, and `lost_on_rewrite`, the fields a rewrite would lose or change. |
| [src/text.rs](../src/text.rs) | `check_printable`, which refuses control characters on input, and `for_terminal` and `for_json`, which escape them on output. `changes_display` is the rule both use. |

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
- `work`: project roots, plus two folders inside no root. Commands run from `work/outside` unless a test picks another folder, and `HOME` points at `work/home`. So a test that unsets `TRACKER_HOME`, or forgets it, gets `work/home/.tracker` rather than the real store, and a root written as `~/…` can be tested.

Every command a test runs is built in one place, `Env::std_cmd()`, which sets both variables and the folder. Tests that spawn the binary themselves, such as the concurrency and closed-pipe tests, use it too.

| Helper | Use |
| --- | --- |
| `Env::new()` | A store that `tracker init` has already set up |
| `Env::without_store()` | An empty `TRACKER_HOME`, for testing `init` and "Run `tracker init`." |
| `env.cmd()` | An `assert_cmd::Command` for the built binary, with `TRACKER_HOME`, `HOME` and the folder set |
| `env.std_cmd()` | The same as a `std::process::Command`, for tests that spawn the process themselves |
| `env.user_home()` | The folder `HOME` points at |
| `env.json(args)` / `env.json_in(dir, args)` | Runs with `--json`, expects exit 0, checks the output against its schema, and returns the parsed `serde_json::Value` |
| `env.error(args, code)` / `env.error_in(dir, args, code)` | Runs with `--json`, expects exit `code`, checks the output against `error.schema.json`, and returns the message |
| `env.add_project(key, root)` | Registers a project with its root at `work/<root>` and returns that path |
| `env.create(project, title)` | Creates an item and returns its ID |
| `env.dir(rel)`, `env.item_path(id)`, `env.read_store_file(name)` | Paths and raw store files |
| `parse(stdout)` | Parses stdout as JSON, showing the output if it doesn't parse |
| `check_schema(name, value)` | Checks a value against `schemas/<name>.schema.json` |

`std_cmd` finds the binary with `env!("CARGO_BIN_EXE_tracker")`, which Cargo sets for integration tests. Tests assert on the `--json` output, not on the text, because the text can change.

`env.json` and `env.json_in` also check every output against its command's schema with `check_schema`, and `schema_for` panics for a command that has no schema yet. `check_schema` loads `schemas/` into a `jsonschema` registry under each schema's `$id` (`https://tracker.invalid/schemas/…`, a domain that can never resolve), so a `$ref` from one schema to another resolves from the local files. [tests/schemas.rs](../tests/schemas.rs) checks that each schema is valid and named after its file, that every item file the tool writes matches `item.schema.json`, and that wrong output is caught, including through a `$ref`. A schema check that never fires would otherwise look the same as one that passes.

### Coverage of the _Done when_ checks

| _Done when_ (PLAN.md) | Tests (`tests/create_and_list.rs` unless noted) |
| --- | --- |
| Three creates in two projects get `XX-1`, `XX-2` and `YY-1`. `list` shows all three, and `project list` shows the counts. | `ids_count_up_per_project_and_list_shows_every_item`, `project_list_counts_every_state` |
| Inside XX's root, `create` needs no `--project`. Outside every project root, without it, `create` exits 1. | `create_inside_a_project_root_needs_no_project_flag`, `create_outside_every_root_without_project_exits_1`, `nested_roots_pick_the_deepest`, `project_flag_overrides_the_current_folder` |
| An empty or whitespace-only title exits 1. A description is stored in `description`. | `empty_or_whitespace_title_exits_1`, `description_is_stored_in_the_description_field` |
| An item file read back and written again is byte-for-byte unchanged. | `item_file_read_back_and_written_again_is_unchanged`, `item_file_has_two_space_indents_and_a_fixed_field_order`, and `tests/fixtures.rs` for files an earlier binary wrote |
| Ten `create` commands started at once get ten different IDs. | `ten_creates_at_once_get_ten_different_ids` |
| Every command's `--json` output parses. | `every_command_prints_json_that_parses` |
| DESIGN.md's links to `judge-eval/`, PLAN.md and walkthrough.html still work. | `tests/docs_links.rs`: `design_doc_still_links_to_the_files_moved_with_it`, `relative_links_in_the_docs_resolve`, and `the_docs_folder_is_checked`, since the links in `docs/` are checked too |
| After `cargo build`, `git status -- tracker` shows nothing from `target/`. | Checked by hand (below) |

Further tests cover the edges: `create_makes_an_open_medium_change`, `unknown_project_exits_1_and_names_the_known_ones`, `ids_come_from_counters_json_not_from_the_files`, `commands_without_a_store_say_run_tracker_init`, `init_again_keeps_the_store`, `project_add_refuses_bad_input`, `project_keys_are_upper_case_and_roots_absolute`, `project_add_keeps_settings_written_by_hand` and `bad_arguments_exit_1_and_help_exits_0`.

The fixes after review 1 each brought a regression test, listed with its bug [below](#fixes-after-review-1).

**The lock test really tests the lock.** With `store.lock()` removed from `create`, `ten_creates_at_once_get_ten_different_ids` failed in 5 out of 5 runs, with duplicate IDs. With the lock in place it passes. That test spawns ten real processes at once with `std::process::Command`. It also checks that `counters.json` ends at 11 and that `items/` holds exactly ten files, so no `.tmp` file is left over.

**The round-trip test** creates items through the binary, with a title and description holding unicode, quotes, a backslash and line breaks. It then parses each file with `tracker::item::from_json`, writes it again with `tracker::item::to_json`, and compares the text. That shows the current code is consistent with itself, but it can't catch a change that stops older files loading, because it writes the files with the same code that reads them.

**The fixtures are what PLAN.md's integration risks rely on.** `tests/fixtures/slice-1/` holds two item files that the slice 1 binary wrote: one with unicode, quotes, a backslash, tabs and line breaks, and one with no description. [tests/fixtures.rs](../tests/fixtures.rs) checks that every fixture loads, comes back byte for byte, matches `item.schema.json`, and lists exactly as it is on disk. Storing `kind` in capitals, for example, fails it with "XX-1.json no longer loads". **Never edit a fixture.** A slice that changes the item file adds its own `tests/fixtures/slice-<n>/` folder, written by its binary, and every folder stays tested.

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
- **Only an item ID can become a file path.** `Store::item_path` refuses anything that isn't `KEY-n`, with no leading zeros, so an ID typed later, as in slice 4's `show` and `delete`, can't name a file outside `items/`.
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

### Security items

Each is fixed in its own commit, and each test was seen failing with the fix taken out.

| Item | Commit | Fix | Test |
| --- | --- | --- | --- |
| S1. Control characters in titles reach the terminal | `f5b121c` | `check_printable` in `main.rs` refuses control characters, line breaks of every kind, and the marks that reorder text, naming the character, such as `U+001B`. It covers titles and project names, since both are printed. | `titles_and_project_names_cant_hold_terminal_control_characters` |
| S2. Project keys and item IDs can name files outside the store | `2d4a474` | `config::load` refuses a key that isn't valid, with exit 3. `Store::item_path` refuses anything that isn't an item ID. The rules are `config::is_valid_key` and `item::is_valid_id`. | `a_bad_project_key_in_config_json_is_refused`, and `tests/item_ids.rs` |
| S3. A new store is readable by other users | `ce8fb99` | `init` creates the store's folder with mode 0700, and leaves a folder that already exists alone. | `a_new_store_is_readable_only_by_its_owner` (Unix only) |

A few consequences:

- An existing `~/.tracker` keeps its permissions. Run `chmod 700 ~/.tracker` to tighten it.
- Tabs are now refused in titles. An item that already has one still loads, because the check runs only on input.
- Descriptions aren't checked, since they can have several lines and nothing prints them yet. Slice 4's `show` should escape their control characters.
- One bad key in `config.json` stops every command that reads it, until the key is renamed there.

### Design decisions

The review found two places where the code and DESIGN.md disagreed, settled after discussion.

| Question | Decision | Commits |
| --- | --- | --- |
| Which project does a command cover? | DESIGN.md §3 now states the rule per kind of command. A one-project command such as `create` uses the current folder's project or `--project <KEY>`, and exits 1 outside every root. A listing command, `list` or `digest`, shows the current project inside a root and every project outside one, with `--all` for every project and its output naming what it covered. `review`, `judges` and `audit` always cover every project. `list` doesn't do this yet: slice 3's prompt and _Done when_ now include it. The same commit fixed sentences that read as if BM were the tool's own project, and added a note that the examples use BM and TR. | `d9c0327` |
| Where are the `--json` shapes defined, and how do errors relate to gate refusals? | JSON Schemas in [schemas/](../schemas/), checked against every output by the tests (see [JSON output](#json-output) and [The harness](#the-harness)). Every non-zero exit has `error` and `exit_code`, and slice 6's refusals add `id`, `result`, `gates` and `failures` to that. A command that returns an item wraps it, so `create --json` now prints `{"item": …}`. | `315b84d`, `8d5e49f`, `542d7a4` |

### Test gaps

| Gap | Commit | What it adds | Test |
| --- | --- | --- | --- |
| 1. The round-trip test can't catch older files that stop loading | `0bf580b` | Item files written by the slice 1 binary, in `tests/fixtures/slice-1/`. | `tests/fixtures.rs` |
| 2. `HOME` isn't isolated | `b9f4a5b` | `HOME` points at a temporary folder, and `Env::std_cmd()` is the one place a test command is built. | `without_tracker_home_the_store_is_in_home_not_the_real_one` |
| 3. The `~/` expansion of roots has no test | `e5bca0b` | A root written as `~/proj` in `config.json` is found from inside it, and counts as the same root as its full path. | `a_root_written_by_hand_with_a_tilde_is_inside_home` |
| 4. `docs_links.rs` doesn't check `docs/` | `7816399` | Every `.md` file in `docs/` has its links checked, without being listed. | `the_docs_folder_is_checked`, `relative_links_in_the_docs_resolve` |
| 5. The bugs have no regression tests | none | Already done: each fix above brought its own test. | |

For gaps 1, 3 and 4, the new test was seen failing with what it guards broken. The link check only checked that a file exists. Since review 2 it also checks that a `#L…` line anchor names lines the file has, but not that they still hold what the doc means.

Still open from the review: its four nits, which are small cleanups ([slice-1-review-1.md](slice-1-review-1.md#nits)).

## Fixes after review 2

[slice-1-review-2.md](slice-1-review-2.md) found five things to fix before slice 2, and some smaller ones. Each fix is in its own commit with its tests.

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| 1. An item file's name and `id` aren't checked | `ead1e81` | `read_item` checks that `XX-1.json` holds `XX-1` in project `XX`, for `load_items` and the new `load_item`. | `tests/item_files.rs`: `a_file_whose_name_and_id_disagree_is_refused` and the two after it |
| 2. A rewrite drops fields this version doesn't know | `b104bf9` | `Store::update_item` refuses a rewrite that `item::lost_on_rewrite` says would lose or change a field. `save_item` refuses to overwrite a file. Every fixture must be rewritable without loss. | `tests/item_files.rs`: `update_item_…`, `save_item_never_overwrites_a_file`; `tests/fixtures.rs` |
| 3. Text from files, folder names and errors reaches the terminal unchanged | `6ec034f` | The new `text` module escapes every output, text and JSON, and `project add` refuses a root holding a control character. | `tests/output.rs` |
| 4. A relative root in `config.json` matches different folders | `d3ea4eb` | `config::load` refuses a root that isn't absolute after `~/` expansion. | `a_relative_root_in_config_json_is_refused` |
| 5. `init` says "Nothing changed" after a repair, and accepts a corrupt config | `536a921` | `init` lists what it put back (`repaired`), says what it means, and checks the existing files before writing anything. | `init_says_which_parts_it_put_back`, `init_refuses_a_store_whose_files_cant_be_read` |
| Bad counter values | `8142f0f` | A counter of 0 or at the largest `u64` exits 3 and writes nothing, instead of a confusing error or a panic. | `a_counter_below_1_or_at_its_largest_is_refused` |
| `project list` hides items of unregistered projects | `0873bcd` | They're listed after the registered ones, and in `unregistered` in the JSON. | `project_list_counts_items_whose_project_isnt_registered` |
| `item.schema.json` doesn't enforce its title rule | `1254541` | A `pattern` refuses what `create` refuses. | `a_wrong_output_is_caught` in `tests/schemas.rs` |
| Exit-3 errors aren't checked against the schema | `64d5dc2` | `Env::error` and `Env::error_in` check every error they see. | Every test that expects a failure under `--json` |
| Line anchors aren't checked | `b04dcb3` | A `#L12` or `#L12-L30` link must name lines the file has. | `line_anchors_name_lines_that_exist` |
| No `rust-version` | `a610a92` | `rust-version = "1.89"` in Cargo.toml, for `File::lock`. | |

A few consequences:

- **clap's error messages lose their colour**, because they're now written as plain text so they can be escaped. Slice 5 brings colour back with anstream.
- **A hand-edited item with a tab in its title now lists**, with the tab shown as `\u{9}`.
- **One mismatched item file stops `list` and `project list` with exit 3**, as an unparseable one does. Slice 5 makes both report the file and skip it.

Still open from review 2: a non-UTF-8 `TRACKER_HOME` under `--json` (untested on macOS), the symlinked `.tmp` file, checking hand-written project names and `design`, and its two commit-message points. DESIGN.md §4.4's "all validated" sentence now says what is checked on input, what is checked again when a file is read, and that free text from a file is escaped on output instead.

## Limits, and what later slices change

- **Item files change shape in later slices.** Each new field needs `#[serde(default, skip_serializing_if = …)]` so that files written by this slice still load and still round-trip byte for byte, which `tests/fixtures.rs` checks. Put each field where §4.3 places it: for example, `labels` between `priority` and `created`, and `history` before `description`. Add it to `schemas/item.schema.json` in the same slice, and add a `tests/fixtures/slice-<n>/` folder of files written by that slice's binary.
- **This version refuses to rewrite an item it would lose fields from**, such as one a newer tracker wrote. `Store::update_item` is the write slice 2's moves should use: it re-reads the file under the lock and exits 3 rather than drop a field. So install a newer binary only between items still (PLAN.md, integration risks), but a mistake now fails loudly instead of losing data.
- **`Status` already has all five states**, but no command moves between them. Slice 2 adds the moves and `history`.
- **Slice 5's store checks are partly here already.** A store with `chmod 000` exits 3 (`an_unreadable_store_exits_3_and_isnt_reported_as_missing`), and `commands_without_a_store_say_run_tracker_init` covers the "Run `tracker init`." wording for all four commands that need a store. Slice 5 still has to skip unparseable item files and test that no temporary files are left after every command.
- **The tool isn't installed yet.** `cargo install --path .` and self-tracking with a `TR` project start after slice 6.

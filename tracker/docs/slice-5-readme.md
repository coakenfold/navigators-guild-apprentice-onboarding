# Slice 5: Bad data and polish

`tracker` is a command-line work tracker, written in Rust, that keeps one central list of work items across many projects and, in later slices, decides when an item is really done. This document covers **slice 5** of the 16 in [PLAN.md](PLAN.md): what it built, how it works, and how to test it. The earlier readmes cover the rest: [slice 1](slice-1-readme.md) the store, `create` and `list`, [slice 2](slice-2-readme.md) the states and `history`, [slice 3](slice-3-readme.md) priority, kind, labels, `edit` and `list`'s filters, and [slice 4](slice-4-readme.md) `show` and `delete`.

After slice 5 you can **keep working past a corrupt file, and get help text, colour and empty states**. The spec is [DESIGN.md](DESIGN.md). This slice implements [§4.4](DESIGN.md#44-handling-bad-data)'s first three points, colour from [§8](DESIGN.md#8-technology), and the polish PLAN.md's prompt lists. It also settles the length of a title, which [§2.1](DESIGN.md#21-items) now records, and one item slice 4 left for this slice: whether a move or `edit` rewrites a file whose values were only written differently by hand. Slices 1–5 are the course submission (PLAN.md's checkpoint).

| | |
| --- | --- |
| Status | Every _Done when_ check has a test, and every test passes, as does the test command. Nothing in this slice needs a check by hand. PLAN.md has slice 5 as `in review`. From [review 1](reviews/slice-5-review-1.md), all three bugs, the DESIGN.md item and all four test gaps are done ([below](#fixes-after-review-1)). The branch isn't merged into `main`. What's left is listed under [Still open](#still-open). |
| Branch | `feat/tracker/slice-5`, from `main` at `e3854b8` (slice 4 merged): `942f844` (anstream and anstyle), `a59f11f` (`Styled`), `0dfa462` (the slice's code and tests, in one commit, since most of `src/main.rs` changed for both halves at once), `51958aa` (DESIGN.md §2.1 and §4.4), `9765c91` (this readme and PLAN.md's status), `2b0a767` (review 1), then review 1's fixes, `e76eabb` to `af02b3b` ([Fixes after review 1](#fixes-after-review-1)) |
| Test command | `cargo fmt --check && cargo clippy -- -D warnings && cargo test`, run in `tracker/` |
| Tests | 194: 39 in `tests/create_and_list.rs`, 34 in `tests/organise.rs`, 27 in `tests/states.rs`, 24 in `tests/show_and_delete.rs`, 14 in `tests/item_files.rs`, 11 in `tests/docs_links.rs`, 10 in `tests/bad_data.rs`, 9 in `tests/polish.rs`, 7 in `tests/fixtures.rs`, 6 in `tests/output.rs`, 4 each in `tests/schemas.rs` and `tests/store_writes.rs`, 2 in `tests/item_ids.rs`, 2 unit tests in `src/main.rs` and 1 in `src/store.rs` |

## Contents

- [What's in this slice](#whats-in-this-slice)
- [Try it](#try-it)
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

- **A corrupt item file costs one item.** `list` and `project list` skip any item file that can't be read as the item its name says: JSON that doesn't parse, a value this version doesn't know (such as `"status": "closed"`), a file that isn't UTF-8, an empty file, a folder named like an item file, a file whose name and `id` disagree, or one whose `project` and ID disagree. Each is reported on stderr with its name, the parser's line and column, the reason, and what to do. The list says after its rows that something was left out, and `--json` lists the files as `skipped`.
- **A command that names a corrupt item refuses with exit 3**, with the same name, line, reason and fix, such as `tracker show BM-2`.
- **Every I/O error says what to do.** A file or folder the store can't read, write, create or lock names the path and the system's reason, and for a permission problem, how to check and fix it. A `config.json` or `counters.json` that doesn't parse names the line, the column, the reason, and what the file should hold.
- **A missing store** says "Run `tracker init`." for every command, as it did, and a store missing only `config.json` now says that `init` puts it back, rather than that there's no store at all.
- **Colour** through anstream and anstyle: bold headings in `list`, `project list` and `show`, each state and priority in its own colour, the new state after a move, and a red `error:` and a yellow `warning:` on stderr. anstream drops the codes when the output isn't a terminal, or `NO_COLOR` is set.
- **`--help` on every command and flag.** The top level now says where the store is, how to start, and what the exit codes mean, and `init`, `project add`, `create`, `start` and `show` say more under `--help`.
- **Empty states for `list`:** no projects, no items in the project (or the store), "No open items. Nice work!" when only finished or shelved items are left, and "No items match these filters." when a filter matched nothing.
- **A heading row for `list`** that names each column, lined up with the rows below it, as `project list` has.
- **A title is at most 200 characters** in `create` and `edit --title`.
- **A move or `edit` rewrites a value only written differently**, such as a time ending `+00:00` from a hand edit, rather than refusing it as a newer tracker's field. A field this version doesn't know is still refused.

**Not built yet:**

- Colour and empty states for the commands later slices add. Slice 16 brings them up to this slice's standard ([PLAN.md](PLAN.md#slice-16-digest-audit-judge-report)).
- Removing a corrupt item file through `delete`, which still refuses one ([Decisions worth knowing](#decisions-worth-knowing)).
- Escaping invisible characters, such as zero-width spaces, in titles and labels (slice 3 review 1's finding 5, [Still open](#still-open)).

## Try it

Always set `TRACKER_HOME` while developing. Without it, the tool uses your real store at `~/.tracker`.

```sh
cd guild-portfolio/tracker
cargo build

export DEMO=$(mktemp -d)
export TRACKER_HOME=$DEMO/store
export PATH="$PWD/target/debug:$PATH"
cd $DEMO

tracker list                                       # exit 1: Run `tracker init`.
tracker init
mkdir -p bookmarks && cd bookmarks
tracker project add bm --name bookmarks --root $DEMO/bookmarks
tracker list                                       # No BM items yet.
tracker create "Search filters bookmarks by tag" --kind capability --priority high --label feature
tracker create "Sort by date"
tracker create "Import from Pocket" --label import
tracker start BM-2
tracker list

# Break BM-2's file by hand: remove the comma after its title.
sed -i '' 's/"Sort by date",/"Sort by date"/' $TRACKER_HOME/items/BM-2.json   # GNU sed: -i without ''
tracker list                                       # BM-1 and BM-3, and a warning for BM-2
tracker show BM-2                                  # exit 3, with the file, line and reason
tracker list --json                                # "skipped" names the file
sed -i '' 's/"Sort by date"/"Sort by date",/' $TRACKER_HOME/items/BM-2.json

tracker shelve BM-1 --reason "After BM-2"
tracker shelve BM-3 --reason "Pocket shut down"
tracker submit BM-2
tracker list                                       # No open items. Nice work!
tracker list --priority low                        # No items match these filters.
tracker create "$(printf 'x%.0s' {1..201})"        # exit 1: at most 200 characters
chmod 000 $TRACKER_HOME
tracker list                                       # exit 3, with how to fix it
chmod 700 $TRACKER_HOME
find $TRACKER_HOME -name '*.tmp'                   # nothing
tracker --help
tracker list --help
```

What it printed, run against the debug binary on macOS, with the temporary folder shortened to `$DEMO` and stderr shown where it appeared. Piped like this, there's no colour. At a terminal the heading rows are bold, `in-progress` is cyan, `high` is red, and `error:` and `warning:` are red and yellow.

```text
$ tracker list
error: There's no tracker store at $DEMO/store. Run `tracker init`.
$ tracker list
No BM items yet. Create one with `tracker create "title" --project BM`.

BM only. `--all` for every project.
Open, in-progress and review items. `--status done,shelved` for the others.
$ tracker list
  ID    PRIORITY  KIND        STATUS       TITLE                            LABELS
BM  bookmarks
  BM-1  high      capability  open         Search filters bookmarks by tag  [feature]
  BM-2  medium    change      in-progress  Sort by date
  BM-3  medium    change      open         Import from Pocket               [import]

BM only. `--all` for every project.
Open, in-progress and review items. `--status done,shelved` for the others.
$ tracker list                    # after breaking BM-2.json
warning: Skipped $DEMO/store/items/BM-2.json, line 4, column 3: expected `,` or `}`. It's JSON, laid out as in DESIGN.md §4.3, so fix it by hand, or move it out of items/ to set it aside.
  ID    PRIORITY  KIND        STATUS  TITLE                            LABELS
BM  bookmarks
  BM-1  high      capability  open    Search filters bookmarks by tag  [feature]
  BM-3  medium    change      open    Import from Pocket               [import]

BM only. `--all` for every project.
Open, in-progress and review items. `--status done,shelved` for the others.
1 item file can't be read as an item, so it isn't listed. The warnings on stderr say which, and why.
$ tracker show BM-2
error: Can't read $DEMO/store/items/BM-2.json, line 4, column 3: expected `,` or `}`. It's JSON, laid out as in DESIGN.md §4.3, so fix it by hand, or move it out of items/ to set it aside.
$ tracker list --json             # the end of it
  "skipped": [
    {
      "file": "$DEMO/store/items/BM-2.json",
      "line": 4,
      "column": 3,
      "reason": "expected `,` or `}`"
    }
  ]
$ tracker list                    # after shelving BM-1 and BM-3, and submitting BM-2
No open items. Nice work!

BM only. `--all` for every project.
Open, in-progress and review items. `--status done,shelved` for the others.
$ tracker list --priority low
No items match these filters.

BM only. `--all` for every project.
Open, in-progress and review items. `--status done,shelved` for the others.
Filters: priority low.
$ tracker create "xxx…"           # 201 characters
error: A title can be at most 200 characters, and this one has 201. Say what the item is in a line, and put the details in --description.
$ tracker list                    # after chmod 000 $TRACKER_HOME
error: Can't read the store at $DEMO/store: Permission denied (os error 13). Check its permissions with `ls -ld $DEMO/store`. If it's yours, `chmod u+rwx $DEMO/store` lets you read and write it again.
$ find $TRACKER_HOME -name '*.tmp'
```

The exit codes were 1 for the missing store and the long title, 3 for `show BM-2` and the unreadable store, and 0 for the rest, including the `list` that skipped a file.

## How it works

### A file that can't be read as an item

`Store::load_items` reads every `*.json` file in `items/` and checks each one through `check_item`, which returns the item or a `Skipped`: the file, the parser's line and column if it stopped somewhere, the reason, and the fix. `load_items` now returns both lists, and `list` and `project list` print the items and report the rest.

| What's wrong | Line | Reason, as reported | Fix, as reported |
| --- | --- | --- | --- |
| JSON that doesn't parse, or isn't an item | the parser's | the parser's message, without its "at line … column …" | fix it by hand, or move it out of `items/` |
| A value this version doesn't know, such as `"status": "closed"` | the parser's | ``unknown variant `closed`, expected one of …`` | the same |
| Not UTF-8 | none | it isn't UTF-8 text | the same |
| An empty file | 1 | `EOF while parsing a value` | the same |
| A folder named like an item file | none | it's a folder, not a file | move it out of `items/` |
| A symlink to a file that doesn't exist | none | it's a symlink to a file that doesn't exist | remove it from `items/` |
| A name that isn't an ID, such as `notes.json` | none | it isn't named after an item ID | move it out of `items/` |
| Name and `id` disagree, such as a copy | none | it holds the item `"XX-1"`, but its name says XX-9 | rename the file or fix its `"id"` |
| `project` and the ID disagree | none | it's in project `"YY"`, but its ID XX-1 says XX | fix its `"project"` |

The last three were refused with exit 3 before this slice. §4.4 checks them when a file is read, and they now cost one item like a parse error does, rather than the whole list. A command that names the item still refuses: `load_item` and `load_item_with_text` read through `item_text` and the same `check_item`, and turn a `Skipped` into an exit-3 error with the same details.

A file that's gone by the time `load_items` reads it, because a `delete` removed it after the folder was listed, is left out without a word: `list` takes no lock, and that item is gone. `read_listed` reads each file and says which of the three it was: an item, a skipped file, or gone.

A file or folder that can't be read at all is different. If `items/` or an item file has no read permission, `load_items` fails with exit 3, since that's a problem with the store, not with one item, and skipping it would hide every item in it.

**Where the report goes.** In text mode each skipped file is one `warning:` line on stderr, and stdout ends with "1 item file can't be read as an item, so it isn't listed.", so a list saved from stdout alone still says something is missing. With `--json` nothing goes to stderr, and the files are in `skipped`. The exit code is 0, because the list worked.

### Error messages

Two helpers in [src/error.rs](../src/error.rs) build every store error:

- `error::io(doing, path, e)` for a file or folder: "Can't read …/counters.json: Permission denied (os error 13).", then, for a permission problem, how to check it with `ls -ld` and that the store should be yours. `error::io_in_store` adds, for a missing file, to run `tracker init`, and is used only for what `init` creates: the store's folder, `config.json`, `counters.json`, `items/`, the lock, and a file written into one of those folders. A missing item file isn't one of them.
- `error::parse(path, e, fix)` for a JSON file: "Can't parse …/config.json, line 3, column 5: expected `,` or `}`.", then `fix`. `config.json`'s fix points to §4.2's layout and says every command refuses until it parses. `counters.json`'s says what it maps and that each counter must be above the highest number used.

`Store::require_exists` gives the unreadable store its own message, with `ls -ld` and `chmod u+rwx` for the store's folder, since that's what a `chmod 000` breaks. `Store::locate` refuses a store folder that's a file, or that has a file where a folder above it should be, with exit 3 and which path it is.

### Colour

[src/styled.rs](../src/styled.rs) holds `Styled`: pieces of text, each with an `anstyle::Style`. A command builds its text as `Styled`, and `main` renders it and writes it through `anstream::stdout()`, which strips the codes unless stdout is a terminal, and honours `NO_COLOR` and `CLICOLOR_FORCE`. stderr's `error:` and `warning:` go through `anstream::stderr()` the same way.

`Styled::render` escapes each piece with `text::for_terminal` and only then wraps it in its codes. Before this slice, `main` escaped the whole output, which would also have escaped the colour codes. Now the escaping happens per piece, so a title edited by hand is still escaped, and can't add colour of its own (`colour_doesnt_stop_a_title_edited_by_hand_being_escaped`).

| What | Style |
| --- | --- |
| Heading rows of `list` and `project list`, project lines in `list`, `show`'s title and section names | bold |
| `in-progress` | cyan |
| `review` | yellow |
| `done` | green |
| `shelved` | dim |
| `open` | plain |
| priority `high` | red |
| priority `low` | dim |
| `error:` | bold red |
| `warning:` | bold yellow |

The question `delete` asks on stderr has no colour, so nothing changes there whether stderr is a terminal or not.

**Tables.** `table` became `styled_table`, which takes cells with a style each and returns one `Styled` line per row. The widths are counted from the plain text, and the colour goes round the cell, never the padding, so the columns line up with colour or without. `table` is now `styled_table` for plain cells.

### Help text

clap builds `--help` from the doc comments. Every command and flag had one already. This slice adds:

- at the top level, where the store is and how to start (`--help`), and the exit codes and a pointer to each command's `--help` (both `-h` and `--help`)
- a second paragraph, shown by `--help`, for `init`, `project add`, `create`, `start` and `show`
- the 200-character limit on `create`'s title and `edit --title`

`every_command_and_flag_has_help_text`, a unit test in `src/main.rs`, walks clap's command tree and fails for any command or flag without help text, so a later slice can't add one bare.

### Empty states

`list` picks one of four messages when it has no rows, by what's missing:

| When | It says |
| --- | --- |
| No projects, and no items | No projects yet. Register one with `tracker project add KEY --name NAME --root PATH`, then create items in it with `tracker create "title"`. |
| No items in the project it covers, or the store | No BM items yet. Create one with `tracker create "title" --project BM`. (or `--project KEY` for every project) |
| Items, but none open, in progress or in review, and no filter given | No open items. Nice work! |
| A filter matched nothing | No items match these filters. |

The footer still follows, saying which project the list covered and which filters it used, so the last case names the filters.

### The heading row

`list` now starts with `  ID  PRIORITY  KIND  STATUS  TITLE  LABELS`, in bold at a terminal. It's the first row of the same table as the items, so it's lined up with them by construction, wide characters included. `ID` is indented two spaces, like the IDs under each project line. `LABELS` is left out when no listed item has a label, so the heading never names a column that's empty. There's no heading row with an empty state.

### The longest title

`check_title` refuses a title of more than 200 characters, counted as Unicode characters, in `create` and `edit --title`. A label's limit is 50 because it's printed in every row, and a reason's is 500 because it's printed among other lines. A title is printed in every `list` row too, and heads `show`, so it should say in a line what the item is. 200 is room for a long sentence: PLAN.md's longest outcome is under 110. The message says to put the details in `--description`. Like a label's limit, it's a rule for what comes in, not for the file: a longer title written by hand still lists, and `item.schema.json` doesn't limit it.

### A value only written differently

Slice 4 split what a rewrite would change into `unknown` fields, which a newer tracker may have added, and `respelled` ones, which hold the same value written another way, such as a time ending `+00:00` or `"labels": []`. `show` already told them apart. `Store::update_item` now does too: it refuses only `unknown` fields, still with exit 3 and the newer-tracker message, and rewrites a respelled value the way tracker writes it. Nothing is lost, since the time is the same instant and the empty list means no labels.

### Temporary files

Every write already went through `store::write_file`: a temporary file beside the target, flushed, then renamed over it, and removed if anything fails (§4.4, slice 1). This slice adds no new kind of write, and `no_temporary_files_are_left_in_the_store_after_any_command` checks the store after every command, including refusals, a delete typed at a terminal, and a `create` whose write fails.

## JSON output

`list --json` and `project list --json` gain `skipped`, always there, and empty when every file was read:

```json
"skipped": [
  { "file": "/…/store/items/BM-2.json", "line": 4, "column": 3, "reason": "expected `,` or `}`" },
  { "file": "/…/store/items/notes.json", "line": null, "column": null, "reason": "it isn't named after an item ID, such as BM-12.json" }
]
```

`line` and `column` are null when the JSON parsed, or couldn't be read as text, and the problem is elsewhere. `reason` is only the reason: the fix is in the text, since it's for a person, and §5 tells agents never to edit the store. [list.schema.json](../schemas/list.schema.json) defines `skipped` under `$defs`, and [project-list.schema.json](../schemas/project-list.schema.json) refers to it.

No other JSON changed. The text changed in many places, which §3 allows: `list`'s heading row and empty states, the messages above, and colour at a terminal.

## Code tour

- [src/styled.rs](../src/styled.rs) (new): `Styled`, its `render`, and the styles for headings, states, priorities, `error:` and `warning:`.
- [src/error.rs](../src/error.rs): `error::io`, `error::io_in_store`, `error::parse`, `error::json_problem`, which splits a serde_json error into line, column and reason, and `error::at`.
- [src/store.rs](../src/store.rs): `Skipped`, `check_item`, `load_items` returning the skipped files, `item_text` reporting a file that isn't UTF-8 or a broken symlink, `read_listed`, `check_not_a_file`, `update_item` refusing only unknown fields, and `require_exists`'s messages.
- [src/config.rs](../src/config.rs): the fix for a `config.json` that doesn't parse.
- [src/main.rs](../src/main.rs): `Output` carries `Styled` text and `warnings`, `print_out` and `report` write through anstream, `styled_table`, `list_table` and `empty_list` for `list`, `skipped_json`, `skipped_warnings` and `skipped_note`, `section` for `show`, `MAX_TITLE`, the new help text, and the two unit tests.
- [tests/bad_data.rs](../tests/bad_data.rs) (new) and [tests/polish.rs](../tests/polish.rs) (new): this slice's tests.
- [tests/common/mod.rs](../tests/common/mod.rs): every test command runs without `NO_COLOR`, `CLICOLOR` and `CLICOLOR_FORCE`, so a developer's settings can't change what a test sees, and `script` gets `TERM=xterm-256color`, so a terminal test sees colour wherever it runs.

## Testing

```sh
cd guild-portfolio/tracker
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

All three pass, and so does `cargo clippy --all-targets -- -D warnings`. To run this slice's tests alone: `cargo test --test bad_data --test polish`, and `cargo test --bin tracker` for the help unit tests.

### Coverage of the _Done when_ checks

Every _Done when_ check in [PLAN.md's slice 5](PLAN.md#slice-5-bad-data-and-polish) has a test:

| _Done when_ | Tests |
| --- | --- |
| A hand-corrupted item file is reported with its name, line and reason, and the other items still list | `a_hand_corrupted_item_file_is_reported_and_the_others_still_list` in `tests/bad_data.rs`: the JSON's `skipped` with line 4, column 3 and the reason, stderr's warning with the name, line, reason and fix, stdout's note, `project list`'s counts, `show` and `start` refusing with the same details, and the file left as it was. Also `a_file_with_an_unknown_value_is_skipped_with_the_parsers_reason`, `a_file_that_isnt_utf8_or_is_empty_or_is_a_folder_is_skipped`, and the three `…_is_skipped` tests in `tests/item_files.rs` |
| With no store, commands say "Run `tracker init`." and never panic | `without_a_store_every_command_says_run_tracker_init`: every command, as JSON and as text, with no "panicked" on stderr and nothing created. Also `a_store_missing_only_config_json_says_to_put_it_back` |
| A store with `chmod 000` exits 3 | `a_store_with_chmod_000_exits_3`: every command, `init` included, says how to check the permissions and doesn't panic. Also `an_items_folder_with_chmod_000_exits_3`. Both skip themselves where permissions don't apply, as for root; on macOS, where they were run, neither skipped |
| No temporary files are left in the store after any command | `no_temporary_files_are_left_in_the_store_after_any_command`, which checks the whole store after every command, including refusals, a delete typed at a terminal under `script`, and a `create` that can't write. `tests/store_writes.rs` still covers `write_file` itself |
| Every command has `--help` text | `every_command_has_help_text` in `tests/polish.rs` runs `-h` and `--help` for all 15 commands and checks each one's first line, its usage and its options, and the exit codes at the top level. `every_command_and_flag_has_help_text` and `the_command_line_is_well_formed` in `src/main.rs` check clap's tree |
| `tracker list \| cat` contains no colour codes | `output_that_isnt_a_terminal_has_no_colour_codes`: literally `list --all \| cat` through `sh`, and stdout and stderr of `list`, `project list`, `show`, a move and an error, all piped. `colour_is_there_when_forced_and_turns_off_with_no_color` and `a_terminal_gets_colour` check that the colour is there to turn off |
| An empty list prints its empty-state message | `an_empty_list_prints_its_empty_state`: all four messages, no heading row with any of them, and empty `items` in the JSON |
| `list`'s text starts with a heading row that names each column, lined up with the rows below it | `list_starts_with_a_heading_row_lined_up_with_the_rows_below_it`: two projects, every state shown, labels, and a title in Chinese with an emoji, checked by terminal column for each field of each row. `the_text_list_marks_priority_kind_and_labels` in `tests/organise.rs` pins the exact lines |
| _Prompt:_ decide whether a title needs a length limit | `a_title_can_be_at_most_200_characters`: 200 Chinese characters are taken, 201 are refused by `create` and `edit --title`, and the item is left as it was |
| _Slice 4:_ whether a move or `edit` rewrites a file whose values are only written differently | `update_item_rewrites_a_value_only_written_differently`, and `update_item_refuses_a_file_it_would_lose_fields_from` for a newer tracker's field |

**Changed tests.** The three tests in `tests/item_files.rs` that expected `list` to exit 3 for a file whose name, `id` or `project` disagree now expect it skipped, and `show` still refusing it. `the_text_list_marks_priority_kind_and_labels` and `the_scope_is_reported_even_when_nothing_matches` in `tests/organise.rs` expect the heading row and the new empty state, and `a_move_refuses_an_item_whose_history_has_a_command_it_doesnt_know` compares its row's words rather than its spacing, which the heading row widened. `a_wrong_output_is_caught` in `tests/schemas.rs` adds `skipped`.

**Seen failing.** The tests were written after the code, so each was checked by breaking the code on purpose, then putting it back:

- Refusing a file instead of skipping it in `load_items` failed the three skip tests in `tests/bad_data.rs`.
- Writing to `std::io::stdout` instead of `anstream::stdout` failed `output_that_isnt_a_terminal_has_no_colour_codes`, and the two other colour tests in `tests/polish.rs`.
- Leaving out the heading row failed `the_text_list_marks_priority_kind_and_labels` and `list_starts_with_a_heading_row_lined_up_with_the_rows_below_it`.
- Raising the title limit tenfold failed `a_title_can_be_at_most_200_characters`.

**Fixtures.** The item file's shape didn't change, but PLAN.md asks for a file saved by slice 5, and it's the last shape before slice 6 adds `criteria`. So `tests/fixtures/slice-5/` holds XX-9 to XX-12, one in each state slice 5 can reach ([Fixes after review 1](#fixes-after-review-1)). `tests/fixtures.rs` loads every slice's files.

## Decisions worth knowing

- **Warnings go to stderr, and a note to stdout.** stdout stays a list that can be piped, but says on its own that something was left out. With `--json`, stderr stays empty and `skipped` holds the news.
- **A skipped file doesn't change the exit code.** The list worked. An agent reads `skipped`. §3 and §4.4 now say so.
- **A file whose name, `id` or `project` disagree is skipped too**, where it used to stop the list. §4.4's reason for one file per item is that a bad file costs one item, and these are bad files.
- **An unreadable `items/` folder or item file isn't skipped.** It exits 3, as §3 says for a store that can't be read. Skipping it would hide a whole store behind one warning.
- **`show` and every other command that names a corrupt item exit 3**, with the details `list` reports. Slice 4 left this to slice 5. There's no item to show or move.
- **`delete` still refuses a corrupt item.** Its record needs the title and state, which can't be read, and a delete can't be undone. The fix is to repair the file and then delete it, or move the file out of `items/` by hand, which leaves no line in `deleted.json` ([Still open](#still-open)).
- **The fix isn't in the JSON.** §5 tells agents never to edit the store, so the fix is for a person, and only the text has it.
- **`open` has no colour.** It's the default state, and the ones that need something from someone, review especially, stand out against it.
- **Colour respects `NO_COLOR` and `CLICOLOR_FORCE`**, because anstream does. The tests use `CLICOLOR_FORCE` to see colour without a terminal.
- **The heading row is indented two spaces**, to line up with the IDs under each project line, so the text starts with two spaces.
- **"No open items. Nice work!"** is PLAN.md's wording. It shows only when nothing is open, in progress or in review, and the footer under it says those are the states shown.
- **200 characters for a title**, for the reasons [above](#the-longest-title). §2.1 now says so.
- **A respelled value is rewritten, not refused.** Slice 2 review 2's item 4 asked for it, and slice 4 left the choice here. Refusing it blamed a hand edit on a newer tracker, and left an item that no move could change.

## Fixes after review 1

[Review 1](reviews/slice-5-review-1.md) found three bugs, one place where the code and DESIGN.md disagreed, and four test gaps. Each fix is its own commit, with its tests:

| Finding | Commit | What changed | Tests |
| --- | --- | --- | --- |
| Bug 1: a broken symlink, or a file deleted mid-listing, stopped the whole list and said to run `tracker init` | `e76eabb` | `load_items` skips a broken symlink and reports it, with "Remove it from items/.", and leaves out a file that's gone. A command that names it exits 3 with the same details, not "There's no item". The "Run `tracker init`" hint moved from `error::io` to `error::io_in_store`, used only for what `init` creates | `a_symlink_to_a_missing_file_is_skipped_and_named_without_suggesting_init` in `tests/bad_data.rs`, and `a_file_removed_after_listing_is_left_out_without_a_word` in `src/store.rs`, since no integration test can time the race |
| Bug 2: `TRACKER_HOME` naming a file gave "Not a directory" or "File exists" | `a99f678` | `Store::locate` refuses, with exit 3, a store folder that's a file or sits under one, and says to point `TRACKER_HOME` at a folder or move the file. The same for `~/.tracker` with no `TRACKER_HOME` | `a_store_folder_that_is_a_file_is_named_with_how_to_fix_it`: every command, `init` included, a folder inside a file, and `~/.tracker` |
| Bug 3: four common errors didn't say how to fix the problem | `f43eb23` | "There's no item XX-9." names the `list` that shows every item. An empty `--name` gives an example, a root that can't be used says to check the path or create the folder, and no `HOME` says to set `TRACKER_HOME` | `common_errors_say_how_to_fix_the_problem` in `tests/polish.rs`: the missing ID through all eight commands that take one, and the other three |
| Finding 4: DESIGN.md didn't say a listing that skips a file exits 0 | `8ab514c` | §3's exit codes and §4.4 say so, and §4.4 adds the broken symlink and the file deleted mid-listing | `tests/docs_links.rs` still passes |
| Test gap 1: `every_command_has_help_text` checked nothing per option under `--help` | `b1da69f` | Under `--help`, an option or argument must have its description beside it or, for a command with long help, on the next line, indented further. Seen failing with `--unlabel`'s description removed | `every_command_has_help_text` |
| Test gaps 2 and 3 | with bugs 1–3 | The tests above | |
| A flaky test, found while fixing the above: `lock_unchanged_holds_the_lock_only_while_the_file_is_as_it_was` failed once | `290eeed` | A lock is released when its file is closed, but a process that another test in the binary starts gets a copy of every open file, the lock's included, until its program runs. A probe measured it on macOS: with processes being started, about 1 drop in 100 left the lock held, for at most 0.2 ms, and none without. The test now retries the lock for up to 5 seconds after the drop, through `locks_within`. A tracker command has one thread, so this only affects tests | A temporary test that dropped the lock 3,000 times while another thread started processes failed 69 times with the old check, and passed with the new one. With the drop left out, the test still fails |
| Test gap 4: no fixture saved by slice 5 | `af02b3b` | `tests/fixtures/slice-5/`: XX-9 open, with every field and an edit with `fields`, `kind_before` and `labels_removed`; XX-10 in progress; XX-11 done and unchecked; XX-12 reopened from done, so with `previous_verification`, then shelved. Review can't be reached until slice 9 | `slice_5_wrote_four_fixtures_one_in_each_state`, and the four fixture tests that load, list and show every fixture |

## Still open

Everything here but the first group is carried over from [slice 4's readme](slice-4-readme.md#still-open).

**This slice**

- **Review 1 isn't independent.** It was written in the same session that built the slice, so PLAN.md's #adversarial-review matters more here.
- **A symlink to an item file that does exist is still followed** when read, as it has been since slice 1. A write replaces the link with a file, so nothing is written through it. Review 1 suggested skipping every symlink in `items/`, which would also stop a symlinked item from listing; only the broken ones are skipped for now.
- **The code and its tests landed as one commit** (`0dfa462`), as slices 2–4 did, though the earlier reviews asked for one commit per step. The colour module and the dependencies are their own commits.
- **`delete` can't remove a corrupt item file.** The file can be repaired and then deleted, or moved out of `items/` by hand, which leaves no line in `deleted.json` for slice 16's digest. A delete that records only the ID would need `deleted.schema.json` to allow a record without a title and state.
- **A temporary file left by a crash stays.** A command that dies between creating its temporary file and renaming it, such as one killed with `kill -9`, leaves `XX-1.json.4242.tmp` behind. Every listing ignores it, and the next write uses its own process ID, so nothing breaks, but nothing removes it either. `audit` (slice 16) could report it.
- **The terminal tests have only run on macOS** (slice 4's test gap 5). `a_terminal_gets_colour` and the `script` half of the temporary-files test join them.

**From slice 4** (reviews 1 and 2)

- **Review 1's bug 3. A title edited by hand can break `deleted.schema.json`**, in `delete --json` and the line in `deleted.json`.
- **Review 2's process point: `3880bf9`** put an unrelated change on slice 4's branch.

**Security** (slice 3 review 2)

- **3. Lookalike labels**, such as one with a Unicode hyphen that looks like `needs-review`.
- **4. Whether T9 keeps holding an item whose `needs-review` an edit removed**, for slice 9 to decide.

**Low severity** (slice 3 review 1)

- **4. Hand-edited labels aren't matched case-insensitively.**
- **5. Invisible characters are accepted in labels and titles.** Slice 4 suggested this slice for it. It's still open: `text::changes_display` doesn't count zero-width characters, and counting them would change what `check_printable` refuses, which deserves its own decision about which characters and its own tests.

**Test gaps** (slice 3)

- Hand-edited labels in the text list. (Two projects in the text list are now covered by `list_starts_with_a_heading_row_lined_up_with_the_rows_below_it`.)

**Process** (slices 2 and 3)

- **The `unchecked` note is still in the present tense.** It's cheapest to change before self-tracking starts after slice 6.

**Carried over from slice 2**

- **`status <id> closed` is refused** (slice 2 review 2, item 5).
- **Commands on an item whose project isn't in `config.json`** (item 7): the moves and `edit` still have no test.
- **`error.schema.json`'s `allowed[].via` enum lists only today's four moves** (item 3).
- **Concurrent moves on one item aren't tested**, nor a delete and a move at the same moment beyond the re-read test.
- **The text output of `shelve` and `reopen` isn't tested.**

Slice 2 review 2's item 4, a hand-edited timestamp blamed on a newer tracker, is settled by this slice ([above](#a-value-only-written-differently)).

## Limits, and what later slices change

- **Slice 6** adds `criterion`, `evidence` and `gates`. Each needs help text, which `every_command_and_flag_has_help_text` enforces, and `tests/polish.rs`'s `COMMANDS` list needs the new commands. `gates`' refusal is the first output with many reasons, and should colour each gate's name. `criterion accept` asks its question on stderr, uncoloured, like `delete`. New item fields need `show` lines through `styled_table`, and a file that holds a criterion this version can't parse is skipped like any other.
- **Slice 7** prints each check's command before running it, and its last lines of output. That output is a check's own text, so it must go through `Styled::push`, which escapes it, and never be added to a rendered string.
- **Slice 9**'s `review` inbox needs an empty state, which §3 gives: "Nothing needs you." `review` lists every project, so it should report skipped files as `list` does, through `skipped_warnings` and `skipped_json`.
- **Slice 16** brings `--help` and empty states up to this slice's standard for every command added since. `audit` reads every file, and should report the files `load_items` skips, and leftover temporary files, rather than skip them. Its "IDs used twice" can't see an ID held by a file that doesn't parse.
- **Item files change shape again later.** A field this version doesn't know still makes `update_item` refuse, as before, and a value only written differently is rewritten. A new field needs what the earlier readmes list: a serde default and `skip_serializing_if`, its place in §4.3's order, an entry in `item.schema.json`, and a fixtures folder.

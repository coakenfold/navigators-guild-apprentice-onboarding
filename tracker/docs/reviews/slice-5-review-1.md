# Slice 5 review 1

A review of slice 5 (Bad data and polish) against [PLAN.md](../PLAN.md)'s Slice 5 prompt and _Done when_ list, its integration risks, the [DESIGN.md](../DESIGN.md) sections the slice touches (§2.1's title, §3's exit codes and `--json` rule, §4.4's handling of bad data, and §8's colour), the conventions in [CLAUDE.md](../../CLAUDE.md), and the claims in [slice-5-readme.md](../slice-5-readme.md). It also checks what [slice 4 review 1](slice-4-review-1.md#slice-5-must-decide) and [slice 4 review 2](slice-4-review-2.md#slice-5-must-decide) left for slice 5 to decide.

|                 |                                                                                                                                                                                                                                                       |
| --------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Date            | 2026-11-17                                                                                                                                                                                                                                            |
| Branch          | `feat/tracker/slice-5` at `9765c91`, working tree clean. The slice is five commits on `main`'s `e3854b8`: `942f844` (anstream and anstyle), `a59f11f` (`Styled`), `0dfa462` (all the code and tests), `51958aa` (DESIGN.md §2.1 and §4.4) and `9765c91` (the readme and PLAN.md's status) |
| Method          | Read the diff against `main`, then every changed source, test and schema file in full. Ran the test command and `cargo clippy --all-targets -- -D warnings`. Reproduced each finding marked "Reproduced" against the debug binary in a temporary store. This review was written in the same session that built the slice, so it isn't an independent one ([Process](#process)) |
| Line references | Point at `9765c91`                                                                                                                                                                                                                                    |

## Verdict

Slice 5 does what the plan asks. Every _Done when_ check passes and has a test. `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and all 189 tests pass, 16 of them in the new [tests/bad_data.rs](../../tests/bad_data.rs) and [tests/polish.rs](../../tests/polish.rs) and 2 in `src/main.rs`. The readme's per-file test counts are right.

- **A corrupt item file costs one item.** `list` and `project list` skip it, and report its name, the parser's line and column, the reason and a fix, on stderr and in `--json`'s `skipped`. A command that names the item exits 3 with the same details. Files whose name, `id` or `project` disagree, which used to stop the whole list, are skipped the same way.
- **Colour can't be used to get round the escaping.** `Styled::render` escapes each piece before wrapping it in colour codes, so a title edited by hand stays escaped with colour on, and can't add colour of its own.
- **The four things slice 4's reviews left to this slice are settled.** `show` and `delete` refuse a corrupt item with its details, the question on stderr has no colour, the temporary-file test includes a delete typed at a terminal, and a move or `edit` now rewrites a value only written differently instead of blaming a newer tracker.

I found no bug that loses data, and no new security hole. The findings that matter most:

- **Bug 1:** one dangling symlink in `items/`, or an item deleted while `list` reads the folder, stops the whole list with exit 3, and tells me to run `tracker init`, which wouldn't help.
- **Bug 3:** "There's no item XX-9.", the error every ID command can give, still doesn't say what to do next, nor do three others. The prompt asks for messages "that say how to fix the problem".
- **Test gap 4:** PLAN.md's integration risks ask for a test of "an item file saved by slice 5", and there's none. Slice 6 is the next change to the item file.
- **Finding 4:** §3 says exit code 3 means "a file … is unreadable", but a `list` that skipped one exits 0, and DESIGN.md doesn't say so.

What was done about each finding, after the review, is under [Outcome](#outcome) at the end.

## Done when

| Check | Result | Tests |
| --- | --- | --- |
| A hand-corrupted item file is reported with its name, line and reason, and the other items still list | Pass | [bad_data.rs:40](../../tests/bad_data.rs#L40) (JSON, stderr, stdout's note, `project list`, `show` and `start` refusing, file untouched), [bad_data.rs:99](../../tests/bad_data.rs#L99), [bad_data.rs:116](../../tests/bad_data.rs#L116), and [item_files.rs:46](../../tests/item_files.rs#L46), [item_files.rs:74](../../tests/item_files.rs#L74), [item_files.rs:96](../../tests/item_files.rs#L96). But see bug 1 |
| With no store, commands say "Run `tracker init`." and never panic | Pass | [bad_data.rs:165](../../tests/bad_data.rs#L165), every command as JSON and as text, and [bad_data.rs:186](../../tests/bad_data.rs#L186) for a missing `config.json` alone |
| A store with `chmod 000` exits 3 | Pass | [bad_data.rs:230](../../tests/bad_data.rs#L230), every command with `init`, and [bad_data.rs:254](../../tests/bad_data.rs#L254) for `items/`. Neither skipped on macOS |
| No temporary files are left in the store after any command | Pass | [bad_data.rs:293](../../tests/bad_data.rs#L293), after every command, refusals, a delete typed under `script`, and a `create` that can't write |
| Every command has `--help` text | Pass | [polish.rs:39](../../tests/polish.rs#L39) for all 15 commands, and [main.rs:1912](../../src/main.rs#L1912), which walks clap's tree for every command and flag. See test gap 1 |
| `tracker list \| cat` contains no colour codes | Pass | [polish.rs:115](../../tests/polish.rs#L115), literally through `sh`, plus five other commands' stdout and stderr. [polish.rs:137](../../tests/polish.rs#L137) and [polish.rs:175](../../tests/polish.rs#L175) show the colour is there to turn off |
| An empty list prints its empty-state message | Pass | [polish.rs:214](../../tests/polish.rs#L214), all four messages |
| `list`'s text starts with a heading row that names each column, lined up with the rows below it | Pass | [polish.rs:256](../../tests/polish.rs#L256), by terminal column with wide characters, and [organise.rs:764](../../tests/organise.rs#L764) for the exact lines |
| _Prompt:_ every write goes to a temporary file that is then renamed | Pass | Unchanged since slice 1: every write goes through `store::write_file`. [tests/store_writes.rs](../../tests/store_writes.rs) |
| _Prompt:_ error messages that say how to fix the problem | Partly | Store, config and counters errors now do ([error.rs:43-54](../../src/error.rs#L43-L54)). Four common ones still don't (bug 3), and two new ones point the wrong way (bugs 1 and 2) |
| _Prompt:_ decide whether a title needs a length limit | Pass | 200 characters, recorded in §2.1. [polish.rs:307](../../tests/polish.rs#L307) |

## Bugs

1. **A dangling symlink, or an item deleted mid-listing, stops the whole list, and the hint says to run `tracker init`.** `load_items` returns an error for any read failure other than invalid UTF-8 ([store.rs:516-523](../../src/store.rs#L516-L523)), and `error::io` adds "Run `tracker init` to put back what's missing." to every "not found" ([error.rs:50](../../src/error.rs#L50)).

   Reproduced with `ln -s $D/nowhere.json $TRACKER_HOME/items/XX-7.json`. `tracker list` and `list --json` exit 3 with "Can't read …/items/XX-7.json: No such file or directory (os error 2). Run `tracker init` to put back what's missing." `init` doesn't create item files, so the hint can't help.

   The same path is reached without a hand edit. `list` doesn't take the lock, so a `delete` that removes a file between `read_dir` and the read makes the list fail the same way. (Found by reading the code. The window is small.) §4.4's whole point is that one file costs one item, not the listing.

   Fix: in `load_items`, a file that's gone by the time it's read is skipped silently, since it was deleted, and a symlink whose target is missing (`fs::symlink_metadata` succeeds, `fs::metadata` doesn't) is skipped and reported, with "it's a symlink to a file that doesn't exist" and "Remove it from items/". Give `error::io`'s "Run `tracker init`" hint only for the parts `init` creates, by passing it in from `require_exists` and `write_file` rather than adding it to every "not found".

2. **When `TRACKER_HOME` names a file, the messages don't say so or how to fix it.** Reproduced with `TRACKER_HOME=$D/f`, a plain file. `tracker list` exits 3 with "Can't read $D/f/config.json: Not a directory (os error 20)." `tracker init` exits 3 with "Can't create $D/f: File exists (os error 17).", which reads as if the problem were that something exists. Fix: in `Store::locate` or at the top of `init` and `require_exists`, check that the store's path, if it exists, is a folder, and say "TRACKER_HOME names $D/f, which is a file, not a folder. Point TRACKER_HOME at a folder, or move the file." Exit 3, since it's the store that can't be used.

3. **Four common errors still don't say how to fix the problem.** Reproduced:

   | Message | Where | A fix it could give |
   | --- | --- | --- |
   | "There's no item XX-9." | [store.rs:356](../../src/store.rs#L356), [store.rs:452](../../src/store.rs#L452), [store.rs:482](../../src/store.rs#L482), for `show`, `edit`, every move and `delete` | "`tracker list --all --status open,in-progress,review,done,shelved` lists every item." |
   | "A project's --name can't be empty." | [main.rs:485](../../src/main.rs#L485) | "Give it a readable name, such as --name bookmarks." |
   | "Can't use /nope as a project root: No such file or directory (os error 2)" | [main.rs:492](../../src/main.rs#L492) | "Create the folder first, or check the path." |
   | "Neither TRACKER_HOME nor HOME is set, so there's no store to use." | [store.rs:51](../../src/store.rs#L51) | "Set TRACKER_HOME to the store's folder, such as ~/.tracker." |

   The first is the one an agent will see most, since §5 has it pick up work with `tracker show <ID>`. The prompt asks for "error messages that say how to fix the problem", and the readme's "Every I/O error says what to do" is true, but these aren't I/O errors. "Can't read the current folder: {e}" ([main.rs:720](../../src/main.rs#L720) and two more) is the same, but rare enough to leave.

## Security

No new holes. I checked:

- **Colour and escaping.** Each piece is escaped before its codes go round it ([styled.rs:53-64](../../src/styled.rs#L53-L64)), and [polish.rs:161](../../tests/polish.rs#L161) shows a title holding `\u001b[31m` stays escaped with colour forced on.
- **Skipped file names.** A file named `XX-9\e[31m.json` in `items/` is reported as `XX-9\u{1b}[31m.json` on stderr and `XX-9\u001b[31m.json` in the JSON. Reproduced. The parser's reason, which can quote the file's content, goes through the same escaping.
- **`NO_COLOR`** beats `CLICOLOR_FORCE`: with both set, `list` printed no colour codes. Reproduced.
- **A symlinked item file is followed when read.** With `items/XX-8.json` a symlink to a file outside the store, `list` shows it. Reproduced. That's been true since slice 1, and a write replaces the symlink with a file through `rename`, so nothing is written through it. It isn't a new hole, but bug 1's fix should look at symlinks as a whole: skipping every symlink in `items/` with a report would be simpler than handling the broken ones only.

## Where the code and DESIGN.md disagree, or the design is unclear

4. **DESIGN.md doesn't say what exit code a listing that skipped a file gives.** §3 says exit code 3 means "the store, a file or git is unreadable". `list` with a corrupt file exits 0 ([main.rs](../../src/main.rs)'s `list` returns `Ok`), which is what §4.4's "the other items still list" suggests, and the readme says so under Decisions. But `51958aa`'s new §4.4 text doesn't, so an agent reading §3 could expect exit 3, and an agent that checks only the exit code never learns that an item is missing. Fix: add to §4.4 "The listing still exits 0, so an agent reads `skipped`", and to §3's exit-code paragraph "A listing that skips a file it can't parse still exits 0 (§4.4)".

## Test gaps

1. **`every_command_has_help_text` checks nothing per option under `--help`.** [polish.rs:54](../../tests/polish.rs#L54) sets `long_help_below` to true for `--help`, which makes the assertion on the next line always pass. Only the `-h` pass checks that each option has a description. The unit test at [main.rs:1912](../../src/main.rs#L1912) covers every flag, so nothing is uncovered, but the integration test claims more than it checks. Either drop the per-option loop for `--help`, or check there that each option's line is followed by an indented description.
2. **No test covers an item file that disappears or is a broken symlink while listing** (bug 1).
3. **No test covers `TRACKER_HOME` naming a file** (bug 2), nor the four messages in bug 3.
4. **There's no fixture for an item file saved by slice 5.** PLAN.md's integration risks say "Keep slice 1's test that a file comes back byte-for-byte unchanged, and add one for an item file saved by slice 5." The readme says no fixture was added because the item file's shape didn't change, which follows [tests/fixtures.rs](../../tests/fixtures.rs)'s own rule, but not PLAN.md's. Slice 5 is the course submission and the last shape before slice 6 adds `criteria` and `written`, so a `tests/fixtures/slice-5/` folder is the record the plan asks for. Add one with an item in each state, including an `edit` entry with `fields`, `kind_before` and `labels_removed` and a `previous_verification`, written by this binary, and add a test like `slice_2_wrote_three_fixtures_with_history`.

## Process

- **The code and its tests landed as one commit** (`0dfa462`, 1,568 lines added across 12 files), and the tests were written after the code. The readme says both. Its "Seen failing" list holds: each of the four deliberate breaks was run in this session against the same code before it was committed, and failed the tests the readme names. The colour module and the dependencies are their own commits, which is better than slices 2–4.
- **This review isn't independent.** It was written in the same session, by the same agent, that built the slice. The PLAN.md checkpoint sends slices 1–5 to #adversarial-review, and that review matters more for this slice than the earlier ones did.
- **The readme is accurate**, apart from what bug 1 changes: "for a missing file, to run `tracker init`" ([How it works: Error messages](../slice-5-readme.md#error-messages)) describes the hint bug 1 shows is wrong for item files. Its test counts, commits, Try it output and Still open list match the code and `git log`.

## Still open, carried forward

There's no earlier review of slice 5. The readme's [Still open](../slice-5-readme.md#still-open) carries slice 4's open items, and I checked that none of them changed on this branch except these two:

- **Slice 2 review 2's item 4, a hand-edited timestamp blamed on a newer tracker**, is closed. [store.rs:447-466](../../src/store.rs#L447-L466) refuses only unknown fields, and [item_files.rs:170](../../tests/item_files.rs#L170) tests a move through `edit` on a file with `+00:00` and `"labels": []`.
- **Slice 3 review 1's finding 5, invisible characters**, which slice 4's readme suggested for this slice, is still open, and the readme says why.

## Slice 6 must decide

- **G1 will break this slice's tests too.** PLAN.md's slice 6 prompt names `tests/states.rs`, whose items start with no criteria. So do `busy_store` in [polish.rs:67](../../tests/polish.rs#L67), which starts XX-2 and XX-3, and `every_command` in [bad_data.rs:143](../../tests/bad_data.rs#L143), which starts XX-1. The shared helper that gives items a criterion should cover these. Otherwise the colour, chmod and temporary-file tests will stop testing what they're named for, and fail on a G1 refusal instead.
- **How a refusal of several lines is printed.** §2.4's gate refusal is several lines, and exits 2. Today every error goes through `report` ([main.rs:418](../../src/main.rs#L418)), which escapes line breaks so one error is one line. Slice 6 has to choose between printing the refusal as the command's output, with exit 2, or letting `report` keep the line breaks of a message tracker wrote itself.
- **Help, colour and empty states for the new commands.** `criterion`, `evidence` and `gates` need help text, which [main.rs:1912](../../src/main.rs#L1912) enforces, and entries in [polish.rs](../../tests/polish.rs)'s `COMMANDS`, which nothing enforces. `criterion accept`'s question should stay uncoloured on stderr, like `delete`'s.
- **The fixture.** If test gap 4 isn't fixed here, slice 6 should save `tests/fixtures/slice-5/` with this binary before it changes the item file.

## Outcome

Added after the fixes. The findings above are left as written, so their line links point at `9765c91`, not at the fixed code. Each fix is in its own commit, with its tests, and the readme's [Fixes after review 1](../slice-5-readme.md#fixes-after-review-1) lists them too.

Every finding is resolved: all three bugs, finding 4 and all four test gaps. The commits after this review (`2b0a767`), in order:

- **Fixes:** `e76eabb` (bug 1), `a99f678` (bug 2), `f43eb23` (bug 3), `8ab514c` (finding 4), `b1da69f` (test gap 1) and `af02b3b` (test gap 4). Test gaps 2 and 3 were fixed by the tests that came with bugs 1–3.
- **Docs:** `3e1824a` (the readme) and `0a91129` (this Outcome).
- **The flaky lock test**, found while making the fixes: `290eeed` (the fix) and `90fad8d` (its docs).

| Finding | Resolved by |
| --- | --- |
| Bug 1: a broken symlink, or a file deleted mid-listing, stops the whole list and suggests `tracker init` | `e76eabb`: `load_items` reads each file through `read_listed`, which skips and reports a broken symlink ("it's a symlink to a file that doesn't exist", "Remove it from items/.") and leaves out a file that's gone. `item_text`, which `load_item`, `load_item_with_text` and `update_item` now all read through, refuses a broken symlink with exit 3 rather than "There's no item". `error::io` no longer adds "Run `tracker init`"; `error::io_in_store` does, and is used only for the parts `init` creates. Tests: `a_symlink_to_a_missing_file_is_skipped_and_named_without_suggesting_init`, and the unit test `a_file_removed_after_listing_is_left_out_without_a_word`, since no integration test can time the race |
| Bug 2: `TRACKER_HOME` naming a file | `a99f678`: `Store::locate` calls `check_not_a_file`, which refuses with exit 3 a store folder that's a file, or that has a file above it, naming the file, for `TRACKER_HOME` and for `~/.tracker`. Test: `a_store_folder_that_is_a_file_is_named_with_how_to_fix_it`, for every command, `init` included |
| Bug 3: four errors that don't say how to fix the problem | `f43eb23`: each now ends with the fix the review suggested. The project root's says "Check the path, or create the folder first.", since the error can be other than "not found". No `HOME` gives an absolute example, `/Users/me/.tracker`, since a relative `TRACKER_HOME` is refused. Test: `common_errors_say_how_to_fix_the_problem`, with the missing ID through all eight commands that take one |
| Finding 4: the exit code of a listing that skipped a file | `8ab514c`: §3 and §4.4 say it exits 0 and that an agent reads `skipped`. §4.4 also names the broken symlink and the file deleted mid-listing |
| Test gap 1: `--help`'s per-option check always passed | `b1da69f`: under `--help`, each option and argument line must have its description beside it, or on the next line indented further. Arguments (`<ID>`) are now checked too |
| Test gap 2: no test of a vanished file or a broken symlink | `e76eabb`, with bug 1 |
| Test gap 3: no test of `TRACKER_HOME` naming a file, or of bug 3's messages | `a99f678` and `f43eb23`, with bugs 2 and 3 |
| Test gap 4: no fixture saved by slice 5 | `af02b3b`: `tests/fixtures/slice-5/`, XX-9 to XX-12, written by this binary: one open with every field and an edit with `fields`, `kind_before` and `labels_removed`, one in progress, one done and one shelved after a reopen, with `previous_verification`. Review can't be reached before slice 9. Test: `slice_5_wrote_four_fixtures_one_in_each_state`. This also settles the last point of [Slice 6 must decide](#slice-6-must-decide) |
| _Done when_: the prompt's "error messages that say how to fix the problem", marked Partly | Pass now, with bugs 1–3: the hint that pointed the wrong way is gone, and the four common errors say what to do |
| Security: symlinks in `items/` as a whole | Not taken. Only broken symlinks are skipped. Skipping every symlink would also hide a symlinked item that works, and a write already replaces a symlink with a file. It's in the readme's [Still open](../slice-5-readme.md#still-open) |
| Process: the readme's "for a missing file, to run `tracker init`" | `3e1824a`: the readme describes `error::io_in_store`, and has the new test counts, commits, fixtures and a [Fixes after review 1](../slice-5-readme.md#fixes-after-review-1) section. The review not being independent stays in its Still open |
| Not a finding: `lock_unchanged_holds_the_lock_only_while_the_file_is_as_it_was` failed once in a full `cargo test` | `290eeed`: the test now retries the lock for up to 5 seconds after dropping it, through `locks_within`. The cause: a process that another test in the binary starts holds a copy of the lock's file until its program runs. A probe measured it on macOS: with processes being started, about 1 drop in 100 left the lock held, for at most 0.2 ms, and none without. A tracker command has one thread, so the lock itself was never wrong |

Seen failing:

- **Bugs 1–3:** bug 1's integration test was run against the source before `e76eabb`, bug 2's against `e76eabb` and bug 3's against `a99f678`, and each failed. Bug 1's unit test calls the new `read_listed`, so it can't build against the old code.
- **Test gap 1:** its check was run with `--unlabel`'s description removed from `src/main.rs`, and failed under `--help`, which it couldn't before.
- **The lock test:** a temporary test dropped the real `lock_unchanged`'s lock 3,000 times while another thread started processes. The old check failed 69 times and the new one never did. With the drop left out, the fixed test fails.

After all of this, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and all 194 tests pass, 5 more than at `9765c91`: 2 in `tests/bad_data.rs`, 1 each in `tests/polish.rs` and `tests/fixtures.rs`, and 1 unit test in `src/store.rs`.

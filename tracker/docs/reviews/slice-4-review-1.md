# Slice 4 review 1

A review of slice 4 (Show and delete) against [PLAN.md](../PLAN.md)'s Slice 4 prompt and _Done when_ list, the [DESIGN.md](../DESIGN.md) sections it touches (§2.7's shared confirmation, §2.8's delete, §3's `show` and `delete` rows and its rule that ID commands need no project, §4.1's `deleted.json`, §4.4's lock, re-read and counter tripwire), the conventions in [CLAUDE.md](../../CLAUDE.md), and the claims in [slice-4-readme.md](../slice-4-readme.md).

|                 |                                                                                                                                                                                                        |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Date            | 2026-11-17                                                                                                                                                                                             |
| Branch          | `feat/tracker/slice-4` at `75ba405`, working tree clean. The slice is `c980317` (`feat: slice 4 first pass`). `bf229d0` and `1d2f64e` are slice 3's fixes, and `c1a08b2` and `75ba405` are docs |
| Method          | Read the diff against `main`, the code, schemas and tests. Ran the test command. Reproduced each finding marked "Reproduced" against the debug binary in a temporary store, typing answers through `script` |
| Line references | Point at `75ba405`                                                                                                                                                                                     |

## Verdict

Slice 4 does what the plan asks. Every _Done when_ check passes and has a test, and the readme records that the by-hand check was done in a real terminal. `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and all 158 tests pass, 15 of them new in [tests/show_and_delete.rs](../../tests/show_and_delete.rs). The readme's test counts are correct.

- **`show`** prints every field the item has, both verification records, and the history with each entry's time, command, move, terminal flag and details. Free text is escaped in every place it's printed.
- **`delete`** works in every state and only from a terminal. It says what will be lost, asks for the ID, then re-reads the file under the lock and refuses if anything changed. It writes the `deleted.json` line before it removes the file, and takes the line back out if the removal fails. I checked that rollback by hand in both cases, with an existing `deleted.json` and without one.
- **`tty::confirm`** is the one shared helper the prompt asks for. It uses `tty::is_interactive()`, so the confirmation and history's `interactive` flag always agree.
- **The counter tripwire** now reads `deleted.json` as well as `items/`, which closes a real gap: before this, deleting a project's newest item removed the only evidence of its number.
- **Slice 3's two fixes** (`bf229d0`, `1d2f64e`) are each in their own commit, each with a test seen failing first.

I found no bug that loses data, and no new security hole. The findings are:

- `show --json` doesn't print "the item exactly as its file holds it", as the readme, the schema and a test name claim (finding 1).
- `delete`'s refusal tells an agent to shelve items that can't be shelved (finding 2).
- The test harness hangs rather than fails if a prompt never appears (test gap 1).
- `tty::confirm` leaves the re-read under the lock to each caller, which is the step slice 9's `approve` most needs to get right (finding 9).

## Outcome

Added after the fixes. The findings below are left as written, so their line links point at `75ba405`, not at the fixed code. Each fix is in its own commit, and the readme's [Fixes after review 1](../slice-4-readme.md#fixes-after-review-1) lists them too.

Seen failing: the tests for bugs 1, 2, 4 and 5 were each run with their fix commit's tests and its parent's `src/` and `schemas/`, and each failed. Test gap 2's test failed with `delete_item`'s rollback removed. Test gap 1's test wasn't run against the old harness, which would hang rather than fail. Test gaps 4 and 6 pin behaviour that was already right, so they pass on the old code by design.

After the fixes, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and all 166 tests pass, 8 more than at `75ba405`.

| Finding | Resolved by |
| --- | --- |
| Bug 1: `show --json` isn't "the item exactly as its file holds it" | `6c1a831`: option (a). `show` calls `item::lost_on_rewrite` and names what it can't show, at the end of the text and as `warning` beside `item`, which `show.schema.json` now allows. The schema's description, the readme and the test name (now `show_json_is_the_item_as_its_file_holds_it_when_this_version_wrote_it`) no longer claim exactness. `Store::load_item_with_text` reads the item and its text in one read. Tests: `show_names_the_fields_of_a_file_from_a_newer_tracker`, and `show_prints_an_item_whose_history_has_a_command_it_doesnt_know` now checks the warning |
| Bug 2: the refusal tells an agent to shelve items that can't be shelved | `ca0d1b3`: the shelve advice is given only where `states::find` draws a shelve, for open and in-progress items. Review, done and shelved items get "Only a human deletes items, so a human decides whether to delete XX-3, which is done." Test: `delete_refused_without_a_terminal_suggests_shelve_only_where_it_works` |
| Bug 3: a title with a line break breaks `deleted.schema.json` | Still open. Not taken in this round |
| Bug 4: the warning calls an unchecked done item "verified" | `9ed9576`: "done at <time>, unchecked: no gates ran". An `auto` record reads "verified by its gates at", and a `human` one "approved by a human at" |
| Bug 5: the warning understates what's lost in a newer tracker's file | `bef1a78`: the second option. `delete` exits 3 before asking, naming the fields, as `update_item` does for a rewrite, since a newer tracker may have more to remove, such as slice 8's ref. Test: `delete_refuses_a_file_from_a_newer_tracker_before_asking` |
| Security 6: the readme's `script` recipe | `41a1197`: no code change. §7 and the readme's Try it say the recipe is the bypass §7 describes, and that only the digest tells a delete made through it from yours |
| Security 7: a forged `deleted.json` line blocks `create` | `f7dd832`: no code change. DESIGN.md §3's `audit` row and PLAN.md's slice 16 _Done when_ have `audit` report a `deleted.json` ID at or above its project's counter |
| Design 8: §2.8's example is out of date | `fc24732`: the example shows what `delete` prints, with the title, "deleted.json records the delete for the digest" and the shared question. Its commit and `refs/tracker/BM-12` stay, for slice 8 |
| Design 9: `tty::confirm` doesn't own the re-read under the lock | `5a51de2`: `Store::lock_unchanged(id, before, command)` takes the lock and refuses with exit 1 if the item was removed or changed. `delete` calls it. Test: `lock_unchanged_holds_the_lock_only_while_the_file_is_as_it_was`, which also covers an item removed while the question waits |
| Design 10: what `digest ack` asks for | `44b2d11`: it asks for `ack`, not the project key. Under the lock it builds the digest again and, if that differs from the one shown, records nothing and exits 1. §2.7, `tty::confirm`'s comment, and PLAN.md's slice 16 prompt and _Done when_ say so. `8799c74` records it in the readme |
| Design 11: `show` needs a `config.json` that parses | `1fcdeec`: kept as it is. `show` exits 3 on a `config.json` that can't be parsed, like every command. §3 says so. `2c1a976` names the commit in the readme |
| Test gap 1: the terminal harness hangs | `ef82659`: `typing` reads `script`'s output on a thread and gives up after `TERMINAL_TIMEOUT` (20 seconds), killing the command and panicking with the output so far. Test: `the_terminal_harness_fails_rather_than_hangs_when_the_prompt_never_comes` |
| Test gap 2: `delete_item`'s rollback | `b44b843`: `a_delete_whose_file_cant_be_removed_takes_its_record_back_out`, Unix only, with `items/` set to `chmod 500`, both with and without an earlier `deleted.json`. It skips where permissions aren't enforced, as for root |
| Test gap 3: a newer tracker's file in `show --json` and `delete` | `6c1a831` and `bef1a78`: the tests for bugs 1 and 5, each with a `criteria` field |
| Test gap 4: a cancelled delete with `--json` | `ce27825`: `delete_json_in_a_terminal_cancelled_prints_only_the_error_on_stdout` |
| Test gap 5: the terminal tests have only run on macOS | Still open |
| Test gap 6: `show` of an item whose project isn't in `config.json` | `96ee241`: `show_and_delete_work_on_an_item_whose_project_isnt_in_config_json`, which also deletes one |
| Process: the readme is accurate apart from finding 1 | `6c1a831` corrects finding 1's claims. `8ab0809` adds the readme's Fixes after review 1 section and updates its Status row and test counts |
| Process: the slice is one commit | Unchanged. This round's fixes are one commit each |
| Process: PLAN.md marks slice 4 `done` before this review | Still open |
| Slice 5 must decide | Unchanged, for slice 5 |

## Done when

| Check | Result | Tests |
| --- | --- | --- |
| `show` of a missing ID exits 1 | Pass | [show_and_delete.rs:59](../../tests/show_and_delete.rs#L59), which also covers a non-ID (`../config`), the text form and a lower-case ID |
| `echo XX-1 \| tracker delete XX-1` is refused and the file remains | Pass | [show_and_delete.rs:206](../../tests/show_and_delete.rs#L206), as text and as JSON, and with empty stdin. The file stays byte for byte and no `deleted.json` is written |
| By hand in a terminal, for an item in each state: the prompt names the state and what will be lost, typing the ID deletes it, and a wrong ID cancels | Pass | By a person, per the readme's [Checks done by hand](../slice-4-readme.md#checks-done-by-hand). Also automated under `script`: [show_and_delete.rs:254](../../tests/show_and_delete.rs#L254) (all five states, with review reached by a hand edit) and [show_and_delete.rs:330](../../tests/show_and_delete.rs#L330) (another ID, extra words, an empty line, then the ID in lower case) |
| Deleting `XX-2` and then creating an item gives `XX-3` | Pass | [show_and_delete.rs:364](../../tests/show_and_delete.rs#L364) |
| A delete leaves one line in `deleted.json` with the ID, title, state, time and `interactive`, and nothing else of the item | Pass | [show_and_delete.rs:377](../../tests/show_and_delete.rs#L377), which checks the key order, that no description, label, design or history text is in the file, and that `items/` is empty |
| _Prompt:_ `show` prints every field, the verification record, and history with timestamps | Pass | [show_and_delete.rs:90](../../tests/show_and_delete.rs#L90), [show_and_delete.rs:171](../../tests/show_and_delete.rs#L171). Compared with [item.rs:11-39](../../src/item.rs#L11-L39), no field is missing from the text |
| _Prompt:_ one shared helper that refuses unless `tty::is_interactive()`, prints what will be lost, and requires the ID typed back | Pass | [tty.rs:37-86](../../src/tty.rs#L37-L86), exercised by every delete test |
| _Prompt:_ `delete` adds its line to `deleted.json` as §2.8 describes | Pass | As above. The line is compact JSON passed through `text::for_json`, so a title holding U+2028 can't split it |
| _Integration risk:_ files from earlier slices still load | Pass | [fixtures.rs](../../tests/fixtures.rs) now also runs `show` on every slice 1–3 fixture |

Also checked by hand in a temporary store:

- With `items/` set to `chmod 500`, `delete` exits 3 with "Nothing was deleted." It leaves `deleted.json` as it was. If there was no `deleted.json` before, it removes the one it just created.
- A corrupt item file can't be deleted (exit 3). The readme says so and leaves it to slice 5 (see [Slice 5 must decide](#slice-5-must-decide)).
- A `deleted.json` line naming `XX-50` makes `create --project XX` refuse with exit 3 and "at least 51".

## Bugs

1. **`show --json` isn't "the item exactly as its file holds it."** [main.rs:1449](../../src/main.rs#L1449) serializes the parsed `Item` struct, not the file. The two differ whenever the file holds something this version doesn't know:
   - a top-level field it doesn't know is dropped
   - a history `via` it doesn't know is printed as `"unknown"`

   Reproduced: with `criteria` added to XX-1's file and a history entry with `"via": "approve"`, `show XX-1 --json` has no `criteria` key, and the entry's `via` is `"unknown"`. The text form has the same gap and gives no hint that anything was left out.

   Three places claim exactness: the readme ("`--json` prints the item exactly as its file holds it"), [show.schema.json](../../schemas/show.schema.json)'s description of `item`, and the test name `show_json_is_the_item_exactly_as_its_file_holds_it`. That test only covers a file this version wrote, where the two forms always match. Today this matters only if an older binary reads a newer file. But §5 makes `tracker show <ID>` the agent's way to pick up work, item files change shape in slices 6, 7, 8, 9 and 12, and [`item::lost_on_rewrite`](../../src/item.rs#L305-L317) already finds exactly these fields. Either:
   - **(a)** call `lost_on_rewrite` in `show` and say what this version can't show, in the text ("This file has fields this version of tracker doesn't know: criteria. Use a newer tracker to see them.") and in the JSON (a `warning` beside `item`, which §3's wrapping exists for), or
   - **(b)** keep the behaviour and correct the three claims to "the item as this version reads it".

   I'd do (a). It costs a few lines and reuses a helper that `update_item` already trusts.

2. **`delete`'s refusal tells an agent to shelve items that can't be shelved.** The `otherwise` text at [main.rs:1484-1488](../../src/main.rs#L1484-L1488) says "An item that shouldn't be done is shelved instead, with `tracker shelve <ID> --reason "…"`" whatever the state. Reproduced: `echo XX-1 | tracker delete XX-1` on a done item gives that advice, and following it fails with "XX-1 is done, and `shelve` only moves an item that is open or in-progress" (exit 1). An item in review is refused the same way (§2.2: "I reject it first"), and an item that's already shelved is told to shelve itself. §5 tells agents to act on these messages, so wrong advice costs an agent a refused call and a guess. Give the advice only for open and in-progress items. For review, done and shelved items, say that a human decides whether to delete it, and stop there.

3. **`delete --json`, and the `deleted.json` line, can break `deleted.schema.json`.** [deleted.schema.json](../../schemas/deleted.schema.json) takes its `title` rule from `item.schema.json` through a `$ref`, and that rule forbids a line break. A title edited by hand can hold one. Reproduced: with XX-2's title set to `"two\nlines"`, `delete XX-2 --json` exits 0 and prints `"title": "two\nlines"`, which fails the schema, and the same line is now permanent in `deleted.json`. This is slice 3 review 1's finding 4 in a new place: the item's own `--json` already breaks `item.schema.json` in this case. It matters more here because nothing can ever correct a line in `deleted.json`, and slice 16's digest will validate or parse every line. I'd loosen the record's `title` to any non-empty string, since it records what was there and isn't input, and say so in the schema's description.

4. **The warning for an unchecked done item says it was "verified".** [main.rs:1542](../../src/main.rs#L1542) prints `done, verified unchecked at <time>`. With `by: "unchecked"` nothing verified the item, and this is the last thing I read before a delete that can't be undone. Something like `done at <time>, unchecked: no gates ran` would read correctly. It ties into the open item about the `unchecked` note's tense, and both are cheapest to change before self-tracking starts after slice 6.

5. **`delete`'s warning understates what's lost in a file from a newer tracker.** It has the same cause as finding 1: [`deletion_warning`](../../src/main.rs#L1538-L1566) lists only the fields this version knows. Reproduced: a done item whose file also holds `criteria` is described as "removes its file and its history (1 entry)". Once slice 8 lands, an older binary would also delete a newer item without removing its `refs/tracker/<ID>`. PLAN.md's bootstrap rule (install between items, never roll back) makes this unlikely. The fix for finding 1 covers it: name the unknown fields in the warning, or refuse with exit 3, as `update_item` does, and point to the newer tracker.

## Security

No new holes. These are the points I checked:

- **Paths.** The ID goes through `normalize_id` and then `item::is_valid_id` in [`Store::item_path`](../../src/store.rs#L79-L87), so `delete ../config` is refused (tested). If an item file is a symlink, `remove_file` removes the link, not its target.
- **Writes.** `deleted.json` is written through `write_file`, which since slice 3 uses a temporary file named with the process ID and opened with `create_new`. It can't be redirected through a planted symlink.
- **Terminal output.** The warning goes through `text::for_terminal`, and the title inside it is escaped on its own, so a line break in a hand-edited title can't add a line to the warning. The typed answer, when repeated in "Cancelled: "…" isn't XX-1", is escaped by `fail`. `show` escapes every cell and detail ([show_and_delete.rs:183](../../tests/show_and_delete.rs#L183)). Zero-width characters still get through, which is slice 3 review 1's open finding 5.
- **Races.** The lock isn't held while I decide. The re-read compares the file's whole text, which was read before the item was parsed, so any change in between can only cause a refusal ([show_and_delete.rs:441](../../tests/show_and_delete.rs#L441)).

6. **The readme carries a copy-paste recipe for getting past the terminal check.** [Try it](../slice-4-readme.md#try-it) shows `(sleep 1; echo BM-2; sleep 1) | script -q /dev/null tracker delete BM-2`, and the test harness does the same. This is the limit §7 already admits. The technique is well known, and the demo needs it. But TR's agents work in this folder from slice 7 on and will read this readme. A delete made this way records `"interactive": true` and looks exactly like mine, so the only defence is slice 16's digest, which I have to read. Nothing needs to change in the code. I'd add one line to §7, or to the readme beside the demo, saying that the recipe is the bypass §7 describes, and that a delete made through it is told apart from mine only by my checking the digest.

7. **A forged `deleted.json` line blocks `create` for its project.** [`highest_number_used`](../../src/store.rs#L256-L291) trusts every line it can parse. An agent that appends `{"id":"XX-999999",…}` makes every `create --project XX` exit 3 until I raise the counter. That's no worse than editing `counters.json` directly, which the agent could also do, and the message says how to recover. I'm noting it only so that slice 16's `audit` knows to check that each `deleted.json` ID is at or below its project's counter.

## Where the code and DESIGN.md disagree, or the design is unclear

8. **§2.8's example is out of date.** It still shows `Type BM-12 to delete:` and "the next digest lists the delete". The code asks `Type BM-12 to confirm, or anything else to cancel:` and says `deleted.json records the delete for the digest`. The readme lists these differences, and §2.7 was updated for the shared confirmation, but §2.8 wasn't. DESIGN.md is the spec, so change the example to what the code prints. Leave its "verified at a41be07" and `refs/tracker/BM-12` lines, which slice 8 delivers.

9. **`tty::confirm` doesn't own the re-read under the lock.** The plan's helper "refuses …, prints what will be lost, and requires the ID", and `confirm` does all three. But §2.7 now says the shared confirmation also re-reads the item under the lock and does nothing if it changed. In the code, that step is 15 lines in `delete` ([main.rs:1496-1513](../../src/main.rs#L1496-L1513)), and the readme's "To add a human-only command" asks every later caller to copy it. It's the step most likely to be got wrong, and slice 9's `approve` runs the gates between the question and the write. A small helper would keep it in one place. For example, `Store::lock_unchanged(id, &before) -> Result<File>` would take the lock, re-read the text, return the lock guard, and give the "removed" and "changed" errors that `delete` gives today. `criterion accept`, `approve` and `reject` would then each call it in one line.

10. **What does `digest ack` ask me to type?** §2.7 says each human action is confirmed "by typing the item ID", and `confirm`'s comment lists `digest ack` as a future user. But `digest ack` acts on a project, not an item, and has no item file to re-read. Decide now, while the helper is new, that it asks for the project key, and how a changed digest is detected under the lock (perhaps by comparing what the warning listed with what's there now). Then say so in §2.7.

11. **`show` needs a `config.json` that parses, though it only reads a project's name.** §3 says a command that takes an item ID "needs no project". `show` already works when the item's project isn't registered, but [main.rs:1345](../../src/main.rs#L1345) exits 3 if `config.json` can't be parsed (reproduced). Every other command does the same today, so this is consistent. But `show` is read-only, and it's what I'd reach for while repairing a broken store. Slice 5's bad-data work could have `show` print "(config.json can't be read)" in place of the project name. The readme records the current behaviour as a decision.

## Test gaps

1. **The terminal harness hangs rather than fails.** In [`Env::typing`](../../tests/common/mod.rs#L180-L225), the loop reads stdout until the prompt appears or `script` ends. It keeps stdin open, as it has to on macOS. If the command waits for input without printing the expected prompt, it never ends and the read never returns. That happens if `PROMPT` or the question's wording changes, or if a later command asks a second question. Then `cargo test` hangs with no message. (Found by reading the code; I didn't run a hanging test.) Read on a thread with a deadline, such as 10 seconds, and kill the child and panic with the output so far when it passes. This matters more as slices 6, 9 and 16 add three more users.
2. **`delete_item`'s rollback isn't tested.** Both branches of [store.rs:336-351](../../src/store.rs#L336-L351) worked when I checked them by hand with `chmod 500 items/`. A Unix-only test that does the same, skipped when running as root, would guard the "listed but didn't happen" ordering the readme relies on.
3. **No test covers a file from a newer tracker in `show --json` or `delete`.** [item_files.rs](../../tests/item_files.rs)' new test checks the text and the `via`, but not the fields that go missing (findings 1 and 5). Add one with an unknown top-level field once finding 1 is decided.
4. **A cancelled delete with `--json` isn't tested.** The success path's JSON is tested, and so is the refusal without a terminal. A wrong ID typed with `--json` should still put one `error.schema.json` object on stdout while the question goes to stderr.
5. **The terminal tests have only run on macOS.** The readme says so. The util-linux branch of [`script_cmd`](../../tests/common/mod.rs#L230-L246) is untried with piped stdin. Run it once on Linux before relying on it, or note in the harness that it's unverified there.
6. **`show` of an item whose project isn't in `config.json`** was checked by hand, but no test pins it (readme, Still open).

## Process

- **The readme is accurate apart from finding 1.** The test counts, the clippy claim, the line-by-line delete steps and the list of what's not built yet all match the code. Its [Limits](../slice-4-readme.md#limits-and-what-later-slices-change) section is a good hand-off to later slices.
- **The slice is one commit again** (`c980317`). The readme says so, and that only three of its tests were checked by breaking the code on purpose. That's honest, and it's the fourth slice in a row. Slice 5 is the last chance before `tracker` tracks itself, so commit test-first in steps there.
- **PLAN.md marks slice 4 `done` before this review**, as for slices 2 and 3. The readme lists this under Still open.

## Slice 5 must decide

Slice 5 is about bad data, so it needs to settle what `show` and `delete` do with it:

- **A corrupt item file can't be deleted.** Both `show` and `delete` exit 3 today, so the only way to remove one is by hand, which leaves no line in `deleted.json` and so nothing for the digest. §4.4 says a file that can't be parsed "costs one item, not the whole tracker". `delete` could still work from a terminal: warn that the file can't be read, take the ID from the file name, and record `"title": null` (or the raw first line). That needs `deleted.schema.json` to allow it. `show` should print the parser's message and the file's path, which it already does.
- **Colour** must stay off `show`'s output and off the question on stderr when either isn't a terminal. A test that redirects stderr while stdin is a terminal would check the second.
- **Its "no temporary files are left behind" check** should include a `delete`, which writes `deleted.json` through a temporary file.

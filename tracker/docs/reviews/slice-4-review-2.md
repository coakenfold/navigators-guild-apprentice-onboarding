# Slice 4 review 2

A second review of slice 4 (Show and delete). It checks the branch against [PLAN.md](../PLAN.md)'s Slice 4 prompt and _Done when_ list, and against the [DESIGN.md](../DESIGN.md) sections the slice touches (§2.7's shared confirmation, §2.8's delete, §3's `show` and `delete` rows and its rule for commands that take an ID, §4.1's `deleted.json`, and §4.4's lock, re-read and counter tripwire). It also checks the conventions in [CLAUDE.md](../../CLAUDE.md), the claims in [slice-4-readme.md](../slice-4-readme.md), and each fix that [review 1](slice-4-review-1.md)'s [outcome](slice-4-review-1.md#outcome) claims.

|                 |                                                                                                                                                                  |
| --------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Date            | 2026-11-17                                                                                                                                                       |
| Branch          | `feat/tracker/slice-4` at `3880bf9`, working tree clean. Review 1 covered the branch up to `75ba405`. This review covers that plus the 21 commits after it, from `6c1a831` to `3880bf9`: nine `fix:`, `refactor:` and `test:` commits, ten `docs:` commits (one of which, `44b2d11`, also changes `tty::confirm`'s doc comment), and `3880bf9`, which adds a Claude Code skill outside `tracker/` |
| Method          | Read the diff against `main`, then the slice's source, tests and schemas in full. Ran the test command and `cargo clippy --all-targets -- -D warnings`. Reproduced each finding marked "Reproduced" against the debug binary in a temporary store, typing answers through `script`. Ran HEAD's `tests/show_and_delete.rs` against `75ba405`'s `src/` and `schemas/`, and ran the suite against a copy of the code with `lock_unchanged` changed, each in a copy outside the repo |
| Line references | Point at `3880bf9`                                                                                                                                               |

## Verdict

Slice 4 still does what the plan asks, and review 1's fixes are real. `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and all 166 tests pass. The readme's per-file counts are right.

- **Every fix review 1's outcome claims is in the code**, each in its own commit, and each has a test. Its "seen failing" claim holds: HEAD's `tests/show_and_delete.rs` run against `75ba405`'s `src/` and `schemas/` fails exactly four tests, the ones for bugs 1, 2, 4 and 5, and passes the other 18.
- **`Store::lock_unchanged`** keeps the re-read under the lock in one place, as review 1's finding 9 asked. `delete` calls it, and the readme's "To add a human-only command" points later slices at it.
- **The docs fixes are in**: §2.8's example, §7's note about the `script` recipe, §3's `audit` row, and §2.7's `digest ack`.

I found no bug that loses data, and no new security hole. The findings that matter most:

- **Bug 1:** the fix for review 1's bug 5 treats a hand edit as a newer tracker's file. A file holding `"labels": []`, `"design": null` or a timestamp ending `+00:00` can't be deleted with `tracker delete`, and `show` tells me to "use a newer tracker" that doesn't exist.
- **Finding 3:** §3 now says every command needs a `config.json` that parses. But `delete`, the moves and most `edit`s never read it, so they work on a store whose config is broken.
- **Test gap 1:** no test notices if `lock_unchanged` stops holding the lock. With it changed to give the lock up before returning, every test still passes.
- **Review 1's bug 3 is still open**, and affects more titles than it said: an empty title also breaks `deleted.schema.json` ([below](#still-open-carried-forward)).

What was done about each finding, after the review, is under [Outcome](#outcome) at the end.

## Done when

| Check | Result | Tests |
| --- | --- | --- |
| `show` of a missing ID exits 1 | Pass | [show_and_delete.rs:59](../../tests/show_and_delete.rs#L59) |
| `echo XX-1 \| tracker delete XX-1` is refused and the file remains | Pass | [show_and_delete.rs:261](../../tests/show_and_delete.rs#L261), as text and as JSON, and with nothing on stdin |
| By hand in a terminal, for an item in each state: the prompt names the state and what will be lost, typing the ID deletes it, and a wrong ID cancels | Pass | By a person, per the readme's [Checks done by hand](../slice-4-readme.md#checks-done-by-hand). Also under `script`: [show_and_delete.rs:401](../../tests/show_and_delete.rs#L401) (all five states) and [show_and_delete.rs:477](../../tests/show_and_delete.rs#L477) |
| Deleting `XX-2` and then creating an item gives `XX-3` | Pass | [show_and_delete.rs:511](../../tests/show_and_delete.rs#L511) |
| A delete leaves one line in `deleted.json` with the ID, title, state, time and `interactive`, and nothing else of the item | Pass | [show_and_delete.rs:524](../../tests/show_and_delete.rs#L524). The line breaks `deleted.schema.json` when the title was edited by hand ([below](#still-open-carried-forward)) |
| _Prompt:_ `show` prints every field, the verification record, and history with timestamps | Pass | [show_and_delete.rs:145](../../tests/show_and_delete.rs#L145), [show_and_delete.rs:72](../../tests/show_and_delete.rs#L72) |
| _Prompt:_ one shared helper that refuses unless `tty::is_interactive()`, prints what will be lost, and requires the ID typed back | Pass | [tty.rs:38-87](../../src/tty.rs#L38-L87), through every delete test. The re-read under the lock that §2.7 adds is [store.rs:206-224](../../src/store.rs#L206-L224), but see test gap 1 |

## Bugs

1. **A hand-edited item file is treated as a newer tracker's, so `delete` refuses it and `show` misreports it.** `bef1a78` made `delete` exit 3 whenever [`item::lost_on_rewrite`](../../src/item.rs#L305-L317) finds anything ([main.rs:1505-1514](../../src/main.rs#L1505-L1514)). That function lists every top-level field whose value would change if the file were written back. That includes fields this version doesn't know, but also values it knows and writes differently. Those come from hand edits, which §1 says I can make, not from a newer tracker.

   Reproduced, each on an open item in a temporary store:

   | Hand edit | `show` | `delete` at a terminal |
   | --- | --- | --- |
   | `"labels": []` | exit 0, ending "XX-1's file has fields this version of tracker doesn't know, or can't show in full: labels. It was probably written by a newer tracker, so use that one to see them." | exit 3: "… It was probably written by a newer tracker, which knows what deleting it must remove, so use that one. Nothing was deleted." |
   | `"design": null` | the same, naming `design` | exit 3, naming `design` |
   | `"created"` ending `+00:00` instead of `Z` | the same, naming `created` | exit 3, naming `created` |

   There's no newer tracker to use, so the only way out is to fix the file by hand, or remove it by hand. Removing it leaves no line in `deleted.json`, which is what §2.8's record exists to prevent. The moves and `edit` already refuse such a file through `update_item`. The readme carries that as slice 2 review 2's item 4 ("A hand-edited timestamp is blamed on a newer tracker"). This slice spreads the problem to `show`'s warning and to `delete`, the one command that has no reason to care how a value is written.

   The fix is to split what `lost_on_rewrite` finds in two:
   - **unknown**: a top-level key the struct doesn't have, or a value that holds one (such as a `verification` with slice 8's `commit`, or a history `via` this version reads as `Unknown`). These come from a newer tracker. `delete` should still refuse them, and `show` should keep its warning.
   - **written differently**: a known field whose value only round-trips to a different spelling (`[]`, `null`, a timestamp's offset). Nothing is hidden. `delete` should go ahead, and `show` should either say nothing or say "XX-1's file spells labels differently from how tracker writes it".

   One way to tell them apart: deserialize the file into a `serde_json::Value`, delete each key the rewrite also has, and check whether anything is left at the top level, or inside `verification`, `previous_verification` or `history`. A simpler option is to call a value "written differently" when both sides are empty or null, or both are timestamps that parse to the same instant. I'd do the simpler version now, since slice 5 is about bad data, and add a test for each row of the table above.

2. **A delete that is run again after a crash adds a second line for the same ID.** [`delete_item`](../../src/store.rs#L355-L388) writes the record before it removes the file, so that a crash leaves "a delete that's listed but didn't happen, which I'd notice". The obvious next step is to run `tracker delete XX-1` again, and it appends a second `XX-1` line. Slice 16's digest would then list the delete twice. And if `audit`'s "IDs used twice" counts `deleted.json`, as the readme's Limits section suggests, it would report XX-1 forever. (Found by reading the code. A crash in that window is unlikely.) `delete_item` could skip the append when the last line of `deleted.json` already names this ID. Or slice 16 could treat repeated lines for one ID as one delete. Either is fine. I'd note the choice in slice 16's prompt rather than change the code now.

## Security

No new holes. I checked:

- **The re-read.** `lock_unchanged` compares the file's whole text, and the warning was built from an item parsed from that same text ([main.rs:1500](../../src/main.rs#L1500)). An agent's edit while the question waits can therefore only cause a refusal, which [show_and_delete.rs:619](../../tests/show_and_delete.rs#L619) shows end to end. But no test checks that the lock is still held when `lock_unchanged` returns (test gap 1).
- **The new warning text.** The field names that `show` and `delete` print come from the file's keys, which a hand edit controls. They're escaped like all output: `text::for_terminal` in the text, `text::for_json` in the JSON, and `fail`'s escaping in the error.
- **The non-terminal refusal** ([main.rs:1517-1529](../../src/main.rs#L1517-L1529)) now suggests `shelve` only where `states::find` draws a shelve, so an agent isn't told to try a move that will fail.

## Where the code and DESIGN.md disagree, or the design is unclear

3. **§3 says every command needs `config.json` to parse, but most commands that take an ID never read it.** `1fcdeec` settled review 1's finding 11 by adding to [§3](../DESIGN.md#3-interface): a command that takes an ID "still needs `config.json` to parse, like every other command, `show` included … A store whose config can't be read is repaired by hand first, and a command that carried on would hide that it's broken." The code calls `config::load` only in `project add`, `project list`, `create`, `list`, `show` ([main.rs:1350](../../src/main.rs#L1350)), and `edit --design` ([main.rs:892](../../src/main.rs#L892)). `start`, `submit`, `shelve`, `reopen`, `status`, `delete`, and `edit` without `--design` go ahead without it. Reproduced: with `config.json` cut to `{`, `show XX-6` exits 3, while `start XX-6` exits 0, and `delete XX-6` at a terminal deletes the item and exits 0.

   So the decision rests on a rule the code doesn't follow, and the one command it singles out, `show`, is the read-only one. Either:
   - **(a)** make it true: have `Store::require_exists` also parse `config.json`, so every command that needs the store refuses a broken config with exit 3, and add a test that runs each command against a config cut to `{`, or
   - **(b)** correct §3 to say that only commands that use the config read it, and reopen finding 11, since `show` could then print "(config.json can't be read)" like the others.

   I'd do (a). It keeps the decision that was just made, it's a few lines, and it belongs with slice 5's "a missing store or config gives a clear message" (§4.4).

4. **The refusal text in `tty::confirm` speaks only of an ID.** §2.7 and `44b2d11` say that `digest ack` will confirm by typing `ack`, and `confirm`'s doc comment says so. But its refusal without a terminal ([tty.rs:41-42](../../src/tty.rs#L41-L42)) says "needs a human at an interactive terminal, who types the ID to confirm it", so slice 16's `digest ack` would be refused with a message about an ID. It's a one-word change ("who types {id} to confirm it"). It's cheapest to make now, while the readme still tells slice 16 to reuse `confirm` unchanged.

## Test gaps

1. **Nothing checks that `lock_unchanged` holds the lock it returns.** [item_files.rs:270-314](../../tests/item_files.rs#L270-L314) is named `lock_unchanged_holds_the_lock_only_while_the_file_is_as_it_was`. But it only checks the refusals, and that the lock is released afterwards. I changed `lock_unchanged` to drop the lock and return a freshly opened, unlocked `lock` file. All 143 tests in `show_and_delete`, `item_files`, `states`, `organise`, `create_and_list`, `store_writes` and `fixtures` still passed. That is the guarantee `delete` relies on between its re-read and its write, and slice 9's `approve` will rely on it too (PLAN.md's integration risks). Add a check inside that test: while the returned guard is alive, `File::try_lock` on the store's `lock` file, opened a second time, fails with `WouldBlock`. After the guard is dropped, it succeeds.
2. **No test covers a hand-edited file in `show` or `delete`** (bug 1). The newer-tracker tests ([show_and_delete.rs:91](../../tests/show_and_delete.rs#L91), [show_and_delete.rs:329](../../tests/show_and_delete.rs#L329)) use `edit_file`, which writes through `serde_json` and so never produces `[]`, `null` or a non-`Z` timestamp unless asked to. Add the three rows of bug 1's table once it's fixed.
3. **No test covers a command against a broken `config.json`** (finding 3). `commands_without_a_store_say_run_tracker_init` covers a missing store, but nothing covers a config that's there and won't parse.

## Process

- **The readme is out of date in three places.** Its Branch row ([slice-4-readme.md:10](../slice-4-readme.md#L10)) lists the commits only up to `c1a08b2`, not review 1's fixes. Step 2 of "How it works: `show`" ([line 249](../slice-4-readme.md#L249)) says `show` reads with `Store::load_item`. Since `6c1a831` it uses `load_item_with_text`. And the coverage table ([line 361](../slice-4-readme.md#L361)) still names `show_json_is_the_item_exactly_as_its_file_holds_it`, which `6c1a831` renamed to `show_json_is_the_item_as_its_file_holds_it_when_this_version_wrote_it`. Review 1's outcome says the readme stopped claiming exactness, and its prose did, but this table cell didn't. Everything else in the readme matches the code, including the test counts and the Status row.
- **Review 1's fixes were made the way the earlier reviews asked**: one commit per finding, each with its test, and the behaviour changes seen failing. That's a clear improvement on the slice's own single commit.
- **`3880bf9` puts an unrelated change on the slice's branch.** It adds `.claude/skills/slice-review/SKILL.md`, outside `tracker/`, under a `feat:` header. It will merge with slice 4 unless it's moved. That's harmless, but say so in the pull request, or cherry-pick it to `main` on its own.

## Still open, carried forward

From [review 1](slice-4-review-1.md), and already under the readme's [Still open](../slice-4-readme.md#still-open):

- **Bug 3: a hand-edited title breaks `deleted.schema.json`.** It's wider than review 1 said. Reproduced at `3880bf9`: `"two\nlines"` and `""` both delete with exit 0, and `delete --json` prints, and `deleted.json` keeps, `"title": "two\nlines"` and `"title": ""`. Both break the `$ref` to `item.schema.json`'s title rule ([deleted.schema.json:13-16](../../schemas/deleted.schema.json#L13-L16), [item.schema.json:14-19](../../schemas/item.schema.json#L14-L19)), the first by its pattern and the second by `minLength`. A control character, written as a JSON escape, breaks it the same way. The fix review 1 suggested still fits: the record's `title` is any string, since it records what was there, with the schema's description saying why.
- **Test gap 5: the terminal tests have only run on macOS.** I couldn't check the util-linux branch either.
- **Process: PLAN.md marks slice 4 `done` before its reviews.**

The readme also carries slice 2's and slice 3's open items. None of them changed on this branch, apart from slice 2 review 2's item 4, which bug 1 above extends.

## Slice 5 must decide

Review 1's [list](slice-4-review-1.md#slice-5-must-decide) still applies: a corrupt item file can't be deleted, colour must stay off `show`'s output and off the question on stderr, and the check for leftover temporary files should include a `delete`. Two more points from this review:

- **What a hand edit that only respells a value means**, as against a newer tracker's field (bug 1). Slice 5 handles bad data, so it should decide this for `show`, `delete`, the moves and `edit` together, and close slice 2 review 2's item 4 at the same time.
- **Whether every command refuses a broken `config.json`** (finding 3). §4.4's "a missing store or config gives a clear message" is in slice 5's prompt, so that's where the rule §3 now states gets enforced, or §3 gets corrected.

## Outcome

Added after the fixes. The findings above are left as written, so their line links point at `3880bf9`, not at the fixed code. Each fix is in its own commit, with its tests, and the readme's [Fixes after review 2](../slice-4-readme.md#fixes-after-review-2) lists them too.

Seen failing: the tests for bugs 1 and 2 and findings 3 and 4 were each run against their fix's parent, and failed. Bug 1's test in `tests/item_files.rs` uses the new `Lost`, so it can't build against the old code, and only its test in `tests/show_and_delete.rs` was run. Test gap 1's check was run against a copy of `lock_unchanged` changed to drop the lock and return an unlocked `lock` file, as the review did, and failed.

After the fixes, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and all 170 tests pass, 4 more than at `3880bf9`.

| Finding | Resolved by |
| --- | --- |
| Bug 1: a hand-edited item file is treated as a newer tracker's | `9d16c32`: `item::lost_on_rewrite` returns a `Lost` with `unknown` fields (a field this version doesn't know, at the top level or inside a record or the history) and `respelled` ones (`[]`, `null`, or a time with another offset for the same instant). `show` warns and `delete` refuses only for `unknown`. `Store::update_item` still refuses both, as slice 5 must decide. Tests: `show_and_delete_accept_a_file_whose_values_are_only_written_differently`, one row per line of the table, and `lost_on_rewrite_tells_unknown_fields_from_values_written_differently` |
| Bug 2: a delete run again after a crash adds a second line | `62d6bc9`: fixed in the code rather than in slice 16's prompt. `delete_item` adds no line when `deleted.json` already has one with the ID, anywhere in the file. Test: `a_delete_run_again_after_a_crash_doesnt_list_it_twice` |
| Design 3: most commands that take an ID never read `config.json` | `6f22b81`: option (a). `Store::require_exists` ends with `config::load`, so every command but `init`, which already checked it, exits 3 on a config that can't be parsed. Test: `every_command_refuses_a_config_json_that_cant_be_parsed`, which also covers `delete` at a terminal |
| Design 4: `tty::confirm`'s refusal speaks only of an ID | `0dd432c`: "who types {id} to confirm it". The readme's Try it shows the new text. `echo_the_id_into_delete_is_refused_and_the_file_remains` checks it |
| Test gap 1: nothing checks that `lock_unchanged` holds the lock | `803c83c`: `lock_unchanged_holds_the_lock_only_while_the_file_is_as_it_was` checks that a second `try_lock` gives `WouldBlock` while the guard lives, and succeeds after it's dropped |
| Test gap 2: no test of a hand-edited file in `show` or `delete` | `9d16c32`: bug 1's tests |
| Test gap 3: no test of a command against a broken `config.json` | `6f22b81`: finding 3's test |
| Process: the readme is out of date in three places | `f4df32e`: the Branch row lists every commit range, `show`'s step 2 names `load_item_with_text`, and the coverage table has the renamed test. The readme also gets Fixes after review 2 and new test counts |
| Process: review 1's fixes were made as asked | Unchanged. This round's fixes are one commit each, too |
| Process: `3880bf9` puts an unrelated change on the branch | Still open, for the pull request. The readme's Still open lists it |
| Carried forward: review 1's bug 3, a hand-edited title breaks `deleted.schema.json` | Still open. The readme's entry now includes the empty title and control characters |
| Carried forward: review 1's test gap 5, the terminal tests have only run on macOS | Still open |
| Carried forward: PLAN.md marks slice 4 `done` before its reviews | Still open |
| Slice 5 must decide | Unchanged, for slice 5. Bug 1 leaves the moves and `edit` refusing a respelled file, and finding 3's rule is now enforced, so slice 5's message for a broken config builds on `Store::require_exists` |

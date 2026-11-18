# Slice 6 review 1

A first review of slice 6 (Criteria and simple gates). It checks the branch against [PLAN.md](../PLAN.md)'s Slice 6 prompt and _Done when_ list and its integration risks, and against the [DESIGN.md](../DESIGN.md) sections the slice touches: §2.1's `written`, `criteria` and `history`, §2.3's criteria and evidence, §2.4's gates G1, G2 and G6, §2.7's `criterion accept`, §3's commands, refusal shape and exit codes, §4.3's field order, §4.4's escaping and one-item-at-a-time rule, and §7's limit on what G6 reads. It also checks the conventions in [CLAUDE.md](../../CLAUDE.md), the claims in [slice-6-readme.md](../slice-6-readme.md), and the "Slice 6 must decide" lists in slice 5's [review 1](slice-5-review-1.md#slice-6-must-decide) and [review 2](slice-5-review-2.md#slice-6-must-decide).

|                 |   |
| --------------- | - |
| Date            | 2026-11-17 |
| Branch          | `feat/tracker/slice-6` at `095b250`, working tree clean. The slice is 4 commits on `main`'s `440628f`: `be91d1f` (sha2), `4d2d6ad` (the code, schemas, tests and fixtures), `6eb689d` (DESIGN.md) and `095b250` (the readme and PLAN.md's status) |
| Method          | Read DESIGN.md and PLAN.md in full, and the diff of each against `main`. Read slice 5's two reviews, CLAUDE.md and the readme. Read the diff against `main` and every changed source, test, schema and fixture file in full. Ran the test command and `cargo clippy --all-targets -- -D warnings`. Reproduced each finding marked "Reproduced" against the debug binary in a temporary store, typing into `criterion accept` through a pseudo-terminal. Ran the readme's Try it verbatim. **Written in the same session that built the slice**, so not independent ([Process](#process)) |
| Line references | Point at `095b250` |

## Verdict

Slice 6 does what the plan asks. `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo clippy --all-targets -- -D warnings` and all 237 tests pass. Every _Done when_ bullet and every requirement in the prompt has a test, and the readme's per-file counts and Try it output are right.

- **The gates refuse as §2.4 says.** Every failure is listed, nothing changes, and the exit code is 2, under `--json` with `id`, `result`, `gates` and `failures`. The tests check the item file byte for byte after every refusal.
- **G6 follows §7.** It reads `design` relative to the root, refuses a file that leads outside it through a symlinked file or folder, `..` or an absolute path, and never opens anything but a regular file, which settles slice 5 review 2's point about pipes.
- **Every item in slice 5's two "Slice 6 must decide" lists is handled**, apart from colour for the new commands ([Process](#process)): the shared helper gives test items a criterion, a refusal prints one reason per line, the new commands have help and `COMMANDS` entries, a criterion's text and evidence are one line of at most 500 characters, and `tests/fixtures/slice-6/` holds the new shape.

I found no bug that loses data. The findings that matter most:

- **Finding 4:** `criterion accept` can't say which written criteria it drops, so accepting my own hand edit also accepts an agent's removal of another criterion, unseen.
- **Finding 6:** nothing at submit requires a criterion. An item whose criteria were all removed after `start` reaches done.
- **Finding 5:** G1 ignores an item file it can't read, so a second item can start while the first is in progress.
- **Bug 1:** a judge criterion from a newer tracker is read as a manual one, so `gates` and `submit` give a wrong reason with exit 2, where every other command says the file is from a newer tracker.

What was done about each finding, after the review, is under [Outcome](#outcome) at the end.

## Done when

| Check | Result | Tests |
| --- | --- | --- |
| `start` with no criteria is refused by G1 with exit 2 | Pass | [gates.rs:90](../../tests/gates.rs#L90), through `start` and `status <id> in-progress` |
| …and so is `start` while another item in the project is in progress | Pass | [gates.rs:114](../../tests/gates.rs#L114), [gates.rs:142](../../tests/gates.rs#L142) (in review), [gates.rs:157](../../tests/gates.rs#L157) (every reason at once). But see finding 5 |
| A manual criterion with no evidence fails G2 | Pass | [gates.rs:204](../../tests/gates.rs#L204), through `gates`, `submit` and `status <id> done` |
| A `design` naming a missing section fails G6 | Pass | [gates.rs:287](../../tests/gates.rs#L287). But see bug 2 |
| …and a numbered section with content under it passes | Pass | [gates.rs:260](../../tests/gates.rs#L260), and the unit tests at [gates.rs:395](../../src/gates.rs#L395) |
| A `design` whose file is a symlink to one outside the root fails G6 | Pass | [gates.rs:318](../../tests/gates.rs#L318), which also covers a symlinked folder, and [gates.rs:357](../../tests/gates.rs#L357) for `..` and an absolute path |
| `gates` lists every failure and leaves the item file byte-for-byte unchanged | Pass | [gates.rs:411](../../tests/gates.rs#L411), and every test through `refused_by_gate` ([gates.rs:23](../../tests/gates.rs#L23)) |
| A refused `submit` exits 2 and also changes nothing | Pass | [gates.rs:459](../../tests/gates.rs#L459), [gates.rs:204](../../tests/gates.rs#L204) |
| `create --done-when a --done-when b` adds two criteria and two `written` fingerprints | Pass | [criteria.rs:80](../../tests/criteria.rs#L80), against SHA-256 values worked out with `shasum` |
| `evidence` on a script criterion exits 1 | Pass | [criteria.rs:316](../../tests/criteria.rs#L316) |
| `criterion accept` is refused outside an interactive terminal | Pass | [criteria.rs:403](../../tests/criteria.rs#L403), with the ID piped in. Typing the ID is tested under `script` ([criteria.rs:434](../../tests/criteria.rs#L434), [criteria.rs:535](../../tests/criteria.rs#L535), [criteria.rs:550](../../tests/criteria.rs#L550)), and I typed it through a pseudo-terminal for finding 4 |
| _Prompt:_ `criterion add`, with `--check` or manual | Pass | [criteria.rs:151](../../tests/criteria.rs#L151), [criteria.rs:194](../../tests/criteria.rs#L194) |
| _Prompt:_ `criterion set <n> --check` and `criterion remove` | Pass | [criteria.rs:220](../../tests/criteria.rs#L220), [criteria.rs:255](../../tests/criteria.rs#L255) |
| _Prompt:_ `written` taken at create, and an interactive-only `criterion accept` with `via: "criterion-accept"` | Pass | [criteria.rs:108](../../tests/criteria.rs#L108), [criteria.rs:296](../../tests/criteria.rs#L296), [criteria.rs:434](../../tests/criteria.rs#L434). But see finding 4 |
| _Prompt:_ give `tests/states.rs`'s items a criterion through the shared helper | Pass | `Env::create` ([mod.rs:314](../../tests/common/mod.rs#L314)) calls `add_criterion` ([mod.rs:329](../../tests/common/mod.rs#L329)). All 27 tests in `tests/states.rs` still test moves |
| _Integration risks:_ a fixture of the item file's new shape | Pass | [fixtures.rs:167](../../tests/fixtures.rs#L167), and the tests over every slice's fixtures |

## Bugs

1. **A judge criterion from a newer tracker is read as a manual one, so `gates` and `submit` give a wrong reason.** [`Criterion`](../../src/item.rs#L55) has no judge fields, and serde ignores the ones it doesn't know, so a criterion with `ask` loads with neither `check` nor `ask`, and [`is_manual`](../../src/item.rs#L68) says it's manual. `gates_dry_run` ([main.rs:1656](../../src/main.rs#L1656)) and `submit` (through [`apply`](../../src/main.rs#L2019-L2033)) run G2 on it before anything looks for fields this version doesn't know.

   Reproduced: with `"criteria": [{ "text": "Errors say what to do", "ask": [ … ] }]` written into XX-4's file, `gates XX-4` and `submit XX-4` both exit 2 with "G2  Criterion 1, "Errors say what to do", is manual and has no evidence. Record where to look with `tracker evidence XX-4 1 …`". `start XX-4` exits 3 with "has fields this version of tracker doesn't know … It was probably written by a newer tracker", because G1 passes and `update_item` then refuses. `show` warns too.

   Nothing is written, so no data is lost. But the advice is wrong: following it, an agent would record evidence for a judge criterion, and `evidence` would then exit 3. Slice 5 made a point of naming a newer tracker's file rather than misreading it, and during self-tracking the installed binary is a slice away from the code. Fix: in `gates_dry_run` and in `apply`, before running any gate, refuse a file with unknown fields with exit 3, as `delete` does ([main.rs:2320](../../src/main.rs#L2320)). `apply` has the item but not its text, so `move_item` and `status` would load it with `load_item_with_text`.

2. **G6 counts some lines as headings, and as content, that markdown doesn't.** [`outside_fences`](../../src/gates.rs#L333-L354) closes a fence on any line starting with the same marker, and [`heading`](../../src/gates.rs#L357-L368) trims all leading space. Reproduced, each in its own file, each passing `gates` when it should fail:

   | Design doc | Section | Why it shouldn't pass |
   | --- | --- | --- |
   | `` ``` ``, `code`, `` ```rust ``, `## 2. Inside the fence`, `text`, `` ``` `` | `2` | A closing fence has no info string, so `` ```rust `` doesn't close the block, and the heading is code |
   | `    ## 3. Indented code`, `    text` | `3` | Four spaces of indent make a code block, not a heading |
   | `## 4. Empty code block`, `` ``` ``, `` ``` `` | `4` | The only "content" is an empty code block's fences |

   The readme says "Lines in fenced code blocks are never headings" ([slice-6-readme.md:141](../slice-6-readme.md#L141)), which the first row disproves. All three need a design doc written oddly, so this is low severity, but G6 is meant to stop an item pointing at a section that says nothing. Fix: close a fence only on a line that is the marker alone, with at least as many characters as it opened with; take a heading only with at most three spaces of indent, as CommonMark does; and don't count a line that's only a fence marker as content.

## Security

3. **A line break in a criterion's text or evidence, written by hand, prints as a line of its own.** `criterion remove` prints the removed text ([main.rs:1499](../../src/main.rs#L1499)), and `criterion set` the evidence it drops ([main.rs:1473](../../src/main.rs#L1473)), straight into `Output::plain`. [`Styled::render`](../../src/styled.rs#L53-L56) escapes every control character but keeps line breaks, since it renders text of several lines.

   Reproduced: with criterion 1's text edited to `ok\nXX-5: in-progress → done`, `criterion remove XX-5 1` printed:

   ```
   Removed criterion 1 from XX-5: ok
   XX-5: in-progress → done
   The ones after it moved up one.
   ```

   and `criterion set` on evidence `x\nfake line` printed `Its evidence, "x` and `fake line", was removed: …` as two lines. §4.4 says every output escapes control characters "so no text can change what the terminal shows", and `show` does this for the same fields by escaping each one on its own line first. It needs a hand edit, which §7 already allows an agent with a shell, so it's low severity. Fix: interpolate `text::for_terminal(&removed.text, false)` and the evidence the same way, as `show` does. The `--json` output and the gate refusal's details are already escaped.

4. **`criterion accept` can't say which written criteria it drops, so accepting my own hand edit also accepts an agent's removal, unseen.** [`acceptance_warning`](../../src/main.rs#L1588-L1615) lists each criterion as it is now, and only counts the fingerprints in `written` that no criterion matches ([main.rs:1607](../../src/main.rs#L1607)), since a SHA-256 can't be turned back into its text.

   Reproduced: XX-7 was created with "Search filters by tag" and "Empty results say so". An agent ran `criterion remove XX-7 2`, which leaves `written` alone, as it should. Then I edited criterion 1 by hand to "Search filters by tag, case-insensitively" and ran `criterion accept XX-7` at a terminal. It showed:

   ```
     1. Search filters by tag, case-insensitively  (new to written)
   2 written criteria match none of them now, and are dropped.
   Type XX-7 to confirm, or anything else to cancel:
   ```

   I typed XX-7, and `written` now holds one fingerprint, for my edit. The agent's removal of "Empty results say so" is accepted too, and slice 11's T5 will never fire for it. Nothing in history records the removal either, since §2.1 doesn't give `criterion remove` an entry. "2 … are dropped" is the only clue, and only if I expected 1.

   T5 exists so that an agent can't quietly drop a criterion it was given (§2.1), and `criterion accept` is the one way to clear it, so this matters more than the rest. The design names the fingerprint as SHA-256 (§2.1, §4.3, PLAN.md's prompt), so the fix is a design change:

   - **(a)** Keep the text in `written`, not its hash, or beside it. `criterion accept` can then name each criterion it drops, and slice 11's T5 can say which criterion was reworded or removed rather than only that one was. The file is local plain text that already holds the criteria, so a hash hides nothing.
   - **(b)** Keep the hashes, and give a `criterion remove` that takes out a written criterion a history entry with the removed text, so `accept` can name it. That still misses a criterion reworded by a hand edit, which is what `accept` is for.

   I'd pick (a). It touches §2.1's `written` row, §4.3's example, PLAN.md's prompt, `item.schema.json` and the slice 6 fixtures, so it's best done before self-tracking writes TR items in this shape.

I also checked:

- **Every gate message is escaped where it's printed.** The details go through `text::for_terminal` in `fail` ([main.rs:511-512](../../src/main.rs#L511-L512)), and `--json` through `json_text`. A criterion's text and a design path in a refusal can't change the terminal.
- **`criterion accept` changes nothing if the item changed while I decided.** It takes the lock through `Store::lock_unchanged`, as `delete` does, and [criteria.rs:550](../../tests/criteria.rs#L550) tests it.
- **Every criterion command reads the item under the lock and writes through `update_item`**, so it's atomic and refuses a newer tracker's file.
- **G6 never opens a pipe or a device.** A symlink loop in the design path fails G6 with the system's "Too many levels of symbolic links" and no hint, which is acceptable for a gate. A huge regular file inside the root is read whole: low severity, since the agent would have to point `design` at it.

## Where the code and DESIGN.md disagree, or the design is unclear

5. **G1 ignores an item file it can't read, so a second item can start while the first is in progress.** [`apply`](../../src/main.rs#L2021-L2024) drops the files `load_items` skipped, and the readme lists this as a limit ([slice-6-readme.md:215](../slice-6-readme.md#L215)).

   Reproduced: XX-1 in progress, its file then broken by deleting a comma. `start XX-2` succeeded, and `list` named the broken file on stderr.

   §4.4 says both that a file that can't be read "costs one item, not the whole tracker" and that `start` refuses while another item in the project is in progress (G1). They pull in different directions here, and the code chose the first without the design saying so. A newer tracker's item with a state this version doesn't know would be skipped the same way. Fix:

   - **(a)** G1 fails when a skipped file's name, such as `XX-1.json`, puts it in the item's project: "XX-1.json can't be read as an item, so G1 can't tell whether it's in progress. Fix it, or move it out of items/." The file name is checked against the ID already, so it's reliable enough.
   - **(b)** Keep the code, and say in §4.4 that G1 can't see such a file.

   I'd pick (a): a broken file in the project is rare, and failing closed is what a gate is for.

6. **Nothing at submit requires a criterion.** G1 checks for one at `start` only, and G2 passes when there are no manual criteria.

   Reproduced: XX-3 was started with one script criterion, then `criterion remove XX-3 1` emptied it, and `submit XX-3` printed "Gates G2 and G6 passed." and moved it to done. An item started before slice 6, with no criteria, would do the same.

   §2.1 says criteria are required "before work starts", and §1's goal is that no item reaches done unless its checks pass. Slice 11's T5 will hold an item whose criteria changed after start, but only for one with a `started` fingerprint, and until slice 9 nothing holds anything. Fix:

   - **(a)** G2 also fails an item with no criteria ("XX-3 has no criteria, so nothing says when it's done"), and §2.4's G2 row says so.
   - **(b)** Leave it to T5.

   I'd pick (a): it's one line, and it doesn't depend on how the criteria came to be empty.

7. **§3 says the criterion commands "work at any time", but the code refuses a done item, and an item in review unless from a terminal.** [`check_criteria_changeable`](../../src/main.rs#L1362-L1378) applies `edit`'s rules, and `criterion accept` refuses a done item too. The readme gives the reason, a done item's verification record is about the criteria it was verified against, which is sound, but [DESIGN.md:439](../DESIGN.md#L439) still says "at any time". Fix: §3's paragraph says "in any state but done, and for an item in review only from an interactive terminal, as `edit`".

8. **A check command has no length limit, unlike a criterion's text and evidence.** [`check_command`](../../src/main.rs#L1320-L1335) checks only that it's one line. Reproduced: `criterion add XX-6 long --check "<100,000 x's>"` succeeded. Slice 5 review 2 asked for limits on a criterion's text and evidence, which this slice set at 500. A check is printed in `show` today, and from slice 7 before it runs and in every G3 failure, so it's as much "printed among other lines" as a reason. §2.3 says nothing. Fix: one line of at most 500 characters, like the text, with "put longer commands in a script the check runs", and a line in §2.3.

9. **§2.4 doesn't say what "content under it" means, so G6 had to guess.** The code counts any non-blank line before the next heading of the same level or higher, so a section with only subsections that say something passes, and a code block counts. Both are reasonable, but they're the code's choices, recorded only in the readme. Fix: one sentence in §2.4's G6 row, decided with bug 2's fix.

## Test gaps

1. **No test of a newer tracker's criterion through `gates` or `submit`** (bug 1). Add one with an `ask` field, expecting exit 3 and the item unchanged.
2. **No test of G6 with an info string on a closing fence, an indented heading, or an empty code block** (bug 2). Add the three rows above to [gates.rs:287](../../tests/gates.rs#L287)'s table, or to the unit tests in `src/gates.rs`.
3. **No test that text output escapes a hand-edited criterion or evidence** (finding 3). [criteria.rs:569](../../tests/criteria.rs#L569) checks `show` only.
4. **No test that `criterion set` and `evidence` leave `written` alone**, from a terminal or not. [criteria.rs:296](../../tests/criteria.rs#L296) covers `add` and `remove` only. §2.1's rule is that only text changes reach `written`, and slice 11 depends on it.
5. **No test that `criterion accept` refuses a done item**, which the readme lists as a decision.
6. **No test pins what G1 does with an unreadable item file** (finding 5), either way.

## Process

- **This review isn't independent.** It was written in the same session, with the same context, that built the slice a few hours before, so it shares the builder's blind spots. Slice 5's reviews were written in new sessions. Run review 2 in a new session before merging, and PLAN.md's #adversarial-review still matters most.
- **The code, schemas, tests and fixtures landed as one commit** (`4d2d6ad`), as slice 5's `0dfa462` did, which slice 5's reviews noted. No new test was seen failing before the code it tests existed, since they were written after it. The changes to existing tests were driven by real failures, though: 41 tests across 7 files failed once the slice's code was in, most of them on G1, before the shared helper gave items a criterion.
- **`6eb689d` settles part of slice 11 early.** It adds to §2.1 that `written` has no order and that T5 "asks whether each fingerprint still matches some criterion's text". That follows from agents' criteria having no fingerprint, and the code relies on it in `criterion accept`, but slice 11 should know it's been decided, and finding 4 may change it.
- **The readme is right about what it covers**: the 237 tests and each file's count, the Try it output, the coverage table's test names and the decisions all checked out. But it misses things:
  - It overclaims about fences (bug 2).
  - Its Limits leave out a submit with no criteria (finding 6) and the newer tracker's criterion (bug 1).
  - It has no "Still open" section. Slice 5's readme ([slice-5-readme.md:363](../slice-5-readme.md#L363)) carried the open items of slices 2–5, and slice 6's drops them. They're still open, so they should be carried forward.
- **Colour for the new commands wasn't added**, and the readme doesn't say so. Slice 5's review 1 listed it for slice 6, and slice 5's readme says slice 16 brings later commands up to its standard. Only the red `error:` is coloured. Saying so under "Not built yet" is enough.

## Slice 7 must decide

- **The shared test helper's check is `true`, which passes before the work.** Slice 7's `start` runs every check and warns about one that already passes, so every test item will warn, and slice 11's T13 will hold every submit. Slice 7 should pick a helper check whose result a test controls, such as `test -f done`, so that tests of moves don't depend on the warning or the hold.
- **Where G3 and G4 go in `gates::submit`.** §2.4 orders the static gates (G2, G5, G6) before the scripts (G3, G4). Slice 7 adds G3 and G4 after G6, and slice 8 puts G5 between G2 and G6.
- **`secs` in the refusal and the dry run.** [submit.schema.json](../../schemas/submit.schema.json) allows `secs` on a gate, but `gates.schema.json` and `error.schema.json` set `additionalProperties: false` without it, so a G3 or G4 entry with `secs` would fail both.
- **The unchecked note names G2 and G6.** Once checks run, a passing submit has run G3 and G4 too, and slice 8 makes it `auto`. Slice 7 has to decide what a submit's record says in between.
- **`criterion set --check` after `start`** must run the new check once, as §2.4 says, and record it in `started.checks`.
- **A check is printed before it runs.** Finding 8's limit should be settled first, and the printed command escaped like any other text from the file.

## Outcome

Added after the fixes. The findings above are left as written, so their line links point at `095b250`, not at the fixed code. Each fix is in its own commit, with its tests, and the readme's [Fixes after review 1](../slice-6-readme.md#fixes-after-review-1) lists them too.

Every numbered finding is resolved: both bugs, findings 3 to 9 and all six test gaps. Where a finding offered a choice, option (a) was taken, and finding 8's limit is 1000 characters rather than the 500 suggested. The process notes weren't part of this round, and "Slice 7 must decide" is left for slice 7. The commits after this review (`04fab3f`), in order:

- **Fixes:** `9046539` (bug 1 and test gap 1), `a16fd2b` (bug 2 and test gap 2), `3ab8de7` (finding 3 and test gap 3), `05f4bba` (finding 4), `c1e91fd` (finding 5 and test gap 6), `c08cdba` (finding 6), `b51549b` (finding 7), `ab324b4` (finding 8), `22fc352` (finding 9), `e137f93` (test gap 4) and `0b6aa89` (test gap 5).
- **Docs:** `3af1c73` (the readme) and the commit that adds this Outcome.

| Finding | Resolved by |
| --- | --- |
| Bug 1: a newer tracker's judge criterion is read as a manual one | `9046539`, as suggested: `refuse_newer_file` refuses a file with fields this version doesn't know, with exit 3, in `gates_dry_run` and at the top of `apply`, before any gate. `move_item` and `status` load the item with its text for it. Every move gets the check, so `shelve` and `reopen` now give this message rather than `update_item`'s. Test: `a_newer_trackers_file_is_refused_with_exit_3_before_any_gate_runs` |
| Bug 2: G6 counts some lines as headings, and as content, that markdown doesn't | `a16fd2b`, as suggested: `outside_fences` is replaced by `lines`, which says what each line is. A fence closes only on the same character, at least as long, with nothing after it, and a backtick fence's info string can't hold a backtick. A heading or fence takes at most three spaces of indent, and a tab makes indented code. A fence line is never content. The readme's "Lines in fenced code blocks are never headings" is now true. Tests: two unit tests in `src/gates.rs`, with the review's three rows |
| Finding 3 (security): a line break in a criterion's text or evidence, written by hand, prints as a line of its own | `3ab8de7`, as suggested, for `criterion remove` and `criterion set`. `criterion accept`'s warning had the same problem, since `tty::confirm` keeps line breaks, and is fixed in the same commit. Test: `text_output_escapes_a_hand_edited_criterion_and_its_evidence` |
| Finding 4 (security): `criterion accept` can't say which written criteria it drops | `05f4bba`, option (a): `written` holds each criterion's text, and `criterion accept` lists each text it drops, saying it was removed or reworded by hand. DESIGN.md §2.1, §2.3, §2.5's T5, §2.7, §4.3's example and §6, PLAN.md's slice 6 prompt and _Done when_ and slice 11's T5, `item.schema.json`, `tests/schemas.rs` and the slice 6 fixtures follow. The fixtures' `written` values were edited to the texts their hashes stood for, checked with `shasum`, which is what this version writes. Nothing uses `sha2` now, so it's removed, and slice 11's prompt names it for T5's criteria hash. Test: `criterion_accept_names_each_written_criterion_it_drops`, the review's case |
| Finding 5: G1 ignores an item file it can't read | `c1e91fd`, option (a): G1 fails on each skipped file whose name puts it in the item's project, saying it can't tell whether that item is in progress, with where and why the file can't be read and the fix. A file not named after an item ID, or in another project, doesn't count. DESIGN.md §2.4's G1 row and §4.4 say so, and the readme's limit is replaced by its cost: a corrupt file blocks every start in its project. Tests: `g1_fails_while_an_item_file_in_the_project_cant_be_read`, and two tests in `tests/bad_data.rs` that started an item beside a broken file now `edit` it instead and check `start`'s refusal |
| Finding 6: nothing at submit requires a criterion | `c08cdba`, option (a): G2 fails an item with no criteria, with the message G1 gives. DESIGN.md §2.4's G2 row says so. Test: `an_item_with_no_criteria_fails_g2` |
| Finding 7: §3 says the criterion commands "work at any time" | `b51549b`: §3 says they and `evidence` work in any state but done, and for an item in review only from an interactive terminal, as `edit` does |
| Finding 8: a check command has no length limit | `ab324b4`, with a limit of 1000 characters rather than 500, since a command is often longer than the sentence it proves. The refusal says to put the steps in a script the check runs. DESIGN.md §2.3 and `--check`'s help say so. Test: added to `criterion_add_refuses_bad_input_and_a_repeated_criterion`, through `criterion add` and `criterion set` |
| Finding 9: §2.4 doesn't say what "content under it" means | `22fc352`: G6's row says the heading is a `#` heading outside code whose first word is the section number, and its content any line before the next heading of the same level or higher that isn't blank, a heading or a code fence |
| Test gap 1: no test of a newer tracker's criterion through `gates` or `submit` | `9046539`, with bug 1, through `start` and `status` too |
| Test gap 2: no test of G6 with an info string on a closing fence, an indented heading, or an empty code block | `a16fd2b`, with bug 2, in the unit tests in `src/gates.rs` |
| Test gap 3: no test that text output escapes a hand-edited criterion or evidence | `3ab8de7`, with finding 3 |
| Test gap 4: no test that `criterion set` and `evidence` leave `written` alone | `e137f93`: `criterion_set_and_evidence_leave_written_alone_from_a_terminal_or_not` |
| Test gap 5: no test that `criterion accept` refuses a done item | `0b6aa89`: `criterion_accept_refuses_a_done_item` |
| Test gap 6: no test pins what G1 does with an unreadable item file | `c1e91fd`, with finding 5 |
| Process: this review isn't independent | Still open. Run review 2 in a new session before merging |
| Process: the code, schemas, tests and fixtures landed as one commit | Can't change now. This round's fixes are one commit each, with their tests |
| Process: `6eb689d` settles part of slice 11 early | Still true, and finding 4 kept it: T5 asks whether each text in `written` is still some criterion's text. Slice 11's prompt now says T5 names it |
| Process: the readme overclaims about fences, and its Limits miss two things | No longer apply: bug 2's fix makes the fence claim true, and findings 1 and 6 are fixed, so neither is a limit now. `3af1c73` brings the readme up to date with every fix |
| Process: the readme has no "Still open" section | Not done in this round. Slices 2–5's open items are still open, and still not carried forward |
| Process: colour for the new commands wasn't added, and the readme doesn't say so | Not done in this round |
| Slice 7 must decide | Left for slice 7. The last item, the check's length limit, is settled at 1000 characters by `ab324b4`, but the printed command still needs escaping when slice 7 prints it |

Seen failing:

- **Bug 1 and test gap 1:** the new test failed against `04fab3f`'s code: `gates` exited 2, with G2's reason.
- **Bug 2 and test gap 2:** the unit test of what must fail failed on the review's first row. The test of what must pass passed before the fix too, so it guards the fix rather than proving the bug.
- **Finding 3 and test gap 3:** the test failed at `criterion set`'s output, then, with that fixed, at `criterion accept`'s warning, then at `criterion remove`'s output, each fixed in turn.
- **Finding 4:** the new test failed with the old warning, "2 written criteria match none of them now, and are dropped."
- **Finding 5 and test gap 6:** the new test failed on `start XX-2`'s exit 0.
- **Finding 6:** the new test failed on `gates`'s exit 0.
- **Finding 8:** the extended test failed on the 1001-character check being taken.
- **Test gaps 4 and 5:** these cover code that already worked, so each test was run against a deliberate break, then the code was restored. Test gap 4's caught `criterion set` changed to add its criterion's text to `written` from a terminal. Test gap 5's caught `criterion accept` with its done check taken out, through its message checks: the exit code stayed 1, since the question it then asked went unanswered.

After all of this, `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo clippy --all-targets -- -D warnings` and all 245 tests pass, 8 more than at `095b250`: 3 in `tests/gates.rs`, 3 in `tests/criteria.rs` (4 new, and the SHA-256 test gone with the hash), and 2 unit tests in `src/gates.rs`. The readme's Try it still prints what it shows.

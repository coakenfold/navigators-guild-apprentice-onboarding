# Slice 6 review 2

A second review of slice 6 (Criteria and simple gates). It checks the branch against [PLAN.md](../PLAN.md)'s Slice 6 prompt and _Done when_ list and its integration risks, and against the [DESIGN.md](../DESIGN.md) sections the slice touches: §2.1's `written`, `criteria`, `verification` and `history`, §2.3's criteria and evidence, §2.4's gates G1, G2 and G6, §2.5's T5, which reads `written`, §2.7's `criterion accept`, §3's commands, refusal shape and exit codes, §4.3's field order and note, §4.4's escaping, lock and one-item-at-a-time rule, §6's trust table and §7's limits. It also checks the conventions in [CLAUDE.md](../../CLAUDE.md), the claims in [slice-6-readme.md](../slice-6-readme.md), and whether each fix that [review 1](slice-6-review-1.md)'s [Outcome](slice-6-review-1.md#outcome) records is in the code, with a test.

|                 |   |
| --------------- | - |
| Date            | 2026-11-17 |
| Branch          | `feat/tracker/slice-6` at `f7e8833`. The working tree has one uncommitted line in `tracker/CLAUDE.md`, about not reading `src/main.rs` whole, and none in the slice's code. The slice is 19 commits on `main`'s `440628f`: the 4 review 1 read (`be91d1f`, `4d2d6ad`, `6eb689d`, `095b250`), then 15 new since review 1's pinned `095b250`: `04fab3f` (review 1), 11 fixes from `9046539` to `0b6aa89`, and `3af1c73`, `5e2b859` and `f7e8833` (the readme and review 1's Outcome) |
| Method          | Written in a new session, with no part in building the slice. Read PLAN.md in full, and DESIGN.md's §2.1, §2.3, §2.4, §2.5, §2.7, §3, §4.3, §4.4, §6, §7 and §9's rules, which cover every section the slice names or changes. Read review 1 in full, the readme, CLAUDE.md, and slice 5 review 2's "Slice 6 must decide". Read the slice's diff against `main`, `src/gates.rs`, `tests/gates.rs` and `tests/criteria.rs` in full, and the functions of `src/main.rs` the diff touches. Ran `cargo fmt --check`, both clippy commands and `cargo test`. Reproduced each finding marked "Reproduced" against the debug binary in a temporary store, using `script` or a Python pseudo-terminal for the terminal-only paths. Ran the readme's Try it. Ran two fixes' tests against the commit before each fix, in a scratch worktree |
| Line references | Point at `f7e8833` |

## Verdict

Slice 6 does what the plan asks, and every fix in review 1's Outcome is in the code. `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo clippy --all-targets -- -D warnings` and all 245 tests pass. Every _Done when_ bullet and every requirement in the prompt has a test.

- **Review 1's fixes hold up.** Both bugs, findings 3 to 9 and all six test gaps are fixed as the Outcome says, each with its test. I ran two of those tests against the code before their fix, and both failed at the exit code, as the Outcome says they did.
- **G1 holds when several agents start at once.** Three rounds of ten `start` commands, run at the same moment in one project, each let exactly one through. No test checks this yet (test gap 1).
- **The readme is accurate**: its test counts, its Try it output and its coverage table all check out.

I found no bug that loses data. The findings that matter most:

- **Finding 2:** from a terminal, `criterion remove` takes a criterion I wrote out of `written` without asking for the ID and without a history entry. So the `script` route that §7 concedes gets past both T5 and the digest, more quietly than it gets past `criterion accept`.
- **Finding 4:** the new `unchecked` note is in the present tense again. Slice 2's review asked for that to be fixed before self-tracking starts, and self-tracking starts after this slice.
- **Finding 3:** `written` protects only a criterion's text, so before start an agent can replace a check I gave a criterion, and nothing records it.
- **Bug 1:** `criterion accept` asks for the ID before it refuses a newer tracker's file.

## Done when

| Check | Result | Tests |
| --- | --- | --- |
| `start` with no criteria is refused by G1 with exit 2 | Pass | [gates.rs:90](../../tests/gates.rs#L90), through `start` and `status <id> in-progress` |
| …and so is `start` while another item in the project is in progress | Pass | [gates.rs:114](../../tests/gates.rs#L114), [gates.rs:142](../../tests/gates.rs#L142) (in review), [gates.rs:157](../../tests/gates.rs#L157) (an unreadable file), [gates.rs:196](../../tests/gates.rs#L196) (every reason). Two starts at once are checked only by hand (test gap 1) |
| A manual criterion with no evidence fails G2 | Pass | [gates.rs:287](../../tests/gates.rs#L287), through `gates`, `submit` and `status <id> done`, and [gates.rs:325](../../tests/gates.rs#L325) for an item with no criteria |
| A `design` naming a missing section fails G6 | Pass | [gates.rs:404](../../tests/gates.rs#L404), and the unit tests at [gates.rs:496](../../src/gates.rs#L496) and [gates.rs:512](../../src/gates.rs#L512). But see finding 6 |
| …and a numbered section with content under it passes | Pass | [gates.rs:377](../../tests/gates.rs#L377), and the unit tests at [gates.rs:489](../../src/gates.rs#L489) and [gates.rs:557](../../src/gates.rs#L557) |
| A `design` whose file is a symlink to one outside the root fails G6 | Pass | [gates.rs:435](../../tests/gates.rs#L435), with a symlinked folder too, [gates.rs:474](../../tests/gates.rs#L474) for `..` and an absolute path, and [gates.rs:492](../../tests/gates.rs#L492) for a pipe |
| `gates` lists every failure and leaves the item file byte-for-byte unchanged | Pass | [gates.rs:528](../../tests/gates.rs#L528), and every test through `refused_by_gate` ([gates.rs:23](../../tests/gates.rs#L23)) |
| A refused `submit` exits 2 and also changes nothing | Pass | [gates.rs:576](../../tests/gates.rs#L576), [gates.rs:287](../../tests/gates.rs#L287) |
| `create --done-when a --done-when b` adds two criteria, and their two texts to `written` | Pass | [criteria.rs:75](../../tests/criteria.rs#L75) |
| `evidence` on a script criterion exits 1 | Pass | [criteria.rs:365](../../tests/criteria.rs#L365) |
| `criterion accept` is refused outside an interactive terminal | Pass | [criteria.rs:452](../../tests/criteria.rs#L452), with the ID piped in. Typing it under `script`: [criteria.rs:483](../../tests/criteria.rs#L483), [criteria.rs:554](../../tests/criteria.rs#L554), [criteria.rs:663](../../tests/criteria.rs#L663), [criteria.rs:678](../../tests/criteria.rs#L678). But see bug 1 |
| _Prompt:_ `criterion add`, with `--check` or manual | Pass | [criteria.rs:146](../../tests/criteria.rs#L146), [criteria.rs:189](../../tests/criteria.rs#L189) |
| _Prompt:_ `criterion set <n> --check` and `criterion remove` | Pass | [criteria.rs:232](../../tests/criteria.rs#L232), [criteria.rs:267](../../tests/criteria.rs#L267) |
| _Prompt:_ `written` taken at create, and an interactive-only `criterion accept` with `via: "criterion-accept"` | Pass | [criteria.rs:75](../../tests/criteria.rs#L75), [criteria.rs:103](../../tests/criteria.rs#L103), [criteria.rs:483](../../tests/criteria.rs#L483). But see findings 2 and 3 |
| _Prompt:_ G6 reads `design` relative to the root, and fails a file outside it after symlinks | Pass | [gates.rs:377](../../tests/gates.rs#L377), [gates.rs:435](../../tests/gates.rs#L435), [gates.rs:474](../../tests/gates.rs#L474) |
| _Prompt:_ give `tests/states.rs`'s items a criterion through the shared helper | Pass | `Env::create` ([mod.rs:314](../../tests/common/mod.rs#L314)) calls `add_criterion` ([mod.rs:329](../../tests/common/mod.rs#L329)). All 27 tests in `tests/states.rs` still test moves |
| _Integration risks:_ a fixture of the item file's new shape | Pass | [fixtures.rs:167](../../tests/fixtures.rs#L167), and the tests over every slice's fixtures. But see [Process](#process) |

## Bugs

1. **`criterion accept` asks for the ID before it refuses a newer tracker's file.** [`criterion_accept`](../../src/main.rs#L1527) loads the item at [main.rs:1536](../../src/main.rs#L1536) and asks at [main.rs:1558](../../src/main.rs#L1558). Only `update_item`, at the very end, refuses the file. `delete` checks for unknown fields before its prompt ([main.rs:2365-2374](../../src/main.rs#L2365-L2374)), and since `9046539` so do `gates` ([main.rs:1680](../../src/main.rs#L1680)) and every move ([main.rs:2045](../../src/main.rs#L2045)).

   Reproduced: I edited XX-1's one criterion to `{ "text": "Errors say what to do, reworded", "ask": [ … ] }`, a judge criterion as slice 12 will write it. `criterion accept XX-1`, run at a pseudo-terminal, printed:

   ```
     1. Errors say what to do, reworded  (new to written)
   Accepting also drops this criterion from written. It was removed, or reworded by hand:
     - Errors say what to do
   Type XX-1 to confirm, or anything else to cancel:
   ```

   I typed XX-1, and it exited 3 with "… has fields this version of tracker doesn't know, so rewriting it would lose them: criteria. … Nothing was written."

   Nothing is lost. But I was asked to decide something the command couldn't do. The warning also describes the judge criterion as if it were plain text, which is the same misreading review 1's bug 1 fixed for the gates. Fix: call `refuse_newer_file(store, &id, &before)` right after the load, before the done check, as `delete` does. Test gap 2.

## Security

2. **From a terminal, `criterion remove` takes a written criterion out of `written` without asking for the ID and without a history entry, so the `script` route gets past T5 and the digest at once.** [`criterion_remove`](../../src/main.rs#L1504-L1508) takes the text out of `written` whenever `tty::is_interactive()` is true, and adds no history entry. §2.1 ([DESIGN.md:93](../DESIGN.md#L93)) gives history entries only to moves, `edit` and `criterion accept`. `criterion accept`, the other command that changes `written`, asks for the ID and adds an entry that the digest lists (§2.6, §2.7).

   Reproduced: XX-1 was created with "Search filters by tag" and "Empty results say so". I then ran `script -q /dev/null tracker criterion remove XX-1 2 </dev/null`, which is what `Env::in_terminal` ([mod.rs:136](../../tests/common/mod.rs#L136)) runs in the tests. It asked nothing and exited 0. Afterwards both `written` and `criteria` held only "Search filters by tag", and the file had no `history`. `criterion accept` then said "XX-1's written criteria already match its criteria. Nothing changed."

   Why it matters: §7 accepts that the terminal check can be got round with `script`, and relies on the digest to catch it. As §6 puts it ([DESIGN.md:787](../DESIGN.md#L787)), "the digest lists every human action and shelve, mine included, so I'd see one I didn't make". An agent that runs `criterion accept` this way still leaves a `criterion-accept` entry, and has to answer the prompt. One that runs `criterion remove` leaves nothing: before start, slice 11's T5 sees `written` agree with the criteria, and slice 16's digest has no entry to list. It's the quietest way to drop a criterion I wrote, which is what `written` exists to stop. Fix:

   - **(a)** `criterion remove` leaves `written` alone from a terminal too. Removing a criterion I wrote then always ends with `criterion accept`, which names the text, asks for the ID and leaves the entry the digest lists. `criterion add` can keep adding its text to `written` from a terminal, since that only makes T5 stricter. §2.1's `written` row, the readme's table and [criteria.rs:308](../../tests/criteria.rs#L308) follow.
   - **(b)** Keep the behaviour, and give a terminal `criterion remove` that changes `written` a history entry, with a new `via` and the removed text, which slice 16's digest lists alongside `criterion accept`.

   I'd pick (a): then only one command can change `written` in a way that weakens T5, and it's the one that already asks and records.

3. **`written` keeps only the text, so before start an agent can replace a check I gave a criterion, and nothing records it.** Before start, §2.5's T5 ([DESIGN.md:321](../DESIGN.md#L321)) fires only when a text in `written` is gone. [`criterion_set`](../../src/main.rs#L1460) changes a check, which nothing in `written` holds.

   Reproduced: from a terminal (under `script`), `criterion add XX-1 "Sorts newest first" --check "cargo test sorts_newest_first"` put the text in `written`. Then, not from a terminal, `criterion set XX-1 2 --check "cargo test sorts"` exited 0, with `written` unchanged and no history. Slice 11's fingerprint, taken at start, would cover only the new check.

   §3 ([DESIGN.md:439](../DESIGN.md#L439)) says that before start "an agent can add criteria and give each one a check or judge", and §2.5 that "giving a criterion its first check or judge before start" doesn't count. Both read as a criterion's first check. Neither says what happens to a check I gave, and §6 has no row for it. T13 catches the new check only if it passes at start. A wider filter that also runs a test failing for some other reason gets past it.

   In the self-tracking plan, criteria come from `--done-when`, so the agent writes every check and this doesn't arise. It matters when I give the check myself, as §2.3 encourages. Fix:

   - **(a)** For a criterion that had a check when it was written, `written` keeps the check as well, such as `{ "text": …, "check": … }`, and T5 compares both. That changes `written`'s shape again, so it's best decided before self-tracking writes TR items, as review 1's finding 4 was.
   - **(b)** Say in §2.1, §3 and §6 that `written` protects a criterion's text, not its check, and that before start an agent can replace a check I gave, which only T13 can catch. Add a line to §7.

   I'd pick (b). The plan has me write the texts and the agents write the checks, and (a) makes T5 and `criterion accept` more complex for a case that rarely arises.

I also checked:

- **Every new message escapes text from the file where it's printed.** G1's message about an unreadable file, G2's criterion text and G6's paths all go through `text::for_terminal` in `fail` ([main.rs:500](../../src/main.rs#L500)), and `--json` through `json_text`.
- **`criterion accept` changes nothing if the item changed while I decided**: it takes the lock through `Store::lock_unchanged` ([main.rs:1562](../../src/main.rs#L1562)), as [criteria.rs:678](../../tests/criteria.rs#L678) tests.
- **G6 checks the canonical path, then reads it.** In between, a folder on the way could be swapped for a symlink, or the file for a pipe. Only a hand edit timed to that instant could do it, and all it learns is whether G6 passed. This is the same gap that slice 5's readme leaves open for `items/`.
- **A negative or non-numeric criterion number** is refused by clap with exit 1, as [criteria.rs:258-263](../../tests/criteria.rs#L258-L263) tests for a word.

## Where the code and DESIGN.md disagree, or the design is unclear

4. **The new `unchecked` note is in the present tense again, and §2.1, §4.3 and §9 still describe `unchecked` as it was before the gates.** [`UNCHECKED_NOTE`](../../src/states.rs#L37-L38) now reads "No checks ran: this version of tracker runs only gates G2 and G6, which don't test the work, so nothing checked it."

   [Slice 2 review 2's item 9](slice-2-review-2.md#L143) asked for a note that stays true. Slice 3 review 1 ([slice-3-review-1.md:116](slice-3-review-1.md#L116)) and the readmes of slices 3, 4 and 5 carried it forward as "cheapest to change before self-tracking starts after slice 6". This slice rewrote the note but kept "this version". Self-tracking runs on this binary, so slice 7's TR item will reach done with this note, and slice 7's binary will then show "this version of tracker runs only gates G2 and G6" under it, which won't be true of that version.

   The design has the same problem. §2.1's `verification` row ([DESIGN.md:89](../DESIGN.md#L89)) says `unchecked` is "for items done before the gates existed", and §4.3 ([DESIGN.md:682](../DESIGN.md#L682)) quotes the old note as its example. §9 ([DESIGN.md:843](../DESIGN.md#L843)) says that until slice 9, everything that passes the gates is auto-verified, but slices 6 and 7 record `unchecked`.

   Fix:

   - Write a note that stays true, such as "Submitted before tracker ran checks. Gates G2 and G6 passed, but nothing tested the work."
   - In §2.1's row, say `unchecked` covers items done before the gates proved the work against a commit (slice 8).
   - In §4.3, quote the new note.
   - In §9, say "reaches done without me" rather than "auto-verified".

5. **A failing `gates` doesn't say the item can't be submitted anyway, and its JSON has no `status`.** [`gates_dry_run`](../../src/main.rs#L1683-L1685) returns the refusal before it looks at the state, and the refusal's fields are `id`, `result`, `gates` and `failures` ([gates.rs:108-113](../../src/gates.rs#L108-L113)). The passing output has both the line and the field ([main.rs:1691-1704](../../src/main.rs#L1691-L1704)).

   Reproduced: open XX-1, with a manual criterion and no evidence. `gates XX-1 --json` exited 2 with "XX-1 would be refused at submit: G2 failed …" and no `status`. Once evidence was recorded, it passed and said "It's open, and only an in-progress item can be submitted."

   §3 ([DESIGN.md:434](../DESIGN.md#L434)) has `gates` answer "what would block a submit?", and for an open item the state is part of the answer. An agent reading only the refusal's JSON can't tell, and fixing every reason listed still won't let it submit. Low severity. Fix: add `status` to a gate refusal's fields, both in `error.schema.json` and in §3's description of the refusal, and print the same line under the reasons.

6. **G6 counts an HTML comment as content, so a heading with only `<!-- TODO -->` under it passes.** §2.4's G6 row ([DESIGN.md:287](../DESIGN.md#L287)) counts "any line … that isn't blank, a heading or a code fence", and [`lines`](../../src/gates.rs#L389-L418) does exactly that.

   Reproduced: a design doc with `### 2.6 Sorting`, a blank line, `<!-- TODO: write this -->`, then `### 2.7 Next`. With `design` set to `DESIGN.md#2.6`, `gates` passed G6.

   The rendered section is empty, and an empty section is what G6 exists to catch (review 1's bug 2). A TODO comment is the usual placeholder in markdown precisely because it doesn't render. Low severity. Fix:

   - **(a)** A line that's only an HTML comment, or that sits inside a comment spanning several lines, isn't content, and §2.4's row says so.
   - **(b)** Leave it, and list it in the readme's limits beside setext headings.

   I'd pick (a). A comment is easy to recognise, and unlike a "TBD" in plain text, nobody reading the rendered doc would see it.

## Test gaps

1. **No test that two `start` commands run at once in one project let only one through.** G1 runs under the lock: `move_item` takes it at [main.rs:1961](../../src/main.rs#L1961), and `apply` reads every item under it at [main.rs:2050](../../src/main.rs#L2050). It holds: three rounds of ten `start --json` commands, run at once on XX-1 to XX-10, each gave one exit 0 and nine exit 2. But that was checked by hand. Several agents starting work at once is exactly what G1's one-item-at-a-time rule is for, and slice 7 has to run checks outside the lock, which is what could break it ([Slice 7 must decide](#slice-7-must-decide)). Add a test like `ten_creates_at_once_get_ten_different_ids` ([create_and_list.rs:259](../../tests/create_and_list.rs#L259)).
2. **No test that `criterion accept` refuses a newer tracker's file before it asks** (bug 1). [gates.rs:241](../../tests/gates.rs#L241) covers `gates` and the moves only.

## Process

- **This review is independent.** It was written in a new session, which settles review 1's first process note. It read DESIGN.md by section, as the review skill now asks, rather than whole. The sections are listed under Method.
- **The fixes follow review 1 closely.** Each is its own commit with its tests, and option (a) was taken wherever there was a choice, as the Outcome says. I spot-checked its "Seen failing" claims. Bug 1's test, taken from `9046539`, failed against `04fab3f`. Finding 6's test, taken from `c08cdba`, failed against `c1e91fd`. Both failed at their exit-code check.
- **`05f4bba` changed what `written` holds**, from SHA-256 hashes to texts. A binary built from `4d2d6ad` up to `3ab8de7` wrote hashes, and this version reads each of them as a text: `criterion accept` would list every 64-character hash as a dropped criterion, and slice 11's T5 would fire on it. No store should hold such an item, since self-tracking hasn't started. But if this branch was ever installed with `cargo install`, start self-tracking from a fresh store.
- **The readme is right about what it covers.** Its 245 tests and each file's count, its Try it output, which I ran exactly as written, its coverage table's test names, and its table of fixes all check out. One sentence overclaims: it says `tests/fixtures/slice-6/` holds "three item files this slice's binary wrote" ([slice-6-readme.md:199](../slice-6-readme.md#L199)). In fact `05f4bba` edited XX-13's and XX-14's `written` by hand, from hashes to texts, as review 1's Outcome says. They still round-trip byte for byte through this binary, so they do their job, but the sentence should say they were edited.
- **The uncommitted line in `tracker/CLAUDE.md`** is about how reviews read `src/main.rs`, not about the slice. Commit it separately, here or on `main`, so it doesn't land in a fix commit.

## Still open, carried forward

- **Review 1, Process: the readme has no "Still open" section.** Slices 2–5's open items still aren't carried forward, which [slice-6-readme.md:233](../slice-6-readme.md#L233) admits. One of them now reaches this slice's text: slice 3 review 1's low-severity item 5, invisible characters. `check_printable` accepts a zero-width space in a criterion's text, so an agent can remove a criterion I wrote and add one that looks the same. Reproduced: after `criterion remove` of "Search filters by tag" and `criterion add` of the same words with U+200B before " tag", `criterion accept` listed `1. Search filters by​ tag  (new to written)` and `- Search filters by tag` as dropped. The two look identical on screen. T5 will still fire, which is safe, but its hold and `criterion accept`'s warning will show two texts I can't tell apart.
- **Review 1, Process: colour for the new commands** still isn't added. [slice-6-readme.md:233](../slice-6-readme.md#L233) says so, but "Not built yet" doesn't.
- **Slice 2 review 2's item 9, the `unchecked` note's tense**, is now finding 4.

## Slice 7 must decide

- **Everything in review 1's [list](slice-6-review-1.md#slice-7-must-decide) still applies**, as its Outcome says:
  - The shared test helper's check is `true` ([mod.rs:329-331](../../tests/common/mod.rs#L329-L331)).
  - Where G3 and G4 go in `gates::submit`.
  - `secs` is missing from `gates.schema.json` and `error.schema.json`.
  - What a submit's record says once checks run. Finding 4 settles the wording before then.
  - `criterion set --check` after `start` must run the new check once.
  - A check must be escaped when it's printed before it runs.
- **G1 has to run again under the lock after `start`'s checks.** Today `start` holds the lock from reading the item to writing it, so G1 can't race (test gap 1). §4.4 says the lock isn't held while checks run, so slice 7's `start` will let it go to run them. Two starts in one project could then both pass G1 before either writes. Slice 7 should take the lock again after the checks, re-read the item as `Store::lock_unchanged` does, run G1 again before writing, and test two starts at once.
- **What `gates` runs for an item that isn't in progress.** Today it runs G2 and G6 in any state. From slice 7, it would also run every criterion's check and the project's checks, which are shell commands, on an open or done item. §2.4 says an agent can ask "what's left?" at any time. Slice 7 should decide whether that includes running checks for an item that can't be submitted. Finding 5's `status` helps either way.

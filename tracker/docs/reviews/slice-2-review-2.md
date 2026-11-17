# Slice 2 review 2

A second review of slice 2 (States) against [PLAN.md](../PLAN.md)'s Slice 2 prompt and _Done when_ list, the [DESIGN.md](../DESIGN.md) sections it touches (§2.1, §2.2, §2.6–§2.8, §3, §4.3, §4.4), [CLAUDE.md](../../CLAUDE.md)'s conventions, and the fixes recorded after [review 1](slice-2-review-1.md).

|                 |                                                                                                                                      |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| Date            | 2026-11-16                                                                                                                           |
| Branch          | `feat/tracker/slice-2` at `44abdb8` (`refactor: moving files into docs`), working tree clean                                         |
| Method          | Read the code, tests, schemas and docs, ran the test command, and reproduced each finding below against the debug binary in a temporary store |
| Line references | Point at `44abdb8`                                                                                                                   |

## Verdict

**Slice 2's behaviour is faithful to the plan, but the branch is red.** The code does what PLAN.md's slice 2 asks. Every _Done when_ check has a passing test, and the review 1 fixes landed as described in [slice-2-readme.md](../slice-2-readme.md#fixes-after-review-1):

- `submit` prints §3's shape.
- A refused move carries `allowed` as JSON.
- `interactive` is worked out in one function and is tested as `true` under `script`.
- Reasons are capped at 500 characters.
- `--help` shows that `--reason` is needed.
- Reopening moves the record to `previous_verification`.

I found no new bugs in the state machine. A ten-way concurrent `start` still gives one winner and one history entry.

The last commit moved DESIGN.md, PLAN.md and the reviews into `docs/` without updating anything that points at them. As a result:

- `cargo test` fails: 4 of the 5 tests in `tests/docs_links.rs`.
- About 100 relative links across the docs are broken.
- CLAUDE.md, PLAN.md and DESIGN.md still say DESIGN.md sits at the crate root.

DESIGN.md §9 says each slice must pass the test command before the next one starts, so slice 2 isn't finished until this is fixed. Fixing it is mechanical, and finding 1 lists what to change.

Apart from that, the rest is items from review 1 that are still open (one security item, which still reproduces) and a few places where DESIGN.md and the schemas need a decision before slices 6 and 9 build on them.

## Done when

| Check | Result | Test in [tests/states.rs](../../tests/states.rs) |
| --- | --- | --- |
| open → in-progress → done → open, through the commands and through `status` | Pass | `an_item_goes_open_in_progress_done_open_through_the_commands`, `…_through_status` |
| `status <id> open` without `--reason` exits 1 | Pass | `status_open_and_status_shelved_need_a_reason` |
| `shelve` works from open and from in-progress, and exits 1 without a reason | Pass | `shelve_works_from_open_and_from_in_progress`, `shelve_without_a_reason_exits_1` |
| `reopen` brings a shelved item back to open | Pass | `reopen_brings_a_shelved_item_back_to_open` |
| open → done exits 1 and lists the allowed moves | Pass | `open_to_done_exits_1_and_lists_the_allowed_moves`, `every_move_the_diagram_doesnt_draw_is_refused` |
| Every move adds exactly one history entry | Pass | `every_move_adds_exactly_one_history_entry`, `a_refused_move_adds_no_history_entry` |
| _Prompt:_ `submit` writes `by: "unchecked"`, the time and a note | Pass | `check_unchecked` |
| _Prompt:_ each history entry has `from`, `to`, `via` and `interactive` | Pass | `check_last_move`, `a_move_from_a_terminal_records_interactive_true` |
| _§9:_ `cargo fmt --check && cargo clippy -- -D warnings && cargo test` | **Fail** | fmt and clippy pass. `cargo test --no-fail-fast`: 89 pass, 4 fail, all in `docs_links` (finding 1) |

Checked by hand in a temporary store at `44abdb8`:

- Ten `start XX-1` at once: one exits 0, nine exit 1, one history entry.
- done → reopen → shelve → reopen: `verification` is absent and `previous_verification` keeps the unchecked record.
- A 500-character reason of `é` (1,000 bytes) is accepted, so the limit counts characters, as the schema's `maxLength` does.
- A tab in a reason is refused.

## Fix before slice 3

### 1. The move into `docs/` broke the test suite, the links, and what the docs say about themselves

`44abdb8` moved `DESIGN.md` and `PLAN.md` from the crate root into `docs/`, and the three reviews into `docs/reviews/`. Nothing that refers to them was updated, so four things broke.

**The tests.** [tests/docs_links.rs](../../tests/docs_links.rs) has four failing tests:

- `design_doc_still_links_to_the_files_moved_with_it` reads `DESIGN.md` from the crate root ([line 53](../../tests/docs_links.rs#L53)).
- `docs()` lists `DESIGN.md` and `PLAN.md` at the root ([lines 66–80](../../tests/docs_links.rs#L66-L80)), so `relative_links_in_the_docs_resolve` and `line_anchors_name_lines_that_exist` panic trying to open them.
- `the_docs_folder_is_checked` expects `docs/slice-1-review-1.md` ([line 89–92](../../tests/docs_links.rs#L89-L92)).

`docs()` also reads only the top of `docs/`, not `docs/reviews/`. So even once the paths are fixed, no review's links would be checked. That includes this one.

**The links.** About 100 relative links no longer resolve:

| File | Broken | Examples |
| --- | --- | --- |
| CLAUDE.md | 2 | `DESIGN.md`, `PLAN.md` |
| docs/DESIGN.md | 10 | `judge-eval/`, `walkthrough.html`, `schemas/`, `schemas/item.schema.json`, `schemas/error.schema.json` |
| docs/slice-1-readme.md | 7 | `../DESIGN.md`, `../PLAN.md`, `slice-1-review-1.md` |
| docs/slice-2-readme.md | 4 | `../DESIGN.md`, `../PLAN.md`, `slice-2-review-1.md` |
| docs/reviews/*.md | 78 | every `../src/…`, `../tests/…`, `../schemas/…` and `../CLAUDE.md`, and every link to a readme |

DESIGN.md's links to `judge-eval/`, PLAN.md and `walkthrough.html` were a slice 1 _Done when_ check. Two of the three are now broken.

**What the docs say.** Several places still say DESIGN.md is at the crate root:

- [CLAUDE.md](../../CLAUDE.md#L3-L5) says "DESIGN.md, PLAN.md, walkthrough.html and `judge-eval/` sit here at the crate root". Every slice session loads this file first.
- DESIGN.md §8 ([line 805](../DESIGN.md#L805)) says the files moved together "so the relative links keep working" and "T8 watches the real design". Neither is true now: DESIGN.md is no longer next to the files it links to.
- DESIGN.md §4.2's TR example has `"design": "DESIGN.md"` ([line 514](../DESIGN.md#L514)).
- PLAN.md's self-tracking checkpoint says `--design DESIGN.md` ([line 144](../PLAN.md#L144)).

Those last two matter beyond links. TR's `design` setting is what G6 resolves and T8 watches. If it's registered as `DESIGN.md`, G6 fails every TR item that sets `design` from slice 6 on. T8 then never fires for changes to the real spec, which is the self-tracking case §8 calls out.

**Fix:**

1. Point the test at `docs/DESIGN.md`, and make `docs()` walk `docs/` recursively.
2. Rewrite the broken links:
   - DESIGN.md: `../judge-eval/`, `../walkthrough.html`, `../schemas/…`
   - readmes: `DESIGN.md`, `PLAN.md`, `reviews/slice-n-review-m.md`
   - reviews: `../../src/…`
   - CLAUDE.md: `docs/DESIGN.md`, `docs/PLAN.md`
3. Update CLAUDE.md's layout paragraph, DESIGN.md §8, DESIGN.md §4.2's TR example and PLAN.md's checkpoint to `docs/DESIGN.md`.

Alternatively, move DESIGN.md and PLAN.md back to the crate root and keep only the readmes and reviews in `docs/`. That's the smaller change, and it keeps §8's reasoning true.

Run the full test command before committing. If the move is kept, use `docs:` rather than `refactor:` as the commit type, since no code changed.

**A side effect to decide on.** Once the reviews are checked, `line_anchors_name_lines_that_exist` will check review 1's `#L…` anchors against today's files, but they point at `54d8f66`. They exist today only by luck of file length. Either leave reviews out of the anchor check (the link check alone is enough), or have reviews link to the files at their commit.

## Security

### 2. A planted `<id>.json.tmp` symlink still redirects a move's write (open since slice 1 review 2)

This still reproduces at `44abdb8`:

```
$ echo precious > victim
$ ln -s "$PWD/victim" $TRACKER_HOME/items/XX-1.json.tmp
$ tracker start XX-1            # exit 0
$ head -2 victim
{
  "id": "XX-1",
$ ls -l $TRACKER_HOME/items/XX-1.json
XX-1.json -> …/victim
```

[`write_file`](../../src/store.rs#L421-L448) opens the fixed name `XX-1.json.tmp` with `File::create`, which follows the link and truncates its target. The `rename` then moves the link itself into place, so the item file becomes a symlink to the victim, and later reads go through it. `save_item` checks the item path with `symlink_metadata` but not the temporary path.

Exploiting it needs write access to the 0700 store, and anyone with that can already make `tracker` run commands (§7), so the extra exposure is small. Slice 2 widened it, though: every move now writes an item file. And the fix is about five lines:

- Open the temporary file with `OpenOptions::new().write(true).create_new(true)`, which refuses to follow an existing link or reuse an existing file.
- Remove a leftover `.tmp` first, or use a name with the process ID in it.

It's cheaper to do now than in slice 5. Every later slice adds writers: `config.json` in slice 3's `edit`, and `deleted.json`, `digests.json` and the judge cache after that.

## Low severity

Items 3–7 were raised in review 1 and are still open, as the readme says. Each was re-checked at `44abdb8`. Items 8 and 9 are new.

3. **One item file with a `via` this version doesn't know makes `list` exit 3 for the whole store.** A history entry with `"via": "edit"`, which slice 3 adds, gives "unknown variant `edit`". `list` then exits 3, though `start` on any other item still works. Slice 5's skip-and-report is the planned answer. A cheaper one fits now: give [`Via`](../../src/item.rs#L74-L82) a `#[serde(other)] Unknown` variant. The file then loads and lists, and [`lost_on_rewrite`](../../src/item.rs#L204-L220) already refuses to write it back, because the history entry would change. That's the behaviour you want from an older binary: read it, never rewrite it.
4. **A hand-edited timestamp is still blamed on "a newer tracker".** With `created` set to `2026-11-16T02:00:00+00:00`, a valid time that jiff writes back as `…Z`, a move exits 3 with "It was probably written by a newer tracker, so use that one" ([store.rs:300-308](../../src/store.rs#L300-L308)). Refusing is right. Telling me to use a newer binary is wrong in this case. Say which field would change and leave out the guess, or guess only for fields this version doesn't know at all.
5. **`status <id> closed` is refused.** §1 calls `done` "the course's 'closed' state" and says `status` "still works as an alias". Clap's error lists the valid states, which is enough to recover. Accepting `closed` as an alias for `done`, or adding a hint, would match §1 better. It's a design call.
6. **The move table has one outcome per command and state.** [`states::find`](../../src/states.rs#L37-L42) returns the first `(via, from)` match. This now shows up in the JSON too. [`allowed_json`](../../src/states.rs#L68-L81) tells agents that `submit` from in-progress leads `"to": "done"`, which stops being true in slice 9, when it can lead to review. `error.schema.json` makes `to` a single state. Decide now whether `to` becomes a list, or whether `submit`'s entry says `done` and the hold is explained elsewhere. Renaming or changing the type of a JSON field later is a breaking change (§3).
7. **A move on an item whose project isn't in `config.json` succeeds.** I re-checked: `start XX-5` with the project removed exits 0. §3 allows it ("a command that takes an item ID needs no project"). Slice 7 runs checks in the project root, and slice 8 needs the root for git, so this has to become an error by then. Decide which commands still work without a project, such as `show`, `reopen`, `shelve` from open, and `delete`.
8. **`shelve` from in-progress skips §2.8's rules, and the readme doesn't say so.** §2.8 and §3 require a clean tree and passing project checks before an in-progress item is shelved. PLAN.md puts that in slice 8, so this is expected. But [slice-2-readme.md](../slice-2-readme.md)'s "Not built yet" and "Limits" sections don't mention it, while they do list every other deferral. Add one line.
9. **The `unchecked` note is written into item files in the present tense.** "No gates ran: this version of tracker has none yet" is saved in `verification.note`, and moves to `previous_verification` on reopen. After slice 6 the files will still say "this version… has none yet". Something tense-free, such as "Submitted before tracker had gates, so nothing checked the work", stays true. Change it now, while only test fixtures hold the old text. Those fixtures stay as they are.

## Where the code and DESIGN.md disagree, or DESIGN.md is unclear

1. **§3 says `submit` lists "every gate and trigger with its id and result", but `holds` lists only the triggers that fired.** §3's own example, and `submit.schema.json` ("Every trigger that fired"), list only fired triggers in `holds`, with no `ok` field. That's fine for a hold. It doesn't match "every … trigger with its … result", and an agent can't tell "T3 didn't fire" from "T3 is disabled" or "T3 doesn't exist yet". Before slice 9, either add a `triggers` array shaped like `gates`, with `ok`, and keep `holds` as the fired subset, or reword §3 to say only fired triggers are listed. The first is the more useful option for agents, and it only adds a field.
2. **§2.1 says every history entry "has the same fields", but they don't.** `reason` is present only for shelve, reopen and (later) reject, and `fields` only for edit and criterion accept ([DESIGN.md §2.1](../DESIGN.md#L93)). The schema and §4.3's notes get this right. Reword §2.1 to "the same required fields", so a reader doesn't expect `reason: null` everywhere.
3. **`error.schema.json` closes the error shape before slice 6 adds to it.** It has `additionalProperties: false`, and `allowed[].via` only allows `start`, `submit`, `shelve` and `reopen`. Slice 6 adds `id`, `result`, `gates` and `failures` to gate refusals, and slice 9 adds `approve` and `reject` to `allowed`. Each is additive and the schema test will catch it, so this isn't a defect. It's listed so slices 6 and 9 extend the schema in the same commit as the code. Its description already says so for slice 6, but not for the `via` enum.
4. **Items in review can't be moved, and the message says so honestly.** `allowed` is `[]` with "this version of tracker can't do either yet". That matches the readme, and slice 9's _Done when_ now checks the change. No action.

## Test gaps

Gaps 2–5 below are from review 1 and are still open. 1 and 6 are new.

1. **The docs tests don't look in `docs/reviews/`** (finding 1). Once the paths are fixed, make `docs()` recursive, and decide how line anchors in reviews are checked.
2. **Concurrent moves on one item.** This is correct by hand, re-checked above, but untested. A ten-way `start` test, like slice 1's ten-way `create`, would guard the lock-then-read order in [`move_item`](../../src/main.rs#L581-L591).
3. **The "would lose fields" refusal through a move command.** It's tested only at the store level. Add a test that writes a fixture with an unknown top-level field (or, after item 3, an unknown `via`), runs `start`, and expects exit 3 and an unchanged file. During self-tracking, the installed binary is a slice behind the code, which is exactly when this path runs.
4. **`created` unchanged by a move.** No test checks it.
5. **A move on an item whose project isn't registered.** No test pins today's behaviour (item 7).
6. **A `via` from a later slice.** Once item 3 is decided, add a fixture with an `edit` entry, and check that `list` either shows it or reports it, and that a move on it exits 3 without writing.

## Process

- **`44abdb8` was committed with a failing test suite.** CLAUDE.md's test command and PLAN.md's build loop both ask for the full suite before each commit, and from slice 7 on, `tracker submit` would refuse this commit under G4. Run the test command before moving files, too. The `docs_links` tests exist to catch exactly this.
- **The readme's status line needs an update once finding 1 is fixed.** It says "Done. Every _Done when_ check passes", which is still true, but the branch currently fails §9's test command. Once fixed, record review 2's outcome there the way review 1's was.
- **The readme's file references need the new paths** ([slice-2-readme.md](../slice-2-readme.md) links `slice-2-review-1.md` as a sibling, but it's now in `reviews/`). This is covered by finding 1's link fixes.

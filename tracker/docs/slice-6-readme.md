# Slice 6: Criteria and simple gates

`tracker` is a command-line work tracker, written in Rust, that keeps one central list of work items across many projects and decides when an item is really done. This document covers **slice 6** of the 16 in [PLAN.md](PLAN.md): what it built, how it works, and how to test it. The earlier readmes cover the rest: [slice 1](slice-1-readme.md) the store, `create` and `list`, [slice 2](slice-2-readme.md) the states and `history`, [slice 3](slice-3-readme.md) priority, kind, labels, `edit` and `list`'s filters, [slice 4](slice-4-readme.md) `show` and `delete`, and [slice 5](slice-5-readme.md) bad data and polish.

After slice 6 you can **write Done-when criteria, and get refused when they're missing or incomplete**. The spec is [DESIGN.md](DESIGN.md). This slice implements [§2.3](DESIGN.md#23-done-when-criteria)'s script and manual criteria, `written` from [§2.1](DESIGN.md#21-items), `criterion accept` from [§2.7](DESIGN.md#27-human-actions), and gates G1, G2 and G6 from [§2.4](DESIGN.md#24-gates-is-it-finished), with the `gates` dry run from [§3](DESIGN.md#3-interface). It's the last slice before `tracker` starts tracking its own build (PLAN.md's checkpoint).

| | |
| --- | --- |
| Status | Every _Done when_ check has a test, and every test passes, as does the test command. PLAN.md has slice 6 as `in review`. From [review 1](reviews/slice-6-review-1.md), both bugs, both security findings, the five DESIGN.md findings and all six test gaps are done ([below](#fixes-after-review-1)). The branch isn't merged into `main`. |
| Branch | `feat/tracker/slice-6`, from `main` at `440628f` (slice 5 merged): `be91d1f` (sha2), `4d2d6ad` (the slice's code, schemas, tests and fixtures, in one commit, since the gates change what most existing tests set up), `6eb689d` (DESIGN.md §2.1, §2.3, §3 and §4.3), `095b250` (this readme and PLAN.md's status), `04fab3f` (review 1), then review 1's fixes, `9046539` to `0b6aa89`, and this readme's record of them |
| Test command | `cargo fmt --check && cargo clippy -- -D warnings && cargo test`, run in `tracker/` |
| Tests | 248: 39 in `tests/create_and_list.rs`, 34 in `tests/organise.rs`, 27 in `tests/states.rs`, 24 in `tests/show_and_delete.rs`, 23 in `tests/criteria.rs`, 22 in `tests/gates.rs`, 14 in `tests/item_files.rs`, 13 in `tests/bad_data.rs`, 11 in `tests/docs_links.rs`, 9 in `tests/polish.rs`, 8 in `tests/fixtures.rs`, 6 in `tests/output.rs`, 5 in `tests/schemas.rs`, 4 in `tests/store_writes.rs`, 2 in `tests/item_ids.rs`, 5 unit tests in the library (4 in `src/gates.rs`, 1 in `src/store.rs`) and 2 in `src/main.rs` |

## Contents

- [What's in this slice](#whats-in-this-slice)
- [Try it](#try-it)
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

- **Criteria.** An item has a **Done when** list, `criteria`. Each criterion has `text`, and either a `check` (a script criterion) or none (a manual one). A manual criterion can have `evidence`, a note saying where to look.
- **`create --done-when "…"`**, repeatable, adds manual criteria at create.
- **`criterion add <id> "text" [--check "cmd"]`** adds a script or manual criterion. **`criterion set <id> <n> --check "cmd"`** gives criterion _n_ a check, replacing what it had. **`criterion remove <id> <n>`** removes one, and the rest move up.
- **`evidence <id> <n> "…"`** records where to look for a manual criterion. A script criterion refuses it with exit 1: its output is its proof.
- **`written`**: the text of each criterion given at create. Afterwards, only `criterion add` from an interactive terminal adds to it, and only `criterion accept` takes a text out. `criterion remove` leaves it alone even from a terminal, since `script` gets past that check (§7). Slice 11's T5 reads it.
- **`criterion accept <id>`**, from an interactive terminal only, records every criterion's text as it is now in `written`, after a human edits one by hand. It says what it records, names each written criterion it drops, asks for the ID through the same confirmation `delete` uses, and adds a history entry with `via: "criterion-accept"`.
- **Gate G1**, on `start`: the item has at least one criterion, no other item in its project is in progress or in review, and no file in `items/` named for the project is one that can't be read.
- **Gate G2**, on `submit`: the item still has a criterion, and every manual criterion has evidence.
- **Gate G6**, on `submit`: if `design` is set, its file exists inside the project's root, after following symlinks, and its section is a numbered heading with something under it.
- **`gates <id>`**, a dry run of the gates `submit` runs, which changes nothing.
- **Every gate failure** lists every reason, changes nothing, and exits 2, with the reasons as `failures` under `--json`. An item file from a newer tracker is refused with exit 3 before any gate runs.
- **`show`** lists the criteria and how each is verified, and `delete`'s warning counts them among what's lost.

**Not built yet:**

- Running checks: G3 and G4 (slice 7). A `check` is stored and shown, but nothing runs it, so a submit that passes G2 and G6 still reaches done `unchecked`.
- G5 and the commit (slice 8), T5's use of `written` (slice 11), and judge criteria with `criterion add --judge` (slice 12). G1's judge rule waits for slice 14.

## Try it

Always set `TRACKER_HOME` while developing. Without it, the tool uses your real store at `~/.tracker`.

```sh
cd guild-portfolio/tracker
cargo build

export DEMO=$(mktemp -d)
export TRACKER_HOME=$DEMO/store
export PATH="$PWD/target/debug:$PATH"
cd $DEMO
mkdir bm
printf '# Bookmarks\n\n## 2. Concepts\n\n### 2.5 Tags\n\nTag chips filter the list.\n\n### 2.6 Sorting\n' > bm/DESIGN.md
tracker init
tracker project add BM --name bookmarks --root bm
cd bm
```

Create an item with two criteria, and start it with no check yet. G1 needs criteria, not checks:

```sh
tracker create "Search filters bookmarks by tag" --design DESIGN.md#2.6 \
  --done-when "Clicking a tag shows only bookmarks with that tag" \
  --done-when "The tag chips read well"
tracker criterion set BM-1 1 --check "cargo test filter_by_tag"
tracker start BM-1
```

A second item in the same project can't start while BM-1 is in progress:

```
$ tracker create "Sort by date" --done-when "Newest first by default"
$ tracker start BM-2
error: BM-2 can't be started: G1 failed. Nothing was changed.
  G1  BM-1 is in progress in BM, and a project works on one item at a time. Submit or shelve BM-1 first.
```

Ask what would block a submit. Criterion 2 is manual with no evidence, and section 2.6 has nothing under it:

```
$ tracker gates BM-1
error: BM-1 would be refused at submit: G2 and G6 failed. This was a dry run, so nothing was changed.
  G2  Criterion 2, "The tag chips read well", is manual and has no evidence. Record where to look with `tracker evidence BM-1 2 "…"`, or give it a check with `tracker criterion set BM-1 2 --check "…"`.
  G6  DESIGN.md has a heading numbered 2.6, but nothing under it, so it doesn't say anything yet. Fix it with `tracker edit BM-1 --design …`.
```

`tracker submit BM-1` refuses with the same reasons and exit 2. Fix both, and it goes through:

```sh
tracker evidence BM-1 2 "the screenshot in docs/tags.png"
tracker evidence BM-1 1 "trust me"     # refused: criterion 1 has a check
tracker edit BM-1 --design DESIGN.md#2.5
tracker gates BM-1
tracker submit BM-1
tracker show BM-1
```

`criterion accept` refuses anything but a terminal:

```
$ tracker criterion accept BM-2 </dev/null
error: `tracker criterion accept BM-2` needs a human at an interactive terminal, who types BM-2 to confirm it, and stdin here isn't a terminal. Nothing was changed. Only a human accepts criteria. An agent never rewords or removes a criterion it was given: it adds one, or says what's wrong in its report.
```

Run it at a terminal after editing a criterion's text in `$TRACKER_HOME/items/BM-2.json`, and it lists each criterion as `already written` or `new to written`, and names each written criterion it drops, before asking for the ID.

## How it works

### Criteria and `written`

A criterion is `{ "text": …, "check": … }` or `{ "text": …, "evidence": … }`, in `criteria`, which sits after `written` and before `verification` in the item file, as §4.3 orders them. Both fields are left out while empty, so items from earlier slices load and come back byte-for-byte unchanged.

Commands number criteria from 1, as `show` lists them. A criterion's text is trimmed, on one line, and at most 500 characters. An item's criteria have different texts: `create` and `criterion add` refuse a repeat. A check is trimmed, on one line and at most 1000 characters, since it's printed before it runs (slice 7) and in gate output.

`written` holds each criterion's text, as it was when written. `create` takes each `--done-when`, whoever runs it: the criteria given at create are the ones I wrote, from the plan. After that:

| Command | From a terminal | Otherwise |
| --- | --- | --- |
| `criterion add` | adds the new text | leaves `written` alone |
| `criterion remove` | leaves `written` alone, so a removed text I wrote goes only through `criterion accept` | the same |
| `criterion set`, `evidence` | leave `written` alone: the text doesn't change | the same |
| `criterion accept` | sets `written` to every criterion's text as it is now | refused |

So an agent can add criteria freely, but a criterion I wrote that it rewords or removes leaves a text in `written` that no criterion has, which slice 11's T5 will hold, naming it. `criterion accept` names each such text before dropping it, so accepting my own hand edit can't also accept an agent's removal unseen.

The criterion commands work in every state but done, whose verification record is about the criteria it was verified against. For an item in review, they work only from a terminal, as `edit` does. They don't add history entries: §2.1 records moves, `edit` and `criterion accept`.

### The gates

`src/gates.rs` runs them and returns a `Report`: the gates that ran, in order, and every `Failure`, each with its gate, the criterion it's about if any, and a message that says what to do. A gate never changes anything.

- **G1** runs inside `start`, and `status <id> in-progress`, after the state check, so a move the diagram doesn't draw is still exit 1. It reads every item under the lock to find another in the same project that's in progress or in review. A file in `items/` that can't be read as an item, but whose name puts it in the project, such as `XX-1.json`, fails G1 too, since it may be the item in progress. G1 uses what the listing found and never opens a pipe or a folder to find out more.
- **G2** fails an item with no criteria, since one can be removed after `start`, and otherwise each manual criterion with no evidence, once per criterion.
- **G6** reads `design` relative to the project's root, follows every symlink with `canonicalize`, and fails a file that ends up outside the root, whether through a symlinked file, a symlinked folder, `..` or an absolute path written by hand. It reads only a regular file, so a pipe can't make it wait forever. A section matches an ATX heading (`#` to `######`, with at most three spaces of indent) whose first word is that number, with a trailing dot ignored on either side, so `## 1. Overview` is section `1`. Code is never a heading, following CommonMark: a fence of three or more backticks or tildes opens a block, which only a fence of the same character, at least as long and with nothing after it, closes, and a line with four spaces of indent or a tab is indented code. "Content under it" is any line, other than a blank line, a heading or a code fence, before the next heading of the same level or higher. So a section whose subsections have text passes, and so does one with a line of code, but a heading followed only by another heading, or by an empty code block, doesn't.

`submit`, and `status <id> done`, run G2 then G6, cheapest first, before anything is written. `gates` runs the same two without the lock and never writes. Before any gate, `gates` and every move refuse, with exit 3, an item file holding fields this version doesn't know, as `delete` does: a newer tracker's judge criterion would otherwise read as a manual one, and G2 would ask for evidence that `evidence` refuses.

### The refusal

A gate refusal is an `Error` with exit 2. `Error` now carries `fields`, extra JSON fields in order, which replaces slice 2's single `allowed`, and `details`, lines the text prints under the message, indented:

```
error: BM-1 can't be submitted: G2 and G6 failed. Nothing was changed.
  G2  Criterion 2, "The tag chips read well", is manual and has no evidence. …
  G6  DESIGN.md has a heading numbered 2.6, but nothing under it, …
```

The first line names the gates that failed, and each line under it is one reason. A `gates` dry run on an item that isn't in progress adds one more line, "It's open, and only an in-progress item can be submitted.", as a passing one does, and `status` says the same in `--json`, since fixing every reason still wouldn't let it be submitted.

## JSON output

| Command | Schema | Shape |
| --- | --- | --- |
| `criterion add`, `set`, `remove`, `evidence` | [criterion.schema.json](../schemas/criterion.schema.json) | `{ "criterion": n, "item": … }` |
| `criterion accept` | [criterion-accept.schema.json](../schemas/criterion-accept.schema.json) | `{ "accepted": true \| false, "item": … }` |
| `gates`, when every gate passes | [gates.schema.json](../schemas/gates.schema.json) | `{ "id", "status", "result": "passed", "gates": [{ "gate", "ok" }] }` |
| `start`, `submit` or `gates`, refused | [error.schema.json](../schemas/error.schema.json) | the error shape, exit 2, plus `id`, `status`, `result: "refused"`, `gates` and `failures: [{ "gate", "criterion"?, "message" }]` |
| `submit` | [submit.schema.json](../schemas/submit.schema.json) | as before, with `gates` now listing G2 and G6 |

[item.schema.json](../schemas/item.schema.json) gains `written` (one-line strings), `criteria` (a criterion with both `check` and `evidence` is refused), and the `criterion-accept` history entry, whose `fields` is exactly `["written"]`. An `edit` can't name `written`. The error schema requires `id`, `status`, `gates` and `failures` together with `result`, and exit code 2 with them.

## Code tour

- `src/item.rs`: `Criterion`, the `written` and `criteria` fields, and `Via::CriterionAccept`.
- `src/gates.rs`: new. `Report`, `Failure`, `start` (G1), `submit` (G2, G6), `check_section` with `lines`, which says what each line of a design doc is, and four unit tests for section matching.
- `src/main.rs`: the `criterion`, `evidence` and `gates` commands, `create --done-when`, the gates in `apply`, `refuse_newer_file`, `show`'s "Done when" section, and `delete`'s warning.
- `src/error.rs`: `fields`, `details` and `Error::gate`.
- `src/tty.rs`: `require_interactive`, split out of `confirm`, so `criterion accept` refuses an agent even when there's nothing to accept.
- `src/store.rs`: `Listed::Item` is boxed, since the item grew and clippy flags the size gap.

## Testing

`Env::create`, the shared helper, now gives each item a script criterion whose check is `true`, so G1 lets it start and G2 needs no evidence, as PLAN.md's prompt asks. `Env::create_bare` makes an item with none, and `Env::add_criterion` adds the same criterion to an item made another way. Tests that started two items in one project start them one after the other, since G1 refuses the second. Tests that used `written`, `criteria` or `criterion-accept` as examples of a newer tracker's fields now use `owner` and a made-up `archive` command, which no slice will add.

### Coverage of the _Done when_ checks

| Check | Test |
| --- | --- |
| `start` with no criteria is refused by G1 with exit 2 | `tests/gates.rs`: `start_with_no_criteria_is_refused_by_g1_with_exit_2` |
| …and so is `start` while another item in the project is in progress | `start_while_another_item_in_the_project_is_in_progress_is_refused_by_g1`, and `…_in_review_…` |
| A manual criterion with no evidence fails G2 | `a_manual_criterion_with_no_evidence_fails_g2` |
| A `design` naming a missing section fails G6 | `a_design_naming_a_missing_section_fails_g6` |
| …and a numbered section with content under it passes | `a_numbered_section_with_content_under_it_passes_g6` |
| A `design` whose file is a symlink to one outside the root fails G6 | `a_design_whose_file_is_a_symlink_outside_the_root_fails_g6` |
| `gates` lists every failure and leaves the item file byte-for-byte unchanged | `gates_lists_every_failure_and_leaves_the_item_file_unchanged`, and every test through `refused_by_gate` |
| A refused `submit` exits 2 and changes nothing | `a_refused_submit_exits_2_and_changes_nothing` |
| `create --done-when a --done-when b` adds two criteria, and their two texts to `written` | `tests/criteria.rs`: `create_done_when_adds_manual_criteria_and_their_written_text`. Until review 1, `written` held each text's SHA-256, and PLAN.md said "fingerprints" |
| `evidence` on a script criterion exits 1 | `evidence_is_recorded_for_a_manual_criterion_only` |
| `criterion accept` is refused outside an interactive terminal | `criterion_accept_is_refused_outside_an_interactive_terminal`, with the ID piped in |

Typing the ID into `criterion accept` is tested automatically under `script`, as `delete` is: `in_a_terminal_criterion_accept_records_the_criteria_as_written`, a wrong ID, and an item changed while deciding. `tests/fixtures/slice-6/` holds three item files this slice's binary wrote, one with a `criterion-accept` entry, and `tests/fixtures.rs` checks them with the rest.

## Decisions worth knowing

- **`written` holds each criterion's text, not its SHA-256.** It held the hash until review 1's finding 4: a hash can't be turned back into its text, so `criterion accept` could only count what it dropped. The file already holds the criteria in plain text, so a hash hid nothing. Nothing uses `sha2` now, so slice 11 adds it for T5's criteria hash.
- **`written` has no order.** An agent's criteria aren't in it, so the list can't line up with `criteria` by position. Slice 11's T5 asks whether each text in it is still some criterion's text, and `criterion accept` treats the same texts in another order as nothing to accept. DESIGN.md §2.1 says so.
- **Criteria have different texts.** A repeat is refused, so each text in `written` names one criterion.
- **`criterion set --check` drops a manual criterion's evidence**, and says so: §2.3 gives evidence to manual criteria only. The schema refuses a criterion with both.
- **`gates` exits 2 when a gate fails**, as the prompt asks, and prints the same error shape as a refused `submit`, with `result: "refused"`, so an agent reads one shape. DESIGN.md §3 now says so.
- **A submit that passes is still `unchecked`.** `auto` means the work was proven, and nothing has run a check yet. The note reads "Submitted before tracker ran checks. Gates G2 and G6 passed, but nothing tested the work.", in the past tense so it stays true after slice 7 runs checks, and `submit` prints it after "Unchecked.". Until review 2 it read "No checks ran: this version of tracker runs only gates G2 and G6…", which the fixture XX-15 keeps. Records written before this slice keep "No gates ran".
- **G1 needs a criterion, not a check.** §2.4 asks only for criteria, and a manual criterion is enough to start. T6 will hold such an item at submit (slice 9). G2 checks for a criterion again at submit.
- **A check is at most 1000 characters**, twice a criterion's text, since a command is often longer than the sentence it proves. A longer one goes in a script the check runs.
- **The criterion commands refuse a done item**, as `edit` does, and `criterion accept` too. Reopen it first.
- **`criterion accept` with nothing to accept still refuses an agent**, and from a terminal it says so without asking for the ID.

## Fixes after review 1

[Review 1](reviews/slice-6-review-1.md) found two bugs, two security problems, five places where the code and DESIGN.md disagreed or the design was unclear, and six test gaps. For each finding with a choice, the review's option (a) was taken, and finding 8's limit is 1000 characters rather than the review's 500. Each fix is its own commit, with its tests, and each test that came with a fix was seen failing first:

| Finding | Commit | What changed | Tests |
| --- | --- | --- | --- |
| Bug 1: a newer tracker's judge criterion read as manual, so `gates` and `submit` gave G2's advice | `9046539` | `refuse_newer_file` refuses, with exit 3, a file with fields this version doesn't know, in `gates` and in `apply` before any gate, so `start`, `submit`, `shelve`, `reopen` and `status` all get it | `a_newer_trackers_file_is_refused_with_exit_3_before_any_gate_runs` in `tests/gates.rs`: a criterion with `ask` through `gates`, `start`, `submit` and `status`, each exit 3 with the file unchanged. It failed on `gates`'s exit 2 |
| Bug 2: G6 took a heading in a code block, an indented heading, and an empty code block's fences as content | `a16fd2b` | `lines` follows CommonMark's fences and indents: a fence closes only on the same character, at least as long, with nothing after it; a heading or fence has at most three spaces of indent; a fence line is never content | Two unit tests in `src/gates.rs`: the review's three cases and three more that must fail, and five that must pass, such as a longer closing fence. The failing set failed on the review's first case |
| Security 3: a line break written by hand into a criterion or its evidence printed as a line of its own | `3ab8de7` | `criterion remove`, `criterion set` and `criterion accept`'s warning escape each text on its own first, as `show` does. `criterion accept` wasn't in the review, but had the same problem | `text_output_escapes_a_hand_edited_criterion_and_its_evidence` in `tests/criteria.rs`, seen failing at each of the three in turn |
| Security 4: `criterion accept` couldn't say which written criteria it dropped | `05f4bba` | `written` holds each criterion's text instead of its SHA-256, and `criterion accept` lists each text it drops. DESIGN.md §2.1, §2.3, §2.5, §2.7, §4.3 and §6, PLAN.md's slice 6 and 11 prompts, `item.schema.json` and the slice 6 fixtures follow, and `sha2` is removed | `criterion_accept_names_each_written_criterion_it_drops`: the review's case, an agent's removal and my rewording, both named. It failed with the old "2 written criteria match none of them" |
| Finding 5: G1 ignored an item file it couldn't read | `c1e91fd` | G1 fails on a skipped file whose name puts it in the item's project, naming the file, why it can't be read and the fix. DESIGN.md §2.4 and §4.4 say so | `g1_fails_while_an_item_file_in_the_project_cant_be_read` in `tests/gates.rs`, which failed on `start`'s exit 0, and two slice 5 tests in `tests/bad_data.rs`, which now `edit` the other items to show they still work and check that `start` names each unreadable file, a pipe included, without hanging |
| Finding 6: nothing at submit required a criterion | `c08cdba` | G2 fails an item with no criteria, with G1's reason. DESIGN.md §2.4's G2 row says so | `an_item_with_no_criteria_fails_g2`, through `gates`, `submit` and `status`. It failed on `gates`'s exit 0 |
| Finding 7: §3 said the criterion commands work at any time | `b51549b` | §3 says they refuse a done item, and an item in review unless from an interactive terminal, as `edit` does | `tests/docs_links.rs` still passes |
| Finding 8: a check command had no length limit | `ab324b4` | `criterion add` and `criterion set` refuse a check longer than 1000 characters, saying to put the steps in a script. DESIGN.md §2.3 and `--check`'s help say so | `criterion_add_refuses_bad_input_and_a_repeated_criterion`: 1001 characters refused by both, 1000 taken. It failed on the 1001 being taken |
| Finding 9: §2.4 didn't say what "content under it" means | `22fc352` | G6's row says what counts as the heading and as content, following bug 2's fix | `tests/docs_links.rs` still passes |
| Test gaps 1, 2, 3 and 6 | `9046539`, `a16fd2b`, `3ab8de7`, `c1e91fd` | With bug 1, bug 2, security 3 and finding 5 | |
| Test gap 4: nothing checked that `criterion set` and `evidence` leave `written` alone | `e137f93` | `criterion_set_and_evidence_leave_written_alone_from_a_terminal_or_not`, with and without a terminal, including a check given to an agent's criterion from one. It caught `criterion set` changed to add its text from a terminal | |
| Test gap 5: nothing checked that `criterion accept` refuses a done item | `0b6aa89` | `criterion_accept_refuses_a_done_item`, at a terminal. Its message checks caught the refusal taken out: the exit code alone didn't, since the unanswered question also exits 1 | |

The review's process notes weren't part of this round: the readme still has no "Still open" section carrying slices 2–5's open items forward, and doesn't say that colour for the new commands waits for slice 16.

## Fixes after review 2

[Review 2](reviews/slice-6-review-2.md) found one bug, two security problems, three places where the code and DESIGN.md disagreed or the design was unclear, and two test gaps. This round fixed the bug, both security problems, findings 4 and 5 and both test gaps. Security 2 took option (a), and security 3 option (b), as the review suggested. Finding 6, G6 and HTML comments, wasn't part of this round.

| Finding | Commit | What changed | Tests |
| --- | --- | --- | --- |
| Bug 1: `criterion accept` asked for the ID before it refused a newer tracker's file | `34b60ce` | It calls `refuse_newer_file` right after the load, before the done check and the question, as `delete` does | `criterion_accept_refuses_a_newer_trackers_file_before_it_asks` in `tests/criteria.rs`, at a terminal. It failed on exit 1, after the question |
| Security 2: from a terminal, `criterion remove` took a written text out of `written`, unasked and unrecorded | `95ee6e4` | `criterion remove` leaves `written` alone from a terminal too, so a text I wrote leaves it only through `criterion accept`, which names it, asks for the ID and leaves a history entry. DESIGN.md §2.1 and §3 and the table above say so | `from_a_terminal_add_changes_written_but_remove_doesnt`, which goes on to accept the removal. It failed on the empty `written` |
| Security 3: `written` keeps only the text, so before start a check I gave can be replaced unrecorded | `28f49d1` | Option (b), docs only: DESIGN.md §2.1, §2.5's T5, §3, §6 and §7 say that `written` protects a criterion's text, not its check, and that only T13 can catch a replaced check | |
| Finding 4: the `unchecked` note was in the present tense | `2a39e8e` | The note reads "Submitted before tracker ran checks. Gates G2 and G6 passed, but nothing tested the work.", and `submit` no longer prints "Gates G2 and G6 passed." on a line of its own as well. DESIGN.md §2.1's `verification` row, §4.3's example and §9 follow | `tests/states.rs` and `tests/show_and_delete.rs` check the new wording |
| Finding 5: a failing `gates` didn't say the item couldn't be submitted anyway | `c86e653` | Every gate refusal has `status`, which `error.schema.json` requires with `result`. A failing dry run on an item that isn't in progress prints, under the reasons, the line a passing one prints. DESIGN.md §3 says so | `a_failing_gates_says_when_the_item_isnt_ready_to_submit_either`, and `refused_by_gate` checks `status` on every refusal. 15 tests failed before the fix |
| Test gap 1: nothing checked that two starts at once let only one through | `5d2b36d` | `ten_starts_at_once_in_one_project_let_exactly_one_through` in `tests/gates.rs`, three rounds of ten | It covers code that already worked, so it was run with the lock taken out of `move_item`: it failed in the first round, with two items in progress |
| Test gap 2: nothing checked that `criterion accept` refuses a newer tracker's file before it asks | `34b60ce` | With bug 1 | |

## Limits, and what later slices change

- Until slice 9, an item whose criteria are all manual, with evidence, reaches done with no human, as PLAN.md's integration risks warn.
- A corrupt file in a project blocks every start in that project until it's fixed or moved out of `items/`, since G1 can't tell whether it's the item in progress. `list` reports it, and so does G1.
- G6 matches ATX headings only. A setext heading (a line of `===` under the title) isn't one, and neither is a heading inside a block quote (`> ## 2.4 Gates`), so G6 fails such a section rather than pass it.
- Nothing runs a `check` yet (slice 7). `criterion add --judge` and judge criteria arrive in slice 12, and the item schema's criterion grows their fields then.

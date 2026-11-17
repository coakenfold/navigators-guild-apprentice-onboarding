# Slice 6: Criteria and simple gates

`tracker` is a command-line work tracker, written in Rust, that keeps one central list of work items across many projects and decides when an item is really done. This document covers **slice 6** of the 16 in [PLAN.md](PLAN.md): what it built, how it works, and how to test it. The earlier readmes cover the rest: [slice 1](slice-1-readme.md) the store, `create` and `list`, [slice 2](slice-2-readme.md) the states and `history`, [slice 3](slice-3-readme.md) priority, kind, labels, `edit` and `list`'s filters, [slice 4](slice-4-readme.md) `show` and `delete`, and [slice 5](slice-5-readme.md) bad data and polish.

After slice 6 you can **write Done-when criteria, and get refused when they're missing or incomplete**. The spec is [DESIGN.md](DESIGN.md). This slice implements [§2.3](DESIGN.md#23-done-when-criteria)'s script and manual criteria, the `written` fingerprints from [§2.1](DESIGN.md#21-items), `criterion accept` from [§2.7](DESIGN.md#27-human-actions), and gates G1, G2 and G6 from [§2.4](DESIGN.md#24-gates-is-it-finished), with the `gates` dry run from [§3](DESIGN.md#3-interface). It's the last slice before `tracker` starts tracking its own build (PLAN.md's checkpoint).

| | |
| --- | --- |
| Status | Every _Done when_ check has a test, and every test passes, as does the test command. PLAN.md has slice 6 as `in review`. No review has run yet, and the branch isn't merged into `main`. |
| Branch | `feat/tracker/slice-6`, from `main` at `440628f` (slice 5 merged): `be91d1f` (sha2), `4d2d6ad` (the slice's code, schemas, tests and fixtures, in one commit, since the gates change what most existing tests set up), `6eb689d` (DESIGN.md §2.1, §2.3, §3 and §4.3), then this readme and PLAN.md's status |
| Test command | `cargo fmt --check && cargo clippy -- -D warnings && cargo test`, run in `tracker/` |
| Tests | 237: 39 in `tests/create_and_list.rs`, 34 in `tests/organise.rs`, 27 in `tests/states.rs`, 24 in `tests/show_and_delete.rs`, 19 in `tests/criteria.rs`, 17 in `tests/gates.rs`, 14 in `tests/item_files.rs`, 13 in `tests/bad_data.rs`, 11 in `tests/docs_links.rs`, 9 in `tests/polish.rs`, 8 in `tests/fixtures.rs`, 6 in `tests/output.rs`, 5 in `tests/schemas.rs`, 4 in `tests/store_writes.rs`, 2 in `tests/item_ids.rs`, 3 unit tests in the library (2 in `src/gates.rs`, 1 in `src/store.rs`) and 2 in `src/main.rs` |

## Contents

- [What's in this slice](#whats-in-this-slice)
- [Try it](#try-it)
- [How it works](#how-it-works)
- [JSON output](#json-output)
- [Code tour](#code-tour)
- [Testing](#testing)
- [Decisions worth knowing](#decisions-worth-knowing)
- [Limits, and what later slices change](#limits-and-what-later-slices-change)

## What's in this slice

**Built:**

- **Criteria.** An item has a **Done when** list, `criteria`. Each criterion has `text`, and either a `check` (a script criterion) or none (a manual one). A manual criterion can have `evidence`, a note saying where to look.
- **`create --done-when "…"`**, repeatable, adds manual criteria at create.
- **`criterion add <id> "text" [--check "cmd"]`** adds a script or manual criterion. **`criterion set <id> <n> --check "cmd"`** gives criterion _n_ a check, replacing what it had. **`criterion remove <id> <n>`** removes one, and the rest move up.
- **`evidence <id> <n> "…"`** records where to look for a manual criterion. A script criterion refuses it with exit 1: its output is its proof.
- **`written`**: one fingerprint, the SHA-256 of the text, per criterion given at create. Only a `criterion` command from an interactive terminal changes it afterwards: `criterion add` adds the new text's, and `criterion remove` takes the removed one's out. Slice 11's T5 reads it.
- **`criterion accept <id>`**, from an interactive terminal only, records every criterion's text as it is now in `written`, after a human edits one by hand. It says what it records, asks for the ID through the same confirmation `delete` uses, and adds a history entry with `via: "criterion-accept"`.
- **Gate G1**, on `start`: the item has at least one criterion, and no other item in its project is in progress or in review.
- **Gate G2**, on `submit`: every manual criterion has evidence.
- **Gate G6**, on `submit`: if `design` is set, its file exists inside the project's root, after following symlinks, and its section is a numbered heading with something under it.
- **`gates <id>`**, a dry run of the gates `submit` runs, which changes nothing.
- **Every gate failure** lists every reason, changes nothing, and exits 2, with the reasons as `failures` under `--json`.
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

Run it at a terminal after editing a criterion's text in `$TRACKER_HOME/items/BM-2.json`, and it lists each criterion as `already written` or `new to written` before asking for the ID.

## How it works

### Criteria and `written`

A criterion is `{ "text": …, "check": … }` or `{ "text": …, "evidence": … }`, in `criteria`, which sits after `written` and before `verification` in the item file, as §4.3 orders them. Both fields are left out while empty, so items from earlier slices load and come back byte-for-byte unchanged.

Commands number criteria from 1, as `show` lists them. A criterion's text is trimmed, on one line, and at most 500 characters. An item's criteria have different texts: `create` and `criterion add` refuse a repeat. A check is trimmed and on one line, since it's printed before it runs (slice 7) and in gate output.

`written` holds the SHA-256 of each criterion's text, in lower-case hex. `create` takes one per `--done-when`, whoever runs it: the criteria given at create are the ones I wrote, from the plan. After that:

| Command | From a terminal | Otherwise |
| --- | --- | --- |
| `criterion add` | adds the new text's fingerprint | leaves `written` alone |
| `criterion remove` | takes the removed text's fingerprint out | leaves `written` alone |
| `criterion set`, `evidence` | leave `written` alone: the text doesn't change | the same |
| `criterion accept` | sets `written` to every criterion's text as it is now | refused |

So an agent can add criteria freely, but a criterion I wrote that it rewords or removes leaves a fingerprint that matches nothing, which slice 11's T5 will hold.

The criterion commands work in every state but done, whose verification record is about the criteria it was verified against. For an item in review, they work only from a terminal, as `edit` does. They don't add history entries: §2.1 records moves, `edit` and `criterion accept`.

### The gates

`src/gates.rs` runs them and returns a `Report`: the gates that ran, in order, and every `Failure`, each with its gate, the criterion it's about if any, and a message that says what to do. A gate never changes anything.

- **G1** runs inside `start`, and `status <id> in-progress`, after the state check, so a move the diagram doesn't draw is still exit 1. It reads every item under the lock to find another in the same project that's in progress or in review. An item file that can't be read is left out, as `list` leaves it out.
- **G2** fails each manual criterion with no evidence, once per criterion.
- **G6** reads `design` relative to the project's root, follows every symlink with `canonicalize`, and fails a file that ends up outside the root, whether through a symlinked file, a symlinked folder, `..` or an absolute path written by hand. It reads only a regular file, so a pipe can't make it wait forever. A section matches an ATX heading (`#` to `######`) whose first word is that number, with a trailing dot ignored on either side, so `## 1. Overview` is section `1`. Lines in fenced code blocks are never headings. "Content under it" is any non-blank line, other than a heading, before the next heading of the same level or higher, so a section whose subsections have text passes, and a heading followed only by another heading doesn't.

`submit`, and `status <id> done`, run G2 then G6, cheapest first, before anything is written. `gates` runs the same two without the lock and never writes.

### The refusal

A gate refusal is an `Error` with exit 2. `Error` now carries `fields`, extra JSON fields in order, which replaces slice 2's single `allowed`, and `details`, lines the text prints under the message, indented:

```
error: BM-1 can't be submitted: G2 and G6 failed. Nothing was changed.
  G2  Criterion 2, "The tag chips read well", is manual and has no evidence. …
  G6  DESIGN.md has a heading numbered 2.6, but nothing under it, …
```

The first line names the gates that failed, and each line under it is one reason.

## JSON output

| Command | Schema | Shape |
| --- | --- | --- |
| `criterion add`, `set`, `remove`, `evidence` | [criterion.schema.json](../schemas/criterion.schema.json) | `{ "criterion": n, "item": … }` |
| `criterion accept` | [criterion-accept.schema.json](../schemas/criterion-accept.schema.json) | `{ "accepted": true \| false, "item": … }` |
| `gates`, when every gate passes | [gates.schema.json](../schemas/gates.schema.json) | `{ "id", "status", "result": "passed", "gates": [{ "gate", "ok" }] }` |
| `start`, `submit` or `gates`, refused | [error.schema.json](../schemas/error.schema.json) | the error shape, exit 2, plus `id`, `result: "refused"`, `gates` and `failures: [{ "gate", "criterion"?, "message" }]` |
| `submit` | [submit.schema.json](../schemas/submit.schema.json) | as before, with `gates` now listing G2 and G6 |

[item.schema.json](../schemas/item.schema.json) gains `written` (64-character hex strings), `criteria` (a criterion with both `check` and `evidence` is refused), and the `criterion-accept` history entry, whose `fields` is exactly `["written"]`. An `edit` can't name `written`. The error schema requires `id`, `gates` and `failures` together with `result`, and exit code 2 with them.

## Code tour

- `src/item.rs`: `Criterion`, the `written` and `criteria` fields, `fingerprint`, and `Via::CriterionAccept`.
- `src/gates.rs`: new. `Report`, `Failure`, `start` (G1), `submit` (G2, G6), `check_section`, and two unit tests for section matching.
- `src/main.rs`: the `criterion`, `evidence` and `gates` commands, `create --done-when`, the gates in `apply`, `show`'s "Done when" section, and `delete`'s warning.
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
| `create --done-when a --done-when b` adds two criteria and two `written` fingerprints | `tests/criteria.rs`: `create_done_when_adds_manual_criteria_and_their_written_fingerprints`, against SHA-256 values worked out with `shasum` |
| `evidence` on a script criterion exits 1 | `evidence_is_recorded_for_a_manual_criterion_only` |
| `criterion accept` is refused outside an interactive terminal | `criterion_accept_is_refused_outside_an_interactive_terminal`, with the ID piped in |

Typing the ID into `criterion accept` is tested automatically under `script`, as `delete` is: `in_a_terminal_criterion_accept_records_the_criteria_as_written`, a wrong ID, and an item changed while deciding. `tests/fixtures/slice-6/` holds three item files this slice's binary wrote, one with a `criterion-accept` entry, and `tests/fixtures.rs` checks them with the rest.

## Decisions worth knowing

- **Each `written` fingerprint is a full SHA-256.** §4.3's example shortens it. Sixty-four hex characters are long, but there's no cut-off to choose or explain.
- **`written` has no order.** An agent's criteria have no fingerprint, so the list can't line up with `criteria` by position. Slice 11's T5 asks whether each fingerprint still matches some criterion's text, and `criterion accept` treats the same fingerprints in another order as nothing to accept. DESIGN.md §2.1 now says so.
- **Criteria have different texts.** A repeat is refused, so each fingerprint names one criterion.
- **`criterion set --check` drops a manual criterion's evidence**, and says so: §2.3 gives evidence to manual criteria only. The schema refuses a criterion with both.
- **`gates` exits 2 when a gate fails**, as the prompt asks, and prints the same error shape as a refused `submit`, with `result: "refused"`, so an agent reads one shape. DESIGN.md §3 now says so.
- **A submit that passes is still `unchecked`.** `auto` means the work was proven, and nothing has run a check yet. The note now reads "No checks ran: this version of tracker runs only gates G2 and G6, which don't test the work, so nothing checked it." Records written earlier keep "No gates ran".
- **G1 needs a criterion, not a check.** §2.4 asks only for criteria, and a manual criterion is enough to start. T6 will hold such an item at submit (slice 9).
- **The criterion commands refuse a done item**, as `edit` does, and `criterion accept` too. Reopen it first.
- **`criterion accept` with nothing to accept still refuses an agent**, and from a terminal it says so without asking for the ID.

## Limits, and what later slices change

- Until slice 9, an item whose criteria are all manual, with evidence, reaches done with no human, as PLAN.md's integration risks warn.
- G1 can't see an item file it can't read, so a corrupt file for an in-progress item doesn't block a start. `list` reports such a file.
- G6 matches ATX headings only. A setext heading (a line of `===` under the title) isn't one.
- Nothing runs a `check` yet (slice 7). `criterion add --judge` and judge criteria arrive in slice 12, and the item schema's criterion grows their fields then.

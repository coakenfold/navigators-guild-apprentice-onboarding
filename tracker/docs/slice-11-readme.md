# Slice 11: Tampered proof is held

`tracker` is a command-line work tracker, written in Rust, that keeps one central list of work items across many projects and decides when an item is really done. This document covers **slice 11** of the 16 in [PLAN.md](PLAN.md): what it built, how it works, and how to test it. The earlier readmes cover the rest: [slice 1](slice-1-readme.md) the store, `create` and `list`, [slice 2](slice-2-readme.md) the states and `history`, [slice 3](slice-3-readme.md) priority, kind, labels, `edit` and `list`'s filters, [slice 4](slice-4-readme.md) `show` and `delete`, [slice 5](slice-5-readme.md) bad data and polish, [slice 6](slice-6-readme.md) criteria and gates G1, G2 and G6, [slice 7](slice-7-readme.md) running checks, with gates G3 and G4 and `started.checks`, [slice 8](slice-8-readme.md) G5, the `auto` record and `refs/tracker/<ID>`, [slice 9](slice-9-readme.md) the inbox, `approve`, `reject` and triggers T6 and T9, and [slice 10](slice-10-readme.md) triggers T1, T2, T3, T4, T7 and T8.

After slice 11 you can **have moved goalposts, checks that pass before the work, and unscoped commits held**. The spec is [DESIGN.md](DESIGN.md). This slice implements triggers T5, T13 and T14 from [§2.5](DESIGN.md#25-review-triggers-does-it-need-a-human), "Checks must fail first" from [§2.4](DESIGN.md#24-gates-is-it-finished) (which T13 reads), `written` and the commit definitions from [§2.1](DESIGN.md#21-items), and `started.criteria_hash` from [§4.3](DESIGN.md#43-item-file). It's tracked in `tracker` itself as item TR-7, with the binary installed from slice 10 ([Tracking TR-7](#tracking-tr-7)). There are no reviews of this slice yet.

| | |
| --- | --- |
| Status | Every _Done when_ check has a test, and every test passes, as does the test command. TR-7 is `done`, but it wasn't auto-verified. The installed slice 10 binary held its submit under T1, T3, T4, T7 and T8, and the user read the diff and approved it from a real terminal: `verification.by` is `human`, at `ce58426`, with `refs/tracker/TR-7`, and the five holds are recorded in `verification.triggers` as approved. All five of its checks failed at `start`, as they should. The installed binary has no T5, T13 or T14, so it couldn't check this slice's own proof, and the orchestrator checked by hand, before the approve, that every commit since `started.head` is scoped to TR-7, that the criteria's texts still match `written`, and that all five checks failed at start ([Checks done by hand](#checks-done-by-hand)). TR-7 is the first TR item to be held and approved, so the approve is also the first use in a real terminal of slice 10's `approve` question, the one that names `HEAD`. That's checked by hand, though this readme has no copy of what the question said. Test commit `e7406d8` changed existing tests before `start`, so T4 is among the holds ([Older tests that changed](#older-tests-that-changed)). Not covered by a test: a check given after `start` firing T5, a design change firing T5, `criterion accept` leaving the fingerprint alone, rewritten commits and a repository with no commits counting for T14, and an item started before slice 11 comparing only `written` ([Still open](#still-open)). `submit`'s help text still names only T1 to T4 and T6 to T9. There are no reviews, so nothing from a review is open. PLAN.md has slice 11 as `in review`. The branch isn't merged into `main`, and neither are slices 7 to 10, which it's stacked on. |
| Branch | `feat/tracker/slice-11`, stacked on `feat/tracker/slice-10` at `7a31712`. Slice 10 isn't merged into `main`, so this slice's commits are `git log --oneline --reverse feat/tracker/slice-10..feat/tracker/slice-11`. In order: `e7406d8` (TR-7's tests and the older tests it changed, committed before `tracker start TR-7`), `46536ec` (PLAN.md: slice 11 in progress), `03408b3` (the triggers, the fingerprint and `unscoped`), `42213dc` (DESIGN.md), `29ad53b` (fixture XX-23 and its test) and `ce58426` (PLAN.md: slice 11 in review). That is 6 commits since slice 10, all scoped to TR-7. This readme comes after them |
| Test command | `cargo fmt --check && cargo clippy -- -D warnings && cargo test`, run in `tracker/`. All three pass. |
| Tests | 403: 46 in `tests/risky_changes.rs`, 39 in `tests/create_and_list.rs`, 36 in `tests/commits.rs`, 34 in `tests/organise.rs`, 28 in `tests/review.rs`, 27 in `tests/states.rs`, 25 in `tests/show_and_delete.rs`, 23 in `tests/criteria.rs`, 22 in `tests/gates.rs`, 18 in `tests/checks.rs`, 17 in `tests/tampered_proof.rs`, 14 in `tests/item_files.rs`, 14 in `tests/fixtures.rs`, 13 in `tests/bad_data.rs`, 11 in `tests/docs_links.rs`, 9 in `tests/polish.rs`, 6 in `tests/output.rs`, 5 in `tests/schemas.rs`, 4 in `tests/store_writes.rs`, 2 in `tests/item_ids.rs`, 8 unit tests in the library (4 in `src/gates.rs`, 2 in `src/git.rs`, 1 each in `src/store.rs` and `src/checks.rs`) and 2 in `src/main.rs` |

## Contents

- [What's in this slice](#whats-in-this-slice)
- [Try it](#try-it)
- [Commands](#commands)
- [How it works](#how-it-works)
- [JSON output](#json-output)
- [Code tour](#code-tour)
- [Testing](#testing)
- [Decisions worth knowing](#decisions-worth-knowing)
- [Still open](#still-open)
- [Limits, and what later slices change](#limits-and-what-later-slices-change)

## What's in this slice

**Built:**

- **T5, goalposts moved.** It fires for either of two reasons, and its message says which:
  - The criteria, kind or design no longer match `started.criteria_hash`, the fingerprint `start` took: "the criteria, their checks, the kind or the design changed after start". So `edit --kind` after `start` fires it, and so does a changed check command or a changed `design`.
  - A text in `written` is no longer any criterion's text, at any time, including before `start`: "a written criterion is no longer any criterion's text: "A bad line is skipped"". Several are joined with "and" and named. Adding a criterion or giving one a check before `start` fires neither.
- **T13, passed before the work.** It fires when a criterion's entry in `started.checks` says it passed: "criterion 1's check already passed before the work, so it can't be proving it". It names every such criterion. A criterion with no entry, such as one given a check while the item was in review (§2.4), counts as a check that didn't pass.
- **T14, unscoped work.** It fires for each commit between `started.head` and `HEAD` that touches the project's root and isn't scoped to any item. The message names each one by short SHA and header, oldest first: `an unscoped commit since start, which no item's scope covers: 24eddbc "tidy the notes"`. It reads the same range as T4, so a rewritten commit counts.
- **`started.criteria_hash`**, written by `start` before `head`: a SHA-256, in 64 lower-case hex characters, of each criterion with its evidence removed, then the kind and the design (`triggers::fingerprint`). `show` prints it as `hash`. It's in `schemas/item.schema.json`, and an item written by an earlier slice, with no hash, still loads and comes back byte for byte.
- **Eleven triggers run.** `triggers::run` now runs T1 to T9, T13 and T14, in that order, so `triggers` in `submit --json` and `gates --json` lists eleven, and six older tests that pinned the list of eight changed ([Older tests that changed](#older-tests-that-changed)).
- **A new test helper check.** `Env::add_criterion` gives a criterion a check that fails when run by `start` and passes elsewhere, in place of `true`, so T13 doesn't hold every test's item ([Testing](#testing)).
- **Slice 10's "Slice 11 must decide" points**, from [review 1](reviews/slice-10-review-1.md#slice-11-must-decide) and [review 2](reviews/slice-10-review-2.md#slice-11-must-decide): T14 shares `since_range` with T4, the unscoped delete that the item re-adds weakened is held by T14 and has a test, `edit --kind` after `start` fires T5, and the helper's `--check true` is gone. A `started.head` that git no longer has isn't decided ([Still open](#still-open)).

**Not built yet:**

- **T10, T11 and T12** (slices 12, 14 and 15), the judge triggers. T13 doesn't cover a criterion judge's answers yet (slice 14), and the fingerprint covers a judge's fields once slice 12 adds them.
- **The digest and `audit`** (slice 16), which list every `criterion accept`, `edit` and `approve`, and are the only place a hand edit of the store shows.
- **A T5 for a check given before `start`.** DESIGN.md §7 says an agent can replace a check the user gave a criterion before `start`, and only T13 sees it. The fingerprint is taken at `start`, so nothing changes that here.
- **`submit`'s help text** for the three new triggers. It names T1 to T4 and T6 to T9 ([Still open](#still-open)).

## Try it

Always set `TRACKER_HOME` while developing. Without it, the tool uses your real store at `~/.tracker`.

The three projects, BM, IM and LM, are the folders `bm`, `im` and `lm` inside one throwaway repository, as TR is a folder inside guild-portfolio. The demo's commits use a made-up name, so they work whatever your own git settings say. BM-1 has a check that already passes, has its kind edited after `start`, and has an unscoped commit made while it's in progress, so T5, T13 and T14 all hold it. IM-1 has a criterion removed before `start`, which fires only T5. LM-1 does nothing wrong, and goes to done. No new refusal is added in this slice, since a trigger holds rather than refuses, so the one refusal is `approve` with piped input, as in slice 9.

```sh
cd guild-portfolio/tracker
cargo build

export DEMO=$(mktemp -d)
export TRACKER_HOME=$DEMO/store
export PATH="$PWD/target/debug:$PATH"
export GIT_AUTHOR_NAME=demo GIT_AUTHOR_EMAIL=demo@example.invalid
export GIT_COMMITTER_NAME=demo GIT_COMMITTER_EMAIL=demo@example.invalid
mkdir -p $DEMO/repo/bm $DEMO/repo/im $DEMO/repo/lm && cd $DEMO/repo
git init --quiet -b main
for d in bm im lm; do printf 'The %s project.\n' $d > $d/README.md; done
git add -A && git commit --quiet -m "chore: set up"
cd bm

tracker init
tracker project add BM --name bookmarks --root .
tracker project add IM --name importer --root ../im
tracker project add LM --name links --root ../lm
tracker create "Search finds near misses" --project BM --done-when "A near miss is found"
tracker criterion set BM-1 1 --check 'true'
echo "near misses" > spec.txt && git add -A && git commit --quiet -m "test(BM-1): describe near misses"
tracker start BM-1
tracker edit BM-1 --kind fix
echo "tidy" > notes.md && git add -A && git commit --quiet -m "tidy the notes"
touch found && git add -A && git commit --quiet -m "feat(BM-1): find near misses"
tracker gates BM-1
tracker submit BM-1
tracker show BM-1 | sed -n '/^Started/,/^$/p'
echo BM-1 | tracker approve BM-1      # exit 1: not a terminal
cd ../im
tracker create "Import a file of links" --project IM --done-when "A file becomes links" --done-when "A bad line is skipped"
tracker criterion remove IM-1 2
tracker criterion set IM-1 1 --check 'test -f found'
tracker start IM-1
touch found && git add -A && git commit --quiet -m "feat(IM-1): import a file"
tracker submit IM-1 --json | jq -c '{status, triggers: [.triggers[] | select(.fired) | .trigger], holds}'
cd ../lm
tracker create "Links have titles" --project LM --done-when "A link shows its title"
tracker criterion set LM-1 1 --check 'test -f found'
tracker start LM-1
touch found && git add -A && git commit --quiet -m "feat(LM-1): show titles"
tracker submit LM-1
jq '.started' $TRACKER_HOME/items/LM-1.json
tracker list --all --status review,done
```

The output, from `tracker init` on, with stdout and stderr together:

```
$ tracker init
Created the store at $DEMO/store.
$ tracker project add BM --name bookmarks --root .
Registered BM (bookmarks) at /private$DEMO/repo/bm.
$ tracker project add IM --name importer --root ../im
Registered IM (importer) at /private$DEMO/repo/im.
$ tracker project add LM --name links --root ../lm
Registered LM (links) at /private$DEMO/repo/lm.
$ tracker create "Search finds near misses" --project BM --done-when "A near miss is found"
Created BM-1: Search finds near misses
1 manual criterion: give it a check with `tracker criterion set BM-1 1 --check "…"`.
$ tracker criterion set BM-1 1 --check 'true'
Criterion 1 of BM-1 now has a check.
$ echo "near misses" > spec.txt && git add -A && git commit --quiet -m "test(BM-1): describe near misses"
$ tracker start BM-1
Running criterion 1's check: `true`
warning: Criterion 1's check, `true`, already passes before the work is done, so it can't be proving the new work. Make sure it tests what criterion 1 says, such as with a filter that matches a test that fails now. BM-1 started anyway.
BM-1: open → in-progress
$ tracker edit BM-1 --kind fix
Edited BM-1: changed kind.
$ echo "tidy" > notes.md && git add -A && git commit --quiet -m "tidy the notes"
$ touch found && git add -A && git commit --quiet -m "feat(BM-1): find near misses"
$ tracker gates BM-1
Running criterion 1's check: `true`
BM-1 passes the gates submit runs: G2, G5, G6, G3 and G4.
A submit would be held for review:
  T5  the criteria, their checks, the kind or the design changed after start
  T13  criterion 1's check already passed before the work, so it can't be proving it
  T14  an unscoped commit since start, which no item's scope covers: 24eddbc "tidy the notes"
$ tracker submit BM-1
Running criterion 1's check: `true`
BM-1: in-progress → review
Every gate passed, and BM-1 is held for review:
  T5  the criteria, their checks, the kind or the design changed after start
  T13  criterion 1's check already passed before the work, so it can't be proving it
  T14  an unscoped commit since start, which no item's scope covers: 24eddbc "tidy the notes"
It waits in `tracker review` for a human to approve or reject it. `tracker show BM-1 --diff` shows its work.
$ tracker show BM-1 | sed -n '/^Started/,/^$/p'
Started:
  at      2026-11-22T22:02:38Z
  hash    20ac8c0b2356a76e104e5942810e1ffd777f69e5f554ab230ed66279201471ca
  head    9e2ce7cc99079aad049af9ec426b1b5e14fa9d37
  checks  criterion 1 passed

$ echo BM-1 | tracker approve BM-1
error: `tracker approve BM-1` needs a human at an interactive terminal, who types BM-1 to confirm it, and stdin here isn't a terminal. Nothing was changed. Only a human approves a held item. It waits in `tracker review` until one approves or rejects it.
[exit 1]
$ cd ../im
$ tracker create "Import a file of links" --project IM --done-when "A file becomes links" --done-when "A bad line is skipped"
Created IM-1: Import a file of links
2 manual criteria: give each a check with `tracker criterion set IM-1 <n> --check "…"`.
$ tracker criterion remove IM-1 2
Removed criterion 2 from IM-1: A bad line is skipped
$ tracker criterion set IM-1 1 --check 'test -f found'
Criterion 1 of IM-1 now has a check.
$ tracker start IM-1
Running criterion 1's check: `test -f found`
IM-1: open → in-progress
$ touch found && git add -A && git commit --quiet -m "feat(IM-1): import a file"
$ tracker submit IM-1 --json | jq -c '{status, triggers: [.triggers[] | select(.fired) | .trigger], holds}'
Running criterion 1's check: `test -f found`
{"status":"review","triggers":["T5"],"holds":[{"trigger":"T5","message":"a written criterion is no longer any criterion's text: \"A bad line is skipped\""}]}
$ cd ../lm
$ tracker create "Links have titles" --project LM --done-when "A link shows its title"
Created LM-1: Links have titles
1 manual criterion: give it a check with `tracker criterion set LM-1 1 --check "…"`.
$ tracker criterion set LM-1 1 --check 'test -f found'
Criterion 1 of LM-1 now has a check.
$ tracker start LM-1
Running criterion 1's check: `test -f found`
LM-1: open → in-progress
$ touch found && git add -A && git commit --quiet -m "feat(LM-1): show titles"
$ tracker submit LM-1
Running criterion 1's check: `test -f found`
LM-1: in-progress → done
Verified automatically: every gate passed at 8d19c98, and refs/tracker/LM-1 keeps that commit.
$ jq '.started' $TRACKER_HOME/items/LM-1.json
{
  "at": "2026-11-22T22:02:39Z",
  "criteria_hash": "08a1f2f496b07e237228c9bdf1d7c78579a9c6152cd1e42bfb4d617c756a7da7",
  "head": "c848f04599825f54c6b065c4b54fc205043d42ff",
  "checks": [
    {
      "criterion": 1,
      "passed": false
    }
  ]
}
$ tracker list --all --status review,done
  ID    PRIORITY  KIND    STATUS  TITLE
BM  bookmarks
  BM-1  medium    fix     review  Search finds near misses
IM  importer
  IM-1  medium    change  review  Import a file of links
LM  links
  LM-1  medium    change  done    Links have titles

Every project.
Filters: status review or done.
```

Notes on the output:

- The commit SHAs, `head` and the times differ on every run. The `hash` doesn't: BM-1's, `20ac8c0b…`, came out the same on two runs, since the fingerprint covers only the criteria, kind and design. It would differ if BM-1's text, check, kind or design differed.
- BM-1's three holds are from three separate causes. T5 fires because `edit --kind fix` changed the kind after `start`, and the fingerprint `start` took is of kind `change`. T13 fires because `true` passes at `start`, so `started.checks` says `criterion 1 passed`, and the `start` warning is the only sign before the submit. T14 fires for `tidy the notes`, which touches `bm/` and has no ID. `feat(BM-1)` and `test(BM-1)` commits are scoped, so they don't. `gates` prints the same holds a submit would get, as it has since slice 9.
- The hold lines don't line up (`T5  the…`, `T13  criterion…`), because the width of the trigger name differs. TR-39 is the item for that.
- IM-1's only hold is T5, from `written`: the text "A bad line is skipped" was written at `create` and then removed. Its fingerprint was taken after the removal, so the hash half stays quiet. Only an interactive `criterion add` or `criterion accept` changes `written` after `create`, so a non-interactive command can't put the text back.
- LM-1 changes nothing after `start`, and `jq '.started'` shows what `start` wrote: `at`, `criteria_hash`, `head`, then `checks`, in that order. `criterion 1` is `false` there, since `found` didn't exist yet.
- `echo BM-1 | tracker approve BM-1` is refused as in slice 9. Typed at a terminal, `approve` runs all eleven triggers again at `HEAD`, as slice 10 describes, before it asks. BM-1 and IM-1 stay in review, and each pauses its project.

## Commands

No command or flag is new. These change:

| Command | What it does |
| --- | --- |
| `tracker start <id>` | Also records `started.criteria_hash`, before `head`. |
| `tracker submit <id>`, `status <id> done` | After the gates pass, T1 to T9, T13 and T14 run, in that order, except any the project's `review.disabled` names (T6 always runs). T5, T13 and T14 hold with a message and no `files`. |
| `tracker gates <id>` | Lists the same holds once every gate passes, and `triggers` lists the eleven that ran. |
| `tracker approve <id>` | Runs the eleven triggers again at `HEAD` before it asks, as slice 10 describes, so a T14 commit made while the item waits shows as "new since the submit". |
| `tracker show <id>` | Prints `hash` in the "Started:" block, between `at` and `head`, for an item that has one. |

Exit codes are as in slice 10. T14 runs `git log` in the project's root, so a root git can't read exits 3 as the other triggers do.

## How it works

### The fingerprint

`triggers::fingerprint(item)` in [src/triggers.rs](../src/triggers.rs) builds a JSON object of `criteria`, `kind` and `design`, in that order, and returns the SHA-256 of its text (the `sha2` crate, added to [Cargo.toml](../Cargo.toml) as 0.11.1) as lower-case hex. Each criterion is cloned with `evidence: None` first, so recording evidence after `start`, which G2 requires for a manual criterion, doesn't move the goalposts. A criterion serialises as it is written to the item file, so a field that a later slice adds, with `skip_serializing_if`, leaves the fingerprint of a criterion without it unchanged. Slice 12's judge fields rely on that.

`apply` in [src/main.rs](../src/main.rs) writes it into `Started::criteria_hash` in [src/item.rs](../src/item.rs) when an item starts, so `status <id> in-progress` records it too. Nothing else writes it. `criterion accept` doesn't take it again ([Decisions](#decisions-worth-knowing)).

### T5

`t5` returns a message built from up to two parts, joined by "; ":

1. If the item has a `criteria_hash` and it differs from `fingerprint(item)` now, "the criteria, their checks, the kind or the design changed after start". An item started before slice 11 has no hash, so this part is skipped.
2. For each text in `written` that no criterion's `text` equals, a quoted name. One gives "a written criterion is no longer any criterion's text: "…"", and several give "written criteria are no longer any criterion's text: "a" and "b"". This part needs no `started`, so it also fires for an item that hasn't started.

Adding a criterion changes the criteria, and so the hash, only after `start`: before it, the hash isn't taken yet. After `start` it fires the first part. `written` is only a list of texts with no order, so reordering doesn't fire the second.

### T13

`t13` reads `started.checks`, collects each entry with `passed: true` and names its `criterion`, so "criterion 1's check" for one and "criteria 1 and 3's checks" for several. An entry is the latest run before the work: the one at `start`, or the one made when a check was given after `start` ([slice 7](slice-7-readme.md)). A criterion with no entry isn't named. An item with no `started` returns nothing.

T13 needs nothing from the check but the entry. It can't tell why a check passed: `cargo test some_filter` that matches no test passes, as does `true`. The test for the filter case runs a real `cargo test` in a throwaway crate ([Testing](#testing)).

### T14

`git::unscoped(root, base)` in [src/git.rs](../src/git.rs) runs `git log -z --full-history --format=%H%n%B <range> -- .` in the project's root and keeps each commit whose first line fails `git::is_scoped_header`. The `.` limits it to commits that touch the root, and `--full-history` keeps a commit that a merge would hide. The range is `since_range(base)`, which is `<base>..HEAD`, the one `git::since` uses for T4, so the two triggers agree on what "after start" means. With no `base`, for an item started in a repository with no commits yet, the range is `HEAD`, so every commit counts. A repository with no `HEAD` returns none.

`is_scoped_header` takes the text between the first `(` and the next `)`, and requires a project key (`config::is_valid_key`), a dash and digits only, then `is_item_header` for that scope. A scope shaped like an ID counts whether or not that item exists: `docs(XX-2): …` isn't unscoped even if there's no XX-2. `BM-1: add search`, `feat(search): …`, `feat(bm-1): …`, `feat(BM-): …`, `feat(BM-1a): …` and `feat(BM-1):add` are unscoped, and so is a merge commit headed "Merge branch …", as is an empty header. `t14` names the commits oldest first, though `unscoped` returns them newest first, and keeps no `files`.

`run` calls `unscoped` once for an item that has `started`. An item with no `started`, written before slice 7, gets none.

### Order and switching off

`run` adds T5 after T4, and T13 and T14 after T9, so a hold list reads in trigger number order, T13 after T9. T5, T13 and T14 each go through `add`, so a project's `review.disabled` switches any of them off, and a switched-off trigger isn't in `triggers`. `config::ALWAYS_ON` is unchanged: none of the three is on it.

## JSON output

No command's JSON shape changes. These do change in content:

| Command | What's new | Schema |
| --- | --- | --- |
| `submit --json`, `gates --json` | `triggers` lists eleven entries, T1 to T9, T13 and T14, and `holds` can include T5, T13 and T14, each with a `message` and no `files` | [submit.schema.json](../schemas/submit.schema.json), [gates.schema.json](../schemas/gates.schema.json) |
| Any command that returns an item | `item.started.criteria_hash`, 64 lower-case hex characters, left out for an item started before slice 11 | [item.schema.json](../schemas/item.schema.json) |

A later slice can add a judge field to a criterion without changing the fingerprint of one that has none. It must not rename or reorder a `Criterion`, `Kind` or the `criteria`, `kind` and `design` keys, which changes the fingerprint of every item that's in progress ([Limits](#limits-and-what-later-slices-change)).

## Code tour

| File | What's new or changed |
| --- | --- |
| [src/triggers.rs](../src/triggers.rs) | `fingerprint`, `t5`, `t13` and `t14`. `run` runs them, and asks `git::unscoped` for an item that has `started` |
| [src/git.rs](../src/git.rs) | `since_range` (shared by `since` and `unscoped`), `is_scoped_header` and `unscoped`. A unit test, `a_header_is_scoped_when_its_scope_is_any_items_id` |
| [src/item.rs](../src/item.rs) | `Started::criteria_hash`, with a serde default and `skip_serializing_if`, before `head` |
| [src/main.rs](../src/main.rs) | `apply` writes `criteria_hash` at start, and `started_text` prints it as `hash` |
| [Cargo.toml](../Cargo.toml), `Cargo.lock` | `sha2` 0.11.1 |
| [schemas/item.schema.json](../schemas/item.schema.json) | `started.criteria_hash`, with the pattern `^[0-9a-f]{64}$` |
| [tests/tampered_proof.rs](../tests/tampered_proof.rs) | New (`e7406d8`): 17 tests in eight modules, `criterion_1` to `criterion_5`, `commits_scoped_elsewhere`, `missing_start_check` and `fingerprint`. Helpers `env_with_project`, `create`, `started`, `write`, `do_the_work`, `submit`, `fired`, `ran`, `hold`, `assert_held_by` and `assert_quiet` |
| [tests/common/mod.rs](../tests/common/mod.rs) | `BEFORE_THE_WORK_FAILS`, and `Env::add_criterion` uses it |
| [tests/fixtures.rs](../tests/fixtures.rs), `tests/fixtures/slice-11/` | XX-23, and `slice_11s_fixture_has_a_criteria_fingerprint_and_the_new_holds` (`29ad53b`) |
| [tests/states.rs](../tests/states.rs), [tests/gates.rs](../tests/gates.rs), [tests/review.rs](../tests/review.rs), [tests/checks.rs](../tests/checks.rs), [tests/show_and_delete.rs](../tests/show_and_delete.rs) | Ten existing tests changed (`e7406d8`), [below](#older-tests-that-changed) |
| [docs/DESIGN.md](DESIGN.md) | `42213dc`: §2.5's T14 row says what counts as scoped and what "since `start`" means, and §4.3 has a bullet for `started.criteria_hash` and what it covers |
| [docs/PLAN.md](PLAN.md) | Slice 11's status, `todo` to `in review` (`ce58426`). `46536ec` marked it `in progress` |

To add a trigger that reads the commits since `start`, as T14 does: add a function to [src/triggers.rs](../src/triggers.rs), call `git::since` or a `git log` over `since_range` so it shares the range, and add an `add("T…", …)` line to `run` in trigger order. To add a field that T5 should cover, put it in `fingerprint`'s object, and say in DESIGN.md §4.3 that it's covered. That changes the fingerprint of every item in progress.

## Testing

Run the test command in `tracker/`. TR-7's five criteria each run their own module in `tests/tampered_proof.rs`, for example `cargo test --test tampered_proof criterion_4::`. The module path ends in `::`, so `criterion_1::` can't match a `criterion_10`.

Every project in `tests/tampered_proof.rs` is XX, whose root `<work>/xx` is a subfolder of the work folder's repository, as TR's root is a folder in guild-portfolio and PLAN.md's integration risk "Self-tracking depends on the folder rule" asks. `env_with_project` commits one file unscoped before any item exists. `create` creates XX-1 and commits it as its test commit. `started` adds a script criterion whose check is `test -f done`, which fails until `do_the_work` adds `xx/done`, and starts the item. No test needs `script`.

**The harness changed.** `Env::add_criterion` in [tests/common/mod.rs](../tests/common/mod.rs) gave a criterion the check `true`. At `start` that passes, so T13 would hold nearly every test that submits. It now gives `BEFORE_THE_WORK_FAILS`: `! ps -o args= -p $PPID | grep -qwE 'start|in-progress|criterion'`. A check runs as a child of the `tracker` that ran it, so `$PPID` is that process, and `ps` reads its arguments. The check fails when that command is `start`, `status <id> in-progress` or a `criterion` command, which are the ones that run a check before the work (§2.4), and passes for `gates`, `submit` and `approve`. It matches whole words, so a command with one of the three words as an argument fails it too. It relies on `ps -o args= -p`, which macOS and Linux both have.

### Coverage of the _Done when_ checks

| _Done when_ | Tests in `tests/tampered_proof.rs` |
| --- | --- |
| T13 fires for a `cargo test` filter that matched no tests at start. | `criterion_1`, 2 tests, in a throwaway crate whose one test, `search_finds_a_tag`, passes once `xx/found` exists: `t13_fires_for_a_cargo_test_filter_that_matched_no_tests_at_start` (the check is `cargo test --offline search_finds_a_tga`, so it passes at start with no test run, and the hold names criterion 1) and `t13_stays_quiet_for_a_cargo_test_filter_that_failed_at_start` |
| T5 fires when a non-interactive command changes or removes a written criterion's text before start, and stays quiet when one adds a criterion or gives one a check. | `criterion_2`, 3 tests: `t5_fires_when_a_written_criterion_is_removed_before_start` (the hold names the removed text and not the kept one), `t5_fires_when_a_written_criterion_is_reworded_before_start` (a remove and an add, since no command rewords) and `t5_stays_quiet_when_a_criterion_is_added_or_given_a_check_before_start` |
| `edit --kind` after start fires T5. | `criterion_3`, 2 tests: `edit_kind_after_start_fires_t5` and `t5_stays_quiet_when_nothing_changed_after_start` |
| T14 fires for an unscoped commit after start, including one headed `XX-1: …`. It doesn't fire for one made before start, or for an unscoped commit that only touches files outside the project root. | `criterion_4`, 5 tests: `t14_fires_for_an_unscoped_commit_after_start` (the hold has the commit's short SHA and header), `t14_fires_for_a_commit_headed_with_the_id_alone`, `t14_doesnt_fire_for_an_unscoped_commit_before_start`, `t14_doesnt_fire_for_an_unscoped_commit_only_outside_the_root` and `t14_holds_an_unscoped_delete_of_a_test_that_the_item_re_adds` |
| A reopened item resubmitted with unchanged code is held by T13. | `criterion_5::a_reopened_item_resubmitted_with_unchanged_code_is_held_by_t13`: done and quiet, then `reopen` and `start`, which warns once, then held by T13 only |

The _Prompt_'s requirements that no _Done when_ bullet names:

| Requirement | Tests |
| --- | --- |
| A criterion with no `started.checks` entry reads as a check that didn't pass. | `missing_start_check::t13_reads_a_missing_started_check_as_one_that_didnt_pass`: the entry is taken out of the file by hand, since the real route, a check given in review, needs a terminal |
| The fingerprint is recorded by `start`, before `head`, in 64 hex characters. | `fingerprint::start_records_the_criteria_fingerprint_before_head`, and the fixture test below |
| Evidence is removed from the fingerprint. | `fingerprint::evidence_recorded_after_start_doesnt_fire_t5`: T5 ran and didn't fire, and the item is held only by T6 |
| A commit scoped to another item isn't unscoped. | `commits_scoped_elsewhere::t14_doesnt_fire_for_a_commit_scoped_to_another_item`, and the unit tests `a_header_is_scoped_when_its_scope_is_any_items_id` and `a_header_is_scoped_to_an_item_only_by_its_conventional_commit_scope` in `src/git.rs` |
| Slice 10 review 2's security 1: an unscoped delete after `start` that the item re-adds weakened. | `criterion_4::t14_holds_an_unscoped_delete_of_a_test_that_the_item_re_adds`: held by T14 alone, since T4 sees only an added file |

### Other tests

- `slice_11s_fixture_has_a_criteria_fingerprint_and_the_new_holds` in `tests/fixtures.rs` reads XX-23, which has `started` in the order `at`, `criteria_hash`, `head`, `checks`, and holds T5, T13 and T14 in review. It checks that the file comes back byte for byte, loses nothing on a rewrite, and matches the item schema. The fixture was written for this slice. It wasn't written by an installed binary, since none has T5, T13 or T14.
- **Seen failing first.** TR-7's five checks, one per `criterion_N::` module, all failed at `start`: `started.checks` records criterion 1 to 5 as `passed: false`. The three other modules (`commits_scoped_elsewhere`, `missing_start_check` and `fingerprint`) and the fixture test run in no criterion's check, so the item doesn't record whether they were seen failing.
- Not tested: a check given after `start` firing T5 (the fingerprint covers it), a changed `design` firing T5, `criterion accept` leaving the fingerprint alone and clearing T5's `written` half, an item with no `criteria_hash` comparing only `written`, and a rewritten commit or an item with no `started.head` for T14.

### Older tests that changed

`e7406d8` changed ten existing tests before `tracker start TR-7`, so they were seen failing with TR-7's. The builder listed them:

| Test | File | What changed |
| --- | --- | --- |
| `submit_prints_the_shape_section_3_gives_it` | `tests/states.rs` | The pinned `triggers` list gained T5, T13 and T14 |
| `a_script_criterion_needs_no_evidence` | `tests/gates.rs` | The same |
| `criterion_1::an_item_with_neither_still_auto_verifies`, `criterion_2::submit_json_matches_the_held_shape_in_section_3`, `tr_29::gates_lists_the_holds_a_submit_would_get` and `tr_29::gates_says_no_holds_when_no_trigger_fires` | `tests/review.rs` | The same, four times |
| `show_prints_every_field_the_verification_records_and_history_with_times` | `tests/show_and_delete.rs` | Expects `BEFORE_THE_WORK_FAILS` where it expected `true`, and "criterion 2 failed" at start where it expected "passed" |
| `a_passing_checks_last_20_lines_are_kept` and `a_kept_line_longer_than_the_limit_is_cut_and_marked` | `tests/checks.rs` | Each sets `review.disabled: ["T13"]`, since its check passes at start on purpose |
| `criterion_add_and_set_refuse_a_newer_trackers_file_before_running_a_check` | `tests/checks.rs` | Its unknown field in `started` is `later_field`, since `criteria_hash` is now known |

`tests/common/mod.rs` changed too: `BEFORE_THE_WORK_FAILS` replaced `--check true` in `Env::add_criterion`. That makes six files with changed lines, and T4's hold names them: `tests/checks.rs`, `tests/common/mod.rs`, `tests/gates.rs`, `tests/review.rs`, `tests/show_and_delete.rs` and `tests/states.rs`, each "(e7406d8 before start)". The mark says when the edit was made, not that it was right, so the approve read them, as §2.5 asks. The two `disabled: ["T13"]` edits are the ones that most need that reading, since they switch a trigger off for a test.

### Checks done by hand

| What | How | Result |
| --- | --- | --- |
| The demo above | Run as written with this branch's `target/debug/tracker`, stderr captured, on a temporary `TRACKER_HOME` | The output above, with the temporary folder replaced by `$DEMO`. Run twice, with the same output apart from times and SHAs and the same `hash` for BM-1 |
| TR-7's approve in a real terminal | The user typed `TR-7` into `tracker approve TR-7` at a real terminal, after reading the diff, with the installed slice 10 binary | TR-7 went from review to done: history has `approve  review → done  from a terminal`, and `verification` is `by: human` at `ce58426` with `refs/tracker/TR-7`. This is the first held TR item, so it's the first time slice 10's `approve` question, which names `HEAD`, marks holds "(new since the submit)" and lists one that "no longer fires", ran in a real terminal. Slice 10's [Still open](slice-10-readme.md#still-open) asked for that check. This readme has no copy of the question's text, so what each of its lines said isn't recorded |
| What the installed binary couldn't check | By the orchestrator, before the approve: every commit since `started.head` (`e7406d8`) is scoped to TR-7, the criteria's texts still match `written`, and all five checks failed at start | Confirmed against the record in this readme: `git log` after `e7406d8` has `46536ec`, `03408b3`, `42213dc`, `29ad53b` and `ce58426`, each `(TR-7)`, `tracker show TR-7` has the five criteria's texts equal to `written`, and `started.checks` has all five `passed: false`. These are what T14, T5 and T13 would have checked |

### Tracking TR-7

TR-7, a `capability` with `design` `docs/DESIGN.md#2.5`, was created at 2026-11-21T07:07:43Z with slice 7's other items. Its five criteria are PLAN.md's _Done when_ bullets, unchanged, each with a check that runs one module of `tests/tampered_proof.rs`. No criterion was added. The slice 10 binary was installed before TR-7 started and tracked it throughout:

| Step | When (UTC) | What |
| --- | --- | --- |
| Tests, the older tests' changes and the helper, committed | | `e7406d8`, at 21:16:50Z, before `start` |
| `tracker start TR-7` | 21:17:19Z | `started.head` `e7406d8`. `started.checks`: all five criteria failed. No `criteria_hash`, since this binary has no T5 |
| The code, DESIGN.md and the fixture | | `03408b3` and `42213dc`, then `29ad53b` |
| `tracker submit TR-7` | 21:26:22Z | G2, G5, G6, G3 and G4 passed, then T1, T3, T4, T6, T7, T8 and T9 ran. Held under T1, T3, T4, T7 and T8 |
| `tracker approve TR-7` | 21:54:42Z | From a terminal, verified `human` at `ce58426` |
| PLAN.md: slice 11 in review | | `ce58426`, made before the approve. TR-7's `verification.commit` is that commit |

- **The five holds** read: T1, "it's a capability, so a human should check the design fit". T3, "dependency manifests changed: Cargo.lock and Cargo.toml". T4, the six test files above, each "(e7406d8 before start)". T7, "974 lines changed, more than the project's max_diff_lines of 300". T8, "the project's design doc changed: docs/DESIGN.md". They're recorded in `verification.triggers`, and `tracker show TR-7` is the record, with each check's command, result and saved output.
- **T5, T13 and T14 didn't run on TR-7.** The installed binary has none of them. This slice's own binary, run on TR-7, would have run all three. By the checks above it wouldn't have fired T13 (no check passed at start) or T14 (every commit since `started.head` is scoped to TR-7), and T5's `written` half is quiet. T5's hash half has nothing to compare, since TR-7 has no `criteria_hash`. That wasn't run.
- **TR-7 has no `criteria_hash`**, and no item started before the next one does. The first TR item started with the slice 11 binary is the first to have one, and the first for which T5's first half can fire.

## Decisions worth knowing

- **`criterion accept` doesn't take the fingerprint again.** It records every criterion's text in `written`, which stops T5's second part from firing. A criterion the user edits by hand after `start` still fires the first part, as any change to the goalposts after `start` does. The alternative, retaking the hash at an accept, would let an accept hide a change to a check or the kind. DESIGN.md §4.3 says so.
- **T14 counts a scope shaped like an ID as scoped, even if no item has that ID.** `is_scoped_header` checks the shape: a project key, a dash and digits. It never reads the store, so a commit scoped to a deleted item or to one in another project doesn't fire it. The cost is that a made-up ID gets past T14, and its commits get past the other triggers too, since they read only the item's own ID. That's the §7 limit "work scoped to the wrong item's ID still gets past both" (G5 and T14). DESIGN.md §2.5 says so.
- **Rewritten commits count as after start.** T14 reads `<started.head>..HEAD`, as T4 does, so a rebase or an amend after `start` gives an item's own commits new SHAs that `started.head` can't reach. An unscoped one then fires T14, and it errs toward more holds. `since_range` is the one place that says so, so a change to it moves both triggers.
- **With no `started.head`, every commit counts.** An item started in a repository with no commits yet has none, so `unscoped` is given no base and reads all of `HEAD`'s history. That matches an item whose whole history is "since start".
- **T14 looks only at commits that touch the project root.** An unscoped commit that only touches files elsewhere in the repository doesn't fire it, by the folder rule (§2.1). `git log … -- .` does this, and `--full-history` keeps a commit that a merge would hide.
- **The fingerprint omits evidence and nothing else.** Evidence is the one criterion field that can only be written once the work exists, and G2 needs it. Everything else, including a check command, is hashed, so changing a check after `start` fires T5.
- **The fingerprint is hashed from serialised JSON** with the keys in a fixed order (`criteria`, `kind`, `design`), and serde_json's `preserve_order` feature keeps that order. A criterion's field order is the struct's, which is the item file's order (§4.3). A change to either changes the fingerprint of every item in progress, and an item started before slice 11 has none, so only `written` is compared for it.
- **T13 reads a missing entry as a check that didn't pass.** A check given while the item is in review isn't run then, so its entry is the old check's, or missing for a manual criterion. T5 holds the item for the change anyway.
- **T5's two parts are one trigger.** One message joins them, so a hold that's both reads as both.
- **T5, T13 and T14 can each be switched off** with `review.disabled`. DESIGN.md §2.5 lists only T6, T10 and T12 as the triggers a project can't switch off.
- **The test helper's check reads `ps`.** `BEFORE_THE_WORK_FAILS` fails only when `tracker` was started with `start`, `in-progress` or `criterion` in its arguments. It needs no file, so the check leaves the tree as it found it (§2.4).
- **Changing existing tests fired T4, and that was accepted.** The pinned `triggers` lists had to grow, and `Env::add_criterion`'s check had to change, as slice 10's readme said. All of it is in `e7406d8`, before `start`, so every edit is marked "before start" and T4's hold is the expected one. Editing an existing test fires T4 whichever commit does it, and slice 12's builder should expect the same for the lists that now name eleven triggers.
- **TR-39 is out of scope.** The hold lines don't line up when trigger names differ in width, now that T13 and T14 sit beside T5. It's a layout fix in `holds_text` in [src/main.rs](../src/main.rs) and in `review`'s list, and it's an open item, not part of this slice.

## Still open

- **Help text.** `submit`'s help, on `Submit` in [src/main.rs](../src/main.rs), lists T1, T2, T3, T4, T6, T7, T8 and T9, and not T5, T13 or T14. It also has a stray line break, and `Started`'s doc comment in [src/item.rs](../src/item.rs) says "slice 11's T5" as if it were still to come. No test checks the text.
- **Test gaps** (the builder's list, [Other tests](#other-tests)): a check given after `start` firing T5, a changed design firing T5, `criterion accept` not retaking the hash, an item without a hash comparing only `written`, and rewritten commits and a missing `started.head` for T14.
- **A `started.head` that git no longer has.** Slice 10's review 2, bug 3, left it to be decided once for T4 and T14. T14 uses the same range as T4, and `git log` over a range with an unknown base fails, so `gates`, `submit` and `approve` exit 3 with git's error. This slice didn't change that, and didn't run it.
- **TR-39**, the misaligned hold lines.
- **Carried from slice 10** ([slice 10's Still open](slice-10-readme.md#still-open)): how T7 counts has no line in DESIGN.md §7, T1 names a source file that the item added and then renamed or deleted, T4 names at most 10 files, and the digest (slice 16) must decide how a reader sees a line added to switch a test off. TR's own `review.risky_paths` is still the user's to write.
- **DESIGN.md §7** still says an agent can replace a check the user gave a criterion before `start`, and only T13 sees it. That stays true.

## Limits, and what later slices change

- **Slice 12** adds judge fields to a criterion. They must serialise only when set, so the fingerprint of a criterion without one is unchanged, and T5 covers them as DESIGN.md §2.1 says of `started`: the fingerprint covers each criterion's text, check command, judge questions and judge inputs. A judge's answers are evidence, not goalposts: don't put them in the criterion `fingerprint` hashes. Slice 12 also adds T10, which `config::ALWAYS_ON` already lists, and `run` then needs the judge settings.
- **Slice 14** extends T13 to a criterion judge whose answers all passed at start. `t13` reads `started.checks`, which holds a criterion's script check and not a judge's answers, so slice 14 needs a record of its own for them. It also adds T11 beside T13, and G1's refusal for a judge with nothing to grade.
- **Slice 15** makes `approve` count held judge questions as passed for that approval only. T5, T13 and T14 are mechanical, so an approve that clears them changes nothing in this slice's code.
- **Slice 16**'s digest lists every `criterion accept`, which is the only thing that clears T5's `written` half, every `edit` that removes `needs-review` or changes a capability's kind, and every `approve`.
- **A change to the fingerprint's inputs** moves every in-progress item's hash, so T5 fires on each. PLAN.md's integration risk says to install a new binary only between items.
- **T14 and unscoped commits, from slice 11 on.** Once this binary is installed, an unscoped commit that touches `tracker/` while a TR item is in progress fires T14 at its submit, and one made while it waits in review fires T14 at `approve`, marked new, since `approve` runs the triggers again. Readme and review commits are unscoped: this readme's own commit is. Make them only when no TR item is in progress or in review, or scope them to the item.
- **The helper check depends on `ps`.** A later slice that adds a command which runs a check before the work, or renames `start`, `in-progress` or `criterion`, must change `BEFORE_THE_WORK_FAILS` too, or its tests' items start passing at start and T13 holds them.
- **A held item pauses its project** until it's decided (§4.4). With this binary installed, a TR item that passes its checks at start, edits its kind after `start`, or sits behind an unscoped commit is held too. TR-8 to TR-12 are `capability`, so T1 holds each, as it held TR-7.
- **The tests that put an item in review by editing its file** still do, in `tests/states.rs`, `tests/gates.rs`, `tests/criteria.rs`, `tests/organise.rs` and `tests/show_and_delete.rs`. They test refusals that don't depend on how the item got there.

# Slice 9: Inbox, approve, reject

`tracker` is a command-line work tracker, written in Rust, that keeps one central list of work items across many projects and decides when an item is really done. This document covers **slice 9** of the 16 in [PLAN.md](PLAN.md): what it built, how it works, and how to test it. The earlier readmes cover the rest: [slice 1](slice-1-readme.md) the store, `create` and `list`, [slice 2](slice-2-readme.md) the states and `history`, [slice 3](slice-3-readme.md) priority, kind, labels, `edit` and `list`'s filters, [slice 4](slice-4-readme.md) `show` and `delete`, [slice 5](slice-5-readme.md) bad data and polish, [slice 6](slice-6-readme.md) criteria and gates G1, G2 and G6, [slice 7](slice-7-readme.md) running checks, with gates G3 and G4 and `started.checks`, and [slice 8](slice-8-readme.md) G5, the `auto` record and `refs/tracker/<ID>`.

After slice 9 you can **see held items in `review` and approve or reject them**. The spec is [DESIGN.md](DESIGN.md). This slice implements the moves into and out of review from [§2.2](DESIGN.md#22-states), triggers T6 and T9 and "Triggers are worked out fresh on every submit" from [§2.5](DESIGN.md#25-review-triggers-does-it-need-a-human), `approve`, `reject` and the `human` record from [§2.7](DESIGN.md#27-human-actions), `review`, `approve`, `reject`, `show --diff`, `status`'s refusal of a held item and the held `submit --json` shape from [§3](DESIGN.md#3-interface), and `holds` and `verification.triggers` from [§4.3](DESIGN.md#43-item-file). It's tracked in `tracker` itself as item TR-5, with the binary installed from slice 8, and it's the first TR item verified against a commit, with a ref ([Tracking TR-5](#tracking-tr-5)). [Review 1](reviews/slice-9-review-1.md)'s fixes are items TR-27 to TR-30 ([below](#fixes-after-review-1)).

| | |
| --- | --- |
| Status | Every _Done when_ check has a test, and every test passes, as does the test command. One half of a bullet isn't done: typing the ID into `approve` in a real terminal, which PLAN.md leaves to a check by hand, hasn't been tried, and neither has `reject` ([Checks done by hand](#checks-done-by-hand)). The tests drive both under `script` only. TR-5 is `done`, verified `auto` at `10e519a` by the installed slice 8 binary, which ran G2, G5, G6, G3 and G4, with `refs/tracker/TR-5` pointing at that commit. All eight of its checks failed at `start`, as they should. That binary has no triggers, so nothing could have held TR-5, and none of its criteria is manual anyway: each has a check, criterion 4 included ([Tracking TR-5](#tracking-tr-5)). `c660f3b` changed four existing tests after `start`, because a manual criterion no longer auto-verifies ([Older tests that changed](#older-tests-that-changed)). From [review 1](reviews/slice-9-review-1.md), bugs 1 and 2, security 1, design 2 and 3 and test gaps 1 to 3 are fixed, as TR-27 to TR-30. Each is `done`, verified `auto` by the installed slice 8 binary, with a commit and `refs/tracker/<ID>` ([Tracking review 1's fixes](#tracking-review-1s-fixes)). Design 1 (`approve` doesn't run the triggers again) is left for slice 10, process 1 (typing into `approve` and `reject` in a real terminal) is still the user's to do before slice 10 starts, and process 2 needs no action ([below](#fixes-after-review-1)). The review's "Slice 10 must decide" list is left for slice 10. PLAN.md has slice 9 as `in review`. The branch isn't merged into `main`, and neither are slices 7 and 8, which it's stacked on. |
| Branch | `feat/tracker/slice-9`, stacked on `feat/tracker/slice-8` at `9db4792`. Slice 8 isn't merged into `main`, so this slice's commits are `git log --oneline --reverse feat/tracker/slice-8..feat/tracker/slice-9`, not the range from `main`'s merge base, `37abcc3`, which would include slices 7 and 8 as well. In order: `ed83b46` (TR-5's tests and the schemas, committed before `tracker start TR-5`), `d7b9e20` (PLAN.md: slice 9 in progress), `3b79ea1` (the triggers, `review`, `approve`, `reject` and `show --diff`), `c660f3b` (four older tests that a T6 hold changes), `0ddc010` (fixtures XX-20 and XX-21), `10e519a` (DESIGN.md), `4c19e71` (PLAN.md: slice 9 in review), `657559b` (this readme's first version) and `236bc39` (review 1). Then review 1's fixes, `a63c850` to `58e0afb` ([below](#fixes-after-review-1)), two docs commits that belong to no item, `e2d3c3d` and `f4ffaa9`, and this readme's update |
| Test command | `cargo fmt --check && cargo clippy -- -D warnings && cargo test`, run in `tracker/`. All three pass. |
| Tests | 337: 39 in `tests/create_and_list.rs`, 36 in `tests/commits.rs`, 34 in `tests/organise.rs`, 28 in `tests/review.rs`, 27 in `tests/states.rs`, 25 in `tests/show_and_delete.rs`, 23 in `tests/criteria.rs`, 22 in `tests/gates.rs`, 18 in `tests/checks.rs`, 14 in `tests/item_files.rs`, 13 in `tests/bad_data.rs`, 12 in `tests/fixtures.rs`, 11 in `tests/docs_links.rs`, 9 in `tests/polish.rs`, 6 in `tests/output.rs`, 5 in `tests/schemas.rs`, 4 in `tests/store_writes.rs`, 2 in `tests/item_ids.rs`, 7 unit tests in the library (4 in `src/gates.rs`, 1 each in `src/store.rs`, `src/checks.rs` and `src/git.rs`) and 2 in `src/main.rs` |

## Contents

- [What's in this slice](#whats-in-this-slice)
- [Try it](#try-it)
- [Commands](#commands)
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

- **Triggers**, which run on `submit` and `status <id> done` once every gate has passed. This slice has two:
  - **T6** fires when any criterion is manual, with neither a check nor a judge, naming each one: "criterion 1 is manual, so only a human can check its evidence".
  - **T9** fires when the item has the label `needs-review`: "it has the label needs-review, which asks for a human's review".
- **A held submit** moves the item from in-progress to review, with each trigger that fired in `holds`. It writes no verification record and creates no ref. A submit that no trigger holds auto-verifies, as in slice 8.
- **`tracker review`**, the inbox: every held item, from every project, wherever it's run, with each hold and a hint to run `tracker show <ID> --diff`, then `approve` or `reject`. It prints "Nothing needs you." when nothing is held, and names files it skipped, as `list` does.
- **`tracker approve <ID>`**, from an interactive terminal only: it says what holds the item, asks for the ID, re-runs submit's gates, G5's second look included, and then either moves the item to done, verified `human` against the `HEAD` its own G5 saw, with the ref set there and the holds moved to `verification.triggers`, or leaves it in review with exit 2.
- **`tracker reject <ID> --reason "…"`**, from an interactive terminal only: it checks the reason, asks for the ID, sends the item back to in-progress, puts the reason in history and drops `holds`.
- **`tracker show <ID> --diff`**, which adds the item's commits, newest first, and each one's patch, oldest first, as `git show` prints it. `show` also prints an item's `holds`, and a `human` record's approved triggers.
- **The move table** has §2.2's three new arrows: `submit` from in-progress to review, `approve` from review to done, and `reject` from review to in-progress. `allowed` lists `submit` twice from in-progress, and `approve` and `reject` from review.
- **Every move refused for an item in review**, through a command or through `status`, names `tracker approve <ID>` and `tracker reject <ID> --reason "…"`. Until now it said this version could do neither, and `allowed` was empty.
- **`tracker gates <ID>`** (changed by [review 1's fixes](#fixes-after-review-1)): once every gate passes, it runs the triggers too and lists them and the holds a submit would get, in the text and in `triggers` and `holds` in the JSON, so an agent can tell before it submits that the item would wait in review. It still changes nothing.

**Not built yet:**

- **T1, T2, T3, T4, T7 and T8** (slice 10), which read the item's changed files, and with them the project's `review.disabled`. `show --diff` doesn't yet list the files that fired a trigger first, which §3 asks for.
- **T5, T13 and T14** (slice 11). A check that passed at start, or an unscoped commit since start, still holds nothing.
- **T10, T11 and T12**, and `approve` counting held judge questions as passed (slices 12, 14 and 15). `approve` runs no judges, since there are none yet.
- **The digest** (slice 16), which lists every `approve` and `reject`, and `review`'s flag for an overdue digest, which §3 says the inbox shows.
- **`tracker judges`** (slice 16), which needs each `approve` and `reject` recorded against the judge questions that held the item (slice 15).

## Try it

Always set `TRACKER_HOME` while developing. Without it, the tool uses your real store at `~/.tracker`.

The two projects, BM and IM, are the folders `bm` and `im` inside one throwaway repository, as TR is a folder inside guild-portfolio. The demo's commits use a made-up name, so they work whatever your own git settings say.

```sh
cd guild-portfolio/tracker
cargo build

export DEMO=$(mktemp -d)
export TRACKER_HOME=$DEMO/store
export PATH="$PWD/target/debug:$PATH"
export GIT_AUTHOR_NAME=demo GIT_AUTHOR_EMAIL=demo@example.invalid
export GIT_COMMITTER_NAME=demo GIT_COMMITTER_EMAIL=demo@example.invalid
mkdir -p $DEMO/repo/bm $DEMO/repo/im && cd $DEMO/repo
git init --quiet -b main
echo notes > im/notes.txt
git add -A && git commit --quiet -m "chore: set up"
cd bm
tracker init
tracker project add BM --name bookmarks --root .
tracker project add IM --name importer --root ../im
tracker review
```

An item with a script criterion auto-verifies, as in slice 8. One with a manual criterion is held under T6, and one labelled `needs-review` under T9. A commit scoped to `BM-12` is made among BM-2's:

```sh
tracker create "Search filters bookmarks by tag"
tracker criterion add BM-1 "Filtering by tag shows only tagged bookmarks" --check 'cat tags.txt'
tracker start BM-1
echo "test filters_by_tag ... ok" > tags.txt && git add tags.txt && git commit --quiet -m "feat(BM-1): filter by tag"
tracker submit BM-1
tracker create "The empty state is friendly" --done-when "The empty state says what to do next"
tracker evidence BM-2 1 "Run the app with no bookmarks and read the message"
tracker start BM-2
echo "No bookmarks yet. Add one with the + button." > empty.txt && git add empty.txt && git commit --quiet -m "feat(BM-2): explain the empty state"
echo "newest first" > sort.txt && git add sort.txt && git commit --quiet -m "feat(BM-12): sort by date"
tracker submit BM-2
tracker create "Import bookmarks from a file" --project IM --label needs-review
tracker criterion add IM-1 "A file of URLs becomes bookmarks" --check 'test -f import.txt'
tracker start IM-1
touch ../im/import.txt && git add ../im && git commit --quiet -m "feat(IM-1): import from a file"
tracker submit IM-1
```

The inbox, and BM-2's work:

```sh
tracker review
tracker review --json
tracker show BM-2 --diff
```

Only a human at a terminal approves or rejects, a held item pauses its project, and `show --diff` needs a repository:

```sh
echo BM-2 | tracker approve BM-2      # exit 1: not a terminal
tracker reject BM-2                   # exit 1: no reason
echo BM-2 | tracker reject BM-2 --reason "Say where the + button is."   # exit 1: not a terminal
tracker status BM-2 done              # exit 1: a held item is a human's to decide
tracker approve BM-1                  # exit 1: BM-1 isn't in review
tracker create "Sort bookmarks by date" --done-when "Newest first by default"
tracker start BM-3                    # exit 2: G1, BM-2 is in review
mkdir $DEMO/plain
tracker project add PL --name plain --root $DEMO/plain
tracker create "Anything" --project PL
tracker show PL-1 --diff              # exit 3: not in a git work tree
tracker list --all
cat $TRACKER_HOME/items/BM-2.json
```

The output, from `tracker init` on, with stdout and stderr together:

```
$ tracker init
Created the store at $DEMO/store.
$ tracker project add BM --name bookmarks --root .
Registered BM (bookmarks) at $DEMO/repo/bm.
$ tracker project add IM --name importer --root ../im
Registered IM (importer) at $DEMO/repo/im.
$ tracker review
Nothing needs you.
$ tracker create "Search filters bookmarks by tag"
Created BM-1: Search filters bookmarks by tag
$ tracker criterion add BM-1 "Filtering by tag shows only tagged bookmarks" --check 'cat tags.txt'
Added criterion 1 to BM-1: Filtering by tag shows only tagged bookmarks
$ tracker start BM-1
Running criterion 1's check: `cat tags.txt`
BM-1: open → in-progress
$ echo "test filters_by_tag ... ok" > tags.txt && git add tags.txt && git commit --quiet -m "feat(BM-1): filter by tag"
$ tracker submit BM-1
Running criterion 1's check: `cat tags.txt`
BM-1: in-progress → done
Verified automatically: every gate passed at eb6677c, and refs/tracker/BM-1 keeps that commit.
$ tracker create "The empty state is friendly" --done-when "The empty state says what to do next"
Created BM-2: The empty state is friendly
1 manual criterion: give it a check with `tracker criterion set BM-2 1 --check "…"`.
$ tracker evidence BM-2 1 "Run the app with no bookmarks and read the message"
Recorded evidence for criterion 1 of BM-2.
$ tracker start BM-2
BM-2: open → in-progress
$ echo "No bookmarks yet. Add one with the + button." > empty.txt && git add empty.txt && git commit --quiet -m "feat(BM-2): explain the empty state"
$ echo "newest first" > sort.txt && git add sort.txt && git commit --quiet -m "feat(BM-12): sort by date"
$ tracker submit BM-2
BM-2: in-progress → review
Every gate passed, and BM-2 is held for review:
  T6  criterion 1 is manual, so only a human can check its evidence
It waits in `tracker review` for a human to approve or reject it. `tracker show BM-2 --diff` shows its work.
$ tracker create "Import bookmarks from a file" --project IM --label needs-review
Created IM-1: Import bookmarks from a file
$ tracker criterion add IM-1 "A file of URLs becomes bookmarks" --check 'test -f import.txt'
Added criterion 1 to IM-1: A file of URLs becomes bookmarks
$ tracker start IM-1
Running criterion 1's check: `test -f import.txt`
IM-1: open → in-progress
$ touch ../im/import.txt && git add ../im && git commit --quiet -m "feat(IM-1): import from a file"
$ tracker submit IM-1
Running criterion 1's check: `test -f import.txt`
IM-1: in-progress → review
Every gate passed, and IM-1 is held for review:
  T9  it has the label needs-review, which asks for a human's review
It waits in `tracker review` for a human to approve or reject it. `tracker show IM-1 --diff` shows its work.
$ tracker review
2 items are held for review, waiting for you to approve or reject each one:

BM-2  The empty state is friendly
  T6  criterion 1 is manual, so only a human can check its evidence
  See its work with `tracker show BM-2 --diff`, then run `tracker approve BM-2` or `tracker reject BM-2 --reason "…"`.

IM-1  Import bookmarks from a file
  T9  it has the label needs-review, which asks for a human's review
  See its work with `tracker show IM-1 --diff`, then run `tracker approve IM-1` or `tracker reject IM-1 --reason "…"`.
$ tracker review --json
{
  "held": [
    {
      "id": "BM-2",
      "project": "BM",
      "title": "The empty state is friendly",
      "holds": [
        {
          "trigger": "T6",
          "message": "criterion 1 is manual, so only a human can check its evidence"
        }
      ],
      "diff": "tracker show BM-2 --diff"
    },
    {
      "id": "IM-1",
      "project": "IM",
      "title": "Import bookmarks from a file",
      "holds": [
        {
          "trigger": "T9",
          "message": "it has the label needs-review, which asks for a human's review"
        }
      ],
      "diff": "tracker show IM-1 --diff"
    }
  ],
  "skipped": []
}
$ tracker show BM-2 --diff
BM-2  The empty state is friendly
  project   BM (bookmarks)
  kind      change
  status    review
  priority  medium
  created   2026-11-21T22:56:42Z
  updated   2026-11-21T22:56:42Z

Done when:
  1. The empty state says what to do next
       manual, evidence: Run the app with no bookmarks and read the message

Started:
  at    2026-11-21T22:56:42Z
  head  eb6677c9b8cd82ed7d3821a845a1b63d1a6353e1

Verification: none.

History, oldest first (2 entries):
  2026-11-21T22:56:42Z  start   open → in-progress    not from a terminal
  2026-11-21T22:56:42Z  submit  in-progress → review  not from a terminal

Held for review by:
  T6  criterion 1 is manual, so only a human can check its evidence

Commits, newest first (1):
  11f9b97  feat(BM-2): explain the empty state

Diff, oldest first:
commit 11f9b974b3e615680afdea76a21629ab44789311
feat(BM-2): explain the empty state

diff --git a/bm/empty.txt b/bm/empty.txt
new file mode 100644
index 0000000..0a0c3f3
--- /dev/null
+++ b/bm/empty.txt
@@ -0,0 +1 @@
+No bookmarks yet. Add one with the + button.
$ echo BM-2 | tracker approve BM-2      # exit 1: not a terminal
error: `tracker approve BM-2` needs a human at an interactive terminal, who types BM-2 to confirm it, and stdin here isn't a terminal. Nothing was changed. Only a human approves a held item. It waits in `tracker review` until one approves or rejects it.
$ tracker reject BM-2                   # exit 1: no reason
error: Rejecting BM-2 needs a reason, which is kept in its history: `tracker reject BM-2 --reason "…"`.
$ echo BM-2 | tracker reject BM-2 --reason "Say where the + button is."   # exit 1: not a terminal
error: `tracker reject BM-2` needs a human at an interactive terminal, who types BM-2 to confirm it, and stdin here isn't a terminal. Nothing was changed. Only a human rejects a held item. It waits in `tracker review` until one approves or rejects it.
$ tracker status BM-2 done              # exit 1: a held item is a human's to decide
error: BM-2 is in review, so `status` can't move it. A held item waits for a human, who either approves it with `tracker approve BM-2` (to done) or rejects it with `tracker reject BM-2 --reason "…"` (to in-progress), from an interactive terminal.
$ tracker approve BM-1                  # exit 1: BM-1 isn't in review
error: BM-1 is done, and `approve` only moves an item that is in review. From done, the only move is `tracker reopen BM-1 --reason "…"` (to open).
$ tracker create "Sort bookmarks by date" --done-when "Newest first by default"
Created BM-3: Sort bookmarks by date
1 manual criterion: give it a check with `tracker criterion set BM-3 1 --check "…"`.
$ tracker start BM-3                    # exit 2: G1, BM-2 is in review
error: BM-3 can't be started: G1 failed. Nothing was changed.
  G1  BM-2 is in review in BM, waiting for a human to approve or reject it, and a project works on one item at a time. BM-3 can start once BM-2 is decided.
$ mkdir $DEMO/plain
$ tracker project add PL --name plain --root $DEMO/plain
Registered PL (plain) at $DEMO/plain.
$ tracker create "Anything" --project PL
Created PL-1: Anything
$ tracker show PL-1 --diff              # exit 3: not in a git work tree
error: Can't show PL-1's commits: PL's root, $DEMO/plain, isn't inside a git work tree. Make it a repository with `git init`, or register PL with a root that's inside one. Nothing was changed.
$ tracker list --all
  ID    PRIORITY  KIND    STATUS  TITLE                         LABELS
BM  bookmarks
  BM-2  medium    change  review  The empty state is friendly
  BM-3  medium    change  open    Sort bookmarks by date
IM  importer
  IM-1  medium    change  review  Import bookmarks from a file  [needs-review]
PL  plain
  PL-1  medium    change  open    Anything

Every project.
Open, in-progress and review items. `--status done,shelved` for the others.
$ cat $TRACKER_HOME/items/BM-2.json
{
  "id": "BM-2",
  "title": "The empty state is friendly",
  "project": "BM",
  "kind": "change",
  "status": "review",
  "priority": "medium",
  "created": "2026-11-21T22:56:42Z",
  "updated": "2026-11-21T22:56:42Z",
  "written": [
    "The empty state says what to do next"
  ],
  "criteria": [
    {
      "text": "The empty state says what to do next",
      "evidence": "Run the app with no bookmarks and read the message"
    }
  ],
  "started": {
    "at": "2026-11-21T22:56:42Z",
    "head": "eb6677c9b8cd82ed7d3821a845a1b63d1a6353e1"
  },
  "holds": [
    {
      "trigger": "T6",
      "message": "criterion 1 is manual, so only a human can check its evidence"
    }
  ],
  "history": [
    {
      "at": "2026-11-21T22:56:42Z",
      "from": "open",
      "to": "in-progress",
      "via": "start",
      "interactive": false
    },
    {
      "at": "2026-11-21T22:56:42Z",
      "from": "in-progress",
      "to": "review",
      "via": "submit",
      "interactive": false
    }
  ]
}
```

Notes on the output:

- The SHAs and times differ on every run, since each commit records when it was made.
- On macOS, `project add` prints the root with `/private` in front of `$DEMO`, since it stores the path with symlinks followed.
- BM-2's held submit names no `Running` line, since its only criterion is manual and BM has no project checks. Its file has `holds`, just before `history`, and no `verification`, and `git show-ref` would list no `refs/tracker/BM-2`.
- `show --diff` lists only BM-2's commit. `feat(BM-12): sort by date`, made in between, isn't BM-2's, so neither its header nor `sort.txt` appears. The patch names `bm/empty.txt` by its path from the top of the repository, as `git show` prints it.
- The inbox's heading ends "waiting for you to approve or reject each one:", which reads the same for one item or many. `review` in [src/main.rs](../src/main.rs) uses one wording for any count, and the JSON doesn't have it. It used to end "…reject it:", which read wrongly for two items ([review 1's bug 1](#fixes-after-review-1), TR-27).
- `tracker reject BM-2` without `--reason` is refused before anything is asked, so it never reaches the terminal check.
- `tracker start BM-3` fails G1 only because BM-2 is held: a held item pauses its project until it's decided (§4.4). BM-3 has a manual criterion, so G1's "no criteria" failure doesn't appear.
- `show PL-1 --diff`'s exit 3 says tracker can't show PL-1's commits. It used to reuse `git::require_work_tree`'s message, which speaks of recording or proving the work, though `show` only reads it ([review 1's bug 2](#fixes-after-review-1), TR-27: `commits_and_patches` now asks `git::is_work_tree` and words the error itself).
- The demo doesn't run `gates`. Since TR-29, `tracker gates BM-2` there, once its gates pass, would say "A submit would be held for review:" and name T6, and its JSON would list T6 and T9 in `triggers` and T6 in `holds`.
- The demo can't show `approve` or `reject` going ahead, since both need a terminal. Typed at one, `approve BM-2` would print what holds BM-2, ask for the ID, re-run the gates, and print "BM-2: review → done" and "Approved: every gate passed again at <short SHA>, and refs/tracker/BM-2 keeps that commit." The tests drive both under `script` ([Testing](#testing)).
- History says `"interactive": false` because the demo ran from a script. Typed at a terminal, it's `true`.

## Commands

| Command | What it does |
| --- | --- |
| `tracker review` | New. Lists every item in review, from every project, sorted by project, then priority, then number, each with its holds and how to see its work and decide it. "Nothing needs you." when there are none. Skipped files are named, as `list` names them. |
| `tracker approve <id>` | New, interactive only. Refused with exit 1 unless the item is in review. Prints what holds it and what approving does, asks for the ID, then re-runs G2, G5 and G6 under the lock and G3 and G4 without it, then G5's second look. Exit 2 if one fails, leaving the item in review. Otherwise review → done, `by: "human"`, with `commit`, `ref`, `checks` and `triggers`. |
| `tracker reject <id> --reason "…"` | New, interactive only. Refused with exit 1 unless the item is in review, or without a reason, before anything is asked. Then asks for the ID, and review → in-progress, with the reason in history and `holds` dropped. |
| `tracker show <id> --diff` | New flag. Adds the item's commits and their patches. Exits 3 if the project's root can't be found or isn't in a git work tree, saying "Can't show XX-1's commits: …". It shows each commit's real lines, whatever textconv filter `.git/config` sets. Without `--diff`, `show` also prints `Held for review by:` for an item in review, and "approved, held by:" in a `human` record. |
| `tracker submit <id>`, `status <id> done` | After the gates pass, T6 and T9 run. If either fires, in-progress → review with `holds`, and no record or ref. Otherwise in-progress → done, `auto`, as in slice 8. |
| `tracker gates <id>` | Changed by TR-29. Once every gate passes, it also runs the triggers and prints "A submit would be held for review:" with each hold, if any, and its JSON has `triggers` and `holds`. When a gate fails, nothing runs, as before. It still changes nothing. |
| `tracker status <id> <state>` | Still never runs `approve` or `reject`. For an item in review it refuses every state, naming both. |
| every move, refused | For an item in review, names `approve` and `reject` and lists both in `allowed`. From in-progress, `allowed` lists `submit` twice, to done and to review. |

The help text for `submit` says what the triggers do, `gates`' says it lists any hold once the gates pass, and `review`, `approve`, `reject` and `--diff` have their own.

`reject`'s reason follows §3's rule for a reason: one line of at most 500 characters, checked by the same `check_reason` that `shelve` and `reopen` use. `approve` and `reject` share `tty::confirm` with `delete` and `criterion accept` ([§2.7](DESIGN.md#27-human-actions)): the ID can be typed in either case, and anything else cancels.

Exit codes in this slice, as §3 defines them:

- `0`: as before, and a held submit, whose output says the item is in review. `review` always exits 0, even with skipped files.
- `1`: as before, plus `approve` or `reject` not from a terminal, cancelled, on an item that isn't in review, or on an item that changed while the question waited or the checks ran, and `reject` without a reason.
- `2`: as before, plus an `approve` whose gates fail when they run again.
- `3`: as before, plus `show --diff` when the root can't be found, isn't in a git work tree, or `git show` fails, and `approve` when G5 can't read git.

## How it works

### The triggers

[src/triggers.rs](../src/triggers.rs) is new. `run(item)` returns one `Ran` per trigger, T6 then T9, each with `hold`: the message if it fired, or `None`. `holds(ran)` keeps the ones that fired as `item::Hold`s.

- `t6` numbers the criteria from 1 and collects those for which `Criterion::is_manual` is true. One gives "criterion N is manual, so only a human can check its evidence", and several "criteria 1 and 3 are manual, so only a human can check their evidence", joined with `states::join`.
- `t9` looks for the label `NEEDS_REVIEW`, `needs-review`, exactly. Labels are lowercased when they're stored (slice 3), so `Needs-Review` typed at `create` matches.

Neither reads git, the config or the item's history, so they can't fail. Every trigger runs on every submit, so `triggers` in the JSON lists both, fired or not.

### A submit that's held

`apply` in [src/main.rs](../src/main.rs) now takes the command, `via`, rather than a move, since `submit` from in-progress has two. `to` starts as `states::only_move(via, from)`'s state, and for `submit`:

1. G2, G5 and G6 under the lock, G3 and G4 without it, G5's second look, and the re-read with `Store::lock_unchanged_after_checks`, as in slice 8. A failure refuses with exit 2.
2. `triggers::run(&item)`. With no holds, `set_ref_at_head` creates `refs/tracker/<ID>` at the `HEAD` G5 saw, still under the lock, and `to` stays done. With any, `to` becomes review and no ref is made.
3. For done, `verification_record(VerifiedBy::Auto, …)` writes the record and "Verified automatically: …". For review, the text says the item is held, lists each hold with `holds_text`, and points to `tracker review` and `tracker show <ID> --diff`.
4. `item.holds` is set to the holds either way, so it's empty, and left out of the file, for a done item.
5. `push_move` adds the history entry with the item's new state as `to`.

The JSON's `result` is `held` or `done`, `triggers` lists each trigger with `fired`, and `holds` the ones that fired.

### Approve

`approve` in [src/main.rs](../src/main.rs):

1. Reads the item and its file's text without the lock, refuses a newer tracker's file with exit 3 (`refuse_newer_file`), and refuses with exit 1 if the diagram draws no `approve` from its state, through `states::not_from`, which lists the allowed moves.
2. `tty::confirm` with `decision_warning`'s text: the item, each hold, "Approving it runs the gates again, and if they pass, moves it to done, verified by a human, with refs/tracker/<ID> keeping the commit they ran against.", and the `show --diff` command.
3. `Store::lock_unchanged`, which refuses with exit 1 if the item changed while I read and decided.
4. `gates::statics` under the lock, then the lock goes, `gates::scripts` and `gates::tree_after_checks` run, and `Store::lock_unchanged_after_checks` takes it again, refusing with exit 1 if a check changed the item. This is submit's order, so `approve` gets G5's checks of the skip-worktree and assume-unchanged flags (slice 8's TR-21 and TR-25) through `gates::g5`, and, like submit, its second look doesn't ask about the flags (§7).
5. If a gate failed, `Report::refusal` with `Run::Approve`: "XX-1 can't be approved: G5 failed when the gates ran again, so it stays in review. Nothing was changed.", exit 2.
6. `set_ref_at_head`, then `verification_record(VerifiedBy::Human, …)`, whose `commit` is the `HEAD` this run's G5 saw, not the submit's, and whose `triggers` are the item's `holds`. `holds` is emptied, and the history entry is `via: "approve"`, review → done.

The triggers don't run again: what I approve is the holds I was shown. The gates run on the item as it was read before the question, which step 3 confirms is still the file.

### Reject

`reject` reads the item, refuses a newer file and an item that isn't in review, as `approve` does, then checks the reason with `check_reason` before asking anything. After `tty::confirm` and `Store::lock_unchanged`, it moves the item to in-progress, empties `holds`, and adds the history entry with `via: "reject"` and the reason. Nothing else changes: `started` stays as it was, so the agent carries on from the same start, and the next submit works out the triggers again (§2.5).

### The gates dry run

`gates_dry_run` in [src/main.rs](../src/main.rs) runs G2, G5, G6, G3 and G4 as before, and exits 2 listing every failure if one fails. When all pass, it calls `triggers::run` on the item as read, then `triggers::holds`, the same two functions `apply` uses. The text then has "A submit would be held for review:" and each hold through `holds_text`, as a held submit's does. The JSON has `triggers`, each with `fired`, and `holds`, after `gates`. Nothing is written: the item's own `holds` change only on a submit, and the triggers read only the item, so the answer is what a submit would get, apart from changes made in between. [schemas/gates.schema.json](../schemas/gates.schema.json) requires both fields.

### The move table

[src/states.rs](../src/states.rs) `MOVES` has 9 rows, one per arrow in §2.2's diagram, in its order: start, submit to done, submit to review, approve, reject, then the shelves and reopens. `find` is gone:

- `has_move(via, from)` says whether the diagram draws any move for a command from a state. `move_item`, `status`, `approve`, `reject` and `delete`'s shelve advice use it.
- `only_move(via, from)` returns the move when there's exactly one, which is every case but `submit` from in-progress, where it returns `None` and `apply` leaves the choice to the triggers.
- `allowed_json` and `allowed` list every row from the item's state, so in-progress gives `submit` twice with one command. For review, `allowed` says "A held item waits for a human, who either approves it with … or rejects it with …, from an interactive terminal."
- `not_from` names the states a command moves from once each, since `submit` now has two rows from in-progress.
- `via_to` still maps no state to `approve` or `reject`, and `status` refuses an item in review before looking, so `status` can't decide a held item.

`item::Via` gains `Approve` and `Reject`, and `needs_reason` is true for `Reject`. A history entry with an unknown `via` is now illustrated by `archive` rather than `approve` in comments and tests.

### Show --diff

`show` calls `commits_and_patches` when `--diff` is given:

1. `gates::root` finds the root, or exits 3 with "Can't show XX-1's commits: …". `git::is_work_tree`, which `require_work_tree` now calls too, says whether the root is in a work tree, and if it isn't, `commits_and_patches` exits 3 with the same "Can't show XX-1's commits: …" and its own wording, not `require_work_tree`'s, which speaks of recording or proving the work.
2. [src/git.rs](../src/git.rs) `item_log`, which `item_commits` now calls, returns each item commit's SHA with its header, newest first, by the same `is_item_header` match as G5.
3. `git::patches` runs one `git show --no-color --no-ext-diff --patch --format='commit %H%n%s'` with the SHAs oldest first, with `--no-ext-diff` and `--no-textconv`, and with `log.showSignature` and `core.quotePath` off, as the `git` helper sets them. A `diff.<driver>.textconv` command set in `.git/config` can rewrite a file's lines before `git show` prints them, and no commit shows that, so `--no-textconv` makes the diff what the commits hold. It reads the output with `from_utf8_lossy`, so a patch with bytes that aren't UTF-8 shows `�` rather than exiting 3. Every file a commit touched is shown, inside the root or not, as §2.1 says for changed files.
4. The text lists the commits, then "Diff, oldest first:" and the patches through `text::for_terminal` with line breaks kept, so a control character in a file can't change the terminal. The JSON has `commits` and `diff` beside `item`.

An item with no commits prints "Commits: none. No commit reachable from HEAD is headed `<type>(XX-1): …`.", and the JSON has `commits: []` and `diff: ""`.

### The inbox

`review` checks that the store exists and that `config.json` parses (§3), loads every item with `Store::load_items`, keeps those in review, and sorts them by project, priority and number. An item in review with no `holds`, which only a hand edit makes, gets "No trigger is recorded: it was put in review by hand." Skipped files go through `skipped_note`, `skipped_json` and `skipped_warnings`, as [slice 5's readme](slice-5-readme.md#limits-and-what-later-slices-change) asked.

## JSON output

| Command | Schema | What changed |
| --- | --- | --- |
| `submit`, `status <id> done` | [submit.schema.json](../schemas/submit.schema.json) | `result` is `held` or `done`, `triggers` lists T6 and T9 with `fired`, and `holds` the ones that fired. An `if`/`then`/`else` ties them: `held` needs `status: "review"` and at least one hold, and `done` needs `status: "done"` and none |
| `gates` | [gates.schema.json](../schemas/gates.schema.json) | When every gate passed, `triggers`, each `{trigger, fired}`, and `holds`, each `{trigger, message}`, are required, in that order after `gates`. When a gate fails, the output is the error shape, with no triggers |
| `review` | [review.schema.json](../schemas/review.schema.json) | New: `held`, each with `id`, `project`, `title`, `holds` and `diff`, the command that shows its work, and `skipped`, as `list` has it |
| `approve` | [approve.schema.json](../schemas/approve.schema.json) | New: `id`, `result: "approved"`, `status: "done"`, `gates`, each `ok: true`, and `item` |
| `reject` | [move.schema.json](../schemas/move.schema.json) | `{"item": …}`, as other moves print |
| `show --diff` | [show.schema.json](../schemas/show.schema.json) | `commits`, each `{sha, header}`, newest first, and `diff`, a string. `dependentRequired` makes each need the other |
| every refusal | [error.schema.json](../schemas/error.schema.json) | `allowed[].via` can be `approve` or `reject`. A refused `approve` that ran its gates is the gate shape, with `result: "refused"` and `status: "review"` |
| every item | [item.schema.json](../schemas/item.schema.json) | `holds`, at least one `{trigger, message}` when present, and `verification.triggers` of the same shape. A new `hold` definition, whose `trigger` matches `^T[0-9]+$`. History's `via` adds `approve` and `reject`, and `reason`'s description names `reject` |

[tests/common/mod.rs](../tests/common/mod.rs) `schema_for` maps `reject` to `move`, and `approve` and `review` to their own schemas, so `Env::json` checks every output. Slice 10 adds T1, T2, T3, T4, T7 and T8 to `triggers` and `holds`, slices 11 and 12 add T5, T10, T13 and T14, and slice 12 adds `verification.judges`. None of these needs a field here to change, since `trigger`'s pattern already allows them.

## Code tour

| File | What's new or changed |
| --- | --- |
| [src/triggers.rs](../src/triggers.rs) | New (`3b79ea1`). `NEEDS_REVIEW`, `Ran`, `run`, `holds`, `t6` and `t9` |
| [src/main.rs](../src/main.rs) | `Command::Review`, `Approve`, `Reject` and `Show`'s `diff`, with help text, and `submit`'s help. `apply` takes `via`, runs `triggers::run`, and calls the new `set_ref_at_head`, `verification_record`, `holds_text` and `push_move`. `move_item` and `status` use `states::has_move`. `check_reason` says "Rejecting". `show` takes `diff`, prints `Held for review by:`, and calls the new `commits_and_patches`. `verification_text` prints "approved, held by:". New `review`, `diff_command`, `approve`, `reject` and `decision_warning`. `create` writes empty `holds`. `delete` uses `has_move`. After review 1: `review`'s heading ends "each one:" (`eafbb1d`), `commits_and_patches` words its own error for a root outside a work tree (`eafbb1d`), and `gates_dry_run` runs `triggers::run` and adds `triggers` and `holds` once every gate passes, with `Gates`' help text (`f4fd771`) |
| [src/states.rs](../src/states.rs) | `MOVES` with 9 rows, `has_move` and `only_move` in place of `find`, `allowed`'s sentence for review, `not_from`'s de-duplicated states, and `via_to`'s comment |
| [src/item.rs](../src/item.rs) | `Item::holds`, `Verification::triggers`, `Hold`, `Via::Approve` and `Via::Reject`, and `needs_reason` for `Reject` |
| [src/git.rs](../src/git.rs) | `item_log`, which `item_commits` now wraps, and `patches`. After review 1: `is_work_tree`, split out of `require_work_tree` (`eafbb1d`), and `--no-textconv` in `patches` (`0311da0`) |
| [src/gates.rs](../src/gates.rs) | `Run::Approve` and its refusal message |
| [src/tty.rs](../src/tty.rs), [src/lib.rs](../src/lib.rs) | `confirm`'s comment names `approve` and `reject`. `pub mod triggers` |
| [schemas/](../schemas/) | `approve` and `review`, new, and `submit`, `show`, `item`, `error` and `move`, as [above](#json-output), all in `ed83b46`. `gates`: `triggers` and `holds` required (`f4fd771`) |
| [tests/review.rs](../tests/review.rs) | New (`ed83b46`): one module per TR-5 criterion, `criterion_1` to `criterion_8`, with 15 tests, and 3 more outside them. Helpers `env_with_project`, `start_manual`, `start_asked_for`, `held_xx_1`, `fired`, `has_ref`, `check_held`, `allowed`, `step`, `piped` and `text`. Review 1's fixes add 10 tests in four modules: `tr_27` (2 tests, `a63c850`), `tr_28` (1, `4d419ea`), `tr_29` (2, `2a84ff8`) and `tr_30` (5, `58e0afb`), with the helper `json_in_terminal` in `tr_30` |
| [tests/gates.rs](../tests/gates.rs), [tests/schemas.rs](../tests/schemas.rs) | Two older tests that pinned the dry run's shape gained `triggers` and `holds`, and `schemas.rs`'s gates sample gained the two fields and bad cases for them (`f4fd771`) |
| [tests/common/mod.rs](../tests/common/mod.rs) | `schema_for`'s three new commands (`ed83b46`) |
| [tests/states.rs](../tests/states.rs), [tests/item_files.rs](../tests/item_files.rs) | Expected values changed before `start` (`ed83b46`), [below](#older-tests-that-changed) |
| [tests/checks.rs](../tests/checks.rs), [tests/criteria.rs](../tests/criteria.rs), [tests/gates.rs](../tests/gates.rs), [tests/show_and_delete.rs](../tests/show_and_delete.rs) | One test each changed after `start` (`c660f3b`), [below](#older-tests-that-changed) |
| [tests/fixtures.rs](../tests/fixtures.rs), `tests/fixtures/slice-9/` | XX-20, XX-21 and `slice_9s_held_and_approved_fixtures_come_back_byte_for_byte` (`0ddc010`) |
| [docs/DESIGN.md](DESIGN.md) | `fae0005`: §2.4 says that once every gate passes, `gates` runs the triggers and lists any hold a submit would get, and §4.3's example `auto` record loses `"triggers": []`, which an `auto` record never has. `10e519a`: §2.7's `approve` bullet says a held submit records no commit and creates no ref, so the record's `commit` is the `HEAD` approve's own G5 saw. §3's `show` row says the diff is each commit's patch, oldest first, as `git show` prints it, and why. §4.3 has a bullet for `holds` and `verification.triggers` |
| [docs/PLAN.md](PLAN.md) | Slice 9's status, `todo` to `in progress` (`d7b9e20`) to `in review` (`4c19e71`) |

To add a trigger, as slices 10 to 15 do: write a function in [src/triggers.rs](../src/triggers.rs) that returns the hold's message or `None`, and add a `Ran` for it to `run`, in trigger order, so `triggers` lists it. A trigger that reads git or the config needs `run` to take them, and `apply` calls `run` after the gates, under the lock. `approve` doesn't run the triggers, so a new one changes nothing there. To add another human-only command, follow `reject`: refuse what it can before `tty::confirm`, then `Store::lock_unchanged` before writing.

## Testing

Run the test command in `tracker/`. TR-5's criteria each run their own module in `tests/review.rs`, for example `cargo test --test review criterion_4::`. The module path ends in `::`, so `criterion_1::` can't also match a `criterion_10`.

`approve` and `reject` need a terminal, so their tests run them under `script` with [tests/common/mod.rs](../tests/common/mod.rs)'s `Env::in_terminal_typing`, as slice 4's `delete` tests do. It returns `None` where `script` isn't installed, and each such test then returns early, passing. That's 10 tests in `tests/review.rs`, 5 from `ed83b46` and all 5 of review 1's `tr_30`, and three of the four changed by `c660f3b`. Every project in `tests/review.rs` is a subfolder of a throwaway repository, as `Env::new` makes it since slice 8.

The harness's only change is `schema_for` ([JSON output](#json-output)).

### Coverage of the _Done when_ checks

| _Done when_ | Tests in `tests/review.rs` |
| --- | --- |
| A manual criterion with evidence is held under T6. A `needs-review` item is held under T9. An item with neither still auto-verifies. | `criterion_1::a_manual_criterion_with_evidence_is_held_under_t6`: `result: "held"`, only T6 fired, one hold naming criterion 1, the file's `holds` equal to the output's, and `check_held`: in review, no record, the last entry a submit to review, no ref. `criterion_1::a_needs_review_item_is_held_under_t9`: the same for T9, naming `needs-review`. `criterion_1::an_item_with_neither_still_auto_verifies`: `done`, both triggers listed with `fired: false`, no `holds`, `auto`, and the ref |
| `submit --json` matches the held shape in §3. | `criterion_2::submit_json_matches_the_held_shape_in_section_3`: for `submit` and `status XX-1 done`, an item held by both triggers prints exactly `id`, `result: "held"`, `status: "review"`, the five gates, T6 and T9 fired, `holds` and `item`, in that order, each hold with only `trigger` and `message` |
| `review` lists held items with their reasons and the `show --diff` hint, and prints "Nothing needs you." when it's empty. | `criterion_3::the_inbox_lists_held_items_with_their_reasons_and_the_diff_hint`: XX-1 and YY-1 held in two projects, an open XX-2 left out, the same JSON outside and inside XX's root, and the text naming each ID, its `tracker show <ID> --diff`, and each hold's trigger and message. `criterion_3::the_inbox_says_nothing_needs_you_when_its_empty`: an empty store, then one with an open and a done item |
| `echo XX-1 \| tracker approve XX-1` is refused. Typing the ID in a real terminal moves the item to done (checked by hand). | `criterion_4::approve_from_piped_stdin_is_refused`: exit 1, "interactive terminal", the error schema, with and without `--json`, the file byte for byte unchanged, no ref. `criterion_4::approve_typed_in_a_terminal_moves_the_item_to_done`, under `script`: after a commit made while it waits, done, `human`, `commit` and the ref at that new `HEAD`, `triggers` equal to the submit's `holds`, an `approve` entry from a terminal, and the item schema. `criterion_4::approve_typed_with_another_id_changes_nothing`, under `script`. The real terminal hasn't been tried ([Checks done by hand](#checks-done-by-hand)) |
| An approve whose gate re-run fails leaves the item in review. A reject puts its reason in history. | `criterion_5::an_approve_whose_gate_rerun_fails_leaves_the_item_in_review`, under `script`: an untracked `xx/stray.txt` fails G5, "stays in review", the file unchanged, no ref. `criterion_5::a_reject_puts_its_reason_in_history`, under `script`: in-progress, no `holds`, no record, the last entry exactly review → in-progress, `reject`, from a terminal, with the reason, and a resubmit held again by the label. `criterion_5::reject_from_piped_stdin_is_refused`: exit 1 for "interactive terminal", and for a missing `--reason`, the file unchanged |
| Every move refused for an item in review, through a command or through `status`, names `approve` and `reject`, and its `allowed` lists both. Until now the message says this version can't do either, and `allowed` is empty. | `criterion_6::every_move_refused_for_an_item_in_review_names_approve_and_reject`: `start`, `submit`, `shelve`, `reopen`, and `status` to each of the five states, run outside every root. Each exits 1, names `tracker approve XX-1` and `tracker reject XX-1`, has `allowed` of exactly approve to done and reject to in-progress, and leaves the file unchanged. The second sentence describes slice 8: `tests/states.rs` `an_item_in_review_is_refused_by_every_move` asserted the empty `allowed` until `ed83b46` |
| `show XX-1 --diff` includes XX-1's commits and none of XX-12's. | `criterion_7::show_diff_includes_xx_1s_commits_and_none_of_xx_12s`: XX-1's test commit, a `feat(XX-1)` commit, a `feat(XX-12)` commit, and a `fix(XX-1)!` commit touching `other/outside.txt`, outside the root. `commits` is the three XX-1 commits, newest first, the diff has `xx/one.txt`, `+the first one` and `other/outside.txt` and nothing of `twelve` or `XX-12`, the text likewise, and plain `show` has neither field |
| A refused move from in-progress lists `submit` twice in `allowed`, once to done and once to review, with the same command. | `criterion_8::a_refused_move_from_in_progress_lists_submit_to_done_and_to_review`: `start`, `status in-progress`, `reopen`, `status review`, `approve` and `reject` on an in-progress item each list submit to done, submit to review and shelve, in that order |

The prompt's other requirements are covered too. `status` refusing an item in review is `criterion_6`'s `status` cases. `states::find` answering whether a command has any move is `has_move`, which every refusal above goes through. `allowed` keeping one entry per arrow is `criterion_6` and `criterion_8`.

The first 18 tests in `tests/review.rs`, which are those of the coverage table and the other tests below, were committed in `ed83b46`, 28 seconds before `tracker start TR-5`, which recorded all eight criteria as failing ([Tracking TR-5](#tracking-tr-5)). For this readme, `ed83b46`'s files were run again in a copy outside the repo, with the code before the slice: all 18 fail, `criterion_1::an_item_with_neither_still_auto_verifies` among them, since slice 8's `submit` lists no triggers.

### Other tests

From `ed83b46`, outside the criteria's modules:

- `approve_on_an_item_that_isnt_in_review_is_refused_with_its_allowed_moves`: an open item, exit 1, `allowed` of start and shelve.
- `approve_writes_nothing_when_a_check_edits_the_item_file`, under `script`: a criterion check that appends a space to the item file once a marker exists outside the repository. The approve exits 1, "changed while its checks ran", and the item stays in review with no record and no ref. This is PLAN.md's integration risk, "Re-reading after the lock", for `approve`.
- `a_held_item_pauses_its_project_until_its_decided`: `start XX-2` while XX-1 is held exits 2, with G1's "XX-1 is in review".

Each failed in the run against the code before the slice.

Elsewhere:

- `tests/fixtures/slice-9/` holds two files this branch's binary wrote. XX-20 is held by T6 and T9, with `holds` and no `verification`. XX-21 was held by T6, rejected with the reason "The empty state is missing.", resubmitted and approved from a terminal, so its record is `human`, with `by`, `at`, `commit`, `ref`, `checks` and `triggers`, in that order, and its history is start, submit, reject, submit, approve. `slice_9s_held_and_approved_fixtures_come_back_byte_for_byte` checks that the folder holds only these, those values, that `holds` comes just before `history`, that each comes back byte for byte with nothing lost on rewrite, and that each matches the item schema. The existing fixture tests load and rewrite them too.

What has no test: `review`'s text for an item in review with no `holds`, and `show --diff`'s text for an item with no commits. `show --diff`'s exit 3, and `reject` cancelled by a wrong ID or changed while the question waits, had none until review 1's fixes ([Test gaps](#test-gaps)), which add `tr_27`'s and `tr_30`'s.

### Older tests that changed

`ed83b46`, before `tracker start TR-5`, changed the expected values of tests that the slice makes untrue. Nothing they check was removed:

| Test | Was | Now |
| --- | --- | --- |
| `tests/states.rs`: `submit_prints_the_shape_section_3_gives_it` | `"triggers": []` | T6 and T9, each `fired: false` |
| `tests/states.rs`: `every_move_the_diagram_doesnt_draw_is_refused`, `nothing_moves_an_item_to_review_by_hand` | `allowed` from in-progress: submit, shelve | submit, submit, shelve |
| `tests/states.rs`: `an_item_in_review_is_refused_by_every_move` | `allowed` empty | approve and reject |
| `tests/item_files.rs`: `lost_on_rewrite_tells_unknown_fields_from_values_written_differently` | `verification.triggers` as the unknown field, and `via: "approve"` as the unknown command | `verification.judges` and `via: "archive"`, since this version knows the other two |

The four `tests/states.rs` tests fail against the code before the slice, as the run of `ed83b46`'s files shows. The `tests/item_files.rs` change passes either way.

`c660f3b`, after the start, changed four tests whose items have a manual criterion. Since `3b79ea1`, T6 holds their submit, so they no longer reach done on their own:

| Test | Was | Now |
| --- | --- | --- |
| `tests/checks.rs`: `verification_checks_name_the_criterion_each_came_from` | reads the record from the submit's output | asserts the submit is held, approves it under `script`, and reads the record from the file. The same three entries, criteria 1, 3 and none, are asserted |
| `tests/criteria.rs`: `criteria_can_change_before_and_after_start_but_not_once_done` | a submit reaches done | asserts the submit is held, then approves it under `script`, so the done item's refusals are still tested |
| `tests/gates.rs`: `a_manual_criterion_with_no_evidence_fails_g2` | with evidence, the submit's status is `done` | `review` |
| `tests/show_and_delete.rs`: `show_prints_every_field_the_verification_records_and_history_with_times` | two submits to done, `auto`, 6 history entries | each submit held and approved under `script`: `human`, "T6  criterion 1 is manual", 8 entries, "in-progress → review", "review → done" and "from a terminal" |

`c660f3b` was committed in the same second as the code, 22:43:04Z, so these weren't seen failing on their own commit. For this readme, `3b79ea1`'s files, which have the old versions, were run in a copy outside the repo: all four fail, at `tests/checks.rs:537`, `tests/criteria.rs:435`, `tests/gates.rs:384` (`"review"` where `"done"` was expected) and in `tests/common/mod.rs:90`, `Env::json_in`'s check for exit 0, which `show_prints_every_field…` reaches when it runs `reopen` on the item its submit left in review. No other test failed there, apart from `docs_links`' `line_anchors_name_lines_that_exist`, which needs the repository's history and fails in any copy outside it.

DESIGN.md §5 says not to edit an existing test just to pass, but to fix a test that's wrong and say so. These are that case: a manual criterion auto-verifying was the behaviour slice 9 removes. Two consequences:

- Three of them, `verification_checks_name_the_criterion_each_came_from`, `criteria_can_change_before_and_after_start_but_not_once_done` and `show_prints_every_field…`, now go through `Env::in_terminal_typing`, so on a machine without `script` they return early and check nothing after the submit.
- Slice 10's T4 would hold TR-5 for `c660f3b`, since it changes lines in existing test files after start.

### Checks done by hand

Each was run with this branch's `target/debug/tracker` in a temporary `TRACKER_HOME`:

| What | How | Result |
| --- | --- | --- |
| The demo above | Run as written, with stderr captured | The output above, with the temporary folder replaced by `$DEMO`. Each command marked with an exit code exited with it, and every other one exited 0 |
| TR-5's tests against the code before the slice | `git archive ed83b46`, then `cargo test --test review` in the copy | All 18 fail |
| The four tests `c660f3b` changed, in their old form | `git archive 3b79ea1`, then each test, and the whole suite, in the copy | All four fail, and nothing else but `line_anchors_name_lines_that_exist`, as [above](#older-tests-that-changed) |
| Typing the ID into `approve` and `reject` in a real terminal | Not done | PLAN.md's _Done when_ and its integration risk, "Interactive-only commands are only half tested", leave this to the user, by hand. The tests type the ID under `script`, which DESIGN.md §7 says looks the same to `tracker` as a real terminal, but that's not the check PLAN.md asks for |

### Tracking TR-5

TR-5, a `capability` with `design` `docs/DESIGN.md#2.7`, was created at 2026-11-21T07:07:43Z with slice 7's other items. Its eight criteria are PLAN.md's _Done when_ bullets, with the sixth's second sentence, which describes slice 8, left out. Each check runs one module of `tests/review.rs`, such as `cargo test --test review criterion_1::`, and the saved output shows each matching 3, 1, 2, 3, 3, 1, 1 and 1 tests. The slice 8 binary was installed before TR-5 started and tracked it throughout:

| Step | When (UTC) | What |
| --- | --- | --- |
| Tests and schemas committed | 22:31:45Z | `ed83b46` |
| `tracker start TR-5` | 22:32:13Z | `started.head` `ed83b46`. `started.checks`: all eight criteria failed |
| PLAN.md: in progress | 22:32:17Z | `d7b9e20` |
| Code and the four older tests | 22:43:04Z | `3b79ea1`, `c660f3b` |
| Fixtures, DESIGN.md | 22:44:24Z, 22:44:35Z | `0ddc010`, `10e519a` |
| `tracker submit TR-5` | 22:48:48Z | done, `auto`, `commit` `10e519a`, `refs/tracker/TR-5` |
| PLAN.md: in review | 22:48:52Z | `4c19e71` |

So:

- **TR-5 is the first TR item with a commit and a ref.** The installed binary ran G2, G5, G6, G3 and G4. G5 found the tree under `tracker/` clean and TR-5's commits, and the record's `commit` is `10e519a`, which `refs/tracker/TR-5` in the guild-portfolio repository points at. G3 ran its 8 checks and G4 TR's `cargo fmt --check`, `cargo clippy -- -D warnings` and `cargo test`, the last taking 59 seconds. Each criterion check's saved output names the tests it ran, such as `test criterion_1::a_manual_criterion_with_evidence_is_held_under_t6 ... ok`.
- **Nothing could have held it.** The slice 8 binary has no triggers. None of TR-5's criteria is manual, since each has a check, criterion 4 included: its check runs the `script` tests, and its "checked by hand" half isn't something a check proves, so it's [left open](#still-open) rather than recorded as evidence. With this slice's binary, TR-5 would still auto-verify, since it has no manual criterion and no `needs-review` label. Slice 10's T1 would hold it, as a `capability`, and T4 for `c660f3b`.
- **Every check failed at start**, as `started.checks` records, and all 18 tests fail against the code before the slice ([above](#coverage-of-the-done-when-checks)).
- **Every commit in the range is scoped to TR-5.** `d7b9e20` came 4 seconds after the start. `4c19e71` came 4 seconds after the submit, so it's one of a done item's commits, after its verified commit: slice 10 counts its file among TR-5's changed files if TR-5 is ever reopened. Slice 11's T14 counts only unscoped commits, and there are none since `started.head`.
- TR-5's file, written by the slice 8 binary, loads in this branch's binary unchanged: it has no `holds` and no `triggers`.

### Tracking review 1's fixes

Review 1 was committed in `236bc39` at 2026-11-21T23:12:05Z, and its fixes were created in the same second, as TR-27 to TR-30, each naming the review and its findings in its description. Each was tracked with the installed slice 8 binary, one at a time, as [PLAN.md](PLAN.md)'s integration risks ("The bootstrap") and [CLAUDE.md](../CLAUDE.md) ask, not with this slice's:

| Item | Kind | For | Tests committed | Started | Fix | Submitted |
| --- | --- | --- | --- | --- | --- | --- |
| TR-27 | `fix` | bugs 1 and 2 | `a63c850`, 23:13:21Z | 23:13:23Z | `eafbb1d`, 23:15:28Z | 23:17:38Z |
| TR-28 | `fix` | security 1 | `4d419ea`, 23:17:56Z | 23:17:57Z | `0311da0`, 23:28:38Z | 23:35:42Z |
| TR-29 | `capability` | design 2 and 3 | `2a84ff8`, 23:38:28Z | 23:38:30Z | `f4fd771` and `fae0005`, both at 23:45:07Z | 23:47:33Z |
| TR-30 | `change` | test gaps 1 to 3 | `58e0afb`, 23:49:48Z | 23:49:52Z | none: the code already worked | 23:52:18Z |

So:

- **Each reached done `auto`**, verified by the installed slice 8 binary, which ran G2, G5, G6, G3 and G4. Each record has a `commit`, the last of the item's commits, and `refs/tracker/<ID>` points at it: `eafbb1d`, `0311da0`, `fae0005` and `58e0afb`. G4's `cargo test` took 62 to 68 seconds. After the last, all 337 tests pass, 10 more than at `657559b`: 2, 1, 2 and 5 in `tests/review.rs`.
- **Nothing could have held them.** That binary has no triggers, and every criterion of the four has a check, so none is manual. None has the label `needs-review`.
- **TR-27's, TR-28's and TR-29's checks all failed at start**, as `started.checks` records, and each test was seen failing on its own before the fix, as [Fixes after review 1](#fixes-after-review-1) says.
- **TR-30's three checks passed at start**, as they had to: its tests cover code that already worked, so the start warned, and slice 11's T13 would hold it. Each test was seen failing against code broken on purpose, then restored. Its one commit, `58e0afb`, is both `started.head` and the verified `commit`.
- **Each filter matches what it names**: `tr_30::reject_` matches two tests and `_approve_json_` two, as the saved output shows, and the other test filters one each. TR-29's criterion 3 is a `grep` on DESIGN.md, not a test, and it failed at start because §4.3's example still had the line.
- **`58e0afb` also picked up two `.claude/agents/` files by mistake.**
- **TR-29's feat commit, `f4fd771`, changed two existing tests**, `tests/gates.rs`'s `a_script_criterion_needs_no_evidence` and the sample in `tests/schemas.rs`, after the start, because the dry run's shape changed. Both pinned that shape exactly. Slice 10's T4 would hold TR-29 for them.
- **Two docs commits belong to no item**, `e2d3c3d` and `f4ffaa9`. They came after TR-30's submit, while no item was in progress.
- Every commit from `a63c850` to `58e0afb` is scoped to one of the four. `236bc39` came before TR-27's start, while no item was in progress.

## Decisions worth knowing

- **A held submit creates no ref and writes no record** (slice 8 review 1's [Slice 9 must decide](reviews/slice-8-review-1.md#slice-9-must-decide), first point). §2.7 creates the ref when an item reaches done, so `set_ref_at_head` runs only on the done branch of `submit`, and in `approve`. Slice 8's `a_second_verification_moves_the_ref_to_the_new_commit` and `delete_removes_a_ref_named_only_by_previous_verification` still pass with it moved.
- **`approve`'s record names the `HEAD` its own G5 saw** (the second point). A held submit has no commit to keep, and work committed while the item waits is what the gates ran against, so the ref goes there. `criterion_4::approve_typed_in_a_terminal_moves_the_item_to_done` commits while the item waits to prove it. DESIGN.md §2.7 says so since `10e519a`.
- **`approve` runs submit's gates in submit's order**, with the lock let go for the checks and the re-read after them, so it inherits G5's flag checks and the second look's limit (§7).
- **`approve` doesn't run the triggers again.** The holds I was shown are what I approve, and they go into `verification.triggers` as they were. A trigger that would fire now, such as a `needs-review` label added while the item waited, isn't recorded. §2.5's fresh triggers apply to the next submit, which follows a reject. [Review 1's design 1](#design-decisions) asks slice 10 to decide whether `approve` should run them, once they read commits.
- **`approve` reads the item once, before asking.** The gates run on that item, after `Store::lock_unchanged` confirms the file is the same, and the write after the checks confirms it again.
- **`reject` checks its reason before asking**, as [slice 4's readme](slice-4-readme.md#limits-and-what-later-slices-change) said it should, so a missing reason never reaches the terminal question.
- **`reject` keeps `started`.** The item goes back to in-progress, not to open, so its `started.head`, which slice 11's T14 reads, and `started.checks`, which T13 reads, stay those of the original start.
- **`holds` sits after `previous_verification` and just before `history`**, and `verification.triggers` between `checks` and `note`, each with a serde default and `skip_serializing_if`, so files written by earlier slices load and come back byte for byte. Both schemas require at least one hold when the field is present.
- **The two `submit` rows share a command**, and `only_move` returns `None` for them, so `apply` picks the outcome from the triggers and nothing else can. Every other command and state has exactly one row.
- **`status` never decides a held item.** `via_to` maps no state to `approve` or `reject`, and `status` refuses an item in review whatever state it's given, naming both commands.
- **T9 reads only the item's labels now**, not whether an edit since `start` removed `needs-review`. This settles [slice 3's review 2 finding 4](slice-3-readme.md#limits-and-what-later-slices-change) the plain way: an edit that removes the label switches T9 off, and slice 16's digest lists that edit, from `labels_removed`, marked by whether it came from a terminal (§2.6).
- **`show --diff` prints each commit's patch, oldest first**, with one `git show`, rather than one diff from the first commit to the last, which would include other items' work between them. DESIGN.md §3's `show` row says so since `10e519a`. Its commit list is newest first, as `item_commits` returns it.
- **`show --diff` reads git's output lossily**, unlike the `git` helper, which refuses output that isn't UTF-8 with exit 3. A diff is only for reading, so a file in another encoding shows `�` rather than stopping the review.
- **`review` sorts by project, then priority, then number**, as `list` does within a project, and lists every project from anywhere, as §3 says.
- **`approve.schema.json` is its own shape**, `result: "approved"` with `gates` and `item`, and no `triggers` or `holds`: the triggers don't run, and the approved holds are in `item.verification.triggers`.
- **The `gates` dry run runs the triggers once every gate passes** (review 1's design 2, option (a)). §3 says `gates` lists `triggers` and `holds`, and §2.4 calls it the agent's way to ask "what's left?", so an agent that stops and reports a hold needs to learn it before it submits. A failed gate runs no trigger, as on a submit, and nothing is written.
- **`show --diff` turns textconv off** (review 1's security 1), as it turns external diff drivers off. The diff is the screen read before an approve, and a `diff.<driver>.textconv` command in `.git/config` can hide lines that no commit shows.
- **TR-5 was tracked by the slice 8 binary**, as slice 8 was by slice 7's ([PLAN.md](PLAN.md)'s "The bootstrap"). This slice's binary is to be installed once the slice is finished, so TR-6 (slice 10) is the first TR item that T6 and T9 can hold.

## Fixes after review 1

[Review 1](reviews/slice-9-review-1.md) found two bugs, one security issue, three places where the code and DESIGN.md disagreed or the design was unclear, three test gaps and two process notes. Bugs 1 and 2, security 1, design 2 and 3 and test gaps 1 to 3 became tracker items, TR-27 to TR-30 ([Tracking review 1's fixes](#tracking-review-1s-fixes)), each with its tests committed before `tracker start` and seen failing, either on the old behaviour or, for TR-30, on code broken on purpose. Option (a) was taken for bug 2, as the review chose. Design 1 is left for slice 10, process 1 is still the user's, and process 2 needs no action. The review's [Outcome](reviews/slice-9-review-1.md#outcome) records the same decisions.

### Low severity

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Bug 1: the inbox's heading said "it" for any number of items | `eafbb1d` (TR-27) | `review`'s heading ends "…, waiting for you to approve or reject each one:", which reads for one item too | `tr_27::the_inbox_heading_doesnt_call_two_items_it`: XX-1 and YY-1 held, and the heading starts "2 items are held", doesn't end in "it" and has "approve or reject each one". Seen failing on the old wording |
| Bug 2: `show --diff` outside a git work tree said tracker can't "record or prove" the work, though `show` only reads it | `eafbb1d` (TR-27) | Option (a). `git::is_work_tree` is split out of `require_work_tree`, which calls it and keeps its message for `start` and `submit`. `commits_and_patches` asks it, and exits 3 with "Can't show XX-1's commits: …" | `tr_27::show_diff_outside_a_git_work_tree_says_it_cant_show_the_commits`: a project root in a folder outside the repository exits 3, names "Can't show XX-1's commits" and "isn't inside a git work tree", doesn't say "record or prove", and leaves the item file unchanged. Seen failing on the old wording |

### Security items

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Security 1: `show --diff` applied textconv filters, so the diff could hide lines the commit holds | `0311da0` (TR-28) | `--no-textconv` in `git::patches`, beside `--no-ext-diff` | `tr_28::show_diff_ignores_a_textconv_filter`: `*.sh diff=quiet` in `.gitattributes`, a `work.sh` with the line `rm -rf the_tests # hidden`, and `diff.quiet.textconv "grep -v hidden"` set in `.git/config`. The JSON `diff` and the text both have the hidden line. Seen failing before the fix: the hidden line was missing from the diff |

### Design decisions

| Question | Decision | Commits |
| --- | --- | --- |
| Design 1: `approve` doesn't run the triggers again, so commits made after a held submit are approved with only the submit's holds | Left for slice 10 (TR-6), as the review suggested. It only matters once slice 10's triggers read commits: T6 and T9 read only the item, which the re-read pins. The review's option (a), `approve` running `triggers::run` before asking, at the `HEAD` it will name, is the one it would pick | none |
| Design 2: the `gates` dry run listed no triggers or holds, though §3 says it does | Option (a). Once every gate passes, `gates_dry_run` runs `triggers::run` and adds `triggers` and `holds` to the JSON, and "A submit would be held for review:" with each hold to the text. `gates.schema.json` requires both fields, and §2.4 gets a sentence saying so | `f4fd771` (TR-29), `fae0005` (DESIGN.md) |
| Design 3: §4.3's example `auto` record had `"triggers": []`, which an `auto` record never has | The line is gone from the example | `fae0005` (TR-29) |

TR-29's tests: `tr_29::gates_lists_the_holds_a_submit_would_get` has a manual criterion with evidence, so T6 fired and T9 didn't, one hold names criterion 1, the keys come in the order `id`, `status`, `result`, `gates`, `triggers`, `holds`, the text names the hold, the item file is unchanged, and the `submit` after it gets the same `triggers` and `holds`. `tr_29::gates_says_no_holds_when_no_trigger_fires` has an empty `holds`, no "held for review" in the text and the file unchanged. Both were seen failing. Criterion 3's check is `! grep -qF '"triggers": []' docs/DESIGN.md`, which failed at start. Two older tests pinned the dry run's shape exactly, `tests/gates.rs`'s `a_script_criterion_needs_no_evidence` and the sample in `tests/schemas.rs`, so they gained the two fields in the feat commit, and the schemas sample gained bad cases: either field missing, a `trigger` named `G6`, and a hold with an empty message.

### Test gaps

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Test gap 1: nothing tested `approve`'s second look at the tree | `58e0afb` (TR-30) | One test. The code already worked | `tr_30::approve_fails_g5_for_a_file_its_checks_leave`, under `script`: a check that leaves `xx/left.txt` once a marker outside the repository exists, made after the held submit. `approve` exits 2 with one G5 failure naming the file, the item stays in review with no record, the file is unchanged and there's no ref. Seen failing with `tree_after_checks` removed from `approve` |
| Test gap 2: `reject`'s re-read under the lock, and a wrong ID typed into it, had no test | `58e0afb` (TR-30) | Two tests, of the three cases the review asked for. The code already worked | `tr_30::reject_writes_nothing_when_the_item_changes_while_deciding`, under `script`: a write while the warning is read makes it exit 1 with "changed while you were deciding", the file as that write left it, and the item still in review. Seen failing with `lock_unchanged` replaced by `store.lock()` in `reject`. `tr_30::reject_typed_with_another_id_cancels_and_changes_nothing`: exit 1, "Cancelled: \"XX-2\" isn't XX-1", the file unchanged. Seen failing with `reject` ignoring `tty::confirm`. The two display paths, `review`'s line for a held item with no `holds` and `show --diff`'s text for an item with no commits, stay untested ([Still open](#still-open)). `show --diff`'s exit 3 is now tested by `tr_27` |
| Test gap 3: no test checked a passing `approve --json`, or its exit 2 refusal, against a schema | `58e0afb` (TR-30) | Two tests, which run `approve` under `script` with stdout redirected to a file (`json_in_terminal`). The code already worked | `tr_30::a_successful_approve_json_matches_its_schema`: exit 0, the approve schema, `id`, `result: "approved"`, `status: "done"`, keys in the order `id`, `result`, `status`, `gates`, `item`, and `item` equal to the file. Seen failing with an extra `"by"` field in the JSON. `tr_30::an_approve_json_refused_by_a_gate_matches_the_refusal_shape`: a stray file fails G5, exit 2, the refusal's `id`, `status: "review"`, `result: "refused"`, `gates` and a G5 failure, the file unchanged. Seen failing with a bare error for approve's refusal |

TR-30's criterion 3 says the exit-2 output is checked "against the approve schema". It's checked against [error.schema.json](../schemas/error.schema.json), because `approve.schema.json` covers only success and points to the error shape for a refusal. The criterion's text is left as written.

The second mutant in review 1's test gap 1, `approve`'s first re-read replaced by a plain `store.lock()`, has no test. The review found it almost harmless, since the second re-read compares the same bytes, and TR-30 doesn't cover it.

### Process

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Process 1: typing the ID into `approve` and `reject` in a real terminal hasn't been tried | none | Still the user's to do, before slice 10 starts, with this branch's binary in a throwaway store. [Checks done by hand](#checks-done-by-hand) still says it wasn't done | |
| Process 2: `c660f3b` changed four existing tests after `start`, though the change was foreseeable | none | No action. It's advice the builders now follow: tests that a change foreseeably breaks are updated before `start` | |

The review's [Slice 10 must decide](reviews/slice-9-review-1.md#slice-10-must-decide) list is left for slice 10, whose item is TR-6, and is carried into [Limits](#limits-and-what-later-slices-change).

A few consequences:

- `review`'s heading no longer says "it" for two items, and `show --diff` for a root outside a git work tree says it can't show the commits. Both are text only.
- `show --diff` prints what the commits hold, whatever textconv filter `.git/config` sets for a file.
- `gates --json` has two required fields more, so a script that read its exact shape sees `triggers` and `holds` once every gate passes. `submit`'s output is unchanged.
- The Try it output changed in two places, the inbox's heading and `show PL-1 --diff`'s message. `gates` isn't in the demo.

## Still open

- **Process** (review 1's process 1; PLAN.md's _Done when_, fourth bullet, and its integration risk "Interactive-only commands are only half tested"): typing the ID into `approve` in a real terminal, and into `reject`, hasn't been checked by hand. TR-5's criterion 4 passed on its `script` tests alone. It's the user's to do before slice 10 starts.
- **Design** (review 1's design 1): `approve` doesn't run the triggers again, so commits made after a held submit are approved with only the submit's holds. Left for slice 10 (TR-6), once its triggers read commits.
- **Slice 10 must decide** ([review 1](reviews/slice-9-review-1.md#slice-10-must-decide)): all six points are left for slice 10 ([Limits](#limits-and-what-later-slices-change)).
- **Test gaps** (review 1's test gap 2, and `approve`'s first re-read from test gap 1): `review`'s line for an item in review with no holds and `show --diff`'s text for an item with no commits have no test, and neither has `approve`'s first re-read, which the second one covers ([Other tests](#other-tests)).
- **Test gaps** (slice 7 review 1's finding 6, carried forward by slice 8's reviews): `Env::add_criterion` in [tests/common/mod.rs](../tests/common/mod.rs) still adds `--check true`, left for slice 11.
- **Test gaps** (slice 8 review 2's test gap 1, third case): `deletion_warning`'s wording for an `unchecked` record with exactly one check still has no test.

## Limits, and what later slices change

- **Slice 10** adds T1, T2, T3, T4, T7 and T8 to `triggers::run`, which then needs the item's changed files, the config and the root. `gates_dry_run` calls `triggers::run` too, since TR-29, so it needs the same inputs, and `gates.schema.json`'s `triggers` and `holds` take the new triggers without a change. Slice 10 also adds `review.disabled`, which must never switch off T6, T10 or T12 (§2.5). T9 isn't on that list, so `disabled` can silence it. `show --diff` lists the files that fired a trigger first, so `Hold` or `commits_and_patches` needs to know which files each hold names, and `holds`' shape must stay `{trigger, message}` for fixture XX-20 and the schemas, or gain a field with a serde default. After a reject, a resubmit is held again for every trigger that still applies, which `apply` already does by running every trigger fresh; `criterion_5::a_reject_puts_its_reason_in_history` checks it for T9. TR-5 would be held by T1, as a `capability`, and by T4, for `c660f3b`.
- **Slice 11** adds T5, T13 and T14. T13 holds every test item whose check passed at start, and `Env::add_criterion`'s check is `true`, so most tests that submit would be held and need `approve` under `script`, as `c660f3b`'s did. Changing the helper to a check that fails at start, as slice 7's readme asks, avoids that. `reject` keeps `started`, so after a reject T13 still reads the original start's results and T14 the original `started.head`.
- **Slice 12** adds T10, for an unsure answer or `auto_verify: false`, and `verification.judges`. A `human` record then has `judges` and `triggers`, and adding `judges` with a serde default and `skip_serializing_if` keeps XX-21 byte for byte. `approve` runs the gates again, so it runs the judges again too, from the cache when nothing changed (§2.3.1).
- **Slice 15** makes `approve` count each judge question held under T10 or T12 as passed for that approval only, and has `approve` and `reject` record my decision against each held question. Both now read `holds` and nothing finer, so the held questions need recording somewhere `approve` and `reject` can read them.
- **Slice 16**'s digest lists every `approve` and `reject` from history, `via` and `interactive`, and `review` says whether a digest is pending or overdue, which §3's inbox shows and this slice doesn't. `audit` checks a `human` record's ref as it checks an `auto` one's.
- **`approve` and `reject` are only as interactive as stdin.** `script` gets past the terminal check, as the tests show, and DESIGN.md §7 says so: an approve made that way records `"interactive": true` and looks like mine, which only the digest (slice 16) can show.
- **A held item pauses its project** until it's decided (§4.4), as the demo's `start BM-3` shows. An agent can't start the next item, so once one is held it stops and reports the holds.
- **`approve` doesn't run the triggers**, so a hold that would fire only now isn't recorded in `verification.triggers`. [Review 1's design 1](reviews/slice-9-review-1.md#where-the-code-and-designmd-disagree-or-the-design-is-unclear) asks slice 10 to decide, once T1 to T8 read commits: either `approve` runs them at the `HEAD` it names and shows the new holds, or §2.7 and §7 say why not. Slice 15's per-question records need the same answer.
- **TR-6 is tracked by this slice's binary**, so T6 and T9 can hold it, and nothing else can. If it's held, only the user at a terminal moves it on, its tree under `tracker/` has to stay clean until then, since `approve` re-runs G5 and G4's `cargo test`, and everything committed after the held submit becomes part of the approved commit ([review 1](reviews/slice-9-review-1.md#slice-10-must-decide)).
- **`show --diff` runs `git show` once with every item commit**, so an item with many commits, or with a commit that touches large files, prints all of it. It reads every commit reachable from `HEAD`, as G5 does, to find them.
- **The tests that put an item in review by editing its file**, in `tests/states.rs`, `tests/gates.rs`, `tests/criteria.rs`, `tests/organise.rs` and `tests/show_and_delete.rs`, still do, though [slice 3's](slice-3-readme.md#limits-and-what-later-slices-change) and [slice 4's](slice-4-readme.md#limits-and-what-later-slices-change) readmes suggested a real hold. They test the refusals, which don't depend on how the item got there, and `tests/review.rs` tests the same refusals with a real hold.
- **`delete`'s warning for an item in review doesn't name its holds**, which slice 4's readme suggested it could.

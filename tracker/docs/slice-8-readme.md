# Slice 8: Submit proves committed work

`tracker` is a command-line work tracker, written in Rust, that keeps one central list of work items across many projects and decides when an item is really done. This document covers **slice 8** of the 16 in [PLAN.md](PLAN.md): what it built, how it works, and how to test it. The earlier readmes cover the rest: [slice 1](slice-1-readme.md) the store, `create` and `list`, [slice 2](slice-2-readme.md) the states and `history`, [slice 3](slice-3-readme.md) priority, kind, labels, `edit` and `list`'s filters, [slice 4](slice-4-readme.md) `show` and `delete`, [slice 5](slice-5-readme.md) bad data and polish, [slice 6](slice-6-readme.md) criteria and gates G1, G2 and G6, and [slice 7](slice-7-readme.md) running checks, with gates G3 and G4 and `started.checks`.

After slice 8 you can **get items auto-verified against a commit, with saved output and a pinned ref**. The spec is [DESIGN.md](DESIGN.md). This slice implements an item's commits and the folder rule from [§2.1](DESIGN.md#21-items), gate G5 from [§2.4](DESIGN.md#24-gates-is-it-finished) and its new paragraph "Checks leave the tree as they found it", the `auto` verification record and `refs/tracker/<ID>` from [§2.7](DESIGN.md#27-human-actions), shelving an in-progress item and deleting the ref from [§2.8](DESIGN.md#28-shelve-vs-delete), `started.head`, `verification.commit` and `verification.ref` from [§4.3](DESIGN.md#43-item-file), and [§4.4](DESIGN.md#44-handling-bad-data)'s rule for a root that isn't in a git work tree. It's tracked in `tracker` itself as item TR-4, with the binary installed from slice 7.

| | |
| --- | --- |
| Status | Every _Done when_ check has a test, and every test passes, as does the test command. TR-4 is `done`, but verified `unchecked`: the installed slice 7 binary ran G3's 8 criterion checks and G4's 3 project checks at submit, but it has no G5, so TR-4's record has no `commit` or `ref`, and its `started` has no `head` ([Tracking TR-4](#tracking-tr-4)). Seven of TR-4's eight checks failed at `start`, as they should. Criterion 8's already passed, since slice 7's re-read under the lock already makes `submit` write nothing when a check edits the item file, so the start warned, and slice 11's T13 would hold such an item. Removing the ref on `delete` is tested only under `script`: typing the ID into `delete` in a real terminal hasn't been checked by hand ([Checks done by hand](#checks-done-by-hand)). There are no reviews of this slice yet. PLAN.md has slice 8 as `in review`. The branch isn't merged into `main`, and neither is slice 7's, which it's stacked on. |
| Branch | `feat/tracker/slice-8`, stacked on `feat/tracker/slice-7` at `4da3efb`. Slice 7 isn't merged into `main`, so this slice's commits are `git log --oneline --reverse feat/tracker/slice-7..feat/tracker/slice-8`, not the range from `main`'s merge base, `37abcc3`, which would include slice 7's as well. In order: `89f5797` (TR-4's tests, committed before `tracker start TR-4`), `42b0a99` (PLAN.md: slice 8 in progress), `ce56ee2` (`src/git.rs`), `1bc57a9` (G5, the `auto` record and ref, and shelving's gates), `8ba0e95` (older tests given a repository and an item commit), `e3aa24f` (tests beyond the criteria), `5195dd6` (fixture XX-19), `d6d3664` (help text and schemas), `fc890cd` (DESIGN.md), `4795629` (shelve's G5 message), `e5f37b1` (PLAN.md: slice 8 in review), then this readme |
| Test command | `cargo fmt --check && cargo clippy -- -D warnings && cargo test`, run in `tracker/`. All three pass. |
| Tests | 295: 39 in `tests/create_and_list.rs`, 34 in `tests/organise.rs`, 27 in `tests/states.rs`, 24 in `tests/commits.rs`, 24 in `tests/show_and_delete.rs`, 23 in `tests/criteria.rs`, 22 in `tests/gates.rs`, 18 in `tests/checks.rs`, 14 in `tests/item_files.rs`, 13 in `tests/bad_data.rs`, 11 in `tests/docs_links.rs`, 11 in `tests/fixtures.rs`, 9 in `tests/polish.rs`, 6 in `tests/output.rs`, 5 in `tests/schemas.rs`, 4 in `tests/store_writes.rs`, 2 in `tests/item_ids.rs`, 7 unit tests in the library (4 in `src/gates.rs`, 1 each in `src/store.rs`, `src/checks.rs` and `src/git.rs`) and 2 in `src/main.rs` |

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

- **An item's commits** are the commits reachable from `HEAD` whose header is `<type>(<ID>): …` or `<type>(<ID>)!: …`, with a lowercase type. A commit scoped to `XX-12`, one headed `XX-1: …`, one that names the ID only in its body, and one on a branch `HEAD` can't reach aren't XX-1's.
- **G5**, on `submit`, `status <id> done` and the `gates` dry run, between G2 and G6: nothing under the project's root is uncommitted or untracked, and the item has at least one commit. Files that `.gitignore` excludes, and files outside the root, don't count, so a subfolder project such as TR isn't refused for a dirty file elsewhere in the repository.
- **G5 again after the checks.** Once G3 and G4 have run, G5 looks at the tree again, and fails if a check changed or added a file under the root, or moved `HEAD`, naming only what changed. DESIGN.md §2.4 now says checks leave the tree as they found it (`fc890cd`).
- **`start`**, and `status <id> in-progress`, record `HEAD` as `started.head`, a full SHA, left out in a repository with no commits yet.
- **A passing submit auto-verifies.** The record is `by: "auto"`, with `commit`, the `HEAD` G5 saw, and `ref`, `refs/tracker/<ID>`, which the submit creates in the project's repository, or moves, pointing at that commit. It keeps slice 7's `checks`, with each check's result, time and last 20 lines of output, and has no `note`. A refused submit creates no ref.
- **Shelving an in-progress item**, by `shelve` or `status <id> shelved`, needs a clean tree under the root (G5's first half, with no commit needed) and passing project checks (G4). If either fails, it's refused with exit 2, listing every reason. It runs the project checks without the lock and re-reads the item afterwards. Shelving an open item is unchanged.
- **`delete`** removes the item's ref, after the ID is typed and before the item's file goes, and its warning names the ref among what's lost and the commit the item was verified at.
- **`show`** prints a `Started:` section, with `at`, `head` and each criterion's result at start, and each verification record's `commit`, `ref` and checks, each with its criterion, result, time, command and saved output, escaped with its line breaks kept. The previous record says its criteria are numbered as they were at that submit.
- **A root that isn't inside a git work tree** makes `start`, `gates`, `submit` and shelving an in-progress item exit 3, changing nothing.

**Not built yet:**

- **Triggers, the inbox, `approve`, `reject` and `show --diff`** (slice 9). Every submit that passes the gates still reaches done, now `auto`, manual criteria included.
- **Changed files and T1–T4, T7 and T8** (slice 10). `src/git.rs` reads an item's commits, but not which files they touched.
- **T5, T13 and T14** (slice 11). `started.head` is recorded for T14, but nothing reads it yet. A check that passes at start still only gets slice 7's warning.
- **`started.criteria_hash`** (slice 11), which goes before `head` in §4.3's order.
- **Judges, and `diff` as a judge's input** (slices 12 and 14), which read the item's commits.

## Try it

Always set `TRACKER_HOME` while developing. Without it, the tool uses your real store at `~/.tracker`.

The project, BM, is the folder `bm` inside a throwaway repository, as TR is a folder inside guild-portfolio. The repository also has `other/notes.txt`, outside BM's root, and BM ignores `*.log`. The demo's commits use a made-up name, so they work whatever your own git settings say.

```sh
cd guild-portfolio/tracker
cargo build

export DEMO=$(mktemp -d)
export TRACKER_HOME=$DEMO/store
export PATH="$PWD/target/debug:$PATH"
export GIT_AUTHOR_NAME=demo GIT_AUTHOR_EMAIL=demo@example.invalid
export GIT_COMMITTER_NAME=demo GIT_COMMITTER_EMAIL=demo@example.invalid
mkdir -p $DEMO/repo/bm $DEMO/repo/other && cd $DEMO/repo
git init --quiet -b main
echo notes > other/notes.txt
echo '*.log' > bm/.gitignore
git add -A && git commit --quiet -m "chore: set up"
cd bm
tracker init
tracker project add BM --name bookmarks --root .
```

Give BM a project check, which fails while a file called `broken` exists:

```sh
cat > $TRACKER_HOME/config.json <<END
{
  "projects": {
    "BM": {
      "name": "bookmarks",
      "root": "$DEMO/repo/bm",
      "checks": ["test ! -f broken && echo all 4 tests passed"]
    }
  }
}
END
```

Create an item, start it, and do the work without committing it. An ignored file and a change outside the root don't count. Then commit it with a bare `BM-1:` header, which isn't an item commit, and amend that to a scoped one:

```sh
tracker create "Search filters bookmarks by tag" --kind capability
tracker criterion add BM-1 "Filtering by tag shows only tagged bookmarks" --check 'cat tags.txt'
tracker start BM-1
echo "test filters_by_tag ... ok" > tags.txt
echo noise > debug.log
echo more >> ../other/notes.txt
tracker gates BM-1          # exit 2: G5, untracked tags.txt and no item commit
git add tags.txt && git commit --quiet -m "BM-1: filter by tag"
tracker gates BM-1          # exit 2: G5, a bare BM-1 header is no item commit
git commit --quiet --amend -m "feat(BM-1): filter by tag"
tracker submit BM-1
git rev-parse --short refs/tracker/BM-1
tracker show BM-1
```

A second item whose check leaves a file in the root, and a shelve that's refused until the tree is clean and the project check passes:

```sh
tracker create "Sort bookmarks by date"
tracker criterion add BM-2 "Newest first by default" --check 'test -f sorted.txt && touch sorted.cache'
tracker start BM-2
touch sorted.txt && git add sorted.txt && git commit --quiet -m "feat(BM-2): sort by date"
tracker submit BM-2         # exit 2: G5, the check left sorted.cache
touch broken
tracker shelve BM-2 --reason "Needs the date field from BM-3."   # exit 2: G5 and G4
rm broken sorted.cache
git rm --quiet sorted.txt && git commit --quiet -m "revert(BM-2): take out the unfinished sort"
tracker shelve BM-2 --reason "Needs the date field from BM-3."
```

`delete` still refuses anything but a terminal, and a project whose root isn't in a repository can't start:

```sh
echo BM-1 | tracker delete BM-1   # exit 1: not a terminal
mkdir $DEMO/plain
tracker project add PL --name plain --root $DEMO/plain
tracker create "Anything" --project PL --done-when "It works"
tracker start PL-1          # exit 3: not in a git work tree
tracker list --status done,shelved
cat $TRACKER_HOME/items/BM-1.json
```

The output, from `tracker init` on, with stdout and stderr together:

```
$ tracker init
Created the store at $DEMO/store.
$ tracker project add BM --name bookmarks --root .
Registered BM (bookmarks) at $DEMO/repo/bm.
$ tracker create "Search filters bookmarks by tag" --kind capability
Created BM-1: Search filters bookmarks by tag
$ tracker criterion add BM-1 "Filtering by tag shows only tagged bookmarks" --check 'cat tags.txt'
Added criterion 1 to BM-1: Filtering by tag shows only tagged bookmarks
$ tracker start BM-1
Running criterion 1's check: `cat tags.txt`
BM-1: open → in-progress
$ echo "test filters_by_tag ... ok" > tags.txt
$ echo noise > debug.log
$ echo more >> ../other/notes.txt
$ tracker gates BM-1          # exit 2: G5, untracked tags.txt and no item commit
Running criterion 1's check: `cat tags.txt`
Running project check: `test ! -f broken && echo all 4 tests passed`
error: BM-1 would be refused at submit: G5 failed. This was a dry run, so nothing was changed.
  G5  BM's root has an uncommitted change: untracked bm/tags.txt. Commit it, scoped to BM-1, or remove it: the proof has to match a real commit.
  G5  BM-1 has no commits. Commit its work with a message headed `<type>(BM-1): …`, such as `feat(BM-1): …`, so the proof matches a real commit (§2.1).
$ git add tags.txt && git commit --quiet -m "BM-1: filter by tag"
$ tracker gates BM-1          # exit 2: G5, a bare BM-1 header is no item commit
Running criterion 1's check: `cat tags.txt`
Running project check: `test ! -f broken && echo all 4 tests passed`
error: BM-1 would be refused at submit: G5 failed. This was a dry run, so nothing was changed.
  G5  BM-1 has no commits. Commit its work with a message headed `<type>(BM-1): …`, such as `feat(BM-1): …`, so the proof matches a real commit (§2.1).
$ git commit --quiet --amend -m "feat(BM-1): filter by tag"
$ tracker submit BM-1
Running criterion 1's check: `cat tags.txt`
Running project check: `test ! -f broken && echo all 4 tests passed`
BM-1: in-progress → done
Verified automatically: every gate passed at 42bc260, and refs/tracker/BM-1 keeps that commit.
$ git rev-parse --short refs/tracker/BM-1
42bc260
$ tracker show BM-1
BM-1  Search filters bookmarks by tag
  project   BM (bookmarks)
  kind      capability
  status    done
  priority  medium
  created   2026-11-21T14:42:45Z
  updated   2026-11-21T14:42:45Z

Done when:
  1. Filtering by tag shows only tagged bookmarks
       check: cat tags.txt

Started:
  at      2026-11-21T14:42:45Z
  head    3411502fa4d1943e4d69fc5303a7699c2c39569a
  checks  criterion 1 failed

Verification:
  by      auto
  at      2026-11-21T14:42:45Z
  commit  42bc260a7cb587cad539066900356f5a8c223621
  ref     refs/tracker/BM-1
  checks:
    criterion 1, passed in 0s: `cat tags.txt`
      test filters_by_tag ... ok
    project, passed in 0s: `test ! -f broken && echo all 4 tests passed`
      all 4 tests passed

History, oldest first (2 entries):
  2026-11-21T14:42:45Z  start   open → in-progress  not from a terminal
  2026-11-21T14:42:45Z  submit  in-progress → done  not from a terminal
$ tracker create "Sort bookmarks by date"
Created BM-2: Sort bookmarks by date
$ tracker criterion add BM-2 "Newest first by default" --check 'test -f sorted.txt && touch sorted.cache'
Added criterion 1 to BM-2: Newest first by default
$ tracker start BM-2
Running criterion 1's check: `test -f sorted.txt && touch sorted.cache`
BM-2: open → in-progress
$ touch sorted.txt && git add sorted.txt && git commit --quiet -m "feat(BM-2): sort by date"
$ tracker submit BM-2         # exit 2: G5, the check left sorted.cache
Running criterion 1's check: `test -f sorted.txt && touch sorted.cache`
Running project check: `test ! -f broken && echo all 4 tests passed`
error: BM-2 can't be submitted: G5 failed. Nothing was changed.
  G5  The checks left an uncommitted change: untracked bm/sorted.cache under BM's root. A check must leave the tree as it found it, so make it write outside the root, or to a file that .gitignore excludes.
$ touch broken
$ tracker shelve BM-2 --reason "Needs the date field from BM-3."   # exit 2: G5 and G4
Running project check: `test ! -f broken && echo all 4 tests passed`
error: BM-2 can't be shelved while it's in progress: G5 and G4 failed. The next item builds on this tree, so it must be clean and pass the project checks. Nothing was changed.
  G5  BM's root has uncommitted changes: untracked bm/broken and untracked bm/sorted.cache. Commit them, scoped to BM-2, or remove them: the next item builds on what's committed.
  G4  A project check failed: `test ! -f broken && echo all 4 tests passed` (exit 1). It printed nothing.
$ rm broken sorted.cache
$ git rm --quiet sorted.txt && git commit --quiet -m "revert(BM-2): take out the unfinished sort"
$ tracker shelve BM-2 --reason "Needs the date field from BM-3."
Running project check: `test ! -f broken && echo all 4 tests passed`
BM-2: in-progress → shelved
$ echo BM-1 | tracker delete BM-1   # exit 1: not a terminal
error: `tracker delete BM-1` needs a human at an interactive terminal, who types BM-1 to confirm it, and stdin here isn't a terminal. Nothing was changed. Only a human deletes items, so a human decides whether to delete BM-1, which is done.
$ mkdir $DEMO/plain
$ tracker project add PL --name plain --root $DEMO/plain
Registered PL (plain) at $DEMO/plain.
$ tracker create "Anything" --project PL --done-when "It works"
Created PL-1: Anything
1 manual criterion: give it a check with `tracker criterion set PL-1 1 --check "…"`.
$ tracker start PL-1          # exit 3: not in a git work tree
error: PL's root, $DEMO/plain, isn't inside a git work tree, so tracker can't record or prove PL-1's work against a commit. Make it a repository with `git init`, or register PL with a root that's inside one. Nothing was changed.
$ tracker list --status done,shelved
  ID    PRIORITY  KIND        STATUS   TITLE
BM  bookmarks
  BM-1  medium    capability  done     Search filters bookmarks by tag
  BM-2  medium    change      shelved  Sort bookmarks by date

BM only. `--all` for every project.
Filters: status done or shelved.
$ cat $TRACKER_HOME/items/BM-1.json
{
  "id": "BM-1",
  "title": "Search filters bookmarks by tag",
  "project": "BM",
  "kind": "capability",
  "status": "done",
  "priority": "medium",
  "created": "2026-11-21T14:42:45Z",
  "updated": "2026-11-21T14:42:45Z",
  "criteria": [
    {
      "text": "Filtering by tag shows only tagged bookmarks",
      "check": "cat tags.txt"
    }
  ],
  "started": {
    "at": "2026-11-21T14:42:45Z",
    "head": "3411502fa4d1943e4d69fc5303a7699c2c39569a",
    "checks": [
      {
        "criterion": 1,
        "passed": false
      }
    ]
  },
  "verification": {
    "by": "auto",
    "at": "2026-11-21T14:42:45Z",
    "commit": "42bc260a7cb587cad539066900356f5a8c223621",
    "ref": "refs/tracker/BM-1",
    "checks": [
      {
        "criterion": 1,
        "cmd": "cat tags.txt",
        "ok": true,
        "secs": 0,
        "output": "test filters_by_tag ... ok"
      },
      {
        "cmd": "test ! -f broken && echo all 4 tests passed",
        "ok": true,
        "secs": 0,
        "output": "all 4 tests passed"
      }
    ]
  },
  "history": [
    {
      "at": "2026-11-21T14:42:45Z",
      "from": "open",
      "to": "in-progress",
      "via": "start",
      "interactive": false
    },
    {
      "at": "2026-11-21T14:42:45Z",
      "from": "in-progress",
      "to": "done",
      "via": "submit",
      "interactive": false
    }
  ]
}
```

Notes on the output:

- The SHAs and times differ on every run, since each commit records when it was made.
- On macOS, `project add` prints the root with `/private` in front of `$DEMO`, since it stores the path with symlinks followed.
- G5 names files by their path from the top of the repository, `bm/tags.txt`, not from BM's root. `debug.log`, which `.gitignore` excludes, and `../other/notes.txt`, outside the root, aren't named.
- After the `BM-1: filter by tag` commit the tree is clean, so only the missing item commit is left. Amending its message makes it BM-1's.
- `start BM-2` ran the check once, and it failed before `sorted.txt` existed, so it left nothing. At the submit it passed and left `sorted.cache`, which G5's second look names, and G3 passes.
- The refused shelve names `sorted.cache` again: it's still there, and the next item would build on it. The revert commit is scoped to BM-2, as DESIGN.md §2.8 asks.
- `start PL-1` refuses before any check runs, and PL-1 stays open with no `started`.
- History says `"interactive": false` because the demo ran from a script. Typed at a terminal, it's `true`.

## Commands

No command or flag is new. These change:

| Command | What it does now |
| --- | --- |
| `tracker start <id>`, `status <id> in-progress` | G1, then records `HEAD` under the lock, then each criterion check without it, as in slice 7. Exits 3 if the root isn't in a git work tree. |
| `tracker submit <id>`, `status <id> done` | G2, G5 and G6 under the lock, then G3 and G4 without it, then G5's second look. Under the lock again, refuses if anything failed, or creates `refs/tracker/<ID>` and moves the item to done, `auto`, with `commit` and `ref`. Exits 3 if the root isn't in a git work tree. |
| `tracker gates <id>` | G2, G5, G6, G3 and G4, then G5's second look, without the lock. It changes nothing, and exits 3 if the root isn't in a git work tree. |
| `tracker shelve <id> --reason "…"`, `status <id> shelved` | From in-progress: G5's clean tree under the lock, then G4 without it, then G5's second look, then the re-read under the lock. Refused with exit 2 listing every reason, or exit 1 if the item changed meanwhile. From open: as before. |
| `tracker delete <id>` | For an item whose record or previous record names a ref, and whose ref exists, the warning lists the ref, and the ref is removed once the ID is typed and the item re-read. A done `auto` item's warning gives its commit, cut to 7 characters. |
| `tracker show <id>` | Adds `Started:`, and each record's `commit`, `ref` and checks with their saved output. |

The help text for `start`, `submit` and `shelve` says what they now need (`d6d3664`).

Exit codes in this slice, as [§3](DESIGN.md#3-interface) defines them:

- `0`: as before. A shelve from in-progress that passes G5 and G4 exits 0.
- `1`: as before, plus shelving an in-progress item whose file changed while its project checks ran.
- `2`: a gate refused, now including G5 on `submit` and `gates`, and G5 or G4 on shelving an in-progress item.
- `3`: as before, plus `start`, `gates`, `submit` and shelving an in-progress item when the project's root isn't inside a git work tree, or git can't be run or fails. A root that doesn't exist, or a project missing from `config.json`, isn't exit 3: G5 fails with a reason, as G3 and G4 do.

## How it works

### Reading git

[src/git.rs](../src/git.rs) runs `git` as a subprocess in the project's root, with stdin closed. Its private `git` helper turns off `log.showSignature` and `core.quotePath`, so a user's settings don't change what it reads, and fails with exit 3, quoting git's stderr, if git can't run or exits non-zero. The public functions:

- `require_work_tree`: `git rev-parse --is-inside-work-tree`. Anything but `true` is exit 3, with the message the demo shows.
- `head`: `git rev-parse --verify --quiet HEAD^{commit}`, the full SHA, or `None` in a repository with no commits.
- `uncommitted`: `git status --porcelain=v1 -z --untracked-files=all -- .`, run in the root, so only files under it are listed, by their path from the repository's top, each as `untracked`, `modified`, `deleted`, `added`, `renamed`, `copied` or `unmerged`. `-z` keeps names exactly, and `--untracked-files=all` names each file in an untracked folder.
- `is_item_header`: whether a commit's first line is `^[a-z]+\(<ID>\)!?: `, matched by hand, a byte at a time, rather than with the `regex` crate.
- `item_commits`: `git log -z --format=%H%n%B HEAD`, keeping the SHAs whose header `is_item_header` accepts, newest first. It reads every commit reachable from `HEAD`, not only those that touch the root, as §2.1 defines an item's commits.
- `ref_name`, `set_ref` (`git update-ref`), `ref_exists` (`git show-ref --verify --quiet`) and `delete_ref` (`git update-ref -d`), for `refs/tracker/<ID>`.

### G5

[src/gates.rs](../src/gates.rs) `statics` runs G2, then `g5`, then G6, which is §2.4's cheapest-first order for the static gates. It returns a `Result` now, since G5 can exit 3.

`g5(item, config, needs_commit)`:

1. Finds the root with `root`, the same function the checks use. If it can't, G5 fails with the reason, such as a project missing from `config.json`, and nothing else runs.
2. `git::require_work_tree`, which exits 3.
3. Records `HEAD` in `Report::head`, and the list from `git::uncommitted` in `Report::uncommitted`.
4. If the list isn't empty, one failure names at most 10 files (`git::LISTED_FILES`), then "N more". For a submit it ends "the proof has to match a real commit", and for a shelve "the next item builds on what's committed" (`4795629`).
5. If `needs_commit`, a second failure when `git::item_commits` finds none.

`Report::extend` keeps the other report's `head` when it has one, and appends its `uncommitted`.

### G5 again after the checks

`tree_after_checks` runs once the checks have run, in `gates_dry_run`, `submit` and an in-progress shelve, before the lock is taken again:

- It runs `git::uncommitted` again, and fails G5 for each file not in `Report::uncommitted`, so a file that was already there isn't named twice: "The checks left … under XX's root. A check must leave the tree as it found it…".
- It reads `HEAD` again, and fails G5 if it isn't `Report::head`: "HEAD moved from … to … while the checks ran…". So the recorded commit is the one the checks ran against.
- If the root can't be found, it does nothing, since G5 has already failed.

The `gates` list still names G5 once, at its place after G2. A failure from the second look goes into `failures` with `gate: "G5"`, so G5 is listed with `ok: false`.

### What start records

`apply` in [src/main.rs](../src/main.rs), for `start`: after G1 passes under the lock, `gates::head_at_start` finds the root, calls `git::require_work_tree`, which exits 3 before any check runs, and reads `HEAD`. A root that can't be found gives `None` without a refusal, as slice 7 lets such a start go ahead and records its checks as failed. Then the lock is let go and the checks run as in slice 7. `item::Started::head` holds the SHA, between `at` and `checks`.

### A passing submit

`apply`, for `submit`:

1. Under the lock: `gates::statics` (G2, G5, G6).
2. The lock is let go. `gates::scripts` runs G3 and G4, then `gates::tree_after_checks`.
3. `Store::lock_unchanged_after_checks` takes the lock again, and refuses with exit 1 if the item's file isn't exactly what was read. This is slice 7's re-read, now covered for `submit` by criterion 8's tests.
4. If any gate failed, the refusal, with exit 2. No ref has been created.
5. `git::set_ref` points `refs/tracker/<ID>` at `Report::head`, still under the lock. If the root or `HEAD` can't be found at this point, which G5 passing rules out, it exits 3.
6. The item gets `Verification { by: Auto, commit, git_ref, checks, note: None }`, and `submit` prints "Verified automatically: every gate passed at <short SHA>, and refs/tracker/<ID> keeps that commit."

`gates::scripts` is now `g3` and `g4` together, so shelving can run `g4` on its own. `states::UNCHECKED_NOTE` is gone: no command writes an `unchecked` record now, and records that have the note keep it.

### Shelving an in-progress item

`apply`'s arm `Via::Shelve if step.from == Status::InProgress`, which `shelve` and `status <id> shelved` both reach:

1. Under the lock: `gates::g5(item, config, false)`, the clean tree without the item-commit rule.
2. The lock is let go. `gates::g4` runs the project checks, then `gates::tree_after_checks`.
3. `Store::lock_unchanged_after_checks`, exit 1 if the item changed.
4. If anything failed, `Report::refusal` with `Run::Shelve`: "XX-1 can't be shelved while it's in progress: … The next item builds on this tree, so it must be clean and pass the project checks. Nothing was changed."

G3 doesn't run: the item's own tests were committed before `start` and still fail, which is why it's being shelved. A shelve from open takes the old path and runs nothing.

### Deleting an item's ref

`delete` in [src/main.rs](../src/main.rs):

1. `ref_to_delete`, before the question: if the item's `verification` or `previous_verification` names a `ref`, it loads the config, finds the root with `gates::root`, and asks `git::ref_exists`. Otherwise it asks git nothing. A root that can't be found gives `None`, and the ref is left, as §2.7 now says.
2. `deletion_warning(item, has_ref)` adds `refs/tracker/<ID>` to what's lost, and for an `auto` record with a commit says "verified by its gates at <short SHA>", as §2.8's example does.
3. `tty::confirm`, then `Store::lock_unchanged`, as in slice 4.
4. `git::delete_ref`, before the line in `deleted.json` and the file. If it fails, the delete exits 3 and the item is still there to delete again.

### What show prints

`show` calls `started_text` for `Started:`, then `verification_text` for each record. `verification_text` prints `by`, `at`, `commit`, `ref` and `note` as rows, then `checks:`, one line per check, "criterion 1, passed in 0s: `cmd`" or "project, …", with each line of its saved output under it, indented six spaces. Everything goes through `Styled::push`, so a control character in a check's output is printed as `\u{1b}` and so on, as [slice 5's readme](slice-5-readme.md#limits-and-what-later-slices-change) asks. The output is what was saved at that submit, so it still reads `test filters_by_tag ... ok` after the test is renamed.

## JSON output

| Command | Schema | What changed |
| --- | --- | --- |
| `gates`, `submit`, `status <id> done` | [gates.schema.json](../schemas/gates.schema.json), [submit.schema.json](../schemas/submit.schema.json) | `gates` lists G2, G5, G6, G3 and G4. `submit.schema.json`'s description says so. The `gate` pattern, `^G[0-9]+$`, already allowed G5 |
| `start`, `submit`, `gates` or an in-progress shelve, refused | [error.schema.json](../schemas/error.schema.json) | A shelve's refusal is the gate shape, with `result: "refused"`, `status: "in-progress"`, `gates` of G5 and G4, and `failures`. The description names shelving among the commands that print it |
| every item | [item.schema.json](../schemas/item.schema.json) | `started.head`, `verification.commit` and `verification.ref`, with a new `sha` definition: 40 hex digits, or 64 in a SHA-256 repository. `ref` must be `refs/tracker/<ID>`. `by`'s `unchecked` now means done before the gates proved the work against a commit |

`start`'s and `submit`'s output return the item, so `head`, `commit` and `ref` appear there with no other schema change. Slice 9 adds `"to": "review"` results and real `triggers` and `holds` to `submit`, and `show --diff`'s commits beside `item`. Slice 11 adds `started.criteria_hash`, and slice 12 `verification.judges`. None of these needs a field here to change.

## Code tour

| File | What's new or changed |
| --- | --- |
| [src/git.rs](../src/git.rs) | New (`ce56ee2`). `git`, `require_work_tree`, `head`, `uncommitted`, `is_item_header`, `item_commits`, `ref_name`, `set_ref`, `ref_exists`, `delete_ref`, `LISTED_FILES`, and the unit test `a_header_is_scoped_to_an_item_only_by_its_conventional_commit_scope` |
| [src/gates.rs](../src/gates.rs) | `Report::head` and `uncommitted`, `Run::Shelve` and its refusal message, `statics` (now G2, G5, G6, returning a `Result`), `g5`, `tree_after_checks`, `short`, `uncommitted_list`, `g5_failure`, `scripts` split into `g3` and `g4`, `head_at_start`, and `root` made public |
| [src/main.rs](../src/main.rs) | `apply`: `head` at start, G5 and `tree_after_checks` at submit, `git::set_ref`, the `auto` record, and the in-progress shelve arm. `gates_dry_run` runs `tree_after_checks`. `show` calls the new `started_text` and `verification_text`, which replaces `verification_rows`. `delete` calls the new `ref_to_delete` and `git::delete_ref`, and `deletion_warning` takes `has_ref`. Help text for `start`, `submit` and `shelve` |
| [src/item.rs](../src/item.rs) | `Started::head`, `Verification::commit` and `Verification::git_ref` (serialised as `ref`), each with a serde default and `skip_serializing_if` |
| [src/states.rs](../src/states.rs) | `UNCHECKED_NOTE` removed |
| [src/lib.rs](../src/lib.rs) | `pub mod git` |
| [schemas/](../schemas/) | `item`, `error` and `submit`, as [above](#json-output) |
| [tests/commits.rs](../tests/commits.rs) | New: 17 tests for TR-4's eight criteria (`89f5797`) and 7 more (`e3aa24f`), with `env_with_repo`, `env_with_item`, `env_ready_to_submit`, `set_project`, `g5_failures`, `g5_passes` and `check_that_edits` |
| [tests/common/mod.rs](../tests/common/mod.rs) | `git`, `git_ok`, `git_init` and `git_commit_all`. `Env::without_store` makes the work folder a repository, `create_bare` commits after creating, and `commit_for` commits again ([Testing](#testing)) |
| [tests/fixtures.rs](../tests/fixtures.rs), `tests/fixtures/slice-8/` | XX-19 and `slice_8s_xx_19_has_a_commit_and_a_ref_and_comes_back_byte_for_byte` (`5195dd6`) |
| [docs/DESIGN.md](DESIGN.md) | `fc890cd`: §2.4's "Checks leave the tree as they found it", §2.7's ref left when the root can't be found, §2.8's shelve refusal and re-read, §3's refusal list, §4.3's `head`, `commit` and `ref` bullet and `note`, §4.4's exit-3 list, and §7's limit on interrupting `tracker`, which now names shelving |
| [docs/PLAN.md](PLAN.md) | Slice 8's status, `todo` to `in progress` (`42b0a99`) to `in review` (`e5f37b1`) |

To run git from another command, such as slice 9's `approve` or slice 10's changed files: add a function to `src/git.rs` that goes through its `git` helper, so a failure exits 3 the same way, and call it with the root from `gates::root`. To re-run G5 after checks, as `approve` must, call `gates::g5` before letting the lock go and `gates::tree_after_checks` after the checks, as `apply`'s submit arm does.

## Testing

Run the test command in `tracker/`. TR-4's criteria each run their own tests in `tests/commits.rs`, for example `cargo test --test commits g5_fails_for_`.

Every project in `tests/commits.rs` is the subfolder `xx` of a throwaway repository, `<work>/repo`, which also holds `other/notes.txt`, so the folder rule is tested as TR uses it. Its helpers commit through [tests/common/mod.rs](../tests/common/mod.rs)'s `git`, which sets `GIT_CONFIG_GLOBAL=/dev/null` and `GIT_CONFIG_NOSYSTEM=1` and gives a test identity, unsigned, on branch `main`, so a developer's own git settings can't change a test.

The harness changed for every other test file too. G5 now runs on every `submit` and `gates`, so:

- `Env::without_store`, and so `Env::new`, makes the work folder a git repository with one empty commit, so every project root under it is a subfolder of one.
- `Env::create_bare`, and so `Env::create`, commits everything in that repository as `test(<ID>): the work` after creating the item, so it has a commit and the tree is clean, as its tests would be committed before `start`.
- `Env::commit_for(id)` does the same again, for a test that writes a file in a root after creating the item.

### Coverage of the _Done when_ checks

| _Done when_ | Tests in `tests/commits.rs` |
| --- | --- |
| In a throwaway repo, an uncommitted change fails G5, and so does an untracked file. | `g5_fails_for_an_uncommitted_change`: an unstaged change to `xx/a.txt`, then the same change staged, each one G5 failure naming it, then committed, passing. `g5_fails_for_an_untracked_file`: `src/helper.rs` in a new folder, named by its own path, not its folder's |
| An ignored file doesn't fail it, and neither does a dirty file outside a subfolder project's root. | `g5_ignores_an_ignored_file`: `debug.log` under `*.log`. `g5_ignores_a_dirty_file_outside_a_subfolder_projects_root`: a changed `other/notes.txt`, a new `other/new.txt` and a new `top.txt` pass, then `xx/new.txt` fails, and the message names none of the others |
| `feat(XX-1): …` and `fix(XX-1)!: …` count as XX-1's commits. | `a_feat_commit_scoped_to_the_item_counts_as_its_commit`, `a_breaking_fix_commit_scoped_to_the_item_counts_as_its_commit`: "has no commits" before, a pass after |
| These don't: a commit scoped to `XX-12`, one headed `XX-1: …`, one that names XX-1 only in its body, a `feat(XX-1)` commit on another branch. | `a_commit_scoped_to_another_item_doesnt_count_as_its_commit`, `a_commit_headed_with_the_bare_id_doesnt_count_as_its_commit`, `a_commit_naming_the_item_only_in_its_body_doesnt_count_as_its_commit` (the body holds a whole `feat(XX-1): …` line), `a_commit_on_another_branch_doesnt_count_as_its_commit`, which then fast-forwards `main` to it and passes. Each fails G5 for "has no commits" and nothing else |
| A passing submit records the SHA and each check's result, duration and output, and creates the ref. Deleting the item removes the ref. | `a_passing_submit_records_the_commit_and_each_checks_result_and_creates_the_ref`: `auto`, the work's SHA, `refs/tracker/XX-1`, no `note`, and both checks with `criterion`, `cmd`, `ok`, `secs` and `output`, in the JSON and the file, with the ref at that SHA. `a_passing_submits_ref_is_removed_when_the_item_is_deleted`: under `script`, the prompt names the ref, the file and the ref are gone, and the commit stays on `main` |
| The commit is still readable after its branch is rebased away and `git gc --prune=now` runs. After the test is renamed, `show` still prints the saved output. | `the_verified_commit_outlives_a_rebase_and_gc`: verified on branch `work`, which is rebased onto a moved `main`. After `reflog expire` and `gc --prune=now`, a scratch commit nothing points to is gone, which shows the gc pruned, but the verified commit and its `xx/done.txt` are still readable. `the_saved_output_shows_after_the_test_is_renamed`: after `git mv` to `tests/tag_filter.sh`, `show` prints `test filters_by_tag ... ok` and the old command, and nothing of `tag_filter` |
| Shelving an in-progress item is refused while the tree is dirty or a project check fails, and allowed once both are fixed. | `shelving_an_in_progress_item_is_refused_while_the_tree_is_dirty_or_a_project_check_fails`: an untracked `broken`, which also fails the project check `test ! -f broken`, fails G5 and G4. Committed, only G4 fails. Removed but not committed, only G5 fails, naming `xx/broken`. The file is unchanged after each. Committed as `revert(XX-1): …`, the shelve goes ahead |
| If a check edits the item file while the gates run, `submit` writes nothing and exits 1. | `submit_writes_nothing_when_a_criterion_check_edits_the_item_file`, `submit_writes_nothing_when_a_project_check_edits_the_item_file`: a check that runs `tracker edit XX-1 --priority high` once a file outside the repository exists. Exit 1, "changed while its checks ran", the edit kept, no verification record, no new history entry but the edit's, and no ref |

The eight criteria's 17 tests were committed in `89f5797`, 24 seconds before `tracker start TR-4`, which recorded criteria 1–7 as failing and criterion 8 as passing ([Tracking TR-4](#tracking-tr-4)). The code came in `ce56ee2` and `1bc57a9`.

For this readme, `tests/commits.rs` as of `e3aa24f` was run against `42b0a99`'s `src/` and `schemas/`, the code before the slice, in a copy outside the repo: 21 of its 24 tests fail. The three that pass cover code that was already there: criterion 8's two, since slice 7's `lock_unchanged_after_checks` already guards `submit`, and `an_open_item_shelves_whatever_the_tree_and_checks_say`, since shelving an open item hasn't changed since slice 2.

### Other tests

From `e3aa24f`, beyond the criteria, each seen failing in that run but the last:

- `start_records_head`: `started.head` is the repository's `HEAD`.
- `a_root_outside_any_git_work_tree_exits_3`: `gates` and `start` exit 3 for a root outside any repository, then, once the root has been made a repository, started, and had its `.git` removed, `submit` and `shelve` exit 3. The item file is unchanged after each.
- `a_check_that_leaves_a_file_in_the_root_fails_g5`: a check that `touch`es `left-behind.txt` passes G3 and fails G5 with "The checks left", and no ref is made. A second submit names the file once, under the first look, not as left by the checks.
- `a_check_that_moves_head_fails_g5`: a project check that commits fails G5 with "HEAD moved from".
- `a_shelve_writes_nothing_when_a_project_check_edits_the_item_file`: TR-18's test for `start`, made for an in-progress shelve. Exit 1, and the item stays in progress with the edit kept.
- `show_prints_started_and_each_saved_output_escaped`: `Started:`, `head`, "criterion 1 failed", `commit`, `ref`, both checks with their output, and an escape in a check's output printed as `a\u{1b}[2Kb`.
- `an_open_item_shelves_whatever_the_tree_and_checks_say`: an untracked file and a failing project check don't stop an open item's shelve.

Elsewhere:

- `src/git.rs` `a_header_is_scoped_to_an_item_only_by_its_conventional_commit_scope`: three headers that match, and eleven that don't, such as `feat(BM-1):add search`, `Feat(BM-1): …`, `feat(bm-1): …`, `feat(BM-1)!!: …` and one with a leading space.
- `tests/fixtures/slice-8/XX-19.json`, written by this branch's binary: done, `auto`, with `started.head`, `commit` and `ref`, a criterion check naming criterion 1 and a project check. `slice_8s_xx_19_has_a_commit_and_a_ref_and_comes_back_byte_for_byte` checks that the folder holds only it, its values, the order of `started`'s fields (`at`, `head`, `checks`) and the record's (`by`, `at`, `commit`, `ref`, `checks`), and that it comes back byte for byte with nothing lost on rewrite. The existing fixture tests load and rewrite it too, and slice 7's fixtures still load unchanged.

Not tested: that a second verification moves the ref to the new commit, that `delete` removes a ref named only by `previous_verification`, `status <id> shelved` on a dirty tree (it takes the same `apply` arm as `shelve`), and a repository with no commits, where `started.head` is left out.

### Older tests that changed

`8ba0e95` updated tests whose setup or expected values slice 8 makes untrue. Nothing they check was removed, except the `unchecked` note, which no command writes now:

| Test | Was | Now |
| --- | --- | --- |
| `tests/gates.rs`: `a_manual_criterion_with_no_evidence_fails_g2`, `an_item_with_no_criteria_fails_g2`, `a_script_criterion_needs_no_evidence`, `a_design_naming_a_missing_section_fails_g6`, `gates_lists_every_failure_and_leaves_the_item_file_unchanged` | `gates` of G2, G6, G3 and G4 | G2, G5, G6, G3 and G4. Those that write files in the root call `commit_for` |
| `tests/gates.rs`: `gates_says_when_the_item_isnt_ready_to_submit` | "…: G2, G6, G3 and G4." | "…: G2, G5, G6, G3 and G4." |
| `tests/gates.rs`: `a_design_whose_file_is_a_symlink_outside_the_root_fails_g6`, `a_refused_submit_exits_2_and_changes_nothing`, `a_failing_gates_says_when_the_item_isnt_ready_to_submit_either` | no commit | `commit_for` after writing the root, so G5 passes and the old failure is the only one |
| `tests/states.rs`: `check_unchecked`, used by three tests | `unchecked`, with the note | `check_verified`: `auto`, a `commit`, `refs/tracker/<ID>`, no `note` |
| `tests/states.rs`: `submit_prints_the_shape_section_3_gives_it` | G2, G6, G3 and G4 | with G5 |
| `tests/states.rs`: `the_text_output_names_the_move` | "Unchecked. Submitted before…" | "Verified automatically: every gate passed at <short SHA>, and refs/tracker/XX-1 keeps that commit." |
| `tests/organise.rs`: `by_keeps_done_items_verified_that_way` | `--by unchecked` finds the done item | `--by auto` does |
| `tests/show_and_delete.rs`: `show_prints_every_field_the_verification_records_and_history_with_times` | `unchecked` and its note | `Started:`, `head`, the numbering sentence, `auto`, `commit`, the ref and the check |
| `tests/show_and_delete.rs`: `in_a_terminal_typing_the_id_deletes_an_item_in_each_state` | "is done at <time>, unchecked: no checks ran" | "is done, verified by its gates at <short SHA>", and the ref among what's lost |
| `tests/checks.rs`: `criterion_add_and_set_refuse_a_newer_trackers_file_before_running_a_check` | `started.head` as the newer tracker's field | `started.criteria_hash`, since this version knows `head` |
| `tests/checks.rs`: `a_passing_check_with_an_escaped_process_finishes_soon_and_passes` | `escaped.pid` left in the root | a `.gitignore` for it, so G5's second look passes |
| `tests/item_files.rs`: `lost_on_rewrite_tells_unknown_fields_from_values_written_differently` | `verification.commit` as the unknown field | `verification.triggers` |
| `tests/polish.rs`: `busy_store`, a helper | no commit | `commit_for("XX-3")` before starting it |

`8ba0e95` was committed in the same second as the code, `14:28:20Z`, so these weren't seen failing on their own commit. DESIGN.md §5 says not to edit an existing test just to pass, but to fix a test that's wrong and say so: these are that case. Slice 10's T4 will hold an item for this kind of change.

### Checks done by hand

Each was run with this branch's `target/debug/tracker` in a temporary `TRACKER_HOME`:

| What | How | Result |
| --- | --- | --- |
| The demo above | Run as written, with stderr captured | The output above, with the temporary folder replaced by `$DEMO` |
| Shelving an in-progress item whose project was removed from `config.json` | Start GO-1, write `{"projects":{}}` to the config, then `shelve` and `gates` | `shelve` exits 2: G5 fails, "Can't look at GO-1's work in git: GO isn't in config.json…", and so does G4. In slice 7's check by hand, `shelve` still worked ([slice 7's readme](slice-7-readme.md#checks-done-by-hand)). `gates` fails G5, G3 and G4 |
| Typing the ID into `delete` in a real terminal | Not done | Removing the ref is tested only under `script`, by `a_passing_submits_ref_is_removed_when_the_item_is_deleted`, which returns early, passing, where `script` isn't installed |

### Tracking TR-4

TR-4, a `capability` with `design` `docs/DESIGN.md#2.4`, was created at 2026-11-21T07:07:43Z with slice 7's other items. Its eight criteria are PLAN.md's _Done when_ bullets, with the first and second bullets each split in two, and each check runs only that criterion's tests in `tests/commits.rs`, matching 2, 2, 2, 4, 2, 2, 1 and 2 tests. The slice 7 binary was installed at 14:06Z, before TR-4 started, and tracked it throughout:

| Step | When (UTC) | What |
| --- | --- | --- |
| Tests committed | 14:14:46Z | `89f5797` |
| `tracker start TR-4` | 14:15:10Z | `started.checks`: criteria 1–7 failed, criterion 8 passed, with slice 7's warning. No `head`: slice 7 doesn't write one |
| Code and older tests | 14:28:20Z | `ce56ee2`, `1bc57a9`, `8ba0e95` |
| More tests, fixture, help, DESIGN.md | 14:29:18Z to 14:32:43Z | `e3aa24f`, `5195dd6`, `d6d3664`, `fc890cd`, `4795629` |
| `tracker submit TR-4` | 14:36:26Z | done, `unchecked` |
| PLAN.md: in review | 14:36:31Z | `e5f37b1` |

So:

- **TR-4 is the first item whose checks the installed `tracker` ran**, as slice 7's reviews asked. G3 ran its 8 checks and G4 TR's `cargo fmt --check`, `cargo clippy -- -D warnings` and `cargo test` (52 seconds). Each criterion check's saved output names the tests it ran, such as `test g5_fails_for_an_untracked_file ... ok`.
- **It's still `unchecked`**, with slice 7's note, "Submitted before tracker ran checks against a recorded commit. Gates G2, G6, G3 and G4 passed, but no commit ties the checks to the work." The slice 7 binary has no G5, so nothing checked that the tree was clean or that TR-4 had a commit, no `commit` or `ref` was recorded, and there's no `refs/tracker/TR-4`. A done item's record can't be changed, so it stays that way.
- **Criterion 8 already passed at start.** Its tests check that `submit` writes nothing when a check edits the item file, and slice 7's `lock_unchanged_after_checks` already did that, as [slice 7's readme](slice-7-readme.md#decisions-worth-knowing) says ("The re-read after checks is already here"). So the check can't fail before the work, the start warned, and slice 11's T13 would hold such an item. The criterion can't be reworded, and it still proves what it says.
- `e3aa24f`'s seven tests came after the code, so nothing saw them fail at the time. Six of them fail against `42b0a99`'s code, as [above](#coverage-of-the-done-when-checks) says.
- Every commit in the range is scoped to TR-4. `42b0a99` came 6 seconds after the start, and `e5f37b1` 5 seconds after the submit, so it's one of a done item's commits. Slice 11's T14 counts only unscoped commits, and slice 10 counts its files among TR-4's changed files if TR-4 is ever reopened.
- TR-4's file, written by the slice 7 binary, loads in this branch's binary unchanged: it has no field this slice doesn't know.

## Decisions worth knowing

- **G5 looks at the tree again after the checks** (the [Slice 8 must decide](reviews/slice-7-review-1.md#slice-8-must-decide) lists of slice 7's [review 1](reviews/slice-7-review-1.md) and [review 2](reviews/slice-7-review-2.md#slice-8-must-decide)). A check that leaves a file, or commits, fails this submit rather than the next one, or the next item's shelve. §2.4 says checks leave the tree as they found it, and that a check that needs to write does so outside the root or to an ignored file, as `cargo test` does under `target/`.
- **The second look names only what's new**, by comparing `git::uncommitted`'s lines. A file that was already uncommitted is named once, under G5's first failure. A file already modified that a check changes again isn't named a second time, since its line is the same, but G5 has already failed for it.
- **`verification.commit` is `HEAD` when G5 first looked**, and the second look fails G5 if `HEAD` moved, so it's the commit the checks ran against. §4.3 says so.
- **Shelving needs G5's clean tree and G4, but not G3 or an item commit.** The item's own tests still fail when it's shelved, and an item shelved before any work has nothing to commit. A shelve's G5 message says the next item builds on what's committed, not that the proof must match a commit (`4795629`).
- **Shelving an in-progress item whose project left `config.json` is refused**, since G5 and G4 can't run. In slice 7's check by hand, such a `shelve` still worked. This settles [slice 2's open item 7](slice-2-readme.md#still-open) for shelving: a move that needs the root needs the project.
- **No root, no exit 3.** A root that doesn't exist, or a project missing from `config.json`, fails G5 with a reason, as G3 and G4 fail. Exit 3 is for a root that exists but isn't in a git work tree, and for git that can't run or fails (§4.4).
- **`start` records `HEAD` under the lock, before the checks**, and exits 3 before any check runs if the root isn't in a work tree, so a start that couldn't be proved later isn't recorded. In a repository with no commits, `head` is left out.
- **Item commits are matched by hand, not with `regex`.** [§8](DESIGN.md#8-technology) lists `regex` for this, but the pattern is fixed and `is_item_header` matches it in a few lines, so slice 8 adds no dependency. Slice 12 adds `regex` for a judge's `each`, and could use it here then.
- **G5 lists at most 10 files**, then "N more", since a refusal with hundreds of lines buries the reason. Each is named by its path from the top of the repository, as git prints it.
- **The ref is created under the lock, after the re-read, and only for a submit that passed.** A refused or changed submit leaves none. It's created before the item file is written, so a write that fails then leaves a ref for an item that isn't done. The next verification moves it, and `delete` removes it.
- **`delete` removes the ref first**, before the line in `deleted.json` and the file, so a ref git won't remove leaves the item in place to delete again, as [slice 4's readme](slice-4-readme.md#limits-and-what-later-slices-change) asks. A ref in a root that can't be found is left, and §2.7 now says so.
- **`ref_to_delete` asks git only when a record names a ref**, so deleting an item that was never verified `auto` needs no git, and works for an item whose root has gone.
- **No command writes `unchecked` now.** `UNCHECKED_NOTE` is removed. Records written before this slice keep their note, whichever slice's wording it has, and still load unchanged. `auto` records have no `note`.
- **`show` says a previous record's criteria are numbered as at that submit** (slice 7 review 2's finding 2), next to its heading.
- **Tests read no git config of the developer's**, through `GIT_CONFIG_GLOBAL=/dev/null` and `GIT_CONFIG_NOSYSTEM=1`. `tracker` itself does read the user's git config, except `log.showSignature` and `core.quotePath`, which `git::git` turns off.

## Still open

There are no reviews of this slice yet, so nothing below comes from one.

**This slice**

- **Typing the ID into `delete` in a real terminal hasn't been checked by hand.** Removing the ref is tested only under `script`, as PLAN.md's [integration risks](PLAN.md#integration-risks) say interactive commands are.
- **TR-4 is verified `unchecked`**, with no commit or ref, by the slice 7 binary, and criterion 8's check passed at start ([Tracking TR-4](#tracking-tr-4)). Neither can change now that TR-4 is done.

**Test gaps** (slice 7 review 1's finding 6, carried forward)

- `Env::add_criterion` in [tests/common/mod.rs](../tests/common/mod.rs) still adds `--check true`, left for slice 11.

**Process**

- This slice's binary is to be installed with `cargo install --path .` before TR-5 (slice 9) starts, so slice 9's item is the first that G5 checks and the first that can reach done `auto`.

## Limits, and what later slices change

- **Slice 9** adds triggers after every gate passes. A held submit goes to review, and §2.7 creates the ref only when the item reaches done, so `git::set_ref` must move to the done branch of `apply`, and a held item gets no `commit` or ref until `approve`. `approve` re-runs the gates, G5 and `tree_after_checks` included, so it lets the lock go for the checks, re-reads with `Store::lock_unchanged_after_checks`, then sets the ref and writes `by: "human"` with the commit. `show --diff` can start from `git::item_commits`, which returns SHAs, newest first, and needs a new function for the combined diff. `submit` from in-progress then has two outcomes, so `Run::Submit`'s text and `submit.schema.json`'s "result is always done" change. Slice 9's item, TR-5, is the first TR item G5 checks, so everything under `tracker/`, docs and reviews included, must be committed, scoped to it, before its submit.
- **Slice 10** works out changed files from the item's commits (§2.1): `item_commits` gives the SHAs, and it needs each commit's files, including deleted ones, made relative to the root with `git` run in it, while files outside the root still count toward T7. A scoped commit made after an item is done, such as `e5f37b1`, is one of its commits, and counts if the item is reopened. Changing an existing test, as `8ba0e95` did, fires T4.
- **Slice 11**'s T14 reads `started.head..HEAD` for unscoped commits that touch the root. `started.head` is missing for a repository that had no commits at start, where every commit counts, and for items started before this slice, such as TR-4, which T14 must treat as having nothing to compare rather than failing. `started.criteria_hash` goes before `head` (§4.3), and `item::Started`'s comment says so. Slice 11 also changes `Env::add_criterion`'s `true`, which T13 would hold, and TR-4's criterion 8 is the kind of check T13 holds.
- **Slices 12 and 14**: a judge whose input is `diff` reads the item's commits, and slice 14's G1 refuses to start one before the item has a commit, which `git::item_commits` answers. Slice 12 decides whether judges run before or after `tree_after_checks`. A judge runs in an empty temporary folder, so it can't change the tree either way.
- **Slice 16**'s digest lists each auto-verified item's commit from `verification.commit`, and `audit` finds done items with no record. Records from before this slice are `unchecked`, with no commit or ref, and must still list. `delete`'s warning for an `unchecked` record still says "no checks ran", which isn't true of records slice 7 wrote, such as TR-4's, which ran 11 checks. Slice 16 brings `delete`'s text up to date with the rest.
- **The ref is local.** `refs/tracker/<ID>` isn't pushed unless pushed by hand, so a fresh clone has no refs and a verified commit that a rebase dropped is only in the clone that verified it (§2.7).
- **A subfolder project is checked only under its root** (§7). A change outside the root that the project's tests read isn't caught by G5, and the verified commit may not hold what they read.
- **`item_commits` reads the whole history reachable from `HEAD`** on every G5, as §2.1 defines an item's commits. In a large repository, such as guild-portfolio, that's every commit message, each time.
- **Interrupting `tracker` leaves a check running**, now for shelving too, which §7 says. A process a project check starts outside its group is left running, as slice 7's TR-13 describes.
- **Checks need `sh`, `kill` and `git`**, so `tracker` runs on Unix-like systems with git installed. Without git, `start`, `gates`, `submit` and an in-progress shelve exit 3.

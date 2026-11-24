# Slice 10: Risky changes are held

`tracker` is a command-line work tracker, written in Rust, that keeps one central list of work items across many projects and decides when an item is really done. This document covers **slice 10** of the 16 in [PLAN.md](PLAN.md): what it built, how it works, and how to test it. The earlier readmes cover the rest: [slice 1](slice-1-readme.md) the store, `create` and `list`, [slice 2](slice-2-readme.md) the states and `history`, [slice 3](slice-3-readme.md) priority, kind, labels, `edit` and `list`'s filters, [slice 4](slice-4-readme.md) `show` and `delete`, [slice 5](slice-5-readme.md) bad data and polish, [slice 6](slice-6-readme.md) criteria and gates G1, G2 and G6, [slice 7](slice-7-readme.md) running checks, with gates G3 and G4 and `started.checks`, [slice 8](slice-8-readme.md) G5, the `auto` record and `refs/tracker/<ID>`, and [slice 9](slice-9-readme.md) the inbox, `approve`, `reject` and triggers T6 and T9.

After slice 10 you can **have new capabilities, new dependencies, risky paths, edited tests, big diffs and design changes held**. The spec is [DESIGN.md](DESIGN.md). This slice implements an item's changed files and source files from [§2.1](DESIGN.md#21-items), triggers T1, T2, T3, T4, T7 and T8, with "How T1, T2, T3, T4, T7 and T8 read the changes", from [§2.5](DESIGN.md#25-review-triggers-does-it-need-a-human), `approve`'s second run of the triggers from [§2.7](DESIGN.md#27-human-actions), the `review` settings and their defaults from [§4.2](DESIGN.md#42-config), and `files` in a hold, from [§4.3](DESIGN.md#43-item-file). `show --diff` listing the files that fired a trigger first is in [§3](DESIGN.md#3-interface)'s table. It's tracked in `tracker` itself as item TR-6, with the binary installed from slice 9 ([Tracking TR-6](#tracking-tr-6)). [Review 1](reviews/slice-10-review-1.md)'s fixes are items TR-31 to TR-36 ([Tracking review 1's fixes](#tracking-review-1s-fixes)). [Review 2](reviews/slice-10-review-2.md)'s fixes are items TR-37 and TR-38 ([Tracking review 2's fixes](#tracking-review-2s-fixes)).

| | |
| --- | --- |
| Status | Every _Done when_ check has a test, and every test passes, as does the test command. TR-6 is `done`, verified `auto` at `e650ecf` by the installed slice 9 binary, which ran G2, G5, G6, G3 and G4, then T6 and T9, and held nothing, with `refs/tracker/TR-6` pointing at that commit. All seven of its checks failed at `start`, as they should. The builder added criteria 6 and 7 to PLAN.md's five, and every one has a check ([Tracking TR-6](#tracking-tr-6)). `b7ba4ee` changed six existing tests before `start`, because their pinned trigger lists grew ([Older tests that changed](#older-tests-that-changed)). From [review 1](reviews/slice-10-review-1.md), bug 1, bug 2, security 1 and 2, design 1, 2 and 3 and test gap 1 are fixed, as TR-31 to TR-36. Each is `done`, verified `auto` by the installed slice 9 binary with no holds, with a commit and `refs/tracker/<ID>`, and every test written for them was seen failing first, apart from two that passed at `start` as they had to ([Tracking review 1's fixes](#tracking-review-1s-fixes)). Bug 3, design 4 and test gap 2 won't be fixed, and process 2 is done in this readme ([Fixes after review 1](#fixes-after-review-1)). The review's "Slice 11 must decide" points are left for slice 11 (TR-7). From [review 2](reviews/slice-10-review-2.md), bug 1 is fixed as TR-37 (an unknown key under a project exits 3), bug 2 and designs 1 and 2 as TR-38 (DESIGN.md only), and security 2 in this readme, which no longer says tests committed before `start` "can't be weakened" after it, and tells the user to read a "before start" T4 mark's edit in `show --diff` too. TR-37 and TR-38 are `done`, verified `auto` by the installed slice 9 binary with no holds, with a commit and `refs/tracker/<ID>`. `tr_37_c1` and TR-38's three `grep` checks failed at `start`, as they should, and `tr_37_c2` passed, as it had to, and was seen failing with `deny_unknown_fields` but no `judge` field ([Tracking review 2's fixes](#tracking-review-2s-fixes)). Bugs 3 and 4 and test gap 1 won't be fixed. Security 1 (b) is left to slice 11's T14, and TR-7's commits get read by hand at its `approve` ([Fixes after review 2](#fixes-after-review-2)). Not done: review 1's process 1, `approve`'s new question, which names the `HEAD`, marks each hold "(new since the submit)" and lists a hold that "no longer fires", hasn't been seen in a real terminal, and it's the user's to check, as is review 2's process 1, TR's own `review.risky_paths`. Review 2's "Slice 11 must decide" points are left for TR-7. The slice 9 check by hand predates both changes ([Checks done by hand](#checks-done-by-hand)). One gap is still open in DESIGN.md §7: how T7 counts ([Still open](#still-open)). PLAN.md has slice 10 as `in review`. The branch isn't merged into `main`, and neither are slices 7, 8 and 9, which it's stacked on. |
| Branch | `feat/tracker/slice-10`, stacked on `feat/tracker/slice-9` at `71826d8`. Slice 9 isn't merged into `main`, so this slice's commits are `git log --oneline --reverse feat/tracker/slice-9..feat/tracker/slice-10`. In order: `b7ba4ee` (TR-6's tests, fixture XX-22 and six older tests' expected values, committed before `tracker start TR-6`), `6955644` (PLAN.md: slice 10 in progress), `2a4d5df` (the triggers, the config, `git::changes`, the schemas and `approve`), `9475fb3` (help text), `e650ecf` (DESIGN.md), `2c9da0b` (PLAN.md: slice 10 in review), `ed6d12a` (slice 9's check by hand, which isn't this slice's work), `bdd3e71` (this readme's first version) and `c473b57` (review 1). Then review 1's fixes, `3363871` to `89069f3` ([below](#fixes-after-review-1)), `6976f54` (this readme's first update), `55c6064` (review 1's Outcome) and `3a63ca8` (review 2). Then review 2's fixes: `d6dfc5a` and `469f8bb` (TR-37) and `daa5047` (TR-38) ([below](#fixes-after-review-2)). That is 26 commits since slice 9, `ed6d12a` among them. This readme's second update and review 2's Outcome come after them |
| Test command | `cargo fmt --check && cargo clippy -- -D warnings && cargo test`, run in `tracker/`. All three pass. |
| Tests | 384: 46 in `tests/risky_changes.rs`, 39 in `tests/create_and_list.rs`, 36 in `tests/commits.rs`, 34 in `tests/organise.rs`, 28 in `tests/review.rs`, 27 in `tests/states.rs`, 25 in `tests/show_and_delete.rs`, 23 in `tests/criteria.rs`, 22 in `tests/gates.rs`, 18 in `tests/checks.rs`, 14 in `tests/item_files.rs`, 13 in `tests/bad_data.rs`, 13 in `tests/fixtures.rs`, 11 in `tests/docs_links.rs`, 9 in `tests/polish.rs`, 6 in `tests/output.rs`, 5 in `tests/schemas.rs`, 4 in `tests/store_writes.rs`, 2 in `tests/item_ids.rs`, 7 unit tests in the library (4 in `src/gates.rs`, 1 each in `src/store.rs`, `src/checks.rs` and `src/git.rs`) and 2 in `src/main.rs` |

## Contents

- [What's in this slice](#whats-in-this-slice)
- [Try it](#try-it)
- [Commands](#commands)
- [The review settings](#the-review-settings)
- [How it works](#how-it-works)
- [JSON output](#json-output)
- [Code tour](#code-tour)
- [Testing](#testing)
- [Decisions worth knowing](#decisions-worth-knowing)
- [Fixes after review 1](#fixes-after-review-1)
- [Fixes after review 2](#fixes-after-review-2)
- [Still open](#still-open)
- [Limits, and what later slices change](#limits-and-what-later-slices-change)

## What's in this slice

**Built:**

- **Six triggers**, which run on `submit` and `status <id> done` once every gate has passed, beside slice 9's T6 and T9. Each message says why, and the file triggers name their files:
  - **T1** fires when the item's kind is `capability`, or an item commit added a source file: "new source file: src/search.rs".
  - **T2** fires when a changed file matches `risky_paths`: "risky file changed: src/store.rs".
  - **T3** fires when a dependency manifest or lockfile changed: "dependency manifest changed: Cargo.toml".
  - **T4** fires when an existing test file had a line changed or removed, or was deleted, and names each file's commits, marked before or after `start`: "existing test file modified: tests/sums.rs (b7ba4ee before start, 1a2b3c4 after start)".
  - **T7** fires when the item's commits changed more lines than `max_diff_lines`: "34 lines changed, more than the project's max_diff_lines of 20".
  - **T8** fires when the project's `design` file changed: "the project's design doc changed: docs/DESIGN.md".
- **The item's changed files**, worked out from its commits as §2.1 defines them: every file those commits touched, deleted ones included, each commit compared with its first parent, with paths made relative to the project's root before they're matched against a glob, whatever `diff.relative` says. A file outside the root counts toward T7 and matches no glob. For a commit made after `start`, a test file is existing if it exists at `started.head`, so an item that commits its tests before `start` fires T4 if it changes or removes a line of them, or deletes them, after it. Lines it adds to switch a test off, such as `#[ignore]`, only add lines, so T4 doesn't see them (§2.5).
- **The project's `review` settings** in `config.json`: `risky_paths`, `source_paths`, `test_paths`, `manifests`, `max_diff_lines` and `disabled`, each with its default ([below](#the-review-settings)). `disabled` never switches off T6, T10 or T12. A glob that can't be parsed, or a key that isn't one of those settings or `digest`, makes every command exit 3, and so does a key directly under a project that isn't one of §4.2's (`name`, `root`, `design`, `checks`, `check_timeout_secs`, `review` and `judge`).
- **`files` in a hold**: a hold from a file trigger lists the files that fired it, relative to the root, in `holds` and in `verification.triggers`.
- **`show --diff`** prints "Files that fired a trigger:", each as the trigger and the file, before the commits and the diff, from the item's `holds`, or from a done item's approved `verification.triggers`.
- **`approve` runs the triggers again** at the `HEAD` it names, before it asks. Its question names that `HEAD` and every hold that fires there, marking each one that isn't among the submit's "(new since the submit)", and lists each hold of the submit's that no longer fires, marked "(no longer fires)". If G5's `HEAD` then isn't the one the question named, it writes nothing and exits 1. The holds it showed go in `verification.triggers` (slice 9 review 1's design 1).
- **A hold's `files` in the schemas**: `submit.schema.json` and `gates.schema.json` now take their `holds` from one definition in `item.schema.json`.

**Not built yet:**

- **T5, T13 and T14** (slice 11). A check that passed at start, a criterion changed after start and an unscoped commit since start still hold nothing.
- **T10, T11 and T12** (slices 12, 14 and 15), the judge triggers, and `approve` counting held judge questions as passed (slice 15). `approve` runs no judges, since there are none yet.
- **The digest** (slice 16), which lists every `approve`, `reject` and edit that weakens T1 or T9, and `review`'s flag for an overdue digest.
- **A DESIGN.md §7 line for how T7 counts** ([Still open](#still-open)). §7's other two lines from review 1, for a `-diff` attribute and an unscoped commit before `start`, are written.
- **The `digest` and `judge` settings' use** (slices 16 and 12). `review.digest` and a project's `judge` are accepted and kept as written, and nothing reads them.

## Try it

Always set `TRACKER_HOME` while developing. Without it, the tool uses your real store at `~/.tracker`.

The two projects, BM and IM, are the folders `bm` and `im` inside one throwaway repository, as TR is a folder inside guild-portfolio. BM has the usual files: a `src/` folder, a `tests/` folder, a `Cargo.toml` and a design doc. The demo's commits use a made-up name, so they work whatever your own git settings say. It edits `config.json` with `jq`, as a user would by hand.

```sh
cd guild-portfolio/tracker
cargo build

export DEMO=$(mktemp -d)
export TRACKER_HOME=$DEMO/store
export PATH="$PWD/target/debug:$PATH"
export GIT_AUTHOR_NAME=demo GIT_AUTHOR_EMAIL=demo@example.invalid
export GIT_COMMITTER_NAME=demo GIT_COMMITTER_EMAIL=demo@example.invalid
mkdir -p $DEMO/repo/bm/src $DEMO/repo/bm/tests $DEMO/repo/bm/docs $DEMO/repo/im/tests $DEMO/repo/im/docs && cd $DEMO/repo
git init --quiet -b main
printf 'pub fn all() {}\n' > bm/src/lib.rs
printf 'pub fn save() {}\n' > bm/src/store.rs
printf '#[test]\nfn adds() { assert_eq!(1 + 1, 2); }\n' > bm/tests/sums.rs
printf '[package]\nname = "bm"\n' > bm/Cargo.toml
printf '# Design\n\n## 1. Search\n\nBy title.\n' > bm/docs/DESIGN.md
printf '#[test]\nfn adds() { assert_eq!(1 + 1, 2); }\n' > im/tests/sums.rs
printf '# Design\n\n## 1. Import\n\nFrom a file.\n' > im/docs/DESIGN.md
git add -A && git commit --quiet -m "chore: set up"
cd bm
```

BM-1 does one commit's work that every file trigger notices: a new source file, a risky file, a manifest, an edited test line, a design change and more than BM's `max_diff_lines`. `gates` says what a submit would be held for, `submit` holds it, and `show --diff` lists the files first. IM-1 changes a design doc and adds a test case, but IM switches T8 off, and adding a test case doesn't fire T4. A bad glob is refused with exit 3:

```sh
tracker init
tracker project add BM --name bookmarks --root . --design docs/DESIGN.md
tracker project add IM --name importer --root ../im --design docs/DESIGN.md
jq '.projects.BM.review = {risky_paths: ["src/store.rs"], max_diff_lines: 20}' $TRACKER_HOME/config.json > $DEMO/c.json && mv $DEMO/c.json $TRACKER_HOME/config.json
tracker create "Search filters bookmarks by tag" --done-when "A search for a tag lists only tagged bookmarks"
tracker criterion set BM-1 1 --check 'test -f src/search.rs'
tracker start BM-1
seq 1 25 | sed 's/.*/pub fn tag_&() {}/' > src/search.rs
printf 'pub fn save() { tag(); }\n' > src/store.rs
printf '[package]\nname = "bm"\n\n[dependencies]\nfuse = "1"\n' > Cargo.toml
printf '#[test]\nfn adds() { assert!(true); }\n' > tests/sums.rs
printf '# Design\n\n## 1. Search\n\nBy title and by tag.\n' > docs/DESIGN.md
git add -A && git commit --quiet -m "feat(BM-1): search by tag"
tracker gates BM-1
tracker submit BM-1
tracker show BM-1 --diff | grep -A8 'Files that fired'
echo BM-1 | tracker approve BM-1      # exit 1: not a terminal
tracker create "Import bookmarks from a file" --project IM --done-when "A file of URLs becomes bookmarks"
tracker criterion set IM-1 1 --check 'test -f import.txt'
jq '.projects.IM.review = {disabled: ["T8", "T99"]}' $TRACKER_HOME/config.json > $DEMO/c.json && mv $DEMO/c.json $TRACKER_HOME/config.json
cd ../im
tracker start IM-1
touch import.txt && printf '#[test]\nfn adds() { assert_eq!(1 + 1, 2); }\n\n#[test]\nfn imports() {}\n' > tests/sums.rs && printf '# Design\n\n## 1. Import\n\nFrom a file, or a URL.\n' > docs/DESIGN.md
git add -A && git commit --quiet -m "feat(IM-1): import from a file"
tracker submit IM-1 --json | jq -c '{result, status, triggers: [.triggers[].trigger], holds}'
jq '.projects.IM.review.test_paths = ["tests/[x"]' $TRACKER_HOME/config.json > $DEMO/c.json && cp $TRACKER_HOME/config.json $DEMO/good.json && mv $DEMO/c.json $TRACKER_HOME/config.json
tracker list --all      # exit 3: a bad glob
cp $DEMO/good.json $TRACKER_HOME/config.json
tracker list --all --status open,review,done
jq '.holds' $TRACKER_HOME/items/BM-1.json
```

The output, from `tracker init` on, with stdout and stderr together:

```
$ tracker init
Created the store at $DEMO/store.
$ tracker project add BM --name bookmarks --root . --design docs/DESIGN.md
Registered BM (bookmarks) at /private$DEMO/repo/bm.
$ tracker project add IM --name importer --root ../im --design docs/DESIGN.md
Registered IM (importer) at /private$DEMO/repo/im.
$ jq '.projects.BM.review = {risky_paths: ["src/store.rs"], max_diff_lines: 20}' $TRACKER_HOME/config.json > $DEMO/c.json && mv $DEMO/c.json $TRACKER_HOME/config.json
$ tracker create "Search filters bookmarks by tag" --done-when "A search for a tag lists only tagged bookmarks"
Created BM-1: Search filters bookmarks by tag
1 manual criterion: give it a check with `tracker criterion set BM-1 1 --check "…"`.
$ tracker criterion set BM-1 1 --check 'test -f src/search.rs'
Criterion 1 of BM-1 now has a check.
$ tracker start BM-1
Running criterion 1's check: `test -f src/search.rs`
BM-1: open → in-progress
$ seq 1 25 | sed 's/.*/pub fn tag_&() {}/' > src/search.rs
$ printf 'pub fn save() { tag(); }\n' > src/store.rs
$ printf '[package]\nname = "bm"\n\n[dependencies]\nfuse = "1"\n' > Cargo.toml
$ printf '#[test]\nfn adds() { assert!(true); }\n' > tests/sums.rs
$ printf '# Design\n\n## 1. Search\n\nBy title and by tag.\n' > docs/DESIGN.md
$ git add -A && git commit --quiet -m "feat(BM-1): search by tag"
$ tracker gates BM-1
Running criterion 1's check: `test -f src/search.rs`
BM-1 passes the gates submit runs: G2, G5, G6, G3 and G4.
A submit would be held for review:
  T1  new source file: src/search.rs
  T2  risky file changed: src/store.rs
  T3  dependency manifest changed: Cargo.toml
  T4  existing test file modified: tests/sums.rs (bec83f9 after start)
  T7  34 lines changed, more than the project's max_diff_lines of 20
  T8  the project's design doc changed: docs/DESIGN.md
$ tracker submit BM-1
Running criterion 1's check: `test -f src/search.rs`
BM-1: in-progress → review
Every gate passed, and BM-1 is held for review:
  T1  new source file: src/search.rs
  T2  risky file changed: src/store.rs
  T3  dependency manifest changed: Cargo.toml
  T4  existing test file modified: tests/sums.rs (bec83f9 after start)
  T7  34 lines changed, more than the project's max_diff_lines of 20
  T8  the project's design doc changed: docs/DESIGN.md
It waits in `tracker review` for a human to approve or reject it. `tracker show BM-1 --diff` shows its work.
$ tracker show BM-1 --diff | grep -A8 'Files that fired'
Files that fired a trigger:
  T1  src/search.rs
  T2  src/store.rs
  T3  Cargo.toml
  T4  tests/sums.rs
  T8  docs/DESIGN.md

Commits, newest first (1):
  bec83f9  feat(BM-1): search by tag
$ echo BM-1 | tracker approve BM-1      # exit 1: not a terminal
error: `tracker approve BM-1` needs a human at an interactive terminal, who types BM-1 to confirm it, and stdin here isn't a terminal. Nothing was changed. Only a human approves a held item. It waits in `tracker review` until one approves or rejects it.
$ tracker create "Import bookmarks from a file" --project IM --done-when "A file of URLs becomes bookmarks"
Created IM-1: Import bookmarks from a file
1 manual criterion: give it a check with `tracker criterion set IM-1 1 --check "…"`.
$ tracker criterion set IM-1 1 --check 'test -f import.txt'
Criterion 1 of IM-1 now has a check.
$ jq '.projects.IM.review = {disabled: ["T8", "T99"]}' $TRACKER_HOME/config.json > $DEMO/c.json && mv $DEMO/c.json $TRACKER_HOME/config.json
$ cd ../im
$ tracker start IM-1
Running criterion 1's check: `test -f import.txt`
IM-1: open → in-progress
$ touch import.txt && printf '#[test]\nfn adds() { assert_eq!(1 + 1, 2); }\n\n#[test]\nfn imports() {}\n' > tests/sums.rs && printf '# Design\n\n## 1. Import\n\nFrom a file, or a URL.\n' > docs/DESIGN.md
$ git add -A && git commit --quiet -m "feat(IM-1): import from a file"
$ tracker submit IM-1 --json | jq -c '{result, status, triggers: [.triggers[].trigger], holds}'
Running criterion 1's check: `test -f import.txt`
{"result":"done","status":"done","triggers":["T1","T2","T3","T4","T6","T7","T9"],"holds":[]}
$ jq '.projects.IM.review.test_paths = ["tests/[x"]' $TRACKER_HOME/config.json > $DEMO/c.json && cp $TRACKER_HOME/config.json $DEMO/good.json && mv $DEMO/c.json $TRACKER_HOME/config.json
$ tracker list --all      # exit 3: a bad glob
error: $DEMO/store/config.json gives IM a review setting that can't be used: "tests/[x" isn't a valid glob: error parsing glob 'tests/[x': unclosed character class; missing ']'. Fix it there.
$ cp $DEMO/good.json $TRACKER_HOME/config.json
$ tracker list --all --status open,review,done
  ID    PRIORITY  KIND    STATUS  TITLE
BM  bookmarks
  BM-1  medium    change  review  Search filters bookmarks by tag
IM  importer
  IM-1  medium    change  done    Import bookmarks from a file

Every project.
Filters: status open, review or done.
$ jq '.holds' $TRACKER_HOME/items/BM-1.json
[
  {
    "trigger": "T1",
    "message": "new source file: src/search.rs",
    "files": [
      "src/search.rs"
    ]
  },
  {
    "trigger": "T2",
    "message": "risky file changed: src/store.rs",
    "files": [
      "src/store.rs"
    ]
  },
  {
    "trigger": "T3",
    "message": "dependency manifest changed: Cargo.toml",
    "files": [
      "Cargo.toml"
    ]
  },
  {
    "trigger": "T4",
    "message": "existing test file modified: tests/sums.rs (bec83f9 after start)",
    "files": [
      "tests/sums.rs"
    ]
  },
  {
    "trigger": "T7",
    "message": "34 lines changed, more than the project's max_diff_lines of 20"
  },
  {
    "trigger": "T8",
    "message": "the project's design doc changed: docs/DESIGN.md",
    "files": [
      "docs/DESIGN.md"
    ]
  }
]
```

Notes on the output:

- The commit's SHA, and the times in an item file, differ on every run. T4's hold names the SHA of BM-1's commit, which changed `tests/sums.rs` after `start`, so it differs too. A test file changed in a commit before `start` would read "(… before start)", and one changed on both sides names both.
- On macOS, `project add` prints the root with `/private` in front of `$DEMO`, since it stores the path with symlinks followed. The error for the bad glob names `config.json` by its path in the store.
- BM-1's 34 lines are the 25 of `src/search.rs` and the 9 other lines the commit added or removed. `max_diff_lines` is 20 for BM, set by hand.
- T7 has no file in its hold, so `show --diff` lists five files, one each for T1, T2, T3, T4 and T8, and the T7 hold in `holds` has no `files`.
- `gates` runs the triggers once every gate passes and prints the same holds a submit gets, as it has since slice 9, but now with eight triggers to run. Its criterion check runs again, as `submit`'s does.
- IM-1's JSON lists `T1, T2, T3, T4, T6, T7, T9` in `triggers`. T8 is missing because IM switches it off, and `T99` in the same `disabled` list is ignored, since it names no trigger. T4 ran and stayed quiet: the commit added a test case to `tests/sums.rs` and removed no line.
- `echo BM-1 | tracker approve BM-1` is refused as in slice 9. The demo can't show `approve` going ahead, since it needs a terminal. Typed at one, it now runs the triggers again, then prints what holds BM-1 at `HEAD`, naming that commit and marking any hold that wasn't in the submit's "(new since the submit)", before it asks for the ID. The tests drive that under `script` ([Testing](#testing)), and it hasn't been seen in a real terminal ([Still open](#still-open)).
- History says `"interactive": false` because the demo ran from a script. Typed at a terminal, it's `true`.
- The last command prints BM-1's `holds`, which are in its file just before `history`. Each has `files` but T7's. The file has no `verification`.

## Commands

| Command | What it does |
| --- | --- |
| `tracker submit <id>`, `status <id> done` | After the gates pass, T1, T2, T3, T4, T6, T7, T8 and T9 run, in that order, except any the project's `review.disabled` names (T6 always runs). If one fires, in-progress → review with `holds`, each with its `files` if it's about files, and no record or ref. Otherwise in-progress → done, `auto`. Exit 3 if the project's root can't be found or git can't say what the item's commits changed. |
| `tracker gates <id>` | Once every gate passes, it runs the same triggers and lists the holds a submit would get, with `triggers` listing only those that ran. Nothing is written. |
| `tracker approve <id>` | Changed. Before it asks, it runs the triggers again at `HEAD`, and exits 3 for a root that isn't in a git work tree. The question names `HEAD` and each hold, marking each new since the submit, and lists a hold of the submit's that no longer fires as "(no longer fires)". With none firing, it says "and nothing holds it now". After the gates it exits 1, writing nothing, if G5 saw a different `HEAD`. The record's `triggers` are the holds shown. |
| `tracker show <id> --diff` | Changed. "Files that fired a trigger:" comes first, one line each for a trigger and file, then the commits and the diff. Left out when no hold names a file. |

The help text for `submit` names the eight triggers and says the project's review settings tune them, and `approve`'s says it runs the triggers again at `HEAD` and exits 1 if `HEAD` moved while it asked.

Exit codes in this slice, as §3 defines them:

- `0`, `1` and `2`: as in slice 9, plus `approve` exiting 1 when `HEAD` moved while it waited.
- `3`: as before, plus any command, when a glob in a project's `review` settings can't be parsed, or the settings have a key that isn't one of §4.2's. Also `submit`, `status <id> done`, `gates` and `approve`, when the project's root can't be found or git can't list what the item's commits changed, and `approve` when its root isn't in a git work tree.

## The review settings

A project's `review` object in `config.json` ([§4.2](DESIGN.md#42-config)) is set by hand. `project add` doesn't write one, and a project without one takes every default. Each setting left out takes its own:

| Setting | Default | Used by |
| --- | --- | --- |
| `risky_paths` | none | T2 |
| `source_paths` | `["src/**"]`, minus anything `test_paths` matches | T1 |
| `test_paths` | `["tests/**"]` | T4, and what `source_paths` leaves out |
| `manifests` | `**/Cargo.toml`, `**/Cargo.lock`, `**/package.json`, `**/package-lock.json`, `**/npm-shrinkwrap.json`, `**/yarn.lock`, `**/pnpm-lock.yaml`, `**/go.mod`, `**/go.sum`, `**/pyproject.toml`, `**/requirements.txt`, `**/Gemfile`, `**/Gemfile.lock` | T3 |
| `max_diff_lines` | 300 | T7 |
| `disabled` | none | every trigger but T6, T10 and T12 |
| `digest` | none | slice 16. Kept as written, such as `{"every_days": 7}`, and read by nothing yet |

The defaults are the constants in [src/config.rs](../src/config.rs), and `Review`'s methods return them. A setting that's present replaces its default, it doesn't add to it. Each glob is matched against the changed file's path relative to the project's root, and `*` doesn't match `/`, so `*.js` names only files at the top of the root and `**` matches any depth. `config::glob_set` builds a set with `literal_separator` on, which gives that rule. `disabled` is read by `Review::runs`: a name that isn't a trigger's is ignored, so a typo like `"T99"` silences nothing and says nothing. A *key* that isn't in the table is refused: `Review` has `deny_unknown_fields`, so `"risky_path"` makes every command exit 3 with serde's "unknown field `risky_path`, expected one of …", rather than leaving T2 on its default of never firing. `digest` is a `serde_json::Value`, since slice 16 hasn't said what it holds.

## How it works

### The item's changed files

`triggers::changes(root, id, started)` in [src/triggers.rs](../src/triggers.rs) returns a `Changes`, which every file trigger reads. `started` is the item's `started.head`, or `None` for an item that never started. The item's commits come from `git::item_commits`, the same match as G5's, oldest first. `git::changes` in [src/git.rs](../src/git.rs) runs one `git show` over them with `-z --no-renames --diff-merges=first-parent --no-ext-diff --no-textconv --no-relative --raw --numstat` and `log.showRoot=true`, and returns each commit's `FileChange`s: the commit's SHA, the path from the top of the repository, git's status letter, and the lines added and removed, or `None` for a binary file. `--no-relative` stops `diff.relative` in `.git/config` from leaving out the files outside the folder `git` runs in, which would fail the root's prefix strip for every path. `git::patches`, which `show --diff` uses, has it too. A rename is a delete and an add, a merge commit is compared with its first parent, and a root commit with nothing. `git::prefix` gives the root's path from the top of the repository, such as `tracker/`, which `changes` strips from every path. A file that doesn't start with it is outside the root and counts only toward `Changes::lines`.

`changes` also asks `git::since(started)` for the commits `HEAD` reaches that `started` doesn't, the item's commits after `start`. For each change inside the root, in commit order:

1. `lines` adds the file's added and removed lines. This runs before the root check, so a file outside the root counts toward T7.
2. Whether the file is **existing** depends on the commit. For a commit after `start`, it's existing if `git::exists_at(started, path)` says the file is there at `started.head`, whoever added it. For a commit before `start`, or with no `started.head`, it's existing if no earlier item commit has put it in `added`.
3. `A` puts the path in `added`. A path in `added` isn't existing for a commit before `start`.
4. `D` on an existing file adds an `Edit` to `existing_changed` as deleted. Any other status on an existing file adds one as modified if it removed at least one line, or if it's binary. A changed line is a removed line and an added one, so it counts, and a file that only gained lines doesn't. An `Edit` is one commit's change to one file: its path, whether it deleted it, the commit's 7-character SHA, and whether the commit was after `start`.
5. The path goes in `files`, whichever of those happened.

So a test file the item's own `test(XX-1)` commit added before `start` is existing for every commit after it, and changing or removing a line of it fires T4. Lines added to switch it off, such as `#[ignore]` above a test or `return;` as its first line, only add lines, so T4 doesn't fire. §2.5 says so, and leaves it to slice 16's digest. A test file the item adds after `start` isn't, and T4 stays quiet while it's edited, since it's new work. An unscoped commit before `start` that deletes an existing test, which the item's own commit then adds back, isn't caught: the item's own commit adds a file that no longer exists, so it's new, and §7 says so. A file that an item commit added and a later one deleted stays in `added`, so T1 would still list it, which no test covers ([Still open](#still-open)).

### The triggers

`triggers::run(item, config)` finds the project's `review` settings and `design`, calls `gates::root` for the root, builds `Globs` from the four glob lists, calls `changes`, and runs each trigger in order, T1, T2, T3, T4, T6, T7, T8, T9, adding a `Ran` only when `Review::runs` says it's on. A `Ran` now has `files` beside `hold`. `run` passes the item's `started.head` to `changes`, and returns a `Result`, since a root that can't be found, or git failing, exits 3, as does a glob that can't be built. `holds` copies each fired `Ran`'s `files` into the `Hold`.

- `t1` collects `added` paths that `Globs::is_source` accepts, which is a match on `source_paths` and none on `test_paths`. A capability gets "it's a capability, so a human should check the design fit", or "it's a capability, and adds the source file …" when it added any. Any other kind gets "new source file: …" for one or more files. A new source file includes a new top-level module, since any added source file counts, so the code has no separate rule for one.
- `t2` collects `files` that match `risky_paths`. With no `risky_paths` it never fires.
- `t3` collects `files` that match `manifests`.
- `t4` collects `existing_changed` paths that match `test_paths`, then says "existing test file modified: …", "existing test file deleted: …", or both, joined by "; ". A file both modified and deleted is listed as deleted. Each file reads `tests/old.rs (b7ba4ee before start, 1a2b3c4 after start)`, with one entry for every commit that changed it, in commit order, and its `named` closure builds that from `existing_changed`. The hold's `files` stay plain paths. An item whose test commit updates a pinned value before `start` is still held, but the message says "before start", which says when the edit was made. It doesn't say the edit was right: a `test(XX-1)` commit made before `start` can weaken a test in the same commit that updates a pinned list, and gets the same mark, so the edit is still worth reading in `show --diff` whichever side of `start` it's on (§2.5). A commit counts as "after start" when `started.head` can't reach it, as `git::since` lists them, so a rebase or an amend after `start` gives every rewritten commit a new SHA and marks it all "after start".
- `t7` compares `Changes::lines` with `max_diff_lines`, and fires only for more. A hold has no `files`.
- `t8` takes the project's `design`, drops any `#section` and a leading `./`, and fires if that path is among `files`. A project with no `design` never fires it.
- Each message names at most `git::LISTED_FILES` files and then "N more", through `file_list`, which G5's messages use too. The hold's `files` has every file.

T6 and T9 are as in slice 9, and read only the item.

### A submit that's held

`apply` in [src/main.rs](../src/main.rs) is as slice 9 describes, with one change: `triggers::run` takes the config and can fail, so a project whose root can't be found exits 3 after the gates passed. The triggers run once, after G5's second look, under the lock, and a submit that no trigger holds creates its ref as before. `gates_dry_run` makes the same call.

### Approve runs the triggers

`approve` in [src/main.rs](../src/main.rs) reads the item and its file's text, and refuses a newer file or a move the diagram doesn't draw, as in slice 9. Then, before `tty::confirm`:

1. It loads the config and finds the root with `gates::root`, exiting 3 with "Can't approve XX-1: …. Nothing was changed." if it can't. `git::require_work_tree` exits 3 if the root isn't in a git work tree, so this check comes before any question.
2. `git::head(root)` is the `HEAD` the question names.
3. `triggers::run` and `triggers::holds` work out the holds at that `HEAD`. These are the item's commits as they are now, so a commit made since the submit counts.
4. `decision_warning(item, holds, what)` prints each hold and marks it "(new since the submit)" when `item.holds` doesn't contain it. A hold with the same trigger and message but different `files` counts as new, since `Hold` compares all three fields. `reject` calls it with `item.holds`, so nothing it prints is marked. After the holds that fire, it lists each hold in `item.holds` whose trigger isn't among them, as `  T3  <message> (no longer fires)`, such as one whose trigger was switched off in `review.disabled` while the item waited. A hold whose trigger still fires, with another message or other `files`, isn't listed as gone: it shows as new. When nothing fires, the first line ends "and nothing holds it now." rather than saying no trigger was recorded. The question's second line says approving "runs the gates again at <short HEAD>".

Then, as in slice 9, `Store::lock_unchanged`, the statics under the lock, the scripts and G5's second look without it, and the re-read. A failed gate exits 2. If none failed, one more check comes before anything is written: if `report.head`, the `HEAD` G5 saw, isn't the `HEAD` the question named, `approve` exits 1 with "HEAD moved from … to … while XX-1's approval waited … Nothing was changed. Run `tracker approve XX-1` again to see them at the new HEAD." Otherwise `set_ref_at_head`, and `item.holds` is set to the holds that were shown, so `verification_record` copies them into `triggers`, and the move to done empties `holds` as before.

So a commit made while I read the question is never approved unseen. A commit made before it is shown, and its holds are marked.

### Show --diff lists the files first

`show` in [src/main.rs](../src/main.rs) calls `trigger_files(item)` when `--diff` is given. It reads the item's `holds`, or, when there are none, the `triggers` of its verification record, so a done item that a human approved still lists them. Each file in each hold's `files` is printed as `  T3  Cargo.toml`, through `text::for_terminal`, under "Files that fired a trigger:", before "Commits," and "Diff,". A hold with no `files`, such as T7's, or one written by slice 9, adds nothing, and with none the section is left out. The JSON is unchanged: the item is in `item`, so `files` are in its holds.

### The config

`config::load` in [src/config.rs](../src/config.rs) builds each project's four glob lists once it has parsed the file, so a glob that can't be built exits 3 with the project key and the glob, "… gives IM a review setting that can't be used: "tests/[x" isn't a valid glob: … Fix it there.", whichever command was run. This is the same refusal a bad root gets, so a trigger is never unable to run at a submit. `Project::review` returns the settings or the defaults. `Review` refuses a key it doesn't know (`deny_unknown_fields`), so `config::load` fails on it with serde's message, line and column, as for any other bad part of the file, and exit 3. `Project::review` is `None` until set, so a `config.json` without it comes back as it was. `Project` refuses a key it doesn't know too (`deny_unknown_fields`), so `"risky_paths"` written beside `checks` rather than inside `review`, or `"reveiw"`, exits 3 naming the key (review 2's bug 1), where it used to load with every setting on its default. `judge: Option<serde_json::Value>`, which slice 12 will read, is kept as written, with a serde default and `skip_serializing_if`, as `Review::digest` is. `Config` itself, the file's top level, still skips unknown keys, so slice 12's global `judge` object, which §4.2 puts beside `projects`, has no field to need.

## JSON output

| Command | Schema | What changed |
| --- | --- | --- |
| `submit`, `status <id> done`, `gates` | [submit.schema.json](../schemas/submit.schema.json), [gates.schema.json](../schemas/gates.schema.json) | `triggers` lists each trigger that ran, T1, T2, T3, T4, T6, T7, T8, T9, minus any `review.disabled` names. `holds`' items are now `item.schema.json`'s `hold`, so they may have `files` |
| every item | [item.schema.json](../schemas/item.schema.json) | `hold` gains `files`: an array of at least one non-empty string, left out for a trigger that isn't about files. `holds` and `verification.triggers` both use it |
| `show --diff` | [show.schema.json](../schemas/show.schema.json) | No change. The files are in the item's holds |
| T4's hold | no schema change | Its `message` names each file's commits, such as `tests/old.rs (b7ba4ee before start, 1a2b3c4 after start)`. `files` stay plain paths, so they still match the globs |

An item file written by slice 9 has holds without `files`, and comes back byte for byte, since `Hold::files` has a serde default and `skip_serializing_if`. `tests/fixtures/slice-10/XX-22.json` is a held item written by slice 10's binary. Slices 11 and 12 add T5, T13, T14 and T10 to `triggers`, which needs no schema change, since `trigger`'s pattern allows them. A trigger that names files, such as slice 11's T14, which lists commits, can use `files` as T1 to T4 and T8 do, or add a field.

## Code tour

| File | What's new or changed |
| --- | --- |
| [src/triggers.rs](../src/triggers.rs) | `Ran::files`, `Changes`, `Edit`, `changes(root, id, started)`, `Globs`, `file_list`, `files_word`, and `t1`, `t2`, `t3`, `t4`, `t7` and `t8`. `run(item, config)` returns a `Result`, skips triggers the project switches off and passes `started.head` to `changes`. `t4` names each file's commits. `holds` copies `files` |
| [src/config.rs](../src/config.rs) | `Review`, with `risky_paths`, `source_paths`, `test_paths`, `manifests`, `max_diff_lines` and `disabled`, its default constants, `ALWAYS_ON` and `runs`, and `digest`. `Review` refuses unknown keys, and so does `Project` (TR-37), which also has `judge`, kept as written. `glob_set`, `Project::review`, and `load`'s refusal of a glob that can't be built |
| [src/git.rs](../src/git.rs) | `prefix`, `FileChange` (with the commit's `sha`), `changes`, `since` and `exists_at`. `changes` and `patches` pass `--no-relative` |
| [src/item.rs](../src/item.rs) | `Hold::files` |
| [src/main.rs](../src/main.rs) | `gates_dry_run`, `apply` and `approve` pass the config to `triggers::run`. `approve` finds the root, runs the triggers and names `HEAD` before it asks, and exits 1 if G5's `HEAD` differs. `decision_warning` takes the holds, marks new ones and lists the submit's holds that no longer fire. `show` calls the new `trigger_files`. `project_add` writes `review: None` and `judge: None`. Help text for `Submit` and `Approve` |
| [Cargo.toml](../Cargo.toml), `Cargo.lock` | `globset` |
| [schemas/](../schemas/) | `item.schema.json`: `hold.files`. `submit.schema.json` and `gates.schema.json`: `holds` refer to it |
| [tests/risky_changes.rs](../tests/risky_changes.rs) | New (`b7ba4ee`): one module per TR-6 criterion, `criterion_1` to `criterion_7`, with 33 tests in all, 2 outside them. Review 1's fixes add 11 in modules `tr_31_c1` to `tr_35_c1` and the helper `started_after_tests`, and review 2's add 2 in `tr_37_c1` and `tr_37_c2`, 46 in all. Helpers `started_with`, `write`, `remove`, `commit`, `set_review`, `set_project_field`, `submit`, `fired`, `ran`, `hold`, `assert_held_by`, `assert_not_held` and `has_ref` |
| [tests/fixtures.rs](../tests/fixtures.rs), `tests/fixtures/slice-10/` | XX-22 and `slice_10s_held_fixture_names_the_files_that_fired_each_hold` (`b7ba4ee`) |
| [tests/states.rs](../tests/states.rs), [tests/gates.rs](../tests/gates.rs), [tests/review.rs](../tests/review.rs) | Six tests' expected `triggers` list grew to eight (`b7ba4ee`), [below](#older-tests-that-changed) |
| [docs/DESIGN.md](DESIGN.md) | `e650ecf`: §2.5's "How T1, T2, T3, T4, T7 and T8 read the changes", §2.7's `approve` bullet, §4.2's `disabled` and glob rules and §4.3's `holds`. `89069f3` (TR-36): §2.5's rule for "existing" and T4's naming of commits, the T3 examples in §2.5 and §3, and a §7 bullet. `daa5047` (TR-38): §2.5 says what "after start" means and that a before-start edit is still worth reading in `show --diff`, and §2.5's and §3's T4 examples read as the code prints them, with `files` in §3's two holds |
| [docs/PLAN.md](PLAN.md) | Slice 10's status, `todo` to `in progress` (`6955644`) to `in review` (`2c9da0b`) |

To add a trigger that reads the item's changed files, as slice 11's T14 may: add its function to [src/triggers.rs](../src/triggers.rs), return the message and the files, and add an `add("T14", …)` line to `run` in trigger order. Read `Changes` for files, or extend `changes` and `git::changes` for what they don't hold, such as an author. A trigger that can be switched off needs nothing else, since `add` asks `Review::runs`. One that mustn't be, as T10 and T12, goes in `config::ALWAYS_ON`.

## Testing

Run the test command in `tracker/`. TR-6's criteria each run their own module in `tests/risky_changes.rs`, for example `cargo test --test risky_changes criterion_4::`. The module path ends in `::`, so `criterion_1::` can't match a `criterion_10`. The fixes' checks name their module without it, such as `cargo test --test risky_changes tr_31_c1` or `tr_37_c1`, since no other module's name starts that way.

Every project in `tests/risky_changes.rs` is XX, whose root `<work>/xx` is a subfolder of a throwaway repository, as `Env::new` makes it, so a path from the top of the repository differs from one from the root, as PLAN.md's integration risk "Self-tracking depends on the folder rule" asks. `started_with` writes and commits each file of a base as an unscoped commit, creates XX-1 and starts it, so the files exist before the item's work. Each test then writes files, commits them as `feat(XX-1): the work`, and submits. Four tests run `reject` or `approve` under `script` with `Env::in_terminal_typing`, which returns `None` where `script` isn't installed, and each then returns early, passing: `criterion_5`'s, both of `criterion_6`'s and `tr_35_c1`'s. Nothing in [tests/common/mod.rs](../tests/common/mod.rs) changed.

### Coverage of the _Done when_ checks

| _Done when_ | Tests in `tests/risky_changes.rs` |
| --- | --- |
| Each of the six triggers has one test where it fires and one where it stays quiet. | `criterion_1`, 17 tests. T1: `t1_fires_for_a_new_source_file` (the message and `files`), `t1_fires_for_a_capability`, `t1_stays_quiet_for_a_change_to_an_existing_source_file` (T1 still listed in `triggers`) and `t1_stays_quiet_for_a_new_test_file`, with `tests/**` named in `source_paths`. T2: `t2_fires_for_a_risky_path` and `t2_stays_quiet_for_a_path_that_isnt_risky`. T3: `t3_fires_for_a_changed_manifest` and `t3_stays_quiet_when_no_manifest_changed`. T4: `t4_fires_for_a_changed_line_in_an_existing_test_file` and `t4_stays_quiet_for_a_new_test_file`. T7: `t7_fires_for_more_changed_lines_than_max_diff_lines`, `t7_fires_by_default_above_300_lines`, `t7_counts_lines_changed_outside_the_root` and `t7_stays_quiet_at_max_diff_lines`. T8: `t8_fires_when_the_design_doc_changes` and `t8_stays_quiet_when_another_doc_changes`. And `the_gates_dry_run_lists_the_same_holds` |
| Adding a test case to an existing test file doesn't fire T4. Changing a line in one does, and so does deleting one. | `criterion_2`, 5 tests (and `tr_31_c1` and `tr_31_c2` for a test committed before `start`, [below](#fixes-after-review-1)): `adding_a_test_case_to_an_existing_test_file_doesnt_fire_t4`, `changing_a_line_in_an_existing_test_file_fires_t4`, `deleting_an_existing_test_file_fires_t4` (the message and `files`), `renaming_an_existing_test_file_fires_t4` and `changing_a_test_file_the_item_added_doesnt_fire_t4` |
| In a subfolder project, `source_paths: ["src/**"]` matches `<root>/src/…` and not a `src/` folder elsewhere in the repo. | `criterion_3`, 3 tests: `source_paths_match_src_under_the_subfolder_root` (`files` is `src/deep/search.rs`, relative to the root), `source_paths_dont_match_a_src_folder_elsewhere_in_the_repo` (at the top of the repository, in another folder and nested in the root) and `globs_match_from_the_root_not_from_the_top_of_the_repo`, with `xx/src/**` |
| `"disabled": ["T8"]` silences T8, but `["T6"]` doesn't silence T6. | `criterion_4`, 2 tests: `disabling_t8_silences_it` (T8 isn't in `triggers`, T1 is) and `disabling_t6_doesnt_silence_it`, with `["T6", "T10", "T12"]` |
| After a reject, a resubmit is held again for every trigger that still applies. | `criterion_5::a_resubmit_after_a_reject_is_held_again_for_every_trigger_that_applies`, under `script`: held by T3, T8 and T9, rejected, the label removed and an existing test edited, then held by T3, T4 and T8, with T3's hold equal to the first one |
| `approve` runs the triggers before asking, at the `HEAD` it names, shows every hold that fires there with the new ones marked, and records them in `verification.triggers`. It exits 1, writing nothing, if `HEAD` moves before its gates run. | `criterion_6`, 2 tests under `script`. `approve_shows_and_records_a_hold_from_a_commit_made_after_the_submit`: T3's line says "new since the submit", T9's doesn't, the output names `HEAD`'s short SHA, and the record's `commit` is that `HEAD`, its `triggers` are T3 and T9, and the item matches its schema. `approve_writes_nothing_when_head_moves_while_it_waits`: a commit made while the question waits makes it exit 1 with "HEAD moved", the file unchanged and no ref |
| `show --diff` lists the files that fired a trigger before the item's commits and their diff. | `criterion_7::show_diff_lists_the_files_that_fired_a_trigger_first`: "Files that fired a trigger:" comes before "Commits," and "Diff,", lists `T3  Cargo.toml` and `T4  tests/list.rs` and not a file no trigger named, and the JSON's first hold has `files` |

The _Prompt_'s requirements that no _Done when_ bullet names:

| Requirement | Tests |
| --- | --- |
| Deleted files count, and `source_paths` excludes `test_paths`. | `criterion_2::deleting_an_existing_test_file_fires_t4`, and `criterion_1::t1_stays_quiet_for_a_new_test_file`, where `source_paths` names `tests/**` |
| Files outside the root still count toward T7, and match no glob. | `criterion_1::t7_counts_lines_changed_outside_the_root`, and `a_file_outside_the_root_matches_no_glob`, with `risky_paths` of `["**"]` |
| `disabled` is part of §4.2's `review` config with its defaults. | `criterion_1::t7_fires_by_default_above_300_lines` for `max_diff_lines`. The other defaults are the ones every other test uses. The `manifests` default at depth, and the `disabled` typo, have no test |
| A glob that can't be parsed exits 3. | `an_invalid_review_glob_exits_3_naming_it`, with `list` |
| Slice 9 review 1's design 1. | `criterion_6`, above |

### Other tests

- `slice_10s_held_fixture_names_the_files_that_fired_each_hold` in `tests/fixtures.rs` reads XX-22, which slice 10's binary wrote with T3 and T4 holds, checks its two holds and `files`, and that it comes back byte for byte, loses nothing on a rewrite, and matches the item schema.
- Not tested: rewritten item commits after `start` (a rebase or an amend, which mark every commit "after start"), a `started.head` that git no longer has (every command that runs the triggers exits 3 with git's error), and the order T4 lists its files in when more than 10 changed (review 2's test gap 1, [Still open](#still-open)). Also `approve`'s exit 3 for a root outside a git work tree, T1's file list for a file added and then deleted, a binary file counting as a changed test, a merge commit, T8 for a design path with `./`, and the `manifests` default for a manifest in a subfolder (which review 1 checked by hand, with `judge-eval/package.json`).

### Older tests that changed

`b7ba4ee` changed six existing tests before `tracker start TR-6`, so they were seen failing with TR-6's. Each pinned the list of triggers a submit or `gates` runs as T6 and T9, and now lists the eight. Only expected values changed, and it added lines only:

| Test | File |
| --- | --- |
| `submit_prints_the_shape_section_3_gives_it` | `tests/states.rs` |
| `a_script_criterion_needs_no_evidence` | `tests/gates.rs` |
| `criterion_1::an_item_with_neither_still_auto_verifies` | `tests/review.rs` |
| `criterion_2::submit_json_matches_the_held_shape_in_section_3` | `tests/review.rs` |
| `tr_29::gates_lists_the_holds_a_submit_would_get` | `tests/review.rs` |
| `tr_29::gates_says_no_holds_when_no_trigger_fires` | `tests/review.rs` |

The commit message says four, and its diff has six, as does everything above. T4 wouldn't fire for any of them, since they only gained lines. From slice 11, editing an existing test fires T4 whichever commit does it, so a later change to one of these lists, as slice 11's T5, T13 and T14 will make, should go in the item's test commit before `start`. Review 1's fixes, TR-31 to TR-36, changed no existing test: every test they added is new, and `criterion_2::changing_a_test_file_the_item_added_doesnt_fire_t4` stays as it was, since it adds the file after `start`, which is still the case the new rule leaves alone.

### Checks done by hand

Each was run with this branch's `target/debug/tracker` in a temporary `TRACKER_HOME`:

| What | How | Result |
| --- | --- | --- |
| The demo above | Run as written, with stderr captured | The output above, with the temporary folder replaced by `$DEMO`. Each command marked with an exit code exited with it, and every other one exited 0 |
| `approve`'s new question in a real terminal | Not done | The question names the `HEAD` and marks each hold "(new since the submit)". PLAN.md's integration risk, "Interactive-only commands are only half tested", asks for each interactive command's prompt to be tried once. The user's check on 2026-11-22 of `approve` and `reject`, with the installed slice 9 binary ([slice 9's readme](slice-9-readme.md#checks-done-by-hand)), came before this change, so it didn't see the new question. TR-35 then changed the question again, so it also lists a hold of the submit's that "no longer fires". The tests type the ID into it under `script`, which §7 says looks the same to `tracker` as a real terminal |

### Tracking TR-6

TR-6, a `capability` with `design` `docs/DESIGN.md#2.5`, was created at 2026-11-21T07:07:43Z with slice 7's other items. Its first five criteria are PLAN.md's _Done when_ bullets, unchanged. The builder added the sixth, for `approve`'s second run of the triggers (slice 9 review 1's design 1), and the seventh, for `show --diff`'s list of files, which PLAN.md's _Prompt_ asks for and no bullet checks. Each check runs one module of `tests/risky_changes.rs`, such as `cargo test --test risky_changes criterion_6::`, and the saved output shows each matching 17, 5, 3, 2, 1, 2 and 1 tests. The slice 9 binary was installed before TR-6 started and tracked it throughout:

| Step | When (UTC) | What |
| --- | --- | --- |
| Tests, fixture XX-22 and the older tests' expected values committed | 00:37:48Z | `b7ba4ee` |
| `tracker start TR-6` | 00:38:34Z | `started.head` `b7ba4ee`. `started.checks`: all seven criteria failed |
| The code | 00:39:04Z to 00:40:23Z | `2a4d5df`, `9475fb3` (help text) and `e650ecf` (DESIGN.md) |
| `tracker submit TR-6` | 00:45:31Z | G2, G5, G6, G3 and G4 passed (G4's `cargo test` took 72s), then T6 and T9, and nothing was held: verified `auto` at `e650ecf`, with `refs/tracker/TR-6` |
| PLAN.md: slice 10 in review | | `2c9da0b`, made after the submit |

- **Nothing could have held it** but T6 and T9, which read only the item. None of its criteria is manual, since each has a check, and it has no `needs-review` label. The slice 10 binary, run on TR-6, would have held it, by the rules above, under T1, since it's a `capability`, T3, since `Cargo.toml` and `Cargo.lock` changed, T7, since its commits changed well over 300 lines, and T8, since `docs/DESIGN.md` changed. That wasn't run.
- **`tracker show TR-6` is the record**: each check's command, result and saved output.
- **The review's slice 10 points were handled as follows.** TR-6's criteria all have checks. Its tree stayed clean, since it wasn't held. The `holds` shape stayed loadable: `files` has a default. Which of T1, T4 and T8 to switch off for TR is still the user's to decide ([Limits](#limits-and-what-later-slices-change)).

### Tracking review 1's fixes

Review 1 was committed in `c473b57` at 2026-11-22T01:07:41Z, and its fixes were created in the same second, as TR-31 to TR-36, each naming the review and its findings in its description. Each was tracked with the installed slice 9 binary, one at a time, as [PLAN.md](PLAN.md)'s integration risks ("The bootstrap") and [CLAUDE.md](../CLAUDE.md) ask, not with this slice's:

| Item | Kind | For | Tests committed | Started | Fix | Submitted |
| --- | --- | --- | --- | --- | --- | --- |
| TR-31 | `fix` | design 1 and test gap 1 | `3363871`, 01:08:48Z | 01:08:52Z | `35d05ec`, 01:13:24Z | 01:17:26Z |
| TR-32 | `fix` | security 1, the code | `0a299c1`, 01:17:49Z | 01:17:52Z | `5b866cd`, 01:19:54Z | 01:22:31Z |
| TR-33 | `capability` | design 2 | `50ae411`, 01:26:20Z | 01:26:22Z | `f9ff0c7`, 01:28:42Z | 01:31:32Z |
| TR-34 | `fix` | bug 1 | `ccabaf9`, 01:31:55Z | 01:31:58Z | `c147e5c`, 01:34:03Z | 01:36:46Z |
| TR-35 | `fix` | bug 2 | `e08023c`, 01:37:20Z | 01:37:22Z | `aa6c781`, 01:38:55Z | 01:41:36Z |
| TR-36 | `change` | security 1's `-diff` attribute, security 2 and design 3 | none: its checks are `grep`s | 01:42:05Z | `89069f3`, 01:42:46Z | 01:46:56Z |

So:

- **Each reached done `auto`**, verified by the installed slice 9 binary, which ran G2, G5, G6, G3 and G4, then T6 and T9. `tracker gates` showed no hold before each submit, and none was held. Each record has a `commit`, the last of the item's commits, and `refs/tracker/<ID>` points at it: `35d05ec`, `5b866cd`, `f9ff0c7`, `c147e5c`, `aa6c781` and `89069f3`. G4's `cargo test` took 74 to 81 seconds. After the last, all 382 tests pass, 11 more than at `bdd3e71`: 3, 2, 3, 2 and 1 in `tests/risky_changes.rs` for TR-31 to TR-35.
- **Nothing could have held them.** That binary has only T6 and T9, which read the item, and none of the six has a manual criterion or the label `needs-review`.
- **Eight of the ten checks failed at start**, as `started.checks` records, TR-36's three included. Two passed, as they had to, and the next two points say why.
- **TR-31's criterion 2 passed at start.** Adding lines to a test never fired T4, so it guards behaviour the fix mustn't lose. It was seen failing by temporarily counting every change to an existing file.
- **TR-34's criterion 2 passed at start**, since every key it lists loaded before. It was seen failing on `digest` once `deny_unknown_fields` alone was added, which is why the fix adds the field. Slice 11's T13 would hold TR-31 and TR-34 for those two checks.
- **Each filter matches what it names.** The saved output shows `tr_31_c1` matching 2 tests, `tr_31_c2` 1, `tr_32_c1` 2, `tr_33_c1` 3, `tr_34_c1` 1, `tr_34_c2` 1 and `tr_35_c1` 1. TR-36's criterion 3 is `test -f docs/DESIGN.md && grep -qF … && ! grep -qF …`, so a DESIGN.md that can't be read fails it.
- **No existing test changed**, as [Older tests that changed](#older-tests-that-changed) says.
- **Every commit from `3363871` to `89069f3` is scoped to one of the six.** `c473b57` came before TR-31's start, while no item was in progress.

### Tracking review 2's fixes

Review 2 was committed in `3a63ca8` at 2026-11-22T02:13:08Z, and its fixes were created in the same second, as TR-37 and TR-38, each naming the review and its findings in its description. Each was tracked with the installed slice 9 binary, one at a time, as for review 1's:

| Item | Kind | For | Tests committed | Started | Fix | Submitted |
| --- | --- | --- | --- | --- | --- | --- |
| TR-37 | `capability` | bug 1 | `d6dfc5a`, 02:13:58Z | 02:14:00Z | `469f8bb`, 02:19:39Z | 02:22:25Z |
| TR-38 | `change` | bug 2, design 1's DESIGN.md half and design 2 | none: its checks are `grep`s | 02:22:46Z | `daa5047`, 02:23:10Z | 02:25:49Z |

So:

- **Each reached done `auto`**, verified by the installed slice 9 binary, which ran G2, G5, G6, G3 and G4, then T6 and T9, and held nothing. Each record has a `commit`, the last of the item's commits, and `refs/tracker/<ID>` points at it: `469f8bb` and `daa5047`. G4's `cargo test` took 80 seconds each time. After TR-37, all 384 tests pass, 2 more than the 382 at `55c6064`, which are `tr_37_c1` and `tr_37_c2` in `tests/risky_changes.rs`.
- **Nothing could have held them.** That binary has only T6 and T9, which read the item, and neither item has a manual criterion or the label `needs-review`. TR-37 is a `capability`, as [CLAUDE.md](../CLAUDE.md) asks for an item that adds a field (`Project::judge`), and review 2's process 1 asks for it from now on.
- **Four of the five checks failed at start, as they should, and one passed, as it had to.** `started.checks` has TR-37's criterion 1 failing, and all three of TR-38's. TR-37's criterion 2 passed at start, since every key §4.2 lists loaded before the change. It was seen failing with `deny_unknown_fields` on `Project` but no `judge` field, which is why the fix adds one. Slice 11's T13 would hold TR-37 for that check.
- **Each filter matches what it names.** The saved output shows `tr_37_c1` matching 1 test and `tr_37_c2` 1. TR-38's checks are all `grep -qF` on `docs/DESIGN.md`, so a file that can't be read fails them, and criterion 3 counts the T4 example with `test $(grep -cF …) -ge 2`, so it needs both examples.
- **The user's real `config.json` was checked read-only before the change.** It holds only keys §4.2 lists under each project, so refusing the others made no existing config invalid, and no test's config needed changing.
- **No existing test changed.** `d6dfc5a` only adds 38 lines to `tests/risky_changes.rs`.
- **Every commit from `d6dfc5a` to `daa5047` is scoped to one of the two.** `3a63ca8` came before TR-37's start, while no item was in progress.

## Decisions worth knowing

- **"Existing" depends on which side of `start` the commit is.** §2.5 says so. For a commit before `start`, a file is existing if no item commit added it, so an item's `test(XX-1)` commit is free to add test files. For a commit after `start`, it's existing if it exists at `started.head`, whoever added it, so a line changed or removed from a test committed before `start` fires T4 after it (review 1's design 1). Lines added to switch a test off, such as `#[ignore]` or an early `return;`, only add lines, so T4 doesn't see them, and "can't be weakened" would claim more than the code gives (review 2's security 2). §2.5 leaves that to slice 16's digest. A test the item adds after `start` is new work, and is free to change.
- **Changed files come from one `git show` of every item commit, each against its first parent.** A rename is a delete and an add, so renaming a test file fires T4. `-z` keeps odd file names whole.
- **T7 counts every commit's lines, summed**, not the net diff from before the first commit to `HEAD`. An item that adds a file and then removes it counts both. Tests, files outside the root and deleted files are included, since a big change hides surprises wherever it is.
- **A typo in `disabled` is ignored.** `Review::runs` asks only whether a name is in the list. `"T99"` silences nothing, and so does `"t8"`. DESIGN.md §4.2 says so, and a misspelled name leaves the trigger on, which fails safe.
- **A key in `review` that isn't §4.2's exits 3 for every command**, as a bad glob does (review 1's bug 1). A misspelled `risky_path` would otherwise leave T2 on its default of never firing, and nothing would show it. `digest` is accepted, since §4.2 lists it, and kept as written for slice 16. The same goes for a key directly under a project (review 2's bug 1): `Project` has `deny_unknown_fields`, so `risky_paths` written beside `checks`, or a misspelled `review`, exits 3 naming it, and `judge` is accepted as a `serde_json::Value` kept as written for slice 12.
- **A glob that can't be parsed exits 3 for every command**, at `config::load`, as a bad root does, rather than failing a submit after its gates have run. The error names the project and the glob.
- **A switched-off trigger isn't listed in `triggers`.** §3's `triggers` says which ran, and a trigger that didn't run has no `fired: false` to report. `ran` in the tests asserts it.
- **`risky_paths` is empty by default**, so T2 never fires until a project sets it, since only the project knows which paths are risky. TR sets none yet. `manifests` defaults to the usual manifests and lockfiles, at any depth.
- **`approve` runs the triggers before it asks, not after the gates.** What I'm shown has to be what's recorded. If the triggers ran after the question, a commit made while I read it could add a hold I never saw. Running them before, and exiting 1 if `HEAD` moved by the time G5 looked, closes that. The holds it shows are the ones it keeps, so a hold from a commit made since the submit goes in `verification.triggers` with the submit's.
- **`approve` runs the triggers once, outside the lock**, with the item as it read it. `Store::lock_unchanged` confirms the file hasn't changed, and the `HEAD` comparison confirms git hasn't.
- **`approve` exits 3 before its question for a root that isn't in a git work tree**, as `submit` does, since the triggers need git to read the item's commits.
- **T4's message names each file's commits, and the hold's `files` stay plain paths** (review 1's design 2). The message is for the human at `approve`, who needs to tell an expected-value update made before `start` from an edit after it, and the globs and `show --diff`'s list read `files`. The mark says when, not that the edit was right: a weakening can sit in a `test(TR-n)` commit made before `start`, so the user reads T4's files in `show --diff` whichever side of `start` the mark says (review 2's design 1).
- **"After start" means `started.head` can't reach the commit.** `git::since` is `git rev-list <started.head>..HEAD`, so rewriting an item's commits after `start` gives them new SHAs and marks them all "after start", and it errs towards more holds. §2.5 says so (review 2's bug 2). Deciding by author date against `started.at` was left out, since an agent can set a commit's date.
- **`git::changes` and `git::patches` pass `--no-relative`**, as they pass `--no-ext-diff` and `--no-textconv`: a setting in `.git/config` that no commit shows mustn't change which files the triggers see or what `show --diff` prints (review 1's security 1).
- **`approve` lists a hold that no longer fires, matched by trigger.** A hold of the submit's whose trigger isn't among the holds at `HEAD` is listed "(no longer fires)". The question doesn't say "at <HEAD>", as the review suggested, since nothing needed it.
- **A hold's `files` are relative to the root**, not the repository, so they match the globs and what the project's own tools print. `show --diff` prints the diff itself with paths from the top of the repository, as `git show` does.
- **T7's hold has no `files`**, since it isn't about particular files, and `show --diff`'s list omits it. Slice 11's T14 lists commits, which also aren't files.
- **TR's own `review` config is the user's to set**, since it changes what holds TR's own items ([Limits](#limits-and-what-later-slices-change)). This slice sets nothing there.

## Fixes after review 1

[Review 1](reviews/slice-10-review-1.md) found three bugs, two security issues, four places where the code and DESIGN.md disagreed or the design was unclear, two test gaps and two process notes. Bugs 1 and 2, security 1 and 2, designs 1 to 3 and test gap 1 became tracker items, TR-31 to TR-36 ([Tracking review 1's fixes](#tracking-review-1s-fixes)), each with its tests committed before `tracker start` and seen failing, apart from TR-31's criterion 2 and TR-34's criterion 2, which passed at `start` as they had to, and TR-36, whose checks are `grep`s on DESIGN.md that failed at `start`. Option (a) was taken wherever the review chose it. Bug 3, design 4 and test gap 2 won't be fixed, process 2 is done in this readme, and process 1 is still the user's. The review's [Outcome](reviews/slice-10-review-1.md#outcome) records the same decisions.

### Fix before slice 11

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Bug 1: a misspelled key in a project's `review` settings was ignored, so the setting never took effect | `c147e5c` (TR-34) | Option (a). `#[serde(deny_unknown_fields)]` on `config::Review`, and `digest: Option<serde_json::Value>`, with a serde default and `skip_serializing_if`, since §4.2 lists it. A misspelled key exits 3 for every command, naming it and the keys allowed | `tr_34_c1::a_misspelled_review_key_exits_3_naming_it`: `risky_path` makes `list` exit 3 naming it. Seen failing before the fix. `tr_34_c2::a_review_with_every_key_in_4_2_loads`: every key §4.2 lists, with `digest` of `{"every_days": 7}`, loads. It passed before the work and failed on `digest` once `deny_unknown_fields` alone was added |

### Low severity

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Bug 2: when a submit's hold no longer fired, `approve`'s question said "no trigger recorded" and didn't name it | `aa6c781` (TR-35) | `decision_warning` lists each of `item.holds` whose trigger isn't among the holds at `HEAD` as "(no longer fires)", and the first line ends "and nothing holds it now." when none fires. The review's "at <HEAD>" wording isn't used, since the criterion doesn't need it | `tr_35_c1::approve_lists_a_submit_hold_that_no_longer_fires`, under `script` and skipped where it isn't installed: a T3 hold, then `disabled: ["T3"]`, and the question has a T3 line saying "no longer fires" and never "no trigger recorded". Seen failing before the fix. It doesn't pin the new empty-case wording |
| Bug 3: T1 listed a source file the item added and then renamed or deleted | none | Won't fix. The hold is right, since T1 fires anyway, and only its file list is off. It's unlikely in TR, where a slice seldom adds a file and then moves it ([Still open](#still-open)) | |

### Security items

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Security 1: `diff.relative` in `.git/config` hid every file from the triggers, and a `-diff` attribute hid a file's lines from T7 | `0a299c1` and `5b866cd` (TR-32) for the code, `89069f3` (TR-36) for the attribute | `--no-relative` on `git::changes` and `git::patches`. For the attribute, option (a): §7 has a bullet saying a `-diff` attribute in `.git/info/attributes` hides a file's lines from T7, that a `.gitattributes` does the same but shows in the item's commits, and that T4 still fires for a test file | `tr_32_c1::the_file_triggers_still_fire_with_diff_relative_set` and `tr_32_c1::show_diff_still_lists_files_outside_the_root_with_diff_relative_set`, which set `diff.relative` in a subfolder project's repository. Both seen failing before the fix. TR-36's criterion 1 is a `grep -qF` on §7, seen failing |
| Security 2: an unscoped commit before `start` could delete an existing test so that the item re-added it weakened, out of T4's reach | `89069f3` (TR-36) | Option (a): the same §7 bullet says an unscoped commit before `start` can delete an existing test so that the item re-adds it as a new file, and that the unscoped commit is in history for the digest's reader to find | TR-36's criterion 2, a `grep -qF` on §7, seen failing |

### Design decisions

| Question | Decision | Commits |
| --- | --- | --- |
| Design 1: an item could weaken a test it added itself after `start` without firing T4 | Option (a). For a commit after `started.head`, a test file counts as existing if it exists at `started.head`, whoever added it. `git::FileChange` carries the commit's SHA, and the new `git::since` and `git::exists_at` answer which commits are after `start` and what was there. §2.5's sentence says so. The old test that pinned the other rule, `criterion_2::changing_a_test_file_the_item_added_doesnt_fire_t4`, is unchanged and still right, since it adds the file after `start` | `3363871`, `35d05ec` (TR-31), `89069f3` (DESIGN.md) |
| Design 2: T4's hold didn't say which commit changed a test | Option (a). The message names each file's commits by 7-character SHA, marked before or after `start`: `tests/old.rs (b7ba4ee before start, 1a2b3c4 after start)`. `files` stay plain paths. No existing test pinned the shape | `50ae411`, `f9ff0c7` (TR-33), `89069f3` (DESIGN.md) |
| Design 3: T3's message doesn't name the dependency, as §2.5's and §3's examples do | Option (a). Both examples read `dependency manifest changed: package.json`, as the code prints it | `89069f3` (TR-36) |
| Design 4: §7 doesn't say T7 sums every commit's lines | Won't fix. §2.5 already states the rule, and it only over-holds ([Still open](#still-open)) | none |

TR-31's tests: `tr_31_c1::changing_a_line_of_a_test_committed_before_start_fires_t4` and `removing_a_test_committed_before_start_fires_t4` commit the test with `started_after_tests` before `start`, then change or remove it in a `feat(XX-1)` commit, and the submit is held by T4 alone, naming the file. `tr_31_c1` was seen failing: the item auto-verified before the fix. `tr_31_c2::adding_lines_to_a_test_committed_before_start_doesnt_fire_t4` adds a test case after `start` and isn't held. TR-33's tests: `tr_33_c1::the_hold_names_each_commit_marked_before_or_after_start` has an unscoped commit, a `test(XX-1)` edit before `start` and a weakening after it, and the hold's message has both SHAs with their marks while `files` is `["tests/old.rs"]`. `the_text_output_names_them_too` checks `submit`'s text, and `a_deleted_file_names_its_commit_too` a deletion. All three were seen failing.

### Test gaps

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Test gap 1: nothing covered an item whose tests are committed before `start` and changed after it | `3363871` (TR-31) | `started_after_tests` commits the item's tests with a `test(XX-1)` commit before `start`, as TR's workflow does | `tr_31_c1`, `tr_31_c2` and `tr_33_c1`, above |
| Test gap 2: the readme's untested cases are still untested | none | Won't fix. The remaining cases, `approve`'s exit 3 for a root outside a git work tree, a binary test file, a merge commit and a design path with `./`, are unlikely in TR, and the binary case fails safe. Security 1's fix brought its own tests, as the review asked ([Still open](#still-open)) | |

### Process

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Process 1: `approve`'s new question hasn't been seen in a real terminal | none | Still the user's. The first held TR item, from slice 11 on, shows it. TR-35 has since changed the question again, so a check now sees both changes ([Checks done by hand](#checks-done-by-hand)) | |
| Process 2: the Branch row said four older tests changed, and a bullet had a stray comma | none | Done in this readme. The Branch row says six, which the diff and [Older tests that changed](#older-tests-that-changed) already said, and "held for both,." reads "held for both." | |

The review's [Slice 11 must decide](reviews/slice-10-review-1.md#slice-11-must-decide) points are left for slice 11 (TR-7) and carried into [Limits](#limits-and-what-later-slices-change). TR's own `review` settings, such as `risky_paths`, are the user's to write, and the user chose to keep every trigger on.

A few consequences:

- A test committed before `start` is now existing for every commit after it, so an item that fixes a test after `start`, which honest work does, is held by T4, with the commit marked "after start". Updating a pinned value in the `test(TR-n)` commit before `start` still holds the item, and the message says "before start".
- A hold's `message` for T4 is longer, and carries SHAs that change when a commit is rewritten. `files` don't.
- A `config.json` with a key `review` doesn't know, which loaded before, makes every command exit 3 until it's fixed.
- `approve`'s question can have a line that the submit's hold list had and the holds at `HEAD` don't.
- The Try it output changed in three places, T4's line in `gates` and in `submit`, and its `message` in the hold's JSON, and the commit's SHA with them. `show --diff`'s list of files is the same.

## Fixes after review 2

[Review 2](reviews/slice-10-review-2.md) found four bugs, two security issues, two places where the code and DESIGN.md disagreed or the design was unclear, one test gap and one process note. Bug 1 became TR-37, with its tests committed before `tracker start` and seen failing. Bug 2 and designs 1 and 2 became TR-38, which changes DESIGN.md only, and its `grep` checks failed at `start`. Security 2 and design 1's other half are in this readme. Option (a) was taken wherever the review chose it. Bugs 3 and 4 and test gap 1 won't be fixed, security 1's (b) is left to slice 11's T14, and process 1 is the user's. The review's [Outcome](reviews/slice-10-review-2.md#outcome) records the same decisions.

### Fix before slice 11

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Bug 1: a misspelled `review`, or a `review` setting put directly under the project, was ignored without a word | `469f8bb` (TR-37) | Option (a). `#[serde(deny_unknown_fields)]` on `config::Project`, and `judge: Option<serde_json::Value>`, kept as written, with a serde default and `skip_serializing_if`. `project_add` writes `judge: None`. A key under a project that §4.2 doesn't list exits 3 for every command, naming it | `tr_37_c1::a_misplaced_project_key_exits_3_naming_it`: `risky_paths` beside `review` makes `list` exit 3 naming it. Seen failing before the fix. `tr_37_c2::a_project_with_every_key_in_4_2_loads`: `name`, `root`, `design`, `checks`, `check_timeout_secs`, `review` and a `judge` with `max_failures`, `auto_verify` and `timeout_secs` load. It passed at `start`, and was seen failing with `deny_unknown_fields` but no `judge` field |

### Low severity

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Bug 2: "after start" means "not reachable from `started.head`", so rewriting the item's commits after `start` marks them all "after start" | `daa5047` (TR-38) | Option (a). §2.5 says a commit counts as after start when `started.head` can't reach it, so a rebase or an amend marks them all as after start | TR-38's criterion 1, a `grep -qF` on §2.5, seen failing. No test rewrites commits (test gap 1) |
| Bug 3: if `started.head` is gone from the repository, `gates`, `submit` and `approve` exit 3 with a raw git error | none | Won't fix. It's very unlikely, since it needs a rebase and a prune of unreachable commits, and it fails closed, with exit 3 and nothing written. Slice 11's T14 reads the same range and will meet it ([Still open](#still-open)) | |
| Bug 4: T4 names at most 10 files, oldest commit first, so an after-start edit is the first cut to "N more" | none | Won't fix. It's unlikely, since it needs a test commit that changes more than 10 existing test files, and the hold still fires, with `files` and `show --diff` listing every file ([Still open](#still-open)) | |

### Security items

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Security 1: after `start`, an unscoped commit can delete an existing test so that the item re-adds it weakened | none | Option (b), left to slice 11's T14, which reads the unscoped commits. This binary judges only TR-7 without T14, and TR-7's commits get read by hand at its `approve`, as the bootstrap rule asks ([Still open](#still-open)) | |
| Security 2: lines that switch an existing test off are added lines, so T4 doesn't see them, and the readme said the tests "can't be weakened" after `start` | this readme's update | Option (a). [What's in this slice](#whats-in-this-slice) and [Decisions worth knowing](#decisions-worth-knowing) now say an item that changes or removes a line of a test committed before `start`, or deletes it, fires T4, and that lines added to switch a test off, such as `#[ignore]` or `return;`, don't. §2.5 already says so, and leaves it to slice 16's digest. The code is unchanged | None. A docs fix, and the code's behaviour is as it was |

### Design decisions

| Question | Decision | Commits |
| --- | --- | --- |
| Design 1: "before start" says when a test was edited, not that the edit was sanctioned | Option (a). §2.5 says the mark says when the edit was made, not that it was right, so an edit marked before start is still worth reading in `show --diff`. This readme drops "tells that sanctioned edit from one made after it", and where it said TR-7's T4 hold "reads as the expected hold" ([Limits](#limits-and-what-later-slices-change)) it now says to read the edit in `show --diff` as well. The review's option (b), a removed-line count in the mark, wasn't taken, since a one-line weakening reads the same | `daa5047` (TR-38) for DESIGN.md, and this readme's update |
| Design 2: §2.5's and §3's T4 examples don't match the message the code prints | Option (a). Both read `existing test file modified: tests/bookmarks.spec.js (1a2b3c4 after start)`, and §3's two holds have `files` | `daa5047` (TR-38) |

TR-38's checks: criterion 1 is three `grep -qF` on §2.5's new clause, criterion 2 two on the sentence about `show --diff`, and criterion 3 the two T4 examples and §3's two `files` lines. All three failed at `start`.

### Test gaps

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Test gap 1: nothing tests rewritten commits, a missing `started.head` or the order T4 lists its files in | none | Won't fix. It covers bugs 2 to 4. Bugs 3 and 4 aren't fixed, and bug 2 is now documented in §2.5 ([Still open](#still-open)) | |

### Process

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Process 1: in TR's real config a review fix that changes the triggers' own code auto-verifies | none | `review.risky_paths` for TR is the user's to write in `~/.tracker/config.json`, and the review's suggested list (`src/triggers.rs`, `src/gates.rs`, `src/git.rs`, `src/store.rs`, `src/config.rs` and `schemas/**`) has been put to them. Bug 1's fix, which the review wanted first, is in, so a misplaced key can't leave it off. From now on, a review fix that adds behaviour or a field is created as `capability`, as CLAUDE.md says, as TR-37 was | |

The review's [Slice 11 must decide](reviews/slice-10-review-2.md#slice-11-must-decide) points are left for slice 11 (TR-7) and carried into [Limits](#limits-and-what-later-slices-change).

A few consequences:

- A `config.json` with a key under a project that §4.2 doesn't list, which loaded before, makes every command exit 3 until it's fixed. A key in the file's top level isn't refused.
- The Try it output didn't change, apart from the commit's SHA, which differs on every run.
- Rewriting an item's commits after `start` marks them all "after start" in T4's message, and can add a hold, as §2.5 now says.

## Still open

- **DESIGN.md §7** (review 1's design 4, which won't be fixed): T7 counts every commit's lines, summed, so an item that adds a large file and then removes it is held for both. §2.5 says "the lines each commit added and removed, summed", but §7 doesn't give it as a limit. It only over-holds.
- **Low severity:**
  - Review 1's bug 3, which won't be fixed: T1 names a source file that the item added and then renamed or deleted, as if it were new, since `added` keeps a path after a later `D`. T1 fires anyway, so only the list is off. It's unlikely in TR.
  - Review 2's bug 3, which won't be fixed: a `started.head` that git no longer has makes `gates`, `submit` and `approve` exit 3 with git's error. It needs a rewrite of the item's commits and a prune of what they replaced, and it fails closed. An item in review can then only be rejected and shelved, then reopened and started again, which the message doesn't say.
  - Review 2's bug 4, which won't be fixed: T4 names at most 10 files, oldest commit first, so with more than 10 changed test files an after-start edit can be in "and 2 more". The hold still fires, and `files` and `show --diff` list every file.
- **Security** (review 2's security 1 and 2):
  - After `start`, an unscoped commit can delete an existing test and the item's own commit re-add it weakened, and this binary has no T14 to see the unscoped commit. Left to slice 11's T14. TR-7's commits get read by hand at its `approve`.
  - Lines added to switch a test off, such as `#[ignore]` or `return;`, don't fire T4. The readme no longer claims otherwise, and §2.5 leaves it to slice 16's digest.
- **Process:**
  - Review 1's process 1: `approve`'s question, which names the `HEAD`, marks holds "(new since the submit)" and lists a hold that "no longer fires", hasn't been seen in a real terminal ([Checks done by hand](#checks-done-by-hand)). It's the user's to check by hand, with this slice's binary in a throwaway store, as slice 9's was. The first held TR item, from slice 11 on, shows it.
  - Review 2's process 1: TR's `review.risky_paths` is the user's to write in `~/.tracker/config.json`, and until it's written a review fix that changes the triggers' own code auto-verifies, since TR sets none and review fixes aren't `capability` unless they add behaviour or a field. The review's suggested list has been put to the user.
- **Test gaps** (review 1's test gap 2, which won't be fixed, and the builder's note): `approve`'s exit 3 for a root outside a git work tree, T1's list for a file added and then deleted, a binary file counting as a changed test, a merge commit, a design path with `./` and the `manifests` default at depth have no test ([Other tests](#other-tests)). `tr_35_c1` doesn't pin the new wording for an empty hold list. Review 2's test gap 1, which won't be fixed: nothing tests rewritten commits after `start`, a missing `started.head` or the order T4 lists its files in.
- **Carried from slice 9** ([slice 9's Still open](slice-9-readme.md#still-open)): `review`'s line for a held item with no `holds` and `show --diff`'s text for an item with no commits have no test, and neither does `approve`'s first re-read as a plain `store.lock()`. `deletion_warning`'s wording for an `unchecked` record with one check, from slice 8, and `Env::add_criterion`'s `--check true`, left for slice 11, are also still open.
- **Slice 9's "Slice 10 must decide" points**: design 1 is fixed by criterion 6, `disabled` never switching off T6 and the `holds` shape are done, and TR's criteria and tree are as [above](#tracking-tr-6). Which of T1, T4 and T8 to switch off for TR is still the user's to decide, and the user chose to keep every trigger.
- **Slice 11 must decide** ([review 1](reviews/slice-10-review-1.md#slice-11-must-decide) and [review 2](reviews/slice-10-review-2.md#slice-11-must-decide)): left for slice 11 (TR-7), and in [Limits](#limits-and-what-later-slices-change).

## Limits, and what later slices change

- **Slice 11** adds T5, T13 and T14. T14 reads commits between `started.head` and `HEAD`. `git::since(root, started.head)` already lists them as SHAs, and `Changes` holds every item commit's files, so T14 needs only each commit's header and whether it touches the root, from a log of its own. It lists commits, not files. T13 holds every test item whose check passed at start, and `Env::add_criterion`'s check is `true`, so most tests that submit would be held. Changing the helper to a check that fails at start, as slice 7's readme asks, avoids that, and it fires T4 through `tests/common/mod.rs`. T5's fingerprint is taken at `start`, so it must not change while an item is in progress. Editing an existing test fires T4 and holds the item whichever commit does it, so an item that fixes a test pinned by an earlier slice, as `b7ba4ee` did, is held. Its hold names the commit and marks it before or after `start`, so TR-7's test commit, which must add T5, T13 and T14 to the six pinned `triggers` lists in `tests/states.rs`, `tests/gates.rs` and `tests/review.rs` and gives T9's line a comma, is held with "before start". That mark says when the edit was made, not that it was right, so read those lines in `show --diff` as well as the hold, and make the edits in the test commit before `start`. TR-7 is a `capability`, so T1 holds it, likely with T7 and T8, and this binary has no T5, T13 or T14, so its record is read by hand, as the bootstrap rule says.
- **T14 reads the same range as TR-31** (review 2). It should use `git::since`, so it meets a rewritten item's commits, which `started.head` can't reach and so count as after `start`, and a `started.head` that git no longer has, which exits 3 today. Decide both once, for T4 and T14 together. T14 also closes review 2's security 1, the unscoped delete after `start` that the item re-adds weakened. Once T5 is in, `edit --kind` after `start` fires it, so a review fix's kind must be right before `start`. TR-7's expected holds are T1, T8 and T4 with "before start" for the pinned lists, and T7 once its code is in, and its record is read by hand.
- **T14 and unscoped commits.** Once slice 11's binary is installed, an unscoped commit that touches `tracker/` while a TR item is in progress fires T14 at its submit, and one made while it waits in review fires T14 at `approve`, marked new, since `approve` runs the triggers again. Readme and review commits such as `bdd3e71`, `c473b57` and `ed6d12a` are unscoped. Make them only when no TR item is in progress or in review, or scope them to the item.
- **Slice 12** adds T10, which `config::ALWAYS_ON` already lists, with `verification.judges`. `run` then needs the judge settings and the cache, beside the config it takes now. A `human` record then has `judges` and `triggers`, and `approve` runs the gates again, so it runs the judges again too, from the cache when nothing changed (§2.3.1). `approve` also runs the triggers again, before the question and outside the lock, so T10's judge answers come from the cache there too.
- **Slice 15** makes `approve` count each judge question held under T10 or T12 as passed for that approval only, and has `approve` and `reject` record the decision against each held question. `approve` now shows the holds at `HEAD`, and `verification.triggers` holds those, so a held question's record has to go where `approve` reads it before it asks, as the triggers' holds do.
- **Slice 16**'s digest lists every `approve` and `reject`, and every edit that removes `needs-review` or moves a capability's kind, which switch off T9 and T1. §2.5 says the digest is there to catch an added line that switches a test off, such as `test.skip`, which T4 doesn't see, but PLAN.md's prompt gives it no check for one, so slice 16's builder must decide how a reader would see it. It lists nothing about a change to `config.json`, where `review.disabled` and the globs live, and PLAN.md's slice 16 gives `audit` no check of it. §7 says the store files are plain text.
- **A held item pauses its project** until it's decided (§4.4), and from slice 11 on, slice 10's binary will hold nearly every TR item. TR-7 to TR-12 are `capability`, so T1 holds each. A change to `docs/DESIGN.md`, TR's design file, fires T8. A new dependency or a changed `Cargo.lock` fires T3, and an item with more than 300 changed lines fires T7. T2 holds nothing unless TR's config sets `risky_paths`, such as `src/store.rs` and `src/gates.rs`, which §4.2's example names. Which of T1, T4, T7 and T8 TR's `review` switches off, and whether it sets `risky_paths`, is the user's to decide, because it changes what holds TR's own items. T6, T10 and T12 can't be switched off.
- **`show --diff` runs `git show` once with every item commit**, and `git::changes` runs it once too, so an item with many commits reads all of them each time `submit`, `gates` and `approve` run the triggers. A project with a long history of item commits would feel it, as G5's `item_commits` already scans everything reachable from `HEAD`.
- **The globs are compiled on every command**: `config::load` builds each project's four sets to check them, and `triggers::run` builds them again. A project with large glob lists pays twice.
- **`approve` and `reject` are only as interactive as stdin.** `script` gets past the terminal check, as the tests show, and §7 says so: an approve made that way records `"interactive": true` and looks like mine, which only the digest (slice 16) can show.
- **The tests that put an item in review by editing its file**, in `tests/states.rs`, `tests/gates.rs`, `tests/criteria.rs`, `tests/organise.rs` and `tests/show_and_delete.rs`, still do. They test refusals that don't depend on how the item got there.
- **A subfolder project's files outside the root** count toward T7 and appear in `show --diff`, and match no glob (§2.1). A project whose risky files live outside its folder has to be registered with the repository's top as its root, as §7 says of G5.

# Slice 10: Risky changes are held

`tracker` is a command-line work tracker, written in Rust, that keeps one central list of work items across many projects and decides when an item is really done. This document covers **slice 10** of the 16 in [PLAN.md](PLAN.md): what it built, how it works, and how to test it. The earlier readmes cover the rest: [slice 1](slice-1-readme.md) the store, `create` and `list`, [slice 2](slice-2-readme.md) the states and `history`, [slice 3](slice-3-readme.md) priority, kind, labels, `edit` and `list`'s filters, [slice 4](slice-4-readme.md) `show` and `delete`, [slice 5](slice-5-readme.md) bad data and polish, [slice 6](slice-6-readme.md) criteria and gates G1, G2 and G6, [slice 7](slice-7-readme.md) running checks, with gates G3 and G4 and `started.checks`, [slice 8](slice-8-readme.md) G5, the `auto` record and `refs/tracker/<ID>`, and [slice 9](slice-9-readme.md) the inbox, `approve`, `reject` and triggers T6 and T9.

After slice 10 you can **have new capabilities, new dependencies, risky paths, edited tests, big diffs and design changes held**. The spec is [DESIGN.md](DESIGN.md). This slice implements an item's changed files and source files from [§2.1](DESIGN.md#21-items), triggers T1, T2, T3, T4, T7 and T8, with "How T1, T2, T3, T4, T7 and T8 read the changes", from [§2.5](DESIGN.md#25-review-triggers-does-it-need-a-human), `approve`'s second run of the triggers from [§2.7](DESIGN.md#27-human-actions), the `review` settings and their defaults from [§4.2](DESIGN.md#42-config), and `files` in a hold, from [§4.3](DESIGN.md#43-item-file). `show --diff` listing the files that fired a trigger first is in [§3](DESIGN.md#3-interface)'s table. It's tracked in `tracker` itself as item TR-6, with the binary installed from slice 9 ([Tracking TR-6](#tracking-tr-6)). There are no reviews of this slice yet.

| | |
| --- | --- |
| Status | Every _Done when_ check has a test, and every test passes, as does the test command. TR-6 is `done`, verified `auto` at `e650ecf` by the installed slice 9 binary, which ran G2, G5, G6, G3 and G4, then T6 and T9, and held nothing, with `refs/tracker/TR-6` pointing at that commit. All seven of its checks failed at `start`, as they should. The builder added criteria 6 and 7 to PLAN.md's five, and every one has a check ([Tracking TR-6](#tracking-tr-6)). `b7ba4ee` changed six existing tests before `start`, because their pinned trigger lists grew ([Older tests that changed](#older-tests-that-changed)). Not done: `approve`'s new question, which names the `HEAD` and marks each hold "(new since the submit)", hasn't been seen in a real terminal. The slice 9 check by hand predates that change ([Checks done by hand](#checks-done-by-hand)). Two gaps are stated but not yet in DESIGN.md §7 ([Still open](#still-open)). There are no reviews yet. PLAN.md has slice 10 as `in review`. The branch isn't merged into `main`, and neither are slices 7, 8 and 9, which it's stacked on. |
| Branch | `feat/tracker/slice-10`, stacked on `feat/tracker/slice-9` at `71826d8`. Slice 9 isn't merged into `main`, so this slice's commits are `git log --oneline --reverse feat/tracker/slice-9..feat/tracker/slice-10`. In order: `b7ba4ee` (TR-6's tests, fixture XX-22 and four older tests' expected values, committed before `tracker start TR-6`), `6955644` (PLAN.md: slice 10 in progress), `2a4d5df` (the triggers, the config, `git::changes`, the schemas and `approve`), `9475fb3` (help text), `e650ecf` (DESIGN.md) and `2c9da0b` (PLAN.md: slice 10 in review) |
| Test command | `cargo fmt --check && cargo clippy -- -D warnings && cargo test`, run in `tracker/`. All three pass. |
| Tests | 371: 39 in `tests/create_and_list.rs`, 36 in `tests/commits.rs`, 34 in `tests/organise.rs`, 33 in `tests/risky_changes.rs`, 28 in `tests/review.rs`, 27 in `tests/states.rs`, 25 in `tests/show_and_delete.rs`, 23 in `tests/criteria.rs`, 22 in `tests/gates.rs`, 18 in `tests/checks.rs`, 14 in `tests/item_files.rs`, 13 in `tests/bad_data.rs`, 13 in `tests/fixtures.rs`, 11 in `tests/docs_links.rs`, 9 in `tests/polish.rs`, 6 in `tests/output.rs`, 5 in `tests/schemas.rs`, 4 in `tests/store_writes.rs`, 2 in `tests/item_ids.rs`, 7 unit tests in the library (4 in `src/gates.rs`, 1 each in `src/store.rs`, `src/checks.rs` and `src/git.rs`) and 2 in `src/main.rs` |

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
- [Still open](#still-open)
- [Limits, and what later slices change](#limits-and-what-later-slices-change)

## What's in this slice

**Built:**

- **Six triggers**, which run on `submit` and `status <id> done` once every gate has passed, beside slice 9's T6 and T9. Each message says why, and the file triggers name their files:
  - **T1** fires when the item's kind is `capability`, or an item commit added a source file: "new source file: src/search.rs".
  - **T2** fires when a changed file matches `risky_paths`: "risky file changed: src/store.rs".
  - **T3** fires when a dependency manifest or lockfile changed: "dependency manifest changed: Cargo.toml".
  - **T4** fires when an existing test file had a line changed or removed, or was deleted: "existing test file modified: tests/sums.rs".
  - **T7** fires when the item's commits changed more lines than `max_diff_lines`: "34 lines changed, more than the project's max_diff_lines of 20".
  - **T8** fires when the project's `design` file changed: "the project's design doc changed: docs/DESIGN.md".
- **The item's changed files**, worked out from its commits as §2.1 defines them: every file those commits touched, deleted ones included, each commit compared with its first parent, with paths made relative to the project's root before they're matched against a glob. A file outside the root counts toward T7 and matches no glob.
- **The project's `review` settings** in `config.json`: `risky_paths`, `source_paths`, `test_paths`, `manifests`, `max_diff_lines` and `disabled`, each with its default ([below](#the-review-settings)). `disabled` never switches off T6, T10 or T12. A glob that can't be parsed makes every command exit 3.
- **`files` in a hold**: a hold from a file trigger lists the files that fired it, relative to the root, in `holds` and in `verification.triggers`.
- **`show --diff`** prints "Files that fired a trigger:", each as the trigger and the file, before the commits and the diff, from the item's `holds`, or from a done item's approved `verification.triggers`.
- **`approve` runs the triggers again** at the `HEAD` it names, before it asks. Its question names that `HEAD` and every hold that fires there, marking each one that isn't among the submit's "(new since the submit)". If G5's `HEAD` then isn't the one the question named, it writes nothing and exits 1. The holds it showed go in `verification.triggers` (slice 9 review 1's design 1).
- **A hold's `files` in the schemas**: `submit.schema.json` and `gates.schema.json` now take their `holds` from one definition in `item.schema.json`.

**Not built yet:**

- **T5, T13 and T14** (slice 11). A check that passed at start, a criterion changed after start and an unscoped commit since start still hold nothing.
- **T10, T11 and T12** (slices 12, 14 and 15), the judge triggers, and `approve` counting held judge questions as passed (slice 15). `approve` runs no judges, since there are none yet.
- **The digest** (slice 16), which lists every `approve`, `reject` and edit that weakens T1 or T9, and `review`'s flag for an overdue digest.
- **DESIGN.md §7's lines** for the two limits in [Still open](#still-open): what "existing" lets through, and how T7 counts.

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
  T4  existing test file modified: tests/sums.rs
  T7  34 lines changed, more than the project's max_diff_lines of 20
  T8  the project's design doc changed: docs/DESIGN.md
$ tracker submit BM-1
Running criterion 1's check: `test -f src/search.rs`
BM-1: in-progress → review
Every gate passed, and BM-1 is held for review:
  T1  new source file: src/search.rs
  T2  risky file changed: src/store.rs
  T3  dependency manifest changed: Cargo.toml
  T4  existing test file modified: tests/sums.rs
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
  9e93176  feat(BM-1): search by tag
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
    "message": "existing test file modified: tests/sums.rs",
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

- The commit's SHA, and the times in an item file, differ on every run.
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
| `tracker approve <id>` | Changed. Before it asks, it runs the triggers again at `HEAD`, and exits 3 for a root that isn't in a git work tree. The question names `HEAD` and each hold, marking each new since the submit. After the gates it exits 1, writing nothing, if G5 saw a different `HEAD`. The record's `triggers` are the holds shown. |
| `tracker show <id> --diff` | Changed. "Files that fired a trigger:" comes first, one line each for a trigger and file, then the commits and the diff. Left out when no hold names a file. |

The help text for `submit` names the eight triggers and says the project's review settings tune them, and `approve`'s says it runs the triggers again at `HEAD` and exits 1 if `HEAD` moved while it asked.

Exit codes in this slice, as §3 defines them:

- `0`, `1` and `2`: as in slice 9, plus `approve` exiting 1 when `HEAD` moved while it waited.
- `3`: as before, plus any command, when a glob in a project's `review` settings can't be parsed. Also `submit`, `status <id> done`, `gates` and `approve`, when the project's root can't be found or git can't list what the item's commits changed, and `approve` when its root isn't in a git work tree.

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

The defaults are the constants in [src/config.rs](../src/config.rs), and `Review`'s methods return them. A setting that's present replaces its default, it doesn't add to it. Each glob is matched against the changed file's path relative to the project's root, and `*` doesn't match `/`, so `*.js` names only files at the top of the root and `**` matches any depth. `config::glob_set` builds a set with `literal_separator` on, which gives that rule. `disabled` is read by `Review::runs`: a name that isn't a trigger's is ignored, so a typo like `"T99"` silences nothing and says nothing.

## How it works

### The item's changed files

`triggers::changes(root, id)` in [src/triggers.rs](../src/triggers.rs) returns a `Changes`, which every file trigger reads. The item's commits come from `git::item_commits`, the same match as G5's, oldest first. `git::changes` in [src/git.rs](../src/git.rs) runs one `git show` over them with `-z --no-renames --diff-merges=first-parent --no-ext-diff --no-textconv --raw --numstat` and `log.showRoot=true`, and returns each commit's `FileChange`s: the path from the top of the repository, git's status letter, and the lines added and removed, or `None` for a binary file. A rename is a delete and an add, a merge commit is compared with its first parent, and a root commit with nothing. `git::prefix` gives the root's path from the top of the repository, such as `tracker/`, which `changes` strips from every path. A file that doesn't start with it is outside the root and counts only toward `Changes::lines`.

For each change inside the root, in commit order:

1. `lines` adds the file's added and removed lines. This runs before the root check, so a file outside the root counts toward T7.
2. `A` puts the path in `added`. A path in `added` isn't existing.
3. `D` on an existing file adds it to `existing_changed` as deleted. Any other status on an existing file adds it as modified if it removed at least one line, or if it's binary. A changed line is a removed line and an added one, so it counts, and a file that only gained lines doesn't.
4. The path goes in `files`, whichever of those happened.

"Existing" means no item commit has added the file, not that it was in the repository before `start`. So a test file that the item's own `test(XX-1)` commit added, and a later commit changed, isn't existing, and T4 stays quiet. A file that an item commit added and a later one deleted stays in `added`, so T1 would still list it, which no test covers.

### The triggers

`triggers::run(item, config)` finds the project's `review` settings and `design`, calls `gates::root` for the root, builds `Globs` from the four glob lists, calls `changes`, and runs each trigger in order, T1, T2, T3, T4, T6, T7, T8, T9, adding a `Ran` only when `Review::runs` says it's on. A `Ran` now has `files` beside `hold`. `run` returns a `Result`, since a root that can't be found, or git failing, exits 3, as does a glob that can't be built. `holds` copies each fired `Ran`'s `files` into the `Hold`.

- `t1` collects `added` paths that `Globs::is_source` accepts, which is a match on `source_paths` and none on `test_paths`. A capability gets "it's a capability, so a human should check the design fit", or "it's a capability, and adds the source file …" when it added any. Any other kind gets "new source file: …" for one or more files. A new source file includes a new top-level module, since any added source file counts, so the code has no separate rule for one.
- `t2` collects `files` that match `risky_paths`. With no `risky_paths` it never fires.
- `t3` collects `files` that match `manifests`.
- `t4` collects `existing_changed` paths that match `test_paths`, then says "existing test file modified: …", "existing test file deleted: …", or both, joined by "; ". A file both modified and deleted is listed as deleted.
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
4. `decision_warning(item, holds, what)` prints each hold and marks it "(new since the submit)" when `item.holds` doesn't contain it. A hold with the same trigger and message but different `files` counts as new, since `Hold` compares all three fields. `reject` calls it with `item.holds`, so nothing it prints is marked. The question's second line says approving "runs the gates again at <short HEAD>".

Then, as in slice 9, `Store::lock_unchanged`, the statics under the lock, the scripts and G5's second look without it, and the re-read. A failed gate exits 2. If none failed, one more check comes before anything is written: if `report.head`, the `HEAD` G5 saw, isn't the `HEAD` the question named, `approve` exits 1 with "HEAD moved from … to … while XX-1's approval waited … Nothing was changed. Run `tracker approve XX-1` again to see them at the new HEAD." Otherwise `set_ref_at_head`, and `item.holds` is set to the holds that were shown, so `verification_record` copies them into `triggers`, and the move to done empties `holds` as before.

So a commit made while I read the question is never approved unseen. A commit made before it is shown, and its holds are marked.

### Show --diff lists the files first

`show` in [src/main.rs](../src/main.rs) calls `trigger_files(item)` when `--diff` is given. It reads the item's `holds`, or, when there are none, the `triggers` of its verification record, so a done item that a human approved still lists them. Each file in each hold's `files` is printed as `  T3  Cargo.toml`, through `text::for_terminal`, under "Files that fired a trigger:", before "Commits," and "Diff,". A hold with no `files`, such as T7's, or one written by slice 9, adds nothing, and with none the section is left out. The JSON is unchanged: the item is in `item`, so `files` are in its holds.

### The config

`config::load` in [src/config.rs](../src/config.rs) builds each project's four glob lists once it has parsed the file, so a glob that can't be built exits 3 with the project key and the glob, "… gives IM a review setting that can't be used: "tests/[x" isn't a valid glob: … Fix it there.", whichever command was run. This is the same refusal a bad root gets, so a trigger is never unable to run at a submit. `Project::review` returns the settings or the defaults. serde skips what it doesn't know in `review`, and `Project::review` is `None` until set, so a `config.json` without it comes back as it was.

## JSON output

| Command | Schema | What changed |
| --- | --- | --- |
| `submit`, `status <id> done`, `gates` | [submit.schema.json](../schemas/submit.schema.json), [gates.schema.json](../schemas/gates.schema.json) | `triggers` lists each trigger that ran, T1, T2, T3, T4, T6, T7, T8, T9, minus any `review.disabled` names. `holds`' items are now `item.schema.json`'s `hold`, so they may have `files` |
| every item | [item.schema.json](../schemas/item.schema.json) | `hold` gains `files`: an array of at least one non-empty string, left out for a trigger that isn't about files. `holds` and `verification.triggers` both use it |
| `show --diff` | [show.schema.json](../schemas/show.schema.json) | No change. The files are in the item's holds |

An item file written by slice 9 has holds without `files`, and comes back byte for byte, since `Hold::files` has a serde default and `skip_serializing_if`. `tests/fixtures/slice-10/XX-22.json` is a held item written by slice 10's binary. Slices 11 and 12 add T5, T13, T14 and T10 to `triggers`, which needs no schema change, since `trigger`'s pattern allows them. A trigger that names files, such as slice 11's T14, which lists commits, can use `files` as T1 to T4 and T8 do, or add a field.

## Code tour

| File | What's new or changed |
| --- | --- |
| [src/triggers.rs](../src/triggers.rs) | `Ran::files`, `Changes`, `changes`, `Globs`, `file_list`, `files_word`, and `t1`, `t2`, `t3`, `t4`, `t7` and `t8`. `run(item, config)` returns a `Result` and skips triggers the project switches off. `holds` copies `files` |
| [src/config.rs](../src/config.rs) | `Review`, with `risky_paths`, `source_paths`, `test_paths`, `manifests`, `max_diff_lines` and `disabled`, its default constants, `ALWAYS_ON` and `runs`. `glob_set`, `Project::review`, and `load`'s refusal of a glob that can't be built |
| [src/git.rs](../src/git.rs) | `prefix`, `FileChange` and `changes` |
| [src/item.rs](../src/item.rs) | `Hold::files` |
| [src/main.rs](../src/main.rs) | `gates_dry_run`, `apply` and `approve` pass the config to `triggers::run`. `approve` finds the root, runs the triggers and names `HEAD` before it asks, and exits 1 if G5's `HEAD` differs. `decision_warning` takes the holds and marks new ones. `show` calls the new `trigger_files`. `project_add` writes `review: None`. Help text for `Submit` and `Approve` |
| [Cargo.toml](../Cargo.toml), `Cargo.lock` | `globset` |
| [schemas/](../schemas/) | `item.schema.json`: `hold.files`. `submit.schema.json` and `gates.schema.json`: `holds` refer to it |
| [tests/risky_changes.rs](../tests/risky_changes.rs) | New (`b7ba4ee`): one module per TR-6 criterion, `criterion_1` to `criterion_7`, with 33 tests in all, 2 outside them. Helpers `started_with`, `write`, `remove`, `commit`, `set_review`, `set_project_field`, `submit`, `fired`, `ran`, `hold`, `assert_held_by`, `assert_not_held` and `has_ref` |
| [tests/fixtures.rs](../tests/fixtures.rs), `tests/fixtures/slice-10/` | XX-22 and `slice_10s_held_fixture_names_the_files_that_fired_each_hold` (`b7ba4ee`) |
| [tests/states.rs](../tests/states.rs), [tests/gates.rs](../tests/gates.rs), [tests/review.rs](../tests/review.rs) | Six tests' expected `triggers` list grew to eight (`b7ba4ee`), [below](#older-tests-that-changed) |
| [docs/DESIGN.md](DESIGN.md) | `e650ecf`: §2.5's "How T1, T2, T3, T4, T7 and T8 read the changes", §2.7's `approve` bullet, §4.2's `disabled` and glob rules and §4.3's `holds` |
| [docs/PLAN.md](PLAN.md) | Slice 10's status, `todo` to `in progress` (`6955644`) to `in review` (`2c9da0b`) |

To add a trigger that reads the item's changed files, as slice 11's T14 may: add its function to [src/triggers.rs](../src/triggers.rs), return the message and the files, and add an `add("T14", …)` line to `run` in trigger order. Read `Changes` for files, or extend `changes` and `git::changes` for what they don't hold, such as an author. A trigger that can be switched off needs nothing else, since `add` asks `Review::runs`. One that mustn't be, as T10 and T12, goes in `config::ALWAYS_ON`.

## Testing

Run the test command in `tracker/`. TR-6's criteria each run their own module in `tests/risky_changes.rs`, for example `cargo test --test risky_changes criterion_4::`. The module path ends in `::`, so `criterion_1::` can't match a `criterion_10`.

Every project in `tests/risky_changes.rs` is XX, whose root `<work>/xx` is a subfolder of a throwaway repository, as `Env::new` makes it, so a path from the top of the repository differs from one from the root, as PLAN.md's integration risk "Self-tracking depends on the folder rule" asks. `started_with` writes and commits each file of a base as an unscoped commit, creates XX-1 and starts it, so the files exist before the item's work. Each test then writes files, commits them as `feat(XX-1): the work`, and submits. Three tests run `reject` or `approve` under `script` with `Env::in_terminal_typing`, which returns `None` where `script` isn't installed, and each then returns early, passing: `criterion_5`'s and both of `criterion_6`'s. Nothing in [tests/common/mod.rs](../tests/common/mod.rs) changed.

### Coverage of the _Done when_ checks

| _Done when_ | Tests in `tests/risky_changes.rs` |
| --- | --- |
| Each of the six triggers has one test where it fires and one where it stays quiet. | `criterion_1`, 17 tests. T1: `t1_fires_for_a_new_source_file` (the message and `files`), `t1_fires_for_a_capability`, `t1_stays_quiet_for_a_change_to_an_existing_source_file` (T1 still listed in `triggers`) and `t1_stays_quiet_for_a_new_test_file`, with `tests/**` named in `source_paths`. T2: `t2_fires_for_a_risky_path` and `t2_stays_quiet_for_a_path_that_isnt_risky`. T3: `t3_fires_for_a_changed_manifest` and `t3_stays_quiet_when_no_manifest_changed`. T4: `t4_fires_for_a_changed_line_in_an_existing_test_file` and `t4_stays_quiet_for_a_new_test_file`. T7: `t7_fires_for_more_changed_lines_than_max_diff_lines`, `t7_fires_by_default_above_300_lines`, `t7_counts_lines_changed_outside_the_root` and `t7_stays_quiet_at_max_diff_lines`. T8: `t8_fires_when_the_design_doc_changes` and `t8_stays_quiet_when_another_doc_changes`. And `the_gates_dry_run_lists_the_same_holds` |
| Adding a test case to an existing test file doesn't fire T4. Changing a line in one does, and so does deleting one. | `criterion_2`, 5 tests: `adding_a_test_case_to_an_existing_test_file_doesnt_fire_t4`, `changing_a_line_in_an_existing_test_file_fires_t4`, `deleting_an_existing_test_file_fires_t4` (the message and `files`), `renaming_an_existing_test_file_fires_t4` and `changing_a_test_file_the_item_added_doesnt_fire_t4` |
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
- Not tested: `approve`'s exit 3 for a root outside a git work tree, T1's file list for a file added and then deleted, a binary file counting as a changed test, a merge commit, T8 for a design path with `./`, and the `manifests` default for a manifest in a subfolder.

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

The commit message says four, and its diff has six. T4 wouldn't fire for any of them, since they only gained lines. From slice 11, editing an existing test fires T4 whichever commit does it, so a later change to one of these lists, as slice 11's T5, T13 and T14 will make, should go in the item's test commit before `start`.

### Checks done by hand

Each was run with this branch's `target/debug/tracker` in a temporary `TRACKER_HOME`:

| What | How | Result |
| --- | --- | --- |
| The demo above | Run as written, with stderr captured | The output above, with the temporary folder replaced by `$DEMO`. Each command marked with an exit code exited with it, and every other one exited 0 |
| `approve`'s new question in a real terminal | Not done | The question names the `HEAD` and marks each hold "(new since the submit)". PLAN.md's integration risk, "Interactive-only commands are only half tested", asks for each interactive command's prompt to be tried once. The user's check on 2026-11-22 of `approve` and `reject`, with the installed slice 9 binary ([slice 9's readme](slice-9-readme.md#checks-done-by-hand)), came before this change, so it didn't see the new question. The tests type the ID into it under `script`, which §7 says looks the same to `tracker` as a real terminal |

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

## Decisions worth knowing

- **"Existing" means no item commit added the file.** §2.5 says so. T4's "existing test file" and T1's "new source file" both ask whether one of the item's commits added it, not whether it was in the repository at `start`. The effect is that a test the item added itself is free to change, and an item can weaken one after `start` without firing T4 ([Still open](#still-open)).
- **Changed files come from one `git show` of every item commit, each against its first parent.** A rename is a delete and an add, so renaming a test file fires T4. `-z` keeps odd file names whole.
- **T7 counts every commit's lines, summed**, not the net diff from before the first commit to `HEAD`. An item that adds a file and then removes it counts both. Tests, files outside the root and deleted files are included, since a big change hides surprises wherever it is.
- **A typo in `disabled` is ignored.** `Review::runs` asks only whether a name is in the list. `"T99"` silences nothing, and so does `"t8"`. DESIGN.md §4.2 says so, and a misspelled name leaves the trigger on, which fails safe.
- **A glob that can't be parsed exits 3 for every command**, at `config::load`, as a bad root does, rather than failing a submit after its gates have run. The error names the project and the glob.
- **A switched-off trigger isn't listed in `triggers`.** §3's `triggers` says which ran, and a trigger that didn't run has no `fired: false` to report. `ran` in the tests asserts it.
- **`risky_paths` is empty by default**, so T2 never fires until a project sets it, since only the project knows which paths are risky. TR sets none yet. `manifests` defaults to the usual manifests and lockfiles, at any depth.
- **`approve` runs the triggers before it asks, not after the gates.** What I'm shown has to be what's recorded. If the triggers ran after the question, a commit made while I read it could add a hold I never saw. Running them before, and exiting 1 if `HEAD` moved by the time G5 looked, closes that. The holds it shows are the ones it keeps, so a hold from a commit made since the submit goes in `verification.triggers` with the submit's.
- **`approve` runs the triggers once, outside the lock**, with the item as it read it. `Store::lock_unchanged` confirms the file hasn't changed, and the `HEAD` comparison confirms git hasn't.
- **`approve` exits 3 before its question for a root that isn't in a git work tree**, as `submit` does, since the triggers need git to read the item's commits.
- **A hold's `files` are relative to the root**, not the repository, so they match the globs and what the project's own tools print. `show --diff` prints the diff itself with paths from the top of the repository, as `git show` does.
- **T7's hold has no `files`**, since it isn't about particular files, and `show --diff`'s list omits it. Slice 11's T14 lists commits, which also aren't files.
- **TR's own `review` config is the user's to set**, since it changes what holds TR's own items ([Limits](#limits-and-what-later-slices-change)). This slice sets nothing there.

## Still open

- **DESIGN.md §7** (the builder's note on TR-6, with no review yet): T1 and T4 call a file "existing" when no item commit added it, so an item can weaken a test it added itself, after `start`, without firing T4. §2.5 states the rule, but §7 has no line saying what it lets through. From slice 11, T4 holds an item for editing an existing test whichever commit does it, which doesn't close this, since the file isn't existing.
- **DESIGN.md §7**: T7 counts every commit's lines, summed, so an item that adds a large file and then removes it is held for both,. §2.5 says "the lines each commit added and removed, summed", but §7 doesn't give it as a limit.
- **Process** (the builder's note): `approve`'s new question, which names the `HEAD` and marks holds "(new since the submit)", hasn't been seen in a real terminal ([Checks done by hand](#checks-done-by-hand)). It's the user's to check by hand, with this slice's binary in a throwaway store, as slice 9's was.
- **Test gaps** (the builder's note): `approve`'s exit 3 for a root outside a git work tree, T1's list for a file added and then deleted, a binary file counting as a changed test, a merge commit, a design path with `./` and the `manifests` default at depth have no test ([Other tests](#other-tests)).
- **Carried from slice 9** ([slice 9's Still open](slice-9-readme.md#still-open)): `review`'s line for a held item with no `holds` and `show --diff`'s text for an item with no commits have no test, and neither does `approve`'s first re-read as a plain `store.lock()`. `deletion_warning`'s wording for an `unchecked` record with one check, from slice 8, and `Env::add_criterion`'s `--check true`, left for slice 11, are also still open.
- **Slice 9's "Slice 10 must decide" points**: design 1 is fixed by criterion 6, `disabled` never switching off T6 and the `holds` shape are done, and TR's criteria and tree are as [above](#tracking-tr-6). Which of T1, T4 and T8 to switch off for TR is still the user's to decide.

## Limits, and what later slices change

- **Slice 11** adds T5, T13 and T14. T14 reads commits between `started.head` and `HEAD`, which `Changes` doesn't, since it holds every item commit's files, so T14 needs a log of its own, or `git::changes` over a range. It lists commits, not files. T13 holds every test item whose check passed at start, and `Env::add_criterion`'s check is `true`, so most tests that submit would be held. Changing the helper to a check that fails at start, as slice 7's readme asks, avoids that. T5's fingerprint is taken at `start`, so it must not change while an item is in progress. From slice 11, editing an existing test fires T4 and holds the item whichever commit does it, so an item that fixes a test pinned by an earlier slice, as `b7ba4ee` did, is held. This slice's `changes` is the base: a T4 that reads commits since `start` would have to treat "existing" as this slice does, or the gap in [Still open](#still-open) stays.
- **Slice 12** adds T10, which `config::ALWAYS_ON` already lists, with `verification.judges`. `run` then needs the judge settings and the cache, beside the config it takes now. A `human` record then has `judges` and `triggers`, and `approve` runs the gates again, so it runs the judges again too, from the cache when nothing changed (§2.3.1). `approve` also runs the triggers again, before the question and outside the lock, so T10's judge answers come from the cache there too.
- **Slice 15** makes `approve` count each judge question held under T10 or T12 as passed for that approval only, and has `approve` and `reject` record the decision against each held question. `approve` now shows the holds at `HEAD`, and `verification.triggers` holds those, so a held question's record has to go where `approve` reads it before it asks, as the triggers' holds do.
- **Slice 16**'s digest lists every `approve` and `reject`, and every edit that removes `needs-review` or moves a capability's kind, which switch off T9 and T1. It lists nothing about a change to `config.json`, where `review.disabled` and the globs live, and PLAN.md's slice 16 gives `audit` no check of it. §7 says the store files are plain text.
- **A held item pauses its project** until it's decided (§4.4), and from slice 11 on, slice 10's binary will hold nearly every TR item. TR-7 to TR-12 are `capability`, so T1 holds each. A change to `docs/DESIGN.md`, TR's design file, fires T8. A new dependency or a changed `Cargo.lock` fires T3, and an item with more than 300 changed lines fires T7. T2 holds nothing unless TR's config sets `risky_paths`, such as `src/store.rs` and `src/gates.rs`, which §4.2's example names. Which of T1, T4, T7 and T8 TR's `review` switches off, and whether it sets `risky_paths`, is the user's to decide, because it changes what holds TR's own items. T6, T10 and T12 can't be switched off.
- **`show --diff` runs `git show` once with every item commit**, and `git::changes` runs it once too, so an item with many commits reads all of them each time `submit`, `gates` and `approve` run the triggers. A project with a long history of item commits would feel it, as G5's `item_commits` already scans everything reachable from `HEAD`.
- **The globs are compiled on every command**: `config::load` builds each project's four sets to check them, and `triggers::run` builds them again. A project with large glob lists pays twice.
- **`approve` and `reject` are only as interactive as stdin.** `script` gets past the terminal check, as the tests show, and §7 says so: an approve made that way records `"interactive": true` and looks like mine, which only the digest (slice 16) can show.
- **The tests that put an item in review by editing its file**, in `tests/states.rs`, `tests/gates.rs`, `tests/criteria.rs`, `tests/organise.rs` and `tests/show_and_delete.rs`, still do. They test refusals that don't depend on how the item got there.
- **A subfolder project's files outside the root** count toward T7 and appear in `show --diff`, and match no glob (§2.1). A project whose risky files live outside its folder has to be registered with the repository's top as its root, as §7 says of G5.

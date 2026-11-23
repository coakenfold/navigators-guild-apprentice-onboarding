# Slice 14: Judges must fail first

`tracker` is a command-line work tracker, written in Rust, that keeps one central list of work items across many projects and decides when an item is really done. This document covers **slice 14** of the 16 in [PLAN.md](PLAN.md): what it built, how it works, and how to test it. The earlier readmes cover the rest: [slice 1](slice-1-readme.md) the store, `create` and `list`, [slice 2](slice-2-readme.md) the states and `history`, [slice 3](slice-3-readme.md) priority, kind, labels, `edit` and `list`'s filters, [slice 4](slice-4-readme.md) `show` and `delete`, [slice 5](slice-5-readme.md) bad data and polish, [slice 6](slice-6-readme.md) criteria and gates G1, G2 and G6, [slice 7](slice-7-readme.md) running checks, with gates G3 and G4 and `started.checks`, [slice 8](slice-8-readme.md) G5, the `auto` record and `refs/tracker/<ID>`, [slice 9](slice-9-readme.md) the inbox, `approve`, `reject` and triggers T6 and T9, [slice 10](slice-10-readme.md) triggers T1, T2, T3, T4, T7 and T8, [slice 11](slice-11-readme.md) triggers T5, T13 and T14 and the criteria fingerprint, [slice 12](slice-12-readme.md) judge criteria, G3's judge half and trigger T10, with a fake judge command, and [slice 13](slice-13-readme.md) `tracker-judge`, the real judge command.

After slice 14 you can **have judges that pass before the work, or get retried until they pass, held**. The spec is [DESIGN.md](DESIGN.md). This slice implements what [§2.4](DESIGN.md#24-gates-is-it-finished) says about judges at `start` and about `gates` dry runs, trigger T11 and T13's judge half from [§2.5](DESIGN.md#25-review-triggers-does-it-need-a-human), G1's judge half from [§2.3.1](DESIGN.md#231-judges-claude-via-the-claude-cli), `judge/failures.json` from [§4.1](DESIGN.md#41-layout), `judge.max_failures` from [§4.2](DESIGN.md#42-config), and `started.checks` for judges from [§4.3](DESIGN.md#43-item-file). It also fixes the list of commands that run checks or judges in [§7](DESIGN.md#7-honest-limits). The slice is tracked in `tracker` itself as item TR-10, with the binary installed from slice 13 ([Tracking TR-10](#tracking-tr-10)). It has no review yet.

| | |
| --- | --- |
| Status | Every automated _Done when_ check has a test, and every test passes, as does the test command (`cargo fmt --check`, `cargo clippy -- -D warnings` and `cargo test` were run on the finished branch). TR-10 is `done`, but it wasn't auto-verified. It has six criteria: PLAN.md's four _Done when_ bullets, as written, and two the orchestrator added before `start` from [slice 13's review 2](reviews/slice-13-review-2.md): criterion 5, "the test harness puts a failing `tracker-judge` and `claude` first on `PATH`" (process 1), and criterion 6, "§7's list of commands that run checks or judges" (design 2). All six checks failed at `start`. The installed slice 13 binary held the submit under T1 (the new source files), T4 (`tests/judges.rs`, whose existing tests were changed in `3fa1f52`, before `start`), T7 (1161 lines changed) and T8 (DESIGN.md §2.4, §2.5, §4.1, §4.3 and §7). The user read the diff and approved it from a terminal: `verification.by` is `human`, at `fdaa0cb`, with `refs/tracker/TR-10`, and the four holds are recorded in `verification.triggers`. Before the approve, the orchestrator checked by hand that the four written criteria are unchanged, that all six checks failed at `start`, and that every commit since `start` is scoped to TR-10. **One slip:** six trigger lists pinned in `tests/gates.rs`, `tests/review.rs` and `tests/states.rs` needed T11 added, and that was done after `start`, in `d202442`, so those six additions weren't seen failing first ([Testing](#testing)). They are additions only, so T4 stayed quiet. No review of slice 14 exists, so there are no review fixes. Left for later on purpose: TR-47, TR-48 and TR-50 are deferred ([Still open](#still-open)). PLAN.md has slice 14 as `in review`. The branch isn't merged into `main`, and neither are slices 7 to 13, which it's stacked on. |
| Branch | `feat/tracker/slice-14`, stacked on `feat/tracker/slice-13` at `c0e758a`. Slice 13 isn't merged into `main`, so this slice's commits are `git log --oneline --reverse feat/tracker/slice-13..feat/tracker/slice-14`. In order, all scoped to TR-10: `3fa1f52` (the tests, committed before `tracker start TR-10`), `4b76943` (PLAN.md: slice 14 in progress), `d202442` (T11 in the six pinned trigger lists), `519e61d` (the code), `d2ad26c` (DESIGN.md and the schemas) and `fdaa0cb` (PLAN.md: slice 14 in review). That is 6 commits. This readme is one more, and it belongs to no item |
| Test command | `cargo fmt --check && cargo clippy -- -D warnings && cargo test`, run in `tracker/`. All three pass at `fdaa0cb` |
| Tests | 494: 46 in `tests/risky_changes.rs`, 42 in `tests/judges.rs`, 39 in `tests/create_and_list.rs`, 36 in `tests/commits.rs`, 34 in `tests/organise.rs`, 28 in `tests/review.rs`, 27 in `tests/states.rs`, 25 in `tests/show_and_delete.rs`, 23 in `tests/criteria.rs`, 22 in `tests/gates.rs`, 21 in `tests/tampered_proof.rs`, 20 in `tests/judges_first.rs`, 18 in `tests/checks.rs`, 15 in `tests/fixtures.rs`, 14 in `tests/item_files.rs`, 13 in `tests/bad_data.rs`, 12 in `tests/tracker_judge.rs`, 11 in `tests/docs_links.rs`, 9 in `tests/polish.rs`, 6 in `tests/output.rs`, 6 in `tests/t5_and_fingerprint.rs`, 5 in `tests/schemas.rs`, 4 in `tests/store_writes.rs`, 2 in `tests/item_ids.rs`, 1 each in `tests/harness_check.rs` and `tests/submit_help.rs`, 12 unit tests in the library (4 in `src/gates.rs`, 4 in `src/judge.rs`, 2 in `src/git.rs`, 1 each in `src/store.rs` and `src/checks.rs`) and 2 in `src/main.rs`. That is 20 more than slice 13's 474, all in the new `tests/judges_first.rs` |

## Contents

- [What's in this slice](#whats-in-this-slice)
- [Try it](#try-it)
- [Commands](#commands)
- [What judge/failures.json holds](#what-judgefailuresjson-holds)
- [How it works](#how-it-works)
- [Code tour](#code-tour)
- [Testing](#testing)
- [Decisions worth knowing](#decisions-worth-knowing)
- [Still open](#still-open)
- [Limits, and what later slices change](#limits-and-what-later-slices-change)

## What's in this slice

**Built:**

- **G1 refuses a judge with nothing to grade.** At `start`, [src/gates.rs](../src/gates.rs) `g1_judges` asks [src/judge.rs](../src/judge.rs) `nothing_at_start` about each judge criterion. A `diff` judge needs at least one item commit. A file input must exist and be readable. An `each` must match the inputs once comments are stripped. Nothing is asked, and each reason says to commit a stub as `test(<ID>): …` first.
- **`start` runs every judge once.** After G1 and outside the lock, `gates::grade_judges` grades each judge criterion, as G3 does at submit, and the result goes in `started.checks` beside the script checks'. A judge passed at start only if every question got its passing answer. An unsure answer, or a call that failed, didn't pass.
- **A judge that can't be graded doesn't refuse the start.** `start` warns, records the judge as not passing, and G3 asks it again at submit.
- **T13 reads judges.** A judge whose answers all passed at start holds the item, and `start` warns about it. T13's message says "judge" or "check" for each criterion.
- **T11, `judge/failures.json` and `judge.max_failures`.** T11 holds an item once judges have failed `max_failures` (default 3) different requests since `start`. The requests are counted in a new file in the store, outside the item ([What judge/failures.json holds](#what-judgefailuresjson-holds)).
- **`gates` counts too, but only for an item in progress.** A dry run before `start` counts nothing, so an agent can see its judges fail on the stub. A `gates` run on an item in progress counts each new failed request, as a submit does.
- **The harness keeps every test off the real judge.** [tests/common/mod.rs](../tests/common/mod.rs) puts a `tracker-judge` and a `claude` that only fail first on `PATH` for every command a test runs ([Testing](#testing)).
- **DESIGN.md and the schemas say it.** §2.4 describes judges at `start` and `gates`' counting. §2.5's T11 row is rewritten and T13's mentions judges. §4.1, §4.3 and three schema descriptions follow. §7's interrupt bullet lists the commands that run checks and the ones that run judges.
- **Slice 13's "Slice 14 must decide" points**, from [review 1](reviews/slice-13-review-1.md) and [review 2](reviews/slice-13-review-2.md): see [Decisions](#decisions-worth-knowing).

**Not built yet:**

- **Disputes and T12** (slice 15). A wrong fail still counts toward T11 like any other.
- **`tracker judges`, the digest's judge entries and `audit`'s look at `judge/failures.json`** (slice 16).
- **TR-47** (a `diff` judge's block comments), **TR-48** (what else a real judge sees) and **TR-50** (escaping `</material>`, and pinning step 1's prompt). All three are deferred ([Still open](#still-open)).
- **Judges for a criterion given after `start`.** `criterion add --judge` and `criterion set --judge` don't run the judge ([Decisions](#decisions-worth-knowing)).

## Try it

Always set `TRACKER_HOME` while developing. Without it, the tool uses your real store at `~/.tracker`.

**This demo never calls the real `claude` or `tracker-judge`.** Its judge command is a shell script that reads the request and prints whatever is in `reply.json`, as `tests/judges_first.rs` does, and a small `reply` script writes that file, so the demo sets each answer. Four projects are folders in one throwaway repository, as TR is a folder in guild-portfolio:

- **BM-1** has a `diff` judge. `start` is refused until the item has a commit. The judge fails on the stub, then passes on the work, so BM-1 goes to done.
- **CM-1**'s judge passes at start, so `start` warns and T13 holds the submit.
- **DM-1**'s `max_failures` is 2. `gates` before `start` counts nothing, and neither does the request asked at `start`. Two different failed requests are counted, one of them twice over, and the submit is held by T11.
- **EM-1**'s judge command prints something that isn't a reply. `start` warns and goes ahead.

Each project's stub has its own text, since the cache is keyed by the request and not the item.

```sh
cd guild-portfolio/tracker
cargo build
export DEMO=$(mktemp -d)
export TRACKER_HOME=$DEMO/store
export PATH="$PWD/target/debug:$PATH"
export GIT_AUTHOR_NAME=demo GIT_AUTHOR_EMAIL=demo@example.invalid
export GIT_COMMITTER_NAME=demo GIT_COMMITTER_EMAIL=demo@example.invalid
mkdir -p $DEMO/judge $DEMO/repo
printf '#!/bin/sh\ncat > /dev/null\ncat "$(dirname "$0")/reply.json"\n' > $DEMO/judge/fake && chmod +x $DEMO/judge/fake
printf '#!/bin/sh\nprintf '"'"'{"adapter":"fake 1","model":"claude-haiku-5-5","answers":{"works":{"answer":"%%s","reason":"The fake says %%s."}}}'"'"' $1 $1 > "$(dirname "$0")/reply.json"\n' > $DEMO/judge/reply && chmod +x $DEMO/judge/reply
export PATH=$DEMO/judge:$PATH
printf '%s\n' '{"text":"The change is small","inputs":["diff"],"ask":[{"id":"works","ask":"Is this change small?"}]}' > $DEMO/judge/diff.json
printf '%s\n' '{"text":"The code does the work","inputs":["src/lib.rs"],"ask":[{"id":"works","ask":"Does this code do the work?"}]}' > $DEMO/judge/lib.json
cd $DEMO/repo && git init --quiet -b main
for p in bm cm dm em; do mkdir -p $p/src; printf 'fn run() {\n    todo!("%s")\n}\n' $p > $p/src/lib.rs; done
git add -A && git commit --quiet -m "chore: set up"
tracker init
jq --arg c "$DEMO/judge/fake" '.judge = {command: $c, model: "claude-haiku-5-5"}' $TRACKER_HOME/config.json > $DEMO/config.new && mv $DEMO/config.new $TRACKER_HOME/config.json
for p in bm cm dm em; do tracker project add $(echo $p | tr a-z A-Z) --name $p --root $DEMO/repo/$p; done
reply no
```

The demo itself, from the same shell. It was run line by line, as one script, with stdout and stderr together. A `$ ` line is the command, and `[exit N]` follows each one that fails:

```sh
tracker create "Small change" --project BM
tracker criterion add BM-1 --judge $DEMO/judge/diff.json
tracker start BM-1      # exit 2: a diff judge has no commit to grade yet
cd $DEMO/repo
printf 'fn run() {\n    todo!("bm stub")\n}\n' > bm/src/lib.rs
git add -A && git commit --quiet -m "test(BM-1): a stub for the judge to grade"
tracker start BM-1
tracker show BM-1 | grep '^  checks  '
printf 'fn run() {\n    step();\n}\n' > bm/src/lib.rs
git add -A && git commit --quiet -m "feat(BM-1): the work"
reply yes
tracker submit BM-1
tracker create "Already passes" --project CM
tracker criterion add CM-1 --judge $DEMO/judge/lib.json
reply yes
tracker start CM-1
tracker show CM-1 | grep '^  checks  '
printf 'fn run() {\n    step();\n}\n' > cm/src/lib.rs
git add -A && git commit --quiet -m "feat(CM-1): the work"
tracker submit CM-1
tracker create "Retried" --project DM
tracker criterion add DM-1 --judge $DEMO/judge/lib.json
jq '.projects.DM.judge = {max_failures: 2}' $TRACKER_HOME/config.json > $DEMO/config.new && mv $DEMO/config.new $TRACKER_HOME/config.json
reply no
printf 'fn run() {\n    todo!("dm stub")\n}\n' > dm/src/lib.rs
git add -A && git commit --quiet -m "test(DM-1): a stub for the judge to grade"
tracker gates DM-1      # exit 2: DM-1 is still open
jq 'has("DM-1")' $TRACKER_HOME/judge/failures.json
tracker start DM-1
tracker gates DM-1      # exit 2: the same request as at start
jq '.["DM-1"] | {at_start: (.at_start | length), failed: (.failed | length)}' $TRACKER_HOME/judge/failures.json
printf 'fn run() {\n    step(1);\n}\n' > dm/src/lib.rs
git add -A && git commit --quiet -m "feat(DM-1): version 1"
tracker gates DM-1      # exit 2: a new request fails
tracker gates DM-1      # exit 2: the same request, answered from the cache
printf 'fn run() {\n    step(2);\n}\n' > dm/src/lib.rs
git add -A && git commit --quiet -m "feat(DM-1): version 2"
tracker gates DM-1      # exit 2: a second new request fails
jq '.["DM-1"] | {at_start: (.at_start | length), failed: [.failed[] | .refs]}' $TRACKER_HOME/judge/failures.json
printf 'fn run() {\n    step(3);\n}\n' > dm/src/lib.rs; git add -A && git commit --quiet -m "feat(DM-1): version 3"
reply yes
tracker submit DM-1
tracker create "No login" --project EM
tracker criterion add EM-1 --judge $DEMO/judge/lib.json
printf 'Not logged in\n' > $DEMO/judge/reply.json
tracker start EM-1
tracker show EM-1 | grep '^  checks  '
tracker list --all --status in-progress,review,done
```

The output:

```
$ tracker create "Small change" --project BM
Created BM-1: Small change
$ tracker criterion add BM-1 --judge $DEMO/judge/diff.json
Added criterion 1 to BM-1, a judge: The change is small
Its question is asked at submit, once every other gate passes.
$ tracker start BM-1      # exit 2: a diff judge has no commit to grade yet
error: BM-1 can't be started: G1 failed. Nothing was changed.
  G1  Criterion 1's judge reads BM-1's diff, and BM-1 has no commits yet, so it has nothing to grade. Commit what it grades, or a stub it can grade, as `test(BM-1): …` before start, and the judge should fail on it (§2.3.1).
[exit 2]
$ cd $DEMO/repo
$ printf 'fn run() {\n    todo!("bm stub")\n}\n' > bm/src/lib.rs
$ git add -A && git commit --quiet -m "test(BM-1): a stub for the judge to grade"
$ tracker start BM-1
Asking criterion 1's judge 1 question: `$DEMO/judge/fake`
BM-1: open → in-progress
$ tracker show BM-1 | grep '^  checks  '
  checks  criterion 1 failed
$ printf 'fn run() {\n    step();\n}\n' > bm/src/lib.rs
$ git add -A && git commit --quiet -m "feat(BM-1): the work"
$ reply yes
$ tracker submit BM-1
Asking criterion 1's judge 1 question: `$DEMO/judge/fake`
BM-1: in-progress → done
Verified automatically: every gate passed at 2a902fa, and refs/tracker/BM-1 keeps that commit.
$ tracker create "Already passes" --project CM
Created CM-1: Already passes
$ tracker criterion add CM-1 --judge $DEMO/judge/lib.json
Added criterion 1 to CM-1, a judge: The code does the work
Its question is asked at submit, once every other gate passes.
$ reply yes
$ tracker start CM-1
Asking criterion 1's judge 1 question: `$DEMO/judge/fake`
warning: Criterion 1's judge already passes before the work is done: every answer is the passing one, so its questions can't tell the code now from the finished work. Make sure they ask about what the work changes, and that the judge grades a stub it fails. CM-1 started anyway, and T13 will hold it.
CM-1: open → in-progress
$ tracker show CM-1 | grep '^  checks  '
  checks  criterion 1 passed
$ printf 'fn run() {\n    step();\n}\n' > cm/src/lib.rs
$ git add -A && git commit --quiet -m "feat(CM-1): the work"
$ tracker submit CM-1
Asking criterion 1's judge 1 question: `$DEMO/judge/fake`
CM-1: in-progress → review
Every gate passed, and CM-1 is held for review:
  T13  criterion 1's judge already passed before the work, so it can't be proving it
It waits in `tracker review` for a human to approve or reject it. `tracker show CM-1 --diff` shows its work.
$ tracker create "Retried" --project DM
Created DM-1: Retried
$ tracker criterion add DM-1 --judge $DEMO/judge/lib.json
Added criterion 1 to DM-1, a judge: The code does the work
Its question is asked at submit, once every other gate passes.
$ jq '.projects.DM.judge = {max_failures: 2}' $TRACKER_HOME/config.json > $DEMO/config.new && mv $DEMO/config.new $TRACKER_HOME/config.json
$ reply no
$ printf 'fn run() {\n    todo!("dm stub")\n}\n' > dm/src/lib.rs
$ git add -A && git commit --quiet -m "test(DM-1): a stub for the judge to grade"
$ tracker gates DM-1      # exit 2: DM-1 is still open
Asking criterion 1's judge 1 question: `$DEMO/judge/fake`
error: DM-1 would be refused at submit: G3 failed. This was a dry run, so nothing was changed.
  G3  Criterion 1's judge answered no to c1:works, "Does this code do the work?", and only yes passes: The fake says no. Its excerpt follows.
        [src/lib.rs]
        fn run() {
            todo!("dm stub")
        }
  It's open, and only an in-progress item can be submitted.
[exit 2]
$ jq 'has("DM-1")' $TRACKER_HOME/judge/failures.json
false
$ tracker start DM-1
Criterion 1's judge: answered from the cache
DM-1: open → in-progress
$ tracker gates DM-1      # exit 2: the same request as at start
Criterion 1's judge: answered from the cache
error: DM-1 would be refused at submit: G3 failed. This was a dry run, so nothing was changed.
  G3  Criterion 1's judge answered no to c1:works, "Does this code do the work?", and only yes passes: The fake says no. Its excerpt follows.
        [src/lib.rs]
        fn run() {
            todo!("dm stub")
        }
[exit 2]
$ jq '.["DM-1"] | {at_start: (.at_start | length), failed: (.failed | length)}' $TRACKER_HOME/judge/failures.json
{
  "at_start": 1,
  "failed": 0
}
$ printf 'fn run() {\n    step(1);\n}\n' > dm/src/lib.rs
$ git add -A && git commit --quiet -m "feat(DM-1): version 1"
$ tracker gates DM-1      # exit 2: a new request fails
Asking criterion 1's judge 1 question: `$DEMO/judge/fake`
error: DM-1 would be refused at submit: G3 failed. This was a dry run, so nothing was changed.
  G3  Criterion 1's judge answered no to c1:works, "Does this code do the work?", and only yes passes: The fake says no. Its excerpt follows.
        [src/lib.rs]
        fn run() {
            step(1);
        }
[exit 2]
$ tracker gates DM-1      # exit 2: the same request, answered from the cache
Criterion 1's judge: answered from the cache
error: DM-1 would be refused at submit: G3 failed. This was a dry run, so nothing was changed.
  G3  Criterion 1's judge answered no to c1:works, "Does this code do the work?", and only yes passes: The fake says no. Its excerpt follows.
        [src/lib.rs]
        fn run() {
            step(1);
        }
[exit 2]
$ printf 'fn run() {\n    step(2);\n}\n' > dm/src/lib.rs
$ git add -A && git commit --quiet -m "feat(DM-1): version 2"
$ tracker gates DM-1      # exit 2: a second new request fails
Asking criterion 1's judge 1 question: `$DEMO/judge/fake`
error: DM-1 would be refused at submit: G3 failed. This was a dry run, so nothing was changed.
  G3  Criterion 1's judge answered no to c1:works, "Does this code do the work?", and only yes passes: The fake says no. Its excerpt follows.
        [src/lib.rs]
        fn run() {
            step(2);
        }
[exit 2]
$ jq '.["DM-1"] | {at_start: (.at_start | length), failed: [.failed[] | .refs]}' $TRACKER_HOME/judge/failures.json
{
  "at_start": 1,
  "failed": [
    [
      "c1:works"
    ],
    [
      "c1:works"
    ]
  ]
}
$ printf 'fn run() {\n    step(3);\n}\n' > dm/src/lib.rs; git add -A && git commit --quiet -m "feat(DM-1): version 3"
$ reply yes
$ tracker submit DM-1
Asking criterion 1's judge 1 question: `$DEMO/judge/fake`
DM-1: in-progress → review
Every gate passed, and DM-1 is held for review:
  T11  judges failed 2 different requests since start, and judge.max_failures is 2, so the work may have been changed until a judge passed: c1:works and c1:works
It waits in `tracker review` for a human to approve or reject it. `tracker show DM-1 --diff` shows its work.
$ tracker create "No login" --project EM
Created EM-1: No login
$ tracker criterion add EM-1 --judge $DEMO/judge/lib.json
Added criterion 1 to EM-1, a judge: The code does the work
Its question is asked at submit, once every other gate passes.
$ printf 'Not logged in\n' > $DEMO/judge/reply.json
$ tracker start EM-1
Asking criterion 1's judge 1 question: `$DEMO/judge/fake`
warning: Criterion 1's judge couldn't be graded at start: Criterion 1's judge couldn't be asked: `$DEMO/judge/fake` replied with something that isn't a judge's answers (expected value at line 1 column 1): Not logged in A failed call never counts as a pass. It counts as not passing, and is asked again at submit. EM-1 started anyway.
EM-1: open → in-progress
$ tracker show EM-1 | grep '^  checks  '
  checks  criterion 1 failed
$ tracker list --all --status in-progress,review,done
  ID    PRIORITY  KIND    STATUS       TITLE
BM  bm
  BM-1  medium    change  done         Small change
CM  cm
  CM-1  medium    change  review       Already passes
DM  dm
  DM-1  medium    change  review       Retried
EM  em
  EM-1  medium    change  in-progress  No login

Every project.
Filters: status in-progress, review or done.
```

Notes on the output:

- The commit SHA differs on every run.
- **BM-1** shows G1's refusal for a `diff` judge with no commit, `start` going ahead once `test(BM-1): …` is committed, and `started.checks` recording the judge as `failed`, which is what the line "criterion 1 failed" means: it didn't pass.
- **DM-1's `gates` before `start`** runs the judge on the stub and fails G3, but `has("DM-1")` is `false`: nothing was counted. `judge/failures.json` exists at that point because BM-1's and CM-1's starts made it ([What judge/failures.json holds](#what-judgefailuresjson-holds)).
- **The request asked at `start` is cached.** Both `start DM-1` and the next `gates` print "answered from the cache", because the stub didn't change between the open item's `gates` and `start`. `at_start` holds one request, and `failed` is empty after the repeat.
- **Version 1 is counted once** though `gates` ran twice, and version 2 once, so `failed` has two entries. The third version passes, and the submit is held by T11 with `2 >= max_failures`. T1 doesn't fire for DM-1 because the demo edits an existing file.
- **T11's message** lists each failed request's refs. Here both are `c1:works`, since the same question failed on two different requests, so the message reads "c1:works and c1:works".
- **EM-1** started although its judge command printed something that isn't a reply. The warning wraps the tool's G3 wording, and says the judge is asked again at submit.
- The `Asking criterion 1's judge …` line is printed only when the judge is really called, and `Criterion 1's judge: answered from the cache` when it isn't.

## Commands

No command or flag is new, and no `--json` shape changed. Six commands behave differently:

| Command | What changes |
| --- | --- |
| `start <ID>` | G1 also refuses an item whose judge has nothing to grade, with exit 2 and every reason. Then it runs each judge once, outside the lock, as it runs each check. The results go in `started.checks`, and a judge that passes, or can't be graded, adds a warning (`warnings` in `--json`). It starts the item's T11 count again |
| `gates <ID>` | For an item in progress, each request a judge failed is counted toward T11 in `judge/failures.json`. For an open item or one in review, nothing is counted. T11 runs with the other triggers once every gate passes |
| `submit <ID>` | Counts the failed requests of its own run, under the lock, then runs T11 with the other triggers. T13 now reads judges |
| `approve <ID>` | Runs T11 on the requests already counted. It counts nothing new, since the item is in review |
| `show <ID>` | Prints `started.checks` for judges too, as "criterion N passed" or "failed" |
| `submit --help` | Names T11 and says T13 covers judges |

The exit codes are as in [§3](DESIGN.md#3-interface): a refused `start` exits 2, as G1's other reasons do. A damaged `judge/failures.json` exits 3, as any store file that can't be parsed does ([§4.4](DESIGN.md#44-handling-bad-data)). The input rules for the criterion's judge file are [slice 12's](slice-12-readme.md).

## What judge/failures.json holds

[§4.1](DESIGN.md#41-layout) lists the file. It is `judge/failures.json` in the store, found with [src/store.rs](../src/store.rs) `judge_failures_path`. It's made by the first `start` of an item with a judge, or by the first request counted, and it holds one object per item ID:

| Field | What it holds |
| --- | --- |
| `at_start` | The fingerprint of each request the item's judges were asked at its last `start`. These never count, even when a later `gates` repeats one. Left out when empty |
| `failed` | One entry for each different request that a judge failed since then, with `request` (the fingerprint) and `refs` (the refs of the questions in it that didn't pass). Left out when empty |

`start` writes the entry again, so it empties `failed` and replaces `at_start`. It writes nothing for an item with no judge that has no entry. A request is counted at most once for an item: not if it is in `at_start`, and not if it's already in `failed`. T11 fires when `failed` has `max_failures` entries or more.

The file is JSON that `tracker` writes, with the temporary-file-and-rename every write uses. The cache key and the fingerprint are [slice 12's](slice-12-readme.md), so a changed excerpt, question or model is a new request, and the same request on another item is the same fingerprint. If the file can't be parsed, the error names it and says to fix it or remove it, which starts every item's count again. Nothing in `audit` reads it yet (slice 16).

## How it works

### G1 for judges

`apply` in [src/main.rs](../src/main.rs) handles `start` (and every other move) and loads the item's items and config first. It extends G1's report with `gates::g1_judges` before it tests whether G1 passed, so one refusal lists the item's missing criteria, a busy project and every judge with nothing to grade. `g1_judges` finds the project root (the whole list fails with one message if it can't) and asks `judge::nothing_at_start` about each judge criterion. That function reads the inputs the way G3 does, a file with `read_file` and `diff` with `read_diff` after `git::item_commits` finds at least one commit, strips comments, and finds the `each` excerpts with the same `excerpts` function. It never calls the judge. A criterion with no `each` has one excerpt per input, so only a missing file or an empty diff can fail it.

### Running the judges at start

After G1 and `gates::head_at_start`, `apply` drops the lock, runs `gates::criterion_runs` for the checks and `gates::grade_judges` for the judges, and takes the lock back with `lock_unchanged_after_checks`, which refuses if the item changed meanwhile. Judges take up to `judge.timeout_secs` for each question, so they run outside the lock as checks do ([§4.4](DESIGN.md#44-handling-bad-data)). `grade_judges` is G3's own loop (`gates::judges` now calls it), so a start and a submit ask the same questions, build the same requests and share one cache. G1 runs again under the lock, as it did for checks.

With the lock held, `judge::record_start` writes the item's entry in `judge/failures.json` with every request that was answered. `apply` then builds `started.checks`: one entry for each criterion that has a check or a judge, sorted by criterion number. A judge's entry is `passed` when `judge_passed` says so: it has no failed calls, at least one answer, and every answer's outcome is a pass. An unsure answer isn't one, and a judge that couldn't be graded gets `passed: false` and a warning from `judges_at_start`.

### Counting failed requests

`count_failures` in [src/main.rs](../src/main.rs) counts toward T11, for an item whose status is `in-progress` and no other. `gates_dry_run` calls it under a fresh lock after the judges have run, and `apply` calls it for `submit` under the lock it already holds, before it decides whether the gates passed, so a refused submit counts too. `judge::record_failures` takes the answers of the run. An answer's request counts when its outcome isn't a pass, so unsure counts, and a failed call counts for nothing, since it has no answer to carry a fingerprint. Answers of the same request are grouped, so one request that fails two questions is one failed request with two refs. A request already in `at_start` or `failed` isn't added. When nothing was added, the file isn't written.

### T11, and T13 for judges

`triggers::run` takes the item's `failed` requests as a new argument, read by `failed_requests` in [src/main.rs](../src/main.rs) (empty for an item with no judge, without reading the file), and `t11` fires when there are at least `max_failures` of them, with the project's `max_failures()` (default `DEFAULT_MAX_FAILURES`, 3, in [src/config.rs](../src/config.rs)). T11 runs after T10 and before T13, and every pinned list of triggers gained it, which is why six of the older tests changed. A project can switch T11 off in `review.disabled`, like any trigger except T6, T10 and T12 (`config::ALWAYS_ON`, unchanged) ([Decisions](#decisions-worth-knowing)).

T13 reads `started.checks` as it did, and `t13` says "judge" for an entry whose criterion is a judge and "check" for the rest. A judge given after `start` has no entry, which T13 reads as not passing, and the change that gave it fired T5.

## Code tour

| File | What's new or changed |
| --- | --- |
| [src/judge.rs](../src/judge.rs) | `nothing_at_start` (G1's reasons), `ItemFailures` and `FailedRequest` (the file's shape), `record_start`, `record_failures`, `failed_requests`, `read_failures` and `write_failures` |
| [src/gates.rs](../src/gates.rs) | `g1_judges` (G1's judge half), `grade_judges` (one loop for G3 and `start`, which `judges` now calls) and `judge_criteria` |
| [src/triggers.rs](../src/triggers.rs) | `t11`, T11 in `run` (a new `failed` argument), and `t13`'s judge wording |
| [src/config.rs](../src/config.rs) | `DEFAULT_MAX_FAILURES` and `Project::max_failures`. The `max_failures` field was already read and kept, and now it has a reader |
| [src/store.rs](../src/store.rs) | `judge_failures_path` |
| [src/main.rs](../src/main.rs) | `apply` (G1 for judges, grading and recording at `start`, counting at `submit`, `started.checks` for judges), `gates_dry_run`, `count_failures`, `failed_requests`, `judge_passed`, `judges_at_start`, `approve` (reads the counted requests), and the `start` and `submit` help text |
| [tests/judges_first.rs](../tests/judges_first.rs) | New, 579 lines, 20 tests, in `tr10_c1` to `tr10_c6` |
| [tests/common/mod.rs](../tests/common/mod.rs) | `Env::guard` and `guard_dir`, `guarded_path` and `GUARD_MESSAGE`, and `write_guards`. `Env::cmd` and `Env::std_cmd` put the guard's folder first on `PATH` |
| [tests/judges.rs](../tests/judges.rs) | The fake judge answers unsure while `at-start` exists, `start_and_work` and `start` start with it and change both messages, two tests start directly, and `tr45_c2` commits a stub first |
| [tests/gates.rs](../tests/gates.rs), [tests/review.rs](../tests/review.rs) and [tests/states.rs](../tests/states.rs) | T11 in the six pinned trigger lists (one, four and one) |
| [schemas/](../schemas/) | Descriptions only, in `gates.schema.json`, `item.schema.json` and `move.schema.json`. No field or shape changed |
| [docs/DESIGN.md](DESIGN.md) | §2.4's judges at `start` and its paragraph on `gates`, §2.5's T11 row, §4.1's `failures.json`, §4.3's `started.checks` and §7's interrupt bullet |
| [docs/PLAN.md](PLAN.md) | Slice 14's status, `todo` to `in review` (`fdaa0cb`). `4b76943` marked it `in progress` |

To count another kind of failure toward T11, give `record_failures` its answers, since it only counts what carries a request fingerprint. To make another trigger read the criteria's `started.checks` entries, note that an entry's number is the criterion's, as for checks, and that `criterion remove` renumbers them.

## Testing

Run the test command in `tracker/`. TR-10's criteria run their own tests by prefix, such as `cargo test --test judges_first tr10_c1_`. No test calls `tracker-judge` or the real `claude`.

**The harness change.** Every `Env` now has a `guard` folder, outside the work folder's repository, with a `tracker-judge` and a `claude` that read stdin, print `a test reached the real judge command: <name> isn't a fake. Set the store's judge.command to one.` on stderr and exit 1. `Env::cmd` and `Env::std_cmd` put that folder first on `PATH`, so a test that leaves the judge command at its default, `tracker-judge`, reaches only the guard, and one that sets a fake reaches the fake. A test that spawns something else by hand must use `guarded_path`.

**The fake judge in `tests/judges_first.rs`** is a shell script in `<work>/judge/` that saves each request as `request-<n>.json`, waits if `sleep` exists and prints `reply.json`. `reply` and `reply_text` set the answer, and `asked` counts the requests. The fixtures are project XX with `src/lib.rs` holding `STUB`, and `work(env, n)`, which writes version `n` of the work and commits it, so each version is a different request.

### Coverage of the _Done when_ checks

| _Done when_ | Tests |
| --- | --- |
| `start` refuses a `diff` judge before the item has a commit. | `tr10_c1_`, 4 tests: `tr10_c1_start_refuses_a_diff_judge_before_the_item_has_a_commit` (exit 2, one G1 failure naming criterion 1 and "has no commits", the item still open, the judge never asked), `tr10_c1_a_diff_judge_starts_once_the_item_has_a_commit`, `tr10_c1_start_refuses_a_judge_whose_each_matches_nothing_now` and `tr10_c1_start_refuses_a_judge_whose_input_is_gone` |
| A judge whose answers all pass at start is recorded as already passing and held by T13. | `tr10_c2_`, 5 tests: `tr10_c2_a_judge_that_passes_at_start_is_recorded_and_held_by_t13`, `tr10_c2_a_judge_that_fails_at_start_is_recorded_as_not_passing`, `tr10_c2_an_unsure_answer_at_start_doesnt_pass`, `tr10_c2_a_failed_call_at_start_lets_the_item_start_as_not_passing` and `tr10_c2_start_asks_its_judges_without_holding_the_lock` (a `list` run while `start` waits on a 4-second judge returns in under 3) |
| T11 holds an item after `max_failures` different failed requests. | `tr10_c3_`, 6 tests: `tr10_c3_t11_holds_an_item_after_max_failures_different_failed_requests`, `tr10_c3_fewer_failed_requests_than_max_failures_dont_hold`, `tr10_c3_max_failures_comes_from_the_projects_judge_settings`, `tr10_c3_an_unsure_answer_counts_as_a_failed_request`, `tr10_c3_a_failed_call_doesnt_count` and `tr10_c3_submit_help_names_t11` |
| An unchanged failure that `gates` repeats counts once, and fails at start don't count. | `tr10_c4_`, 2 tests: `tr10_c4_an_unchanged_failure_that_gates_repeats_counts_once` (three `gates` runs, one counted) and `tr10_c4_fails_at_and_before_start_dont_count` (a `gates` before `start`, `start`, and a `gates` after it on unchanged work: none counted, so a `max_failures` of 1 doesn't hold) |
| The test harness puts a `tracker-judge` and a `claude` that fail first on `PATH` (criterion 5). | `tr10_c5_`, 2 tests: `tr10_c5_the_harness_puts_failing_judge_commands_first_on_path` (a check records `command -v` of both, and each is in the guard folder and exits 1 with the message) and `tr10_c5_a_judge_left_at_the_default_command_reaches_only_the_guard` |
| §7's list of commands that run checks or judges names `approve`, and says which commands run judges (criterion 6). | `tr10_c6_section_7_names_the_commands_that_run_checks_and_judges`, which finds §7's interrupt bullet and checks both sentences |

Requirements from the _Prompt_ that no _Done when_ bullet names:

| Requirement | Tests |
| --- | --- |
| G1 refuses a judge that has nothing to grade, not only `diff`. | The `each` and missing-input tests in `tr10_c1_` |
| A failed call at `start` lets it go ahead. | `tr10_c2_a_failed_call_at_start_lets_the_item_start_as_not_passing` (`started.checks` has `[(1, false)]` and the warning says "couldn't be graded") |
| An unsure answer is a failed request. | `tr10_c3_an_unsure_answer_counts_as_a_failed_request` |

The tests that changed before `start`, in `3fa1f52`: `tests/judges.rs`'s fake judge answers unsure to every question while a file named `at-start` exists, and saves those requests apart, so `requests` counts only the ones after the work. `start_and_work` and the new `start` use it, and `start_and_work` now changes both messages (`WORK`), so the submit's excerpts differ from the cached ones. Two tests call `start` directly, and `tr45_c2` commits a stub function first, since its judge now needs one at `start`. This is the edit that fired T4.

**Seen failing first.** All six of TR-10's checks failed at `start`, at `3fa1f52` (the criteria's `started.checks` record `failed` for each). The slip is the six trigger lists. They are `{ "trigger": "T11", "fired": false }` added after T10 in `tests/gates.rs` (one), `tests/review.rs` (four) and `tests/states.rs` (one). They needed T11 as soon as the code ran it, which no test said before `519e61d`, and they were added in `d202442`, which is after `start`, so no failing run of them is recorded. They are additions only, so T4 stayed quiet.

Not covered by a test: a `gates` on an item in review, which counts nothing, a damaged `judge/failures.json`, `criterion remove` renumbering a judge's `started.checks` entry, `approve` reading T11, and the warning for a judge that passes at start through `--json`.

### Checks done by hand

| What | How | Result |
| --- | --- | --- |
| The demo above | Run line by line with the fake judge, stderr captured, on a temporary `TRACKER_HOME`, with the temporary folder replaced by `$DEMO`. The real `claude` and `tracker-judge` weren't called | The output above. Every refusal exits as marked |
| TR-10's checks, before the approve | The orchestrator checked by hand that the four written criteria are unchanged, that all six checks failed at `start`, and that every commit since `start` is scoped to TR-10 | All three held |
| TR-10's approve in a real terminal | The user read the diff and typed `TR-10` into `tracker approve TR-10` | `by: human` at `fdaa0cb`, with `refs/tracker/TR-10`, and four holds |

### Tracking TR-10

TR-10, a `capability` with `design` `docs/DESIGN.md#2.4`, was created at 2026-11-21T07:07:43Z with slice 7's other items. The slice 13 binary was installed before it started and tracked it throughout, so T5, T13 and T14 ran on it:

| Step | When (UTC) | What |
| --- | --- | --- |
| Tests, committed | 14:45:59Z | `3fa1f52`, 46 seconds before `start` |
| `tracker start TR-10` | 14:46:45Z | `started.head` `3fa1f52`. All six checks failed |
| The code | 14:46:50Z to 14:53:03Z | `4b76943`, `d202442`, `519e61d`, `d2ad26c` and `fdaa0cb` |
| `tracker submit TR-10` | 14:59:48Z | The gates passed, then the triggers ran. Held under T1, T4, T7 and T8 |
| `tracker approve TR-10` | 15:37:59Z | From a terminal, verified `human` at `fdaa0cb`, with `refs/tracker/TR-10` |

- **The four holds** read: T1, "it's a capability" and the new source files. T4, "existing test file modified: tests/judges.rs (3fa1f52 before start)". T7, "1161 lines changed, more than the project's max_diff_lines of 300", most of them the new test file. T8, "the project's design doc changed: docs/DESIGN.md". They're recorded in `verification.triggers`, and `tracker show TR-10` is the record, with each check's command, result and saved output.
- **Why T4 fired.** `tests/judges.rs` existed before TR-10. `3fa1f52` changed its fake judge, `start_and_work`, two direct starts and `tr45_c2` so they still test what they did once `start` runs judges and G1 needs something to grade. That's the change the _Prompt_ foresaw, and it's in the test commit, so it was seen failing first. The user read the diff.
- **T13 didn't fire.** Each criterion's check failed at `start`.
- **TR-47, TR-48 and TR-50** were open before this slice, and are still open and deferred.

## Decisions worth knowing

These are the "Slice 14 must decide" points from slice 13's reviews, and what the slice chose:

- **A failed call doesn't count toward T11.** Only answered requests that weren't a pass do: a fail or an unsure. A failed call (no login, a usage limit, a timeout, a refused model) has no answer to carry a fingerprint, and counting it would let an outage hold an item. [`judge::Graded`](../src/judge.rs) records a failure as a message with no fingerprint, so the choice also fits the code as it is.
- **A fail counts once for each request fingerprint, and only while the item is in progress.** A request first asked before `start`, at `start` or in review never counts, nor does a repeat of one asked at `start`. `gates` before `start` records nothing: an agent is meant to see its judges fail on the stub then. The count is the one `judge/failures.json` keeps, and `start` starts it again.
- **A failed call at `start` lets it go ahead, with a warning.** The judge is recorded as not passing, as T13 reads a missing check entry, and G3 asks it again at submit. Refusing `start` would block an agent on a login problem it can't fix, and the submit is where a failed call matters.
- **Judges at `start` run outside the lock, after the checks,** as the checks do, since each question can take `judge.timeout_secs`. `lock_unchanged_after_checks` refuses the start, writing nothing, if the item changed meanwhile.
- **A judge given after `start` isn't run then,** unlike a check. `criterion add --judge` and `set --judge` don't call the judge, so the criterion has no `started.checks` entry, which T13 reads as not passing. The change fired T5 anyway, and the item is held for a human.
- **A judge's start result goes in `started.checks`,** beside the checks', since a criterion has a check or a judge and never both. `passed` means every answer was the passing one, and the item's schema description says so. No new field was added, so item files written by earlier slices are unchanged.
- **An unsure answer is a failed request and isn't a pass,** for T11, T13 and `started.checks`. A project's `auto_verify: false` still holds under T10 as before.
- **T11 stays switchable.** `config::ALWAYS_ON` is still T6, T10 and T12, so a project can put T11 in `review.disabled`, as it can T13. PLAN.md's prompt for this slice doesn't ask for a change, so none was made.
- **The harness guard is on `PATH`, not in the store.** A test sets the store's `judge.command` to a fake, or reaches only the guard. The guard folder is outside the work folder's repository, so it never makes a file in a project's tree, and no test needs the installed `tracker-judge` or the real `claude`.
- **`judge/failures.json` is outside the item,** so a `gates` dry run can count without writing the item file. [§2.4](DESIGN.md#24-gates-is-it-finished) says so: a dry run can't be used to try out rewordings for free.
- **T11's message lists refs, not requests.** Two different requests that fail the same question give "c1:works and c1:works". It tells a reader which questions failed, not how the requests differ, which `failures.json` keeps by fingerprint.

## Still open

From the build and the checks. Slice 14 has no review yet, so nothing here comes from one. The slice 13 findings it took are done ([What's in this slice](#whats-in-this-slice)).

**Deferred items:**

- **TR-47.** A `diff` judge's hunks are stripped one at a time, so a block comment that opens above a hunk reaches the judge. [Slice 13's readme](slice-13-readme.md#limits-and-what-later-slices-change) said to do it before slice 14. It wasn't, and it's still open. With G1 now requiring a `diff` judge to have a commit and `start` grading it, a diff judge's start request has the same flaw its submit request does.
- **TR-48.** What else a real judge sees (the environment details and the user's email note), and review 2's design 1 about §2.3.1's Pinned model bullet.
- **TR-50.** Escaping `</material>` in both adapters, pinning step 1's prompt, and the empty `is_error` reply. To do together, the next time the judge prompt changes.

**Checks not run:**

- **A real judge at `start`.** No run used the real `claude`. The tests and the demo use a fake judge command, so the cost and the time of a real call for each question and match at `start` aren't measured. A `start` now makes the same calls a `gates` run does.
- **A terminal `approve` of a held T11 item.** `tests/judges_first.rs` reads the hold through `submit --json`, not `approve`. No test types an ID.

**Test gaps** (see [Testing](#testing)): `gates` on an item in review, a damaged `judge/failures.json`, `criterion remove` renumbering a judge's entry, and `approve` reading T11.

**Carried from slice 13** ([its Still open](slice-13-readme.md#still-open)): the empty temporary folder a timed-out `tracker-judge` leaves, the tests needing Node, `judge-eval` not run with the installed binary and `claude-sonnet-5-5` not rerun, and the points slice 13 carried from slice 12 (TR-44, the cache key, the `approve` ordering and the cosmetic help text for `--judge`).

## Limits, and what later slices change

- **Slice 15** disputes a `no`. A dispute holds the item under T12, and T11 still counts a disputed fail, since it was a failed request. If a dispute should stop that, it has to say so in the T11 row of [§2.5](DESIGN.md#25-review-triggers-does-it-need-a-human) and in `record_failures`. `approve` counts questions held under T10 or T12 as passed for that approval only, and it reads `judge/failures.json` through `failed_requests` without counting, so a T11 hold stays a hold for it. Slice 15 must also decide whether a missing `reason` fails the call, a point carried from slice 13.
- **Slice 16**'s `audit` and `judges` read the store. `judge/failures.json` is a new file that hand-editing or removing resets for every item (its error says so), and `audit` doesn't read it, so a count moved by hand isn't reported. A report of the failed requests for each question would read its `refs`, which the file keeps for that, and `started.checks` is a list of passed flags whose criterion is either a check or a judge, so a reader must look at the criterion. The digest's judge entries read `verification.judges`, as before.
- **Any change to the request** (the model, a prompt, a question) is a new fingerprint, so it is a new failed request for T11, and a changed stub at `start` is a new `at_start` entry. The cache is keyed the same way, so unchanged work isn't asked again at `start`, at `gates` or at submit.
- **TR-47 and a `diff` judge.** Until it's done, a `diff` judge sees comment text that §2.3.1 says it doesn't. A project shouldn't add one before then. Slice 14 doesn't change what the judge sees, only when it's asked.
- **Interrupting `tracker`.** `start` now runs judges, so a Ctrl-C during `start` can leave the judge command and its `claude` running, as §7's interrupt bullet says for every command that runs judges. The bullet now lists them: `start`, `gates`, `submit` and `approve`.
- **A new trigger** goes into every pinned trigger list in [tests/gates.rs](../tests/gates.rs), [tests/review.rs](../tests/review.rs) and [tests/states.rs](../tests/states.rs), and into `triggers::run`'s order, as T11 did. Add the tests to the item's first test commit, so they're seen failing, which this slice didn't do.

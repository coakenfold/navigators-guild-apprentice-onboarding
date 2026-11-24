# Slice 12: Judge criteria

`tracker` is a command-line work tracker, written in Rust, that keeps one central list of work items across many projects and decides when an item is really done. This document covers **slice 12** of the 16 in [PLAN.md](PLAN.md): what it built, how it works, and how to test it. The earlier readmes cover the rest: [slice 1](slice-1-readme.md) the store, `create` and `list`, [slice 2](slice-2-readme.md) the states and `history`, [slice 3](slice-3-readme.md) priority, kind, labels, `edit` and `list`'s filters, [slice 4](slice-4-readme.md) `show` and `delete`, [slice 5](slice-5-readme.md) bad data and polish, [slice 6](slice-6-readme.md) criteria and gates G1, G2 and G6, [slice 7](slice-7-readme.md) running checks, with gates G3 and G4 and `started.checks`, [slice 8](slice-8-readme.md) G5, the `auto` record and `refs/tracker/<ID>`, [slice 9](slice-9-readme.md) the inbox, `approve`, `reject` and triggers T6 and T9, [slice 10](slice-10-readme.md) triggers T1, T2, T3, T4, T7 and T8, and [slice 11](slice-11-readme.md) triggers T5, T13 and T14 and the criteria fingerprint.

After slice 12 you can **have yes/no judges grade qualities at submit, using a fake judge command**. The spec is [DESIGN.md](DESIGN.md). This slice implements judge criteria from [§2.3.1](DESIGN.md#231-judges-claude-via-the-claude-cli), G3's judge half from [§2.4](DESIGN.md#24-gates-is-it-finished), trigger T10 from [§2.5](DESIGN.md#25-review-triggers-does-it-need-a-human), the `--judge` flags and the `judges` field of `gates` and `submit` from [§3](DESIGN.md#3-interface), the `judge` settings from [§4.2](DESIGN.md#42-config), and the criterion's judge fields and `verification.judges` from [§4.3](DESIGN.md#43-item-file). The judge command is a contract, and in this slice only a fake script keeps it: the real command, `tracker-judge`, is slice 13's. The slice is tracked in `tracker` itself as item TR-8, with the binary installed from slice 11 ([Tracking TR-8](#tracking-tr-8)). [Review 1](reviews/slice-12-review-1.md) found four bugs, one security item, four design questions and three test gaps. Its fixes are item TR-45 ([Fixes after review 1](#fixes-after-review-1)). [Review 2](reviews/slice-12-review-2.md) found two bugs, two design questions, one test gap and one process note. Its fixes are item TR-46 and one docs commit ([Fixes after review 2](#fixes-after-review-2)).

| | |
| --- | --- |
| Status | Every _Done when_ check has a test, and every test passes, as does the test command. TR-8 is `done`, but it wasn't auto-verified. The installed slice 11 binary held its submit under T1, T3, T4, T7 and T8, and the user read the diff and approved it from a terminal: `verification.by` is `human`, at `932c1c4`, with `refs/tracker/TR-8`, and the five holds are recorded in `verification.triggers` as approved. TR-8 has ten criteria: PLAN.md's nine _Done when_ bullets, as written, and a tenth, "`auto_verify: false` holds under T10", which the builder added before `start`. All ten checks failed at `start`. Before the approve, the orchestrator checked by hand that the nine written criteria are unchanged, that all ten checks failed at `start`, and that every commit since `start` is scoped to TR-8. Not covered by a test: a terminal `approve` with a judge criterion, the `show` text for a judge, and `evidence` refused for a judge ([Testing](#testing)). A judge whose input is `diff` and a piped `approve` got tests in review 1's fixes. The real judge, `tracker-judge`, doesn't exist yet, so the default `judge.command` fails every call until slice 13 ([Limits](#limits-and-what-later-slices-change)). **Review 1's fixes are done, except what it left on purpose** ([Fixes after review 1](#fixes-after-review-1)). TR-45 took bug 1 (a), bug 2, bug 3, design 1, design 2 (a), design 4, security 1 (b) and test gaps 1 to 3 as one item, so the user approves once. It is `done`, but it wasn't auto-verified: the installed slice 11 binary held its submit under T8 (DESIGN.md) and T13 (criterion 5, whose check passed at `start`), and the user read the diff and approved it from a terminal: `verification.by` is `human`, at `d356d17`, with `refs/tracker/TR-45`. Criteria 1 to 4 and 6 failed at `start`. Won't fix, as unlikely: bug 4 (a huge `context_lines`, `judge.timeout_secs` or `check_timeout_secs` panics) and design 3 (`item::Question` refuses a newer tracker's question field, so the item is unreadable and not a newer file). Security 1's option (a), the judge program's path in the cache key and the reply's `adapter` in the record, is left for slice 13, and so are review 1's "Slice 13 must decide" points. Still not covered by a test: a terminal `approve` that goes through to done (review 2's TR-46 tests `approve`'s question only), `show`'s judge text, `evidence` for a judge, a reply that can't be cached and a file input deleted after `criterion add` ([Still open](#still-open)). [Review 2](reviews/slice-12-review-2.md)'s fixes are done, except what is left on purpose ([Fixes after review 2](#fixes-after-review-2)). Process 1 (`cargo test` failed at `945390f`) was fixed in `e4b9dde`. TR-46 took bug 1, design 1 and design 2, and is `done`: the installed slice 11 binary verified it `auto` at `a59adbc`, with `refs/tracker/TR-46` and no holds, and all 3 of its checks failed at `start`. Not fixed: bug 2 (a block comment that opens above a `diff` hunk reaches a `diff` judge) is TR-47 (`fix`, low priority), to do before any project gets a `diff` judge, and test gap 1 won't be fixed, so the wording is corrected instead. Review 2's "Slice 13 must decide" points are left for slice 13 (TR-9). PLAN.md has slice 12 as `in review`. The branch isn't merged into `main`, and neither are slices 7 to 11, which it's stacked on. |
| Branch | `feat/tracker/slice-12`, stacked on `feat/tracker/slice-11` at `97345fa`. Slice 11 isn't merged into `main`, so this slice's commits are `git log --oneline --reverse feat/tracker/slice-11..feat/tracker/slice-12`. In order: `b8f378c` (TR-8's tests and the older tests it changed, committed before `tracker start TR-8`), `a03a8b8` (PLAN.md: slice 12 in progress), `1c95fda` (the code), `a301e27` (fixture XX-24 and its test), `919e022` (DESIGN.md) and `932c1c4` (PLAN.md: slice 12 in review), all scoped to TR-8. That is 6 commits. Then `b79fc39` (this readme) and `6d1c664` (review 1), which belong to no item, and review 1's fixes, all scoped to TR-45: `deb662a` (tests), `5d63075` (the fixes) and `d356d17` (DESIGN.md). That is 11 commits since slice 11. Then `bfe5220` (review 1's Outcome), `945390f` (this readme's first update) and `e4b9dde` (review 2, with a pointer to it at the end of review 1's Outcome), which belong to no item, and review 2's fixes, scoped to TR-46: `84b8c04` (tests) and `a59adbc` (the fix). That is 16 commits since slice 11. This readme's second update and review 2's Outcome come after them |
| Test command | `cargo fmt --check && cargo clippy -- -D warnings && cargo test`, run in `tracker/`. All three pass at `a59adbc`. They didn't at `945390f`: `cargo test` failed in `docs_links::heading_anchors_name_headings_that_exist`, because review 1's Outcome linked to a readme heading that the next commit added. `e4b9dde` fixed it ([Fixes after review 2](#process)) |
| Tests | 462: 46 in `tests/risky_changes.rs`, 42 in `tests/judges.rs`, 39 in `tests/create_and_list.rs`, 36 in `tests/commits.rs`, 34 in `tests/organise.rs`, 28 in `tests/review.rs`, 27 in `tests/states.rs`, 25 in `tests/show_and_delete.rs`, 23 in `tests/criteria.rs`, 22 in `tests/gates.rs`, 21 in `tests/tampered_proof.rs`, 18 in `tests/checks.rs`, 15 in `tests/fixtures.rs`, 14 in `tests/item_files.rs`, 13 in `tests/bad_data.rs`, 11 in `tests/docs_links.rs`, 9 in `tests/polish.rs`, 6 in `tests/output.rs`, 6 in `tests/t5_and_fingerprint.rs`, 5 in `tests/schemas.rs`, 4 in `tests/store_writes.rs`, 2 in `tests/item_ids.rs`, 1 each in `tests/harness_check.rs` and `tests/submit_help.rs`, 12 unit tests in the library (4 in `src/gates.rs`, 4 in `src/judge.rs`, 2 in `src/git.rs`, 1 each in `src/store.rs` and `src/checks.rs`) and 2 in `src/main.rs`. That is 47 more than slice 11's 415: 42 in `tests/judges.rs`, 4 in `src/judge.rs` and 1 in `tests/fixtures.rs`. Review 1's TR-45 added 7 of them: 6 in `tests/judges.rs` and 1 in `src/judge.rs` (451 before). Review 2's TR-46 added 4, all in `tests/judges.rs` (458 before) |

## Contents

- [What's in this slice](#whats-in-this-slice)
- [Try it](#try-it)
- [Commands](#commands)
- [The judge file](#the-judge-file)
- [The judge command's contract](#the-judge-commands-contract)
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

- **Judge criteria.** A criterion with `ask`, a list of yes/no questions, is a judge criterion (`Criterion::is_judge`). It has `inputs` (files under the project's root, or `diff`), and optionally `each`, `context_lines` and `keep_comments`. A judge criterion has no `check` and no `evidence`, so it isn't manual and T6 doesn't hold it.
- **`criterion add --judge <file>` and `criterion set <n> --judge <file>`**, which read a [judge file](#the-judge-file), validate it and copy it into the item. They refuse invalid JSON, a field that isn't a judge's, an invalid regex, a missing input and an `each` that matches nothing. `set` refuses a file whose `text` differs from the criterion's.
- **Request building.** One excerpt for the whole input, or with `each` one excerpt per line that matches, with `context_lines` lines either side. Comments are stripped from code unless `keep_comments`, in the style of the file's extension (`judge::strip_comments`). An excerpt over `judge.max_input_bytes` is refused before the judge is called.
- **The judge command.** `judge::grade` runs the config's `judge.command` with `sh -c`, sends the request on stdin, stops it after `judge.timeout_secs` for each question, and reads the answers from stdout ([the contract](#the-judge-commands-contract)).
- **A cache.** Each reply is kept in `judge/cache/` under a fingerprint of the judge command and the request, so unchanged work gets the same answer, and the judge isn't called again.
- **G3's judge half**, `gates::judges`, run only once every other gate has passed. A `no` (or the answer `pass_when` doesn't name) fails G3, and so do a missing input, an `each` that matches nothing, an oversized excerpt and a failed call. An `unsure` answer doesn't fail it.
- **T10**, `triggers::t10`. It fires for an `unsure` answer, or when the project's `judge.auto_verify` is `false` and a judge graded the item. It runs on every submit and `approve`, and `review.disabled` can't switch it off (`config::ALWAYS_ON` already named it).
- **`--json`.** `gates` and `submit` list every judge question as `judges`, left out for an item with no judge. A G3 failure for a question carries `ref`, `answer` and `reason`. A done item's `verification.judges` has one entry for each question and excerpt.
- **Config.** The global `judge` object (`command`, `model`, `max_input_bytes`, `context_lines`) and a project's `judge` object (`max_failures`, `auto_verify`, `timeout_secs`) are read, with the defaults [below](#how-it-works).
- **`show`** prints a judge criterion's inputs, `each`, context, comments and questions, and a done item's judge answers.
- **`regex`** 1.13.1, added to [Cargo.toml](../Cargo.toml), for `each`.
- **Slice 11's "Slice 12 must decide" points**, from [slice 11's review 2](reviews/slice-11-review-2.md#slice-12-must-decide) and [review 1](reviews/slice-11-review-1.md#slice-12-must-decide): see [Decisions](#decisions-worth-knowing).

**Not built yet:**

- **`tracker-judge`, the real judge command** (slice 13). The default `judge.command` is `tracker-judge`, which doesn't exist until then, so a judge criterion on a store whose config has no `judge.command` fails G3 with "couldn't be run". Judge-eval isn't pointed at it either (slice 13).
- **Judges at `start`, G1's refusal for a judge with nothing to grade, T11, and T13 for judges** (slice 14). A judge isn't run at `start` and has no entry in `started.checks`, so T13 is quiet for it. `criterion add --judge` and `criterion set --judge` do refuse a judge with nothing to grade today, but a judge added another way, such as by editing the file, isn't refused at `start`.
- **Disputes and T12** (slice 15): `submit --dispute`, and `approve` counting held judge questions as passed for that approval. Today an approve just re-asks the questions (from the cache) and moves the item to done if every gate passes.
- **`tracker judges`, and the digest's judge entries** (slice 16).
- **A `max_failures` that does anything.** It's read and kept, and slice 14 uses it.

## Try it

Always set `TRACKER_HOME` while developing. Without it, the tool uses your real store at `~/.tracker`.

The two projects, BM and IM, are the folders `bm` and `im` inside one throwaway repository, as TR is a folder inside guild-portfolio. The judge command is a small shell script that saves the request it gets and prints whatever is in `reply.json`, so the demo sets each answer. BM-1 has a script criterion and a judge criterion. It's refused when the judge says no, then held under T10 when it says unsure. IM-1 has only a judge, which says yes, so it goes to done. The refusals are `--judge` with a file that isn't JSON, a file whose `each` matches nothing, a file whose text isn't the criterion's, `evidence` for a judge, and a submit the judge fails.

```sh
cd guild-portfolio/tracker
cargo build

export DEMO=$(mktemp -d)
export TRACKER_HOME=$DEMO/store
export PATH="$PWD/target/debug:$PATH"
export GIT_AUTHOR_NAME=demo GIT_AUTHOR_EMAIL=demo@example.invalid
export GIT_COMMITTER_NAME=demo GIT_COMMITTER_EMAIL=demo@example.invalid
mkdir -p $DEMO/repo/bm/src $DEMO/repo/im/src $DEMO/judge && cd $DEMO/repo
git init --quiet -b main
printf 'fn load() {\n    // BUG? this looks wrong\n    bail!("the config is missing");\n}\n' > bm/src/errors.rs
printf 'fn load() {\n    bail!("the link file is missing: run tracker import --help");\n}\n' > im/src/errors.rs
git add -A && git commit --quiet -m "chore: set up"
printf '#!/bin/sh\nd=$(dirname "$0")\ncat > "$d/last-request.json"\necho call >> "$d/calls"\ncat "$d/reply.json"\n' > $DEMO/judge/fake && chmod +x $DEMO/judge/fake
printf '%s' '{"adapter":"fake 1","model":"claude-haiku-5-5","answers":{"next_step":{"answer":"no","reason":"It only says the config is missing."}}}' > $DEMO/judge/reply.json
printf '%s\n' '{"text":"Every error says what to do next","inputs":["src/errors.rs"],"each":"bail!","context_lines":1,"ask":[{"id":"next_step","ask":"Does this error message tell the user what to do next?","yes":"the message gives a concrete next step","no":"the message only reports the problem"}]}' > $DEMO/judge/errors.json
sed 's/"bail!"/"panic!"/' $DEMO/judge/errors.json > $DEMO/judge/nothing.json
echo 'not json' > $DEMO/judge/bad.json
cd bm
tracker init
tracker project add BM --name bookmarks --root .
tracker project add IM --name importer --root ../im
jq --arg c "$DEMO/judge/fake" '.judge = {command: $c, model: "claude-haiku-5-5"}' $TRACKER_HOME/config.json > $DEMO/config.new && mv $DEMO/config.new $TRACKER_HOME/config.json
tracker create "Errors help" --project BM --done-when "A missing file is reported"
tracker criterion set BM-1 1 --check 'test -f found'
tracker criterion add BM-1 --judge $DEMO/judge/bad.json      # exit 1: not JSON
tracker criterion add BM-1 --judge $DEMO/judge/nothing.json      # exit 1: each matches nothing
tracker criterion set BM-1 1 --judge $DEMO/judge/errors.json      # exit 1: the text differs
tracker criterion add BM-1 --judge $DEMO/judge/errors.json
tracker evidence BM-1 2 "I read it"      # exit 1: a judge takes no evidence
tracker show BM-1 | sed -n '/^Done when/,/^$/p'
echo "errors" > spec.txt && git add -A && git commit --quiet -m "test(BM-1): describe the errors"
tracker start BM-1
touch found && git add -A && git commit --quiet -m "feat(BM-1): report a missing file"
tracker gates BM-1      # exit 2: the judge said no
tracker gates BM-1      # exit 2: the judge said no
cat $DEMO/judge/calls | wc -l
tracker gates BM-1 --json | jq -c '.failures[] | select(.ref) | {ref, answer, reason}'
tracker submit BM-1      # exit 2: the judge said no
printf '%s' '{"adapter":"fake 1","model":"claude-haiku-5-5","answers":{"next_step":{"answer":"unsure","reason":"The next step may be printed by the caller."}}}' > $DEMO/judge/reply.json
printf 'fn load() {\n    bail!("the config is missing");\n}\n' > src/errors.rs && git add -A && git commit --quiet -m "feat(BM-1): drop the comment"
tracker gates BM-1 --json | jq -c '{judges, holds}'
tracker submit BM-1
tracker show BM-1 | sed -n '/^Held/,$p'
cd ../im
printf '%s' '{"adapter":"fake 1","model":"claude-haiku-5-5","answers":{"next_step":{"answer":"yes","reason":"It names the command to run."}}}' > $DEMO/judge/reply.json
tracker create "Imports help" --project IM
tracker criterion add IM-1 --judge $DEMO/judge/errors.json
tracker start IM-1
touch found && git add -A && git commit --quiet -m "feat(IM-1): import a file"
tracker submit IM-1
jq . $DEMO/judge/last-request.json
tracker show IM-1 | sed -n '/^Verification/,$p'
jq '.criteria[0], .verification.judges' $TRACKER_HOME/items/IM-1.json
tracker list --all --status review,done
```

The output, from `tracker init` on, with stdout and stderr together:

```
$ tracker init
Created the store at $DEMO/store.
$ tracker project add BM --name bookmarks --root .
Registered BM (bookmarks) at $DEMO/repo/bm.
$ tracker project add IM --name importer --root ../im
Registered IM (importer) at $DEMO/repo/im.
$ jq --arg c "$DEMO/judge/fake" '.judge = {command: $c, model: "claude-haiku-5-5"}' $TRACKER_HOME/config.json > $DEMO/config.new && mv $DEMO/config.new $TRACKER_HOME/config.json
$ tracker create "Errors help" --project BM --done-when "A missing file is reported"
Created BM-1: Errors help
1 manual criterion: give it a check with `tracker criterion set BM-1 1 --check "…"`.
$ tracker criterion set BM-1 1 --check 'test -f found'
Criterion 1 of BM-1 now has a check.
$ tracker criterion add BM-1 --judge $DEMO/judge/bad.json
error: The judge file $DEMO/judge/bad.json isn't valid JSON: expected ident at line 1 column 2.
[exit 1]
$ tracker criterion add BM-1 --judge $DEMO/judge/nothing.json
error: The judge file $DEMO/judge/nothing.json has nothing to grade: its each, `panic!`, matches nothing in src/errors.rs, once comments are stripped.
[exit 1]
$ tracker criterion set BM-1 1 --judge $DEMO/judge/errors.json
error: The judge file $DEMO/judge/errors.json has the text "Every error says what to do next", but criterion 1 of BM-1 is "A missing file is reported". A criterion keeps its text, so make the file's text match it, or add the judge as a new criterion with `tracker criterion add BM-1 --judge …`.
[exit 1]
$ tracker criterion add BM-1 --judge $DEMO/judge/errors.json
Added criterion 2 to BM-1, a judge: Every error says what to do next
Its question is asked at submit, once every other gate passes.
$ tracker evidence BM-1 2 "I read it"
error: Criterion 2 of BM-1 has a judge, and a judge's answers are its proof, so it takes no evidence. Evidence is only for a manual criterion, one nothing can run.
[exit 1]
$ tracker show BM-1 | sed -n '/^Done when/,/^$/p'
Done when:
  1. A missing file is reported
       check: test -f found
  2. Every error says what to do next
       judge, reads src/errors.rs, once per match of `bail!` with 1 line of context, asks 1 question:
         next_step: Does this error message tell the user what to do next?

$ echo "errors" > spec.txt && git add -A && git commit --quiet -m "test(BM-1): describe the errors"
$ tracker start BM-1
Running criterion 1's check: `test -f found`
BM-1: open → in-progress
$ touch found && git add -A && git commit --quiet -m "feat(BM-1): report a missing file"
$ tracker gates BM-1
Running criterion 1's check: `test -f found`
Asking criterion 2's judge 1 question at src/errors.rs:3: `$DEMO/judge/fake`
error: BM-1 would be refused at submit: G3 failed. This was a dry run, so nothing was changed.
  G3  Criterion 2's judge answered no to c2:src/errors.rs:3:next_step, "Does this error message tell the user what to do next?", and only yes passes: It only says the config is missing. Its excerpt follows.
        [match]
        src/errors.rs:3: bail!("the config is missing");
  
        [code]
  
            bail!("the config is missing");
        }
[exit 2]
$ tracker gates BM-1
Running criterion 1's check: `test -f found`
Criterion 2's judge at src/errors.rs:3: answered from the cache
error: BM-1 would be refused at submit: G3 failed. This was a dry run, so nothing was changed.
  G3  Criterion 2's judge answered no to c2:src/errors.rs:3:next_step, "Does this error message tell the user what to do next?", and only yes passes: It only says the config is missing. Its excerpt follows.
        [match]
        src/errors.rs:3: bail!("the config is missing");
  
        [code]
  
            bail!("the config is missing");
        }
[exit 2]
$ cat $DEMO/judge/calls | wc -l
       1
$ tracker gates BM-1 --json | jq -c '.failures[] | select(.ref) | {ref, answer, reason}'
Running criterion 1's check: `test -f found`
Criterion 2's judge at src/errors.rs:3: answered from the cache
{"ref":"c2:src/errors.rs:3:next_step","answer":"no","reason":"It only says the config is missing."}
$ tracker submit BM-1
Running criterion 1's check: `test -f found`
Criterion 2's judge at src/errors.rs:3: answered from the cache
error: BM-1 can't be submitted: G3 failed. Nothing was changed.
  G3  Criterion 2's judge answered no to c2:src/errors.rs:3:next_step, "Does this error message tell the user what to do next?", and only yes passes: It only says the config is missing. Its excerpt follows.
        [match]
        src/errors.rs:3: bail!("the config is missing");
  
        [code]
  
            bail!("the config is missing");
        }
[exit 2]
$ printf '%s' '{"adapter":"fake 1","model":"claude-haiku-5-5","answers":{"next_step":{"answer":"unsure","reason":"The next step may be printed by the caller."}}}' > $DEMO/judge/reply.json
$ printf 'fn load() {\n    bail!("the config is missing");\n}\n' > src/errors.rs && git add -A && git commit --quiet -m "feat(BM-1): drop the comment"
$ tracker gates BM-1 --json | jq -c '{judges, holds}'
Running criterion 1's check: `test -f found`
Asking criterion 2's judge 1 question at src/errors.rs:2: `$DEMO/judge/fake`
{"judges":[{"ref":"c2:src/errors.rs:2:next_step","criterion":2,"answer":"unsure","reason":"The next step may be printed by the caller.","result":"unsure"}],"holds":[{"trigger":"T10","message":"a judge answered unsure to c2:src/errors.rs:2:next_step, so a human decides: The next step may be printed by the caller."}]}
$ tracker submit BM-1
Running criterion 1's check: `test -f found`
Criterion 2's judge at src/errors.rs:2: answered from the cache
BM-1: in-progress → review
Every gate passed, and BM-1 is held for review:
  T10  a judge answered unsure to c2:src/errors.rs:2:next_step, so a human decides: The next step may be printed by the caller.
    c2:src/errors.rs:2:next_step: Does this error message tell the user what to do next?
      unsure: The next step may be printed by the caller.
      [match]
      src/errors.rs:2: bail!("the config is missing");
     
      [code]
      fn load() {
          bail!("the config is missing");
      }
It waits in `tracker review` for a human to approve or reject it. `tracker show BM-1 --diff` shows its work.
$ tracker show BM-1 | sed -n '/^Held/,$p'
Held for review by:
  T10  a judge answered unsure to c2:src/errors.rs:2:next_step, so a human decides: The next step may be printed by the caller.
$ cd ../im
$ printf '%s' '{"adapter":"fake 1","model":"claude-haiku-5-5","answers":{"next_step":{"answer":"yes","reason":"It names the command to run."}}}' > $DEMO/judge/reply.json
$ tracker create "Imports help" --project IM
Created IM-1: Imports help
$ tracker criterion add IM-1 --judge $DEMO/judge/errors.json
Added criterion 1 to IM-1, a judge: Every error says what to do next
Its question is asked at submit, once every other gate passes.
$ tracker start IM-1
IM-1: open → in-progress
$ touch found && git add -A && git commit --quiet -m "feat(IM-1): import a file"
$ tracker submit IM-1
Asking criterion 1's judge 1 question at src/errors.rs:2: `$DEMO/judge/fake`
IM-1: in-progress → done
Verified automatically: every gate passed at a1feb42, and refs/tracker/IM-1 keeps that commit.
$ jq . $DEMO/judge/last-request.json
{
  "model": "claude-haiku-5-5",
  "state": {
    "match": "src/errors.rs:2: bail!(\"the link file is missing: run tracker import --help\");",
    "code": "fn load() {\n    bail!(\"the link file is missing: run tracker import --help\");\n}"
  },
  "questions": {
    "next_step": {
      "instructions": "Does this error message tell the user what to do next?",
      "criteria": {
        "true": "the message gives a concrete next step",
        "false": "the message only reports the problem"
      }
    }
  }
}
$ tracker show IM-1 | sed -n '/^Verification/,$p'
Verification:
  by      auto
  at      2026-11-23T03:46:03Z
  commit  a1feb422573ed222e4897d83c9aa2aec3c940e61
  ref     refs/tracker/IM-1
  judges:
    c1:src/errors.rs:2:next_step, claude-haiku-5-5 answered yes: It names the command to run.

History, oldest first (2 entries):
  2026-11-23T03:46:03Z  start   open → in-progress  not from a terminal
  2026-11-23T03:46:03Z  submit  in-progress → done  not from a terminal
$ jq '.criteria[0], .verification.judges' $TRACKER_HOME/items/IM-1.json
{
  "text": "Every error says what to do next",
  "inputs": [
    "src/errors.rs"
  ],
  "each": "bail!",
  "context_lines": 1,
  "ask": [
    {
      "id": "next_step",
      "ask": "Does this error message tell the user what to do next?",
      "yes": "the message gives a concrete next step",
      "no": "the message only reports the problem"
    }
  ]
}
[
  {
    "ref": "c1:src/errors.rs:2:next_step",
    "model": "claude-haiku-5-5",
    "request": "dcf21de573873dc22db45b89e801c9ea34a66fcba0d9462a89c0f2674aa6adb6",
    "answer": "yes",
    "reason": "It names the command to run."
  }
]
$ tracker list --all --status review,done
  ID    PRIORITY  KIND    STATUS  TITLE
BM  bookmarks
  BM-1  medium    change  review  Errors help
IM  importer
  IM-1  medium    change  done    Imports help

Every project.
Filters: status review or done.
```

Notes on the output:

- The commit SHAs, the times and every `request` fingerprint differ on every run. The fingerprint includes the judge command, whose path here is the temporary folder.
- The `Asking criterion 2's judge …` and `answered from the cache` lines go to stderr. They're how a person sees whether the judge was called. The first `tracker gates BM-1` calls it. The second, on unchanged work, is answered from the cache, so the `calls` file has one line, and the `gates --json` and the `submit` after it are answered from the cache too.
- BM's first request has a comment, `// BUG? this looks wrong`, above the `bail!`. The excerpt printed with the failure shows the comment's line as empty: the judge never sees it. The `[code]` has one line either side of the match, since the judge file's `context_lines` is 1.
- The refusal's `[match]` and `[code]` are the excerpt the judge saw, and `c2:src/errors.rs:3:next_step` is the question's ref: criterion 2, the file and line of the match, and the question's `id` (§2.3.2).
- After BM-1's commit that removes the comment, the match is on line 2, so the request, its fingerprint and its ref change, and the judge is asked again. The change from `no` to `unsure` is only possible because the work changed: an unchanged request keeps its first answer.
- BM-1's hold is T10 alone, and `submit` prints the unsure answer's question, reason and excerpt under it, as a failed question's refusal does, so the person deciding sees what the judge saw. It sits in review and pauses BM (§4.4), as any held item does. `approve` needs a terminal, so the demo stops there. A human would approve or reject it, as in slice 9.
- IM-1's judge also got a `fake` call, whose request `last-request.json` shows: `model`, `state` with `match` and `code`, and `questions` keyed by `id`.
- `tracker show IM-1` prints the judge's answer with the model that gave it. The item file has the criterion's judge fields between `text` and where `check` and `evidence` would be, in the order `inputs`, `each`, `context_lines`, `keep_comments`, `ask`, and the record's `judges`.

## Commands

| Command | What it does |
| --- | --- |
| `tracker criterion add <id> [text] --judge <file>` | Adds a judge criterion from the file. The file gives the text, so `text` can be left out, and if it's given it must equal the file's. `--judge` can't be used with `--check`. From an interactive terminal the text is also recorded in `written`, as for any added criterion |
| `tracker criterion set <id> <n> --judge <file>` | Gives criterion `n` the judge, in place of a check, manual evidence or an earlier judge. The criterion keeps its text, so a file with other text is refused. `--check` or `--judge` is required, and they can't be used together. After `start`, it also removes the criterion's entry from `started.checks`, since the old check's start result isn't the judge's |
| `tracker criterion set <id> <n> --check "…"` | Also replaces a judge with a check, dropping its judge fields |
| `tracker gates <id>`, `submit <id>`, `status <id> done`, `approve <id>` | After every other gate has passed, G3 asks each judge criterion's questions. A no fails G3 (exit 2), an unsure answer fires T10 once the gates pass, and `gates` and `submit` then print its question, reason and excerpt under the hold, as does `approve`'s question. `approve` is refused without a terminal before it asks any judge |
| `tracker evidence <id> <n> …` | Refuses a judge criterion, since its answers are its proof (exit 1) |
| `tracker show <id>` | Prints a judge criterion's reading and questions, and a done item's judge answers |

Input rules, as in [§3](DESIGN.md#3-interface): a criterion's text is one line of at most 500 characters, and so are each question and the meaning of a yes or no. A usage error, such as `--check` with `--judge`, exits 1, as clap errors do in every earlier slice. A judge file that can't be read or isn't valid exits 1 with a message that completes "The judge file <path> …". A refused gate exits 2. Judges aren't run when one is added, or at `start` (slice 14).

## The judge file

`criterion add --judge` and `criterion set --judge` read a JSON file with exactly these fields (`judge::JudgeFile`, which refuses any other):

| Field | Meaning |
| --- | --- |
| `text` | The criterion's text, one line of at most 500 characters |
| `inputs` | Required, not empty. Files relative to the project's root, or `diff` for the item's changes. An absolute path, or an empty one, is refused |
| `each` | Optional regex. The questions are asked once per matching line |
| `context_lines` | Optional. Lines either side of a match. The config's `judge.context_lines` (default 12) when it's left out |
| `keep_comments` | Optional, false by default. When true, comments aren't stripped |
| `ask` | Required, not empty. Each question has an `id` (letters, digits, `_` and `-`, up to 64, and unique in the file), `ask` (the question), optional `yes` and `no` (what each answer means) and optional `pass_when` (`yes` or `no`, `yes` by default). `pass_when: unsure` is refused |

JSON treats a backslash as an escape, so a regex's backslashes are doubled in the file, as §2.3.1 says. The criterion is stored with the same fields, in this order, between `check` and `evidence`, and each is left out when it isn't set ([schemas/item.schema.json](../schemas/item.schema.json) `question` and `criterion`). `criterion add` and `set` also refuse a file whose input doesn't exist or isn't UTF-8 text, whose input leads outside the root through a symlink (as G6 does for a design), or whose `each` matches nothing in its file inputs once comments are stripped. A `diff` input isn't read then, since the item may have no commits yet.

## The judge command's contract

The config's `judge.command` is a line of `sh`. The tool runs it in the system's temporary folder, writes one JSON request on its stdin, and reads one JSON reply from its stdout. Each excerpt is one request.

The request is `{ "model", "state", "questions" }`:

- `model` is `judge.model` (default `claude-haiku-5-5`).
- `state` is the excerpt as named sections. For a whole-input judge, each input's name maps to its text. For an `each` judge, `match` is the matched line, after its `file:line`, and `code` is the lines around it.
- `questions` maps each question's `id` to `{ "instructions": <the question>, "criteria": { "true": <yes>, "false": <no> } }`, and `criteria` holds only the meanings the question sets. That's the shape judge-eval's adapter reads.

The reply is `{ "adapter", "model", "answers", "cost_usd" }`, where `answers` maps each `id` to `{ "answer": "yes" | "no" | "unsure", "reason": "…" }`. `adapter` is a name and version, and `cost_usd` is optional. The tool refuses a reply that isn't JSON, names another model than the one asked for, leaves a question unanswered, or gives an answer that isn't yes, no or unsure. It also refuses a command that exits non-zero (it shows what the command wrote to stderr), can't be run, or runs longer than `judge.timeout_secs` (default 60) for each question in the request, when it's stopped with everything it started. Each of these fails G3 for the judge criterion, and a failed call never counts as a pass.

## How it works

### Where the judges run

`ask_judges` in [src/main.rs](../src/main.rs) calls `gates::judges` only when the report so far has passed, so a judge never grades work that has already failed a gate (§2.4). It runs after `gates::tree_after_checks`, in `gates_dry_run` and in `apply` for `submit` and `status … done`. `gates::judges` runs each judge criterion through `judge::grade`, then adds a `Failure` for each reason a criterion couldn't be graded and for each answer whose `Answered::outcome` is `Fail`. Every answer, including passes and unsure ones, goes into `Report::judges`, in order. That list is what `--json` shows as `judges`, what T10 reads, and what `verification_record` copies into the record. It's G3's judge half, so it adds nothing to the list of gates that ran: `g3` has already named G3.

A criterion with an empty `ask` isn't a judge, so an item with none never calls a judge, reads nothing from the config's `judge`, and gets no `judges` key in `--json`.

### Building a request

`judge::grade` reads each input: a file through `read_file`, which follows symlinks and refuses a path outside the root, a directory and text that isn't UTF-8, or `diff` through `read_diff`, which is every item commit's patch, oldest first, as `show --diff` prints them. `strip` removes comments unless the criterion keeps them. `strip_comments` is a scanner, not a parser, that picks a style from the file's extension (`style_for`): `//` and `/* … */` for Rust, JavaScript, C and the like, `#` for Python, shell, Ruby, TOML and YAML, and `<!-- … -->` for HTML and XML. It skips strings and character literals, so `"http://…"` and a Rust lifetime aren't cut, keeps every line break so line numbers don't move, and trims each line's trailing spaces. Other files, such as markdown, are left as they are. A `diff` is stripped by `strip_diff`, file by file, in each file's style, keeping its headers and each `\ No newline at end of file` line, which sits inside a hunk and doesn't end it. Over a `diff` input, `each` matches the patch text as `git show` prints it and counts lines from its start, so a new item commit renumbers them, and each commit's subject is in the patch text, unstripped (§2.3.1).

`excerpts` makes the excerpts. Without `each` there's one, holding every input. With it there's one for each line that has a match, so two matches on one line are one excerpt. Its `at` is `file:line`, with the line counted in the stripped text (which keeps every line break, so it's the file's line), and the lines either side are `context_lines` (the criterion's, or the config's `judge.context_lines`, or 12). `each` matches the stripped text, so a match only in a comment doesn't count. An excerpt whose bytes (every section's text) exceed `judge.max_input_bytes` (default 40000) isn't asked: that excerpt's failure says so and names the setting, and `grade` goes on to the next excerpt.

### The cache and the fingerprint

`fingerprint` is the SHA-256, in lower-case hex, of a JSON object of the judge command and the request. `cached` reads `judge/cache/<fingerprint>.json` (`Store::judge_cache_dir`) and uses it only if it answers every question. `save` writes a new reply there through `store::write_file`, so a reader never sees half a file. A reply that can't be cached fails the call. The record's `request` is this fingerprint, which names the cache entry. When the cache is used, `grade` prints "answered from the cache" to stderr, and otherwise "Asking criterion n's judge …".

The key covers the command and the request, which holds the model, the excerpt and every question with its meanings, but not the adapter's name or version, since the adapter reports those only in its reply. A new adapter version run by the same command therefore reuses the old answers until `judge/cache/` is cleared ([Decisions](#decisions-worth-knowing)). The cache is global, so one item's answer is reused by any item whose request is identical, including a different item in another project, if its excerpt and questions match. Nothing prunes it.

The cache is why unchanged work can't be asked again until it gets a better answer: `gates` run repeatedly returns the first answer. Changing the work, a question or the model changes the request and so asks again.

### What each answer means

`Answered::outcome` compares the answer with the question's `pass_when`: the same is a pass, `unsure` is `Unsure`, and the other is a fail. A fail makes a G3 failure for the question, whose message names the criterion, the answer, the ref, the question, what passes, the reason, and says its excerpt follows. The excerpt, as the judge was sent it (`render`: each section as `[name]` and its text), is printed under the failure in text output.

A question's ref is `c<n>:<file>:<line>:<id>` for an `each` match and `c<n>:<id>` for a whole input, with `n` the criterion's number from 1 (§2.3.2). Removing a criterion renumbers the ones after it, and so their refs.

`grade` stops asking a criterion's other excerpts once a call fails, since the next would most likely fail the same way, and each takes a while. A criterion with nothing to grade, a missing input or an oversized excerpt fails without a call for it.

### T10

`triggers::t10` fires for an `unsure` answer. For one, its message names the ref and the reason: "a judge answered unsure to c2:src/errors.rs:2:next_step, so a human decides: …". For several, it counts them and lists the refs. It also fires when `Project::auto_verify` is false and some judge was asked, whatever the answers: "the project's judge settings have auto_verify false, so an item a judge passed waits for a human". An item with no judge, so no `judges`, is quiet. `run` takes `judges` as a third argument, and adds T10 after T9 and before T13, so `triggers` lists twelve. `Review::runs` always runs it, since `config::ALWAYS_ON` has T10, so `review.disabled: ["T10"]` changes nothing.

An unsure answer's question, reason and excerpt are shown by `unsure_text` in [src/main.rs](../src/main.rs), in the layout of a failed question's excerpt (§2.3.1), with `unsure: <reason>` under the question. T10's message carries the reason only when one answer is unsure, so with several, this is where each reason is. `gates`'s dry run and `submit`'s text print it under the holds, and so does `approve`'s question: `decision_warning` takes the answers `approve` has just read from the cache and prints `unsure_text` of them, and `reject` passes none. `review` doesn't print them. For a T10 hold whose message says "answered unsure" it prints a line naming `tracker gates <ID>`, which "shows each unsure answer's question and excerpt" from the cache without a call. An `auto_verify: false` hold, where every answer passed, gets no such line, since `gates` prints no question for it. `show` doesn't print them either, and nothing stores the excerpt ([Still open](#still-open)).

### Approve

`approve` checks the item's status first, then `tty::require_interactive`, so a piped `approve` is refused before any trigger runs or judge is asked (review 1's bug 1, [below](#fixes-after-review-1)). The terminal check isn't at the very top because two older `tests/review.rs` tests expect the status error first. It still runs before any judge or gate. In a terminal, `approve` calls `ask_judges` before it asks, on an empty report, so T10's reasons are in the question it prints, followed by each unsure answer's question, excerpt and reason (`unsure_text`), and so unchanged work gets its answers from the cache. After the confirmation it runs the gates again, and the judges again after the script gates, as `submit` does. A judge that can't be graded fails that second run, which leaves the item in review with exit 2 as any failed gate does. The first call isn't behind the script gates, so a judge whose answer isn't cached is asked at that point even if a check would fail ([Still open](#still-open)).

### Config

`config::Judge` (global: `command`, `model`, `max_input_bytes`, `context_lines`) and `config::ProjectJudge` (`max_failures`, `auto_verify`, `timeout_secs`) both refuse unknown keys, and each setting left out takes its default: `tracker-judge`, `claude-haiku-5-5`, 40000, 12, `auto_verify` true and a timeout of 60 seconds. Both objects are kept as written, so a hand edit survives `project add`. Before this slice a project's `judge` was kept as any JSON value, so one with another key is now refused when the config is read (§4.4). `max_failures` is read and kept, and nothing uses it until slice 14.

### Items written by an older binary

The five new criterion fields and `verification.judges` have serde defaults and `skip_serializing_if`, so an item without a judge comes back byte for byte, and its fingerprint (T5) is unchanged. The `criteria_hash` golden test, `the_fingerprint_of_fixture_xx_23_is_the_one_slice_11_recorded`, passes unchanged. The fingerprint hashes each criterion with its evidence removed, so a judge's fields are covered and a judge's answers, which live in `verification.judges`, aren't.

## JSON output

| Command | What's new | Schema |
| --- | --- | --- |
| `gates --json`, `submit --json` | `judges`, an array of `{ ref, criterion, answer, reason, result }` (`result` is `pass`, `fail` or `unsure`) for every question G3 asked, left out when the item has no judge. It sits after `gates`, before `triggers`. `triggers` lists twelve entries, with T10 after T9 | [gates.schema.json](../schemas/gates.schema.json), [submit.schema.json](../schemas/submit.schema.json), [item.schema.json](../schemas/item.schema.json) `judged` |
| A refused `gates`, `submit` or `approve` | `judges` as above, and each failure for a question carries `ref`, `answer` and `reason`, with the `criterion` it's about | [error.schema.json](../schemas/error.schema.json) |
| Any command that returns an item | A criterion can have `inputs`, `each`, `context_lines`, `keep_comments` and `ask`. `verification.judges` is `{ ref, model, request, answer, reason }` for each question, and `request` is 64 lower-case hex characters | [item.schema.json](../schemas/item.schema.json) |

The criterion schema refuses a criterion with more than one of `check`, `ask` and `evidence`, and one with `inputs`, `each`, `context_lines` or `keep_comments` but no `ask`. A later slice that adds a field to a judge, or to `verification.judges`, must serialise it only when it's set. It must not rename or reorder the criterion's fields, which changes every fingerprint ([Limits](#limits-and-what-later-slices-change)).

## Code tour

| File | What's new or changed |
| --- | --- |
| [src/judge.rs](../src/judge.rs) | New, 909 lines. `parse_file` and `check_inputs` (the judge file), `grade` (one criterion: inputs, excerpts, requests, cache, calls), `Answered`, `Outcome` and `Graded`, `excerpts`, `request`, `fingerprint`, `cached` and `save`, `ask` and `run` (the command, with the timeout), `style_for`, `strip_comments` and `strip_diff`. Four unit tests |
| [src/gates.rs](../src/gates.rs) | `judges` (G3's judge half), `Failure::judge`, `Report::judges` and `judges_json`, and the `judges` field in `refusal`'s JSON and the text of a failed question's excerpt |
| [src/triggers.rs](../src/triggers.rs) | `t10`, and `run` takes the judges' answers |
| [src/item.rs](../src/item.rs) | `Criterion`'s judge fields and `is_judge`, `Question`, `Answer`, `JudgeRecord` and `Verification::judges`. `Criterion` derives `Default` |
| [src/config.rs](../src/config.rs) | `Judge`, `ProjectJudge`, the defaults, `Config::judge_command`, `judge_model`, `max_input_bytes` and `context_lines`, and `Project::auto_verify` and `judge_timeout` |
| [src/main.rs](../src/main.rs) | `criterion add` and `set` take `--judge`, and `criterion_judge` reads, checks and stores it. `ask_judges` is called by `gates_dry_run`, `apply` and `approve`, which refuses a non-terminal first (`tty::require_interactive`). `unsure_text` prints an unsure answer's question, reason and excerpt, `decision_warning` takes the judges' answers and prints them in `approve`'s question, `review` names `tracker gates` only for a T10 hold from an unsure answer, and `criterion_judge` drops a replaced check's `started.checks` entry. `verification_record` copies the answers. `show` prints them (`judge_text`, `verification_text`). `record_evidence` refuses a judge. `submit`'s help names T10 |
| [src/checks.rs](../src/checks.rs), [src/store.rs](../src/store.rs), [src/lib.rs](../src/lib.rs) | `checks::wait` and `stop_group` are public, so `judge::run` stops a command as a check is stopped. `Store::judge_cache_dir`. `pub mod judge` |
| [Cargo.toml](../Cargo.toml), `Cargo.lock` | `regex` 1.13.1 |
| [schemas/item.schema.json](../schemas/item.schema.json), [gates.schema.json](../schemas/gates.schema.json), [submit.schema.json](../schemas/submit.schema.json), [error.schema.json](../schemas/error.schema.json) | The judge fields and `judged` and `question` definitions, `judges` in the commands' output, and a failure's `ref`, `answer` and `reason` |
| [tests/judges.rs](../tests/judges.rs) | New: 42 tests in eighteen modules, `criterion_1` to `criterion_10`, from review 1 `tr45_c1` to `tr45_c5`, from review 2 `tr46_c1` to `tr46_c3`, and helpers |
| [tests/fixtures.rs](../tests/fixtures.rs), `tests/fixtures/slice-12/` | XX-24, and `slice_12s_fixture_has_a_judge_criterion_and_judge_records` |
| [tests/criteria.rs](../tests/criteria.rs), [tests/gates.rs](../tests/gates.rs), [tests/item_files.rs](../tests/item_files.rs), [tests/submit_help.rs](../tests/submit_help.rs), [tests/review.rs](../tests/review.rs), [tests/states.rs](../tests/states.rs) | Existing tests changed in `b8f378c`, [below](#older-tests-that-changed) |
| [docs/DESIGN.md](DESIGN.md) | `919e022`: §2.3.1's bullets for comment stripping, `each`, nothing to grade, the cache and the adapter, and §3's `judges` field. `d356d17`: §2.3.1's step 4, its comment bullet for `diff` inputs, and a §7 bullet on a forged judge answer |
| [docs/PLAN.md](PLAN.md) | Slice 12's status, `todo` to `in review` (`932c1c4`). `a03a8b8` marked it `in progress` |

To add a judge setting: add it to `config::Judge` or `ProjectJudge`, give it a default constant and a method like `Config::max_input_bytes`, and document it in DESIGN.md §4.2. To add a language's comment style, add an extension to `style_for` and, if it's a new kind of comment, a `Style` and a branch in `strip_comments`. To add a field to a question, put it on `item::Question`, with a serde default and `skip_serializing_if`, add it to `request` and to the schema's `question`, and say whether it belongs in the cache key (anything in the request already is).

## Testing

Run the test command in `tracker/`. TR-8's ten criteria each run their own module in `tests/judges.rs`, for example `cargo test --test judges criterion_6::`. The module path ends in `::`, so `criterion_1::` can't match a `criterion_10`. TR-45's criteria run `tr45_c1::` to `tr45_c5::` the same way, and TR-46's run `tr46_c1::` to `tr46_c3::`.

Every project in `tests/judges.rs` is XX, whose root `<work>/xx` is a subfolder of the work folder's repository, as TR's root is a folder in guild-portfolio. `env_with_item` commits `xx/src/errors.rs`, two `bail!`s each after a comment that could sway a model, creates XX-1 and gives it a test commit. The **fake judge** is a shell script in `<work>/judge/`, outside the root, that saves each request as `request-<n>.json` and prints `reply.json`, optionally after sleeping, and `install_fake` makes it the config's `judge.command`. So a test sets the answers with `reply`, reads what the judge was sent with `requests`, and counts the calls. `with_judge` adds a judge file as XX-1's only criterion, and `start_and_work` starts it and commits the work. No test needs `script` or the `claude` CLI. The harness in [tests/common/mod.rs](../tests/common/mod.rs) is unchanged.

### Coverage of the _Done when_ checks

| _Done when_ | Tests in `tests/judges.rs` |
| --- | --- |
| Tests cover a pass, a fail and an unsure answer. | `criterion_1`, 3 tests: `a_yes_passes_and_the_item_is_verified_automatically`, `a_no_fails_g3_and_refuses_the_submit` and `an_unsure_answer_holds_the_item_under_t10` |
| Tests cover a `"pass_when": "no"` question. | `criterion_2`, 2 tests: `a_no_passes_a_pass_when_no_question` and `a_yes_fails_a_pass_when_no_question` |
| Tests cover malformed output and a timeout. | `criterion_3`, 4 tests: `a_reply_that_isnt_json_fails_g3`, `an_answer_that_isnt_yes_no_or_unsure_fails_g3`, `a_reply_missing_a_question_fails_g3` and `a_judge_that_runs_past_its_timeout_is_stopped_and_fails_g3` (the fake sleeps 20 seconds with a 1 second `timeout_secs`, and the test checks it was back in under 15) |
| Tests cover an oversized excerpt, refused before the judge is called. | `criterion_4`, 1 test: `an_excerpt_over_max_input_bytes_fails_g3_without_calling_the_judge`, with `max_input_bytes` 20 and no request recorded |
| Tests cover a cache hit, where the fake judge isn't called again. | `criterion_5`, 2 tests: `unchanged_work_is_answered_from_the_cache` (the answer is changed to no between two `gates`, and the second still says yes with one request recorded and one file in `judge/cache`) and `changed_work_asks_the_judge_again` |
| `--judge` refuses invalid JSON, an invalid regex and an `each` that matches nothing. `set` refuses a file whose `text` differs. | `criterion_6`, 8 tests: `a_judge_file_that_isnt_json_is_refused`, `an_each_that_isnt_a_regex_is_refused`, `an_each_that_matches_nothing_is_refused`, `an_each_that_matches_only_a_comment_is_refused`, `an_input_that_doesnt_exist_is_refused`, `a_valid_judge_is_copied_into_the_item`, `set_refuses_a_judge_whose_text_differs` and `set_gives_a_criterion_with_the_same_text_a_judge` |
| G3 fails a judge after a later commit removes every match. Judges don't run when a script check failed. | `criterion_7`, 3 tests: `g3_fails_a_judge_once_a_commit_removes_every_match`, `judges_dont_run_when_a_script_check_fails` and `judges_dont_run_when_a_project_check_fails`, each checking that no request was recorded |
| The fake judge receives the excerpts with comments stripped, and refs look like `c2:file:line:id`. | `criterion_8`, 4 tests: `the_judge_gets_each_excerpt_without_its_comments` (neither `BUG?` nor `reviewer` was sent), `keep_comments_keeps_them`, `refs_name_the_criterion_the_match_and_the_question` (`c2:src/errors.rs:3:suggests_next_step` and `…:8:…`) and `a_ref_for_a_whole_input_leaves_out_where` (`c2:q`) |
| `--json` failures carry `ref`, `answer` and `reason`. A done item's verification record has one judge entry per question. | `criterion_9`, 2 tests: `a_failed_question_carries_its_ref_answer_and_reason` and `a_done_items_record_has_one_judge_entry_per_question` (two questions, two entries, each with a 64-character `request`) |
| `auto_verify: false` holds under T10 (the criterion the builder added). | `criterion_10`, 3 tests: `auto_verify_false_holds_a_judge_passed_item_under_t10`, `t10_cant_be_switched_off` (`review.disabled: ["T10"]` and an unsure answer) and `t10_stays_quiet_for_an_item_with_no_judge` |

The _Prompt_'s requirements that no _Done when_ bullet names:

| Requirement | Tests |
| --- | --- |
| Judges are asked only after every script gate passes. | `criterion_7::judges_dont_run_when_a_project_check_fails`, and the script check one above |
| Comment stripping, in each language's style, and a diff stripped file by file. | The unit tests in `src/judge.rs`: `c_like_comments_go_and_strings_and_line_breaks_stay`, `hash_comments_go_but_not_inside_strings` and `a_diff_is_stripped_in_each_files_style_and_its_headers_kept` |
| A judge criterion from a file is stored with its fields in order, and an item with one comes back byte for byte. | `fixtures::slice_12s_fixture_has_a_judge_criterion_and_judge_records` reads XX-24: the criterion's keys are `text`, `inputs`, `each`, `context_lines`, `ask`, the record's are `by`, `at`, `commit`, `ref`, `checks`, `judges`, and a judge entry's are `ref`, `model`, `request`, `answer`, `reason`. It checks that the file comes back byte for byte, loses nothing on a rewrite, and matches the schema. By the test's own comment, the fixture is a file slice 12's binary wrote |
| An item without a judge is unchanged, including its fingerprint. | `t5_and_fingerprint::criterion_1::the_fingerprint_of_fixture_xx_23_is_the_one_slice_11_recorded`, unchanged |
| T10 is in every list of triggers. | The six pinned lists that changed ([below](#older-tests-that-changed)), and `submit_help::submit_help_names_every_trigger` |

Not tested by an automated test: a terminal `approve` of an item with a judge criterion that goes through to done (`tr46_c2` opens `approve`'s question in a terminal and cancels it, and returns without checking when the harness has no terminal), `show`'s text for a judge and for its answers, and `evidence` refused for a judge. The last two were checked by hand in [the demo](#try-it). A reply that can't be cached isn't tested either, and neither is a file input deleted after `criterion add`. A judge whose input is `diff` and a piped `approve` were on this list until review 1's TR-45.

### Tests added after review 1

TR-45's tests, from `deb662a`, committed before `tracker start TR-45`. Each module runs one criterion's check.

| Module in `tests/judges.rs` | Tests | What each proves |
| --- | --- | --- |
| `tr45_c1` | `a_piped_approve_is_refused_without_calling_the_judge` | A piped `approve` of a T10 hold, with the cache removed, is refused for having no terminal and the fake judge isn't called |
| `tr45_c2` | `a_diff_judge_gets_no_comments_even_after_a_missing_newline_line`, and the unit test `a_no_newline_marker_stays_in_its_hunk_and_stripping_goes_on` in `src/judge.rs` | A `diff` judge over two item commits, one ending a file without a newline, sends no comment to the fake judge, and its ref starts `c1:diff:`. A `diff` ref names `diff` and a line of the combined patch text, never the changed file, and the test doesn't check the line (review 2's test gap 1). The unit test is bug 2's hunk |
| `tr45_c3` | `set_judge_after_start_drops_the_started_check_entry` | `criterion set --judge` after `start` removes that criterion's `started.checks` entry |
| `tr45_c4` | `submit_and_gates_text_show_the_question_and_excerpt_of_an_unsure_answer` | `submit` and `gates` text name the ref, the question and the excerpt, and `review` names `tracker gates XX-1` |
| `tr45_c5` | `a_reply_naming_another_model_fails_g3` and `a_judge_command_that_exits_non_zero_fails_g3_with_its_message` | Each fails G3 with a reason that says so |

Criterion 6's check is five `grep` calls on DESIGN.md.

### Tests added after review 2

TR-46's tests, from `84b8c04`, committed before `tracker start TR-46`. They're new modules, and no existing test was edited. Each module runs one criterion's check, and all 3 checks failed at `start`.

| Module in `tests/judges.rs` | Tests | What each proves |
| --- | --- | --- |
| `tr46_c1` | `two_unsure_answers_each_show_their_reason_in_gates_and_submit` | With two unsure answers, `submit` and `gates` text each show the reason twice, once for each answer |
| `tr46_c2` | `approve_shows_the_question_excerpt_and_reason_of_an_unsure_answer` | In a terminal, `approve`'s question for a T10 hold shows the question, the excerpt and the reason (the test cancels it with a wrong ID) |
| `tr46_c3` | `review_hints_at_gates_for_an_unsure_answer` and `review_gives_no_hint_for_an_auto_verify_false_hold` | `review` names `tracker gates` with the reworded line for an unsure answer, and doesn't for an `auto_verify: false` hold |

### Older tests that changed

`b8f378c` changed existing tests before `tracker start TR-8`, so they were seen failing with TR-8's. T4's hold names the four that had lines changed, each "(b8f378c before start)":

| Test | File | What changed |
| --- | --- | --- |
| `criterion_accept_refuses_a_newer_trackers_file_before_it_asks` | `tests/criteria.rs` | The made-up field from a newer tracker was `ask`, which is now a real field. It's `rubric` |
| `a_newer_trackers_file_is_refused_with_exit_3_before_any_gate_runs` | `tests/gates.rs` | The same, `ask` to `rubric`. Its pinned `triggers` list also gained T10 in `a_script_criterion_needs_no_evidence` |
| `lost_on_rewrite_tells_unknown_fields_from_values_written_differently` | `tests/item_files.rs` | Its unknown field in `verification` was `judges`, now real. It's `signature` |
| `submit_help_names_every_trigger` | `tests/submit_help.rs` | The list gained T10 |

The pinned `triggers` lists in `tests/states.rs` (`submit_prints_the_shape_section_3_gives_it`) and `tests/review.rs` (`criterion_1`, `criterion_2` and `tr_29`, four lists) only gained a line for T10. T4 fires only when lines are changed or removed, so those two files weren't named. `tests/gates.rs` was, because of the `rubric` change.

### Checks done by hand

| What | How | Result |
| --- | --- | --- |
| The demo above | Run as written with this branch's `target/debug/tracker`, stderr captured, on a temporary `TRACKER_HOME`, with the temporary folder replaced by `$DEMO` | The output above. It exercises every refusal and the cache, and also `evidence` and `show` for a judge |
| TR-8's checks, before the approve | The orchestrator checked by hand, while TR-8 waited in review, that the 9 criteria written at create are unchanged, that all 10 checks failed at `start`, and that every commit since `start` is scoped to TR-8 | All three held. The slice 11 binary that tracked TR-8 has T5, T13 and T14, so it had checked them too, and none fired |
| TR-8's approve in a real terminal | The user read the diff and typed `TR-8` into `tracker approve TR-8` at a terminal | `by: human` at `932c1c4`, with `refs/tracker/TR-8`. The holds recorded are the five below |
| TR-45's checks, before the approve | The orchestrator checked by hand, while TR-45 waited in review, that its 6 written criteria are unchanged and that every commit since `start` is scoped to TR-45 | Both held. The slice 11 binary that tracked it has T5 and T14, so it had checked them too, and neither fired |
| TR-45's approve in a real terminal | The user read the diff and approved it in a terminal | `by: human` at `d356d17`, with `refs/tracker/TR-45`. The holds recorded are T8 (`docs/DESIGN.md`) and T13 (criterion 5) |
| The demo, after review 1's fixes | Run again as written, stderr captured | Only `submit`'s output changed: the T10 hold is followed by the unsure answer's question and excerpt (above). The times, SHAs and `request` fingerprint differ as before |
| The demo, after review 2's fixes | Run again as written, stderr captured | Only `submit`'s output changed from the version above: `unsure: <reason>` under the question. Nothing else differed except the times, SHAs and `request` fingerprint |

### Tracking TR-8

TR-8, a `capability` with `design` `docs/DESIGN.md#2.3.1`, was created at 2026-11-21T07:07:43Z with slice 7's other items. The slice 11 binary was installed before it started and tracked it throughout:

| Step | When (UTC) | What |
| --- | --- | --- |
| Tests and the older tests' changes, committed | 03:08:49Z | `b8f378c`, 21 seconds before `start` |
| `tracker start TR-8` | 03:09:10Z | `started.head` `b8f378c`, with a `criteria_hash` (`d88b85d8…`). All ten checks failed |
| The code | 03:22:46Z | `1c95fda`, then the fixture `a301e27` and DESIGN.md `919e022` |
| `tracker submit TR-8` | 03:29:27Z | The gates passed, then the triggers ran. Held under T1, T3, T4, T7 and T8 |
| `tracker approve TR-8` | 03:41:55Z | From a terminal, verified `human` at `932c1c4` |

- **The five holds** read: T1, "it's a capability, and adds the source file src/judge.rs". T3, "dependency manifests changed: Cargo.lock and Cargo.toml". T4, "existing test files modified", the four files above, each "(b8f378c before start)". T7, "2675 lines changed, more than the project's max_diff_lines of 300". T8, "the project's design doc changed: docs/DESIGN.md". They're recorded in `verification.triggers`, and `tracker show TR-8` is the record, with each check's command, result and saved output.
- **T5, T13 and T14 ran and stayed quiet**, since slice 11's binary has them. It has no T10, which this slice adds, and none of TR-8's criteria is a judge, so there was nothing for T10 to read.
- **A judge criterion has no `started.checks` entry**, since judges don't run at `start`, so T13 is quiet for one until slice 14. TR-8's criteria are all script checks.
- **TR-44** (kind `fix`, priority low) was created by the builder for what this slice left as it was: a clearer message when `started.head` is lost ([Decisions](#decisions-worth-knowing)).

## Decisions worth knowing

- **The cache key is the judge command plus the request, not the adapter's name and version.** DESIGN.md first said the key included them, but the adapter reports them only in its reply, so the key can't be computed before the call. They're kept beside the answers instead. A new adapter run by the same command reuses the old answers until `judge/cache/` is cleared. DESIGN.md §2.3.1 now says so, in its cache bullet and, since review 1, in tracker-judge's step 4 as well. Slice 13, which writes the adapter, may revisit it, for example by putting the adapter's version in the command or in the request.
- **Judge fields are written only when set, between `check` and `evidence`.** That keeps slice 11's `criteria_hash` for an item without a judge, so the XX-23 golden test passes unchanged, and T5 covers a judge's fields as it does a check. The answers aren't covered, since they're in the verification record.
- **Judges don't run at `start`**, so a judge criterion has no `started.checks` entry and T13 is quiet for it. Slice 14 adds both. `criterion add --judge` and `criterion set --judge` after `start` don't ask the judge either, and the change fires T5.
- **Pinned trigger lists changed in the test commit.** The six lists of eleven triggers, and `submit --help`'s list, gained T10 in `b8f378c`, before `start`, so each edit is marked "before start". The fake newer-tracker field in three tests was renamed from `ask` to `rubric`, and `verification.judges` to `verification.signature`, since those are real fields now. These edits fired T4 on TR-8, as slice 11's readme said they would, and the user read them at the approve.
- **A lost `started.head` stays fail-closed.** T4 and T14 read `started.head..HEAD`, and `gates`, `submit` and `approve` exit 3 with git's error if git no longer has it. A base that can't be found can't show every commit since start, so the item isn't passed on a guess. TR-44 is the item for a message that names the lost commit and says to reopen and start the item again.
- **An unsure answer holds, it doesn't fail.** G3 fails only a `no` (or the answer `pass_when` doesn't name) and a judge that couldn't be graded. T10 holds an unsure one for a human, and holds every judge-passed item when `judge.auto_verify` is false. T10 can't be switched off.
- **A call that fails stops that criterion's other excerpts.** A command that errors, times out or gives a bad reply would most likely do it again, and each call is slow. Its failure message says so. A failed call never counts as a pass.
- **The command runs in the system's temporary folder, not the project's root.** The judge sees only what's in the request. `timeout_secs` is for each question, so the limit for a request is that times its questions.
- **A judge only reads inside the root.** An input is a file under the project's root once symlinks are followed, as G6 reads a design (§7), and not an absolute path.
- **The default `judge.command`, `tracker-judge`, doesn't exist until slice 13.** A store with a judge criterion and no `judge.command` in its config fails G3 with the shell's "not found". That's honest: there's no real judge to fail open.
- **Two matches on a line are one excerpt**, so a line is judged once. Its ref names the first match's line.
- **The slice added `regex`** (T3 holds it), since `each` is a regex.
- **`criterion set --check` replaces a judge**, and `--judge` replaces a check and manual evidence, as each replaces what the criterion had. A judge's answers are its proof, so evidence is removed, and `evidence` refuses a judge criterion.
- **`approve` checks the status, then the terminal, then asks any judge.** Review 1's bug 1 asked for `require_interactive` at the top of `approve`. It runs right after the status check instead, because two older `tests/review.rs` tests expect the status error first for a piped `approve` of an item not in review, and editing an existing test fires T4. It still runs before any judge or gate, so a piped approve costs no judge call.
- **A forged judge answer is trusted, and §7 says so.** The judge runs in the caller's environment and the cache is read back as written, so a `tracker-judge` on an agent's `PATH`, or a file in `judge/cache/`, plants an answer that every later run, `approve` included, reuses. Review 1's security 1 offered two fixes. Option (b), saying so in §7, is done. Option (a), the program's path in the fingerprint, is slice 13's, since it writes the program and may change the key.
- **An unsure answer's question, reason and excerpt are printed, not stored.** `gates`, `submit` and `approve`'s question print them from the cache's answer and the request they rebuild, and `review` points at `tracker gates` for a hold from an unsure answer. Storing the excerpt on the hold, so `review` and `show` print it, is left for slice 15, which must choose the shape T12 needs.
- **`review`'s hint is for unsure answers only, and §3's `review` row doesn't name it.** The hint is printed when a T10 hold's message says "answered unsure", since `gates` prints a question and excerpt for no other T10 hold. Review 2 offered adding the hint to §3's row. It wasn't, so the hint is code only, and a later change to it needs no DESIGN.md edit.
- **A `diff` judge's hunks are stripped one at a time.** A block comment that opens above a hunk isn't stripped, and one closed below it can blank added lines (review 2's bug 2). Nothing says so in DESIGN.md, since TR-47 will fix it. Until then, a `diff` judge suits only languages with line comments, such as Rust, Python and shell, and no project has one.
- **A `diff` ref names `diff` and a line of the patch text,** not the changed file. TR-45's criterion 2 says its refs "name the file and line", which is wrong for `diff`, and a criterion can't be reworded, so the wording is fixed here instead (review 2's test gap 1). The line is renumbered by every new item commit.
- **TR-46 is one item for three findings,** so the user approves once, and TR-47 holds the one that was left.
- **TR-45 is one item for ten findings**, so the user approves once.

## Fixes after review 1

[Review 1](reviews/slice-12-review-1.md) found that the slice does what the plan asks, with four bugs, one security item, four design questions, three test gaps and a list of points for slice 13. Its fixes were built as one item, TR-45, with its tests committed before `tracker start`: `deb662a` (tests), `5d63075` (the code) and `d356d17` (DESIGN.md). The review's [Outcome](reviews/slice-12-review-1.md#outcome) gives the reason for each decision.

### Tracking TR-45

TR-45 is a `fix`, created at 2026-11-23T04:03:23Z and tracked with the installed slice 11 binary, so T5, T13 and T14 ran on it, and T10 didn't exist. It has 6 criteria, one for each of the item's six written lines, and the orchestrator checked by hand that they read as given and that every commit since `start` is scoped to TR-45.

| Step | When (UTC) | What |
| --- | --- | --- |
| Tests, committed | 04:05:09Z | `deb662a`, 9 seconds before `start` |
| `tracker start TR-45` | 04:05:18Z | Criteria 1 to 4 and 6 failed. Criterion 5 passed |
| `tracker submit TR-45` | 04:13:37Z | The gates passed. Held under T8 and T13 |
| `tracker approve TR-45` | 04:21:05Z | From a terminal, verified `human` at `d356d17`, with `refs/tracker/TR-45` |

- **The two holds.** T8, "the project's design doc changed: docs/DESIGN.md". T13, "criterion 5's check already passed before the work, so it can't be proving it". The user read the diff and approved both.
- **Criterion 5 passed at `start`** because a reply naming another model and a command that exits non-zero already failed G3 (`ask` refuses both). Its tests guard code that worked, so the builder saw them fail by temporarily disabling each refusal in `ask`, then restored the file. The item doesn't record that.
- **No existing test lines changed**, so T4 stayed quiet. The six tests in `tests/judges.rs` and the unit test are additions.

### Fix before slice 13

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Bug 1: `approve` asks the judges before it checks for a terminal | `5d63075`, option (a), with a deviation | `approve` calls `tty::require_interactive` right after its status check. The review put it at the top, but two `tests/review.rs` tests expect the status error first. It still runs before any judge or gate. Option (b), a first pass that reads only the cache, wasn't taken | `tr45_c1::a_piped_approve_is_refused_without_calling_the_judge`, failed at `start` |
| Bug 2: a `\ No newline at end of file` line ends comment stripping for the rest of a `diff` hunk | `5d63075` | `strip_diff` in [src/judge.rs](../src/judge.rs) keeps such a line as it is, inside the hunk, and goes on stripping. The `$#` loss in shell, which the review said could wait, is left as it was | `tr45_c2::a_diff_judge_gets_no_comments_even_after_a_missing_newline_line` and the unit test `a_no_newline_marker_stays_in_its_hunk_and_stripping_goes_on`, both failed at `start` |

### Low severity

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Bug 3: `criterion set --judge` after `start` keeps the old check's `started.checks` entry | `5d63075` | `criterion_judge` in [src/main.rs](../src/main.rs) removes criterion n's entry, so T13 reads it as missing, as it reads a check given in review. Slice 14's judges still need a start record of their own | `tr45_c3::set_judge_after_start_drops_the_started_check_entry`, failed at `start` |
| Bug 4: a huge `context_lines`, `judge.timeout_secs` or `check_timeout_secs` panics | None | Won't fix, as unlikely: only a hand-written absurd value | None |

### Security items

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Security 1: a judge answer forged once is reused by every later run | `d356d17`, option (b) | A §7 bullet, "A forged judge answer is reused from the cache": the judge runs in the caller's environment, and the cache is trusted as written. Option (a), the resolved program in the fingerprint and the reply's `adapter` in `verification.judges`, is left for slice 13 | Criterion 6's `grep -qF 'A forged judge answer is reused from the cache' docs/DESIGN.md`, failed at `start` |

### Design decisions

| Question | Decision | Commits |
| --- | --- | --- |
| Design 1: §2.3.1's step 4 says the `adapter` field goes into the cache key | Fixed: "The tool keeps `adapter` beside the answers in its cache. It isn't part of the key (Cached by content)." | `d356d17` |
| Design 2: an unsure answer is shown without its question or excerpt | Option (a): `gates` and `submit` print the question and excerpt under T10's hold (`unsure_text`), and `review` names `tracker gates <ID>` for a T10 hold. Option (b), storing the excerpt on the hold, is left for slice 15. Review 2 found the reason missing and `approve` without the excerpt: [Fixes after review 2](#fixes-after-review-2) | `5d63075`, test `tr45_c4::submit_and_gates_text_show_the_question_and_excerpt_of_an_unsure_answer` |
| Design 3: `item::Question` refuses unknown fields, so a newer tracker's question field makes an unreadable item, not a newer file | Won't fix, as unlikely: it needs a later slice to add a question field and an older binary to read the file | None |
| Design 4: §2.3.1 doesn't say what `each` does over a `diff` input, or that commit subjects reach the judge | Fixed: one sentence in the comment bullet, which says `each` matches the patch text and counts from its start, and that commit subjects reach the judge unstripped | `d356d17` |

### Test gaps

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Test gap 1: nothing tests a `diff` input | `deb662a` | A `diff` judge over two item commits, one ending a file without a newline, and a unit case for bug 2's hunk | `tr45_c2`, the two tests above |
| Test gap 2: nothing tests `approve` for an item with a judge | `deb662a` | A piped `approve` of a T10 hold with the cache removed. A terminal approve stays a check by hand | `tr45_c1` |
| Test gap 3: the tool's model check and a non-zero exit are untested | `deb662a` | A reply naming another model, and a fake that exits 1 with a message on stderr | `tr45_c5::a_reply_naming_another_model_fails_g3` and `a_judge_command_that_exits_non_zero_fails_g3_with_its_message`, which passed at `start`: the builder disabled each refusal in `ask` and saw each fail |

### Slice 13 must decide

Left for slice 13, as the review wrote them: the cache key and the adapter's version, which model the reply names, the request `tracker-judge` reads, and that TR-9 shouldn't get a judge criterion before `tracker-judge` is on `PATH`. Its fifth point, bug 1 with a real judge, is fixed. [Limits](#limits-and-what-later-slices-change) has them.

### A few consequences

- A piped `approve` of an item not in review still gets the status error, not the terminal one.
- `gates` and `submit` print more for an unsure answer, so an agent's log has the excerpt. `gates --json` is unchanged.
- Slice 12's own demo output changed in one place, `submit`'s T10 hold.

## Fixes after review 2

[Review 2](reviews/slice-12-review-2.md) found that review 1's fixes are in the code and the design, with two bugs, two design questions, one test gap and one process note. Two fixes were incomplete (bug 1 and design 2 of review 1, as design 1 here), and `cargo test` failed at the branch's tip. The fixes for three of them were built as one item, TR-46, with its tests committed before `tracker start`, and the process note was fixed by a docs commit. Each decision's reason is in the review's Outcome section.

### Tracking TR-46

TR-46 is a `fix`, created at 2026-11-23T04:42:56Z with TR-47, and tracked with the installed slice 11 binary, so T5, T13 and T14 ran on it, and T10 didn't exist. Its 3 criteria are its three written lines, and each check runs one module of `tests/judges.rs`.

| Step | When (UTC) | What |
| --- | --- | --- |
| Tests, committed | 04:43:38Z | `84b8c04`, 6 seconds before `start` |
| `tracker start TR-46` | 04:43:44Z | All 3 checks failed |
| The fix, committed | 04:45:49Z | `a59adbc` |
| `tracker submit TR-46` | 04:49:11Z | The gates passed with no hold, so it went to done, verified `auto` at `a59adbc`, with `refs/tracker/TR-46` |

- **No existing test line changed,** so T4 stayed quiet: `tr46_c1` to `tr46_c3` are new modules.
- **TR-47** (`fix`, low priority) was created with TR-46, for bug 2. It's `open`.

### Fix before slice 13

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Bug 1: with two or more unsure answers, no text output shows any reason | `a59adbc` (TR-46) | `unsure_text` in [src/main.rs](../src/main.rs) prints `unsure: <reason>` under each unsure answer's question, escaped as the question is | `tr46_c1::two_unsure_answers_each_show_their_reason_in_gates_and_submit`, failed at `start` |
| Bug 2: a `diff` judge gets the text of a block comment whenever the hunk starts inside it | None | Not fixed now. TR-47 tracks option (a): strip each side of each file whole, and map each hunk line back by its `@@` header. It's to be done before any project gets a `diff` judge. Option (b), documenting it in DESIGN.md, wasn't taken. Nothing is exposed today: no project has a `diff` judge, and TR's languages use line comments | None |

### Design decisions

| Question | Decision | Commits |
| --- | --- | --- |
| Design 1: `approve` asks the user to decide a T10 hold without the question and excerpt it has just read | Option (a): `decision_warning` takes the judges `approve` has just read from the cache and prints `unsure_text` of them. `reject` passes none. Option (b), storing the excerpt on the hold, stays with slice 15 | `a59adbc`, test `tr46_c2::approve_shows_the_question_excerpt_and_reason_of_an_unsure_answer`, which failed at `start` |
| Design 2: `review`'s T10 hint is printed for an `auto_verify: false` hold, where `gates` shows no question | Option (a), the code half: the hint says "shows each unsure answer's question and excerpt", and `review` prints it only when a T10 hold's message says "answered unsure". §3's `review` row is left unchanged, so the hint isn't in the design | `a59adbc`, tests `tr46_c3::review_hints_at_gates_for_an_unsure_answer` and `review_gives_no_hint_for_an_auto_verify_false_hold`, which failed at `start` |

### Test gaps

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Test gap 1: `tr45_c2` checks only that the ref starts `c1:diff:`, while the readme and TR-45's criterion 2 say the refs "name the file and line" | None | Won't fix. A `diff` ref names `diff` and a line of the combined patch text, so this readme's wording is corrected, in [Tests added after review 1](#tests-added-after-review-1) and [Decisions](#decisions-worth-knowing). TR-45's criterion can't be reworded now, and the test isn't changed | None |

### Process

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Process 1: `cargo test` fails at the branch's tip, because review 1's Outcome links to a readme heading that the next commit adds | `e4b9dde`, option (a) | The commit that saved review 2 also added a one-line pointer to review 2 at the end of review 1's Outcome, so the link test reads review 1 at a commit where the readme's heading exists. Option (b), reordering `bfe5220` and `945390f`, would have rewritten hashes this readme names. From now on a readme's update is committed before the Outcome that links into it, and this update was | `docs_links::heading_anchors_name_headings_that_exist`, which passes at `a59adbc` |

### Slice 13 must decide

Left for slice 13 (TR-9), as review 2 wrote them. In short: the test command passes before TR-9 starts (it does, since `e4b9dde`), where TR-9's criterion 4 is checked (`tracker-judge` isn't on `PATH` until the orchestrator installs it, so the by-hand check uses a throwaway store whose `judge.command` names `target/debug/tracker-judge`), `claude` staying in `tracker-judge`'s process group so a timeout stops it, and what `tracker-judge` writes to stderr, which the tool puts whole in one G3 message. Review 1's points are carried in [Limits](#limits-and-what-later-slices-change).

### A few consequences

- `gates` and `submit` print one more line for each unsure answer, so an agent's log has the reason too. `gates --json` is unchanged.
- `approve`'s question is longer for a T10 hold from an unsure answer.
- `review`'s hint is gone for an `auto_verify: false` hold, so its output is shorter.
- Slice 12's demo output changed in one place again, `submit`'s T10 hold.

## Still open

From the build and the checks, and from [review 1](reviews/slice-12-review-1.md) and [review 2](reviews/slice-12-review-2.md), after their fixes.

**Process and bugs:**

- **TR-47** (review 2's bug 2). A `diff` judge's hunks are stripped one at a time, so a block comment opened above a hunk reaches the judge, and one closed below it can blank added lines. Do it before any project gets a `diff` judge.
- **TR-44.** A clearer error for a lost `started.head` (from slice 10's review 2, bug 3, carried by slice 11's reviews). Today it's git's own message, exit 3.
- **`approve` asks the judges before the script gates re-run.** Its first `ask_judges` runs on an empty report, to put T10's reasons in the question, so an uncached judge can be asked while a check would fail. Review 1's bug 1 made a piped `approve` refuse first, so only a human at a terminal reaches it. The second run, after the confirm, is in order. Unchanged work is answered from the cache, so it only costs a call for work that changed since the submit.
- **Huge settings panic** (review 1, bug 4). A `context_lines` of 2^64 - 1 in a judge file, a `judge.timeout_secs` that overflows when multiplied by the question count, and `check_timeout_secs` since slice 7. Won't fix, as unlikely.
- **A shell `$#` loses its tail** in an excerpt, because `strip_comments` reads a `#` outside a string as a comment in the hash style (review 1, bug 2's last sentence).
- **The cache has no pruning,** and a cached answer outlives the adapter that gave it (see [Decisions](#decisions-worth-knowing)).
- **Help text, cosmetic.** `criterion add --help` and `criterion set --help` have usage lines that name `--check` and not `--judge` (`Usage: tracker criterion set --check <CHECK> <ID> <N>`), and a `set` with neither says only that `--check` is required.

**Security:** review 1's security 1, option (a) (the resolved program in the cache key, and `adapter` in the record). §7 says the cache is trusted. Left for slice 13.

**DESIGN.md:** the `review` hint for an unsure answer isn't in §3's `review` row (review 2's design 2). Review 1's design 3 (`item::Question` refuses a newer tracker's question field, so the item is unreadable and not "a newer file"). Won't fix, as unlikely. Design 2's option (b), storing the excerpt for `review` and `show`, is slice 15's, and so is its reasons (review 2's design 1, option (b)).

**Test gaps:** `tr45_c2` doesn't assert the whole `diff` ref (review 2's test gap 1, won't fix), a terminal `approve` of an item with a judge that goes through to done, `show`'s text for a judge and for its answers, `evidence` refused for a judge, a reply that can't be cached, and a file input deleted after `criterion add`.

**Carried from slice 11** ([slice 11's Still open](slice-11-readme.md#still-open)): the T14 and `criteria_hash` test gaps, a second project's key, review 1's test gaps 1 and 5, TR-39 (the misaligned hold lines) and the items carried from slice 10.

## Limits, and what later slices change

- **Slice 13** writes `tracker-judge`, `src/bin/tracker-judge.rs`, which must read the request and reply as [above](#the-judge-commands-contract): `model`, `state` and `questions` in, `adapter`, `model` and `answers` (and `cost_usd`) out, with each question's `criteria` as `true` and `false`. It runs in the system's temporary folder, so it mustn't rely on the project's root. It must report the model the CLI used, which the tool compares with the one it asked for. The cache is keyed by the command and not by the adapter's version, so slice 13 decides whether the resolved program, or a new adapter's version, goes into the fingerprint, which also closes review 1's security 1 (a): a shim on `PATH` mustn't have its answers reused once `PATH` is normal. It should name a model it read from the CLI's `modelUsage`, not echo the request's, or the tool's model check proves nothing. TR-9 shouldn't be given a judge criterion before `tracker-judge` is on `PATH`. `judge-eval/` must send the same request.
- **Slice 14** decides what a `diff` judge sees at `start` as at submit, so TR-47 should be done first. It runs every judge at `start` and records the answers, for T13 to read: `started.checks` holds a script check's result and not a judge's, so it needs a record of its own. G1 refuses a `start` while a judge has nothing to grade or a `diff` judge has no item commit, which `criterion add --judge` only half does today (it skips `diff`). T11 reads `judge.max_failures`, which is already read, and `judge/failures.json`, which doesn't exist yet. The cache's fingerprint is the natural key for "each failed request counts once": it's the record's `request`.
- **Slice 15** adds `submit --dispute <ref>`. A ref is `c<n>:<file>:<line>:<id>`, as `Answered::judge_ref`, and `Failure::judge` has what T12 must show: the excerpt, the answer and the reason. T12 must show the excerpt, and nothing stores one: `unsure_text` rebuilds it from the cache, so slice 15 chooses the stored shape (review 1's design 2, option (b)), and with it whether `approve` and `review` print each reason. A `diff` ref names a line of the patch text that every new item commit renumbers, so slice 15 also decides whether a dispute's ref for a `diff` match names the changed file and its new line instead. It also makes `approve` count held questions as passed for that approval only, which changes `approve`'s second run in `src/main.rs`. T12, like T10, is in `config::ALWAYS_ON`.
- **Slice 16** adds `tracker judges`, which counts approves and rejects for each question. A question's identity is its ref without the file and line, which a removal renumbers, so it must decide how a question is named across items. The digest lists judge-passed items first, with refs, answers and reasons, which are in `verification.judges`.
- **A change to the criterion's fields** changes the fingerprint of every item that's in progress, so T5 fires on each. Add new fields only when set, and install a new binary only between items, as PLAN.md's integration risks say.
- **A new place that asks judges** must go through `ask_judges`, so it runs only after every other gate has passed and reads the same cache. A new place that reads T10 must pass `run` the answers, as `approve` does.
- **A held item pauses its project** until it's decided (§4.4). With T10 on every submit, a judge-graded TR item that gets an unsure answer waits for the user, and with `auto_verify: false` every one does.
- **The helper check depends on `ps`,** as slice 11's readme says, and nothing in this slice changes it.

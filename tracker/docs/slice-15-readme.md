# Slice 15: Disputes

`tracker` is a command-line work tracker, written in Rust, that keeps one central list of work items across many projects and decides when an item is really done. This document covers **slice 15** of the 16 in [PLAN.md](PLAN.md): what it built, how it works, and how to test it. The earlier readmes cover the rest: [slice 1](slice-1-readme.md) the store, `create` and `list`, [slice 2](slice-2-readme.md) the states and `history`, [slice 3](slice-3-readme.md) priority, kind, labels, `edit` and `list`'s filters, [slice 4](slice-4-readme.md) `show` and `delete`, [slice 5](slice-5-readme.md) bad data and polish, [slice 6](slice-6-readme.md) criteria and gates G1, G2 and G6, [slice 7](slice-7-readme.md) running checks, with gates G3 and G4 and `started.checks`, [slice 8](slice-8-readme.md) G5, the `auto` record and `refs/tracker/<ID>`, [slice 9](slice-9-readme.md) the inbox, `approve`, `reject` and triggers T6 and T9, [slice 10](slice-10-readme.md) triggers T1, T2, T3, T4, T7 and T8, [slice 11](slice-11-readme.md) triggers T5, T13 and T14 and the criteria fingerprint, [slice 12](slice-12-readme.md) judge criteria, G3's judge half and trigger T10, with a fake judge command, [slice 13](slice-13-readme.md) `tracker-judge`, the real judge command, and [slice 14](slice-14-readme.md) judges that must fail first, trigger T11 and `judge/failures.json`.

After slice 15 you can **dispute a judge's wrong fail and decide it yourself**. The spec is [DESIGN.md](DESIGN.md). This slice implements [§2.3.2](DESIGN.md#232-when-a-judge-is-wrong) (disputes, and where they live while the item waits), trigger T12 from [§2.5](DESIGN.md#25-review-triggers-does-it-need-a-human) and what its T11 row says a dispute costs, what `approve` and `reject` record in [§2.7](DESIGN.md#27-human-actions), `held_questions` from [§4.3](DESIGN.md#43-item-file), and `submit --dispute` in [§3](DESIGN.md#3-interface). It also changes [§7](DESIGN.md#7-honest-limits), [§6](DESIGN.md#6-what-im-trusting-and-what-checks-it) and [§5](DESIGN.md#5-working-with-agents) to say what a T11 run can and can't bound, as [slice 14's review 2](reviews/slice-14-review-2.md) asked. The slice is tracked in `tracker` itself as item TR-11, with the binary installed from slice 14 ([Tracking TR-11](#tracking-tr-11)). [Review 1](reviews/slice-15-review-1.md) found six things to fix and a few to leave. The fixes are in, as item TR-53 ([Fixes after review 1](#fixes-after-review-1)). [Review 2](reviews/slice-15-review-2.md) found TR-53's fixes right and complete, with one bug, three design points and two test gaps. The bug is fixed as item TR-54, and two design points went to TR-12 ([Fixes after review 2](#fixes-after-review-2)).

| | |
| --- | --- |
| Status | Every automated _Done when_ check has a test, and every test passes, as does the test command (`cargo fmt --check`, `cargo clippy -- -D warnings` and `cargo test` were run on the finished branch, at `2a9989d`, the last commit to change code, after review 2's fix. They also passed at `7e32521`, after review 1's). TR-11 is `done`, but it wasn't auto-verified. It has eight criteria: criteria 1 to 5 are PLAN.md's _Done when_ bullets, as written (the fifth joins two), and 6 to 8 are DESIGN.md wording the orchestrator added before `start` from [slice 14's review 2](reviews/slice-14-review-2.md) (security 1 and 2, design 1 and design 2). All eight checks failed at `start`. Criteria 1 to 5 run `cargo test --test disputes` by prefix, and 6 to 8 are `grep` checks of DESIGN.md. The installed slice 14 binary held the submit under T1 (it's a capability), T7 (1218 lines changed) and T8 (DESIGN.md changed). T4 stayed quiet: the only existing tests changed are the six pinned trigger lists, by additions, before `start`. The user read the diff and approved it from a terminal: `verification.by` is `human`, at `4dce835`, with `refs/tracker/TR-11`, and the three holds are recorded in `verification.triggers`. Before the approve, the orchestrator checked by hand that the five written criteria are unchanged, that all eight checks failed at `start`, and that every commit since `start` is scoped to TR-11. **Not checked by hand:** approving a T12 hold in a real terminal. The tests do it under `script` in a throwaway store, and the user's approve of TR-11 was of a T1, T7 and T8 hold ([Still open](#still-open)). One settled point still has no test: that a T11 hold records no decision for any question ([Testing](#testing)). **Review 1's fixes are done, as TR-53**, a `fix` item with 6 criteria, all of which failed at `start`. It was held under T4, T7 and T8, and the user read the diff and approved it from a terminal: `verification.by` is `human`, at `7e32521`, with `refs/tracker/TR-53` ([Fixes after review 1](#fixes-after-review-1)). The fixes left three things open on purpose: review 1's security 1 option (b), two test gaps, and what slice 16 must decide ([Still open](#still-open)). **Review 2's fix is done, as TR-54**, a `fix` item with 2 criteria, both of which failed at `start`. It was verified `auto` at `2a9989d`, with `refs/tracker/TR-54` and no holds, under the installed slice 14 binary ([Fixes after review 2](#fixes-after-review-2)). Review 2's design 1 and design 3 are folded into TR-12 as its criteria 8 and 9, and its design 2 and two test gaps are left on purpose ([Still open](#still-open)). PLAN.md has slice 15 as `in review`. The branch isn't merged into `main`, and neither are slices 7 to 14, which it's stacked on |
| Branch | `feat/tracker/slice-15`, stacked on `feat/tracker/slice-14` at `ebbc62f`. Slice 14 isn't merged into `main`, so this slice's commits are `git log --oneline --reverse feat/tracker/slice-14..feat/tracker/slice-15`. In order, all scoped to TR-11: `5e16893` (the tests, committed before `tracker start TR-11`), `d4a4eb2` (PLAN.md: slice 15 in progress), `f2f40ca` (the code), `f5667e4` (two fixtures, XX-25 and XX-26), `db355e4` (DESIGN.md) and `4dce835` (PLAN.md: slice 15 in review). That is 6 commits, all TR-11's. After the review, TR-53's four commits follow: `109b32f` (tests, before `start`), `aff07dc` (a test fixed after `start`), `b09c5e6` (the code) and `7e32521` (DESIGN.md and a schema). After review 2, TR-54's two commits follow: `7752965` (tests, before `start`) and `2a9989d` (the code). The commits that add and update this readme, and the reviews', belong to no item |
| Test command | `cargo fmt --check && cargo clippy -- -D warnings && cargo test`, run in `tracker/`. All three pass at `2a9989d`, the last commit to change code. Later commits change docs only |
| Tests | 537: 46 in `tests/risky_changes.rs`, 42 in `tests/judges.rs`, 39 in `tests/create_and_list.rs`, 36 in `tests/commits.rs`, 34 in `tests/organise.rs`, 29 in `tests/judges_first.rs`, 29 in `tests/disputes.rs`, 28 in `tests/review.rs`, 27 in `tests/states.rs`, 25 in `tests/show_and_delete.rs`, 23 in `tests/criteria.rs`, 22 in `tests/gates.rs`, 21 in `tests/tampered_proof.rs`, 18 in `tests/checks.rs`, 16 in `tests/fixtures.rs`, 14 in `tests/item_files.rs`, 13 in `tests/bad_data.rs`, 12 in `tests/tracker_judge.rs`, 11 in `tests/docs_links.rs`, 9 in `tests/polish.rs`, 6 in `tests/output.rs`, 6 in `tests/t5_and_fingerprint.rs`, 5 in `tests/schemas.rs`, 4 in `tests/store_writes.rs`, 3 in `tests/refusal_wording.rs`, 2 in `tests/item_ids.rs`, 1 each in `tests/harness_check.rs`, `tests/judge_add_message.rs` and `tests/submit_help.rs`, 12 unit tests in the library (4 in `src/gates.rs`, 4 in `src/judge.rs`, 2 in `src/git.rs`, 1 each in `src/store.rs` and `src/checks.rs`) and 2 in `src/main.rs`. That is 33 more than slice 14's 504: 29 in the new `tests/disputes.rs` (11 of them TR-53's, after review 1), 1 in `tests/fixtures.rs` and 3 in the new `tests/refusal_wording.rs` (TR-54's, after review 2). Before review 2's fix the total was 534, and before review 1's fixes 523, with 18 in `tests/disputes.rs` |

## Contents

- [What's in this slice](#whats-in-this-slice)
- [Try it](#try-it)
- [Commands](#commands)
- [What held_questions holds](#what-held_questions-holds)
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

- **`submit --dispute <ref> --reason "…"`, repeatable.** Each `--dispute` is paired with the `--reason` at the same place, and `--reason`'s help says so. `submit` runs every gate as usual. If the only failures are the disputed questions, the item goes to review, held under T12. If anything else fails, the submit is refused, as before.
- **Exit 1 for a ref the judge didn't fail.** A ref the judge passed, answered unsure, or never asked is refused with exit 1, naming the ref and why, and changes nothing in the item. The refusal counts its run toward T11, since the judges were asked and it tells the agent their answer. If a judge call failed and left no answer, the submit is refused for G3 with exit 2 instead.
- **T12.** It fires when the submit disputed at least one fail, and sits between T11 and T13. T12 is in `config::ALWAYS_ON` with T6 and T10, so `review.disabled` can't switch it off. `approve` runs T12 too, on the questions the item holds.
- **`held_questions` on the item.** While the item is in review, each judge question that holds it is kept: a disputed fail (T12) with the agent's reason, and an unsure answer (T10), or, when the project's `auto_verify` is false, every answer a judge didn't fail (T10). Each carries the question's text, `ask`. It's kept without the excerpt ([What held_questions holds](#what-held_questions-holds)).
- **`review`, `show`, `approve` and `reject` show it.** `review` and `show` print each disputed question's ref, the model's answer and reason, and the agent's reason. The held submit and `approve`'s question print the excerpt too, since both have just asked the judge.
- **`approve` counts a disputed fail as passed, for that approval only.** It matches the answer by the question's ref and its request fingerprint, so a change made while the item waited, which gets a new answer, still refuses the approve with G3.
- **`approve` and `reject` record the decision against each held question.** The history entry of each keeps the questions in its own `held_questions`, and an approve copies the submit's questions to `verification.held_questions`, as a reject copies them to its entry. Slice 16's `judges` counts from these.
- **DESIGN.md says it.** §2.3.2 says where a dispute lives and what a refusal costs, §2.5's T11 row says what a dispute costs and to fix every failing `each` match before running `gates` again, §2.7 and §4.3 describe the records, §5 repeats the `each` advice, and §6 and §7 say what a T11 run can't bound.
- **Slice 14's "Slice 15 must decide" points**, from [review 1](reviews/slice-14-review-1.md#slice-15-must-decide) and [review 2](reviews/slice-14-review-2.md#slice-15-must-decide): see [Decisions](#decisions-worth-knowing).

**Not built yet:**

- **`tracker judges`, the digest's dispute entries, and `audit`'s look at `held_questions` and `judge/failures.json`** (slice 16). Nothing reads the recorded decisions yet.
- **Saved rulings** (future work, [§7](DESIGN.md#7-honest-limits)). A decision covers one submit, so a resubmit of unchanged code after a reject needs the same dispute again.
- **TR-47, TR-48 and TR-50** (a `diff` judge's block comments, what else a real judge sees, and escaping `</material>`). All three are deferred ([Still open](#still-open)).

## Try it

Always set `TRACKER_HOME` while developing. Without it, the tool uses your real store at `~/.tracker`.

**This demo never calls the real `claude` or `tracker-judge`.** Its judge command is a shell script that reads the request and prints whatever is in `reply.json`, as `tests/disputes.rs` does, and a small `reply` script writes that file from `id=answer` pairs, so the demo sets each answer. The project BM is a folder in one throwaway repository, as TR is a folder in guild-portfolio. BM-1 has a judge criterion with two questions, `works` and `clear`. The judge answers each version of the code, and the demo disputes what it got wrong:

- **Version 1:** the judge fails `works` and passes `clear`. Three disputes are refused with exit 1: one with no reason, one of the question that passed and one of a question nobody asked. The first asks no judge and counts nothing. The other two ask the judges, so the first of them counts the run toward T11, and the second repeats it.
- **Version 2:** `works` and `clear` both fail. Disputing only `works` is refused with exit 2 for `clear`, and that run counts toward T11.
- **Version 3:** both fails are disputed, and BM-1 is held under T12. `review` and `show` print the questions, `approve` and `reject` are refused without a terminal, and the item file keeps `held_questions`, each with its `ask`. The project's `max_failures` is 5, so the three counted runs don't hold it under T11 too. Last, `submit --help` shows what `--reason` says about pairing.

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
cat > $DEMO/judge/reply <<'EOF'
#!/bin/sh
out="$(dirname "$0")/reply.json"
printf '{"adapter":"fake 1","model":"claude-haiku-5-5","answers":{' > "$out"
sep=''
for pair in "$@"; do
  id=${pair%%=*}; answer=${pair#*=}
  printf '%s"%s":{"answer":"%s","reason":"The fake says %s to %s."}' "$sep" "$id" "$answer" "$answer" "$id" >> "$out"
  sep=,
done
printf '}}' >> "$out"
EOF
chmod +x $DEMO/judge/reply
export PATH=$DEMO/judge:$PATH
printf '%s\n' '{"text":"The code does the work, plainly","inputs":["src/lib.rs"],"ask":[{"id":"works","ask":"Does this code do the work?"},{"id":"clear","ask":"Is this code clear?"}]}' > $DEMO/judge/lib.json
cd $DEMO/repo && git init --quiet -b main
mkdir -p bm/src && printf 'fn run() {\n    todo!()\n}\n' > bm/src/lib.rs
git add -A && git commit --quiet -m "chore: set up"
tracker init
jq --arg c "$DEMO/judge/fake" '.judge = {command: $c, model: "claude-haiku-5-5"}' $TRACKER_HOME/config.json > $DEMO/config.new && mv $DEMO/config.new $TRACKER_HOME/config.json
tracker project add BM --name bm --root $DEMO/repo/bm
jq '.projects.BM.judge = {max_failures: 5}' $TRACKER_HOME/config.json > $DEMO/config.new && mv $DEMO/config.new $TRACKER_HOME/config.json
reply works=no clear=no
```

The demo itself, from the same shell. It was run line by line, as one script, with stdout and stderr together. A `$ ` line is the command, and `[exit N]` follows each one that fails:

```sh
tracker create "Say hello" --project BM
tracker criterion add BM-1 --judge $DEMO/judge/lib.json
cd $DEMO/repo
tracker start BM-1
printf 'fn run() {\n    step(1);\n}\n' > bm/src/lib.rs
git add -A && git commit --quiet -m "feat(BM-1): version 1"
reply works=no clear=yes
tracker submit BM-1 --dispute c1:works      # exit 1: a dispute needs its own reason
tracker submit BM-1 --dispute c1:clear --reason "It is clear."      # exit 1: the judge passed it
tracker submit BM-1 --dispute c9:nope --reason "No such question."      # exit 1: no such question was asked
jq '.["BM-1"] | {at_start: (.at_start | length), failed: (.failed // [] | length)}' $TRACKER_HOME/judge/failures.json
printf 'fn run() {\n    step(2);\n}\n' > bm/src/lib.rs
git add -A && git commit --quiet -m "feat(BM-1): version 2"
reply works=no clear=no
tracker submit BM-1 --dispute c1:works --reason "step calls the helper that does the work, at lib.rs:2."      # exit 2: c1:clear still fails
jq '.["BM-1"] | {at_start: (.at_start | length), failed: (.failed // [] | length)}' $TRACKER_HOME/judge/failures.json
printf 'fn run() {\n    step(3);\n}\n' > bm/src/lib.rs
git add -A && git commit --quiet -m "feat(BM-1): version 3"
tracker submit BM-1 --dispute c1:works --reason "step calls the helper that does the work, at lib.rs:2." --dispute c1:clear --reason "The body is one call."
jq '.["BM-1"] | {at_start: (.at_start | length), failed: (.failed // [] | length)}' $TRACKER_HOME/judge/failures.json
tracker review
tracker show BM-1 | sed -n '/^Held for review/,$p'
echo BM-1 | tracker approve BM-1      # exit 1: approve needs a terminal
echo BM-1 | tracker reject BM-1 --reason "Not yet."      # exit 1: so does reject
jq '{status, holds, held_questions}' $TRACKER_HOME/items/BM-1.json
tracker list --all --status in-progress,review,done
tracker submit --help | sed -n '/^      --reason/,/^$/p'
```

The output:

```
$ tracker create "Say hello" --project BM
Created BM-1: Say hello
$ tracker criterion add BM-1 --judge $DEMO/judge/lib.json
Added criterion 1 to BM-1, a judge: The code does the work, plainly
Its 2 questions are asked at start, and at submit once every other gate passes.
$ cd $DEMO/repo
$ tracker start BM-1
Asking criterion 1's judge 2 questions: `$DEMO/judge/fake`
BM-1: open → in-progress
$ printf 'fn run() {\n    step(1);\n}\n' > bm/src/lib.rs
$ git add -A && git commit --quiet -m "feat(BM-1): version 1"
$ reply works=no clear=yes
$ tracker submit BM-1 --dispute c1:works      # exit 1: a dispute needs its own reason
error: Each --dispute needs its own --reason, and BM-1 has 1 --dispute and 0 --reason. Give them in pairs: `tracker submit BM-1 --dispute <ref> --reason "…"`.
[exit 1]
$ tracker submit BM-1 --dispute c1:clear --reason "It is clear."      # exit 1: the judge passed it
Asking criterion 1's judge 2 questions: `$DEMO/judge/fake`
error: BM-1 can't be submitted with that dispute: the judge passed c1:clear, so there's nothing to dispute. Only a judge's fail can be disputed. BM-1 wasn't changed.
[exit 1]
$ tracker submit BM-1 --dispute c9:nope --reason "No such question."      # exit 1: no such question was asked
Criterion 1's judge: answered from the cache
error: BM-1 can't be submitted with that dispute: no judge question c9:nope was asked. `tracker gates BM-1` lists each question's ref. Only a judge's fail can be disputed. BM-1 wasn't changed.
[exit 1]
$ jq '.["BM-1"] | {at_start: (.at_start | length), failed: (.failed // [] | length)}' $TRACKER_HOME/judge/failures.json
{
  "at_start": 1,
  "failed": 1
}
$ printf 'fn run() {\n    step(2);\n}\n' > bm/src/lib.rs
$ git add -A && git commit --quiet -m "feat(BM-1): version 2"
$ reply works=no clear=no
$ tracker submit BM-1 --dispute c1:works --reason "step calls the helper that does the work, at lib.rs:2."      # exit 2: c1:clear still fails
Asking criterion 1's judge 2 questions: `$DEMO/judge/fake`
error: BM-1 can't be submitted: G3 failed. BM-1 wasn't changed.
  G3  Criterion 1's judge answered no to c1:clear, "Is this code clear?", and only yes passes: The fake says no to clear. Its excerpt follows.
        [src/lib.rs]
        fn run() {
            step(2);
        }
[exit 2]
$ jq '.["BM-1"] | {at_start: (.at_start | length), failed: (.failed // [] | length)}' $TRACKER_HOME/judge/failures.json
{
  "at_start": 1,
  "failed": 2
}
$ printf 'fn run() {\n    step(3);\n}\n' > bm/src/lib.rs
$ git add -A && git commit --quiet -m "feat(BM-1): version 3"
$ tracker submit BM-1 --dispute c1:works --reason "step calls the helper that does the work, at lib.rs:2." --dispute c1:clear --reason "The body is one call."
Asking criterion 1's judge 2 questions: `$DEMO/judge/fake`
BM-1: in-progress → review
Every gate passed but the disputed judge questions, and BM-1 is held for review:
  T12  the agent disputed judges' fails to 2 questions, so a human decides: c1:works and c1:clear
    c1:works: Does this code do the work?
      model answered no: The fake says no to works.
      agent disputes: step calls the helper that does the work, at lib.rs:2.
      [src/lib.rs]
      fn run() {
          step(3);
      }
    c1:clear: Is this code clear?
      model answered no: The fake says no to clear.
      agent disputes: The body is one call.
      [src/lib.rs]
      fn run() {
          step(3);
      }
It waits in `tracker review` for a human to approve or reject it. `tracker show BM-1 --diff` shows its work.
$ jq '.["BM-1"] | {at_start: (.at_start | length), failed: (.failed // [] | length)}' $TRACKER_HOME/judge/failures.json
{
  "at_start": 1,
  "failed": 3
}
$ tracker review
1 item is held for review, waiting for you to approve or reject each one:

BM-1  Say hello
  T12  the agent disputed judges' fails to 2 questions, so a human decides: c1:works and c1:clear
    c1:works
      model answered no: The fake says no to works.
      agent disputes: step calls the helper that does the work, at lib.rs:2.
    c1:clear
      model answered no: The fake says no to clear.
      agent disputes: The body is one call.
  `tracker gates BM-1` shows each disputed question and excerpt, as a G3 failure.
  See its work with `tracker show BM-1 --diff`, then run `tracker approve BM-1` or `tracker reject BM-1 --reason "…"`.
$ tracker show BM-1 | sed -n '/^Held for review/,$p'
Held for review by:
  T12  the agent disputed judges' fails to 2 questions, so a human decides: c1:works and c1:clear
    c1:works
      model answered no: The fake says no to works.
      agent disputes: step calls the helper that does the work, at lib.rs:2.
    c1:clear
      model answered no: The fake says no to clear.
      agent disputes: The body is one call.
$ echo BM-1 | tracker approve BM-1      # exit 1: approve needs a terminal
error: `tracker approve BM-1` needs a human at an interactive terminal, who types BM-1 to confirm it, and stdin here isn't a terminal. Nothing was changed. Only a human approves a held item. It waits in `tracker review` until one approves or rejects it.
[exit 1]
$ echo BM-1 | tracker reject BM-1 --reason "Not yet."      # exit 1: so does reject
error: `tracker reject BM-1` needs a human at an interactive terminal, who types BM-1 to confirm it, and stdin here isn't a terminal. Nothing was changed. Only a human rejects a held item. It waits in `tracker review` until one approves or rejects it.
[exit 1]
$ jq '{status, holds, held_questions}' $TRACKER_HOME/items/BM-1.json
{
  "status": "review",
  "holds": [
    {
      "trigger": "T12",
      "message": "the agent disputed judges' fails to 2 questions, so a human decides: c1:works and c1:clear"
    }
  ],
  "held_questions": [
    {
      "trigger": "T12",
      "ref": "c1:works",
      "ask": "Does this code do the work?",
      "request": "a0ffcd2e6d765b630065e70ab1b813fec2a3b92091ca8c715d5008adce9ddddd",
      "answer": "no",
      "reason": "The fake says no to works.",
      "dispute": "step calls the helper that does the work, at lib.rs:2."
    },
    {
      "trigger": "T12",
      "ref": "c1:clear",
      "ask": "Is this code clear?",
      "request": "a0ffcd2e6d765b630065e70ab1b813fec2a3b92091ca8c715d5008adce9ddddd",
      "answer": "no",
      "reason": "The fake says no to clear.",
      "dispute": "The body is one call."
    }
  ]
}
$ tracker list --all --status in-progress,review,done
  ID    PRIORITY  KIND    STATUS  TITLE
BM  bm
  BM-1  medium    change  review  Say hello

Every project.
Filters: status in-progress, review or done.
$ tracker submit --help | sed -n '/^      --reason/,/^$/p'
      --reason <REASON>
          Why the judge is wrong: one line of at most 500 characters, with evidence such as a file:line. Reasons pair with disputes by position: the first --reason goes with the first --dispute, and so on
```

Notes on the output:

- **The request fingerprint** in `held_questions` differs only if the code, the questions or the model do. Both questions share one request, since a criterion without `each` sends one request holding every question, so both entries have the same `request`.
- **"BM-1 wasn't changed."** ends a refusal that has counted a T11 run, so the two exit 1 refusals that ask the judges and version 2's exit 2 refusal say it, where a refusal that counts nothing (a missing reason, an earlier gate failing) still says "Nothing was changed." The item file is unchanged either way. The message doesn't show the count, so an agent isn't told how many tries are left ([Fixes after review 2](#fixes-after-review-2)).
- **The three exit 1 refusals** of version 1 are different checks. The missing reason is caught before anything is read, so no judge is asked. The other two are caught after the judge has answered, from its answers, so the judge is asked once, then answered from the cache. A refused dispute counts its run, so `failed` is 1 afterwards, from version 1's fail of `works`, and the third refusal repeats a counted request and adds nothing. `at_start` is the request asked at `start`, which never counts.
- **Version 2's run counts** (`failed` is 2): it fails a request not seen before, and a submit that G3 refuses counts. The held submit of version 3 counts too, though its fails were disputed, so `failed` is 3 at the end. That would hold the item under T11 at the default `max_failures` of 3, which is why the demo sets 5 ([Decisions](#decisions-worth-knowing)).
- **The held output** says "Every gate passed but the disputed judge questions", since the disputed fails are still fails: only the others passed. The model's answer and reason, the agent's reason and the excerpt come from the run that held it. `review` and `show` read the item, which keeps no excerpt, so they print the two reasons only, and `review` points at `tracker gates BM-1` for the excerpt.
- **T12's message** names the refs. For one disputed question it reads "the agent disputed a judge's fail, so a human decides: c1:works answered no".
- **`--reason`'s help** now says the first `--reason` goes with the first `--dispute`, and so on. The code always paired them by position, and it was the help that was wrong ([Fixes after review 1](#fixes-after-review-1)).
- **`approve` and `reject` can't be shown going ahead**, since both need a terminal. Typed at one, `approve BM-1` would print what holds BM-1 and each disputed question with its excerpt, ask for the ID, re-run the gates counting each disputed fail as passed, and print "BM-1: review → done". The item would then have `verification.by` of `human`, `verification.held_questions` with the two questions the submit held, and the same two in its `approve` history entry. `reject` would put them in its history entry only. History would say `"interactive": true`.
- Nothing in the demo shows a `--json` output. [JSON output](#json-output) says what changed.

## Commands

One flag is new, and three commands show more:

| Command | What changes |
| --- | --- |
| `submit <ID> --dispute <REF> --reason <REASON>` | Disputes a judge's fail by its ref, as the gate output prints it, such as `c2:src/import.js:57:suggests_next_step`. Repeat the pair for each question. If the disputed questions are the only failures, the item goes to review, held under T12, with `held_questions` set. Anything else failing refuses the submit with exit 2, listing the undisputed failures. A ref the judge didn't fail exits 1, and counts its run toward T11. A judge call that failed leaves no answer to dispute, so the submit is refused for it with exit 2. Reasons pair with disputes by position, as `--reason`'s help says |
| `approve <ID>` | Runs T12 on the item's disputed questions. The gates re-run counts each disputed fail as passed, for the answer it disputed. The warning it prints before the question lists each disputed question with its excerpt. The record keeps the questions the submit held, and the history entry too |
| `reject <ID> --reason "…"` | Records the held questions in its history entry, then clears them from the item |
| `review` | Prints each disputed question's ref, the model's answer and reason, and the agent's reason under its hold, and points at `tracker gates <ID>` for the excerpt |
| `show <ID>` | Prints the same under "Held for review" while the item waits, and under the verification record once it's done |
| `submit --help` | Names T12 and `--dispute`, and its usage line shows the pair |

A refusal that has counted a T11 run ends "<ID> wasn't changed." and not "Nothing was changed." This covers a dispute refused with exit 1 after the judges were asked and a submit refused because a judge's answer wasn't a pass. `gates <ID>` takes no `--dispute`, so a dry run shows a disputed fail as an ordinary G3 failure with its excerpt, and doesn't change the item. The exit codes are as in [§3](DESIGN.md#3-interface): a bad dispute exits 1 (a ref the judge didn't fail, a `--dispute` without its `--reason` or the other way round, a ref disputed twice, or a missing or blank reason), and a submit that another gate, an undisputed question or a failed judge call refuses exits 2.

The input rules for `--reason` are the ones for a `reject` reason: one printable line, trimmed, at most 500 characters ([`check_reason`](../src/main.rs)). The pairing, the duplicates and the reasons are checked before the store is opened, so a malformed dispute exits 1 whatever the item's state. A ref is trimmed and compared whole, so `c2:src/import.js:57:suggests_next_step` must be typed as printed.

## What held_questions holds

[§4.3](DESIGN.md#43-item-file) lists the field. It's on the item while the item waits in review, and it isn't in the file otherwise: `approve` and `reject` empty it. Each entry has these fields, in this order, and [item.schema.json](../schemas/item.schema.json)'s `held_question` is the schema:

| Field | What it holds |
| --- | --- |
| `trigger` | `T12` for a fail the agent disputed, or `T10` for an unsure answer, or, when the project's `auto_verify` is false, for any answer a judge didn't fail |
| `ref` | The question's ref, such as `c1:works` |
| `ask` | The question's text, as the judge was asked it. It's what slice 16's `judges` groups a question by, since the ref's criterion number and `file:line` change. Left out when empty: a question held before TR-53 has none |
| `request` | The fingerprint of the request the answer came from. `approve` counts a disputed fail as passed only for this request |
| `answer` | The model's answer: `yes`, `no` or `unsure` |
| `reason` | The model's one-sentence reason. Empty if the judge gave none |
| `dispute` | The agent's reason, one line of at most 500 characters. Only for T12 |

The same shape appears in three more places, all through the one `held_question` definition:

- **`verification.held_questions`**, on a `human` record, once `approve` ran: the questions the submit held, copied as they were, so they are the same ones `reject` would have recorded.
- **Every `approve` and `reject` history entry's `held_questions`**, which is what `tracker judges` will count. A reopen moves the record to `previous_verification` but leaves history alone, so the decision lasts.
- **`review --json`'s `held[].held_questions`**, left out when the item has none.

The excerpt isn't stored: it can be up to `max_input_bytes`, and the held submit, `approve`'s warning and `gates` each print it from the request they rebuild. `review` and `show` print a question's two reasons and not its excerpt for that reason. An item held for any other reason, such as T6, has no `held_questions`. A T10 hold does too, from an unsure answer or from `auto_verify: false`, with `dispute` left out.

## How it works

### Checking a dispute

`submit` in [src/main.rs](../src/main.rs) turns the flags into pairs, then `apply` runs the gates as before. Before the judges run, `apply` notes whether every earlier gate passed (`asked`). A dispute can only be checked against answers, so if an earlier gate failed and the judges weren't asked, the submit is refused for that gate with exit 2 and the disputes are ignored. If the judges were asked, `check_disputes` looks each ref up in `gates.judges`. A fail passes. A pass, an unsure answer or a ref that nobody asked is refused with exit 1, after the lock is taken again and after `count_failures` has counted the run, since the judges were asked ([T11 and a dispute](#t11-and-a-dispute)). The check also needs every judge to have answered. `apply` looks for a G3 failure with no answer (`unasked`, a failed call), and if there is one it skips `check_disputes` and refuses for that failure with exit 2, as for an earlier gate. Otherwise a failed call would read as "no judge question … was asked", and point the agent at `gates` for a ref that was right.

### Taking the disputed fails out

`count_failures` has already counted the run toward T11 ([slice 14](slice-14-readme.md#counting-failed-runs)), before the gates are judged. Then `take_disputed` removes each disputed question's G3 failure from the report and returns it as a `HeldQuestion` for T12, with the agent's reason, in the order the agent disputed them. If the report still has a failure, `apply` refuses as usual, so a disputed question never hides another gate's failure, and an undisputed question of the same judge is listed. If it's empty, the triggers run with `disputed` as a new argument, and T12 fires when it isn't empty.

### Holding the item

When any trigger fires, the item goes to review. `apply` sets `holds` as before, and now `held_questions` too: the disputed questions, then the T10 questions (`t10_questions`): each unsure answer, and, when the project's `auto_verify` is false, every answer a judge didn't fail, since T10 then holds the item for all of them. `held_question` fills each one's `ask` from the question's text. The output starts "Every gate passed but the disputed judge questions" when any question is T12, and `disputed_text` prints each question after the holds. `held_questions` is set only while the item is in review, and a `reject` or an `approve` clears it with `holds`.

### Approving

`approve` in [src/main.rs](../src/main.rs) reads `held_questions`, takes the T12 ones with `disputed_questions`, and runs the triggers with them, so T12 is among the holds it shows and records. `decision_warning` prints each disputed question with its excerpt, from the cache's answer, before the question is asked. After the ID is confirmed it re-runs the gates and the judges as before, and removes each G3 failure whose ref and request match a disputed question, so the disputed answer counts as passed for this approval and nothing else. A question's answer to new work has a new request and isn't removed, so the approve is refused with G3 and the item stays in review. On success `approve` puts the item's `held_questions`, the ones the submit stored, in `verification.held_questions`, and `record_decision` copies them into the new `approve` history entry. It doesn't build them again from its own re-ask, so `approve` and `reject` record the same questions.

### Rejecting

`reject` takes `held_questions` out of the item with `std::mem::take`, clears `holds` as before, and `record_decision` puts the questions in the `reject` history entry. Nothing carries the dispute over: a resubmit of unchanged code gets the cache's same answer, a fail, and is refused unless it disputes again.

### T11 and a dispute

`count_failures` runs before `check_disputes` and `take_disputed`, so a `gates` or `submit` run that fails a new request counts once whether or not the fail is then disputed, and whether or not the dispute is then refused. An agent normally saw the fail in `gates` first, which counted the run, and the `submit --dispute` that follows repeats a counted request and adds nothing. A refusal with exit 1 counts too, since the judges were asked and the refusal says which way the disputed question went: otherwise an agent could try wording after wording and learn each answer from a refusal that counts nothing ([review 1](reviews/slice-15-review-1.md#security), security 1). This reverses [slice 14's review 2](reviews/slice-14-review-2.md)'s advice to count nothing on it. A submit that stops for an earlier gate counts nothing, since the judges weren't asked, and a failed call counts nothing, as it never did. T11's unit is still runs, and its code didn't change. Both refusals end "<ID> wasn't changed.", since "Nothing was changed." read as a free try ([review 2](reviews/slice-15-review-2.md#bugs), bug 1).

## JSON output

No command or flag has a new output shape, but three shapes gained one optional field. Every output that holds an item gets it through [item.schema.json](../schemas/item.schema.json):

| Output | What's new | Schema |
| --- | --- | --- |
| Any item, such as `show --json`'s `item` | `held_questions`, while it's in review, at least one entry, each with its `ask` when it has one. Left out otherwise | [item.schema.json](../schemas/item.schema.json) `held_questions` and `$defs.held_question` |
| An item's `verification` | `held_questions` for a `human` record, the questions approved. Left out for an `auto` one | [item.schema.json](../schemas/item.schema.json) `verification` |
| An item's `history` entry | `held_questions` on an `approve` or a `reject` that decided some | [item.schema.json](../schemas/item.schema.json) `history` |
| `review --json` | `held[].held_questions`, left out when the item has none, before `diff` | [review.schema.json](../schemas/review.schema.json) |

`submit --json`'s held shape is unchanged: T12 is one more entry in `triggers` and `holds`, with `fired` true, and the questions aren't in it. Read them from the item. A held submit lists every gate that ran as passed, even when it was held for a disputed fail: a disputed fail leaves G3 `ok`, with the fail in `judges` and in `item.held_questions` ([§3](DESIGN.md#3-interface), and [submit.schema.json](../schemas/submit.schema.json)'s description). Files written by earlier slices still load and come back byte for byte, since each new field has a serde default and is left out when empty. [tests/fixtures/slice-15/](../tests/fixtures/slice-15/) has two files that use the fields.

## Code tour

| File | What's new or changed |
| --- | --- |
| [src/item.rs](../src/item.rs) | `HeldQuestion` (the shape above, `judge_ref` serialised as `ref`, and `ask`, a serde default left out when empty), and a `held_questions` field on `Item`, `Verification` and `HistoryEntry`, each with a serde default and left out when empty |
| [src/triggers.rs](../src/triggers.rs) | `t12` (one message for one question, another for several), T12 in `run` (a new `disputed` argument, between T11 and T13) |
| [src/main.rs](../src/main.rs) | `submit` (the pairs and their input rules, and `--reason`'s help), `check_disputes` (its refusal ends "<ID> wasn't changed." since TR-54), `take_disputed`, `t10_questions` (it was `unsure_questions` before TR-53), `held_question`, `disputed_questions`, `disputed_text`, `record_decision`, and changes to `apply` (the `disputes` argument, `asked`, `unasked`, the order of `count_failures` and `check_disputes`, the held output), `approve`, `reject`, `decision_warning`, `show`, `verification_text`, `review` (text and JSON), `verification_record` and the `Submit` clap definition with its help text |
| [src/gates.rs](../src/gates.rs) | `Report::refusal` says "<ID> wasn't changed." for a submit refused while a judge answered something other than a pass, and keeps "Nothing was changed." otherwise (TR-54) |
| [schemas/item.schema.json](../schemas/item.schema.json) | `held_questions` on the item, the verification record and a history entry, and `$defs.held_question`, with `ask` |
| [schemas/submit.schema.json](../schemas/submit.schema.json) | Its description says a disputed fail leaves G3 `ok` (TR-53) |
| [schemas/review.schema.json](../schemas/review.schema.json) | `held_questions` on each held item |
| [tests/disputes.rs](../tests/disputes.rs) | New, 862 lines, 29 tests: 18 in `tr11_c1` to `tr11_c5` and 11 in `tr53_c1` to `tr53_c5`, with the fake judge, `reply`, `started`, `held` and `dispute` helpers |
| [tests/refusal_wording.rs](../tests/refusal_wording.rs) | New (TR-54), 182 lines, 3 tests: `tr54_c1_` and `tr54_c2_`, with a fake judge as in `tests/disputes.rs` |
| [tests/fixtures/slice-15/](../tests/fixtures/slice-15/) | New: XX-25 in review under T12, and XX-26 approved |
| [tests/fixtures.rs](../tests/fixtures.rs) | `slice_15s_fixtures_keep_each_held_question`, which reads both files, and checks their field order, their byte-for-byte round trip and their schema |
| [tests/gates.rs](../tests/gates.rs), [tests/review.rs](../tests/review.rs) and [tests/states.rs](../tests/states.rs) | T12 in the six pinned trigger lists (one, four and one), additions only |
| [docs/DESIGN.md](DESIGN.md) | §2.3.2, §2.5's T11 row, §2.7, §4.3, §5, §6's T11-related row and §7's two bullets on tries. TR-53 changed §2.3.2's refusal bullet, the T11 row and §4.3's `held_questions`, and added a sentence on G3 after §3's held example |
| [docs/PLAN.md](PLAN.md) | Slice 15's status, `todo` to `in review` (`4dce835`). `d4a4eb2` marked it `in progress` |

To record another kind of held question, give `held_question` the trigger's name and the `Answered` it came from, and add the name to the schema's `trigger` enum. To add another trigger that reads the held questions, pass them to `triggers::run` as `disputed` is, and add it to every pinned trigger list in the three test files above before `start`.

## Testing

Run the test command in `tracker/`. TR-11's criteria run their own tests by prefix, such as `cargo test --test disputes tr11_c1_`, and criteria 6 to 8 are `grep` checks. No test calls `tracker-judge` or the real `claude`: the harness's guard from [slice 14](slice-14-readme.md#testing) puts a failing one first on `PATH`, and each test sets the store's `judge.command` to the fake. The harness ([tests/common/mod.rs](../tests/common/mod.rs)) didn't change. TR-54's tests are in [tests/refusal_wording.rs](../tests/refusal_wording.rs), with the same kind of fake judge.

**The fake judge in `tests/disputes.rs`** is a shell script in `<work>/judge/`, outside the project's root, that saves each request as `request-<n>.json` and prints `reply.json`. `reply` writes the answers from `(id, answer)` pairs, with the reason "The fake says no." for a no and "The fake says <answer>." for the rest. The fixtures are project XX with `src/lib.rs` holding `STUB`, and `work(env, n)`, which commits version `n` of the work, so each version is a different request. `started` is XX-1 started on the stub with a one-question judge, `works`, and `held` is that item held by a dispute of `c1:works`.

**The tests that approve or reject** use `Env::in_terminal_typing`, so they run under `script`, and return early, passing without checking anything, where `script` isn't installed, as slice 9's do. Seven of the 29 are such tests: the three in `tr11_c4_`, `tr11_c5_resubmitting_unchanged_code_after_a_reject_needs_the_dispute_again`, and TR-53's `tr53_c2_a_t10_question_records_its_ask_in_the_decision_too`, `tr53_c3_reject_records_every_question` and `tr53_c3_approve_records_the_submits_t10_questions`.

### Coverage of the _Done when_ checks

| _Done when_ | Tests |
| --- | --- |
| A dispute holds the item under T12, showing the excerpt, the model's answer and reason, and the agent's reason. | `tr11_c1_`, 4 tests: `tr11_c1_a_dispute_holds_the_item_under_t12` (result `held`, status `review`, T12 the only trigger fired, the `held_questions` entry's ref, answer, reason and dispute, and T12's place between T11 and T13), `tr11_c1_the_held_submit_shows_the_excerpt_and_both_reasons`, `tr11_c1_review_shows_the_model_s_and_the_agent_s_reasons` (text and `--json`) and `tr11_c1_submit_help_names_t12_and_dispute` |
| Disputing a question that passed or got an unsure answer exits 1. | `tr11_c2_`, 5 tests: `tr11_c2_disputing_a_question_that_passed_exits_1`, `tr11_c2_disputing_an_unsure_answer_exits_1`, `tr11_c2_disputing_a_ref_no_judge_asked_exits_1`, `tr11_c2_a_dispute_refused_with_exit_1_counts_its_run_toward_t11` and `tr11_c2_each_dispute_needs_its_own_reason`. Each refusal leaves the item file byte for byte as it was |
| A dispute doesn't stop another failing gate from refusing the submit. | `tr11_c3_`, 4 tests: `tr11_c3_a_failing_check_still_refuses_a_disputed_submit` (exit 2, G3 on criterion 2), `tr11_c3_a_failing_project_check_still_refuses_a_disputed_submit` (G4), `tr11_c3_an_undisputed_judge_fail_still_refuses_and_is_the_one_listed` (only `c1:clear` is listed) and `tr11_c3_disputing_every_fail_of_two_questions_holds_the_item` (two `--dispute` pairs, both in `held_questions`, in order) |
| Approving a T12 hold moves the item to done. | `tr11_c4_`, 3 tests, under `script`: `tr11_c4_approving_a_t12_hold_moves_the_item_to_done` (done, `human`, the question and both reasons in the prompt, `held_questions` gone from the item and recorded in the verification and in the `approve` history entry), `tr11_c4_a_dispute_covers_only_the_answer_it_disputed` (work changed while it waited, so the approve is refused with G3 and the item stays in review) and `tr11_c4_approving_a_t10_hold_records_the_unsure_question` |
| Resubmitting unchanged code after a reject needs the dispute again. T12 can't be disabled. | `tr11_c5_`, 2 tests: `tr11_c5_resubmitting_unchanged_code_after_a_reject_needs_the_dispute_again` (under `script`: the `reject` entry keeps the question, a plain resubmit is refused with `c1:works`, and a disputed one is held again) and `tr11_c5_t12_cant_be_disabled` (`review.disabled` names T12, and T12 still holds) |
| §7 says one run can try several wordings, and that a shelve, reopen and `start` empties the T11 count (criterion 6). | The item's `grep -qF` check of three phrases. It passes now |
| The T11 row and §5 say to fix every failing `each` match before running `gates` again (criterion 7). | The item's check, a `grep` of the T11 row and one of §5. It passes now |
| §6's T11 row and §7's forged-cache bullet no longer repeat TR-51's clauses (criterion 8). | The item's check: two `! grep -qF` guarded by `test -f docs/DESIGN.md`, and two `grep -qF` of what remains. It passes now |

Requirements from the _Prompt_ and the settled points that no _Done when_ bullet names:

| Requirement | Tests |
| --- | --- |
| `--dispute` is repeatable, with a reason for each. | `tr11_c3_disputing_every_fail_of_two_questions_holds_the_item` and `tr11_c2_each_dispute_needs_its_own_reason` |
| `approve` counts a question held under T10 as before, and records it. | `tr11_c4_approving_a_t10_hold_records_the_unsure_question` |
| `approve` and `reject` record the decision against each held question. | `tr11_c4_approving_a_t12_hold_moves_the_item_to_done` and `tr11_c5_resubmitting_unchanged_code_after_a_reject_needs_the_dispute_again` |
| A `--dispute` refused with exit 1 counts its run toward T11, and so does a disputed fail. | `tr11_c2_a_dispute_refused_with_exit_1_counts_its_run_toward_t11` and `tr53_c1_`, 2 tests, which read `judge/failures.json` |
| A held question records its `ask`, and a file written without it round-trips. | `tr53_c2_`, 3 tests |
| Under `auto_verify: false`, every question is recorded, and `approve` records the submit's. | `tr53_c3_`, 3 tests |
| `--reason`'s help, and the pairing order. | `tr53_c4_`, 2 tests |
| A dispute during an outage is refused for the failed call. | `tr53_c5_a_dispute_during_an_outage_is_refused_for_the_failed_call` |
| A refusal that counted a T11 run says the item wasn't changed, and one that counted nothing keeps "Nothing was changed." | `tr54_c1_a_refused_dispute_says_the_item_wasnt_changed`, `tr54_c2_a_submit_refused_for_a_judge_fail_says_the_item_wasnt_changed` and `tr54_c2_a_submit_refused_before_any_judge_is_asked_says_nothing_was_changed` |
| `held_questions` round-trips, and matches the schema. | `slice_15s_fixtures_keep_each_held_question` in [tests/fixtures.rs](../tests/fixtures.rs), and `Env::json`, which checks each output against its schema |

Not covered by a test:

- **A T11 hold records no decision for any question.** No test approves or rejects a T11 hold and checks that its history entry has no `held_questions`.
- **A duplicate `--dispute`, a blank or too long `--reason`, and a `--reason` with no `--dispute`.** Only a `--dispute` with no `--reason` is tested, and the pairing order ([review 1](reviews/slice-15-review-1.md#test-gaps), test gap 2). The rest share `check_reason` and the pairing code.
- **A hold under both T10 and T12.** A T10 reject is tested, under `auto_verify: false`, but a hold with both kinds of question isn't.
- **`show`'s text for held questions**, in "Held for review" and under a verification record. `review`'s is tested.
- **A T10 hold that first appears at `approve`.** No test approves an item whose re-ask raises a T10 question the submit didn't hold ([review 2](reviews/slice-15-review-2.md#test-gaps), test gap 1, left on purpose).
- **The outage branch for a missing input or an `each` with no matches.** `tr53_c5_` tests only a failed call ([review 2](reviews/slice-15-review-2.md#test-gaps), test gap 2, left on purpose).
- **An approve in which the judge answers a disputed request differently,** such as after the cache was deleted. `approve` matches the ref and the request, not the answer text ([Still open](#still-open)).

**Seen failing first** (TR-11). All eight of TR-11's checks failed at `start`, at `5e16893`: the `tr11_c1_` to `tr11_c5_` tests exercised a flag and a trigger that didn't exist, and criteria 6 to 8 grep text that `db355e4` added later. The six trigger lists, which needed T12 as soon as the code ran it, were changed in `5e16893`, so they were seen failing too: one in `tests/gates.rs`, four in `tests/review.rs` and one in `tests/states.rs`, each by adding `{ "trigger": "T12", "fired": false }` after T11's entry. They are additions only, which is why T4 stayed quiet. This is slice 14's process 1, now a rule in the agents' brief. `slice_15s_fixtures_keep_each_held_question` was in `5e16893` too and failed until `f5667e4` added the two fixtures, but no criterion runs it: it's in the full suite only.

**Seen failing first** (TR-54). Both checks failed at `start`, at `7752965`, which was committed 3 seconds before it. No existing test changed, so T4 stayed quiet. `tr54_c2_a_submit_refused_before_any_judge_is_asked_says_nothing_was_changed` is the guard for the unchanged wording: it passed at `start`, as intended, and criterion 2 failed through the other test, so T13 stayed quiet.

### Checks done by hand

| What | How | Result |
| --- | --- | --- |
| The demo above | Run line by line with the fake judge, stderr captured, on a temporary `TRACKER_HOME`, with the temporary folder replaced by `$DEMO`. The real `claude` and `tracker-judge` weren't called | The output above. Every refusal exits as marked |
| TR-11's checks, before the approve | The orchestrator checked by hand that the five written criteria are unchanged, that all eight checks failed at `start`, and that every commit since `start` is scoped to TR-11 | All three held |
| TR-11's approve in a real terminal | The user read the diff and approved it | `by: human` at `4dce835`, with `refs/tracker/TR-11` and three holds |
| The demo above, after TR-54 | Re-run as written with the fake judge, on a temporary `TRACKER_HOME`, and compared with the output before. The real `claude` and `tracker-judge` weren't called | Three lines changed to "BM-1 wasn't changed.", and the request fingerprint. Nothing else |
| TR-53's checks, before the approve | The orchestrator checked by hand that the six written criteria are unchanged and that every commit since `start` is scoped to TR-53. The user then read the diff and approved it in a terminal | Both held: `by: human` at `7e32521`, with `refs/tracker/TR-53` and three holds |

### Tracking TR-11

TR-11, a `capability` with `design` `docs/DESIGN.md#2.3.2`, was created at 2026-11-21T07:07:43Z with slice 7's other items, with five criteria. The orchestrator added criteria 6 to 8 before `start`, and the written ones are unchanged. The slice 14 binary was installed before it started and tracked it throughout, so T5, T13 and T14 ran on it:

| Step | When (UTC) | What |
| --- | --- | --- |
| Tests, committed | 17:57:09Z | `5e16893`, 34 seconds before `start` |
| `tracker start TR-11` | 17:57:43Z | `started.head` `5e16893`. All eight checks failed |
| The code and docs | 17:57:46Z to 18:06:26Z | `d4a4eb2`, `f2f40ca`, `f5667e4`, `db355e4` and `4dce835` |
| `tracker submit TR-11` | 18:11:15Z | The gates passed, then the triggers ran. Held under T1, T7 and T8 |
| `tracker approve TR-11` | 18:19:48Z | From a terminal, verified `human` at `4dce835`, with `refs/tracker/TR-11` |

- **The three holds** read: T1, "it's a capability, so a human should check the design fit". T7, "1218 lines changed, more than the project's max_diff_lines of 300", 574 of them in `tests/disputes.rs`. T8, "the project's design doc changed: docs/DESIGN.md". They're recorded in `verification.triggers`, and `tracker show TR-11` is the record.
- **T4 didn't fire.** The six trigger lists changed in the test commit, before `start`, and by additions, and `tests/fixtures.rs` only gained a test.
- **T13 didn't fire.** Each criterion's check failed at `start`.
- **T11 and T12 didn't fire,** since TR-11 has no judge criterion.
- **TR-47, TR-48 and TR-50** were open before this slice, and are still open and deferred.

## Decisions worth knowing

These are the "Slice 15 must decide" points from slice 14's reviews, and what the slice chose:

- **A disputed fail still counts toward T11.** A dispute is a fail the agent chose to argue, and `count_failures` runs before the dispute is taken out. An agent usually saw the fail in `gates` first, so the run is counted already, and the `submit --dispute` that follows adds nothing. §2.5's T11 row says so, and `tr53_c1_a_held_dispute_of_a_new_failing_request_adds_one_run` pins it.
- **A `--dispute` refused with exit 1 counts its run.** The judges were asked, and the refusal says which way the disputed question went, so counting nothing let an agent learn answers for free ([review 1](reviews/slice-15-review-1.md#security), security 1). This reverses the first version of this slice, which followed [slice 14's review 2](reviews/slice-14-review-2.md). A submit that stops for an earlier gate counts nothing, since the judges weren't asked, and neither does one refused for a failed judge call.
- **Disputes live in `held_questions`, on the item, while it's in review.** `Hold` has only `trigger`, `message` and `files`, and `approve` runs the gates again, so it needs the refs, the requests and the agent's reasons as data. The field is next to `holds` because a question isn't a hold: one hold, T12 or T10, can name several questions.
- **The excerpt isn't stored.** It can be up to `max_input_bytes`. The submit's output, `approve`'s warning and `gates` print it. `review` and `show` print the two reasons, and `review` points at `gates`. [Slice 12's review 1](reviews/slice-12-review-1.md) offered storing it on the hold as design 2's option (b), and this slice doesn't.
- **Decisions go in the `approve` and `reject` history entries, and in `verification.held_questions`.** History is never rewritten, so a decision lasts through a reopen, which moves the record to `previous_verification`, and slice 16's `judges` can count from history alone.
- **T11 holds record no per-question decision.** A T11 hold is about the item: its message names the questions, and `failures.json` is reset by each `start`. §2.7 says so. `judges` will count T10 and T12 decisions only, and a T11 hold's refs are in its message ([Limits](#limits-and-what-later-slices-change)).
- **T11's unit stays runs.** Slice 14's review 2 asked that any change to the unit come before `judges` counts per question. It didn't change, and §2.5, §5, §6 and §7 now say what a run can't bound: several wordings at once as separate `each` matches, and a shelve, reopen and `start`.
- **A missing judge `reason` passes as an empty string.** `tracker-judge` hands the CLI's reason on as it gave it, empty when the CLI gave none ([slice 13](slice-13-readme.md#limits-and-what-later-slices-change)), and the tool doesn't fail the call for it. A disputed question with an empty model reason is held and printed with the reason empty. The tool's reply type requires the field, so a judge command that leaves the key out entirely is a parse failure, as before.
- **A dispute covers one answer.** `approve` removes only a G3 failure whose ref and request match a held question, so work changed while the item waits gets a new request, and a fail there refuses the approve. `approve` and T12 both read `held_questions` from the item, not from the agent's next submit.
- **The reason is checked as a `reject` reason is:** one printable line, trimmed, at most 500 characters, with the same function. A dispute can't be left without one, and a ref can be disputed once.
- **T12 is in every pinned trigger list, and can't be disabled.** `config::ALWAYS_ON` has named T12 since slice 10, and `tr11_c5_t12_cant_be_disabled` pins it now that T12 runs.
- **`approve` runs T12 as well as the other triggers,** on the questions the item holds, so a T12 hold appears among the holds `approve` shows and records in `verification.triggers`.
- **A held submit also records T10's questions as `held_questions`,** so `approve` and `reject` can record them, and `judges` can count them: each unsure answer, and, under `auto_verify: false`, every answer a judge didn't fail. `approve` copies the stored ones, as `reject` does, and doesn't build them again from its own re-ask.
- **A held question records its `ask`,** the question's text, because the ref's criterion number and `file:line` change, and the ref's question id is only unique within one judge file.
- **A refusal that has counted a T11 run says "<ID> wasn't changed."** Option (b) of [review 2's bug 1](reviews/slice-15-review-2.md#bugs): a refused dispute and a submit refused for a judge's fail or unsure answer no longer end "Nothing was changed.", which read as a free try. A refusal that counts nothing keeps the old words. The message shows no count, since option (a)'s "2 of 3" would tell an agent how many tries it has left.
- **A disputed fail leaves G3 `ok` in `submit --json`.** A held item has passed every gate it could, and the fail is in `judges` and `item.held_questions`. §3 and the schema say so.

## Fixes after review 1

[Review 1](reviews/slice-15-review-1.md) found that slice 15 does what PLAN.md asks and that nothing blocks the install, with two bugs, one security item, three design questions, three test gaps and points for slice 16. Its main finding was that a refused dispute was a free judge oracle that got past T11: an agent could try wording after wording, learn each answer from the refusal, and have nothing counted (security 1). The fixes were built as one item, TR-53, so the user approves once, with its tests committed before `tracker start`: `109b32f` (tests), `aff07dc` (a test's marker text), `b09c5e6` (the code) and `7e32521` (DESIGN.md and a schema). The review's [Outcome](reviews/slice-15-review-1.md#outcome) gives the reason for each decision.

### Tracking TR-53

TR-53 is a `fix`, created at 2026-11-23T18:36:00Z and tracked with the installed slice 14 binary, so T5, T13 and T14 ran on it. It has 6 criteria, and the orchestrator checked by hand that they're unchanged and that every commit since `start` is scoped to TR-53.

| Step | When (UTC) | What |
| --- | --- | --- |
| Tests, committed | 18:39:50Z | `109b32f`, 15 seconds before `start` |
| `tracker start TR-53` | 18:40:05Z | `started.head` `109b32f`. All 6 checks failed |
| The code and docs | after `start` | `aff07dc`, `b09c5e6` and `7e32521` |
| `tracker submit TR-53` | 18:50:10Z | The gates passed, then the triggers ran. Held under T4, T7 and T8 |
| `tracker approve TR-53` | 18:56:58Z | From a terminal, verified `human` at `7e32521`, with `refs/tracker/TR-53` |

- **The three holds** are T4, "existing test file modified: tests/disputes.rs (109b32f before start, aff07dc after start)", T7, "374 lines changed, more than the project's max_diff_lines of 300", and T8 (DESIGN.md). The user read the diff and approved them.
- **Why T4 fired.** `109b32f` renamed TR-11's `tr11_c2_a_dispute_refused_with_exit_1_counts_nothing_toward_t11` to `tr11_c2_a_dispute_refused_with_exit_1_counts_its_run_toward_t11` and changed it to assert 1, not 0, since that is the behaviour security 1 reverses. It was in the test commit, before `start`, so it was seen failing. `aff07dc` fixed the marker text of a test TR-53 itself added, `tr53_c2_files_written_without_ask_round_trip_byte_for_byte`, which reused text the fixture's criterion already held. It edits no test that existed before the review, so nothing was weakened.
- **Seen failing.** All 6 checks failed at `start`. The pairing-order test, `tr53_c4_two_disputes_then_two_reasons_pair_in_order`, passed before the work, since the code was right and only the help was wrong, so criterion 4 failed through `tr53_c4_reason_help_says_reasons_pair_by_position`. T13 didn't fire, since every check failed.
- **After it,** `cargo fmt --check`, `cargo clippy -- -D warnings` and all 534 tests pass, 11 more than before the review, all in `tests/disputes.rs`.

### Bugs

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Bug 1: `--reason`'s help says it goes with "the --dispute before it", but the code pairs by position | `b09c5e6`, option (a) | The help reads "Reasons pair with disputes by position: the first --reason goes with the first --dispute, and so on". Option (b), pairing by position on the command line, isn't taken: the likely case already works | `tr53_c4_reason_help_says_reasons_pair_by_position`, failed at `start`, and `tr53_c4_two_disputes_then_two_reasons_pair_in_order`, which passed at `start` and pins the pairing |
| Bug 2: during a judge outage, a dispute is refused with exit 1 as "no judge question … was asked" | `b09c5e6`, option (a) | `apply` skips `check_disputes` when a G3 failure has no answer, so the submit is refused for the failed call with exit 2, as for an earlier gate. §2.3.2 says so | `tr53_c5_a_dispute_during_an_outage_is_refused_for_the_failed_call`, failed at `start` |

### Security items

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Security 1: a refused dispute is a free judge oracle that gets past T11 | `b09c5e6` and `7e32521`, option (a) | `count_failures` runs before `check_disputes`, so a refused dispute counts its run, as a `gates` run does. §2.3.2 and the T11 row say so, and this reverses [slice 14's review 2](reviews/slice-14-review-2.md)'s advice to count nothing. Option (b), refusing a dispute with a ref nobody asked before the judges run, isn't taken | `tr53_c1_a_refused_dispute_of_a_ref_nobody_asked_counts_a_failing_run` and `tr11_c2_a_dispute_refused_with_exit_1_counts_its_run_toward_t11`, both failed at `start`, and criterion 6's `grep` checks |

### Design decisions

| Question | Decision | Commits |
| --- | --- | --- |
| Design 1: a held question records its ref, not the question, so `judges` has nothing stable to group by | Option (a): `HeldQuestion` has `ask`, the question's text, with a serde default, left out when empty, and in `item.schema.json`'s `held_question`. Review, verification and history carry it. §4.3 says `judges` groups by it within a project | `b09c5e6`, `7e32521` |
| Design 2: under `auto_verify: false`, T10 holds with no question recorded, and `approve` and `reject` record questions from different runs | Options (b) and (a): `t10_questions` replaces `unsure_questions`, and records every answer a judge didn't fail as T10 when `auto_verify` is false. `approve` copies the submit's `held_questions`, as `reject` does, and no longer builds them from its own re-ask | `b09c5e6`, `7e32521` |
| Design 3: a held submit's JSON reports G3 as passed though its judge failed | Option (a): §3 and `submit.schema.json` say a disputed fail leaves G3 `ok`, with the fail in `judges` and in `item.held_questions`. No code changed | `7e32521` |

### Test gaps

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Test gap 1: nothing pins that a disputed fail counts toward T11 | `109b32f` | A test that a held `submit --dispute` of a new failing request adds one run | `tr53_c1_a_held_dispute_of_a_new_failing_request_adds_one_run`, which passed at `start`, since the code already counted it. Criterion 1's check fails at `start` through its other test |
| Test gap 2: only one of the input rules has a test | Not taken | Low value: a ref disputed twice, a blank or over-long reason and a `--reason` with no `--dispute` share `check_reason` and the pairing code. The pairing order is tested, under bug 1 | |
| Test gap 3: `show`'s text, a T10 reject and a combined T10 and T12 hold have no test | Not taken | Low value. A T10 reject is recorded by `tr53_c3_reject_records_every_question`, as a side effect of design 2 | |

### Slice 16 must decide

Left for TR-12, as the review wrote them, except design 1, which came first and is done: what a "judge question" is for `judges` is now the `ask` in each entry. What `judges` and the digest read, and what `audit` checks of `held_questions`, are in [Limits](#limits-and-what-later-slices-change).

### A few consequences

- A `submit --dispute` that is refused with exit 1 counts toward T11, so an agent that disputes wrongly while working on a question can reach `max_failures` and be held.
- `held_questions` entries have an `ask`. A file written without one still loads and comes back byte for byte, but its decisions have no question text for `judges` to group by.
- Under `auto_verify: false`, a hold has a `held_questions` entry for each answer a judge didn't fail, and `approve`'s record has them too, where it had none before.
- A dispute made while a judge call fails exits 2 with G3's message, where it exited 1.
- `--reason`'s help changed, so a script that matches the old text breaks.

## Fixes after review 2

[Review 2](reviews/slice-15-review-2.md) found that TR-53's fixes are right and complete, and that nothing blocks the install. It had one bug, three design points, two test gaps and one process point. The bug was fixed as one item, TR-54, with its tests committed before `tracker start`: `7752965` (tests) and `2a9989d` (the code). Two design points were folded into TR-12, slice 16's item, before it starts. The rest were left or done here. The review's [Outcome](reviews/slice-15-review-2.md#outcome) gives the reason for each decision.

### Tracking TR-54

TR-54 is a `fix`, created at 2026-11-23T20:35:24Z and tracked with the installed slice 14 binary. It has 2 criteria, each checked by `cargo test --test refusal_wording` with its own prefix.

| Step | When (UTC) | What |
| --- | --- | --- |
| Tests, committed | 20:36:09Z | `7752965`, 3 seconds before `start` |
| `tracker start TR-54` | 20:36:12Z | `started.head` `7752965`. Both checks failed |
| The code | 20:39:35Z | `2a9989d` |
| `tracker submit TR-54` | 20:43:37Z | The gates passed and no trigger fired. Verified `auto` at `2a9989d`, with `refs/tracker/TR-54` and no holds |

`tracker show TR-54` records no trigger and no hold. It changed no existing test, so T4 stayed quiet. The fix's code is two small changes, in `Report::refusal` ([src/gates.rs](../src/gates.rs)) and `check_disputes` ([src/main.rs](../src/main.rs)).

### Bugs

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Bug 1: a refused dispute says "Nothing was changed." though it has just counted a run toward T11 | `2a9989d`, option (b) | `check_disputes` ends "<ID> wasn't changed." `Report::refusal` does too for a submit when some judge answer isn't a pass, since `count_failures` has counted that run. A refusal that counts nothing, such as an earlier gate failing, a failed judge call or a missing reason, keeps "Nothing was changed." Option (a), saying "2 of 3", isn't taken: it would tell an agent how many tries it has left | `tr54_c1_a_refused_dispute_says_the_item_wasnt_changed` and `tr54_c2_a_submit_refused_for_a_judge_fail_says_the_item_wasnt_changed`, both failed at `start`, and `tr54_c2_a_submit_refused_before_any_judge_is_asked_says_nothing_was_changed`, which passed at `start` and pins the wording that stays |

### Design decisions

| Question | Decision | Commits |
| --- | --- | --- |
| Design 1: an approve of a held question means different things in different entries, and DESIGN.md doesn't say how `judges` tells them apart | Option (a), folded into TR-12 as its criterion 8: `judges` splits each question's counts by the held question's trigger and answer, and §6 says so. Nothing changes in slice 15's code, since the entry already has both fields | None here: TR-12's |
| Design 2: `approve` records only the submit's held questions, so a T10 hold that first appears at `approve` gets no decision recorded | Not taken: unlikely, since it needs a commit while the item waits that turns a judged input's answer unsure, and only `judges`' counts lose the decision | None |
| Design 3: §2.3.2 says a failed call skips the dispute check, but the code skips it for any G3 failure with no answer | Option (a), folded into TR-12 as its criterion 9: §2.3.2 says any G3 failure without a judge answer, such as a missing input or a failed call, skips it. The code was right | None here: TR-12's |

TR-12's seven written criteria are unchanged. Criteria 8 and 9 were added before it starts.

### Test gaps

| Finding | Commit | Fix | Tests |
| --- | --- | --- | --- |
| Test gap 1: no test covers a T10 hold that first appears at `approve` | Not taken | It belongs with design 2's fix, which wasn't made | |
| Test gap 2: `unasked` is tested only for a failed call | Not taken | Low: a missing input or an `each` with no matches takes the same branch, and each case refuses with exit 2 anyway | |

### Process

| Finding | Commit | Fix |
| --- | --- | --- |
| Process 1: the readme's "Test command" row says the command passes at `4dce835`, though Status says it was run at `7e32521` | This update | The row names `2a9989d`, the last commit to change code |

### Slice 16 must decide

Left for TR-12 as review 2 wrote them, except design 1, now its criterion 8, and design 3, now its criterion 9. What `judges` counts, how it counts items and not entries, its grouping key, escaping `ask`, where decisions are read, what the digest shows of a dispute and what `audit` checks are in [Limits](#limits-and-what-later-slices-change).

### A few consequences

- A refusal that has counted a T11 run says "<ID> wasn't changed." A script that matches "Nothing was changed." in a refused dispute or a refused submit that follows a judge's fail breaks. A refusal before any judge is asked still says it.

## Still open

From the build, the checks and reviews [1](reviews/slice-15-review-1.md) and [2](reviews/slice-15-review-2.md). Everything review 1 asked to be fixed is fixed except what is listed here, and so is review 2's bug 1.

**Checks not run:**

- **A terminal `approve` of a held T12 item.** The tests approve and reject under `script` in a throwaway store, and the user approved TR-11, a different hold, from a terminal. Nobody has typed an ID into `approve` for a T12 hold in a real terminal, so what the question prints for one hasn't been read by a person.
- **A real judge.** No run used the real `claude`. The tests and the demo use a fake judge command, so how a real judge's fail reads next to an agent's dispute, and what a disputed question's reason looks like when the model gives none, are unchecked.

**Security:**

- **Review 1's security 1, option (b)**, refusing a dispute with a ref nobody asked before the judges run, isn't taken. A refusal now counts its run, so an agent that disputes wrongly can still learn answers, but only at T11's cost.

**Design** (review 2): its design 2, that `approve` records only the submit's held questions, so a T10 hold that first appears at `approve` gets no decision recorded (also test gap 1 there). Unlikely, and only `judges`' counts lose the decision. §2.7 and §4.3 don't say that a decision covers the submit's questions only.

**Test gaps** (see [Testing](#testing)): a T10 hold that first appears at `approve` (review 2's test gap 1), the outage branch tested only for a failed call (review 2's test gap 2), a T11 hold recording nothing per question, the input rules for duplicate, blank and over-long reasons ([review 1](reviews/slice-15-review-1.md#test-gaps), test gap 2), `show`'s text for held questions and a hold under both T10 and T12 (test gap 3), and a changed answer to a disputed request.

**Not decided:**

- **An approve matches the request, not the answer.** `approve` removes a G3 failure whose ref and request match a held question, and doesn't compare the answer. If the cache entry is deleted and the judge answers the same request with another fail, the approve still counts it as passed. A deleted cache entry is the "forged judge answer" §7 already describes, and the fail is one a human approved, so this is stated here, not closed.
- **T12's message for several questions** reads "the agent disputed judges' fails to 2 questions", which is clumsy. It names the refs, and nothing matches on the wording.

**Deferred items** (from [slice 14's readme](slice-14-readme.md#still-open)): **TR-47** (a `diff` judge's block comments, still open, and a disputed `diff` question's excerpt has the same flaw), **TR-48** (what else a real judge sees) and **TR-50** (escaping `</material>`, and pinning step 1's prompt). All three are deferred.

**Left for slice 16** (the points of slice 14's reviews that this slice doesn't change, and review 1's "Slice 16 must decide"): [Limits](#limits-and-what-later-slices-change) has them.

**Carried from slice 14** ([its Still open](slice-14-readme.md#still-open)): tries before `start` are free, a judge whose call failed at `start`, and removing `judge/failures.json`, all stated in §7, and the test gaps of `criterion remove` renumbering a judge's entry and `approve` reading T11.

## Limits, and what later slices change

- **Slice 16**'s `judges` counts, for each judge question, how often I approved and how often I rejected the items it held. It reads the `held_questions` of every `approve` and `reject` history entry. Each entry has `ref`, which names a `file:line` for an `each` match, and `ask`, the question's text, which is what `judges` groups by within a project ([§4.3](DESIGN.md#43-item-file)). An entry written before TR-53, such as the two fixtures in [tests/fixtures/slice-15/](../tests/fixtures/slice-15/), has no `ask`, so `judges` must cope with an entry that lacks one. It can only count T10 and T12 decisions. A T11 hold records none, so a question's T11 holds aren't in its counts, and its refs live in the hold's message and in `judge/failures.json`, which each `start` resets ([slice 14's limits](slice-14-readme.md#limits-and-what-later-slices-change)). A reopened item's earlier decisions are in its history, not its record, so `judges` must read history. TR-12's criterion 8 adds that it splits each question's counts by the held question's trigger and answer, since under `auto_verify: false` an approve of a passing answer sides with the judge and an approve of a disputed fail overrules it, and that §6 says so. It must also count each decision once per question, not once per entry, since an `each` question holds one entry per match, and print `ask` through `text::for_terminal`, since a judge file's text comes from an agent.
- **Slice 16**'s digest lists every `approve` and `reject` I made, and every shelve. A disputed question's reasons are in the `approve` entry's `held_questions` and in `verification.held_questions`, so the digest can show them, but no code does yet. Slice 14's review 2 also wants the digest to make a shelve and a reopen of an item with a judge criterion easy to spot, which has no criterion.
- **Slice 16**'s `audit` doesn't read `held_questions`: it can't tell an item in review that holds one from one that doesn't, and it doesn't read `judge/failures.json`, so a count or a decision edited by hand isn't reported. TR-12's criterion 7 adds PLAN.md's check of `refs/tracker/<ID>`. Its criterion 9 adds to §2.3.2 that any G3 failure without a judge answer skips the dispute check, which is what `apply`'s `unasked` already does.
- **Saved rulings** are future work ([§7](DESIGN.md#7-honest-limits)). If they're added, the key is the excerpt and the question, so it survives a model change, and a held question's `request` is the wrong key: it changes with the model. A decision would then no longer cover one submit, and §2.3.2's "no saved rulings in v1" and the resubmit rule of `tr11_c5_` change with it.
- **Any change to the request** (the model, a prompt, a question) is a new fingerprint, so a held item's `request` no longer matches the answer `approve` gets, and the approve is refused with G3. A change to the fingerprint's inputs while an item waits in review needs the agent to dispute again.
- **A new trigger** goes into every pinned trigger list in [tests/gates.rs](../tests/gates.rs), [tests/review.rs](../tests/review.rs) and [tests/states.rs](../tests/states.rs), and into `triggers::run`'s order, as T12 did. Add it in the item's first test commit, so it's seen failing. `triggers::run` has five arguments now, and every caller passes `&[]` for the ones it hasn't got, as `gates_dry_run` does for `disputed`.
- **A new place that moves an item out of review** must clear `held_questions` with `holds`, as `approve` and `reject` do. They are present only while the item is in review, and `record_decision` is the one place their decision is recorded.
- **TR-47 and a `diff` judge.** Until it's done, a `diff` judge sees comment text that §2.3.1 says it doesn't, and a dispute of one is about an excerpt with that flaw. A project shouldn't add one before then.
- **Interrupting `tracker`.** `submit --dispute` asks the judges as `submit` does, so a Ctrl-C during it can leave the judge command and its `claude` running, as §7's interrupt bullet says for every command that runs judges.

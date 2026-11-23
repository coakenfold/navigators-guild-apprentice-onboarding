# Slice 15: Disputes

`tracker` is a command-line work tracker, written in Rust, that keeps one central list of work items across many projects and decides when an item is really done. This document covers **slice 15** of the 16 in [PLAN.md](PLAN.md): what it built, how it works, and how to test it. The earlier readmes cover the rest: [slice 1](slice-1-readme.md) the store, `create` and `list`, [slice 2](slice-2-readme.md) the states and `history`, [slice 3](slice-3-readme.md) priority, kind, labels, `edit` and `list`'s filters, [slice 4](slice-4-readme.md) `show` and `delete`, [slice 5](slice-5-readme.md) bad data and polish, [slice 6](slice-6-readme.md) criteria and gates G1, G2 and G6, [slice 7](slice-7-readme.md) running checks, with gates G3 and G4 and `started.checks`, [slice 8](slice-8-readme.md) G5, the `auto` record and `refs/tracker/<ID>`, [slice 9](slice-9-readme.md) the inbox, `approve`, `reject` and triggers T6 and T9, [slice 10](slice-10-readme.md) triggers T1, T2, T3, T4, T7 and T8, [slice 11](slice-11-readme.md) triggers T5, T13 and T14 and the criteria fingerprint, [slice 12](slice-12-readme.md) judge criteria, G3's judge half and trigger T10, with a fake judge command, [slice 13](slice-13-readme.md) `tracker-judge`, the real judge command, and [slice 14](slice-14-readme.md) judges that must fail first, trigger T11 and `judge/failures.json`.

After slice 15 you can **dispute a judge's wrong fail and decide it yourself**. The spec is [DESIGN.md](DESIGN.md). This slice implements [§2.3.2](DESIGN.md#232-when-a-judge-is-wrong) (disputes, and where they live while the item waits), trigger T12 from [§2.5](DESIGN.md#25-review-triggers-does-it-need-a-human) and what its T11 row says a dispute costs, what `approve` and `reject` record in [§2.7](DESIGN.md#27-human-actions), `held_questions` from [§4.3](DESIGN.md#43-item-file), and `submit --dispute` in [§3](DESIGN.md#3-interface). It also changes [§7](DESIGN.md#7-honest-limits), [§6](DESIGN.md#6-what-im-trusting-and-what-checks-it) and [§5](DESIGN.md#5-working-with-agents) to say what a T11 run can and can't bound, as [slice 14's review 2](reviews/slice-14-review-2.md) asked. The slice is tracked in `tracker` itself as item TR-11, with the binary installed from slice 14 ([Tracking TR-11](#tracking-tr-11)). No review of slice 15 exists yet, so this document has no fixes section.

| | |
| --- | --- |
| Status | Every automated _Done when_ check has a test, and every test passes, as does the test command (`cargo fmt --check`, `cargo clippy -- -D warnings` and `cargo test` were run on the finished branch, at `4dce835`). TR-11 is `done`, but it wasn't auto-verified. It has eight criteria: criteria 1 to 5 are PLAN.md's _Done when_ bullets, as written (the fifth joins two), and 6 to 8 are DESIGN.md wording the orchestrator added before `start` from [slice 14's review 2](reviews/slice-14-review-2.md) (security 1 and 2, design 1 and design 2). All eight checks failed at `start`. Criteria 1 to 5 run `cargo test --test disputes` by prefix, and 6 to 8 are `grep` checks of DESIGN.md. The installed slice 14 binary held the submit under T1 (it's a capability), T7 (1218 lines changed) and T8 (DESIGN.md changed). T4 stayed quiet: the only existing tests changed are the six pinned trigger lists, by additions, before `start`. The user read the diff and approved it from a terminal: `verification.by` is `human`, at `4dce835`, with `refs/tracker/TR-11`, and the three holds are recorded in `verification.triggers`. Before the approve, the orchestrator checked by hand that the five written criteria are unchanged, that all eight checks failed at `start`, and that every commit since `start` is scoped to TR-11. **Not checked by hand:** approving a T12 hold in a real terminal. The tests do it under `script` in a throwaway store, and the user's approve of TR-11 was of a T1, T7 and T8 hold ([Still open](#still-open)). Two settled points have no test of their own: that a disputed fail still counts toward T11, and that a T11 hold records no decision for any question ([Testing](#testing)). No review of slice 15 has been done. PLAN.md has slice 15 as `in review`. The branch isn't merged into `main`, and neither are slices 7 to 14, which it's stacked on |
| Branch | `feat/tracker/slice-15`, stacked on `feat/tracker/slice-14` at `ebbc62f`. Slice 14 isn't merged into `main`, so this slice's commits are `git log --oneline --reverse feat/tracker/slice-14..feat/tracker/slice-15`. In order, all scoped to TR-11: `5e16893` (the tests, committed before `tracker start TR-11`), `d4a4eb2` (PLAN.md: slice 15 in progress), `f2f40ca` (the code), `f5667e4` (two fixtures, XX-25 and XX-26), `db355e4` (DESIGN.md) and `4dce835` (PLAN.md: slice 15 in review). That is 6 commits, and no commit outside TR-11 sits in the range. The commit that adds this readme belongs to no item |
| Test command | `cargo fmt --check && cargo clippy -- -D warnings && cargo test`, run in `tracker/`. All three pass at `4dce835` |
| Tests | 523: 46 in `tests/risky_changes.rs`, 42 in `tests/judges.rs`, 39 in `tests/create_and_list.rs`, 36 in `tests/commits.rs`, 34 in `tests/organise.rs`, 29 in `tests/judges_first.rs`, 28 in `tests/review.rs`, 27 in `tests/states.rs`, 25 in `tests/show_and_delete.rs`, 23 in `tests/criteria.rs`, 22 in `tests/gates.rs`, 21 in `tests/tampered_proof.rs`, 18 in `tests/disputes.rs`, 18 in `tests/checks.rs`, 16 in `tests/fixtures.rs`, 14 in `tests/item_files.rs`, 13 in `tests/bad_data.rs`, 12 in `tests/tracker_judge.rs`, 11 in `tests/docs_links.rs`, 9 in `tests/polish.rs`, 6 in `tests/output.rs`, 6 in `tests/t5_and_fingerprint.rs`, 5 in `tests/schemas.rs`, 4 in `tests/store_writes.rs`, 2 in `tests/item_ids.rs`, 1 each in `tests/harness_check.rs`, `tests/judge_add_message.rs` and `tests/submit_help.rs`, 12 unit tests in the library (4 in `src/gates.rs`, 4 in `src/judge.rs`, 2 in `src/git.rs`, 1 each in `src/store.rs` and `src/checks.rs`) and 2 in `src/main.rs`. That is 19 more than slice 14's 504: 18 in the new `tests/disputes.rs` and 1 in `tests/fixtures.rs` |

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
- [Still open](#still-open)
- [Limits, and what later slices change](#limits-and-what-later-slices-change)

## What's in this slice

**Built:**

- **`submit --dispute <ref> --reason "…"`, repeatable.** Each `--dispute` is paired with the `--reason` at the same place. `submit` runs every gate as usual. If the only failures are the disputed questions, the item goes to review, held under T12. If anything else fails, the submit is refused, as before.
- **Exit 1 for a ref the judge didn't fail.** A ref the judge passed, answered unsure, or never asked is refused with exit 1, naming the ref and why, and changes nothing. The refusal comes before T11 counts anything.
- **T12.** It fires when the submit disputed at least one fail, and sits between T11 and T13. T12 is in `config::ALWAYS_ON` with T6 and T10, so `review.disabled` can't switch it off. `approve` runs T12 too, on the questions the item holds.
- **`held_questions` on the item.** While the item is in review, each judge question that holds it is kept: a disputed fail (T12) with the agent's reason, and an unsure answer (T10). It's kept without the excerpt ([What held_questions holds](#what-held_questions-holds)).
- **`review`, `show`, `approve` and `reject` show it.** `review` and `show` print each disputed question's ref, the model's answer and reason, and the agent's reason. The held submit and `approve`'s question print the excerpt too, since both have just asked the judge.
- **`approve` counts a disputed fail as passed, for that approval only.** It matches the answer by the question's ref and its request fingerprint, so a change made while the item waited, which gets a new answer, still refuses the approve with G3.
- **`approve` and `reject` record the decision against each held question.** The history entry of each keeps the questions in its own `held_questions`, and an approve copies them to `verification.held_questions`. Slice 16's `judges` counts from these.
- **DESIGN.md says it.** §2.3.2 says where a dispute lives and what a refusal costs, §2.5's T11 row says what a dispute costs and to fix every failing `each` match before running `gates` again, §2.7 and §4.3 describe the records, §5 repeats the `each` advice, and §6 and §7 say what a T11 run can't bound.
- **Slice 14's "Slice 15 must decide" points**, from [review 1](reviews/slice-14-review-1.md#slice-15-must-decide) and [review 2](reviews/slice-14-review-2.md#slice-15-must-decide): see [Decisions](#decisions-worth-knowing).

**Not built yet:**

- **`tracker judges`, the digest's dispute entries, and `audit`'s look at `held_questions` and `judge/failures.json`** (slice 16). Nothing reads the recorded decisions yet.
- **Saved rulings** (future work, [§7](DESIGN.md#7-honest-limits)). A decision covers one submit, so a resubmit of unchanged code after a reject needs the same dispute again.
- **TR-47, TR-48 and TR-50** (a `diff` judge's block comments, what else a real judge sees, and escaping `</material>`). All three are deferred ([Still open](#still-open)).

## Try it

Always set `TRACKER_HOME` while developing. Without it, the tool uses your real store at `~/.tracker`.

**This demo never calls the real `claude` or `tracker-judge`.** Its judge command is a shell script that reads the request and prints whatever is in `reply.json`, as `tests/disputes.rs` does, and a small `reply` script writes that file from `id=answer` pairs, so the demo sets each answer. The project BM is a folder in one throwaway repository, as TR is a folder in guild-portfolio. BM-1 has a judge criterion with two questions, `works` and `clear`. The judge answers each version of the code, and the demo disputes what it got wrong:

- **Version 1:** the judge fails `works` and passes `clear`. Three disputes are refused with exit 1: one with no reason, one of the question that passed and one of a question nobody asked. None is counted toward T11.
- **Version 2:** `works` and `clear` both fail. Disputing only `works` is refused with exit 2 for `clear`, and that run counts toward T11.
- **Version 3:** both fails are disputed, and BM-1 is held under T12. `review` and `show` print the questions, `approve` and `reject` are refused without a terminal, and the item file keeps `held_questions`.

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
error: BM-1 can't be submitted with that dispute: the judge passed c1:clear, so there's nothing to dispute. Only a judge's fail can be disputed. Nothing was changed.
[exit 1]
$ tracker submit BM-1 --dispute c9:nope --reason "No such question."      # exit 1: no such question was asked
Criterion 1's judge: answered from the cache
error: BM-1 can't be submitted with that dispute: no judge question c9:nope was asked. `tracker gates BM-1` lists each question's ref. Only a judge's fail can be disputed. Nothing was changed.
[exit 1]
$ jq '.["BM-1"] | {at_start: (.at_start | length), failed: (.failed // [] | length)}' $TRACKER_HOME/judge/failures.json
{
  "at_start": 1,
  "failed": 0
}
$ printf 'fn run() {\n    step(2);\n}\n' > bm/src/lib.rs
$ git add -A && git commit --quiet -m "feat(BM-1): version 2"
$ reply works=no clear=no
$ tracker submit BM-1 --dispute c1:works --reason "step calls the helper that does the work, at lib.rs:2."      # exit 2: c1:clear still fails
Asking criterion 1's judge 2 questions: `$DEMO/judge/fake`
error: BM-1 can't be submitted: G3 failed. Nothing was changed.
  G3  Criterion 1's judge answered no to c1:clear, "Is this code clear?", and only yes passes: The fake says no to clear. Its excerpt follows.
        [src/lib.rs]
        fn run() {
            step(2);
        }
[exit 2]
$ jq '.["BM-1"] | {at_start: (.at_start | length), failed: (.failed // [] | length)}' $TRACKER_HOME/judge/failures.json
{
  "at_start": 1,
  "failed": 1
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
  "failed": 2
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
      "request": "c5e50bf181098c07c1d57b0e460c94fba6c7851cfc6436a795490ecdda8afdac",
      "answer": "no",
      "reason": "The fake says no to works.",
      "dispute": "step calls the helper that does the work, at lib.rs:2."
    },
    {
      "trigger": "T12",
      "ref": "c1:clear",
      "request": "c5e50bf181098c07c1d57b0e460c94fba6c7851cfc6436a795490ecdda8afdac",
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
```

Notes on the output:

- **The request fingerprint** in `held_questions` differs only if the code, the questions or the model do. Both questions share one request, since a criterion without `each` sends one request holding every question, so both entries have the same `request`.
- **The three exit 1 refusals** of version 1 are different checks. The missing reason is caught before anything is read, so no judge is asked. The other two are caught after the judge has answered, from its answers, so the judge is asked once, then answered from the cache. `failed` stays 0 afterwards, since a refused dispute counts nothing, and `at_start` is the request asked at `start`, which never counts.
- **Version 2's run counts** (`failed` is 1): it fails a request not seen before, and a submit that G3 refuses counts. The held submit of version 3 counts too, though its fails were disputed, so `failed` is 2 at the end, short of `max_failures`' default of 3 ([Decisions](#decisions-worth-knowing)).
- **The held output** says "Every gate passed but the disputed judge questions", since the disputed fails are still fails: only the others passed. The model's answer and reason, the agent's reason and the excerpt come from the run that held it. `review` and `show` read the item, which keeps no excerpt, so they print the two reasons only, and `review` points at `tracker gates BM-1` for the excerpt.
- **T12's message** names the refs. For one disputed question it reads "the agent disputed a judge's fail, so a human decides: c1:works answered no".
- **`approve` and `reject` can't be shown going ahead**, since both need a terminal. Typed at one, `approve BM-1` would print what holds BM-1 and each disputed question with its excerpt, ask for the ID, re-run the gates counting each disputed fail as passed, and print "BM-1: review → done". The item would then have `verification.by` of `human`, `verification.held_questions` with the two questions, and the same two in its `approve` history entry. `reject` would put them in its history entry only. History would say `"interactive": true`.
- Nothing in the demo shows a `--json` output. [JSON output](#json-output) says what changed.

## Commands

One flag is new, and three commands show more:

| Command | What changes |
| --- | --- |
| `submit <ID> --dispute <REF> --reason <REASON>` | Disputes a judge's fail by its ref, as the gate output prints it, such as `c2:src/import.js:57:suggests_next_step`. Repeat the pair for each question. If the disputed questions are the only failures, the item goes to review, held under T12, with `held_questions` set. Anything else failing refuses the submit with exit 2, listing the undisputed failures. A ref the judge didn't fail exits 1 |
| `approve <ID>` | Runs T12 on the item's disputed questions. The gates re-run counts each disputed fail as passed, for the answer it disputed. The warning it prints before the question lists each disputed question with its excerpt. The record keeps the held questions, and the history entry too |
| `reject <ID> --reason "…"` | Records the held questions in its history entry, then clears them from the item |
| `review` | Prints each disputed question's ref, the model's answer and reason, and the agent's reason under its hold, and points at `tracker gates <ID>` for the excerpt |
| `show <ID>` | Prints the same under "Held for review" while the item waits, and under the verification record once it's done |
| `submit --help` | Names T12 and `--dispute`, and its usage line shows the pair |

`gates <ID>` takes no `--dispute`, so a dry run shows a disputed fail as an ordinary G3 failure with its excerpt, and doesn't change the item. The exit codes are as in [§3](DESIGN.md#3-interface): a bad dispute exits 1 (a ref the judge didn't fail, a `--dispute` without its `--reason` or the other way round, a ref disputed twice, or a missing or blank reason), and a submit that another gate or an undisputed question refuses exits 2.

The input rules for `--reason` are the ones for a `reject` reason: one printable line, trimmed, at most 500 characters ([`check_reason`](../src/main.rs)). The pairing, the duplicates and the reasons are checked before the store is opened, so a malformed dispute exits 1 whatever the item's state. A ref is trimmed and compared whole, so `c2:src/import.js:57:suggests_next_step` must be typed as printed.

## What held_questions holds

[§4.3](DESIGN.md#43-item-file) lists the field. It's on the item while the item waits in review, and it isn't in the file otherwise: `approve` and `reject` empty it. Each entry has these fields, in this order, and [item.schema.json](../schemas/item.schema.json)'s `held_question` is the schema:

| Field | What it holds |
| --- | --- |
| `trigger` | `T12` for a fail the agent disputed, or `T10` for an unsure answer |
| `ref` | The question's ref, such as `c1:works` |
| `request` | The fingerprint of the request the answer came from. `approve` counts a disputed fail as passed only for this request |
| `answer` | The model's answer: `yes`, `no` or `unsure` |
| `reason` | The model's one-sentence reason. Empty if the judge gave none |
| `dispute` | The agent's reason, one line of at most 500 characters. Only for T12 |

The same shape appears in three more places, all through the one `held_question` definition:

- **`verification.held_questions`**, on a `human` record, once `approve` ran: the disputed questions, and each question that was unsure at the approve's own re-ask.
- **Every `approve` and `reject` history entry's `held_questions`**, which is what `tracker judges` will count. A reopen moves the record to `previous_verification` but leaves history alone, so the decision lasts.
- **`review --json`'s `held[].held_questions`**, left out when the item has none.

The excerpt isn't stored: it can be up to `max_input_bytes`, and the held submit, `approve`'s warning and `gates` each print it from the request they rebuild. `review` and `show` print a question's two reasons and not its excerpt for that reason. An item held for any other reason, such as T6, has no `held_questions`. A T10 hold from an unsure answer does, with `dispute` left out.

## How it works

### Checking a dispute

`submit` in [src/main.rs](../src/main.rs) turns the flags into pairs, then `apply` runs the gates as before. Before the judges run, `apply` notes whether every earlier gate passed (`asked`). A dispute can only be checked against answers, so if an earlier gate failed and the judges weren't asked, the submit is refused for that gate with exit 2 and the disputes are ignored. If the judges were asked, `check_disputes` looks each ref up in `gates.judges`. A fail passes. A pass, an unsure answer or a ref that nobody asked is refused with exit 1, after the lock is taken again and before `count_failures` runs, so nothing is counted for it.

### Taking the disputed fails out

Next `count_failures` counts the run toward T11 ([slice 14](slice-14-readme.md#counting-failed-runs)), as it always did, before the gates are judged. Then `take_disputed` removes each disputed question's G3 failure from the report and returns it as a `HeldQuestion` for T12, with the agent's reason, in the order the agent disputed them. If the report still has a failure, `apply` refuses as usual, so a disputed question never hides another gate's failure, and an undisputed question of the same judge is listed. If it's empty, the triggers run with `disputed` as a new argument, and T12 fires when it isn't empty.

### Holding the item

When any trigger fires, the item goes to review. `apply` sets `holds` as before, and now `held_questions` too: the disputed questions, then each unsure answer as a T10 question (`unsure_questions`). The output starts "Every gate passed but the disputed judge questions" when any question is T12, and `disputed_text` prints each question after the holds. `held_questions` is set only while the item is in review, and a `reject` or an `approve` clears it with `holds`.

### Approving

`approve` in [src/main.rs](../src/main.rs) reads `held_questions`, takes the T12 ones with `disputed_questions`, and runs the triggers with them, so T12 is among the holds it shows and records. `decision_warning` prints each disputed question with its excerpt, from the cache's answer, before the question is asked. After the ID is confirmed it re-runs the gates and the judges as before, and removes each G3 failure whose ref and request match a disputed question, so the disputed answer counts as passed for this approval and nothing else. A question's answer to new work has a new request and isn't removed, so the approve is refused with G3 and the item stays in review. On success `approve` puts the disputed questions, and each question that is unsure now, in `verification.held_questions`, and `record_decision` copies them into the new `approve` history entry.

### Rejecting

`reject` takes `held_questions` out of the item with `std::mem::take`, clears `holds` as before, and `record_decision` puts the questions in the `reject` history entry. Nothing carries the dispute over: a resubmit of unchanged code gets the cache's same answer, a fail, and is refused unless it disputes again.

### T11 and a dispute

`count_failures` runs before `take_disputed`, so a `gates` or `submit` run that fails a new request counts once whether or not the fail is then disputed. An agent normally saw the fail in `gates` first, which counted the run, and the `submit --dispute` that follows repeats a counted request and adds nothing. A refusal with exit 1 happens before `count_failures`, so it counts nothing, and neither does a submit that stops for an earlier gate. T11's unit is still runs, and its code didn't change.

## JSON output

No command or flag has a new output shape, but three shapes gained one optional field. Every output that holds an item gets it through [item.schema.json](../schemas/item.schema.json):

| Output | What's new | Schema |
| --- | --- | --- |
| Any item, such as `show --json`'s `item` | `held_questions`, while it's in review, at least one entry. Left out otherwise | [item.schema.json](../schemas/item.schema.json) `held_questions` and `$defs.held_question` |
| An item's `verification` | `held_questions` for a `human` record, the questions approved. Left out for an `auto` one | [item.schema.json](../schemas/item.schema.json) `verification` |
| An item's `history` entry | `held_questions` on an `approve` or a `reject` that decided some | [item.schema.json](../schemas/item.schema.json) `history` |
| `review --json` | `held[].held_questions`, left out when the item has none, before `diff` | [review.schema.json](../schemas/review.schema.json) |

`submit --json`'s held shape is unchanged: T12 is one more entry in `triggers` and `holds`, with `fired` true, and the questions aren't in it. Read them from the item. Files written by earlier slices still load and come back byte for byte, since each new field has a serde default and is left out when empty. [tests/fixtures/slice-15/](../tests/fixtures/slice-15/) has two files that use the fields.

## Code tour

| File | What's new or changed |
| --- | --- |
| [src/item.rs](../src/item.rs) | `HeldQuestion` (the shape above, `judge_ref` serialised as `ref`), and a `held_questions` field on `Item`, `Verification` and `HistoryEntry`, each with a serde default and left out when empty |
| [src/triggers.rs](../src/triggers.rs) | `t12` (one message for one question, another for several), T12 in `run` (a new `disputed` argument, between T11 and T13) |
| [src/main.rs](../src/main.rs) | `submit` (the pairs and their input rules), `check_disputes`, `take_disputed`, `unsure_questions`, `held_question`, `disputed_questions`, `disputed_text`, `record_decision`, and changes to `apply` (the `disputes` argument, `asked`, the held output), `approve`, `reject`, `decision_warning`, `show`, `verification_text`, `review` (text and JSON), `verification_record` and the `Submit` clap definition with its help text |
| [schemas/item.schema.json](../schemas/item.schema.json) | `held_questions` on the item, the verification record and a history entry, and `$defs.held_question` |
| [schemas/review.schema.json](../schemas/review.schema.json) | `held_questions` on each held item |
| [tests/disputes.rs](../tests/disputes.rs) | New, 574 lines, 18 tests in `tr11_c1` to `tr11_c5`, with the fake judge, `reply`, `started`, `held` and `dispute` helpers |
| [tests/fixtures/slice-15/](../tests/fixtures/slice-15/) | New: XX-25 in review under T12, and XX-26 approved |
| [tests/fixtures.rs](../tests/fixtures.rs) | `slice_15s_fixtures_keep_each_held_question`, which reads both files, and checks their field order, their byte-for-byte round trip and their schema |
| [tests/gates.rs](../tests/gates.rs), [tests/review.rs](../tests/review.rs) and [tests/states.rs](../tests/states.rs) | T12 in the six pinned trigger lists (one, four and one), additions only |
| [docs/DESIGN.md](DESIGN.md) | §2.3.2, §2.5's T11 row, §2.7, §4.3, §5, §6's T11-related row and §7's two bullets on tries |
| [docs/PLAN.md](PLAN.md) | Slice 15's status, `todo` to `in review` (`4dce835`). `d4a4eb2` marked it `in progress` |

To record another kind of held question, give `held_question` the trigger's name and the `Answered` it came from, and add the name to the schema's `trigger` enum. To add another trigger that reads the held questions, pass them to `triggers::run` as `disputed` is, and add it to every pinned trigger list in the three test files above before `start`.

## Testing

Run the test command in `tracker/`. TR-11's criteria run their own tests by prefix, such as `cargo test --test disputes tr11_c1_`, and criteria 6 to 8 are `grep` checks. No test calls `tracker-judge` or the real `claude`: the harness's guard from [slice 14](slice-14-readme.md#testing) puts a failing one first on `PATH`, and each test sets the store's `judge.command` to the fake. The harness ([tests/common/mod.rs](../tests/common/mod.rs)) didn't change.

**The fake judge in `tests/disputes.rs`** is a shell script in `<work>/judge/`, outside the project's root, that saves each request as `request-<n>.json` and prints `reply.json`. `reply` writes the answers from `(id, answer)` pairs, with the reason "The fake says no." for a no and "The fake says <answer>." for the rest. The fixtures are project XX with `src/lib.rs` holding `STUB`, and `work(env, n)`, which commits version `n` of the work, so each version is a different request. `started` is XX-1 started on the stub with a one-question judge, `works`, and `held` is that item held by a dispute of `c1:works`.

**The tests that approve or reject** use `Env::in_terminal_typing`, so they run under `script`, and return early, passing without checking anything, where `script` isn't installed, as slice 9's do. Four of the 18 are such tests: the three in `tr11_c4_` and `tr11_c5_resubmitting_unchanged_code_after_a_reject_needs_the_dispute_again`.

### Coverage of the _Done when_ checks

| _Done when_ | Tests |
| --- | --- |
| A dispute holds the item under T12, showing the excerpt, the model's answer and reason, and the agent's reason. | `tr11_c1_`, 4 tests: `tr11_c1_a_dispute_holds_the_item_under_t12` (result `held`, status `review`, T12 the only trigger fired, the `held_questions` entry's ref, answer, reason and dispute, and T12's place between T11 and T13), `tr11_c1_the_held_submit_shows_the_excerpt_and_both_reasons`, `tr11_c1_review_shows_the_model_s_and_the_agent_s_reasons` (text and `--json`) and `tr11_c1_submit_help_names_t12_and_dispute` |
| Disputing a question that passed or got an unsure answer exits 1. | `tr11_c2_`, 5 tests: `tr11_c2_disputing_a_question_that_passed_exits_1`, `tr11_c2_disputing_an_unsure_answer_exits_1`, `tr11_c2_disputing_a_ref_no_judge_asked_exits_1`, `tr11_c2_a_dispute_refused_with_exit_1_counts_nothing_toward_t11` and `tr11_c2_each_dispute_needs_its_own_reason`. Each refusal leaves the item file byte for byte as it was |
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
| A `--dispute` refused with exit 1 counts nothing toward T11. | `tr11_c2_a_dispute_refused_with_exit_1_counts_nothing_toward_t11`, which reads `judge/failures.json` |
| `held_questions` round-trips, and matches the schema. | `slice_15s_fixtures_keep_each_held_question` in [tests/fixtures.rs](../tests/fixtures.rs), and `Env::json`, which checks each output against its schema |

Not covered by a test:

- **A disputed fail still counts toward T11.** Nothing runs a `submit --dispute` that fails a new request and reads the count after it. The choice is in §2.5's T11 row, and the demo above shows `failed` going up, but no test pins it. A change that moved `take_disputed` before `count_failures` would pass the suite.
- **A T11 hold records no decision for any question.** No test approves or rejects a T11 hold and checks that its history entry has no `held_questions`.
- **A duplicate `--dispute`, a blank or too long `--reason`, and a `--reason` with no `--dispute`.** Only a `--dispute` with no `--reason` is tested. The rest share `check_reason` and the pairing code.
- **`show`'s text for held questions**, in "Held for review" and under a verification record. `review`'s is tested.
- **An approve in which the judge answers a disputed request differently,** such as after the cache was deleted. `approve` matches the ref and the request, not the answer text ([Still open](#still-open)).

**Seen failing first.** All eight of TR-11's checks failed at `start`, at `5e16893`: the `tr11_c1_` to `tr11_c5_` tests exercised a flag and a trigger that didn't exist, and criteria 6 to 8 grep text that `db355e4` added later. The six trigger lists, which needed T12 as soon as the code ran it, were changed in `5e16893`, so they were seen failing too: one in `tests/gates.rs`, four in `tests/review.rs` and one in `tests/states.rs`, each by adding `{ "trigger": "T12", "fired": false }` after T11's entry. They are additions only, which is why T4 stayed quiet. This is slice 14's process 1, now a rule in the agents' brief. `slice_15s_fixtures_keep_each_held_question` was in `5e16893` too and failed until `f5667e4` added the two fixtures, but no criterion runs it: it's in the full suite only.

### Checks done by hand

| What | How | Result |
| --- | --- | --- |
| The demo above | Run line by line with the fake judge, stderr captured, on a temporary `TRACKER_HOME`, with the temporary folder replaced by `$DEMO`. The real `claude` and `tracker-judge` weren't called | The output above. Every refusal exits as marked |
| TR-11's checks, before the approve | The orchestrator checked by hand that the five written criteria are unchanged, that all eight checks failed at `start`, and that every commit since `start` is scoped to TR-11 | All three held |
| TR-11's approve in a real terminal | The user read the diff and approved it | `by: human` at `4dce835`, with `refs/tracker/TR-11` and three holds |

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

- **A disputed fail still counts toward T11.** A dispute is a fail the agent chose to argue, and `count_failures` runs before the dispute is taken out. An agent usually saw the fail in `gates` first, so the run is counted already, and the `submit --dispute` that follows adds nothing. §2.5's T11 row says so. No test pins it ([Testing](#testing)).
- **A `--dispute` refused with exit 1 counts nothing.** The refusal comes before `count_failures`, since nothing was submitted. A submit that stops for an earlier gate counts nothing either, since the judges weren't asked.
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
- **A held submit also records T10's unsure answers as `held_questions`,** so `approve` and `reject` can record them, and `judges` can count them. At `approve` the unsure ones are the answers of its own re-ask, not the stored ones, so a question that's no longer unsure isn't recorded.

## Still open

From the build and the checks. No review of slice 15 has been written, so none of its findings are listed.

**Checks not run:**

- **A terminal `approve` of a held T12 item.** The tests approve and reject under `script` in a throwaway store, and the user approved TR-11, a different hold, from a terminal. Nobody has typed an ID into `approve` for a T12 hold in a real terminal, so what the question prints for one hasn't been read by a person.
- **A real judge.** No run used the real `claude`. The tests and the demo use a fake judge command, so how a real judge's fail reads next to an agent's dispute, and what a disputed question's reason looks like when the model gives none, are unchecked.

**Test gaps** (see [Testing](#testing)): a disputed fail counting toward T11, a T11 hold recording nothing per question, the input rules for duplicate, blank and over-long reasons, `show`'s text for held questions, and a changed answer to a disputed request.

**Not decided:**

- **An approve matches the request, not the answer.** `approve` removes a G3 failure whose ref and request match a held question, and doesn't compare the answer. If the cache entry is deleted and the judge answers the same request with another fail, the approve still counts it as passed. A deleted cache entry is the "forged judge answer" §7 already describes, and the fail is one a human approved, so this is stated here, not closed.
- **T12's message for several questions** reads "the agent disputed judges' fails to 2 questions", which is clumsy. It names the refs, and nothing matches on the wording.

**Deferred items** (from [slice 14's readme](slice-14-readme.md#still-open)): **TR-47** (a `diff` judge's block comments, still open, and a disputed `diff` question's excerpt has the same flaw), **TR-48** (what else a real judge sees) and **TR-50** (escaping `</material>`, and pinning step 1's prompt). All three are deferred.

**Left for slice 16** (the points of slice 14's reviews that this slice doesn't change): [Limits](#limits-and-what-later-slices-change) has them.

**Carried from slice 14** ([its Still open](slice-14-readme.md#still-open)): tries before `start` are free, a judge whose call failed at `start`, and removing `judge/failures.json`, all stated in §7, and the test gaps of `criterion remove` renumbering a judge's entry and `approve` reading T11.

## Limits, and what later slices change

- **Slice 16**'s `judges` counts, for each judge question, how often I approved and how often I rejected the items it held. It reads the `held_questions` of every `approve` and `reject` history entry. Each entry has `ref`, which names a `file:line` for an `each` match, so `judges` must decide how to group a question across items and code that moved: the criterion and question id are inside the ref, and the question text is in the item's criterion. It can only count T10 and T12 decisions. A T11 hold records none, so a question's T11 holds aren't in its counts, and its refs live in the hold's message and in `judge/failures.json`, which each `start` resets ([slice 14's limits](slice-14-readme.md#limits-and-what-later-slices-change)). A reopened item's earlier decisions are in its history, not its record, so `judges` must read history.
- **Slice 16**'s digest lists every `approve` and `reject` I made, and every shelve. A disputed question's reasons are in the `approve` entry's `held_questions` and in `verification.held_questions`, so the digest can show them, but no code does yet. Slice 14's review 2 also wants the digest to make a shelve and a reopen of an item with a judge criterion easy to spot, which has no criterion.
- **Slice 16**'s `audit` doesn't read `held_questions`: it can't tell an item in review that holds one from one that doesn't, and it doesn't read `judge/failures.json`, so a count or a decision edited by hand isn't reported. TR-12's criterion 7 adds PLAN.md's check of `refs/tracker/<ID>`.
- **Saved rulings** are future work ([§7](DESIGN.md#7-honest-limits)). If they're added, the key is the excerpt and the question, so it survives a model change, and a held question's `request` is the wrong key: it changes with the model. A decision would then no longer cover one submit, and §2.3.2's "no saved rulings in v1" and the resubmit rule of `tr11_c5_` change with it.
- **Any change to the request** (the model, a prompt, a question) is a new fingerprint, so a held item's `request` no longer matches the answer `approve` gets, and the approve is refused with G3. A change to the fingerprint's inputs while an item waits in review needs the agent to dispute again.
- **A new trigger** goes into every pinned trigger list in [tests/gates.rs](../tests/gates.rs), [tests/review.rs](../tests/review.rs) and [tests/states.rs](../tests/states.rs), and into `triggers::run`'s order, as T12 did. Add it in the item's first test commit, so it's seen failing. `triggers::run` has five arguments now, and every caller passes `&[]` for the ones it hasn't got, as `gates_dry_run` does for `disputed`.
- **A new place that moves an item out of review** must clear `held_questions` with `holds`, as `approve` and `reject` do. They are present only while the item is in review, and `record_decision` is the one place their decision is recorded.
- **TR-47 and a `diff` judge.** Until it's done, a `diff` judge sees comment text that §2.3.1 says it doesn't, and a dispute of one is about an excerpt with that flaw. A project shouldn't add one before then.
- **Interrupting `tracker`.** `submit --dispute` asks the judges as `submit` does, so a Ctrl-C during it can leave the judge command and its `claude` running, as §7's interrupt bullet says for every command that runs judges.

# Slice 16: Digest, audit, judge report

`tracker` is a command-line work tracker, written in Rust, that keeps one central list of work items across many projects and decides when an item is really done. This document covers **slice 16**, the last of the 16 in [PLAN.md](PLAN.md): what it built, how it works, and how to test it. The earlier readmes cover the rest: [slice 1](slice-1-readme.md) the store, `create` and `list`, [slice 2](slice-2-readme.md) the states and `history`, [slice 3](slice-3-readme.md) priority, kind, labels, `edit` and `list`'s filters, [slice 4](slice-4-readme.md) `show` and `delete`, [slice 5](slice-5-readme.md) bad data and polish, [slice 6](slice-6-readme.md) criteria and gates G1, G2 and G6, [slice 7](slice-7-readme.md) running checks, with gates G3 and G4 and `started.checks`, [slice 8](slice-8-readme.md) G5, the `auto` record and `refs/tracker/<ID>`, [slice 9](slice-9-readme.md) the inbox, `approve`, `reject` and triggers T6 and T9, [slice 10](slice-10-readme.md) triggers T1, T2, T3, T4, T7 and T8, [slice 11](slice-11-readme.md) triggers T5, T13 and T14 and the criteria fingerprint, [slice 12](slice-12-readme.md) judge criteria, G3's judge half and trigger T10, with a fake judge command, [slice 13](slice-13-readme.md) `tracker-judge`, the real judge command, [slice 14](slice-14-readme.md) judges that must fail first, trigger T11 and `judge/failures.json`, and [slice 15](slice-15-readme.md) disputes, trigger T12 and `held_questions`.

After slice 16 you can **audit what the tool verified without you**. `digest` lists, for each project, every item auto-verified since you last acknowledged it, and every shelve and human action. `digest ack` acknowledges one from a terminal, `review` flags a digest that has waited too long, `audit` reports signs that the store was edited by hand, and `judges` shows how you've decided each judge question. The spec is [DESIGN.md](DESIGN.md). This slice implements [§2.6](DESIGN.md#26-digest-auditing-the-auto-verifications) (the digest), `digest ack` in [§2.7](DESIGN.md#27-human-actions), `tracker judges` in [§2.3.2](DESIGN.md#232-when-a-judge-is-wrong), and `digest`, `audit` and `judges` in [§3](DESIGN.md#3-interface). It also changes [§6](DESIGN.md#6-what-im-trusting-and-what-checks-it) and [§7](DESIGN.md#7-honest-limits) to say what these commands catch and don't. The slice is tracked in `tracker` itself as item TR-12, with the binary installed from slice 15 ([Tracking TR-12](#tracking-tr-12)). There's no review of it yet, so this document has no "Fixes after review" section.

| | |
| --- | --- |
| Status | Every _Done when_ check has a test, and every test passes, as does the test command (`cargo fmt --check`, `cargo clippy -- -D warnings` and `cargo test` were run on the finished branch, at `93637e2`, whose code is `ccb8d13`'s). TR-12 is `done`, but it wasn't auto-verified. It has eleven criteria: 1 to 6 are PLAN.md's _Done when_ bullets as written, which TR-12 was created with, and 7 is PLAN.md's later bullet on `audit` checking `refs/tracker/<ID>`, which was added to PLAN.md after TR-12 was created and which the orchestrator added to the item before `start`, 8 and 9 are DESIGN.md wording the orchestrator added before `start` from [slice 15's review 2](reviews/slice-15-review-2.md) (design 1 and design 3), and 10 and 11 are the builder's, added before `start`: an overdue digest in `review`, and `--help` text and empty states. All eleven checks failed at `start`, at `a877beb`. Criterion 9's test, `tr12_c9_a_dispute_beside_a_missing_input_is_refused_for_g3`, passed before the work, since the code was already right, so its check failed through its `grep` of DESIGN.md. The installed slice 15 binary held the submit under T1 (it's a capability), T7 (2595 lines changed) and T8 (DESIGN.md changed). T4 stayed quiet: the only existing test file changed is `tests/common/mod.rs`, which gained four match arms and lost nothing. The user read the diff and approved it from a terminal: `verification.by` is `human`, at `93637e2`, with `refs/tracker/TR-12`, and the three holds are recorded in `verification.triggers`. Before the approve, the orchestrator checked by hand that the six written criteria are unchanged, that all eleven checks failed at `start`, and that every commit since `start` is scoped to TR-12. **Not checked by hand yet:** one `tracker digest ack` typed in a real terminal (typing `ack` acknowledges, and typing the project key cancels), and reading `tracker digest` and `tracker judges` on the real store. Both wait for slice 16's binary to be installed ([Still open](#still-open)). PLAN.md has slice 16 as `in review`. The branch isn't merged into `main`, and neither are slices 7 to 15, which it's stacked on |
| Branch | `feat/tracker/slice-16`, stacked on `feat/tracker/slice-15` at `2468127`. Slice 15 isn't merged into `main`, so this slice's commits are `git log --oneline --reverse feat/tracker/slice-15..feat/tracker/slice-16`. In order, all scoped to TR-12: `a877beb` (the tests and schemas, committed before `tracker start TR-12`), `8b8670c` (PLAN.md: slice 16 in progress), `ccb8d13` (the code: `src/audit.rs`, `src/decisions.rs` and `src/digest.rs`), `d997b33` (DESIGN.md) and `93637e2` (PLAN.md: slice 16 in review). That is 5 commits, all TR-12's. The commit that adds this readme belongs to no item |
| Test command | `cargo fmt --check && cargo clippy -- -D warnings && cargo test`, run in `tracker/`. All three pass at `93637e2` |
| Tests | 559: 46 in `tests/risky_changes.rs`, 42 in `tests/judges.rs`, 39 in `tests/create_and_list.rs`, 36 in `tests/commits.rs`, 34 in `tests/organise.rs`, 29 in `tests/judges_first.rs`, 29 in `tests/disputes.rs`, 28 in `tests/review.rs`, 27 in `tests/states.rs`, 25 in `tests/show_and_delete.rs`, 23 in `tests/criteria.rs`, 22 in `tests/gates.rs`, 21 in `tests/tampered_proof.rs`, 18 in `tests/checks.rs`, 16 in `tests/fixtures.rs`, 14 in `tests/item_files.rs`, 13 in `tests/bad_data.rs`, 12 in `tests/tracker_judge.rs`, 11 in `tests/docs_links.rs`, 10 in `tests/digest.rs`, 9 in `tests/polish.rs`, 8 in `tests/audit.rs`, 6 in `tests/output.rs`, 6 in `tests/t5_and_fingerprint.rs`, 5 in `tests/schemas.rs`, 4 in `tests/store_writes.rs`, 4 in `tests/judge_report.rs`, 3 in `tests/refusal_wording.rs`, 2 in `tests/item_ids.rs`, 1 each in `tests/harness_check.rs`, `tests/judge_add_message.rs` and `tests/submit_help.rs`, 12 unit tests in the library (4 in `src/gates.rs`, 4 in `src/judge.rs`, 2 in `src/git.rs`, 1 each in `src/store.rs` and `src/checks.rs`) and 2 in `src/main.rs`. That is 22 more than slice 15's 537, all in the three new files: 10 in `tests/digest.rs`, 8 in `tests/audit.rs` and 4 in `tests/judge_report.rs`. No unit test was added |

## Contents

- [What's in this slice](#whats-in-this-slice)
- [Try it](#try-it)
- [Commands](#commands)
- [What digests.json holds](#what-digestsjson-holds)
- [How it works](#how-it-works)
- [JSON output](#json-output)
- [Code tour](#code-tour)
- [Testing](#testing)
- [Decisions worth knowing](#decisions-worth-knowing)
- [Still open](#still-open)
- [Limits, and what later slices change](#limits-and-what-later-slices-change)

## What's in this slice

**Built:**

- **`digest [--project KEY] [--all]`.** Each project's digest lists every item auto-verified since that project's last ack, the ones a judge passed first, with their criteria, each judge answer (ref, answer, reason) and each check that ran. Then, oldest first, it lists every shelve, `approve`, `reject`, `criterion accept` and `delete` since the ack, mine included, and every `edit` whose history entry shows it removed `needs-review` or changed kind away from `capability`. Each line says whether it came from an interactive terminal. Inside a project root it shows that project's digest, and outside every root, or with `--all`, every project's.
- **`digest ack [--project KEY]`.** From a terminal only, it shows the project's digest, asks for `ack` to be typed (not an ID), then, under the lock, builds the digest again. If it differs from the one it showed, it records nothing and exits 1. Otherwise it writes the project's entry in `digests.json` ([What digests.json holds](#what-digestsjson-holds)). Outside every root it needs `--project`, so one command can't acknowledge every project.
- **`review` flags an overdue digest.** A project's digest whose oldest entry is at least `review.digest.every_days` old (7 by default) is listed under the inbox, with the command to read it and to acknowledge it. An empty digest is never overdue.
- **`audit`.** It covers every project and reports a status that doesn't match the last move in the item's history, a done item with no verification record, a done `auto` or `human` item whose `refs/tracker/<ID>` is missing or doesn't point at the record's `commit`, holds or held questions on an item that isn't in review (and a T12 hold with no disputed question, or the other way round), an ID used twice, and a `deleted.json` line whose ID is at or above its project's counter. It changes nothing and exits 0 whatever it finds. It doesn't read `judge/` ([§7](DESIGN.md#7-honest-limits)).
- **`judges`.** For each judge question, grouped by its `ask` within a project, it shows how often you approved and how often you rejected the items it held, split by the held question's trigger and answer: a disputed fail (T12), an unsure answer (T10), or a pass held because the project sets `"auto_verify": false` (T10). It reads the `held_questions` of every `approve` and `reject` in every item's history, and counts each decision once per question, trigger and answer, however many `each` matches it held.
- **`--help` text and empty states** for `review`, `approve`, `reject`, `digest`, `digest ack`, `audit` and `judges`, to slice 5's standard. `digest`, `audit` and `judges` print an empty state when they've nothing to report.
- **DESIGN.md says it.** §2.6 says when a digest is overdue, §2.3.2 says `judges` splits its counts and what it reads, §3's `audit` and `judges` rows say what each covers, §6's two rows on vague judge questions and wrong disputes say `judges` shows them, §7 says what `audit` and the digest don't see, and §2.3.2 says any G3 failure without a judge answer skips the dispute check ([Decisions](#decisions-worth-knowing)).
- **Slice 15's "Slice 16 must decide" points**, from [review 1](reviews/slice-15-review-1.md#slice-16-must-decide) and [review 2](reviews/slice-15-review-2.md#slice-16-must-decide): see [Decisions](#decisions-worth-knowing).

**Not built yet:**

- **Nothing in PLAN.md.** Slice 16 is the last slice. [Limits](#limits-and-what-later-slices-change) lists what's left open across the plan.
- **`audit` of `judge/failures.json` and `judge/cache/`.** It doesn't read either, so a T11 count or a cached answer changed by hand isn't reported ([§7](DESIGN.md#7-honest-limits)).
- **A digest entry for a refused dispute or a `gates` run.** Their only trace is T11's count in `judge/failures.json`, which `start` empties.
- **Saved rulings, mutation testing, and a branch or worktree per item** (PLAN.md's "Deferred / out of scope", [§7](DESIGN.md#7-honest-limits)).
- **TR-39, TR-44, TR-47, TR-48 and TR-50**, all open ([Still open](#still-open)).

## Try it

Always set `TRACKER_HOME` while developing. Without it, the tool uses your real store at `~/.tracker`.

**This demo never calls the real `claude` or `tracker-judge`.** Its judge command is a shell script that reads the request and prints whatever is in `reply.json`, as `tests/judge_report.rs` does, and a small `reply` script writes that file, as slice 15's demo does. The project BM is a folder in one throwaway repository. It has four items: BM-1 has a judge criterion and is auto-verified on a judge's pass, BM-2 is auto-verified on a script check, BM-3 has its kind changed from `capability` to `change`, and BM-4 is shelved. All of them are changed from piped stdin, so the digest marks each one "not from a terminal". The demo then shows what `digest ack`, `audit`, `judges` and `review` say, with each refusal, and ends with two hand edits to the throwaway store that `audit` finds.

- **`digest ack` and `judges` with decisions can't be shown going ahead.** `digest ack` needs a terminal, and `judges` only counts decisions made at one (`approve` and `reject`), so the demo shows `judges`' empty state and its refusals only. [Testing](#testing) says which tests run them under `script`. Typed at a terminal, `digest ack --project BM` would print the digest above, then "Acknowledging it records that you've read BM's digest…", ask for `ack`, and print "Acknowledged BM's digest at <time>: 2 items auto-verified and 2 actions."
- **The overdue digest is made by hand.** The demo sets BM's `review.digest.every_days` to 0 in the throwaway config, so a digest with entries is overdue straight away. At the default of 7 days the line wouldn't appear until the oldest entry was a week old.

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
printf '%s\n' '{"text":"The code does the work","inputs":["src/lib.rs"],"ask":[{"id":"works","ask":"Does this code do the work?"}]}' > $DEMO/judge/lib.json
cd $DEMO/repo && git init --quiet -b main
mkdir -p bm/src && printf 'fn run() {\n    todo!()\n}\n' > bm/src/lib.rs
git add -A && git commit --quiet -m "chore: set up"
tracker init
jq --arg c "$DEMO/judge/fake" '.judge = {command: $c, model: "claude-haiku-5-5"}' $TRACKER_HOME/config.json > $DEMO/config.new && mv $DEMO/config.new $TRACKER_HOME/config.json
tracker project add BM --name bm --root $DEMO/repo/bm
```

The demo itself, from the same shell. It was run line by line, as one script, with stdout and stderr together. A `$ ` line is the command, and `[exit N]` follows each one that fails:

```sh
tracker digest
tracker audit
tracker judges
tracker create "Say hello" --project BM
tracker criterion add BM-1 --judge $DEMO/judge/lib.json
cd $DEMO/repo
reply works=no
tracker start BM-1
printf 'fn run() {\n    step(1);\n}\n' > bm/src/lib.rs
git add -A && git commit --quiet -m "feat(BM-1): do the work"
reply works=yes
tracker submit BM-1
tracker create "Write notes" --project BM
tracker criterion add BM-2 "There are notes" --check "test -f NOTES.md"
tracker start BM-2
printf 'Notes\n' > bm/NOTES.md
git add -A && git commit --quiet -m "feat(BM-2): add notes"
tracker submit BM-2
tracker create "Try a thing" --project BM --kind capability
tracker edit BM-3 --kind change
tracker create "Park it" --project BM
tracker shelve BM-4 --reason "Not needed yet."
tracker digest
echo ack | tracker digest ack --project BM      # exit 1: needs a terminal
tracker digest ack --project NOPE      # exit 1: no such project
tracker digest --project BM --all      # exit 1: --project and --all conflict
tracker judges
jq '.projects.BM.review = {digest: {every_days: 0}}' $TRACKER_HOME/config.json > $DEMO/config.new && mv $DEMO/config.new $TRACKER_HOME/config.json
tracker review
tracker audit
jq '.status = "done"' $TRACKER_HOME/items/BM-4.json > $DEMO/item.new && mv $DEMO/item.new $TRACKER_HOME/items/BM-4.json
printf '{"id":"BM-9","title":"Forged","status":"open","at":"2026-11-23T00:00:00Z","interactive":false}\n' >> $TRACKER_HOME/deleted.json
tracker audit
tracker digest ack --help | sed -n 1p
tracker list --all --status open,in-progress,review,done,shelved
```

The output:

```
$ tracker digest
BM's digest is empty: nothing has been auto-verified, shelved or done by hand yet.
$ tracker audit
No problems found in 0 items, counters.json and deleted.json.
$ tracker judges
No decisions on judge questions yet: no item a judge held under T10 or T12 has been approved or rejected.
$ tracker create "Say hello" --project BM
Created BM-1: Say hello
$ tracker criterion add BM-1 --judge $DEMO/judge/lib.json
Added criterion 1 to BM-1, a judge: The code does the work
Its question is asked at start, and at submit once every other gate passes.
$ cd $DEMO/repo
$ reply works=no
$ tracker start BM-1
Asking criterion 1's judge 1 question: `$DEMO/judge/fake`
BM-1: open → in-progress
$ printf 'fn run() {\n    step(1);\n}\n' > bm/src/lib.rs
$ git add -A && git commit --quiet -m "feat(BM-1): do the work"
$ reply works=yes
$ tracker submit BM-1
Asking criterion 1's judge 1 question: `$DEMO/judge/fake`
BM-1: in-progress → done
Verified automatically: every gate passed at 4d629e9, and refs/tracker/BM-1 keeps that commit.
$ tracker create "Write notes" --project BM
Created BM-2: Write notes
$ tracker criterion add BM-2 "There are notes" --check "test -f NOTES.md"
Added criterion 1 to BM-2: There are notes
$ tracker start BM-2
Running criterion 1's check: `test -f NOTES.md`
BM-2: open → in-progress
$ printf 'Notes\n' > bm/NOTES.md
$ git add -A && git commit --quiet -m "feat(BM-2): add notes"
$ tracker submit BM-2
Running criterion 1's check: `test -f NOTES.md`
BM-2: in-progress → done
Verified automatically: every gate passed at 28001e1, and refs/tracker/BM-2 keeps that commit.
$ tracker create "Try a thing" --project BM --kind capability
Created BM-3: Try a thing
$ tracker edit BM-3 --kind change
Edited BM-3: changed kind.
$ tracker create "Park it" --project BM
Created BM-4: Park it
$ tracker shelve BM-4 --reason "Not needed yet."
BM-4: open → shelved
$ tracker digest
BM's digest, never acknowledged
Auto-verified (2), the ones a judge passed first:
  BM-1  Say hello
    verified at 4d629e9 on 2026-11-23T23:01:33Z
    1. The code does the work
    judge c1:works answered yes: "The fake says yes to works."
  BM-2  Write notes
    verified at 28001e1 on 2026-11-23T23:01:34Z
    1. There are notes
    criterion 1's check passed: `test -f NOTES.md` (0s)
Shelves, human actions and edits that switch off a trigger (2), oldest first. Check that you made each one marked from a terminal:
  2026-11-23T23:01:34Z  BM-3  Try a thing
    edit changed kind away from capability, not from a terminal
  2026-11-23T23:01:34Z  BM-4  Park it
    shelve, not from a terminal: "Not needed yet."
Reopen anything that looks wrong with `tracker reopen <ID> --reason "…"`. Once you've read it all, run `tracker digest ack --project BM`.
$ echo ack | tracker digest ack --project BM      # exit 1: needs a terminal
error: `tracker digest ack --project BM` needs a human at an interactive terminal, who types ack to confirm it, and stdin here isn't a terminal. Nothing was changed. Only a human acknowledges a digest, after reading it with `tracker digest --project BM`.
[exit 1]
$ tracker digest ack --project NOPE      # exit 1: no such project
error: There's no project NOPE. Registered projects: BM.
[exit 1]
$ tracker digest --project BM --all      # exit 1: --project and --all conflict
error: the argument '--project <KEY>' cannot be used with '--all'

Usage: tracker digest --project <KEY>

For more information, try '--help'.
[exit 1]
$ tracker judges
No decisions on judge questions yet: no item a judge held under T10 or T12 has been approved or rejected.
$ jq '.projects.BM.review = {digest: {every_days: 0}}' $TRACKER_HOME/config.json > $DEMO/config.new && mv $DEMO/config.new $TRACKER_HOME/config.json
$ tracker review
No items are held for review.

BM's digest is overdue: 4 entries, the oldest from 2026-11-23T23:01:33Z, never acknowledged. Read it with `tracker digest --project BM`, then run `tracker digest ack --project BM`.
$ tracker audit
No problems found in 4 items, counters.json and deleted.json.
$ jq '.status = "done"' $TRACKER_HOME/items/BM-4.json > $DEMO/item.new && mv $DEMO/item.new $TRACKER_HOME/items/BM-4.json
$ printf '{"id":"BM-9","title":"Forged","status":"open","at":"2026-11-23T00:00:00Z","interactive":false}\n' >> $TRACKER_HOME/deleted.json
$ tracker audit
3 problems found in 4 items, counters.json and deleted.json, each a sign that the store was edited by hand:
  BM-4  BM-4 is done, but the last move in its history left it shelved, so its status was edited by hand.
  BM-4  BM-4 is done but has no verification record, so nothing says how it reached done.
  BM-9  Line 1 of deleted.json deletes BM-9, but counters.json's next BM number is 5, so BM-9 was never handed out and the line was written by hand. `tracker create` refuses BM until its counter in counters.json is above 9.
$ tracker digest ack --help | sed -n 1p
Record that you've read a project's digest. Only from an interactive terminal
$ tracker list --all --status open,in-progress,review,done,shelved
  ID    PRIORITY  KIND    STATUS  TITLE
BM  bm
  BM-1  medium    change  done    Say hello
  BM-2  medium    change  done    Write notes
  BM-3  medium    change  open    Try a thing
  BM-4  medium    change  done    Park it

Every project.
Filters: status open, in-progress, review, done or shelved.
```

Notes on the output:

- **The first three commands are the empty states.** BM has no items, so `digest`, `audit` and `judges` each say so in one line. `audit` counts the items it read, and `judges` says which holds record questions.
- **The digest lists BM-1 before BM-2,** because a judge passed it, though BM-2 was verified a moment later. Each item shows its commit, time and criteria, then each judge answer (`c1:works`) or check that ran. The actions come after, oldest first, and each says "not from a terminal". Items and actions with the same second keep the items' order, since `digest::build` sorts stably.
- **BM-3's line is the edit.** The digest lists an `edit` only if its history entry shows `needs-review` was removed or kind was changed away from `capability`. An edit that only adds a label isn't listed.
- **The three refusals are three kinds of error.** A piped stdin is refused by the shared confirmation (exit 1, nothing changed). An unknown `--project` exits 1 and names the registered ones. `--project` with `--all` is clap's own error, which the tool also exits 1 for ([§3](DESIGN.md#3-interface)).
- **`review` has no held item but flags BM.** With every item auto-verified, nothing is held, so it prints "No items are held for review." and then the overdue digest, with its entry count, its oldest entry and the two commands. It prints "Nothing needs you." only when there's neither.
- **`audit` found three things from two edits.** Setting BM-4's `status` to `done` by hand breaks two checks: the history's last move left it `shelved`, and a done item needs a verification record. The line appended to `deleted.json` names BM-9, which was never handed out: counters.json's next BM number is 5. The message says what `create` does about it. The `audit` before the edits found nothing. It exits 0 either way.
- **`--help`.** `digest ack --help` starts with its one-line summary. [Commands](#commands) lists each command's.
- **Nothing in the demo shows a `--json` output.** [JSON output](#json-output) says what each prints.

## Commands

| Command | What it does |
| --- | --- |
| `digest [--project KEY] [--all]` | Prints each covered project's digest: auto-verified items since its last ack, judge-passed first, then shelves, human actions and trigger-switching edits, oldest first. Inside a project root it covers that project, and outside every root, or with `--all`, every project, as `list` picks. `--project` and `--all` conflict. A project with nothing to list says its digest is empty |
| `digest ack [--project KEY]` | Interactive only. Shows the digest, asks for `ack`, then builds it again under the lock, and records nothing, with exit 1, if it changed. Otherwise writes the project's entry in `digests.json` and prints how many items and actions it covered. Outside every root it needs `--project` |
| `review` | Also lists each project whose digest is overdue, after the held items, and says "No items are held for review." when only a digest is overdue |
| `audit` | Reports signs that the store was edited by hand, across every project. Changes nothing, and exits 0 whatever it finds |
| `judges` | Shows, for each judge question, the approve and reject counts, split by the held question's trigger and answer |
| `approve`, `reject`, `review`, `digest`, `digest ack`, `audit`, `judges` `--help` | Each has help text to slice 5's standard: a one-line summary, a longer description and each flag's help, with `--json` documented |

`audit` and `judges` have no flags besides `--json`. `digest ack`'s confirmation is the one slice 4 built ([`tty::confirm`](../src/tty.rs)), given `ack` as the word to type, so it behaves as `approve`'s does: the key can be typed in either case, and anything else, an empty line or the end of input cancels with exit 1. The warning and question go to stderr, so `--json` output stays one object. The exit codes are as in [§3](DESIGN.md#3-interface): a refused `digest ack` (no terminal, a different word, an unknown or missing project, or a digest that changed while it asked) exits 1, a missing or unreadable store exits 3, and `audit`'s findings exit 0. A `digests.json` that can't be parsed exits 3 with a message that says what it holds and that only `digest ack` writes it.

## What digests.json holds

[§4.1](DESIGN.md#41-layout) lists the file. It's in the store's folder, `<TRACKER_HOME>/digests.json`, and the first `digest ack` creates it. It maps each project key to its last ack, and [digest-ack.schema.json](../schemas/digest-ack.schema.json) describes what `digest ack --json` prints, which has the same four values:

| Field | What it holds |
| --- | --- |
| `at` | When the digest was acknowledged. The next digest lists only what's after it |
| `interactive` | Whether the ack came from a terminal |
| `verified` | How many auto-verified items the acknowledged digest listed |
| `actions` | How many shelves, human actions and edits it listed |

Removing a project's entry lists everything again. A project with no entry has never been acknowledged, and its digest says so. Times are kept to the second, and an entry is "since the ack" if it's strictly after `at`, so `digest ack` holds the lock until the clock has passed `at`. Without that wait, an item auto-verified in the same second after the ack would be missed by the next digest.

## How it works

### Building a digest

`digest::build` in [src/digest.rs](../src/digest.rs) takes one project key, every item, the readable lines of `deleted.json` (`deleted_records`) and the project's last `Ack`, and returns a `Digest` with no I/O of its own. An item is listed as verified if it's `done`, its record is `auto` and the record's time is after the ack. `judged` is true when the record has any judge entry. Each history entry after the ack that is a `shelve`, `approve`, `reject` or `criterion-accept`, or an `edit` with `needs-review` in `labels_removed` or a `kind_before` of `capability`, becomes an `Action`, with the entry's `interactive`, reason and `held_questions`. A shelve of an item with a judge criterion is marked `judged`, since starting it again empties its T11 count ([§7](DESIGN.md#7-honest-limits)), so a shelve and a later reopen and `start` is worth a look. Each `deleted.json` line for the project that has an ID and a time, and a time after the ack, becomes a `delete` action, with its `interactive` and `status`. A line that can't be read as a delete record is left out here, and `audit` reports it. Verified items are sorted judge-passed first, then oldest first. Actions are sorted oldest first. The sorts are stable, so entries in the same second keep the items' order.

`digest` and `digest_text` in [src/main.rs](../src/main.rs) print it. Text from an agent, such as a title, a criterion, a reason or a judge's reason, goes through `text::for_terminal`, since it can hold control characters. A project's digest is built from all its items, so an item file that can't be read is skipped and the output says how many were skipped ([slice 5](slice-5-readme.md)).

### Acknowledging a digest

`digest_ack` in [src/main.rs](../src/main.rs) picks the project (`--project`, or the current folder's, or an error that names the registered ones), builds the digest (`project_digest`), and calls `tty::confirm` with the word `ack`. That refuses without a terminal. The digest is shown in the warning, so what's acknowledged is what was read. After the word is typed, it takes the store's lock and builds the digest again. `Digest` derives `PartialEq`, so any difference at all, such as an item auto-verified while the question waited, makes it return an error that records nothing and exits 1, and says to run the command again. If they match, it writes the project's entry (`write_acks`, atomically, like every store write), then waits until the clock passes the ack's second ([What digests.json holds](#what-digestsjson-holds)) before it releases the lock. This is the check `Store::lock_unchanged` makes for an item, but over a project's digest, since there's no item file to read again.

### The overdue flag

`overdue_digests` in [src/main.rs](../src/main.rs) builds every project's digest, and `digest::is_overdue` compares the oldest entry's time (`Digest::oldest`) with now: overdue means at least `every_days` days (86,400 seconds each) old. An empty digest has no oldest entry and is never overdue. `Project::digest_every_days` in [src/config.rs](../src/config.rs) reads `review.digest.every_days`, which was stored but unread until now, and defaults to 7 (`DEFAULT_DIGEST_EVERY_DAYS`). `review` prints each overdue digest after the held items, and `review --json` has `overdue_digests` when any is ([JSON output](#json-output)). The flag doesn't hold anything or change an exit code: `review` is an inbox, and it's the human who acknowledges.

### Auditing the store

`audit::run` in [src/audit.rs](../src/audit.rs) takes the items, the text of `deleted.json`, the counters and the config, and returns the problems, items first by project and number, then `deleted.json`'s lines in order. `check_item` checks each item:

- **`status`:** the last history entry's `to` (or `open`, for an item with no history) must equal the status.
- **`verification`:** a done item needs a record.
- **`ref`:** a done item whose record is `auto` or `human` needs `refs/tracker/<ID>` to exist and to point at the record's `commit` (`ref_problem`, using `git::ref_target`, which resolves the ref to a full SHA, or `None` if there's none). A record that names no commit, or a project root that can't be found, is reported as unable to be checked. An `unchecked` record, from slice 2, has no ref.
- **`held`:** `holds` and `held_questions` are there only while an item is in review ([§4.3](DESIGN.md#43-item-file)), and a T12 hold comes with its disputed questions, so an item outside review with either, a T12 hold with no T12 question, or a T12 question with no T12 hold is reported. `audit` doesn't check that each disputed question's request matches an answer, since that needs the judge cache.

`check_deleted` goes through `deleted.json` line by line: a line that can't be read as a record with a valid ID, an ID that's also an item, an ID deleted on two lines, and a line whose ID number is at or above its project's counter in `counters.json`, which only a hand-written line can be ([slice 4's review 1](reviews/slice-4-review-1.md), finding 7). `audit` in [src/main.rs](../src/main.rs) loads everything, prints the problems, one line each, and then says how many item files were skipped. It reads `judge/` nowhere, and says so in [§7](DESIGN.md#7-honest-limits).

### Counting decisions

`decisions::tally` in [src/decisions.rs](../src/decisions.rs) walks every item's history. Each `approve` or `reject` entry's `held_questions` are counted, once per question, trigger and answer in that entry (`counted`), since an `each` judge holds one entry per match and the decision was one. A question is identified by its project and its `ask`. An entry written before TR-53 has no `ask`, so it's grouped by the question's id, taken from the end of its `ref`, after a character that no text starts with, so it can't join a question that has its text. Each question has a `held` list, one entry for each trigger and answer, with the approved and rejected counts and `held_as`, from `held_as`: `disputed` for T12, `unsure` for a T10 unsure answer, and `passed` for the T10 answers held under `auto_verify: false`. Approving a disputed fail overrules the judge, and approving a pass agrees with it, so the counts are never added together. It reads history and not the verification record, since a reopen moves the record to `previous_verification` and leaves history alone. A T11 hold records no question, so it isn't counted ([§2.7](DESIGN.md#27-human-actions)).

`judges` in [src/main.rs](../src/main.rs) prints each question's `ask` through `text::for_terminal`, since a judge file's text comes from an agent.

## JSON output

Four commands are new, and one gained a field:

| Command | Output | Schema |
| --- | --- | --- |
| `digest --json` | `{"project": <key or null>, "digests": [...], "skipped": [...]}`. Each digest has `project`, `acknowledged` (a time, or null), `verified` (each with `id`, `title`, `judged`, `criteria` and the item's `verification` record) and `actions` (each with `id`, `title`, `action`, `at`, `interactive`, and, when they apply, `reason`, `status`, `judged`, `labels_removed`, `kind_before` and `held_questions`). `project` is the key it covered, or null for every project | [digest.schema.json](../schemas/digest.schema.json) |
| `digest ack --json` | `{"project", "acknowledged", "verified", "actions"}` | [digest-ack.schema.json](../schemas/digest-ack.schema.json) |
| `audit --json` | `{"items": <how many it read>, "problems": [...], "skipped": [...]}`. Each problem has `problem` (`status`, `verification`, `ref`, `held`, `id` or `deleted`), `message`, and the item's `id` when it names one | [audit.schema.json](../schemas/audit.schema.json) |
| `judges --json` | `{"questions": [...], "skipped": [...]}`. Each question has `project`, `ask` (left out for decisions recorded before TR-53), `question` (the id) and `held`: for each trigger and answer, `trigger`, `answer`, `held_as`, `approved` and `rejected` | [judges.schema.json](../schemas/judges.schema.json) |
| `review --json` | `overdue_digests`: for each project whose digest is overdue, `project`, `entries`, `oldest` and `acknowledged`. Left out when none is, so slice 9's shape is unchanged | [review.schema.json](../schemas/review.schema.json) |

Every output holding an item still gets its shape from [item.schema.json](../schemas/item.schema.json), which didn't change. `tests/common/mod.rs`'s `schema_for` knows the four new commands, and `Env::json` checks each output against its schema. A later field on a digest entry or a question's counts can be added without breaking these, since the schemas are open to a reader that ignores what it doesn't know, but a reader that wants a new count must read the schema.

## Code tour

| File | What's new or changed |
| --- | --- |
| [src/digest.rs](../src/digest.rs) | New, 240 lines. `Ack`, `Digest` (`entries`, `oldest`), `Verified`, `Action`, `read_acks` and `write_acks` for `digests.json`, `deleted_records`, `key_of`, `build` and `is_overdue` |
| [src/audit.rs](../src/audit.rs) | New, 220 lines. `Problem`, `run`, `check_item`, `ref_problem` and `check_deleted` |
| [src/decisions.rs](../src/decisions.rs) | New, 125 lines. `Question`, `Held`, `held_as` and `tally` |
| [src/main.rs](../src/main.rs) | `Digest`, `Audit` and `Judges` commands, `DigestCommand::Ack` and their help text, `digest`, `digest_text`, `verified_text`, `action_text`, `project_digest`, `digest_ack`, `audit`, `judges`, `known_project`, `current_dir`, `plural`, `overdue_digests`, and `review` (the overdue digests, in text and JSON) |
| [src/config.rs](../src/config.rs) | `DEFAULT_DIGEST_EVERY_DAYS` and `Project::digest_every_days` |
| [src/git.rs](../src/git.rs) | `ref_target`: the commit `refs/tracker/<ID>` points at, or `None` |
| [src/store.rs](../src/store.rs) | `digests_path`, and `read_counters` and `read_deleted` made public for `audit` |
| [src/tty.rs](../src/tty.rs) | One doc comment: `digest ack` is no longer "later" |
| [src/lib.rs](../src/lib.rs) | The three new modules |
| [schemas/digest.schema.json](../schemas/digest.schema.json), [digest-ack.schema.json](../schemas/digest-ack.schema.json), [audit.schema.json](../schemas/audit.schema.json) and [judges.schema.json](../schemas/judges.schema.json) | New, one for each command |
| [schemas/review.schema.json](../schemas/review.schema.json) | `overdue_digests` |
| [tests/digest.rs](../tests/digest.rs) | New, 564 lines, 10 tests: `tr12_c1_`, `tr12_c2_`, `tr12_c3_`, `tr12_c10_` and `tr12_c11_`, with a fake judge, `reply`, `two_projects`, `auto_verified`, `judge_verified` and `ack` helpers |
| [tests/audit.rs](../tests/audit.rs) | New, 261 lines, 8 tests: `tr12_c4_`, `tr12_c5_`, `tr12_c7_` and `tr12_held_` |
| [tests/judge_report.rs](../tests/judge_report.rs) | New, 316 lines, 4 tests: `tr12_c6_`, `tr12_c8_` and `tr12_c9_`, with `only_question` and `counts` helpers |
| [tests/common/mod.rs](../tests/common/mod.rs) | `schema_for` gained four match arms: `digest ack`, `digest`, `audit` and `judges` |
| [docs/DESIGN.md](DESIGN.md) | §2.3.2 (what `judges` splits and reads, and the dispute check's wording), §2.6 (overdue), §3's `audit` and `judges` rows, §6's two rows, §7's two bullets on `audit`. 18 lines changed |
| [docs/PLAN.md](PLAN.md) | Slice 16's status, `todo` to `in review` (`93637e2`). `8b8670c` marked it `in progress` |

To add another kind of digest entry, add its `Via` to `build`'s `listed` match and, if it needs a new field, to `Action` and `action_text`, then to [digest.schema.json](../schemas/digest.schema.json). To add another check to `audit`, add it to `check_item` or `check_deleted`, name its `problem` kind in [audit.schema.json](../schemas/audit.schema.json), and add a `tr12_c…`-style test that writes the sign by hand. To add another command, give it a schema first and a match arm in `schema_for`, as [CLAUDE.md](../CLAUDE.md) says.

## Testing

Run the test command in `tracker/`. TR-12's criteria run their own tests by prefix, such as `cargo test --test digest tr12_c1_`, and criteria 8 and 9 add a `grep` of DESIGN.md after their test. No test calls `tracker-judge` or the real `claude`: the harness's guard from [slice 14](slice-14-readme.md#testing) puts a failing one first on `PATH`, and each test that needs a judge sets the store's `judge.command` to a fake. The harness ([tests/common/mod.rs](../tests/common/mod.rs)) changed only by the four `schema_for` arms.

**The tests that need a terminal** use `Env::in_terminal` or `Env::in_terminal_typing`, so they run under `script`, and return early, passing without checking anything (or the part after it), where `script` isn't installed, as slice 9's do. Nine of the 22 are such tests: `tr12_c1_digest_ack_project_empties_only_that_projects_digest`, `tr12_c2_digest_ack_asks_for_ack_and_anything_else_cancels`, `tr12_c2_an_item_verified_while_the_question_waits_stops_the_ack`, `tr12_c3_shelves_human_actions_and_trigger_edits_are_listed_marked_by_terminal`, `tr12_c6_judges_shows_the_approve_and_reject_counts_for_each_question`, `tr12_c7_audit_reports_a_human_approved_items_missing_ref`, `tr12_c8_judges_splits_counts_by_the_held_questions_trigger_and_answer`, `tr12_c8_an_each_question_counts_once_per_decision` and `tr12_held_audit_reports_holds_on_an_item_not_in_review`. `tr12_c10_` runs its main checks without a terminal and acknowledges at the end under `script`.

### Coverage of the _Done when_ checks

| _Done when_ | Tests |
| --- | --- |
| `digest` lists every item auto-verified since that project's last ack, judge-passed first. `digest ack --project XX` empties only XX's digest. | `tr12_c1_`, 2 tests: `tr12_c1_digest_lists_every_auto_verified_item_judge_passed_first` and `tr12_c1_digest_ack_project_empties_only_that_projects_digest` (under `script`) |
| From a terminal, `digest ack` asks for `ack`. Typing the project key, or anything else, cancels with exit 1. An item auto-verified while the question waits makes it exit 1, record nothing, and stay in the next digest. | `tr12_c2_`, 3 tests: `tr12_c2_digest_ack_asks_for_ack_and_anything_else_cancels`, `tr12_c2_outside_every_root_digest_ack_needs_a_project` and `tr12_c2_an_item_verified_while_the_question_waits_stops_the_ack` |
| A shelve from piped stdin and one under `script` are listed, marked not from a terminal and from a terminal. So are a delete and an approve, an edit that removes `needs-review` and one that changes kind from `capability` to `change`. An edit that only adds a label isn't. | `tr12_c3_`, 2 tests: `tr12_c3_a_piped_shelve_is_listed_as_not_from_a_terminal` and `tr12_c3_shelves_human_actions_and_trigger_edits_are_listed_marked_by_terminal` (the shelve under `script`, the `approve` and `delete`, and each edit) |
| `audit` reports a hand-edited status, a done item with no verification record, and an ID used twice. | `tr12_c4_`, 3 tests: `tr12_c4_audit_reports_a_hand_edited_status`, `tr12_c4_audit_reports_a_done_item_with_no_verification_record` and `tr12_c4_audit_reports_an_id_used_twice` |
| `audit` reports a `deleted.json` line whose ID is at or above its project's counter. | `tr12_c5_`, 2 tests: `tr12_c5_audit_reports_a_deleted_line_at_or_above_its_counter` and `tr12_c5_audit_reports_a_line_that_cant_be_read` |
| `judges` shows the approve and reject counts for each question. | `tr12_c6_judges_shows_the_approve_and_reject_counts_for_each_question` (a rejected and an approved dispute of one question, text and `--json`) |
| `audit` reports a done `auto` or `human` item whose `refs/tracker/<ID>` is missing, or doesn't point at the record's `commit`. | `tr12_c7_`, 2 tests: `tr12_c7_audit_reports_a_missing_or_moved_ref` and `tr12_c7_audit_reports_a_human_approved_items_missing_ref` |
| `judges` splits the counts by the held question's trigger and answer, and §6 says so (criterion 8). | `tr12_c8_`, 2 tests: `tr12_c8_judges_splits_counts_by_the_held_questions_trigger_and_answer` (a disputed fail approved, an unsure answer approved and a pass held by `auto_verify: false` rejected, counted apart) and `tr12_c8_an_each_question_counts_once_per_decision`. The check also greps §6 for "splits each question's approves and rejects by the held question's trigger and answer" |
| §2.3.2 says any G3 failure without a judge answer skips the dispute check (criterion 9). | `tr12_c9_a_dispute_beside_a_missing_input_is_refused_for_g3` (exit 2, a G3 failure and no `held_questions`), which passed before the work, and the check's `grep` of §2.3.2, which didn't |
| `review` flags a digest older than `review.digest.every_days` as overdue, and says nothing of one that isn't (criterion 10). | `tr12_c10_review_flags_a_digest_older_than_every_days_as_overdue` (XX at 0 days is flagged, YY at the default isn't, and an acknowledged digest isn't) |
| The commands added since slice 5 have `--help` text and empty states (criterion 11). | `tr12_c11_`, 2 tests: `tr12_c11_every_command_added_since_slice_5_has_help_text` (each of `review`, `approve`, `reject`, `digest`, `digest ack`, `audit` and `judges`, with `--help` and `-h`) and `tr12_c11_digest_audit_and_judges_print_an_empty_state` |

Requirements from the _Prompt_ and the settled points that no _Done when_ bullet names:

| Requirement | Tests |
| --- | --- |
| `digest ack` outside every root needs `--project`. | `tr12_c2_outside_every_root_digest_ack_needs_a_project` |
| A decision counts once per question, however many `each` matches it held. | `tr12_c8_an_each_question_counts_once_per_decision` |
| `audit` reports holds on an item that isn't in review. | `tr12_held_audit_reports_holds_on_an_item_not_in_review` |
| Every `--json` output matches its schema. | `Env::json`, which checks each output against its schema |

Not covered by a test:

- **An unreadable `digests.json`.** No test writes one by hand and checks the exit 3 and its message.
- **A shelve of a judged item.** `action_text` prints its warning, and `build` sets `judged`, but no test reads either.
- **A decision with no `ask`.** `tally` groups it by the question's id, but no test feeds it a history entry written before TR-53 (the two files in [tests/fixtures/slice-15/](../tests/fixtures/slice-15/) have none, but only `fixtures.rs` reads them).
- **`audit`'s `held` check for a T12 question with no T12 hold.** The test covers holds on an item that isn't in review, and `check_item` also covers the two T12 cases.
- **`--all` for `digest`,** and an item file that can't be read, in `digest`, `audit` and `judges`' skipped note.

**Seen failing first.** All eleven of TR-12's checks failed at `start`, at `a877beb`, which was committed about a minute before it. Criteria 1 to 7, 10 and 11 exercise commands that didn't exist. Criterion 8's test and its `grep` both failed, since `judges` didn't exist and §6 didn't say it. Criterion 9's test, `tr12_c9_a_dispute_beside_a_missing_input_is_refused_for_g3`, passed at `start`, since `apply`'s `unasked` check already did what §2.3.2 now says, so the criterion failed through its `grep`, and T13 stayed quiet. The schema arms in `tests/common/mod.rs` were in `a877beb` too. They are additions only, which is why T4 stayed quiet.

### Checks done by hand

| What | How | Result |
| --- | --- | --- |
| The demo above | Run line by line with the fake judge, stderr captured, on a temporary `TRACKER_HOME`, with the temporary folder replaced by `$DEMO`. The real `claude` and `tracker-judge` weren't called, and the temporary folder was deleted | The output above. Every refusal exits as marked |
| TR-12's checks, before the approve | The orchestrator checked by hand that the six written criteria are unchanged, that all eleven checks failed at `start`, and that every commit since `start` is scoped to TR-12 | All three held |
| TR-12's approve in a real terminal | The user read the diff and approved it | `by: human` at `93637e2`, with `refs/tracker/TR-12` and three holds |

### Tracking TR-12

TR-12, a `capability` with `design` `docs/DESIGN.md#2.6`, was created at 2026-11-21T07:07:43Z with slice 7's other items, with six criteria from PLAN.md. The orchestrator added criteria 7 to 9 and the builder added 10 and 11 before `start`, and the six written ones are unchanged. The slice 15 binary was installed before it started and tracked it throughout, so T5, T13 and T14 ran on it:

| Step | When (UTC) | What |
| --- | --- | --- |
| Tests, committed | 21:00:35Z | `a877beb`, 69 seconds before `start` |
| `tracker start TR-12` | 21:01:44Z | `started.head` `a877beb`. All eleven checks failed |
| The code and docs | after `start` | `8b8670c`, `ccb8d13`, `d997b33` and `93637e2` |
| `tracker submit TR-12` | 21:18:57Z | The gates passed, then the triggers ran. Held under T1, T7 and T8 |
| `tracker approve TR-12` | 22:05:53Z | From a terminal, verified `human` at `93637e2`, with `refs/tracker/TR-12` |

- **The three holds** read: T1, "it's a capability, and adds the source files src/audit.rs, src/decisions.rs and src/digest.rs". T7, "2595 lines changed, more than the project's max_diff_lines of 300". T8, "the project's design doc changed: docs/DESIGN.md". They're recorded in `verification.triggers`, and `tracker show TR-12` is the record.
- **T4 didn't fire.** The only existing test file changed is `tests/common/mod.rs`, by four added match arms, in the test commit, before `start`.
- **T13 didn't fire.** Each criterion's check failed at `start`, criterion 9's through its `grep`.
- **T11 and T12 didn't fire,** since TR-12 has no judge criterion.

## Decisions worth knowing

These are the "Slice 16 must decide" points from slice 15's reviews, and what the slice chose:

- **`judges` groups a question by its `ask`, per project, and falls back to the ref's question id.** A ref's criterion number and `file:line` change, and a question id is unique only within one judge file, so the text is the stable key ([§4.3](DESIGN.md#43-item-file)). A decision recorded before TR-53 has no `ask`, so it's grouped by the id, apart from any question that has its text, and printed as "its text wasn't recorded".
- **Counts are split by trigger and answer, and they're never added.** Under `auto_verify: false`, approving a passing answer agrees with the judge and approving a disputed fail overrules it, so one total would mix them. `held_as` names which each is: `disputed`, `unsure` or `passed`. §2.3.2 and §6 say so (criterion 8).
- **A decision counts once per question, trigger and answer.** An `each` judge holds one entry for each match, but the item was held, and decided, once. §2.3.2 says `judges` counts the items a question held.
- **Decisions are read from history, not from the record.** A reopen moves the record to `previous_verification`, and history is never rewritten, so history is the one place all of an item's decisions survive.
- **A T11 hold isn't counted.** It records no question ([§2.7](DESIGN.md#27-human-actions)), so a question's T11 holds are in neither side of its counts. `judges`' text says so.
- **The digest lists a held question's reasons.** An `approve` or `reject` action prints each held question's trigger, ref, the model's answer and reason, and for T12 the agent's reason, from the entry's `held_questions`.
- **A shelve of an item with a judge criterion is flagged.** Shelving, reopening and starting an item empties its T11 count ([§7](DESIGN.md#7-honest-limits)), so the digest says to look for a reopen and `start` after it ([slice 14's review 2](reviews/slice-14-review-2.md#slice-15-must-decide)). It doesn't pair them up: that needs the item's later history, which the digest reads as separate entries.
- **`audit` checks `held_questions`.** Holds and held questions belong to an item in review, and a T12 hold and a T12 question go together, so a mismatch is a sign of a hand edit. It doesn't match a question's request to an answer, since that needs the cache.
- **`audit` doesn't read `judge/failures.json`.** A count changed by hand isn't reported, and removing the file resets every T11 count. §7 says so, and so does the digest's blind spot: a refused dispute or a `gates` run leaves no digest entry. The cache has the same gap.
- **Overdue means the oldest entry is at least `every_days` old.** Not the time since the last ack, since a project with nothing to read isn't overdue. An empty digest never is, and a digest that's never been acknowledged counts from its oldest entry. §2.6 says so. `every_days` of 0 makes any digest with entries overdue, which the test and the demo use.
- **`audit` exits 0 whatever it finds.** It's a report, as `review` is. A script that wants to fail on a problem reads `--json`'s `problems`.
- **`digest ack` shows the digest in its warning and compares whole digests.** Anything that changed it, an item or an action, stops the ack, rather than only a new item, since the criterion is that nothing unseen is acknowledged.
- **`digest ack` waits out the second.** It holds the lock until the clock passes the ack's time, so an entry written in the same second isn't missed ([What digests.json holds](#what-digestsjson-holds)).
- **The digest lists my own actions, marked.** §2.6 says the terminal check can be got around on purpose, so leaving mine out would hide an action that got round it. Each line says "from a terminal" or "not from a terminal", and the digest says to check that you made each one marked from a terminal.
- **`digest ack` takes a project and not an ID,** so the word typed is `ack`. Typing the project key cancels it, as the check says.

## Still open

From the build and the checks. There's no review of slice 16 yet, so none of its findings are listed.

**Checks not run** (they wait for slice 16's binary, installed at the end of the cycle):

- **`tracker digest ack` in a real terminal.** The tests run it under `script`, and the demo can't. Nobody has typed `ack` into it in a real terminal, or typed the project key to see it cancel, so what the warning looks like on a person's screen isn't checked.
- **`tracker digest` and `tracker judges` on the real store.** The tests and the demo use throwaway stores and a fake judge, so what they show for the real items, and how long a real digest reads, are unchecked.
- **A real judge.** No run used the real `claude`. `judges` counts what the fake produced.

**Test gaps** (see [Testing](#testing)): an unreadable `digests.json`, a shelve of a judged item, a decision with no `ask`, `audit`'s T12 mismatch in both directions, `digest --all` and the skipped note.

**Not decided:**

- **A `judges` count can't tell a reject about one question from one about another thing.** A reject may be about a new dependency, so §2.3.2 shows approves and rejects apart rather than as an agreement rate. The counts are evidence for the human to read, not a measure.
- **The digest grows until it's acknowledged.** It lists every entry since the ack, with no limit, so a project that's never acknowledged lists everything. Overdue flags it in `review`.

**Open items in `tracker`** (`tracker list --project TR`):

- **TR-39** (low, change): line up hold messages when trigger names differ in width, found while building slice 11.
- **TR-44** (low, fix): say what to do when git no longer has an item's `started.head`, from slice 10's review 2, bug 3.
- **TR-47** (low, fix): strip a `diff` judge's comments over each file's whole text, so a hunk that starts inside a block comment doesn't reach the judge.
- **TR-48** (low, change): say what else a judge sees besides the excerpt, Claude Code's environment note and the account's email.
- **TR-50** (low, fix): escape `</material>` in both judge adapters, pin step 1's prompt in a test, and say an empty `is_error` reply said nothing.

**Carried from slice 15** ([its Still open](slice-15-readme.md#still-open)): review 1's security 1 option (b), review 2's design 2 and its two test gaps, an approve that matches the request and not the answer, and a terminal `approve` of a held T12 item.

## Limits, and what later slices change

Slice 16 is the last slice in [PLAN.md](PLAN.md), so there's no later slice to change this one. What's left open across the plan, and what a change to this slice must not break:

- **Open items.** TR-39, TR-44, TR-47, TR-48 and TR-50 are open in `tracker`, all low priority ([Still open](#still-open)). TR-47 and TR-50 touch a judge's input, so a `diff` judge shouldn't be added to a project before TR-47 is done (a disputed `diff` question's excerpt has the same flaw).
- **Everything in [PLAN.md](PLAN.md)'s "Deferred / out of scope"** stays out: saved rulings for each question, mutation testing, and a branch or worktree per item ([§7](DESIGN.md#7-honest-limits)). If saved rulings are added, a decision no longer covers one submit, `judges` would need to say which counts came from a saved one, and the key is the excerpt and the question, not a held question's `request`, which changes with the model.
- **Slices 7 to 16 aren't merged.** Every slice's row in PLAN.md is `in review` from 7 on, and a row becomes `merged` only once its review fixes are in and its branch is merged. The branches are stacked, so they merge in order.
- **No review of this slice yet.** When there is one, its fixes are new items, and this document gets a `Fixes after review 1` section ([slice 15's readme](slice-15-readme.md) shows the form).
- **`audit` and the digest are reports, not gates.** Neither blocks anything, and `audit` exits 0 whatever it finds. A script that needs a failing exit must read `--json`.
- **`judges` depends on `held_questions` in history.** A change to that shape, or to when `approve` and `reject` record it, changes what `judges` counts. A new place that moves an item out of review must record its decision with `record_decision`, as slice 15's readme says. A new trigger that holds questions must put them in `held_questions` with its own `trigger`, and `decisions::held_as` needs to say what it means.
- **A new human action** goes in the digest's `listed` match in `digest::build`, with its `Via`, and in [§2.6](DESIGN.md#26-digest-auditing-the-auto-verifications) and [§2.7](DESIGN.md#27-human-actions), or the digest won't show it.
- **A new item field that `audit` should check** goes in `check_item`, with a `problem` kind in [audit.schema.json](../schemas/audit.schema.json).
- **`digests.json` is written only by `digest ack`.** A command that edits it would need the lock, and the digest's "since the ack" depends on every other write's second being past the ack's, which `digest ack`'s wait keeps.
- **The real-terminal checks** of `digest ack`, `digest` and `judges` are the user's, once slice 16's binary is installed. PLAN.md's integration risks say every new interactive command is tried once in a real terminal.

# Walkthrough: tracking the tracker's own build

This is a short story of the first time I used `tracker` on real work: its own slice 7. It follows one item, TR-1, from the moment the project was registered to the moment I fixed one of its criteria by hand and approved the change. The commands are the ones from that session, and the dates are as they appear in the item's history.

> **Tracker version.** The session itself ran on the slice 6 binary. Each step was re-run on 2026-11-24 in a throwaway store, with `tracker 0.1.0` built from commit `e5dbdd4` (TR-57). That build has all 16 slices of PLAN.md and their review fixes. The output below is from that re-run, with the session's dates kept, so the hashes differ from the real store. Two things are new since the session: `start` names each check as it runs it, and `show` has a `Started:` block ([step 4](#4-watch-the-item-start)). The triggers this story looks ahead to, T5 and the digest, are built now, and [step 6](#6-whats-next) shows what they do to TR-1.

For what each command does in full, see [DESIGN.md](DESIGN.md). For how the build is split into slices, see [PLAN.md](PLAN.md).

## Contents

1. [Set up the store and register the project](#1-set-up-the-store-and-register-the-project)
2. [See what's available](#2-see-whats-available)
3. [Read the item](#3-read-the-item)
4. [Watch the item start](#4-watch-the-item-start)
5. [Fix a mistaken criterion, with my approval](#5-fix-a-mistaken-criterion-with-my-approval)
6. [What's next](#6-whats-next)
7. [What I took away](#7-what-i-took-away)

## 1. Set up the store and register the project

After slice 6 the tracker was good enough to track itself. This is PLAN.md's "self-tracking starts" checkpoint. I installed it, created the store and registered the `tracker` crate as project `TR`:

```text
$ cargo install --path .
$ tracker init
Created the store at ~/.tracker.
$ tracker project add TR --name tracker --root <guild-portfolio>/tracker --design docs/DESIGN.md
Registered TR (tracker) at <guild-portfolio>/tracker.
```

The store lives in `~/.tracker`. `project add` writes the project's `name`, `root` and `design` into its `config.json`. I then added, by hand, the checks every item in it must pass at submit, and their timeout:

```json
"TR": {
  "name": "tracker",
  "root": ".../guild-portfolio/tracker",
  "design": "docs/DESIGN.md",
  "checks": ["cargo fmt --check", "cargo clippy -- -D warnings", "cargo test"],
  "check_timeout_secs": 300
}
```

Then I created the item for slice 7 from PLAN.md. The slice's outcome became the title, and each "Done when" line became a criterion. The slice adds new behaviour, so its kind is `capability`. On 2026-11-18 it became **TR-1**.

## 2. See what's available

On 2026-11-21 I made a branch for the slice and asked what was on the list:

```text
$ git switch -c feat/tracker/slice-7
$ tracker list
  ID    PRIORITY  KIND        STATUS  TITLE
TR  tracker
  TR-1  medium    capability  open    See checks run at start and submit, and fail with their output

TR only. `--all` for every project.
Open, in-progress and review items. `--status done,shelved` for the others.
```

Because I ran it inside the project's folder, `list` showed only `TR` and said so. `tracker list --all` shows every project. With one project registered, it gave the same single item. The footer also says what's hidden: done and shelved items need `--status done,shelved`.

`tracker --help` lists every command, and ends with the exit codes: 0 success, 1 bad input or not found, 2 refused by a gate, 3 the store, a file or git can't be read.

## 3. Read the item

`tracker show` gives everything about one item:

```text
$ tracker show TR-1
TR-1  See checks run at start and submit, and fail with their output
  project   TR (tracker)
  kind      capability
  status    open
  priority  medium
  design    docs/DESIGN.md#2.4
  created   2026-11-18T13:58:22Z
  updated   2026-11-18T13:58:22Z

Done when:
  1. A failing check shows its command, exit code and last lines of output.
       manual, no evidence yet
  2. A check running longer than its timeout is stopped and fails.
       manual, no evidence yet
  3. A broken project check fails G4 even when every criterion check passes.
       manual, no evidence yet
  4. records a check that fails, as expected, and warns about one that already passes.
       manual, no evidence yet
  5. A passing check's last 20 lines are kept.
       manual, no evidence yet

Verification: none.

History: none yet.
```

All five criteria were **manual**: they had text but no check that proves them. Nothing had happened to the item yet, so its history was empty.

**Criterion 4 is wrong.** It starts mid-sentence. PLAN.md has it as "`` `start` `` records a check that fails…", and the word `start` is gone. The most likely cause is the shell. I put the criterion in double quotes when I created the item, and inside double quotes the shell treats backticks as "run this command and paste its output". It tried to run a command called `start`, got nothing back, and pasted that nothing into the text.

I didn't fix it at this point, which matters in step 5.

## 4. Watch the item start

Next, Claude Code picked up the item. My global `CLAUDE.md` tells an agent working in a tracked project to:

1. Give every criterion a check.
2. Write and commit the tests those checks run.
3. See them fail.
4. Only then run `tracker start`.

It gave each criterion a script check that runs just that criterion's test. It committed the tests as `test(TR-1): add tests for running criterion and project checks`, saw them fail, and started the item. `start` runs every check once, naming each as it goes, and records whether it passed:

```text
$ tracker criterion set TR-1 1 --check 'cargo test --test checks a_failing_check_shows_its_command_exit_code_and_last_lines_of_output'
Criterion 1 of TR-1 now has a check.
...
$ tracker start TR-1
Running criterion 1's check: `cargo test --test checks a_failing_check_shows_its_command_exit_code_and_last_lines_of_output`
Running criterion 2's check: `cargo test --test checks a_check_running_longer_than_its_timeout_is_stopped_and_fails`
Running criterion 3's check: `cargo test --test checks a_broken_project_check_fails_g4_even_when_every_criterion_check_passes`
Running criterion 4's check: `cargo test --test checks start_records_a_check_that_fails_and_warns_about_one_that_already_passes`
Running criterion 5's check: `cargo test --test checks a_passing_checks_last_20_lines_are_kept`
TR-1: open → in-progress
```

Here is the item afterwards:

```text
$ tracker show TR-1
TR-1  See checks run at start and submit, and fail with their output
  ...
  status    in-progress
  updated   2026-11-21T05:19:55Z

Done when:
  1. A failing check shows its command, exit code and last lines of output.
       check: cargo test --test checks a_failing_check_shows_its_command_exit_code_and_last_lines_of_output
  2. A check running longer than its timeout is stopped and fails.
       check: cargo test --test checks a_check_running_longer_than_its_timeout_is_stopped_and_fails
  3. A broken project check fails G4 even when every criterion check passes.
       check: cargo test --test checks a_broken_project_check_fails_g4_even_when_every_criterion_check_passes
  4. records a check that fails, as expected, and warns about one that already passes.
       check: cargo test --test checks start_records_a_check_that_fails_and_warns_about_one_that_already_passes
  5. A passing check's last 20 lines are kept.
       check: cargo test --test checks a_passing_checks_last_20_lines_are_kept

Started:
  at      2026-11-21T05:19:55Z
  hash    56bb0469f8d0097c636d5d0544857a88680472841655a8bae05cc927881bcb5a
  head    aca45bb371e37c68c1a503606d447ab9e8f7d080
  checks  criterion 1 failed, criterion 2 failed, criterion 3 failed, criterion 4 failed, criterion 5 failed

Verification: none.

History, oldest first (1 entry):
  2026-11-21T05:19:55Z  start  open → in-progress  not from a terminal
```

Four things changed:

- **Status** moved from `open` to `in-progress`.
- **Each criterion** went from "manual, no evidence yet" to a `check:` command.
- **Started** records the start: the commit `HEAD` pointed to, a fingerprint (`hash`) of the criteria, kind and design, and that every check failed, as it should before the work exists. Trigger T5 compares the fingerprint at submit, T13 reads the check results, and T14 reads `head`.
- **History** got its first entry. It records the move (`start`, `open → in-progress`) and that it came **not from a terminal**, so it was the agent's command, not one I typed.

The agent's check for criterion 4 is named `start_records_…`, so it knew what the criterion meant. But it left the broken text alone. That's on purpose: my `CLAUDE.md` says never to reword or remove a criterion you were given. Rewording the criteria is how an agent could quietly move the goalposts, so the tracker treats the text I wrote as mine.

## 5. Fix a mistaken criterion, with my approval

The criterion was mine to fix, so I fixed it myself. I opened the item file in my editor:

```sh
code ~/.tracker/items/TR-1.json
```

I changed criterion 4's text to "`` `start` `` records a check that fails, as expected, and warns about one that already passes."

Editing the file alone isn't enough. The item keeps a second list, `written`, which holds each criterion's text as it was given at create. An edited text no longer matches what's in `written`. From the outside, my edit looks exactly like an agent rewording a criterion. Trigger T5 holds any item where that happens.

To tell the tracker the edit was mine, I ran `criterion accept`. It works only from an interactive terminal, so an agent can't run it:

```text
$ tracker criterion accept TR-1
Accepting records the text of each of TR-1's criteria as it is now in written, so a human's edit isn't taken for an agent's:
  1. A failing check shows its command, exit code and last lines of output.  (already written)
  2. A check running longer than its timeout is stopped and fails.  (already written)
  3. A broken project check fails G4 even when every criterion check passes.  (already written)
  4. `start` records a check that fails, as expected, and warns about one that already passes.  (new to written)
  5. A passing check's last 20 lines are kept.  (already written)
Accepting also drops this criterion from written. It was removed, or reworded by hand:
  - records a check that fails, as expected, and warns about one that already passes.
Type TR-1 to confirm, or anything else to cancel: TR-1
Accepted TR-1's criteria: written now records the text of all 5.
```

Before it changed anything, it showed me two things:

- **What it would record.** Four criteria were already in `written`. Criterion 4's new text was "new to written".
- **What it would drop.** It named the old, broken text. If an agent had removed or reworded a different criterion behind my back, that text would be listed here too, and I'd see it before confirming.

Then it asked me to type the item's ID, the same confirmation `delete` uses. A stray Enter can't approve anything.

The item now reads correctly, and the approval is on the record:

```text
$ tracker show TR-1
  ...
  updated   2026-11-21T05:28:22Z

Done when:
  ...
  4. `start` records a check that fails, as expected, and warns about one that already passes.
       check: cargo test --test checks start_records_a_check_that_fails_and_warns_about_one_that_already_passes
  ...

History, oldest first (2 entries):
  2026-11-21T05:19:55Z  start             open → in-progress  not from a terminal
  2026-11-21T05:28:22Z  criterion-accept  stays in-progress   from a terminal
      changed: written
```

### The same record, as JSON

`tracker show TR-1 --json` prints the item as stored, without the formatting. The parts that changed are `written` and the item's history:

```json
"written": [
  "A failing check shows its command, exit code and last lines of output.",
  "A check running longer than its timeout is stopped and fails.",
  "A broken project check fails G4 even when every criterion check passes.",
  "`start` records a check that fails, as expected, and warns about one that already passes.",
  "A passing check's last 20 lines are kept."
],
...
"history": [
  {
    "at": "2026-11-21T05:19:55Z",
    "from": "open",
    "to": "in-progress",
    "via": "start",
    "interactive": false
  },
  {
    "at": "2026-11-21T05:28:22Z",
    "from": "in-progress",
    "to": "in-progress",
    "via": "criterion-accept",
    "interactive": true,
    "fields": [
      "written"
    ]
  }
]
```

`written` now holds criterion 4's corrected text, and the old text is gone, so every criterion's text matches a text in `written` again.

Each part of the text history line comes from one field of the JSON entry:

| Text line                        | JSON field                                        | Meaning                                                  |
| -------------------------------- | ------------------------------------------------- | -------------------------------------------------------- |
| `2026-11-21T05:28:22Z`           | `"at"`                                            | When it happened.                                        |
| `criterion-accept`               | `"via": "criterion-accept"`                       | The command that made the entry.                         |
| `stays in-progress`              | `"from": "in-progress"`, `"to": "in-progress"`    | The state didn't move: `from` equals `to`.               |
| `from a terminal`                | `"interactive": true`                             | It came from an interactive terminal, so from a human.   |
| `changed: written`               | `"fields": ["written"]`                           | The field it changed. A move between states has no `fields`. |

Compare the first entry, from the agent's `start`. It has `"interactive": false` and no `fields`, because it moved the item between states instead of changing a field.

Other commands read these fields. `tracker digest` lists every `criterion-accept` entry as one of my human actions, marked from a terminal or not, so I can check later that I really made each one.

## 6. What's next

TR-1 is in progress with five failing tests, one per criterion. From here:

1. The agent writes the code until those tests pass, committing as `feat(TR-1): …`.
2. `tracker gates TR-1` runs the gates `submit` runs, without changing anything, to show what's left.
3. `tracker submit TR-1` runs the gates. Every criterion's check must pass (G3), and so must the project's checks (G4). Once they do, the triggers run.

When this story happened, no triggers existed yet, so a passing submit went straight to done. With today's tracker, TR-1 is held twice over. It's a `capability` (T1). And `criterion accept` records the new text in `written` but doesn't take the fingerprint again, so the edit after `start` still counts as moved goalposts (T5). Re-run in a throwaway store, the submit says:

```text
$ tracker submit TR-1
...
TR-1: in-progress → review
Every gate passed, and TR-1 is held for review:
  T1  it's a capability, so a human should check the design fit
  T5  the criteria, their checks, the kind or the design changed after start
It waits in `tracker review` for a human to approve or reject it. `tracker show TR-1 --diff` shows its work.
```

It exits 0, since a hold isn't an error. I'd then read the change with `tracker show TR-1 --diff` and decide it with `tracker approve TR-1` or `tracker reject TR-1 --reason "…"` from my own terminal. Like `criterion accept`, both ask me to type the ID.

## 7. What I took away

- **`list` → `show` is the loop for finding work.** `list` says what's open, and `show` says exactly what "done" means for one item.
- **History tells you who did what.** Each entry is marked from a terminal or not, so an agent's moves and mine are easy to tell apart.
- **Agents can't change my criteria, and neither can I by accident.** A hand edit isn't trusted until `criterion accept` shows me what it records and drops, and I type the ID to confirm.
- **Fix criteria before `start`.** I noticed criterion 4 while the item was still open and left it. `start` takes T5's fingerprint, so changing a criterion's text after `start` holds the item for my review, even after `criterion accept` ([step 6](#6-whats-next)). Fixing it while the item is open avoids that.
- **Don't put backticks inside double quotes on the command line.** Use single quotes, as in `--done-when '`start` records…'`. Better still, read each criterion back with `show` right after `create`.

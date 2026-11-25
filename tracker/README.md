# tracker

**A work tracker that won't let anything be marked done until it's proven done.**

`tracker` is a command-line work tracker, written in Rust, for work done with coding agents such as Claude Code. It keeps one list of work items for all your projects, like a small personal JIRA. What makes it different is that nobody marks an item done, agent or human. The agent **submits** the item, and the tool decides:

- Every item says up front what "done" means, as a list of **Done when** criteria. Each criterion is backed by a test or other script (a **check**), or, for things a test can't measure, by short yes/no questions that Claude answers (a **judge**).
- On submit, the tool runs **gates**: every check must pass now, the project's full test suite and linters must pass, and the work must be committed. If any gate fails, the submit is refused and every reason is listed.
- If the gates pass, **review triggers** look for risk in the commits: a new capability, a new dependency, an edited test, a criterion changed after work began, a check that passed before the work existed, and more. If nothing fires, the item is **auto-verified** and goes straight to done. If something fires, the item is **held** in your inbox, with the reasons.
- A periodic **digest** lists everything that reached done without you, so you can audit the machine's decisions without having to approve each item.

Most items reach done with no human involved, because their checks prove them. Your attention goes where the risk is, and the tool says _why_ each held item needs you.

## Contents

- [Why use it](#why-use-it)
- [Install](#install)
- [Quick start](#quick-start)
- [A sample workflow](#a-sample-workflow)
- [Setting it up for an agent](#setting-it-up-for-an-agent)
- [How it decides](#how-it-decides)
- [Command reference](#command-reference)
- [Where things are stored](#where-things-are-stored)
- [Limits](#limits)
- [Further reading](#further-reading)

## Why use it

When an agent says "done", you usually have two choices: trust it, or check everything yourself. `tracker` gives you a third: let a machine check what can be checked, and only look at what can't.

| Without `tracker`                                                       | With `tracker`                                                                                                                         |
| ----------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| "Done" is whatever the agent says it is.                                | "Done" means the item's checks passed against a real commit, and the record says which tests ran and what they printed.                |
| An agent can edit a failing test until it passes.                       | Changing an existing test holds the item for you (T4).                                                                                 |
| A test can pass without testing anything, such as a filter with a typo. | Every check runs once at `start`, before the work exists, and must fail. One that already passes holds the item (T13).                 |
| The goalposts can move quietly.                                         | The criteria are fingerprinted at `start`, and rewording one you wrote holds the item (T5).                                            |
| You review everything, or nothing.                                      | You review what's risky: new capabilities, new dependencies, risky paths and large diffs. The rest is listed in a digest you can skim. |
| Old work can disappear after a rebase.                                  | Every verified commit is kept by a git ref, `refs/tracker/<ID>`, so the tests as they ran can always be read again.                    |

It's built for one person working with agents on their own machine. It catches accidental and lazy shortcuts, not deliberate cheating. [Limits](#limits) says what it doesn't catch.

## Install

You need:

- **Rust 1.89 or later**, to build it.
- **git**. Every project must be inside a git working tree.
- **The `claude` CLI, logged in**, only if you use judge criteria. Items proven by scripts alone never call a model.

From this folder:

```sh
cargo install --path .
```

That installs two binaries into `~/.cargo/bin`: `tracker`, and `tracker-judge`, which `tracker` runs to ask Claude a judge's questions.

## Quick start

```sh
tracker init                                   # create the store at ~/.tracker
cd ~/code/myproject
tracker project add MP --name myproject --root .
tracker create "Search filters by tag" --done-when "Clicking a tag shows only matching items"
tracker list
```

Every project gets a short key, such as `MP`, and its items are numbered from it: `MP-1`, `MP-2` and so on. Commands run inside a project's folder use that project, so you rarely need `--project`.

Add the project's own checks, which run on every submit, to `~/.tracker/config.json`:

```json
{
  "projects": {
    "MP": {
      "name": "myproject",
      "root": "/Users/you/code/myproject",
      "checks": [
        "cargo fmt --check",
        "cargo clippy -- -D warnings",
        "cargo test"
      ]
    }
  }
}
```

Every other setting has a default. [Configuration](#configuration) covers the ones you're most likely to change.

## A sample workflow

This follows two items through a small Rust library, `slugs`, which turns titles into URL slugs. **You** set up the project and write the items. **The agent** (Claude Code, running shell commands) writes the tests and the code, and submits. The tool decides what reaches done.

The output below is real, from runs in a throwaway store, with paths shortened, `cargo`'s build lines left out, and `…` where lines are cut. Two exceptions are marked where they come up: the `approve` prompt, which only runs at a real terminal, and the T13 example, which was run in a second store.

### 1. You: register the project (once)

```console
$ tracker init
Created the store at ~/.tracker.
$ cd ~/code/slugs
$ tracker project add SL --name slugs --root .
Registered SL (slugs) at ~/code/slugs.
```

Then set its checks in `~/.tracker/config.json`, as in [Quick start](#quick-start): `cargo fmt --check`, `cargo clippy -- -D warnings` and `cargo test`.

### 2. You: write the item

Write the criteria in your own words. Each `--done-when` adds a criterion with no check yet. The agent gives each one a check before it starts.

```console
$ tracker create "Slugify turns a title into a URL slug" --priority high \
    --done-when "Letters are lowercased and spaces become hyphens" \
    --done-when "Punctuation is dropped"
Created SL-1: Slugify turns a title into a URL slug
2 manual criteria: give each a check with `tracker criterion set SL-1 <n> --check "…"`.

$ tracker list
  ID    PRIORITY  KIND    STATUS  TITLE
SL  slugs
  SL-1  high      change  open    Slugify turns a title into a URL slug

SL only. `--all` for every project.
Open, in-progress and review items. `--status done,shelved` for the others.
```

The tool remembers the text you wrote. If the agent later rewords or drops one of your criteria, the item is held for you (T5).

### 3. Agent: write failing tests, then start

The agent reads the item, writes one test per criterion in `tests/slugify.rs`, and gives each criterion a check that runs just that test:

```console
$ tracker show SL-1
$ tracker criterion set SL-1 1 --check "cargo test --test slugify lowercases_and_hyphenates"
Criterion 1 of SL-1 now has a check.
$ tracker criterion set SL-1 2 --check "cargo test --test slugify drops_punctuation"
Criterion 2 of SL-1 now has a check.
$ git add tests/slugify.rs && git commit -m "test(SL-1): add slugify tests"
$ tracker start SL-1
Running criterion 1's check: `cargo test --test slugify lowercases_and_hyphenates`
Running criterion 2's check: `cargo test --test slugify drops_punctuation`
SL-1: open → in-progress
```

`start` runs every check once and expects each one to **fail**, because the feature doesn't exist yet. Both did, so it printed no warning. A check that fails before the work and passes after it shows that it really tests the work.

The commit message matters. A commit belongs to an item only if its header is a [Conventional Commit](https://www.conventionalcommits.org/) scoped to the item's ID, `<type>(<ID>): <message>`, such as `test(SL-1): …` or `feat(SL-1): …`. That's how the gates and triggers find the item's work in git.

### 4. Agent: do the work and submit

The agent writes `slugify` and submits, but forgets to commit:

```console
$ tracker submit SL-1
Running criterion 1's check: `cargo test --test slugify lowercases_and_hyphenates`
Running criterion 2's check: `cargo test --test slugify drops_punctuation`
Running project check: `cargo fmt --check`
Running project check: `cargo clippy -- -D warnings`
Running project check: `cargo test`
error: SL-1 can't be submitted: G5 failed. Nothing was changed.
  G5  SL's root has an uncommitted change: modified src/lib.rs. Commit it, scoped to SL-1, or remove it: the proof has to match a real commit.
```

The checks passed, but the proof has to match a real commit, so the submit is refused with exit code 2. The agent commits and submits again:

```console
$ git commit -am "feat(SL-1): slugify titles"
$ tracker submit SL-1
Running criterion 1's check: `cargo test --test slugify lowercases_and_hyphenates`
…
Running project check: `cargo test`
SL-1: in-progress → done
Verified automatically: every gate passed at 3bfeff6, and refs/tracker/SL-1 keeps that commit.
```

Every gate passed and no trigger fired, so SL-1 is done with no human involved. Its record keeps the proof: the commit, and each check's command, result and last lines of output.

```console
$ tracker show SL-1
…
Verification:
  by      auto
  at      2026-11-24T22:03:37Z
  commit  3bfeff6ae17cc38ca4ca57cad8980067562556c9
  ref     refs/tracker/SL-1
  checks:
    criterion 1, passed in 0s: `cargo test --test slugify lowercases_and_hyphenates`
      running 1 test
      test lowercases_and_hyphenates ... ok
      …
```

An agent can run `tracker gates SL-1` at any time to ask "what's left?". It runs the same gates without changing anything, and once they pass, says whether a submit would be held.

### 5. Agent: a riskier item is held for you

The next item adds a capability, a new `src/limit.rs` module, so you created it with `--kind capability`:

```console
$ tracker create "Slugs can be capped at a maximum length" --kind capability \
    --done-when "A slug longer than the limit is cut at the last whole word"
```

The agent follows the same steps: a test and a stub of `src/limit.rs`, committed as `test(SL-2): …`, then `start`, the work as `feat(SL-2): …`, and `submit`:

```console
$ tracker submit SL-2
…
SL-2: in-progress → review
Every gate passed, and SL-2 is held for review:
  T1  it's a capability, and adds the source file src/limit.rs
It waits in `tracker review` for a human to approve or reject it. `tracker show SL-2 --diff` shows its work.
```

The work is proven, but new behaviour deserves a look at how it fits the design, so trigger T1 holds it. The agent stops here and reports the hold. It can't approve its own work: `approve` refuses anything but an interactive terminal.

```console
$ tracker approve SL-2
error: `tracker approve SL-2` needs a human at an interactive terminal, who types SL-2 to confirm it, and stdin here isn't a terminal. Nothing was changed. Only a human approves a held item. It waits in `tracker review` until one approves or rejects it.
```

### 6. You: clear the inbox

`tracker review` is your inbox. It lists held items from every project, wherever you run it:

```console
$ tracker review
1 item is held for review, waiting for you to approve or reject each one:

SL-2  Slugs can be capped at a maximum length
  T1  it's a capability, and adds the source file src/limit.rs
  See its work with `tracker show SL-2 --diff`, then run `tracker approve SL-2` or `tracker reject SL-2 --reason "…"`.
```

`tracker show SL-2 --diff` prints the item, the files that fired each trigger, the item's commits, and their diffs, so you can review the hold without leaving the tool. Then decide. `approve` shows what it will do and asks you to type the ID (this prompt is put together from the source, since it can't be recorded without a real terminal):

```console
$ tracker approve SL-2
SL-2 "Slugs can be capped at a maximum length" is in review, held by:
  T1  it's a capability, and adds the source file src/limit.rs
Approving it runs the gates again at 8de6ead, and if they pass, moves it to done, verified by a human, with refs/tracker/SL-2 keeping that commit.
`tracker show SL-2 --diff` shows its work.
Type SL-2 to confirm, or anything else to cancel: SL-2
```

Or send it back with `tracker reject SL-2 --reason "Cut at a byte, not a character: try 'café-au-lait'"`. The reason goes in the item's history for the agent to read, and the item returns to in-progress.

A held item pauses its project. Only one item per project can be in progress or in review at a time, so nothing else starts until you decide:

```console
$ tracker start SL-3
error: SL-3 can't be started: G1 failed. Nothing was changed.
  G1  SL-2 is in review in SL, waiting for a human to approve or reject it, and a project works on one item at a time. SL-3 can start once SL-2 is decided.
```

### 7. You: audit the digest

Every so often, say weekly, read what reached done without you:

```console
$ tracker digest
SL's digest, never acknowledged
Auto-verified (1), the ones a judge passed first:
  SL-1  Slugify turns a title into a URL slug
    verified at 3bfeff6 on 2026-11-24T22:03:37Z
    1. Letters are lowercased and spaces become hyphens
    2. Punctuation is dropped
    criterion 1's check passed: `cargo test --test slugify lowercases_and_hyphenates` (0s)
    criterion 2's check passed: `cargo test --test slugify drops_punctuation` (0s)
    project check passed: `cargo fmt --check` (0s)
    project check passed: `cargo clippy -- -D warnings` (0s)
    project check passed: `cargo test` (0s)
Reopen anything that looks wrong with `tracker reopen <ID> --reason "…"`. Once you've read it all, run `tracker digest ack --project SL`.
```

The digest also lists every shelve and every human action, such as approves, rejects and deletes, so you'd spot one you didn't make. `tracker digest ack` records that you've read it, and `tracker review` says when a digest is overdue.

### What a check that proves nothing looks like

Once SL-2 is decided, suppose the agent picks up SL-3, "Slugs keep digits", and makes a typo in its test filter. `cargo test --test slugify keeps_digts` matches no test, runs nothing and exits 0, so it "passes". `start` catches it, because it passes before the work exists. (This run was in a second store, where the item was SL-1. The ID is changed to match the story.)

```console
$ tracker start SL-3
Running criterion 1's check: `cargo test --test slugify keeps_digts`
warning: Criterion 1's check, `cargo test --test slugify keeps_digts`, already passes before the work is done, so it can't be proving the new work. Make sure it tests what criterion 1 says, such as with a filter that matches a test that fails now. SL-3 started anyway.
SL-3: open → in-progress
```

The start goes ahead, since a refactor's "nothing changes" check passes at start on purpose. But the submit is held for you:

```console
$ tracker submit SL-3
…
SL-3: in-progress → review
Every gate passed, and SL-3 is held for review:
  T13  criterion 1's check already passed before the work, so it can't be proving it
```

### Judges, for what tests can't measure

Some qualities are hard to test, such as "every error message says what to do next". For those, a criterion can have a **judge**: a few short yes/no questions that Claude answers about a small excerpt of the code. Write the judge as a JSON file:

```json
{
  "text": "Every error message says what went wrong and what to do next",
  "inputs": ["src/errors.rs"],
  "each": "(bail!|anyhow!)\\(",
  "ask": [
    {
      "id": "suggests_next_step",
      "ask": "Does this error message tell the user what to do next, such as a command to run or a value to change?",
      "yes": "the message gives a concrete next step",
      "no": "the message only reports the problem"
    }
  ]
}
```

and add it with `tracker criterion add MP-7 --judge errors-judge.json`. With `each`, the questions are asked once for every match of the pattern, so each error message is graded on its own.

Each question is a separate, one-shot `claude -p` call with no tools and no access to your files. Comments are stripped from the excerpt first, so a comment such as `// this is fine` can't sway it. Answers are cached by content, so re-running unchanged work can't fish for a better answer. A "no" refuses the submit. An "unsure" holds the item for you (T10). If the agent believes a "no" is wrong, it can **dispute** it with `tracker submit MP-7 --dispute <ref> --reason "…"`, which holds the item for you to decide (T12). A dispute never passes anything.

Prefer tests. A judge is evidence, not proof, and if a judged criterion could be a test, it should be one. [DESIGN.md §2.3.1](docs/DESIGN.md#231-judges-claude-via-the-claude-cli) covers judges in full.

## Setting it up for an agent

`tracker` is built to be driven by an agent without you approving each command. Two pieces of setup make that work.

**Instructions.** Put the agent's rules in your global `~/.claude/CLAUDE.md`, so every project gets them. The full text is in [DESIGN.md §5](docs/DESIGN.md#5-working-with-agents). The core of it:

```markdown
## Work tracking (`tracker`)

- Pick up work with `tracker show <ID>`.
- Before `tracker start <ID>`, give every criterion its check, write and commit the tests those checks run,
  and see them fail.
- Never reword or remove a criterion you were given.
- Make each script check run just that criterion's test, such as `cargo test filter_by_label`, not the whole suite.
- Write every commit for an item as a Conventional Commit scoped to its ID: `<type>(<ID>): <short message>`.
- Change items only through `tracker` commands. Never edit files under `~/.tracker`.
- Add `--json` when you need to read a result.
- Run `tracker gates <ID>` to see what's left, then `tracker submit <ID>`.
- If a gate refuses, fix the cause. Never weaken a check or edit an existing test just to pass.
- If an item is held, stop and report the hold reasons. Don't try to approve it.
- If you find work that's out of scope, `tracker create` it instead of doing it.
- Never delete an item. If one shouldn't be done now, `tracker shelve <ID> --reason "…"`.
```

**Permissions.** In `~/.claude/settings.json`, let the agent run `tracker` freely, and stop Claude Code's edit tools from touching the store:

```json
{
  "permissions": {
    "allow": ["Bash(tracker:*)"],
    "deny": ["Edit(~/.tracker/**)"]
  }
}
```

The allow rule also covers `approve`, `reject`, `delete`, `criterion accept` and `digest ack`. That's safe, because each of them refuses anything but an interactive terminal, and asks you to type the item's ID to confirm.

**JSON output.** Every command takes `--json` and prints one JSON object instead of text, so an agent reads failures, refs and hold reasons as fields rather than parsing messages:

```console
$ tracker review --json
{"held":[{"id":"SL-2","project":"SL","title":"Slugs can be capped at a maximum length","holds":[{"trigger":"T1","message":"it's a capability, and adds the source file src/limit.rs","files":["src/limit.rs"]}],"diff":"tracker show SL-2 --diff"}],"skipped":[]}
```

Each command's output is defined by a JSON Schema in [schemas/](schemas/). Every error has `error` and `exit_code`.

**Exit codes:** `0` success, which includes "held for review", `1` bad input or not found, `2` refused by a gate, and `3` the store, a file or git can't be read.

## How it decides

### States

```
                                 ┌─ no triggers ───────────────────▶ done (auto)
  open ──▶ in-progress ──submit──┤                                    ▲
   │        ▲      │             └─ triggers fired ─▶ review ─approve─┘ (human)
   │        │      │                                    │
   │        └──────┼────────── reject (reason) ─────────┘
   │               │
   └───────────────┴── shelve (reason) ──▶ shelved

  done or shelved ──reopen (reason)──▶ open
  any state ──delete (you only)──▶ gone
```

Agents never move an item to done. They submit, and the tool decides. A move the diagram doesn't draw is refused, and the error lists the moves that are allowed.

### Gates: is it finished?

Gates run on `submit`, and again on `approve`. If any fails, nothing changes, and every failure is listed.

| Gate   | Rule                                                                                                               |
| ------ | ------------------------------------------------------------------------------------------------------------------ |
| **G1** | (On `start`.) The item has at least one criterion, and no other item in the project is in progress or in review.   |
| **G2** | The item still has a criterion, and every manual criterion has an evidence note.                                   |
| **G3** | Every criterion's check passes now, and no judge answers with a fail.                                              |
| **G4** | Every project check passes, such as the full test suite and linters.                                               |
| **G5** | Nothing under the project root is uncommitted or untracked, and the item has at least one commit scoped to its ID. |
| **G6** | If the item names a design section, such as `DESIGN.md#2.4`, that section exists.                                  |

### Triggers: does it need a human?

Triggers run once every gate has passed. Each one reads the item's commits in git, not what the agent says. If any fires, the item is held in review.

| Trigger | Holds the item when…                                                              |
| ------- | --------------------------------------------------------------------------------- |
| **T1**  | It's a `capability`, or adds a new source file or module.                         |
| **T2**  | A changed file matches the project's `risky_paths`, such as storage or auth code. |
| **T3**  | A dependency manifest or lockfile changed.                                        |
| **T4**  | An existing test file had lines changed or removed. Adding tests is fine.         |
| **T5**  | The criteria, their checks, or the item's kind or design changed after `start`.   |
| **T6**  | A criterion has no check or judge, so only you can verify it.                     |
| **T7**  | The change is larger than `max_diff_lines` (300 by default).                      |
| **T8**  | The project's design doc changed.                                                 |
| **T9**  | The item has the label `needs-review`, which anyone can add to ask for a look.    |
| **T10** | A judge answered `unsure`.                                                        |
| **T11** | Judges failed the item in several different runs since `start`.                   |
| **T12** | The agent disputed a judge's fail.                                                |
| **T13** | A check or judge already passed at `start`, before the work existed.              |
| **T14** | A commit since `start` touches the project but isn't scoped to this item.         |

Each trigger can be switched off per project, except T6, T10 and T12, which would otherwise let unchecked work reach done. [DESIGN.md §2.5](docs/DESIGN.md#25-review-triggers-does-it-need-a-human) gives each one's exact rule.

### Configuration

Each project's settings live under its key in `~/.tracker/config.json`. Only `name` and `root` are required. The ones you're most likely to set:

```json
"review": {
  "risky_paths": ["src/storage.rs", "src/auth/**"],
  "source_paths": ["src/**"],
  "test_paths": ["tests/**"],
  "max_diff_lines": 300,
  "disabled": ["T8"],
  "digest": { "every_days": 7 }
}
```

The globs match paths relative to the project's root. A project's root can be the top of a git repository, or a folder inside a larger one, such as one package in a monorepo. Either way the tool looks only inside it. [DESIGN.md §4.2](docs/DESIGN.md#42-config) lists every setting.

## Command reference

Run `tracker <command> --help` for any command's options.

| Command                                                                      | What it does                                                            |
| ---------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| **Setup**                                                                    |                                                                         |
| `tracker init`                                                               | Create the store.                                                       |
| `tracker project add <KEY> --name … --root <path>`                           | Register a project.                                                     |
| `tracker project list`                                                       | List projects, with item counts by state.                               |
| **Planning**                                                                 |                                                                         |
| `tracker create "title" [--done-when "…"]… [--kind] [--priority] [--label]…` | Create an open item.                                                    |
| `tracker edit <ID> [--priority] [--label] [--unlabel] [--title] …`           | Change an item's details.                                               |
| `tracker criterion add <ID> "text" [--check "cmd"]`                          | Add a criterion with a check, or a manual one without.                  |
| `tracker criterion add <ID> --judge <file.json>`                             | Add a judge criterion.                                                  |
| `tracker criterion set <ID> <n> --check "cmd"`                               | Give criterion _n_ a check (or `--judge`), replacing what it had.       |
| `tracker evidence <ID> <n> "where to look"`                                  | Note where to look for a manual criterion's proof.                      |
| **Working**                                                                  |                                                                         |
| `tracker start <ID>`                                                         | open → in-progress. Runs each check once, expecting it to fail.         |
| `tracker gates <ID>`                                                         | Dry run: what would block a submit now?                                 |
| `tracker submit <ID> [--dispute <ref> --reason "…"]`                         | Run the gates and triggers: → done, or → review.                        |
| `tracker shelve <ID> --reason "…"`                                           | Put an item back in the backlog.                                        |
| **Reviewing (you, at a terminal)**                                           |                                                                         |
| `tracker review`                                                             | The inbox: every held item, with why it's held.                         |
| `tracker approve <ID>` / `tracker reject <ID> --reason "…"`                  | Decide a held item.                                                     |
| `tracker digest` / `tracker digest ack`                                      | Read what reached done without you, and mark it read.                   |
| `tracker reopen <ID> --reason "…"`                                           | Bring back a done or shelved item.                                      |
| `tracker judges`                                                             | How often you approved and rejected the items each judge question held. |
| `tracker delete <ID>`                                                        | Delete an item for good.                                                |
| **Looking around**                                                           |                                                                         |
| `tracker list [--status] [--priority] [--label] [--kind] [--all]`            | List items, grouped by project and sorted by priority.                  |
| `tracker show <ID> [--diff]`                                                 | Everything about an item. `--diff` adds its commits and their diffs.    |
| `tracker audit`                                                              | Look for signs that the store was edited by hand.                       |

## Where things are stored

Everything lives in one store, `~/.tracker`, or wherever `TRACKER_HOME` points. Set `TRACKER_HOME` to a temporary folder to try the tool without touching your real store.

```
~/.tracker/
├── config.json      # projects, their checks and review policy
├── counters.json    # the next number for each project, so IDs are never reused
├── deleted.json     # one line per deleted item, for the digest
├── digests.json     # when each project's digest was last acknowledged
├── judge/           # cached judge answers, and failed judge runs (T11)
└── items/
    ├── SL-1.json    # one plain JSON file per item
    └── SL-2.json
```

Each item is a plain, readable JSON file, so you can query the store with `jq`. Agents change items only through `tracker` commands, and `tracker audit` reports signs of hand edits.

## Limits

`tracker` raises the cost of a shortcut. It doesn't make one impossible. The main gaps:

- **A weak test written during the work still passes.** T13 catches a check that passes before the work exists, but a test written after `start` that asserts nothing fails at start, because it doesn't exist yet, and passes at submit. The digest is where you catch those.
- **The human-only commands rely on a convention.** `approve`, `reject`, `delete` and `digest ack` refuse a non-interactive shell, which stops an agent's normal commands. A determined agent could get around that, so the digest lists every human action for you to check.
- **The store is plain text.** A shell command could edit it. `tracker audit` detects that afterwards but doesn't prevent it.
- **Judges can be wrong.** A judge's answer is evidence, not proof. Scripts come first, judge-passed items lead the digest, and unsure answers and disputes come to you.
- **One item at a time per project.** Everything happens on the current branch, so a held item pauses its project until you decide.

[DESIGN.md §7](docs/DESIGN.md#7-honest-limits) lists every known limit, and [§6](docs/DESIGN.md#6-what-im-trusting-and-what-checks-it) maps each shortcut an agent might take to what catches it.

## Further reading

- [docs/DESIGN.md](docs/DESIGN.md): the full design and spec.
- [docs/walkthrough.html](docs/walkthrough.html): the tool in use, screen by screen, on a bookmarks app, with output recorded from `tracker 0.1.0` at `e5dbdd4`.
- [docs/PLAN.md](docs/PLAN.md): the build plan, in 16 vertical slices.
- [docs/slice-1-readme.md](docs/slice-1-readme.md) to [docs/slice-16-readme.md](docs/slice-16-readme.md): what each slice built, how it works, and how to test it.
- [docs/self-tracking-walkthrough.md](docs/self-tracking-walkthrough.md) and [docs/creating-items-walkthrough.md](docs/creating-items-walkthrough.md): how `tracker` was used to track its own build, from slice 7 on.
- [judge-eval/](judge-eval/): the evaluation used to choose the judge model.

To work on `tracker` itself, see [CLAUDE.md](CLAUDE.md). The test command, run from this folder, is `cargo fmt --check && cargo clippy -- -D warnings && cargo test`.

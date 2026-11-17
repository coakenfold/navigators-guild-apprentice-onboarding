# Build plan: `tracker`, a verification-gated work tracker

The build order for [DESIGN.md](DESIGN.md), in 16 vertical slices. DESIGN.md is the spec, and each slice's prompt cites its sections. DESIGN.md §9 points here and keeps the rules that apply to every slice. Each slice runs in its own conversation, started in `guild-portfolio/tracker/` so that Claude Code loads the **Shared context** from that folder's CLAUDE.md by itself. Give the agent that slice's **Prompt** and **Done when**. Update the **Status** column as each slice moves on:

- `todo`: not started.
- `in progress`: being built.
- `in review`: built, and its _Done when_ checks pass, but its reviews and their fixes aren't finished.
- `merged`: the last review's fixes are in, and its branch is merged into `main`. Set it in the pull request that merges the branch, not before.

The last value is `merged`, not `done`, because from slice 7 on each slice also has a tracker item, whose `done` state comes earlier: when `tracker submit` passes its gates, before any review.

**Approach:** layer, because this is a new project. §9 used to list 12 layers, and this plan re-sizes them rather than starting over:

- Layer 5 is split into two slices: show and delete, then bad data and polish.
- Layers 3 and 4 are merged.
- Layers 9–11 are re-cut into a human loop, two trigger slices, and four judge slices.
- The inbox, `approve` and `reject`, and T6 come before most triggers. In the old order, held items had no way out, and manual criteria auto-verified, until layer 11.

**Core (smallest useful version):** create items in two projects and list them from one central store (slice 1).

**Assumptions:**

- `tracker` is a general tool for any project. The `XX`, `YY` and `TR` keys below are test and self-tracking projects.
- `guild-portfolio/tracker/` is an ordinary folder in the guild-portfolio git repo, not a repo of its own. That has two consequences:
  - Slice 1 moves DESIGN.md, PLAN.md, walkthrough.html and `judge-eval/` there with `git mv`. Commit DESIGN.md's pending edits, and add `judge-eval/`, in guild-portfolio first, so the history follows the files.
  - TR's project root is a folder inside a larger repo, and so is any project registered the same way.
- **The folder rule:** a project root can be a folder inside a larger repo. G5's clean-tree check, T14's commit range and changed-file glob matching look only inside the project root (`git … -- <root>`). Git's repo-relative paths are made relative to the project root before glob matching. Files an item commit touches outside the root still count toward T7 and appear in `show --diff`. Self-tracking depends on this rule, because without it TR's submits would be refused whenever anything else in the portfolio was dirty. DESIGN.md §2.1 defines it, G5, T14 and §4.4 apply it, and §7 notes the trade-off: a dirty file outside the root that the project depends on isn't caught.
- Three commands weren't in any of the old layers. `project list` and "the current folder picks the default project" go in slice 1, and `create --done-when` goes in slice 6.
- For interactive-only commands, tests check automatically that piped stdin is refused. Whether typing the ID in a real terminal works is checked by hand. No pty test crate.
- The milestones stay as in §9. Slices 1–5 are the course submission to #adversarial-review, and self-tracking starts after slice 6.

## Shared context

The stack, where things live, the conventions and the test command are in `guild-portfolio/tracker/CLAUDE.md`, which Claude Code loads in any session started in that folder. Change them there; this plan doesn't repeat them.

## Slices

| #   | Slice                           | Outcome (I can…)                                                                                      | Depends on | Parallel with | Status |
| --- | ------------------------------- | ----------------------------------------------------------------------------------------------------- | ---------- | ------------- | ------ |
| 1   | Create and list                 | create items in two projects and list them from one store                                             | —          | —             | merged |
| 2   | States                          | start, submit, shelve and reopen items, with history kept                                             | 1          | —             | merged |
| 3   | Organise and filter             | set priority, kind and labels, edit items, filter a sorted list                                       | 2          | —             | merged |
| 4   | Show and delete                 | see an item's full record, and delete one from a real terminal                                        | 3          | —             | merged |
| 5   | Bad data and polish             | keep working past a corrupt file; get help text, colour and empty states                              | 4          | —             | in review |
|     | _Checkpoint: course submission_ |                                                                                                       |            |               |        |
| 6   | Criteria and simple gates       | write Done-when criteria; get refused when they're missing or incomplete                              | 5          | —             | todo   |
|     | _Checkpoint: self-tracking_     |                                                                                                       |            |               |        |
| 7   | Running checks                  | see checks run at start and submit, and fail with their output                                        | 6          | —             | todo   |
| 8   | Submit proves committed work    | get items auto-verified against a commit, with saved output and a pinned ref                          | 7          | —             | todo   |
| 9   | Inbox, approve, reject          | see held items in `review` and approve or reject them                                                 | 8          | —             | todo   |
| 10  | Risky changes are held          | have new capabilities, new dependencies, risky paths, edited tests, big diffs and design changes held | 9          | —             | todo   |
| 11  | Tampered proof is held          | have moved goalposts, checks that pass before the work, and unscoped commits held                     | 10         | —             | todo   |
| 12  | Judge criteria                  | have yes/no judges grade qualities at submit, using a fake judge command                              | 9, 11      | —             | todo   |
| 13  | Real Claude judge               | have judges answered by `claude-haiku-5-5`, with judge-eval running the same binary                   | 12         | —             | todo   |
| 14  | Judges must fail first          | have judges that pass before the work, or get retried until they pass, held                           | 11, 12     | —             | todo   |
| 15  | Disputes                        | dispute a judge's wrong fail and decide it myself                                                     | 9, 12      | —             | todo   |
| 16  | Digest, audit, judge report     | audit auto-verified and shelved items, the store, and each judge question's record                    | 15         | —             | todo   |

Nothing is marked parallel. Slices 1–6 all edit the same clap definitions and item structs. From slice 7 on, the tool's own G1 rule allows only one item in progress per project.

### Slice 1: Create and list

- **Prompt:** "Create the `tracker` crate with `cargo init` in `guild-portfolio/tracker/`, a folder in the existing guild-portfolio git repo that already holds CLAUDE.md. Don't make it a nested repo, and add a `tracker/.gitignore` for `/target`. Use `git mv` to move `DESIGN.md`, `PLAN.md`, `walkthrough.html` and `judge-eval/` there from `chapters/02-the-methodology/02-tracking-your-work/`. Set up the test harness: assert_cmd and tempfile, with every test on its own temporary `TRACKER_HOME`. Implement `init`, `project add`, `project list` (item counts by state), `create "title" [--description]` and `list` as §3 and §4.1–4.3 describe:
  - one item file per item
  - IDs taken from `counters.json` under an exclusive lock on the store's `lock` file
  - the project defaulting to the one whose root contains the current folder

  Every command supports `--json`."

- **Done when:**
  - Three creates in two projects get `XX-1`, `XX-2` and `YY-1`. `list` shows all three, and `project list` shows the counts.
  - Inside XX's root, `create` needs no `--project`. Outside every project root, without `--project`, it exits 1.
  - An empty or whitespace-only title exits 1. A description is stored in `description`.
  - An item file read back and written again is byte-for-byte unchanged.
  - Ten `create` commands started at once get ten different IDs.
  - Every command's `--json` output parses.
  - DESIGN.md's links to `judge-eval/`, PLAN.md and walkthrough.html still work. After `cargo build`, `git status -- tracker` shows nothing from `target/`.

### Slice 2: States

- **Prompt:** "Add the states and moves from §2.2 with `start`, `submit`, `shelve --reason`, `reopen --reason` and the `status <id> <state>` alias. A move the diagram doesn't draw exits 1 and lists the moves allowed from the current state. For now `submit` moves an item straight to done, with a verification record of `by: "unchecked"`, the time, and a note that no gates ran. Every move adds one history entry with `from`, `to`, `via` and `interactive`; slice 16's digest relies on `interactive`."
- **Done when:**
  - An item goes open → in-progress → done → open, both through the commands and through `status`. `status <id> open` without `--reason` exits 1.
  - `shelve` works from open and from in-progress, and exits 1 without a reason. `reopen` brings a shelved item back to open.
  - Open → done exits 1 and lists the allowed moves.
  - Every move adds exactly one history entry.

### Slice 3: Organise and filter

- **Prompt:** "Add `--kind`, `--priority` and repeatable `--label` to `create`, using §2.1's values and defaults; labels are lowercased and de-duplicated. Add an `edit` command for priority, labels (`--label` and `--unlabel`), title, kind, design and description; it adds one history entry, with `via: "edit"`, `from` equal to `to`, and `fields` naming what changed (§2.1), and is refused for done items. `list` sorts by priority within each project, marks priority and kind, and gets the combinable filters `--project`, `--status` (comma-separated), `--priority`, `--label`, `--kind` and `--by`. `--by` checks that the item is done as well as its `verification`, because a file written by slice 2 can hold a record on a shelved item. Without `--status`, `list` shows only open, in-progress and review items. Inside a project root, `list` shows only that project unless `--project` or the new `--all` says otherwise, and both its text and its JSON (`project`: the key, or `null`) say which it covered, as §3 describes."
- **Done when:**
  - `list` shows high before medium before low.
  - An unknown priority or kind exits 1 and names the allowed values. `--label Bug --label bug` is stored once, as `bug`.
  - `edit --priority low --unlabel bug` changes the item and adds one history entry. `edit` on a done item exits 1.
  - `list --status open --priority high --label bug` shows only items that match all three.
  - `--status done` shows only done items, and `--status open,shelved` shows exactly those two states.
  - Plain `list` hides shelved items.
  - Inside XX's root, `list` shows only XX's items and its JSON has `"project": "XX"`. `--all` there, or plain `list` outside every root, shows every project with `"project": null`.

### Slice 4: Show and delete

- **Prompt:** "Add `show <id>`: every field, the verification record, and history with timestamps. Add `delete <id>` as §2.8 describes, including its line in `deleted.json`. Build one shared helper for interactive-only commands: it refuses unless `tty::is_interactive()` (the check that already sets history's `interactive`), prints what will be lost, and requires the ID typed back. Later slices reuse it for `criterion accept`, `approve`, `reject` and `digest ack`."
- **Done when:**
  - `show` of a missing ID exits 1.
  - `echo XX-1 | tracker delete XX-1` is refused and the file remains.
  - By hand in a terminal, for an item in each state: the prompt names the state and what will be lost, typing the ID deletes it, and a wrong ID cancels.
  - Deleting `XX-2` and then creating an item gives `XX-3`.
  - A delete leaves one line in `deleted.json` with the ID, title, state, time and `interactive`, and nothing else of the item.

### Slice 5: Bad data and polish

- **Prompt:** "Handle bad data as §4.4 describes:
  - A missing store says 'Run `tracker init`.'
  - An item file that can't be parsed is reported with its name, line and reason, and skipped.
  - Every write goes to a temporary file that is then renamed over the original.
  - An unreadable store exits 3.

  Then polish: error messages that say how to fix the problem, colour (anstream/anstyle) that turns off when output isn't a terminal, `--help` text on every command and flag, empty states such as 'No open items. Nice work!', and a heading row naming `list`'s columns, as `project list` has. Decide whether a title needs a length limit, as a label (50) and a reason (500) have."

- **Done when:**
  - A hand-corrupted item file is reported with its name, line and reason, and the other items still list.
  - With no store, commands say "Run `tracker init`." and never panic. A store with `chmod 000` exits 3.
  - No temporary files are left in the store after any command.
  - Every command has `--help` text. `tracker list | cat` contains no colour codes. An empty list prints its empty-state message.
  - `list`'s text starts with a heading row that names each column, lined up with the rows below it.

**Checkpoint: course submission.** Send slices 1–5 to #adversarial-review with a short design covering only those slices that links to the full one (§9).

### Slice 6: Criteria and simple gates

- **Prompt:** "Add Done-when criteria as §2.3 and §3 describe:
  - `criterion add`, either with `--check` or manual
  - `criterion set <n> --check` and `criterion remove`
  - `evidence`, for manual criteria only
  - repeatable `create --done-when`, which adds manual criteria
  - the `written` fingerprints (sha2), taken at create
  - interactive-only `criterion accept`, which adds a history entry with `via: "criterion-accept"` (§2.1)

  Add gates G1 (at start: the item has criteria, and no other item in the project is in progress or review), G2 and G6. G6 reads `design` relative to the project's root, and fails a file that, after following symlinks, is outside it (§7). They run in a new `gates <id>` dry run and in front of `submit`. Any failure lists every reason, changes nothing and exits 2.

  G1 refuses `start` for an item with no criteria, and almost every test in `tests/states.rs` starts one that has none. Give those items a criterion, for example by having the shared test helper that creates them add one, so that the move tests still test moves."

- **Done when:**
  - `start` with no criteria is refused by G1 with exit 2, and so is `start` while another item in the project is in progress.
  - A manual criterion with no evidence fails G2. A `design` naming a missing section fails G6, and a numbered section with content under it passes. A `design` whose file is a symlink to one outside the root fails G6.
  - `gates` lists every failure and leaves the item file byte-for-byte unchanged. A refused `submit` exits 2 and also changes nothing.
  - `create --done-when a --done-when b` adds two criteria and two `written` fingerprints. `evidence` on a script criterion exits 1.
  - `criterion accept` is refused outside an interactive terminal.

**Checkpoint: self-tracking starts.**

1. Run `cargo install --path .`, then `tracker init` and `tracker project add TR --name tracker --root <guild-portfolio>/tracker --design docs/DESIGN.md`.
2. Create items for slices 7–16 from this plan. The outcome becomes the title, "Done when" becomes the criteria, and a slice that adds new behaviour gets the kind `capability` (§5).
3. Add DESIGN.md §5's settings and its block for `~/.claude/CLAUDE.md` yourself, trimming the lines about commands that don't exist yet.

### Slice 7: Running checks

- **Prompt:** "Run script checks as §2.4 and §4.2 describe:
  - G3 runs each criterion's `check`, and G4 runs the project's `checks`.
  - Each is a shell command run in the project root. The tool prints it before running it and stops it after `check_timeout_secs`.
  - A failure reports the command, exit code and last lines of output.

  After G1, `start` runs each criterion check once, records the results in `started.checks`, and warns, without refusing, about any check that already passes. Keep the last 20 lines of each check's output for the verification record."

- **Done when:**
  - A failing check shows its command, exit code and last lines of output.
  - A check running longer than its timeout is stopped and fails.
  - A broken project check fails G4 even when every criterion check passes.
  - `start` records a check that fails, as expected, and warns about one that already passes.
  - A passing check's last 20 lines are kept.

### Slice 8: Submit proves committed work

- **Prompt:** "Connect `submit` to git as §2.1, G5 and §2.7 describe:
  - An item's commits are those reachable from `HEAD` whose header matches `^[a-z]+\(<ID>\)!?: `.
  - `start` records `started.head`.
  - G5 requires a clean working tree and at least one item commit. The clean-tree check is limited to the project root by the folder rule in the assumptions, and ignored files don't count.

  A passing submit now auto-verifies. It writes the full verification record and creates `refs/tracker/<ID>`, which `delete` removes. Shelving an in-progress item needs a clean tree and passing project checks. `submit` re-reads the item under the lock and writes nothing (exit 1) if it changed while the gates ran. A project root that isn't inside a git work tree exits 3."

- **Done when:**
  - In a throwaway repo:
    - An uncommitted change fails G5, and so does an untracked file.
    - An ignored file doesn't fail it, and neither does a dirty file outside a subfolder project's root.
  - `feat(XX-1): …` and `fix(XX-1)!: …` count as XX-1's commits. These don't:
    - a commit scoped to `XX-12`
    - one headed `XX-1: …`
    - one that names XX-1 only in its body
    - a `feat(XX-1)` commit on another branch
  - A passing submit records the SHA and each check's result, duration and output, and creates the ref. Deleting the item removes the ref.
  - The commit is still readable after its branch is rebased away and `git gc --prune=now` runs. After the test is renamed, `show` still prints the saved output.
  - Shelving an in-progress item is refused while the tree is dirty or a project check fails, and allowed once both are fixed.
  - If a check edits the item file while the gates run, `submit` writes nothing and exits 1.

### Slice 9: Inbox, approve, reject

- **Prompt:** "Add triggers that run after every gate passes and move the item to review with its reasons in `holds`, starting with T6 (manual criterion) and T9 (the `needs-review` label). Add these commands:
  - `tracker review`, the inbox
  - interactive-only `approve`, which re-runs the gates and then either moves the item to done with `by: "human"` or leaves it in review with exit 2
  - `reject --reason`, which sends the item back to in-progress
  - `show --diff`, which prints the item's commits and their combined diff

  `status` refuses an item in review and points to `approve` and `reject`.

  Add the diagram's arrows to the move table: `submit` from in-progress to review, `approve` from review to done, and `reject` from review to in-progress. `states::find` returns the first row for a command and state, so change it to answer whether the command has any move from that state, and let `submit` pick done or review from the triggers. `allowed` keeps one entry per arrow, each with a single `to`, as §3 describes."

- **Done when:**
  - A manual criterion with evidence is held under T6. A `needs-review` item is held under T9. An item with neither still auto-verifies.
  - `submit --json` matches the held shape in §3.
  - `review` lists held items with their reasons and the `show --diff` hint, and prints "Nothing needs you." when it's empty.
  - `echo XX-1 | tracker approve XX-1` is refused. Typing the ID in a real terminal moves the item to done (checked by hand).
  - An approve whose gate re-run fails leaves the item in review. A reject puts its reason in history.
  - Every move refused for an item in review, through a command or through `status`, names `approve` and `reject`, and its `allowed` lists both. Until now the message says this version can't do either, and `allowed` is empty.
  - `show XX-1 --diff` includes XX-1's commits and none of XX-12's.
  - A refused move from in-progress lists `submit` twice in `allowed`, once to done and once to review, with the same command.

### Slice 10: Risky changes are held

- **Prompt:** "Work out each item's changed files and changed source files as §2.1 defines them:
  - Deleted files count, and `source_paths` excludes `test_paths`.
  - Paths are made relative to the project root before glob matching.
  - Files outside the root still count toward T7.

  Add triggers T1, T2, T3, T4, T7 and T8, with §4.2's `review` config and its defaults, including `disabled`, which never switches off T6, T10 or T12. T4 fires only when an existing test file had lines changed or removed, or was deleted. `show --diff` lists the files that fired a trigger first."

- **Done when:**
  - Each of the six triggers has one test where it fires and one where it stays quiet.
  - Adding a test case to an existing test file doesn't fire T4. Changing a line in one does, and so does deleting one.
  - In a subfolder project, `source_paths: ["src/**"]` matches `<root>/src/…` and not a `src/` folder elsewhere in the repo.
  - `"disabled": ["T8"]` silences T8, but `["T6"]` doesn't silence T6.
  - After a reject, a resubmit is held again for every trigger that still applies.

### Slice 11: Tampered proof is held

- **Prompt:** "Add three triggers:
  - **T5**: compares a fingerprint of the criteria, kind and design taken at `start`, and also fires whenever a written criterion's text no longer matches `written`. `criterion accept` clears my own hand edits.
  - **T13**: fires when a criterion's script check already passed at start.
  - **T14**: fires for unscoped commits between `started.head` and `HEAD` that touch the project root.

  Compute the criteria fingerprint over each serialized criterion with its evidence removed. That way slice 12's judge fields are covered without changing the fingerprint."

- **Done when:**
  - T13 fires for a `cargo test` filter that matched no tests at start.
  - T5 fires when a non-interactive command changes or removes a written criterion's text before start, and stays quiet when one adds a criterion or gives one a check.
  - `edit --kind` after start fires T5.
  - T14 fires for an unscoped commit after start, including one headed `XX-1: …`. It doesn't fire for one made before start, or for an unscoped commit that only touches files outside the project root.
  - A reopened item resubmitted with unchanged code is held by T13.

### Slice 12: Judge criteria (fake judge)

- **Prompt:** "Add judge criteria as §2.3.1 describes, calling the judge command through its stdin/stdout contract. In tests, a fake judge script prints fixed answers. Build:
  - `criterion add` and `criterion set` with `--judge <file>`, with validation
  - request building: the whole input, or one excerpt per `each` match with `context_lines`; comments stripped unless `keep_comments`; refused above `max_input_bytes`
  - `pass_when`, refs, and a cache keyed by request fingerprint in `judge/cache/`

  G3 runs judges only after every script gate passes. It fails on a no, a missing input, zero `each` matches or a failed call. An unsure answer, or `auto_verify: false`, holds the item under T10."

- **Done when:**
  - Tests cover:
    - a pass, a fail and an unsure answer
    - a `"pass_when": "no"` question
    - malformed output and a timeout
    - an oversized excerpt, refused before the judge is called
    - a cache hit, where the fake judge isn't called again
  - `--judge` refuses invalid JSON, an invalid regex and an `each` that matches nothing. `set` refuses a file whose `text` differs.
  - G3 fails a judge after a later commit removes every match. Judges don't run when a script check failed.
  - The fake judge receives the excerpts with comments stripped, and refs look like `c2:file:line:id`.
  - `--json` failures carry `ref`, `answer` and `reason`. A done item's verification record has one judge entry per question.

### Slice 13: Real Claude judge

- **Prompt:** "Build `src/bin/tracker-judge.rs` following §2.3.1's four steps: one `claude -p` call per question, with exactly the flags, system prompt and JSON schema shown there, run in an empty temporary folder. It fails on `is_error`, on a different model in `modelUsage`, or on an answer that isn't yes, no or unsure, and it replies with the adapter version, model, answers and summed `cost_usd`. Point judge-eval's `run.mjs` at it, and add criterion-judge cases, including injection attempts, to `cases.mjs`."
- **Done when:**
  - A fake `claude` on `PATH` that records what it's given confirms that tracker-judge sends the same arguments and prompt as `judge-eval/adapters/claude.mjs`. It also refuses a reply from a different model.
  - `cargo install --path .` installs both binaries.
  - `node run.mjs claude-haiku-5-5` runs tracker-judge.
  - By hand: one real item is judged from inside a Claude Code session, with the judge seeing no CLAUDE.md and having no tools.

### Slice 14: Judges must fail first

- **Prompt:** "Make judges fail first, as checks do (§2.4). G1 refuses `start` while a judge has nothing to grade, or while a `diff` judge has no item commit yet. `start` runs every judge once, and T13 holds an item whose judge answers all passed at start. Add T11 using `judge/failures.json`: each failed request fingerprint counts once, `gates` dry runs count, and fails at start don't."
- **Done when:**
  - `start` refuses a `diff` judge before the item has a commit.
  - A judge whose answers all pass at start is recorded as already passing and held by T13.
  - T11 holds an item after `max_failures` different failed requests.
  - An unchanged failure that `gates` repeats counts once, and fails at start don't count.

### Slice 15: Disputes

- **Prompt:** "Add `submit --dispute <ref> --reason "…"`, repeatable, as §2.3.2 describes. It holds the item under T12 when the disputed questions are the only failures, and exits 1 for a ref the judge didn't fail. `approve` counts questions held under T10 or T12 as passed for that approval only. Both `approve` and `reject` record my decision against each held question."
- **Done when:**
  - A dispute holds the item under T12, showing the excerpt, the model's answer and reason, and the agent's reason.
  - Disputing a question that passed or got an unsure answer exits 1.
  - A dispute doesn't stop another failing gate from refusing the submit.
  - Approving a T12 hold moves the item to done.
  - Resubmitting unchanged code after a reject needs the dispute again. T12 can't be disabled.

### Slice 16: Digest, audit, judge report

- **Prompt:** "Add per-project digests as §2.6 describes:
  - `digest [--project]` lists auto-verified items, judge-passed ones first with refs, answers and reasons. It also lists every shelve, every human action (`approve`, `reject`, `delete`, `criterion accept`), mine included, and every `edit` whose history entry shows it removed `needs-review` or changed kind away from `capability`, each marked by whether it came from an interactive terminal, and says when the digest was last acknowledged.
  - `digest ack` records what was seen in `digests.json`. It confirms through `tty::confirm`, asking me to type `ack` rather than an ID (§2.7). It builds the digest before asking, then, under the lock, builds it again and compares the two. If they differ, it records nothing and exits 1, as `Store::lock_unchanged` does for an item, so an entry added while I read is never acknowledged unseen.
  - `review` flags a digest older than `every_days` as overdue.

  Add `audit` and `judges` (§3), and bring `--help` text and empty states up to slice 5's standard for every command added since."

- **Done when:**
  - `digest` lists every item auto-verified since that project's last ack, judge-passed first. `digest ack --project XX` empties only XX's digest.
  - From a terminal, `digest ack` asks for `ack`. Typing the project key, or anything else, cancels with exit 1. An item auto-verified in that project while the question waits makes it exit 1, record nothing, and leave that item in the next digest.
  - A shelve from piped stdin and one run under `script` (skipped where `script` isn't installed) are both listed, marked not from a terminal and from a terminal. So are a delete and an approve made since the last ack, an edit that removes `needs-review`, and one that changes kind from `capability` to `change`. An edit that only adds a label isn't listed.
  - `audit` reports a hand-edited status, a done item with no verification record, and an ID used twice.
  - `audit` reports a `deleted.json` line whose ID is at or above its project's counter in `counters.json`. Such a line can only have been written by hand, and `create`'s tripwire refuses that project until the counter is raised (slice 4 review 1, finding 7).
  - `judges` shows the approve and reject counts for each question.

## Deferred / out of scope

- Everything in DESIGN.md §1's non-goals.
- Saved rulings for each question, mutation testing, and a branch or worktree per item (§7).

## Integration risks

- **Item files change shape** in slices 6, 7, 8, 9 and 12, and items created by an older binary must still load in the newer one. Keep slice 1's test that a file comes back byte-for-byte unchanged, and add one for an item file saved by slice 5.
- **The T5 fingerprint must not change** while an item is in progress, or T5 fires on that item. Install a new binary only between items.
- **The bootstrap:** track each slice with the binary installed from the previous slice (`cargo install --path .` after each), not with the code you're changing. Until slice 9 lands, anything that passes the gates is auto-verified, manual criteria included. So read the verification records for items 7–9 yourself.
- **Self-tracking depends on the folder rule.** TR's root is a folder inside the guild-portfolio repo, so slices 8, 10 and 11 must test it with a project in a subfolder of a throwaway repo.
- **Interactive-only commands are only half tested.** The tests check that piped input is refused; whether typing the ID works is checked by hand. Try each new interactive command once in a real terminal.
- **The two judge adapters** (Node in judge-eval, Rust in tracker-judge) can drift apart. Slice 13's test that they send the same arguments and prompt is the guard, so re-run it after changing either.
- **Re-reading after the lock:** slice 9's `approve` needs the same re-read under the lock as slice 8's `submit`.

## Build loop

Build one slice, then run its new checks **and** the full existing suite, commit, and move to the next slice. A regression in slice 3 should surface before later slices depend on it. From slice 7 on, the loop is `tracker start` → build → `tracker submit`, and the slice's row here becomes `in review` when its item reaches done. It becomes `merged` only once its review fixes are in and its branch is merged, as for slices 1–6. A review's fixes are new items, unless the review finds that the item doesn't meet its own criteria, which reopens it (DESIGN.md [§2.9](DESIGN.md#29-reopen-vs-a-new-item)).

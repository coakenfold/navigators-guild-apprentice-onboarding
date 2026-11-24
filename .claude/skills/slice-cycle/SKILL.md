---
name: slice-cycle
description: Orchestrate the tracker crate's remaining PLAN.md slices as stacked `gh stack` layers, tracked in tracker itself. Each slice is built, given a readme, reviewed twice with fixes after each review, then installed before the next layer. Works out where the run stands, then continues until the user is needed. Invoke explicitly with /slice-cycle.
disable-model-invocation: true
---

# Run the slice cycle

You're the orchestrator. Subagents do the building, writing and reviewing. You pick what comes next, sort review findings into tracker items, check each agent's work, and stop when the user is needed. Keep your own context small: never read DESIGN.md, `src/main.rs` or whole reviews yourself when a `grep` or a few lines will do.

Paths are relative to the guild-portfolio repo root. The crate is `tracker/`, and its plan is `tracker/docs/PLAN.md`. The agent prompts sit next to this file. Give agents their absolute paths:

- `brief.md`: hard rules and the tracker workflow. Every agent reads it first.
- `build-slice.md`: building one slice.
- `fix-round.md`: building a round of review fixes.
- `review-slice.md`: writing one review with `/slice-review`.

## 1. Find where the run stands

Work it out from the repo and the store each time. Don't trust a summary.

1. `git branch --show-current` and `gh stack view` (or `git log --oneline -8`). The current layer is `feat/tracker/slice-N`.
2. `tracker list --project TR --json`: any item `in-progress` or `review`? Slice N's item is TR-(N−4) for slices 8 to 16. Review fixes are later IDs, labelled `review`.
3. Which of `tracker/docs/slice-N-readme.md`, `tracker/docs/reviews/slice-N-review-1.md` and `-review-2.md` exist, and whether each review ends with `## Outcome`.
4. `git status --short`: anything uncommitted under `tracker/`?

Then pick the step below. If an item is in `review`, the user hasn't approved it yet: say what it's waiting for, and stop. If an item is `in-progress` with no agent running it, read its record and resume or ask.

## 2. The cycle, for slice N

1. **Build.** `general-purpose` agent. Prompt: read `brief.md` and `build-slice.md`; N, the item ID and kind, the branch and what it's stacked on; which binary is installed and which triggers apply; read the previous slice's last review's "Slice N must decide" and both its Outcomes. A capability is held by T1: expected.
2. **Check it, then hand over to the user** (see 4). Wait for their approve.
3. **Readme.** `tracker-docs` agent: write `docs/slice-N-readme.md` by `/slice-readme`'s steps, modelled on the previous slice's readme. Commits are `feat/tracker/slice-(N−1)..feat/tracker/slice-N`. Give it the item's facts: commits, holds, which checks failed at start, `human` or `auto` at which commit, and anything the user decided. Commit `docs: add the slice N readme`.
4. **Review 1.** `general-purpose` agent: read `brief.md` and `review-slice.md`; N, round 1, the file to write, the range, the item and how it was verified, what later slices need. It doesn't commit. You commit `docs: save slice N review 1`.
5. **Sort the findings** (see 3) into fix items with `tracker create`, then build them with a `tracker-fixer` agent: read `brief.md` and `fix-round.md`; the review file, the item IDs in order, which binary is installed. If an item is held, check it and hand over (see 4).
6. **Readme update and Outcome.** `tracker-docs` agent, two commits, readme first: `docs: record slice N review 1's fixes in the slice N readme` (a "Fixes after review 1" section), then `docs: save slice N review 1's outcome`. The Outcome goes at the very end of the review, modelled on an earlier review's Outcome. Add the pointer sentence to the end of the Verdict: "What was done about each finding, after the review, is under [Outcome](#outcome) at the end." List every finding with what resolved it or why it won't be fixed, and give the "Seen failing" facts.
7. **Review 2.** As step 4, round 2. Tell the agent review 1's Outcome holds the triage, and what review 1's fixes were. When you commit it, run `cargo test --test docs_links` first. If it fails because a review's links are checked at that review's last commit, add a one-line pointer to the newer review at the end of the older one, in the same commit.
8. **Sort and fix** as step 5.
9. **Final readme and Outcome.** As step 6, for review 2: "Fixes after review 2", and correct any readme claims review 2 found wrong.
10. **Install and add the next layer.** Make sure no TR item is `in-progress` or `review`. Then `cargo install --path .` from `tracker/`, `gh stack add feat/tracker/slice-(N+1)` from the repo root, and `gh stack submit --auto` once the new branch has commits. Go to step 1 for slice N+1.

Push after each slice's docs commits land, with `gh stack submit --auto` (it pushes every branch in the stack) or `git push -u origin <branch>`. A bare `git push` fails on a branch `gh stack add` made, since it has no upstream, and `git push -q` hides the failure. After pushing, check that `git rev-parse <branch>` and `git rev-parse origin/<branch>` match. Never push while an agent is mid-commit.

## 3. Sorting review findings

- Take the review's recommended option unless it's over-engineered. Prefer the plain fix for the likely case. Skip unlikely edge cases, and say why in the Outcome.
- Group the fixes into as few items as you can, so the user approves once. A fix that changes DESIGN.md is held by T8, so fold docs fixes into one item with the code fixes. A tests-only fix passes at start and T13 holds it, so put it in an item that's held anyway.
- A fix that can avoid every trigger (no DESIGN.md, no existing test edited, kind `fix`, under 300 lines) may verify `auto`, so keep those apart when they're cheap.
- Defer what belongs to a later slice, or what no project needs yet, as a low-priority `tracker create` with the review and finding in its description.
- A finding that fits a later item can be folded into it instead: `tracker criterion add` on the next slice's item before it starts (its written criteria stay as they are), or `tracker edit <ID> --title/--description` plus `criterion add` on an item that hasn't started. A round whose findings all fold away needs no fix item, and goes straight to step 9.
- Process findings go into `brief.md` as rules, or are yours to do. One that asks you to check the installed binary can be done right after the install, before the final docs, since no item is open then.
- Every criterion is scriptable. Docs criteria are `grep -qF` checks, chained with `&&`, and removed text is `test -f F && ! grep -qF '…' F`. A manual criterion holds the item under T6.
- Write criteria as what must be true, and put the finding numbers and chosen options in the description, so the fixer needn't read the whole review.

## 4. Before asking the user to approve

Check the item yourself with `tracker show <ID> --json`:

- `written` texts are all still criteria (none reworded or removed). Note any added ones.
- `started.checks`: which failed at start. A pass at start is a T13 hold. Make sure that was expected.
- `history` is only `start`, `submit`.
- `git log --format='%h %s' <started.head>~1..HEAD`: every commit is scoped to the item.
- For a T4 hold, read the diff of the existing tests it names. For T8, read the DESIGN.md diff. Summarise both.

Then tell the user, in a table, each hold and exactly what to read in `tracker show <ID> --diff > ~/Desktop/<id>.diff`, which decisions to look at, and what manual criteria rest on. End with: run `tracker approve <ID>` or `tracker reject <ID> --reason "…"` in a terminal, and let me know. Stop until they do. After they say so, confirm `verification.by` is `human`.

## 5. Agents

- `general-purpose` for builds and reviews. `tracker-docs` (Sonnet) for readmes and Outcomes, and tell it which commits to make, since its agent file says not to commit. `tracker-fixer` (Sonnet) for fix rounds.
- An agent stopped by a usage limit keeps its work. Resume it with `SendMessage` after the reset. Don't start a new one.
- Read each agent's report, then check its claims that matter with `git log`, `git status` and `tracker show`. If a commit carries an attribution line, amend it out before pushing.
- Agents never call the real `claude` except where a criterion needs a real judge run, and never run `cargo install`.

## 6. Standing decisions by the user

- TR keeps every trigger on. Never add `review.disabled` or suggest weakening a trigger to keep the run going. Held items wait for the user's approve.
- An Outcome goes at the bottom of its review, with a pointer from the Verdict.
- Bootstrap: track each slice and its fixes with the previous slice's installed binary. Install only between items, never mid-item.
- No unscoped commit and no branch move under `tracker/` while a TR item is in progress or in review.
- No attribution lines in commit messages. PR descriptions end with the Claude Code line.
- The judge-eval reruns with the installed binary, and a Sonnet run, are skipped unless the user asks.
- Disk is limited. Agents delete their scratch copies, and nobody copies `tracker/target`.

## 7. Reporting

After each step, tell the user in a few lines what finished, what's running, and what you need from them, if anything. When you're waiting on their approve, say so plainly at the end.

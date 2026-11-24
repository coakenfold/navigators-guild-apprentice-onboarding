# Review a slice

This is a read-only review: change no code, tests or tracker items. `tracker show <ID>` and `tracker list` are read-only and fine against the real store.

Run the project's `/slice-review` skill for the slice number the orchestrator gives you. It can't be invoked as a tool, so read /Users/chadoakenfold/Documents/Code/guild-projects/guild-portfolio/.claude/skills/slice-review/SKILL.md and follow every step exactly.

- **Stacked branches.** Each slice's branch is stacked on the previous slice's, and none is merged into `main`. So the slice's commits are `git log --oneline --reverse feat/tracker/slice-<N-1>..feat/tracker/slice-<N>`, and its diff is over that same range, not from `main`'s merge base. Wherever the skill says `$(git merge-base main HEAD)`, use `feat/tracker/slice-<N-1>` instead. That includes the DESIGN.md diff.
- **Tracker items.** Each slice is tracked as an item, and its review fixes are items too. The orchestrator names them. Check each item's record (`tracker show`). Compare when its tests were committed with its `start` time, and check what its checks ran at submit.
- **Outcome.** Don't write an `## Outcome` section: it's added later, at the bottom of the file. Leave the Verdict's pointer line out too, because it would dangle until the Outcome exists. The orchestrator adds both later.
- **Earlier decisions.** For review 2, review 1's Outcome records the triage, including what was deliberately not fixed. Don't report those again as new findings. List them under "Still open, carried forward".
- **Judgement.** Rank findings by how likely and how harmful they are. Say plainly when a finding is an unlikely edge case.

Don't commit. Final report, short: the path, the Verdict in two or three lines, every finding as one line (section, number, bold claim, how likely it is, and the suggested fix with the option you'd pick), and anything you couldn't check.

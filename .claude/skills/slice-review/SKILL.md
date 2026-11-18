---
name: slice-review
description: Review how faithfully the `tracker` crate implements one slice of PLAN.md, and how well it follows DESIGN.md, and save the report as the next tracker/docs/reviews/slice-N-review-M.md. Invoke explicitly with /slice-review N. It writes review 1 when none exists, review 2 when review 1 exists, and so on.
argument-hint: "N"
disable-model-invocation: true
---

# Review a slice

The slice number: `$ARGUMENTS`, such as `4`. Take the first number in it and ignore any word around it, so `Slice 4` also works. If there's no number, ask which slice.

All paths below are relative to the guild-portfolio repo root (`git rev-parse --show-toplevel`). The crate is `tracker/`.

## Step 1: Pick the review number

List `tracker/docs/reviews/slice-N-review-*.md`. The new review's number M is one more than the highest number there, or 1 if there are none. The report goes to `tracker/docs/reviews/slice-N-review-M.md`.

**Never overwrite an existing review.** If the file for M already exists, stop and say so.

Check that PLAN.md has a `### Slice N: …` heading. If it doesn't, stop and say so.

## Step 2: Read

Read what bears on the slice, and read that part in full. The big files aren't read whole, since a review costs what it reads.

- `tracker/docs/PLAN.md`, in full. Later slices and the integration risks bear on whether this slice is right.
- `tracker/docs/DESIGN.md` (about 130 KB), never whole. List its headings with `grep -n '^#'`, then read these sections in full, with Read's `offset` and `limit`: each one slice N's _Prompt_ and _Done when_ name, each one that `git diff $(git merge-base main HEAD)..HEAD -- tracker/docs/DESIGN.md`, run from the repo root, changes, and §7, its honest limits. Read another section when a finding needs it.
- For review 2 and later: the latest earlier review of this slice, `tracker/docs/reviews/slice-N-review-<M-1>.md`, in full, and only the `## Outcome` section of each one before it.

Then read:

- `tracker/CLAUDE.md`, for the conventions and the test command
- `tracker/docs/slice-N-readme.md`, if it exists. Treat its claims as things to check, not as facts.
- The latest review of the slice before, if there is one, for its "Slice N must decide" section

## Step 3: Review

The question: **for what has been developed, has it faithfully implemented Slice N of PLAN.md, and is it in line with DESIGN.md**, as far as it can be while the tool is still being built? Find every gap, shortcoming, bug, security issue and design deviation you can.

Work in `tracker/`. **Never run `tracker` without `TRACKER_HOME` set to a temporary folder.** Without it, the tool writes to the real store at `~/.tracker`. Don't change any code or test: this step only reads and runs.

1. **Pin what you're reviewing.** The branch is `feat/tracker/slice-N`. Record the commit you review (`git rev-parse --short HEAD`), whether the working tree is clean (`git status --short -- .`), and the slice's commits: `git log --oneline --reverse $(git merge-base main HEAD)..HEAD`, or the merge commit's range if it's merged. For review 2 and later, say which commits are new since the last review's pinned commit.
2. **Read the code.** Read the slice's diff against `main`, over the same range as the commits above. Read a changed source file, test file or schema in full only if it's under about 1,000 lines. For a longer one, read the functions the diff touches, and search for their callers. Never read `src/main.rs` whole (`tracker/CLAUDE.md`). Search the unchanged files for enough to see how the new code fits in, rather than reading them.
3. **Run the test command** from CLAUDE.md, plus `cargo clippy --all-targets -- -D warnings`. Run `cargo fmt --check` and both clippy commands as they are, and `cargo test 2>&1 | grep -E '^test result|FAILED|panicked'`, which prints a line per test binary rather than per test, and still shows a failing test's name and panic. Record what passes and the test count.
4. **Check each _Done when_ bullet**, and each concrete requirement in the slice's _Prompt_, against the code and the tests. Say whether it passes, and which tests cover it. A bullet with no test is either checked by hand (say how) or uncovered.
5. **Check against DESIGN.md.** For each DESIGN.md section the slice touches, compare what the code does with what the section says. Flag where they disagree, and where the design is unclear enough that the code had to guess.
6. **Look for bugs and security issues**: bad input, hand-edited or corrupt files, files from a newer tracker, concurrent runs, the lock, atomic writes, escaping of free text, paths, and anything an agent could misuse. **Reproduce** each one you can against the debug binary in a temporary store, and say whether each finding was reproduced or found by reading the code.
7. **For review 2 and later**, check each earlier finding: is the fix its Outcome claims really in the code, does a test guard it, and was anything left open? Don't report a fixed finding again as new. List what's still open under its original review and number.
8. **Check the readme's claims**, if it exists: test counts, commands, what's built, and what's left open.

## Step 4: Write the report

Match the earlier reviews in `tracker/docs/reviews/`. The order below is their layout. For their tone, read the latest review's Verdict and one of its findings, unless step 2 already read it in full. Use this order, and leave out a section that would be empty:

1. `# Slice N review M`
2. An opening paragraph: what this reviews (slice N, with PLAN.md's title), against what (PLAN.md's Slice N prompt and _Done when_ list, the DESIGN.md sections it touches by number, CLAUDE.md's conventions, the readme's claims), and for review 2 and later, the earlier reviews' outcomes.
3. A two-column table with an empty header row and the rows `Date`, `Branch` (branch, commit, clean or not, which commits are the slice), `Method` (what you read, ran and reproduced) and `Line references` (`Point at <commit>`).
4. `## Verdict`: whether the slice does what the plan asks, the test results, what it gets right, and the findings that matter most, each linked by number.
5. `## Done when`: a table of Check | Result | Tests.
6. `## Bugs`, `## Security`, `## Where the code and DESIGN.md disagree, or the design is unclear`, `## Test gaps` and `## Process`, with numbered findings in each. Each finding starts with a bold one-line claim, then the evidence (with links), how it was reproduced, why it matters, and a concrete fix. When there's a choice, give the options as **(a)**, **(b)** and say which you'd pick.
7. `## Still open, carried forward` (review 2 and later): open findings from earlier reviews, each named by review and number.
8. `## Slice N+1 must decide`: what the next slice in PLAN.md has to settle because of what this one found.

**Don't write an `## Outcome` section.** It's added after the fixes, at the end of the file.

### Links and style

- The report lives in `tracker/docs/reviews/`, so links are relative to it: `[PLAN.md](../PLAN.md)`, `[DESIGN.md](../DESIGN.md)`, `[slice-N-readme.md](../slice-N-readme.md)`, `[review 1](slice-N-review-1.md)`, `[main.rs:1449](../../src/main.rs#L1449)`, `[tests/show_and_delete.rs](../../tests/show_and_delete.rs)`.
- **Line anchors (`#L12` or `#L12-L20`) are fine here**, unlike in the readmes, because the review pins its commit. They must be the lines at that commit.
- Plain words and short sentences, with contractions, and Canadian/British spelling ("colour", "behaviour", "organise").
- Exact counts, real test names, real commit hashes and real output. Never estimate or make one up.
- Name slices by number ("slice 6"). Cite DESIGN.md as `§2.7`.

## Step 5: Verify

1. Run `cargo test --test docs_links` in `tracker/`. Fix every failure in the report and run it again until it passes.
2. Check by eye that every commit hash you cite appears in step 3's `git log`, and that every finding referred to by number exists.

## Step 6: Report

Don't commit. Tell the user:

- The path of the report.
- The Verdict in two or three lines, and the number of findings in each section.
- Anything you couldn't check or reproduce.
- The commit message to use when they're ready: `docs: save slice N review M`.

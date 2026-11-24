---
name: slice-readme
description: Write or update tracker/docs/slice-N-readme.md, a self-contained document for developers who will build later slices of the `tracker` crate or review a slice's work, and who weren't in the sessions that built it. Invoke explicitly with /slice-readme Slice N: Title.
argument-hint: "Slice N: Title"
disable-model-invocation: true
---

# Write a slice readme

The slice: `$ARGUMENTS`, for example `Slice 3: Organise and filter`. If it's empty, ask which slice. Take the number from it, and check the title against PLAN.md's `### Slice N: Title` heading. If they differ, use PLAN.md's title and say so at the end.

All paths below are relative to the guild-portfolio repo root (`git rev-parse --show-toplevel`). The crate is `tracker/`. The doc is `tracker/docs/slice-N-readme.md`, with N in digits.

## Who reads it

Developers who will build a later slice, or review this one, and who weren't in the sessions that built it. They have the repo and nothing else: no chat history, no memory of what was discussed. So the doc must:

- **Stand on its own.** Never write "as discussed", "we decided", "earlier" or anything else that points back to a conversation. When a decision matters, state it and why, or link to the review or DESIGN.md section that records it.
- **Link, don't repeat.** DESIGN.md is the spec and the source files are the truth. Say in a clause what the spec requires, and link to the section. Say what the code does and which values it chose, and link to the file and name the function. Don't copy DESIGN.md's prose, tables, examples or diagrams, and don't paste source code. Only quote what the slice itself produces: command output, item files, JSON.
- **Be exact.** Give real numbers, real test names, real commit hashes and real output, all gathered in step 2. Never estimate them or make them up.
- **Be honest.** If a _Done when_ check fails or has no test, a review finding isn't fixed, or something was only checked by hand, say so plainly in the Status row and in the section it belongs to.

## Step 1: Read

Read these first. Later slices, integration risks and DESIGN.md's limits all bear on what this slice must say.

- `tracker/docs/PLAN.md`, **in full**, from first line to last. Don't skim, grep or read only the slice's part.
- `tracker/docs/DESIGN.md` (about 140 KB), never whole. List its headings with `grep -n '^#'`, then read in full, with Read's `offset` and `limit`, each section slice N's _Prompt_ and _Done when_ name, each one the slice's diff changes, and §7, its honest limits.

Then read:

- `tracker/CLAUDE.md`, for conventions and the test command.
- Every earlier `tracker/docs/slice-*-readme.md`, for the shared opening, the house style, and what they promise this slice ("Slice N adds…", "Limits, and what later slices change").
- `tracker/docs/slice-N-readme.md`, if it exists. If it does, you are **updating** it (see step 3).
- Every `tracker/docs/reviews/slice-N-review-*.md`, in full.

## Step 2: Gather the evidence

Work in `tracker/`. Never run `tracker` without `TRACKER_HOME` set to a temporary folder. Without it, the tool writes to the real store at `~/.tracker`.

1. **Branch and commits.** The branch is `feat/tracker/slice-N`. If it isn't merged yet, the slice's commits are `git log --oneline --reverse $(git merge-base main feat/tracker/slice-N)..feat/tracker/slice-N`. If it's merged, find its merge commit (`git log --oneline --merges --grep 'feat/tracker/slice-N' main`) and use `<merge>^1..<merge>^2`. Run `git status --short -- .` too. Uncommitted changes count as work in progress: describe them, but they have no commit hash, and the Status row must say the work isn't committed.
2. **What changed.** Run `git diff --stat` over the same range, plus the working tree if there are uncommitted changes. Read every changed source file, test file and schema in `tracker/`, and enough of the unchanged ones to describe how the new code fits in.
3. **Tests.** Run the test command, `cargo fmt --check && cargo clippy -- -D warnings && cargo test`. Record whether each part passes. Count the tests per file with:

   ```sh
   cargo test 2>&1 | awk '/Running /{f=$2} /Doc-tests/{f="doc-tests"} /^running [0-9]+ test/{if ($2>0) print f, $2}'
   ```

   If anything fails, the Status row says so, and you quote the failing test names.
4. **_Done when_ coverage.** For each _Done when_ bullet in PLAN.md's slice N, and each concrete requirement in its _Prompt_, find the tests that check it by reading the test files. A bullet with no test is either "checked by hand", with the commands and what they showed, or uncovered. Say which.
5. **Reviews.** For each finding in each review, find the commit that fixed it, from the commit messages and diffs, and the test that guards it. A finding with no commit is still open. Don't count it as fixed because a message sounds related: check that the diff does what the finding asked.
6. **Try it.** Build with `cargo build`. Write a short shell demo that a reader can paste. It should cover each command or flag the slice adds, show at least one refusal for each new kind of error, end with `tracker list` and, if the item file changed, `cat` one item file. Base it on the earlier readmes' demos: `export DEMO=$(mktemp -d)`, `export TRACKER_HOME=$DEMO/store`, and `target/debug` on `PATH`. **Run it exactly as written**, with stderr captured too, and paste the real output with the temporary folder replaced by `$DEMO`. Mark each command that exits non-zero with a trailing comment such as `# exit 1: …`. If the output differs from what you expected, fix the demo or report the bug. Never edit the output by hand.

## Step 3: Write the doc

### Creating it

Use the sections below, in this order. **Required** sections always appear. Leave out an optional section when it would be empty. Never leave a heading with nothing under it.

1. `# Slice N: Title`, with PLAN.md's title.
2. **Opening paragraph** (required). Use the same first sentence as the earlier readmes, which describes `tracker` in one line. Then say that this document covers slice N of the slices in [PLAN.md](PLAN.md), counting them from PLAN.md, and what it covers: what it built, how it works, and how to test it. Link the earlier slices' readmes for what they cover.
3. **What you can do now** (required). One paragraph: "After slice N you can **…**." Then name the spec ("The spec is [DESIGN.md](DESIGN.md).") and the DESIGN.md sections this slice implements, as links.
4. **Facts table** (required): a two-column table with an empty header row, and rows for `Status`, `Branch`, `Test command` and `Tests`. Status says whether every _Done when_ check passes, which reviews' fixes are done (linked to their sections below), and what is still open. Tests gives the total, then the count per file, largest first.
5. `## Contents` (required): a link to every `##` heading that follows, and nothing else.
6. `## What's in this slice` (required): **Built:** as a bullet list, then **Not built yet:**, naming the slice that brings each missing piece. This is especially important where the reader might expect it to be built already.
7. `## Try it` (required): the demo from step 2 and its real output, plus any notes the output needs, such as a field that would differ when typed at a terminal.
8. `## Commands` (required if the slice adds or changes commands or flags): a table of each command and what it does or moves. Then the input rules and the exit codes as they apply in this slice, each linked to DESIGN.md.
9. **Optional topic sections**, such as slice 1's `## The store`: a `##` section for anything the slice adds that a reader will look up on its own, like a file format or a config file.
10. `## How it works` (required): one `###` subsection for each mechanism a later slice will touch or a reviewer must check. Describe the order of operations, the locking, and the edge cases, and give the file and function for each.
11. `## JSON output` (required if any `--json` output changed): a table of command, output shape and schema link, plus what later slices will add without breaking it.
12. `## Code tour` (required): a table of the files this slice added or changed, with the types and functions in each and links to the files. If the slice creates a pattern, add a line on how to extend it, such as how to add a command or a move.
13. `## Testing` (required): how to run the tests, any change to the harness ([tests/common/mod.rs](../tests/common/mod.rs)), the **_Done when_ coverage table** (_Done when_ bullet | tests), further tests for the edge cases, notes on the tests that matter most (what each proves, and whether it was seen failing), the fixtures added, and a `### Checks done by hand` subsection if any were done.
14. `## Decisions worth knowing` (required): bullets, each a bold one-line decision and then why. Include choices DESIGN.md left open, deviations from DESIGN.md or PLAN.md, and anything a later slice could get wrong without knowing it.
15. `## Fixes after review K` (one for each review in `docs/reviews/slice-N-review-K.md`): a sentence saying what the review found, linked to the review, and how the fixes were made (for example "each in its own commit, each test seen failing first"). Then the `###` subsections that have entries, in this order: `Fix before slice N+1`, `Low severity`, `Security items`, `Design decisions`, `Test gaps`, `Process`. Each is a table of Finding | Commit | Fix | Tests, or Question | Decision | Commits for design decisions, with findings numbered as the review numbers them. Add a "A few consequences" bullet list when a fix changes behaviour a reader would notice.
16. `## Still open` (if anything from the reviews or the _Done when_ checks is left): grouped under bold labels, such as **Security**, **Low severity**, **DESIGN.md**, **Test gaps** and **Process**. Each entry names the review and finding it came from. When both reviews raised the same thing, list it once.
17. `## Limits, and what later slices change` (required): bullets, each naming a later slice by number. Say what it will change in this slice's code or files, and what it must not break. Take these from PLAN.md's later prompts and _Done when_ checks, its integration risks, and DESIGN.md. This is the section a later slice's builder reads first.

### Updating it

When the doc exists, keep its structure, wording and history, and don't rewrite sections that are still true. Then:

- Refresh the Status and Tests rows, the Contents list, the Code tour and the _Done when_ coverage table from step 2's evidence.
- Add a `## Fixes after review K` section for each review that doesn't have one yet, before `## Still open`.
- Move items out of `## Still open` once a commit fixes them, and remove the section when it's empty.
- Re-run the Try it demo. Replace its output only if the output has changed.
- Fix any statement that the code no longer bears out.

### Links

The doc lives in `tracker/docs/`, so every link is relative to that folder.

- **DESIGN.md sections.** Link the **first** mention of each section to its heading: `[§3](DESIGN.md#3-interface)`, `[§4.1–4.3](DESIGN.md#41-layout)`. Later mentions of that section stay plain text (`§3`), so the prose stays readable. Anchors follow GitHub's rule: the heading's rendered text in lower case, with punctuation other than `-` and `_` removed and each space turned into `-`. Step 4's test checks every anchor you write.

- **PLAN.md, reviews and other readmes.** Use `[PLAN.md](PLAN.md)`, `[review 1](reviews/slice-N-review-1.md)` and `[slice 1's readme](slice-1-readme.md)`, with heading anchors where they help, such as `reviews/slice-N-review-1.md#nits`.
- **Source, tests and schemas.** Link the file, and name the function or type in backticks next to it: [src/states.rs](../src/states.rs) `find`. **Don't use `#L` line anchors into source files.** They break as soon as the code moves. Name the function instead.
- **Inside the doc.** Link other sections by their heading anchor, such as `[below](#fixes-after-review-1)`. When a heading appears more than once, as `### Low severity` does under each review, the second copy's anchor is `#low-severity-1`.
- **Don't link** to anything outside the repo, or to `target/`.

### Style

Match the earlier readmes:

- Plain words and short sentences, with contractions ("isn't", "doesn't"), and Canadian/British spelling ("colour", "behaviour", "organise").
- Present tense for what the code does. Name slices by number ("slice 6"), not by "later" or "soon".
- Inline code for commands, flags, fields, file names, function names and test names. Bold for the lead phrase of an important bullet.
- Tables for anything with more than one attribute per row: commands, fixes, coverage, files.
- Exact counts and real names everywhere. Never use placeholders such as "TBD" or "several tests".

## Step 4: Verify

1. Run `cargo test --test docs_links` in `tracker/`, fix every failure, and run it again until it passes. It checks every `.md` file in `docs/`, CLAUDE.md and judge-eval's README: that each relative path exists, that each heading anchor matches a heading in its target as GitHub slugs it (repeated headings included), and that each `#L` anchor names lines the file has (against a review's pinned commit). It skips links in code. A review's paths and heading anchors are checked as of the last commit that changed it, or as the files are now while it has uncommitted changes, so a later change to DESIGN.md never breaks an old review. Every other doc is checked as the files are now, so a later heading rename that breaks a link in a readme fails `cargo test`. If it rejects an anchor you believe is right, compare against GitHub's rendering of that heading before changing either the doc or the test.
2. Check by eye that `## Contents` lists every `##` heading, in order; that the per-file counts in the Tests row add up to its total; that every commit hash you cite appears in step 2's `git log`; and that no sentence refers to a conversation.

## Step 5: Report

Don't commit. Tell the user:

- The path of the doc, and whether you created or updated it.
- The Status row as written.
- Anything that didn't pass or couldn't be checked: failing tests, uncovered _Done when_ bullets, open review findings, and a title that didn't match PLAN.md.
- The commit message to use when they're ready. The earlier readmes used `docs: slice N readme`.

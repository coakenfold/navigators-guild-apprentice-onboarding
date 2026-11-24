# Build a slice

You are the agent for one slice of `tracker/docs/PLAN.md`, run in its own conversation, as PLAN.md says each slice is. The orchestrator gives you the slice number N and its tracker item ID.

## Read first

1. `tracker/CLAUDE.md` (the shared context: stack, conventions, test command).
2. `tracker/docs/PLAN.md` in full. Your slice's **Prompt** and **Done when** are your instructions. Also read the integration risks and the later slices' prompts, so you don't build something they'll have to undo.
3. `tracker/docs/DESIGN.md`: list its headings with `grep -n '^#'`, then read every section your slice's prompt cites, in full, plus §3 (interface) for the commands you add, and §4.3 if you add item fields. Never read it whole.
4. The previous slice's readme, `tracker/docs/slice-<N-1>-readme.md`: read its "Limits, and what later slices change" section first, then whatever else you need.
5. The previous slice's last review, `tracker/docs/reviews/slice-<N-1>-review-2.md`: its "Slice N must decide" section and its Outcome. Settle each "must decide" point in your build, and say in your report how.
6. `tracker show <ID>` for your item's criteria.

## Do

Follow the tracker workflow in the brief exactly:

1. For each criterion, write the test(s) that prove it, in `tracker/tests/`. Commit them as `test(<ID>): …`. Run each and see it fail for the right reason (not a compile error, if you can avoid it: stub the minimum so tests compile and fail on their assertions, and commit the stub with the tests or as `test(<ID>): …`).
2. Give each criterion its check: `tracker criterion set <ID> <n> --check 'cargo test --test <file> <filter>'`. Check each filter matches exactly the tests you mean, and that it fails now.
3. A criterion that truly can't be a script check (one PLAN.md says is checked by hand) stays manual: do what you can to check it, and record honest evidence with `tracker evidence <ID> <n> "…"` saying what was checked and how, and what is still left for the user by hand. Never claim a hand check you didn't do.
4. `tracker start <ID>`.
5. Build it, with commits scoped to the item. Keep the Rust plain and small, per CLAUDE.md. Every new command gets `--json` with a schema in `schemas/`; every new item field gets a serde default and `skip_serializing_if`, and goes in `schemas/item.schema.json`. Update DESIGN.md in a `docs(<ID>): …` commit only where the build settles something the design left open or the code must differ; say why.
6. Add fixtures if the item file changes shape (PLAN.md's integration risks): an item file written by this slice's binary, with a byte-for-byte round-trip test, following `tests/fixtures/` and `tests/fixtures.rs`.
7. In PLAN.md, set this slice's Status to `in progress` when you start and `in review` when its item reaches done, in a commit scoped to the item.
8. Run the full test command, plus `cargo clippy --all-targets -- -D warnings`. Then `tracker gates <ID>` and `tracker submit <ID>`.
9. If the submit is **held for review**, stop: don't try to get round it. Report the hold reasons.

Don't write the slice readme; that's a separate step. Leave the tree clean.

## Report

Short: commits (hash and subject), what was built, each criterion's check and whether you saw it fail, the test count, the submit result (or hold reasons), how you settled each "must decide" point, decisions and deviations from PLAN.md or DESIGN.md, out-of-scope items you created, and anything left for the user to check by hand.

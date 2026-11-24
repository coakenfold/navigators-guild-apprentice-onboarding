# Build a round of review fixes

The orchestrator names the slice, the review file, and the item IDs. Build those items in the order given, one at a time, each from `tracker show` to `tracker submit`, before you write the next one's tests. Follow the tracker workflow in the brief exactly. Stay on the current branch.

- Read the findings each item's description names, in the review file. Read the whole review only if you need to. The description says what to do, and the criteria say what must be true. Don't do findings that no item covers: they were deliberately left out.
- Keep each fix plain and small.
- **A new item field** gets a serde default and `skip_serializing_if`, so old files still round-trip byte for byte. Update `schemas/item.schema.json` and any other schema it appears in.
- **Docs-only criteria:** give each criterion a script check that proves the words are there, such as `grep -qF 'the exact new phrase' docs/DESIGN.md` (chain more than one with `&&` if needed). Pick words that aren't in the file yet, so the check fails at start. Don't leave a criterion manual: from slice 10, a manual criterion holds the item for review (T6), and a held item blocks every later item. Commit as `docs(<ID>): …`, and run `cargo test --test docs_links`.
- **Tests for code that already works** pass at start. To see each one fail, break the code temporarily (don't commit that), run the test, then restore the code. Say in your report what you broke and how each test failed.
- **Before each `tracker submit`,** run the full test command and `cargo clippy --all-targets -- -D warnings`.
- **If a submit is held for review,** stop and report the hold reasons.
- Don't touch the slice readme or the review files.

Final report, short, per item: its commits (hash and subject), what changed, what you saw fail and how, and the submit result. Then give the test count, and any decision, deviation, or out-of-scope item you created.

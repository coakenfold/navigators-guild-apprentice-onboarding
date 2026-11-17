# Slice 3 review 2

A second review of slice 3 (Organise and filter). It checks the branch against [PLAN.md](../PLAN.md)'s Slice 3 prompt and _Done when_ list, and against the [DESIGN.md](../DESIGN.md) sections the slice touches (§2.1, §2.2, §2.5's T9, §2.6, §3, §4.3, §4.4 and §6). It also checks the fixes claimed in [review 1](slice-3-review-1.md)'s outcome and in [slice-3-readme.md](../slice-3-readme.md).

|                 |                                                                                                                                                                  |
| --------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Date            | 2026-11-16                                                                                                                                                       |
| Branch          | `feat/tracker/slice-3` at `901bea6` (`docs: slice 3`), working tree clean. Review 1 covered the branch up to `9351296`. This review covers that plus the 15 commits after it, from `1296661` to `901bea6` |
| Method          | Read the code, schemas, tests and docs, ran the test command, and reproduced each finding below against the debug binary in a temporary store                    |
| Line references | Point at `901bea6`                                                                                                                                               |

## Verdict

Slice 3 does what PLAN.md asks, and the review 1 fixes it claims are real. `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and all 134 tests pass. The test counts in the readme are correct for every file.

I checked each fix against the code:

- **Temporary file:** it's now `<file>.<pid>.tmp`, opened with `create_new`, so a planted symlink is refused instead of followed ([store.rs:426-452](../../src/store.rs#L426-L452)).
- **Weakened triggers:** an edit records `kind_before` and `labels_removed` ([main.rs:749-818](../../src/main.rs#L749-L818)). The schema allows those two fields only on an edit entry.
- **Edits to held items:** `edit` refuses an item in review unless stdin is a terminal ([main.rs:742-747](../../src/main.rs#L742-L747)). §2.2, §3 and `--help` say so.
- **List filters:** labels are limited to 50 characters, `--status` values are trimmed, and repeated `--label` filters mean "every label named". Each has a test.
- **DESIGN.md:** the five decisions review 1 asked for are written into §3.
- **Readme:** `docs/slice-3-readme.md` now exists (`901bea6`) and carries the open items forward.

**What's still wrong:**

- **Review 1's one "fix before slice 4" is still open.** A label like `ℂ` breaks the item schema (finding 1). The readme says so plainly.
- **One new bug, of the same kind.** An edit can write a history entry that breaks the schema, and history entries are never rewritten, so the file stays invalid (finding 2).
- **Lookalike labels.** The fix review 1 suggested for invisible characters wouldn't stop a label that only looks like `needs-review` (finding 3).
- **Input that slice 6 will need to handle:** a `design` reference can point outside the project, and DESIGN.md doesn't say what the path is relative to (finding 5).
- The rest is low severity, documentation and test gaps.

## Done when

Every check passes, as in review 1's [Done when table](slice-3-review-1.md#done-when). One test name has changed since then: the done-item check now lives in `edit_works_in_every_state_but_done_and_review`, which also covers review. All other test names and links in that table still hold.

| Check | Result |
| --- | --- |
| `list` shows high before medium before low | Pass |
| An unknown priority or kind exits 1 and names the allowed values. `--label Bug --label bug` is stored once, as `bug` | Pass |
| `edit --priority low --unlabel bug` changes the item and adds one history entry. `edit` on a done item exits 1 | Pass |
| `list --status open --priority high --label bug` matches all three | Pass |
| `--status done` and `--status open,shelved` show exactly those states | Pass |
| Plain `list` hides shelved items | Pass |
| `list` inside XX's root shows `"project": "XX"`. `--all`, or `list` outside every root, shows `null` | Pass |
| _Prompt:_ `--by` checks the state as well as `verification` | Pass |
| _Integration risk:_ slice 3's files come back byte for byte | Pass (`XX-6`, `XX-7`, `XX-8`) |

## Fix before slice 4

1. **Still open from review 1: a label with no lowercase form breaks the item schema.** [`check_label`](../../src/main.rs#L626-L635) hasn't changed. Reproduced again at `901bea6`:

   ```
   $ tracker create a --label 'ℂ' --json        # exit 0
       "labels": [ "ℂ" ]
   ```

   Review 1's [fix](slice-3-review-1.md#fix-before-slice-4) still applies: after lowercasing, refuse any label that still holds an uppercase (`Lu`) or titlecase (`Lt`) character, and add `ℂ` to `labels_are_lowercased_and_stored_once`. The readme's Status row says the branch isn't merged yet. This is the cheapest point to fix it, before slice 4's `show` adds another output that prints labels.

2. **New: an edit can write a history entry that breaks the item schema, and the file stays invalid.** Labels are rebuilt at [main.rs:771-777](../../src/main.rs#L771-L777) and `labels_removed` is worked out at [main.rs:778-783](../../src/main.rs#L778-L783). Neither drops duplicates. So if a file holds `"labels": ["bug", "bug"]`, which review 1's finding 4 says a hand edit can produce, then `--unlabel bug` records both copies. Reproduced:

   ```
   $ tracker edit XX-2 --unlabel bug --json
   … "fields": ["labels"], "labels_removed": ["bug", "bug"] …
   ```

   [item.schema.json:138](../../schemas/item.schema.json#L138) gives `labels_removed` `uniqueItems: true`. `a_wrong_output_is_caught` already shows that `uniqueItems` is enforced, with its `["title", "title"]` case. Before this edit, the item broke the schema only through its `labels`, and a later edit could repair that. Now the bad value is in history, and no command ever rewrites a history entry. Every later `list --json`, `edit --json` and, from slice 4, `show --json` on this item breaks its schema.

   Only a hand-edited file triggers it, but the fix is small and belongs with finding 4. Drop duplicates when the labels are rebuilt, for example by starting from an empty list and calling `add_labels` with the kept labels, then the new ones. `labels_removed` then can't hold duplicates either. Add a test that writes `["bug", "bug"]` into a file, runs `--unlabel bug`, and checks the output against the schema with `Env::json`.

## Security and design

3. **The fix review 1 suggested for invisible characters wouldn't stop a lookalike `needs-review`.** Review 1's finding 5, still open, suggests refusing zero-width characters. That catches `needs​-review` with a hidden U+200B, but not these, all stored without complaint:

   ```
   $ tracker create h --label $'needs‐review' --label $'nеeds-review'
   $ tracker create n --label $'café' --label $'café'
   $ tracker list
     XX-3  medium  change  open  h  [needs‐review, nеeds-review]
     XX-4  high    change  open  n  [café, café]
   ```

   The first label uses U+2010 (a Unicode hyphen) and the second a Cyrillic `е`. Both look like `needs-review` in a terminal, and T9 fires on neither. The third and fourth are the same word, one with a precomposed `é` and one with `e` plus a combining accent. They're stored as two different labels that look identical, and `list --label café` matches only the form typed.

   The impact is still low: an agent gains nothing by leaving the label off, so the risk is only that a list looks as if review was asked for. But it's a whole class of problem, and refusing a few code points won't close it. Two consistent choices:
   - **Make labels ASCII.** Restrict them to something like `[a-z0-9][a-z0-9 _.-]*`. That's simple, it closes the class, and it removes the need for finding 1's check. The cost is dropping the `ünï` case from `labels_are_lowercased_and_stored_once` and saying so in §2.1.
   - **Accept the risk.** Record in §7 that labels are compared as exact strings, so a lookalike label isn't the label, and that the list shows labels as stored. Refusing zero-width characters is still worth doing for titles.

   Decide this together with finding 1, since the ASCII option makes finding 1's fix unnecessary.

4. **Now that edits record `labels_removed`, T9 could keep holding the item instead of relying on the digest.** Review 1's finding 2 was settled with option (b), widened: the digest lists every edit that removes `needs-review`. That catches the removal after the fact, and only if I read the digest, while the item has already auto-verified. With `labels_removed` stored, option (c) is now a single check in slice 9: T9 fires if the item has `needs-review`, **or** if any edit entry since `start` has `needs-review` in `labels_removed`. That keeps T9 a hold, as §2.5 describes it, at no extra cost, and the digest line stays as a second check. This isn't a defect in slice 3. Note it in slice 9's prompt, or record why (b) alone is enough.

   One related point for slice 16: if finding 4 is fixed by matching labels case-insensitively, `--unlabel needs-review` will remove a hand-written `Needs-Review`, and `labels_removed` will record it as `Needs-Review`. The digest's check must then compare lowercased forms.

5. **A `design` reference can point anywhere, and DESIGN.md doesn't say what it's relative to.** [`check_design`](../../src/main.rs#L675-L686) checks only that the reference fits on one line:

   ```
   $ tracker create d --design '../../../etc/passwd#1' --json    # exit 0
       "design": "../../../etc/passwd#1",
   ```

   Nothing reads `design` until slice 6's G6, which opens the file named and looks for the section. Two things need deciding before G6 is built:
   - **What the path is relative to.** §2.1's and §4.3's examples use `DESIGN.md#2.4`. TR's own design doc, though, is at `docs/DESIGN.md` under its root (§4.2, §8). Read from the root, the examples' form fails G6 for TR. Read as "a section of the project's design doc", it would be just `#2.4`. Pick one, and say it in §2.1 and in `create --help`.
   - **Whether G6 may leave the project root.** A single-user tool can tolerate it, and G6 only reports whether a file and heading exist. But §7 says commands run "only inside a registered project root", and the same rule should cover files G6 reads. Refusing absolute paths and `..` when the reference is taken in costs one check, in the same place slice 3 already validates it.

## Low severity

6. **`--unlabel` of a label the item doesn't have is silently ignored.** `edit XX-4 --priority high --unlabel needs-reveiw` prints "Edited XX-4: changed priority." and exits 0. A typo in the one label that matters to T9 is easy to miss. Name the labels that weren't there in the text output, such as "XX-4 had no label needs-reveiw", or refuse the edit, as `--label` together with `--unlabel` already is.

7. **Wide characters push `list`'s columns out of line.** [`table`](../../src/main.rs#L1163-L1195) measures cells with `chars().count()`. Most CJK characters and emoji take two terminal columns, so a row whose title is `中文标题很长` pushes its labels six columns past the others. Slice 5's _Done when_ now requires a heading row "lined up with the rows below it", so this will come up there. §8 has no display-width crate. Either add `unicode-width` in slice 5 and list it in §8, or record that alignment is by character count.

8. **The label column can't show a label that holds `, ` or `]`.** Labels print as `[a, b]` ([main.rs:944-948](../../src/main.rs#L944-L948)). The labels `a, b` and `c]` print as `[a, b, c]]`, which reads as three labels. Refusing `,` in labels is the smallest fix, and it would also allow a comma-separated `--label` later, as `--status` has. Finding 3's ASCII option would cover it too.

## Where the code and DESIGN.md disagree

9. **§3 says `list`'s text "ends by saying which" project it covered.** In the code, the scope line is followed by the hidden-states hint and the `Filters:` line ([main.rs:975-1006](../../src/main.rs#L975-L1006)). The code's order is the right one, so the doc should change: the text "says which, after the rows".

10. **Review 1's outcome table still says there's no `slice-3-readme.md`.** It was written in `eb9c922`, before `901bea6` added the readme. Either update that row to point at `901bea6`, or leave the table as it was written and rely on the readme's Process section, which already says this is done. Review 1 keeps its findings "as written", but its outcome table is meant to be current.

## Test gaps

1. **`kind_before` and `labels_removed` in the schema test.** `a_wrong_output_is_caught` ([tests/schemas.rs:210-246](../../tests/schemas.rs#L210-L246)) checks that a move can't carry `fields`. It doesn't check the matching rules for the two new fields: that a move can't carry `kind_before` or `labels_removed`, that an edit can, and that `labels_removed: ["bug", "bug"]` fails. The `else` branch in [item.schema.json:145-149](../../schemas/item.schema.json#L145-L149) is untested.
2. **Finding 2's regression test**, as described there.
3. **Carried from review 1:** `ℂ` among the refused labels (gap 1), two projects in the text list (gap 5), and hand-edited labels (gap 6).

## Still open, carried forward

These are already listed under the readme's [Still open](../slice-3-readme.md#still-open). They're repeated here only so this review is complete:

- **Review 1:**
  - finding 1, labels like `ℂ` (above)
  - finding 4, hand-edited labels aren't matched case-insensitively
  - finding 5, invisible characters (see finding 3 above)
- **Process:**
  - the present-tense `unchecked` note ([states.rs:37-38](../../src/states.rs#L37-L38)), cheapest to change before self-tracking starts after slice 6
  - PLAN.md marking a slice `done` before its review
- **Slice 2:**
  - a hand-edited timestamp is blamed on "a newer tracker"
  - `status <id> closed` is refused
  - commands on an item whose project isn't registered
  - `error.schema.json`'s `allowed[].via` enum
  - untested concurrent moves
  - untested `shelve` and `reopen` text output

## Suggested order

1. Findings 1 and 3 together. The ASCII decision decides how finding 1 is fixed.
2. Finding 2, together with review 1's finding 4. Dropping duplicates on rebuild fixes both, and needs one test.
3. Test gap 1.
4. Note findings 4 and 5 in slice 9's and slice 6's prompts, so they're decided when those slices start.
5. Findings 6–10 whenever convenient. Finding 7 belongs with slice 5's heading row.

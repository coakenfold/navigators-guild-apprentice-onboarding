# Slice 3 review 1

A review of slice 3 (Organise and filter) against [PLAN.md](../PLAN.md)'s Slice 3 prompt and _Done when_ list, the [DESIGN.md](../DESIGN.md) sections it touches (§2.1's `kind`, `priority`, `labels`, `design` and `history` fields, §2.5's T9, §2.6, §3's `create`, `edit` and `list` rows and its rules for picking a project, §4.3 and §4.4), and the conventions in [CLAUDE.md](../../CLAUDE.md).

|                 |                                                                                                                                                 |
| --------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| Date            | 2026-11-16                                                                                                                                      |
| Branch          | `feat/tracker/slice-3` at `9351296`, working tree clean. The slice is `1296661` (`feat: slice 3 first pass`). `8216c23` and `9351296` came after it and don't change the slice's code |
| Method          | Read the code, schemas and tests, ran the test command, and reproduced each finding below against the debug binary in a temporary store        |
| Line references | Point at `9351296`                                                                                                                              |

## Verdict

Slice 3 does what the plan asks. Every _Done when_ check passes and has a test. `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and all 126 tests pass, 22 of them new in [tests/organise.rs](../../tests/organise.rs).

- **`create`** takes `--kind`, `--priority`, repeatable `--label` and `--design`, with §2.1's defaults. clap names the allowed values when one is wrong, and exits 1.
- **`edit`** checks all its input before it takes the lock. It refuses done items and points to `reopen`. It writes one history entry with `via: "edit"`, `from` equal to `to`, and `fields` in the file's field order.
- **`list`** groups items by project and sorts them by priority, then by number. It marks priority, kind and status, and its filters combine. It reports the project it covered in both its text and its JSON.
- **`--by`** checks that the item is done, so it skips the `verification` record that slice 2 can leave on a shelved item.
- **Files:** new fields go in §4.3's order, each with a serde default and `skip_serializing_if`. Slice 3 fixtures were added, and the schemas cover `labels`, `design` and the edit entry, with an `if`/`then` that requires `fields` on an edit and forbids it on a move.

One real bug turned up. A label can be stored that the item schema forbids, so `create`, `edit` and `list` can print JSON that breaks the schema (finding 1). There is also one design gap that slice 3 opens: an agent can quietly take off the `needs-review` label I added, or downgrade an item's kind before `start`, and nothing reports it (finding 2). The rest is low severity, decisions that DESIGN.md doesn't record yet, test gaps, and process.

## Outcome

Added after the fixes. The findings below are left as written, so their line links point at `9351296`, not at the fixed code. Each fix is in its own commit, and each new test that guards a change was seen failing first. The concurrent-edit and terminal tests cover behaviour that already worked, so each was checked by breaking the code on purpose. After the fixes, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and all 134 tests pass.

| Finding | Resolved by |
| --- | --- |
| Fix before slice 4, item 1: labels with no lowercase form | Still open |
| Security 2: `needs-review` and `kind` can be quietly weakened | Option (b). `f2f07de` makes an edit's history entry record `kind_before` and `labels_removed`, so slice 16's digest can tell these edits apart. DESIGN.md §2.1, §2.6, §3 and §6, and PLAN.md's slice 16, now say the digest lists every edit that removes `needs-review` or changes kind away from `capability`. Unlike (b) as written, the digest lists every such edit, marked by whether it came from a terminal, not only an agent's edits: §2.6 lists my own actions because `script` can fake a terminal. Adds fixture `slice-3/XX-8.json` |
| Security 3: the symlinked `.tmp` file | `4a40b00`: the temporary file is named with the process ID and opened with `create_new`, so a planted symlink is refused rather than followed |
| Low severity 4: hand-edited labels aren't matched case-insensitively | Still open |
| Low severity 5: invisible characters in labels and titles | Still open |
| Low severity 6: no label length limit | `47d4671`: `create` and `edit --label` refuse a label over 50 characters. `--unlabel` has no limit, so a long label written by hand can still be removed. Whether titles need a limit is now in slice 5's prompt (`5634992`) |
| Low severity 7: strict list filters | `9daacb3` trims each `--status` value. `8f85fa4` lets `list --label` repeat, meaning items with every label named, and says so in §3 |
| Low severity 8: no column headings in `list` | `5634992` adds them to slice 5's prompt and _Done when_ |
| Code and DESIGN.md, items 1–4 | Recorded in §3: `e7e3071`, `40b2979`, `b238b17` and `d30b2ba` |
| Code and DESIGN.md, item 5: `edit` on an item in review | `6fb2e70`: refused (exit 1) unless it comes from an interactive terminal, so an agent can't change a held item while I decide it. §2.2, §3 and `--help` say so |
| Test gap 2: `edit` on an item in review | Done with `6fb2e70`, which uses `env_with_every_state` and checks that a piped edit is refused and one from a terminal works |
| Test gap 3: concurrent edits | `4321762`: ten `edit --label` commands at once all land. Slice 2's concurrent moves are still untested |
| Test gap 4: an edit from a terminal records `interactive: true` | `e8b3ac4`. `213c9e0` moved the `script` runner into `tests/common/mod.rs` first, so both tests share it |
| Test gaps 1, 5 and 6 | Not taken |
| Process | Still open: no `slice-3-readme.md` yet, the `unchecked` note's tense, and PLAN.md's Status column |

## Done when

| Check | Result | Tests |
| --- | --- | --- |
| `list` shows high before medium before low | Pass | [organise.rs:385](../../tests/organise.rs#L385), which also checks grouping by project and that an edit re-sorts |
| An unknown priority or kind exits 1 and names the allowed values | Pass | [organise.rs:144](../../tests/organise.rs#L144), for `create`, `edit` and `list` |
| `--label Bug --label bug` is stored once, as `bug` | Pass | [organise.rs:122](../../tests/organise.rs#L122), which also covers `" BUG "`, non-ASCII, and empty or multi-line labels |
| `edit --priority low --unlabel bug` changes the item and adds one history entry | Pass | [organise.rs:177](../../tests/organise.rs#L177) |
| `edit` on a done item exits 1 | Pass | [organise.rs:335](../../tests/organise.rs#L335), which also checks open, in-progress, shelved and reopened items |
| `list --status open --priority high --label bug` shows only items that match all three | Pass | [organise.rs:435](../../tests/organise.rs#L435) |
| `--status done` shows only done items, and `--status open,shelved` exactly those two states | Pass | [organise.rs:527](../../tests/organise.rs#L527) |
| Plain `list` hides shelved items | Pass | [organise.rs:517](../../tests/organise.rs#L517) (done items too) |
| Inside XX's root, `list` shows only XX's items with `"project": "XX"`. `--all` there, or plain `list` outside every root, shows every project with `"project": null` | Pass | [organise.rs:593](../../tests/organise.rs#L593), [organise.rs:634](../../tests/organise.rs#L634) |
| _Prompt:_ `--by` checks the item is done as well as its `verification` | Pass | [organise.rs:557](../../tests/organise.rs#L557), [organise.rs:578](../../tests/organise.rs#L578) (slice 2's shelved fixture with a record) |
| _Prompt:_ `edit` covers priority, labels, title, kind, design and description, and `fields` names what changed | Pass | [organise.rs:216](../../tests/organise.rs#L216), [organise.rs:268](../../tests/organise.rs#L268) |
| _Integration risk:_ files saved by slice 3 load and come back byte-for-byte unchanged | Pass | [tests/fixtures/slice-3/](../../tests/fixtures/slice-3/), checked by [fixtures.rs](../../tests/fixtures.rs) |

Also checked by hand in a temporary store:

- Ten `edit YY-1 --label lN` commands run at once: the item ends with all ten labels and ten history entries, and no `.tmp` file is left behind. `edit` reads and writes under the lock ([main.rs:695-696](../../src/main.rs#L695-L696)).
- `edit` on an item a newer tracker wrote, with a history `via` this version doesn't know, exits 3 and writes nothing. This one is also automated in [item_files.rs](../../tests/item_files.rs).

## Fix before slice 4

1. **A label can be stored that the schema forbids, so the JSON output breaks its own schema.** [`check_label`](../../src/main.rs#L612-L621) stores `label.trim().to_lowercase()`. [item.schema.json](../../schemas/item.schema.json#L36) forbids any character in Unicode's uppercase (`Lu`) or titlecase (`Lt`) categories. Some `Lu` characters have no lowercase form, so `to_lowercase` leaves them unchanged. Examples are `ℂ` (U+2102), `ϒ` (U+03D2) and the mathematical letters such as `𝐀`. Reproduced:

   ```
   $ tracker create a --project XX --label 'ℂ' --json     # exit 0
       "labels": [ "ℂ" ]
   ```

   Run through the test harness's `Env::json`, the same command fails with "This doesn't match schemas/create.schema.json". From then on, every `list --json` that includes the item breaks the schema too, and so does every `edit --json` on it. §3 says the schemas define the interface, so the code and the schema must agree. I'd change the code rather than the schema: after lowercasing, refuse any label that still contains an `Lu` or `Lt` character, with a message such as "A label can't contain the capital letter ℂ, which has no lower-case form." Add `ℂ` to the refused cases in `labels_are_lowercased_and_stored_once`. The other option, loosening the schema to "unchanged by lowercasing", can't be written as a JSON Schema pattern.

## Security

2. **An agent can quietly undo `needs-review`, or downgrade an item's kind before `start`, and nothing reports it.** This is a design gap, not a bug in the slice's code. §2.5 calls T9 "a manual override, for me or the agent", and §5 tells agents to use `tracker edit` for labels. But `edit` accepts `--unlabel needs-review` from an agent's shell like any other edit. Reproduced: `edit XX-4 --unlabel needs-review` exits 0 and records `{"via": "edit", "interactive": false, "fields": ["labels"]}`. When the item is submitted, T9 doesn't fire. The only trace is that history entry, and nothing reads it:
   - §2.6's digest lists auto-verified items, shelves and human actions, not edits.
   - `audit` (§3) checks status changes, not labels.

   The same applies to `--kind capability` changed to `change` before `start`. The T5 fingerprint (§2.1) is taken at `start`, so an earlier change isn't caught. §6's row for "mislabels a capability as a `change`" relies on T1 seeing a new file, and a capability built inside existing files gets past it.

   This should be decided while `edit` is new, before agents track real work with it at slice 6's checkpoint. Options, simplest first:
   - **(a)** Removing `needs-review`, or changing `kind` away from `capability`, needs an interactive terminal, like `criterion accept`. The agent can still add the label, which is the direction T9 needs.
   - **(b)** The digest lists every non-interactive edit that removes `needs-review` or lowers `kind`, the same way it lists shelves.
   - **(c)** T9 fires if `needs-review` was on the item at any time since `start` (or since it was created). That reads history, so it needs no new field.

   (a) fits the existing pattern best and costs one check in [`edit`](../../src/main.rs#L656-L772). Whichever you choose, add a row to §6.

3. **The symlinked `.tmp` file is still open, and `edit` is one more command that writes through it.** See [slice-2-readme.md](../slice-2-readme.md#still-open). [`write_file`](../../src/store.rs#L421-L448) hasn't changed. A planted `XX-1.json.tmp` symlink now redirects an `edit` as well as a move. Nothing else has changed since slice 2's reviews. The fix is still `OpenOptions::new().write(true).create_new(true)` on a name that includes the process ID.

## Low severity

4. **Labels edited by hand aren't matched case-insensitively.** §4.4 says free text read from a file isn't refused, but labels are matched, not just shown. If a file holds `"labels": ["Bug"]`:
   - `list --label bug` doesn't find it.
   - `edit --unlabel bug` can't remove it, and reports "already has those details. Nothing changed."
   - Plain `list --json` prints JSON that breaks the schema (the same constraint as finding 1).
   - A hand-written `["bug", "bug"]` keeps both copies through `edit`, because [the label rebuild](../../src/main.rs#L725-L731) filters but doesn't de-duplicate.

   Comparing lowercased forms in the `list` filter and in `--unlabel`, and de-duplicating when rebuilding, would make all of these work without rewriting the file. Slice 16's `audit` could report labels that aren't in their stored form.
5. **Invisible characters are accepted in labels and titles.** [`changes_display`](../../src/text.rs#L14-L25) catches controls, line separators and bidi marks, but not zero-width characters: U+200B–U+200D, U+2060 and U+FEFF. `--label $'needs​-review'` is stored, and `list` shows it as `[needs​-review]`, which looks exactly like `needs-review`, but T9 won't fire on it. An agent gains little from this, since it could just leave the label off. But it can make a list look as though review was asked for when it wasn't. Refuse these characters in `check_printable`, or escape them in `for_terminal`.
6. **Labels have no length limit.** A reason is limited to 500 characters because it's printed among other lines ([main.rs:1001-1003](../../src/main.rs#L1001-L1003)). A label is printed in every `list` row, and widens the table for the whole list. A small limit, such as 50, matches the reason's rule. Titles have no limit either. That's a separate call for slice 5's polish.
7. **The list filters parse input more strictly than they need to:**
   - `--status 'open, shelved'` is refused because of the space (`invalid value ' shelved'`). clap's tip points at the fix, but trimming each value would be kinder.
   - `list --label a --label b` fails with clap's "cannot be used multiple times", while `create` and `edit` repeat `--label`. DESIGN.md doesn't say whether two label filters mean "both" or "either". Decide that, or keep it single and say so in `--help`.
8. **`list` text has no column headings.** A row reads `XX-1  medium  change  open  title  [bug]`. The values can't be confused with each other today, but a heading row like `project list`'s would make the output self-explaining. That's slice 5 polish, noted here so it isn't lost.

## Where the code and DESIGN.md disagree, or DESIGN.md is unclear

None of these is wrong in the code. Each is a decision that DESIGN.md §3 doesn't record yet, and slice 3 has no readme to record it in (see Process).

1. **An edit that changes nothing writes nothing.** §3 says `edit` "changes an item's details and adds a history entry". The code exits 0, adds no entry and doesn't touch the file ([main.rs:750-752](../../src/main.rs#L750-L752)), and `--json` returns `"fields": []`. That's the better behaviour, so the doc should change: "adds a history entry when something changes".
2. **`--by` without `--status` lists done items, not active ones.** §3 says that without `--status`, `list` shows open, in-progress and review items. [main.rs:843-844](../../src/main.rs#L843-L844) makes `--by` imply `--status done`, which is the only useful reading. Say so in §3.
3. **`edit --design ""` and `edit --description ""` remove the field.** This isn't in §3's `edit` row. Add it, since it's how the field is cleared.
4. **`list --project` accepts a key that isn't in `config.json` if items still use it.** That keeps `list` consistent with `project list`, which counts such items. §3 doesn't mention it. It ties into slice 2's open item 7 (which commands work on an item whose project isn't registered).
5. **`edit` is allowed on an item in review.** §3 refuses only done items, and `status` refuses review items. Editing a held item is mostly harmless: `approve` re-runs the gates, and T5 will catch kind and design changes after `start`. But §2.2 calls a held item "mine to decide". Decide whether an agent should be able to retitle or relabel it while I'm deciding, and write that down either way.

## Test gaps

1. **Labels with no lowercase form** (finding 1). Add `ℂ` to the refused cases.
2. **`edit` on an item in review.** `env_with_every_state` builds one (XX-3), but `edit_works_in_every_state_but_done` ([organise.rs:335](../../tests/organise.rs#L335)) builds its own store and covers only open, in-progress, shelved and done. Add review, which §3 allows.
3. **Concurrent edits.** These were correct when checked by hand (above) but aren't automated. The same applies to slice 2's concurrent moves, which are still open. A ten-way `edit --label` test would guard the lock-then-read order in `edit`.
4. **An edit from a terminal records `interactive: true`.** The `script` test covers moves only. If finding 2 goes the way of option (a) or (b), the edit path's `interactive` flag starts to matter, and it should be tested the same way.
5. **Two projects in the text list.** `the_text_list_marks_priority_kind_and_labels` ([organise.rs:411](../../tests/organise.rs#L411)) has one project. No test checks a heading line before each project, or that the columns line up across projects, which is the reason for building [one table](../../src/main.rs#L884-L916).
6. **Hand-edited labels** (finding 4), once that's decided.

## Process

- **There's no `docs/slice-3-readme.md`.** CLAUDE.md says each slice's readme lives in `docs/`, and slice 2's readme is where its decisions and its [Still open](../slice-2-readme.md#still-open) list live. The `slice-readme` skill added in `9351296` is meant for this. The readme should carry these forward:
  - The decisions in "Where the code and DESIGN.md disagree" above.
  - Slice 2's open items, which slice 3 didn't touch: the symlinked `.tmp` file (finding 3), the hand-edited timestamp message, `status <id> closed`, moves on unregistered projects, and the present-tense `unchecked` note.
- **The `unchecked` note is still in the present tense** ([states.rs:37-38](../../src/states.rs#L37-L38)). Slice 2 review 2 said it was cheapest to change before slice 3. That's still true until self-tracking starts after slice 6, because only the test fixtures hold the old text.
- **PLAN.md marks slice 3 `done` in the commit named "first pass"**, before any review, just as `54d8f66` did for slice 2. So the Status column means "built", not "built and reviewed". That's fine, but PLAN.md's intro should say so, or the status should change only after the review fixes. From slice 7 on, PLAN.md ties `done` to the item reaching done, which happens before a review too.
- **The slice is one commit again** (`1296661`, +1518 lines across 16 files). Slice 2's readme records this as history. From slice 7 on, the work goes through `tracker start` and `submit`, and smaller commits will make T7's diff-size hold and `show --diff` easier to read.

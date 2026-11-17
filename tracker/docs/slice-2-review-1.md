# Slice 2 review 1

A review of slice 2 (States) against [PLAN.md](../PLAN.md)'s Slice 2 prompt and _Done when_ list, the [DESIGN.md](../DESIGN.md) sections it cites (§2.2, plus §2.1, §2.7, §2.8, §3 and §4.3 where they touch states, history and verification), and the conventions in [CLAUDE.md](../CLAUDE.md).

|                 |                                                                                                                                     |
| --------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| Date            | 2026-11-15                                                                                                                          |
| Branch          | `feat/tracker/slice-2` at `54d8f66` (`feat: slice 2`), working tree clean                                                           |
| Method          | Read the code and tests, ran the test command, and reproduced each finding below against the debug binary in a temporary store |
| Line references | Point at `54d8f66`                                                                                                                  |

## Verdict

Slice 2 does what the plan asks. Every _Done when_ check passes and has a test. `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and all 87 tests pass (19 of them new in [tests/states.rs](../tests/states.rs)). The move table in [states.rs](../src/states.rs#L20-L27) matches every edge of §2.2's diagram that has a command so far. Refusals list the allowed moves as runnable commands. Each move reads and writes the item under the store lock and adds exactly one history entry with `from`, `to`, `via` and `interactive`. `submit` writes the `unchecked` record the plan describes. The slice-2 fixtures and the schema tests cover the new fields.

No bugs in the slice's own behaviour turned up. The findings below are about the shape of the JSON output, which later slices will have to change, the `interactive` flag, which slice 16's digest depends on and which isn't tested yet, one security item from slice 1 that now affects every move, and a few decisions that DESIGN.md doesn't record.

## Outcome

Added after the fixes. The findings below are left as written, so their line links point at `54d8f66`, not at the fixed code. [slice-2-readme.md](slice-2-readme.md#fixes-after-review-1) describes each fix and its test.

| Finding | Resolved by |
| --- | --- |
| Fix before slice 3, items 1–3 | One commit each: `d8a9564`, `32ef9f7` and `a370118`, each with its tests. For item 1, `item` stays next to §3's fields, and §3's example now shows it |
| Security, item 4: symlinked `.tmp` file | Still open |
| Security, item 5: faking `interactive` | `ee14706`: DESIGN.md §2.6 has the digest list every shelve and human action, mine included, each marked by whether it came from a terminal, and a delete leaves a line in `deleted.json`. §6 and §7 record what's left. The `audit` suggestion wasn't taken, since the digest covers it |
| Low severity: reasons have no length limit | `6855ce9` |
| Low severity: `--reason` shows as optional in `--help` | `529fa36` |
| Low severity: one outcome per move, the timestamp message, an unknown `via` stopping `list`, `status <id> closed` | Still open |
| Code and DESIGN.md, item 1 | `78c7d51`: reopening moves the record to `previous_verification`, so `verification` exists only while the item is done. `8ad15c1` adds a fixture of that shape |
| Code and DESIGN.md, item 2 | `f817e58` |
| Code and DESIGN.md, item 3 | `602cba8`: an `edit` or `criterion accept` entry has the move's shape, with `from` equal to `to` and a `fields` list |
| Code and DESIGN.md, item 4 | `a568739`: a reminder in slice 9's _Done when_ |
| Code and DESIGN.md, item 5 | `e405dad`. The tests don't start two items at once, since XX-1 is submitted first. G1's need for criteria is what will break them, and slice 6's prompt now says so |
| Test gap 1 | Done with finding 3 |
| Test gaps 2–5 | Still open |
| Process | [slice-2-readme.md](slice-2-readme.md) now exists. The slice itself stays one commit, as history |

## Done when

| Check                                                                                                   | Result | Tests                                                                                                                                                                                                          |
| ------------------------------------------------------------------------------------------------------- | ------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| open → in-progress → done → open, through the commands and through `status`                             | Pass   | [states.rs:92](../tests/states.rs#L92), [states.rs:112](../tests/states.rs#L112)                                                                                                                                 |
| `status <id> open` without `--reason` exits 1                                                           | Pass   | [states.rs:128](../tests/states.rs#L128), which also covers an empty and a blank reason, and `status … shelved`                                                                                                 |
| `shelve` works from open and from in-progress, and exits 1 without a reason                             | Pass   | [states.rs:156](../tests/states.rs#L156), [states.rs:170](../tests/states.rs#L170)                                                                                                                              |
| `reopen` brings a shelved item back to open                                                             | Pass   | [states.rs:192](../tests/states.rs#L192)                                                                                                                                                                         |
| open → done exits 1 and lists the allowed moves                                                         | Pass   | [states.rs:211](../tests/states.rs#L211), and every other undrawn move at [states.rs:229](../tests/states.rs#L229)                                                                                              |
| Every move adds exactly one history entry                                                               | Pass   | [states.rs:307](../tests/states.rs#L307) (eight moves, earlier entries unchanged), [states.rs:335](../tests/states.rs#L335) (a refused move adds none)                                                          |
| _Prompt:_ `submit` writes `by: "unchecked"`, the time and a note that no gates ran                      | Pass   | `check_unchecked` at [states.rs:60](../tests/states.rs#L60)                                                                                                                                                      |
| _Prompt:_ each entry has `interactive`                                                                  | Partly | Only `false` is ever tested. See finding 3                                                                                                                                                                       |
| _Integration risk:_ files saved by slice 2 load and come back byte-for-byte unchanged                   | Pass   | [tests/fixtures/slice-2/](../tests/fixtures/slice-2/), checked by [fixtures.rs](../tests/fixtures.rs)                                                                                                            |

Checked by hand as well, in a temporary store:

- Ten `start XX-3` commands run at once: one exits 0, nine exit 1, and the file has one history entry.
- Five `submit` and five `shelve` commands run at once on one in-progress item: exactly one move wins and the history is `["start","submit"]`.
- `script -q /dev/null tracker start XX-1` records `"interactive": true`. An ordinary call from an agent's shell records `false`.
- A file with a field that a newer tracker would write (`started`) is refused with exit 3, and nothing is written.

## Fix before slice 3

1. **`submit --json` returns `{"item": …}`, but §3 defines a different shape for it.** §3 says the `submit` and `gates` output "lists every gate and trigger with its id and result", and shows `{"id", "result", "status", "gates", "holds"}`, with `result` set to `"held"` or `"refused"`. Slice 2 gives `submit`, and `status <id> done` which maps onto it, the shared [move.schema.json](../schemas/move.schema.json), which has `additionalProperties: false` and only `item`. Slice 6 or 9 will then have to add `id`, `result`, `status` and `gates` to a command that agents are already parsing. Adding fields doesn't break anything, but the obvious way to reach §3's shape, dropping or moving `item`, would. §3 says that removing or renaming a JSON field is a breaking change. Give `submit` its own schema now, with `{"id", "result": "done", "status": "done", "gates": [], "holds": [], "item": {…}}`, so later slices only fill those fields in. Also decide whether `item` stays next to them. CLAUDE.md's rule that a command returning an item wraps it as `{"item": …}` and §3's example of a held submit currently disagree on that.

2. **The allowed moves are in the JSON only as prose.** A refused move prints `{"error": "XX-1 is in-progress, and `reopen` only moves … the allowed moves are `tracker submit XX-1` (to done) and …", "exit_code": 1}`. An agent has to parse that sentence to learn what it can do next. §3 says the text "can change wording freely" and the JSON fields are the interface, and the tests check the wording (`contains("the only move is")`, `contains("approve or reject")`), so rewording a message breaks tests that are meant to read JSON (CLAUDE.md). Add a field such as `"allowed": [{"via": "start", "to": "in-progress", "command": "tracker start XX-1"}]` to move refusals, either through a `move-error` schema or by letting [error.schema.json](../schemas/error.schema.json) take optional fields, as it already plans for gate refusals. Then have the tests assert on the field.

3. **`interactive` is only ever tested as `false`, and how it's worked out isn't shared.** The plan says slice 16's digest relies on this flag to tell my shelves from an agent's (§2.6, §2.8). [main.rs:665](../src/main.rs#L665) works it out inline as `stdin().is_terminal()`, and no test sees `true`. Slice 4 builds "one shared helper for interactive-only commands", and if that helper ends up with a different rule (for example also requiring stderr to be a terminal), the history flag and the human-only gate would disagree about what counts as me. Do two things now:
   - Move the check into one function, such as `text::is_interactive()` or a small `tty` module, that both `apply` and slice 4's helper call.
   - Add a test that runs a move under `script -q /dev/null …` (macOS and BSD) or `script -qc "…" /dev/null` (util-linux), and expects `"interactive": true`. That uses no pty crate, so it stays within the plan's "no pty test crate" rule. Skip the test where `script` isn't installed.

## Security

4. **A planted `<id>.json.tmp` symlink makes every move overwrite the file it points to.** This is review 2's "symlinked temporary file" item, still open. Slice 2 makes it matter more: every `start`, `submit`, `shelve` and `reopen` now writes an item file, where slice 1 only wrote items on `create`. Reproduced:

   ```
   $ echo precious > victim
   $ ln -s "$PWD/victim" $TRACKER_HOME/items/XX-4.json.tmp
   $ tracker start XX-4          # exit 0
   $ head -2 victim
   {
     "id": "XX-4",
   ```

   [write_file](../src/store.rs#L421-L448) opens the fixed name with `File::create`, which follows the link and truncates its target. Then `rename` moves the link itself into place, so `XX-4.json` becomes a symlink to `victim`. It needs write access to the 0700 store, and anyone with that access can already make `tracker` run commands (§7), so the extra exposure is small. The fix is still cheap: remove any existing `.tmp` first and open the new one with `OpenOptions::new().write(true).create_new(true)`, or use a random temporary name. Slice 5 asks for "every write goes to a temporary file that is then renamed", and that's a good moment for it at the latest.

5. **An agent can make its moves look like mine.** `script -q /dev/null tracker shelve XX-1 --reason …` records `interactive: true`, so the shelve won't appear in the digest as an agent's. §7 already says the interactive-terminal rule "is a convention, not security", but it talks only about human actions (approve, delete…). The same gap applies to the digest's list of shelved items. Add a line to §7 or §2.6, and consider having `audit` (slice 16) flag `interactive: true` entries made while an item was in progress, since that's when an agent is usually at work.

## Low severity

- **The move table assumes one outcome per command and state.** [find](../src/states.rs#L36-L40) returns the first move matching `(via, from)`. In slice 9, `submit` from in-progress leads to either done or review, depending on the triggers, so adding `step(Submit, InProgress, Review)` to `MOVES` would never be found. That doesn't break anything yet, but slice 9 will need the table to name the possible outcomes of a move, with the command picking one.
- **Reasons have no length limit.** A 20,000-character `--reason` is accepted, and the item file grows to 20 KB. It's printed on one line in later slices (`show`, `digest`, `review`). Cap it, for example at 500 characters, with a message saying to put details in the description.
- **A hand-edited timestamp is blamed on "a newer tracker".** With `created` changed by hand to `2026-11-16T02:00:00+00:00`, a valid RFC 3339 time, every move refuses with exit 3: "has fields this version of tracker would lose or change by rewriting it: created. It was probably written by a newer tracker". Refusing is right, since a rewrite would change the value. The message is wrong in this case. Say "would change by rewriting it" without guessing why, or name the value that would change.
- **One item file with a `via` this version doesn't know makes `list` exit 3 for the whole store.** For example `"via": "approve"`, which slice 9 adds. Skipping files that can't be parsed is slice 5's job, so this is in scope there. It gets more likely from slice 3, which adds `edit` entries, so it's worth testing explicitly in slice 5 with a file written by a later slice.
- **`--reason` shows as optional in `--help`.** The usage line is `tracker shelve [OPTIONS] <ID>`, and the flag says "Required" only in its description. Leaving it as an `Option` is reasonable, because it gives the clearer "needs a reason" message, but the usage line could show it, for example with `value_name = "REASON (required)"` or a custom `override_usage`.
- **`status <id> closed` is refused.** §1 says "the course's 'closed' state is `done`" and that `status` "still works as an alias". Someone following the course might type `closed`. Clap's error lists the valid states, which is enough, but accepting `closed` as another name for `done`, or adding a note to the message, would be friendlier. This is a design question as much as a code one.

## Where the code and DESIGN.md disagree, or DESIGN.md is silent

1. **The verification record is kept through reopen and shelve.** The code keeps it as "the record of its last verification" ([item.rs:20-24](../src/item.rs#L20-L24)), so fixture XX-4 is shelved with a `verification` from an earlier done. That's a sensible choice, and it fits §2.7 ("if the item is reopened and verified again, the ref moves"). But §2.1 only says the record is "filled in when the item reaches done", and §4.3 doesn't say what happens to it on reopen. Slice 3's `list --by` ("keeps done items verified that way") and slice 16's digest and `audit` will need to read `status` together with `verification`, not `verification` alone. Record the rule in §2.1 or §4.3.
2. **`verification.note` and `history[].reason` aren't in DESIGN.md.** [item.schema.json](../schemas/item.schema.json) defines both, and §4.3 says the schema defines the fields, so this isn't strictly a conflict. But §4.3's example is what a reader sees, and it has neither. Add a `reason` to one history entry in the example and mention `note` for `unchecked` records.
3. **History entries can't describe an `edit` yet.** §2.1 says history has "one entry per state change or `edit`". The schema's `move` requires `from` and `to`, and `Via` has no `edit`. Slice 3 will have to choose between an `edit` entry with `from == to` and a separate entry shape. That choice changes the schema and the slice-16 digest, so decide it at the start of slice 3 rather than partway through.
4. **The message for an item in review says approve and reject don't exist.** "A held item waits for a human to approve or reject it, and this version of tracker can't do either yet" ([states.rs:69-74](../src/states.rs#L69-L74)) is accurate for slice 2. §3 asks for "a pointer to `approve` and `reject`", which slice 9 adds. Nothing reaches review without a hand edit yet, so this is fine for now. It's listed so slice 9 doesn't forget to change it.
5. **Two items in one project can be in progress at once.** §4.4 and G1 forbid it, but G1 arrives in slice 6, so this is expected. The tests start XX-1 and XX-2 together ([states.rs:378-385](../tests/states.rs#L378-L385)), and slice 6 will have to rewrite those tests.

## Test gaps

1. `interactive: true` is never tested (finding 3).
2. Concurrent moves on one item aren't tested. They behaved correctly by hand (see above), but nothing guards the lock-then-read order in [move_item](../src/main.rs#L570-L580) against a regression. Slice 1 has a ten-way `create` test, and a matching ten-way `start` test, which expects one success and one history entry, would be cheap.
3. The "would lose fields" refusal is tested only at the store level ([item_files.rs:98](../tests/item_files.rs#L98)). No test runs a move command against a file written by a newer tracker and checks for exit 3 and an unchanged file. That's the path an older installed binary would actually take during self-tracking.
4. No test checks that a move leaves `created` unchanged, or the text output of `shelve` and `reopen`. Only `start` and `submit` are checked at [states.rs:404](../tests/states.rs#L404).
5. A move on an item whose project has been removed from `config.json` succeeds. That matches §3 ("a command that takes an item ID needs no project"), but it isn't tested, and slice 7 will need that project's root. Decide now whether that becomes an error.

## Process

- Slice 1 has a readme ([slice-1-readme.md](slice-1-readme.md)) that records what was built, what was deferred and the fixes after each review. Slice 2 has none, so the decisions above (keeping the verification record, recording the mapped command rather than `status` in `via`, `reason` as an `Option`) are recorded only in code comments. A short `slice-2-readme.md` would keep the docs folder consistent, and the outcome of this review can go there.
- Slice 2 landed as one 1,000-line commit, `feat: slice 2`. That's allowed before slice 7, when commits start being scoped to items. Splitting states, schema, fixtures and tests into separate commits would make the slice easier to review and to bisect, as slice 1's fix commits were.

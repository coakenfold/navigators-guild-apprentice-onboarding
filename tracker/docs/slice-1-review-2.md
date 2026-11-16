# Slice 1 review 2

A second review of slice 1 ([slice-1-readme.md](slice-1-readme.md)), after the fixes from [review 1](slice-1-review-1.md). It checks the code against [PLAN.md](../PLAN.md)'s Slice 1 prompt and _Done when_ list, the [DESIGN.md](../DESIGN.md) sections the slice cites (§3, §4.1–4.4), and the conventions in [CLAUDE.md](../CLAUDE.md).

|                 |                                                                                                                            |
| --------------- | -------------------------------------------------------------------------------------------------------------------------- |
| Date            | 2026-11-15                                                                                                                 |
| Branch          | `feat/tracker/slice-1` at `7816399`, with edits to `slice-1-readme.md` and `slice-1-review-1.md` not yet committed         |
| Method          | Read the code, ran the test command, and reproduced each finding below against the built debug binary in a temporary store |
| Line references | Point at `7816399`                                                                                                         |

## Verdict

Slice 1 is implemented faithfully. Every _Done when_ check passes. `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and all 46 tests are clean. Each fix that review 1 lists is in the code with its regression test. The `git mv` kept history, and nothing in the repo still points at the old `chapters/02-the-methodology/02-tracking-your-work/` folder. Apart from the deferrals the readme lists (list scoping, priority sort, skipping corrupt files, `digests.json` and `judge/`), the code matches DESIGN.md.

What's left is mostly hardening around store files that are edited by hand, or by an agent's shell command (§7). It's small now, but slice 2 starts rewriting item files and slice 4 starts printing more agent-written text, so the first five findings should be fixed before slice 2.

## Outcome

Added after the fixes. The findings below are left as written, so their line links point at `7816399`, not at the fixed code. [slice-1-readme.md](slice-1-readme.md#fixes-after-review-2) describes each fix and its test.

| Finding                                                   | Resolved by                                                                                     |
| --------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Fix before slice 2, items 1–5                             | One commit each: `ead1e81`, `b104bf9`, `6ec034f`, `d3ea4eb` and `536a921`, each with its tests |
| Low severity: bad counter values                          | `8142f0f`                                                                                       |
| Low severity: `project list` hides unregistered projects  | `0873bcd`                                                                                       |
| Low severity: non-UTF-8 `TRACKER_HOME`, symlinked `.tmp`, hand-written names and `design` | Still open                                       |
| Code and DESIGN.md, item 1                                | DESIGN.md §4.4 now says what is checked on input, what again on reading, and that the rest is escaped on output |
| Code and DESIGN.md, item 2                                | `1254541`                                                                                       |
| Test gaps 1 and 4                                         | `64d5dc2` and `b04dcb3`                                                                         |
| Test gaps 2 and 3                                         | Done with the fixes they test                                                                   |
| Commit status                                             | `73d3f08` committed this review. The two commit-message points are left as history             |
| Nits: `rust-version`                                      | `a610a92`                                                                                       |

## Fix before slice 2

1. **An item file's name and its `id` aren't checked against each other.** [load_items](../src/store.rs#L221-L242) trusts the `id` inside the file. With `XX-1.json` copied to `XX-9.json`, `list --json` shows `XX-1` twice, with no warning. [highest_number_on_disk](../src/store.rs#L193-L213) reads numbers from file names, so the tripwire and the listing disagree about which items exist. In slice 2, a command that loads `XX-9.json` and saves through `item_path(&item.id)` would overwrite the real `XX-1.json`. Add one `load_item(id)` that refuses a file whose `id` doesn't match its name, or whose `id` doesn't start with its own `project`, and have `load_items` report mismatches.

2. **Fields this version doesn't know are dropped when an item is rewritten.** [Item](../src/item.rs#L10-L22) has neither `deny_unknown_fields` nor a catch-all field. The readme admits this, and PLAN.md's only defence is "install a new binary only between items". Once slice 2 rewrites items, an older binary will silently delete newer fields. The cheapest guard is to refuse, with exit 3, to rewrite an item that has fields this version doesn't know.

3. **The S1 fix only checks input, so other text still reaches the terminal unchanged.**
   - A hand-edited title such as `"a\u001b[2Kfake"` is printed raw by `list`.
   - **Project roots aren't checked at all.** A folder named `r\e[31mRED` is accepted by `project add` and printed raw by both `project add` and `project list` ([main.rs:206-223](../src/main.rs#L206-L223), [main.rs:246](../src/main.rs#L246)).
   - Error messages repeat input back unchanged: `tracker create t --project $'\e[2J\e[HOK'` clears the screen ([main.rs:390-401](../src/main.rs#L390-L401)). [item_path](../src/store.rs#L65-L73)'s error does the same, and slice 4 will pass it typed IDs.

   Escape control and bidi characters at print time, in one place that `table` and `fail` both use. Keep the input check too. Slice 4's `show` and slice 9's `review` need the same escaping.

4. **A relative root in `config.json` matches different folders depending on where you run.** [root_path](../src/config.rs#L33-L40) leaves a relative root relative to the current folder, so a hand-written `"root": "."` claims every folder: `create` outside every project got `DOT-1`. This is the same class of bug as review 1's relative `TRACKER_HOME` (bug 2). Have [config::load](../src/config.rs#L43-L61) refuse a root that isn't absolute after `~/` expansion.

5. **`init` says "Nothing changed" after repairing the store, and doesn't check config.** With `counters.json` deleted, `init` recreates it as `{}` but prints "already exists. Nothing changed." and `"created": false` ([store.rs:84-91](../src/store.rs#L84-L91), [main.rs:175-187](../src/main.rs#L175-L187)). With `config.json` replaced by `garbage`, `init` still exits 0 with the same message. It should say what it put back, warn that counters for projects with items have to be set again, and exit 3 on an unparseable config.

## Low severity

- **Bad counter values.** With `{"XX": 0}`, [next_id](../src/store.rs#L166-L189) writes the counter forward and then `save_item` fails with the confusing `"XX-0" isn't an item ID`. With `18446744073709551615`, `create` **panics** with exit 101 at [store.rs:185](../src/store.rs#L185); a release build would wrap the counter to 0 instead. Check that the counter is at least 1 and use `checked_add`.
- **Non-UTF-8 `TRACKER_HOME` with `--json`.** `json!({"home": store.home})` at [main.rs:185](../src/main.rs#L185) should panic on a non-UTF-8 path, because a `Path` that isn't UTF-8 can't be serialized. This wasn't reproduced, because macOS (APFS) refuses such folder names, but Linux allows them.
- **Symlinked temporary file.** [write_file](../src/store.rs#L271-L298) uses `File::create` on a fixed `<file>.tmp`, which follows a symlink planted there. The 0700 store makes this hard to exploit; removing any stale `.tmp` and opening with `create_new` closes it.
- **Hand-written project names and `--design` values aren't checked for control characters.** `config::load` checks only keys. `design` isn't printed yet, but G6 in slice 6 will open it as a path.
- **`project list` hides items whose project isn't in `config.json`,** while `list` shows them, so the two disagree about what the store holds.

## Where the code and DESIGN.md disagree

1. **§4.4 says "Titles, labels, project keys, kinds and states are all validated."** Kinds and states are validated on load by serde, and keys in `config.json` by `config::load`. Titles, item IDs and each item's `project` are only checked on input (findings 1 and 3).
2. **[item.schema.json](../schemas/item.schema.json) claims a rule it doesn't enforce.** It describes `title` as "One line, with no control characters", but has no pattern for it. Add the pattern, or reword the description to match what files may hold.

## Test gaps

1. **Exit-3 errors aren't checked against `error.schema.json`.** Only the exit-1 helper `error_message` calls `check_schema`. [The lost-counter test](../tests/create_and_list.rs#L634-L644), the bad-key test and the unreadable-store test only parse the output.
2. **No test that `init` reports a repair,** or refuses a corrupt `config.json` (finding 5).
3. **Findings 1–4 have no tests:** a file name that doesn't match its `id`, a hand-edited control character in a title, a root with control characters, and a relative root.
4. **`docs_links.rs` doesn't check `#L…` anchors.** The readme says so. Review 1's line links are already stale, as its Outcome section notes, and this review's will go stale the same way.

## Commit status

- **Uncommitted edits:** `docs/slice-1-readme.md` and `docs/slice-1-review-1.md` (review 1's Outcome section) are changed but not committed. So is this file.
- **`d9c0327 feat: list default to the current folder's project` changes only docs.** `git log` reads as if list scoping shipped, when slice 3 builds it. A `docs:` type would say what it did.
- **The `fix:` subjects describe the bug, not the fix,** such as "fix: non-UTF-8 argument panics". Conventional Commits subjects usually say what the commit does: "fix: don't panic on a non-UTF-8 argument".

## Nits

- Review 1's nits are still open, most importantly: [Cargo.toml](../Cargo.toml) has no `rust-version`, but the code needs 1.89 for `File::lock` (and 1.88 for `if let … &&` chains).

## Before slice 2

1. Commit the doc edits, including this review.
2. Fix findings 1–5, each with a regression test (test gap 3).
3. Check exit-3 errors against the error schema (test gap 1).
4. Add `rust-version = "1.89"` to Cargo.toml.

The low-severity items fit into slice 5's polish, except escaping `design` before slice 6 reads it.

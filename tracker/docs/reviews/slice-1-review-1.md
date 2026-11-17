# Slice 1 review 1

A review of slice 1 ([slice-1-readme.md](../slice-1-readme.md)) against [PLAN.md](../PLAN.md)'s Slice 1 prompt and _Done when_ list, the [DESIGN.md](../DESIGN.md) sections the slice cites (§3, §4.1–4.4), and the conventions in [CLAUDE.md](../../CLAUDE.md).

|                 |                                                                                                                      |
| --------------- | -------------------------------------------------------------------------------------------------------------------- |
| Date            | 2026-11-15                                                                                                           |
| Branch          | `feat/tracker/slice-1`, on top of `90bae1d`, with slice 1 not yet committed                                          |
| Method          | Read the code, ran the test command, and reproduced each finding below against the built binary in a temporary store |
| Line references | Point at the uncommitted working tree as it was at review time                                                       |

## Verdict

Slice 1 is implemented faithfully. Every _Done when_ check passes, `cargo fmt --check`, `cargo clippy -- -D warnings` and `cargo clippy --all-targets -- -D warnings` are clean, and all 23 tests pass. The code is small and plain, and the concurrency test really does exercise the lock.

The problems are edge cases, two security issues worth fixing before slice 4, one place where the code doesn't match DESIGN.md, and the slice not being committed yet.

| _Done when_                                                                                                          | Result                                                                                                             |
| -------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| Three creates in two projects get `XX-1`, `XX-2` and `YY-1`; `list` shows all three; `project list` shows the counts | ✅ Tested                                                                                                          |
| Inside XX's root, `create` needs no `--project`; outside every root, it exits 1                                      | ✅ Tested, including nested roots and the `xx-other` sibling case                                                  |
| An empty or whitespace-only title exits 1; a description is stored in `description`                                  | ✅ Tested                                                                                                          |
| An item file read back and written again is byte-for-byte unchanged                                                  | ✅ Tested, but see [test gap 1](#test-gaps)                                                                        |
| Ten `create` commands started at once get ten different IDs                                                          | ✅ Ten real processes                                                                                              |
| Every command's `--json` output parses                                                                               | ✅ Errors included                                                                                                 |
| DESIGN.md's links still work; after `cargo build`, `git status -- tracker` shows nothing from `target/`              | ✅ A test covers the links. Checked by hand that `target/` shows only as ignored and that there's no nested `.git` |

## Outcome

Added after the fixes. The findings below are left as written on 2026-11-15, so their line links point at the code as it was then, not at the fixed code. [slice-1-readme.md](../slice-1-readme.md#fixes-after-review-1) describes each fix and its test.

| Finding                      | Resolved by                                                                                                                                         |
| ---------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| Bugs 1–8                     | One commit each: `beaf786`, `f7f0be9`, `2b05305`, `21e42d6`, `f0d6da1`, `439ff19`, `96be208` and `e79f279`, each with a regression test             |
| S1–S3                        | One commit each: `f5b121c`, `2d4a474` and `ce8fb99`, each with a regression test                                                                   |
| Code and DESIGN.md, item 1   | `d9c0327`: DESIGN.md §3 now says which project each kind of command covers, and slice 3 will build it for `list`                                   |
| Code and DESIGN.md, item 2   | `315b84d`, `8d5e49f` and `542d7a4`: JSON Schemas in `schemas/` that the tests check every output against, one error shape, and items wrapped as `{"item": …}` |
| Code and DESIGN.md, item 3   | No action needed                                                                                                                                    |
| Test gaps 1–4                | One commit each: `0bf580b`, `b9f4a5b`, `e5bca0b` and `7816399`                                                                                      |
| Test gap 5                   | Done as each fix landed                                                                                                                             |
| Commit status                | `d05d77b` committed slice 1. The renames went in with the crate rather than first, but git detected all 15 (14 at 100%, PLAN.md at 99%), and `git log --follow` finds their history |
| Before slice 2, steps 1–4    | All done                                                                                                                                            |
| Nits                         | Still open                                                                                                                                          |

## Bugs

Each of these was reproduced.

1. **A non-UTF-8 argument panics (exit 101).** `tracker create $'\xff' --project XX` panics because [main.rs:86](../../src/main.rs#L86) calls `std::env::args()`, which panics on non-Unicode input. Use `std::env::args_os()` instead.

2. **A relative `TRACKER_HOME` gives a different store in each folder.** With `TRACKER_HOME=store`, `tracker init` in `a/` prints "Created the store at store.", then `tracker list` in `a/sub` says "Run `tracker init`." The cause is [store.rs:17-24](../../src/store.rs#L17-L24). Refuse a relative path with exit 1. Making it absolute wouldn't help, because the result would still depend on the current folder.

3. **An unreadable store is reported as missing.** With the store at `chmod 000`, commands say "Run `tracker init`." and exit 1, but §3 says exit 3. The cause is `is_file()` at [store.rs:74](../../src/store.rs#L74), which treats a permission error as "doesn't exist". Slice 5's `chmod 000` test will fail on this helper as written. Check `fs::metadata` instead: `NotFound` exits 1 with "Run `tracker init`.", and any other error exits 3.

4. **A closed pipe panics.** With about 3,000 items, `tracker list | head -1` panics with "failed printing to stdout: Broken pipe". The cause is `print!` and `println!` at [main.rs:127-129](../../src/main.rs#L127-L129). Write with `writeln!` and exit quietly on `ErrorKind::BrokenPipe`.

5. **Two projects can share one root.** `tracker project add YY --name y --root <XX's root>` is accepted. Inside that root, `create` then picks XX without saying so, because of the strict `>` in [config.rs:108-113](../../src/config.rs#L108-L113). Refuse a root that's already registered, and keep allowing nested roots.

6. **IDs can be reused if the counter is lost.** With `counters.json` reset to `{}` and only `XX-2.json` left in `items/`, `create` hands out XX-1. If XX-1 had been deleted, its `feat(XX-1)` commits would count as the new item's work, which is exactly what §2.8 warns about. The guard at [store.rs:113](../../src/store.rs#L113) only checks the one file it's about to write. Refuse with exit 3 when a project has item files but no counter, or a counter at or below the highest number on disk. `counters.json` stays the source of IDs (§4.4); the files are only a tripwire.

7. **A partly deleted store gives a raw error.** With `items/` removed, `list` exits 3 with "Can't read …/items: No such file or directory". §4.4 asks for "Run `tracker init`." for a missing store, and `init` would repair this one.

8. **Writes aren't flushed to disk.** [write_file](../../src/store.rs#L165-L176) never calls `sync_all` on the temporary file before renaming it, so a crash can leave an empty `counters.json`, which feeds into bug 6. If `fs::write` fails partway, for example on a full disk, the `.tmp` file is also left behind ([store.rs:170](../../src/store.rs#L170)), which slice 5's "no temporary files are left" check will catch.

## Security

- **S1. Control characters in titles reach the terminal unchanged.** `tracker create $'evil\x1b[2K\x1b[1Gfake: all good\x07' --project XX` is accepted, saved, and printed as-is by `create` and `list`. An agent-written title can rewrite what I see, and later slices put agent text in front of me in `review` and `show` right before I approve. Refuse control characters in titles at [main.rs:285](../../src/main.rs#L285), and in labels from slice 3. That fix also covers line breaks other than `\n` and `\r` (U+2028, NEL, `\v`, `\f`). Escape control characters when printing descriptions in slice 4.

- **S2. Project keys from `config.json` aren't checked, so a key can write outside the store.** With a hand-written key `../../pwned`, `create` run inside its root wrote `pwned-1.json` two levels above `items/`. §4.4 says project keys are validated, but only `project add` checks them; [config::load](../../src/config.rs#L43-L48) doesn't. This matters more in slice 4: [item_path](../../src/store.rs#L51-L53) will receive IDs typed by the user, so `tracker delete ../config` would point at `config.json`. Check keys in `config::load` with the same rule as `normalize_key`, and have `item_path` refuse any ID that isn't `KEY-n`.

- **S3. Store files can be read by other local users.** Under umask 022, `init` creates the store folders as 0755 and its files as 0644. Making the store folder 0700 at `init` is cheap, though optional for a personal tool.

## Where the code and DESIGN.md disagree

1. **`list` ignores the current folder's project.** §3 says "If the current folder is inside a registered project's root, that project is the default", and §2.6 applies that to `digest` (its own project inside a root, every project outside one). The code applies it only to `create`. [slice-1-readme.md](../slice-1-readme.md#decisions-worth-knowing) explains that choice, but slice 3's prompt doesn't mention it, so it will get lost. Decide it in DESIGN.md §3. The suggestion here is to do what `digest` does.

2. **The JSON output is an interface, but it's only written down in slice-1-readme.md.** §3 says renaming a JSON field is a breaking change. Yet the shapes for `init`, `project add`, `project list`, `create`, `list` and the error object `{"error", "exit_code"}` appear only in [slice-1-readme.md](../slice-1-readme.md#json-output). §3's gate refusals use `failures`, so decide how the two errors relate before agents depend on either, and record the shapes in DESIGN.md §3.

3. **Deferred items that are documented and can't happen yet.** No action is needed now:
   - `init` doesn't create `digests.json` or `judge/` (§4.1).
   - `list` sorts by number and shows done and shelved items (slice 3).
   - A bad item file stops `list` instead of being skipped (slice 5).

## Test gaps

1. **The round-trip test can't catch the break it exists for.** [The test](../../tests/create_and_list.rs#L205-L224) writes files with the current binary and reads them with the current library. PLAN.md's first integration risk is about files written by an _older_ binary. Check in a fixture file now, such as `tests/fixtures/slice-1/XX-1.json` with a description and unicode, and keep testing in every later slice that it loads and round-trips.

2. **`HOME` isn't isolated.** [Env::cmd](../../tests/common/mod.rs#L53-L58) sets only `TRACKER_HOME`, and the [concurrency test](../../tests/create_and_list.rs#L263) builds its own `Command`. A future test that forgets `TRACKER_HOME` would write to the real `~/.tracker`. Point `HOME` at a temporary folder as well.

3. **The `~/` expansion of roots has no test.** [config.rs:33](../../src/config.rs#L33) expands a leading `~/`, and [slice-1-readme.md](../slice-1-readme.md#configjson) describes it, but nothing tests it. Isolating `HOME` (gap 2) makes it testable.

4. **`docs_links.rs` doesn't scan the `docs/` folder.** [slice-1-readme.md](../slice-1-readme.md)'s `../src/…` links will break as files move.

5. **None of the bugs above has a regression test yet.** Add one with each fix.

## Commit status

- **Nothing is committed.** There are 15 staged renames, unstaged edits to PLAN.md and CLAUDE.md, and the whole crate (including `Cargo.lock`) is untracked. Yet PLAN.md marks slice 1 `done`, and its build loop says to commit before the next slice starts. Commit the `git mv` on its own first, so `git log --follow` sees clean renames, then the crate.
- **The move itself is right.** HEAD already had DESIGN.md's edits and `judge-eval/`, as PLAN.md's assumptions ask, and every rename is 100% similar.

## Nits

- [main.rs:367-393](../../src/main.rs#L367-L393): the manual grouping loop could be `items.chunk_by(|a, b| a.project == b.project)`.
- [store.rs:153](../../src/store.rs#L153): `sort_by_key` copies the project name on every comparison. `sort_by` avoids it.
- `table` in [main.rs](../../src/main.rs#L403-L426) measures width with `chars().count()`, so CJK text and emoji misalign.
- [Cargo.toml](../../Cargo.toml) has no `rust-version`, though CLAUDE.md relies on `File::lock`, which needs Rust 1.89.

## Before slice 2

1. Commit slice 1: the renames first, then the crate.
2. Fix bugs 1–3, S1 and S2, each with a regression test.
3. Add the slice 1 fixture file (test gap 1) and isolate `HOME` in the harness (test gap 2).
4. Settle whether `list` defaults to the current folder's project, and write the answer into DESIGN.md §3.

The rest fits into slice 5's polish.

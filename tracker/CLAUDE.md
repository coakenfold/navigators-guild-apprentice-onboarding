# tracker

A verification-gated work tracker: a Rust CLI for any project. Spec: [DESIGN.md](docs/DESIGN.md), and every § below is a section of it. Build order, and each slice's prompt and _Done when_: [PLAN.md](docs/PLAN.md).

This folder is part of the guild-portfolio git repo, not a repo of its own. Slice 1 moved DESIGN.md, PLAN.md, walkthrough.html and `judge-eval/` here with `git mv` from the repo's `chapters/02-the-methodology/02-tracking-your-work/`. `walkthrough.html` and `judge-eval/` sit at the crate root. DESIGN.md, PLAN.md and each slice's readme are in `docs/`, and the reviews are in `docs/reviews/`.

## Stack

- Rust stable. 1.99 is installed, and `File::lock` needs 1.89 or later.
- Use the crates from §8, each added in the slice that first needs it:
  - clap (derive), serde and serde_json, jiff, globset, sha2, regex, anstream and anstyle
  - dev crates: assert_cmd, tempfile, trycmd and jsonschema (with `default-features = false`, so it never fetches schemas over the network)
- `git` and `claude` run as subprocesses.

## Where things live

- Suggested modules: `src/main.rs` (clap), `src/store.rs` (paths, lock, atomic writes, counters), `src/item.rs` (serde structs), `src/config.rs`, `src/states.rs`, `src/gates.rs`, `src/triggers.rs`, `src/git.rs`, `src/judge.rs` and `src/bin/tracker-judge.rs`.
- Integration tests go in `tests/`.
- §4.2's example `risky_paths` already name `src/store.rs` and `src/gates.rs`.

## Conventions

- Exit codes are `0`/`1`/`2`/`3` as in §3.
- Every command a slice adds gets `--json` in that slice, and its tests read the JSON, not the text.
- Every `--json` output has a JSON Schema in `schemas/` (§3). `Env::json` checks each output against its command's schema, and panics for a command without one, so a new command needs its schema first. A new item field goes into `schemas/item.schema.json` in the same slice. A command that returns an item wraps it as `{"item": …}`, and every error has `error` and `exit_code`.
- Tests set `TRACKER_HOME` to a temporary folder and never touch `~/.tracker`.
- Anything involving git is tested in throwaway repos made with tempfile. Where the folder rule matters (§2.1), put the project in a subfolder of the throwaway repo.
- Item files are pretty-printed with two-space indents and a fixed field order (§4.3). Each new field gets a serde default and `skip_serializing_if`, so files written by earlier slices still load and come back byte-for-byte unchanged.
- Keep the Rust plain and small, since this is a first Rust project: standard library and plain functions rather than traits or generics.
- Commits are Conventional Commits. From slice 7 on, the work is tracked in `tracker` itself:
  - Commits are scoped to the item, such as `test(TR-n): …` or `feat(TR-n): …`.
  - The tests are committed and seen failing before `tracker start`.

## Test command

Run from this folder: `cargo fmt --check && cargo clippy -- -D warnings && cargo test`. Slice 1 sets this up.

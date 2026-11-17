# Slice 5 review 2

A second review of slice 5 (Bad data and polish). It checks the branch against [PLAN.md](../PLAN.md)'s Slice 5 prompt and _Done when_ list and its integration risks, and against the [DESIGN.md](../DESIGN.md) sections the slice touches (§2.1's title, §3's exit codes and `--json` rule, §4.4's handling of bad data, and §8's colour). It also checks the conventions in [CLAUDE.md](../../CLAUDE.md), the claims in [slice-5-readme.md](../slice-5-readme.md), and each fix that [review 1](slice-5-review-1.md)'s [outcome](slice-5-review-1.md#outcome) claims.

|                 |                                                                                                                                                                                                                                                       |
| --------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Date            | 2026-11-17                                                                                                                                                                                                                                            |
| Branch          | `feat/tracker/slice-5` at `9434db1`, working tree clean. The slice is 17 commits on `main`'s `e3854b8`. Review 1 covered them up to `9765c91`. This review covers that plus the 12 after it, from `2b0a767` to `9434db1`: review 1 itself (`2b0a767`), three bug fixes with their tests (`e76eabb`, `a99f678`, `f43eb23`), two test fixes (`b1da69f`, `af02b3b`), one DESIGN.md fix (`8ab514c`), the flaky lock test's fix (`290eeed`), and four `docs:` commits (`3e1824a`, `0a91129`, `90fad8d`, `9434db1`) |
| Method          | Read DESIGN.md, PLAN.md and review 1 in full, then the diff against `main` and against `9765c91`, and every changed source, test, fixture and schema file in full. Ran the test command and `cargo clippy --all-targets -- -D warnings`. Reproduced each finding marked "Reproduced" against the debug binary in a temporary store. Ran HEAD's tests against `9765c91`'s `src/`, and against HEAD's `src/` with `--unlabel`'s help removed, each in a copy outside the repo. Ran the readme's Try it. Written in a new session, not the one that built the slice or made review 1's fixes |
| Line references | Point at `9434db1`                                                                                                                                                                                                                                    |

## Verdict

Slice 5 still does what the plan asks, and review 1's fixes are real. `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo clippy --all-targets -- -D warnings` and all 194 tests pass. The readme's per-file counts are right, and its Try it prints what it says.

- **Every fix review 1's outcome claims is in the code**, each in its own commit, each with a test. Its "seen failing" claims hold where I could check them: HEAD's `tests/bad_data.rs` and `tests/polish.rs` run against `9765c91`'s `src/` fail exactly the three tests for bugs 1–3 and pass the other 16. With `--unlabel`'s description removed, `every_command_has_help_text` fails on `tracker edit --help`, which it couldn't before.
- **The fixture PLAN.md's integration risks ask for is there.** `tests/fixtures/slice-5/` holds four files, one in each state slice 5 can reach, and they load, list, show and round-trip through the same tests as every earlier slice's.
- **A broken symlink and a file deleted mid-listing now cost one item, not the list**, and "Run `tracker init`" is gone from errors about item files.

I found no bug that loses data. The findings that matter most:

- **Bug 1:** review 1's bug 1 was fixed for one kind of bad entry in `items/`, not the class. A symlink loop still stops the whole list with exit 3, and a named pipe makes `list` and `show` hang.
- **Security 4:** a named pipe in `items/` makes a writing command that names it hang while it holds the store's lock, so every other writing command waits behind it.
- **Finding 5:** §4.4 lists the skipped cases one at a time, and says nothing about the others, so the code handled exactly the ones it names.

## Done when

| Check | Result | Tests |
| --- | --- | --- |
| A hand-corrupted item file is reported with its name, line and reason, and the other items still list | Pass | [bad_data.rs:40](../../tests/bad_data.rs#L40), [bad_data.rs:99](../../tests/bad_data.rs#L99), [bad_data.rs:116](../../tests/bad_data.rs#L116), [bad_data.rs:144](../../tests/bad_data.rs#L144) (new: a broken symlink), and [item_files.rs:47](../../tests/item_files.rs#L47), [item_files.rs:75](../../tests/item_files.rs#L75), [item_files.rs:97](../../tests/item_files.rs#L97). But see bug 1 |
| With no store, commands say "Run `tracker init`." and never panic | Pass | [bad_data.rs:288](../../tests/bad_data.rs#L288), [bad_data.rs:309](../../tests/bad_data.rs#L309), and [bad_data.rs:211](../../tests/bad_data.rs#L211) (new: a store folder that's a file). But see bug 2 |
| A store with `chmod 000` exits 3 | Pass | [bad_data.rs:353](../../tests/bad_data.rs#L353), [bad_data.rs:377](../../tests/bad_data.rs#L377). Neither skipped on macOS |
| No temporary files are left in the store after any command | Pass | [bad_data.rs:416](../../tests/bad_data.rs#L416) |
| Every command has `--help` text | Pass | [polish.rs:39](../../tests/polish.rs#L39), which now checks each option and argument under `--help` too, and [main.rs:1914](../../src/main.rs#L1914) |
| `tracker list \| cat` contains no colour codes | Pass | [polish.rs:131](../../tests/polish.rs#L131), [polish.rs:153](../../tests/polish.rs#L153), [polish.rs:191](../../tests/polish.rs#L191) |
| An empty list prints its empty-state message | Pass | [polish.rs:230](../../tests/polish.rs#L230) |
| `list`'s text starts with a heading row that names each column, lined up with the rows below it | Pass | [polish.rs:272](../../tests/polish.rs#L272), [organise.rs:764](../../tests/organise.rs#L764) |
| _Prompt:_ every write goes to a temporary file that is then renamed | Pass | Unchanged: every write goes through `store::write_file`. [tests/store_writes.rs](../../tests/store_writes.rs) |
| _Prompt:_ error messages that say how to fix the problem | Pass | Review 1's four messages now do: [polish.rs:344](../../tests/polish.rs#L344). Bugs 2 and 3 are two smaller gaps |
| _Prompt:_ decide whether a title needs a length limit | Pass | 200 characters, in §2.1. [polish.rs:323](../../tests/polish.rs#L323) |
| _Integration risks:_ a test for an item file saved by slice 5 | Pass | [fixtures.rs:115](../../tests/fixtures.rs#L115), and [fixtures.rs:161](../../tests/fixtures.rs#L161), [fixtures.rs:180](../../tests/fixtures.rs#L180) and [fixtures.rs:206](../../tests/fixtures.rs#L206) over every slice's files |

## Bugs

1. **A symlink loop still stops the whole list, and a named pipe hangs it.** `e76eabb` taught [`read_listed`](../../src/store.rs#L544-L569) about one kind of entry that isn't a readable file: a symlink whose target is missing. Any other read error still fails the listing with exit 3 ([store.rs:563](../../src/store.rs#L563)), and the only file-type check is `is_dir` ([store.rs:545](../../src/store.rs#L545)), so anything else is read as if it were a file. [`item_text`](../../src/store.rs#L366-L382) has the same shape.

   Reproduced in a store holding XX-1 and XX-2:

   | In `items/` | `list --all` | `show` of it |
   | --- | --- | --- |
   | `XX-7.json`, a symlink to itself | exit 3: "Can't read …/XX-7.json: Too many levels of symbolic links (os error 62)." | exit 3, the same |
   | `XX-8.json` and `XX-9.json`, symlinks to each other | `project list` exits 3, the same message for XX-9 | not tried |
   | `XX-5.json`, made with `mkfifo` | still running after 3 seconds, until killed | still running after 2 seconds, until killed |

   A symlink to `/dev/zero` would be read until memory runs out, since `fs::read_to_string` has no limit. (Found by reading the code. Not run.)

   This is the class review 1's bug 1 was about: §4.4's point is that one bad entry costs one item, not the listing. The loop's message also says nothing about what to do, which the prompt asks of every error. None of these come from tracker itself. They need a hand edit or a shell command, as the broken symlink did.

   Fix: in both `read_listed` and `item_text`, look at the entry before reading it. If `fs::metadata` (which follows links) fails but `fs::symlink_metadata` succeeds, it's a symlink that can't be followed: skip it, with "it's a symlink that can't be followed" and "Remove it from items/.", which also covers the missing target. If `fs::metadata` succeeds and `is_file()` is false, skip it with "it isn't a regular file", which also covers today's folder case. Only then read it. Opening the file and checking its metadata from the open handle would close the gap between the check and the read, but a pipe blocks on `open` too, so checking first is the simpler fix and good enough here.

2. **When `TRACKER_HOME` names a symlink to a folder that doesn't exist, `list` says to run `tracker init`, and `init` fails with "File exists".** `a99f678`'s [`check_not_a_file`](../../src/store.rs#L706-L722) finds a store path that's a file, using `fs::metadata`, which fails for a dangling symlink, so the link passes. [`require_exists`](../../src/store.rs#L168-L178) then finds no `config.json` and says "There's no tracker store at …/store. Run `tracker init`." `init`'s `create_private_dir` ([store.rs:107](../../src/store.rs#L107)) can't make a folder where the link is, and says "Can't create …/store: File exists (os error 17).", with exit 3 and no hint.

   Reproduced with `ln -s $S/nowhere $S/store` and `TRACKER_HOME=$S/store`. The same happens with no `TRACKER_HOME` if `~/.tracker` is such a link, which a store moved to another disk and then unmounted would leave. That's the loop review 1's bug 2 was about, for a rarer case.

   Fix: in `check_not_a_file`, when `fs::symlink_metadata(home)` succeeds and `fs::metadata(home)` fails with "not found", refuse with exit 3: "TRACKER_HOME names $S/store, a symlink to $S/nowhere, which doesn't exist. Create $S/nowhere, or point TRACKER_HOME at a folder." I'd say so rather than have `init` create the link's target, since a missing target is more often a disk that isn't there than a store that was never made.

3. **The `ls -ld` and `chmod` commands the permission messages suggest don't quote the path.** [error.rs:46-49](../../src/error.rs#L46-L49) and [store.rs:186-193](../../src/store.rs#L186-L193) put the path into the command as it is. Reproduced with `TRACKER_HOME="$S/with space/store"` and `chmod 000` on it: `list` says "Check its permissions with `ls -ld $S/with space/store`. If it's yours, `chmod u+rwx $S/with space/store` lets you read and write it again." Pasted, both commands name two paths that don't exist. A home folder rarely has a space in it, so this is minor, but these are the only messages that hand over a command to paste. Fix: wrap the path in single quotes whenever it holds anything but letters, digits and `/._-~`, and escape a single quote inside it as `'\''`. A small `shell_quote` helper in `error.rs` serves both.

## Security

4. **A named pipe in `items/` makes a writing command hang while it holds the store's lock, so every other writing command waits behind it.** [`move_item`](../../src/main.rs#L1397-L1407) takes the lock ([main.rs:1400](../../src/main.rs#L1400)) and then reads the item ([main.rs:1401](../../src/main.rs#L1401)). `edit` does the same ([main.rs:1008-1009](../../src/main.rs#L1008-L1009)). If the item file is a pipe, the read never returns, and the lock is never released.

   Reproduced: with `mkfifo items/XX-5.json`, `tracker start XX-5` was still running after 4 seconds, and a `tracker create b --project XX` started a second after it was still waiting 3 seconds later. Killing `start` let `create` go on. (It then refused, because the counter tripwire counts `XX-5.json` by its name, as it should.)

   §4.4 says the operating system releases the lock if a command crashes, "so there's never a stale lock to clear". A command that hangs doesn't crash, so the lock stays until someone finds and kills it, and an agent would see every `tracker` command it runs hang. Writing a pipe into the store takes a shell command, which §7 already says an agent can run. It's a way to stop the tracker, not to get past it, so it's low severity. Bug 1's fix closes it, since `item_text` would refuse the pipe before reading it. Slice 8's `submit` and slice 9's `approve` read the item under the lock too, so the fix should be in `item_text`, where they all read, not in each command.

I also checked:

- **The new messages escape what they print.** `check_not_a_file`'s message includes the path from `TRACKER_HOME`, and `item_text`'s includes the ID typed. Both go through `report`, which escapes them, or `json_text` with `--json`.
- **`Listed::Gone` can't hide an item that exists.** It's returned only when the read says "not found" and the path isn't even a symlink, so it's a file that was really removed.

## Where the code and DESIGN.md disagree, or the design is unclear

5. **§4.4 lists the entries a listing skips one at a time, and says nothing about the rest.** [§4.4](../DESIGN.md#44-handling-bad-data) names a file that can't be parsed, one that isn't the item its name says, one that isn't UTF-8, and, since `8ab514c`, a symlink to a file that doesn't exist. It then says "A file or folder the store can't read at all, such as one without permission, isn't skipped: that's the store, and exits `3`." The code skips one more case that §4.4 doesn't name, a folder called `XX-4.json` (the readme's table has it), and treats every case §4.4 doesn't name as "can't read at all", which is how bug 1's symlink loop stops the list. A pipe isn't covered by either rule.

   The design should give the rule rather than the list, so the next odd entry doesn't need its own fix. For example, replace "The same goes for an item file that … is a symlink to a file that doesn't exist." with: "The same goes for an item file that parses but isn't the item its name says (below), isn't UTF-8, or isn't a regular file once symlinks are followed, such as a folder, a pipe, or a symlink that leads nowhere." Keep the exit 3 for a permission problem, which is the store's, as now.

## Test gaps

1. **No test covers a symlink loop or a pipe in `items/`** (bug 1 and finding 4). With the fix, add both to [bad_data.rs:144](../../tests/bad_data.rs#L144)'s test, with `list`, `show` and `start`. For the pipe, run the command with a timeout, so a regression fails the test rather than hanging the suite.
2. **No test covers `TRACKER_HOME` naming a symlink to nothing** (bug 2). [bad_data.rs:211](../../tests/bad_data.rs#L211) covers a file and a folder inside a file, and `~/.tracker` as a file.
3. **No test checks that `list --json` keeps stderr empty when it skips a file.** The readme says "With `--json` nothing goes to stderr, and the files are in `skipped`", and the code does that ([main.rs:364-371](../../src/main.rs#L364-L371)). But `Env::json_in` reads only stdout, and every test that reads `skipped` goes through it. If the warnings were printed before checking `--json`, an agent piping stdout and stderr together would get text before its JSON, and every test would still pass. Add one assertion to [bad_data.rs:40](../../tests/bad_data.rs#L40): run `list --json` and check that stderr is empty.

## Process

- **Review 1's fixes were made the way the earlier reviews asked**: one commit per finding, each with its test, the behaviour changes seen failing, and the readme and the outcome brought up to date in their own commits. I checked the seen-failing claims for bugs 1–3 and test gap 1 ([Verdict](#verdict)). I couldn't check the lock test's claim (a temporary test that dropped the lock 3,000 times), since that test isn't in the repo.
- **`290eeed` is a sound fix for a test, not for the lock.** The test now retries the lock for up to 5 seconds after dropping it ([item_files.rs:397](../../tests/item_files.rs#L397)), and still checks that the lock is held while the guard lives ([item_files.rs:412](../../tests/item_files.rs#L412)). The explanation, that a child process another test starts briefly holds a copy of the lock's file, fits what a multithreaded test binary does, and a tracker command has one thread.
- **The readme is out of date in two places.** Its Branch row ([slice-5-readme.md:10](../slice-5-readme.md#L10)) ends at "`e76eabb` to `af02b3b`", and leaves out `290eeed` and the four `docs:` commits after it, though its Fixes table lists `290eeed`. And "Every I/O error says what to do" ([slice-5-readme.md:33](../slice-5-readme.md#L33)) is wider than the sentence after it, which is accurate: bug 1's "Too many levels of symbolic links" and bug 2's "File exists" say nothing about what to do. Everything else checked out: the test counts, the Try it output, the coverage table, the Fixes table and the Still open list.

## Still open, carried forward

From [review 1](slice-5-review-1.md), and in the readme's [Still open](../slice-5-readme.md#still-open):

- **Security: a symlink in `items/` that works is followed.** Review 1 suggested skipping every symlink, and the outcome didn't take it, since that would hide a symlinked item that works. That's a reasonable call. Bug 1's fix keeps it: a symlink to a regular file is still read.
- **Process: review 1 wasn't independent.** This one was written in a new session, but by the same kind of agent, so PLAN.md's #adversarial-review still matters most.
- **Process: the code and its tests landed as one commit** (`0dfa462`). That can't change now. Review 1's fixes didn't repeat it.

The readme also carries slice 2's, slice 3's and slice 4's open items. I checked that the files they're about (`schemas/deleted.schema.json`, `src/item.rs`, `src/tty.rs` and `src/states.rs`) didn't change on this branch, so none of them moved.

## Slice 6 must decide

Review 1's [list](slice-5-review-1.md#slice-6-must-decide) still applies, apart from the fixture, which `af02b3b` added:

- **G1 will break this slice's tests too.** `busy_store` ([polish.rs:83](../../tests/polish.rs#L83)) starts XX-2 and XX-3, and `every_command` ([bad_data.rs:266](../../tests/bad_data.rs#L266)) starts XX-1, all with no criteria.
- **How a refusal of several lines is printed**, since `report` ([main.rs:418](../../src/main.rs#L418)) puts every error on one line.
- **Help and colour for `criterion`, `evidence` and `gates`**, and their entries in [polish.rs](../../tests/polish.rs)'s `COMMANDS`, which nothing enforces.

Three more from this review:

- **G6 reads a file from the project's root, and should follow the same rule as `items/`.** `design` names a file that G6 opens. If that's a pipe, or a symlink that leads nowhere, `gates` and `submit` hang or fail with a bare I/O error, as bug 1's entries do. Whatever rule bug 1's fix and finding 5 settle should apply there: a design file that isn't a regular file fails G6 with a reason, rather than being read.
- **Length limits for a criterion's text and an evidence note.** This slice settled a title's limit (200) as the course's last decision about free text that's printed among other lines. A criterion's text is printed in `show` and in every gate refusal, and an evidence note is free text "saying where to look" (§2.3). Slice 6 should say whether each is one line, and how long it can be, as §2.1 does for a title and a label and §3 for a reason.
- **A fixture saved by slice 6.** Slice 6 adds `criteria` and `written`. `tests/fixtures/slice-5/` is the shape before them, and slice 6 needs its own folder after them, with a criterion of each kind it adds, so slice 7 can check it loads them unchanged.

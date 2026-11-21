# Slice 7 review 2

A second review of slice 7 (Running checks). It checks the branch against [PLAN.md](../PLAN.md)'s Slice 7 prompt and _Done when_ list and its integration risks, and against the [DESIGN.md](../DESIGN.md) sections the slice touches or changes: §2.4's G3, G4, "Checks must fail first" and its paragraph on `tracker gates`, §3's `start`, `gates`, `submit` and `criterion` rows and refusal shape, §4.2's `checks` and `check_timeout_secs`, §4.3's `started` and `verification.checks`, §4.4's lock and re-read, and §7's limits, including the one the fixes added. It also checks the conventions in [CLAUDE.md](../../CLAUDE.md), the claims in [slice-7-readme.md](../slice-7-readme.md), the slice's tracker items (TR-1, TR-2 and TR-3, and TR-13 to TR-18 for review 1's fixes), and whether each fix that [review 1](slice-7-review-1.md)'s [Outcome](slice-7-review-1.md#outcome) records is in the code, with a test.

|                 |   |
| --------------- | - |
| Date            | 2026-11-21 |
| Branch          | `feat/tracker/slice-7` at `30d2a4d`, with a clean working tree. The slice is 30 commits on `main`'s `37abcc3`: the 15 review 1 read, up to its pinned `40a7c7a`, then 15 new: `3ec8698` (review 1), `065a308` (the creating-items walkthrough's note), the 11 commits of TR-13 to TR-18 from `f7ef9dd` to `02ef286`, and `464c239` and `30d2a4d` (the readme's record of the fixes, and review 1's Outcome) |
| Method          | Written in a new session, with no part in building the slice or its fixes. Read PLAN.md in full, and DESIGN.md's §2.4, §3, §4.2, §4.3, §4.4 and §7 in full, which cover every section the slice names and every line its diff changes. Read review 1 in full, the readme, CLAUDE.md, and slice 6 review 2's "Slice 7 must decide". Read the slice's diff against `main` and the diff since `40a7c7a`, `src/checks.rs` and `tests/checks.rs` in full, and the functions of `src/main.rs`, `src/gates.rs` and `src/store.rs` the diff touches. Ran `cargo fmt --check`, both clippy commands and `cargo test`, and each of TR-13 to TR-18's check filters. Ran `tracker show` for TR-1 to TR-18 with the installed binary, and read TR-1's file, `counters.json` and TR's config without changing them. Reproduced each finding marked "Reproduced" against the debug binary in a temporary store. Ran the readme's Try it. Ran TR-13's, TR-14's, TR-15's and TR-16's tests against their test commits' code, extracted with `git archive` into a scratch folder outside the repo |
| Line references | Point at `30d2a4d` |

## Verdict

Slice 7 does what the plan asks, and every fix in review 1's Outcome is in the code, with a test. `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo clippy --all-targets -- -D warnings` and all 267 tests pass. Every _Done when_ bullet, and every criterion of TR-2, TR-3 and TR-13 to TR-18 that has a check, has a test. Each of TR-13 to TR-18's check filters matches exactly one test.

- **Review 1's fixes hold up.** Bugs 1, 4 and 5, finding 8 and both test gaps are fixed as the Outcome says, and findings 6 and 7 and bug 2's limit are in DESIGN.md. I ran TR-13's, TR-14's, TR-15's and TR-16's tests against their test commits, and each failed as the Outcome says: TR-13's after 30.05s, against its limit of 10s.
- **The fixes were tracked as the rules ask.** Each item's tests were committed before its `tracker start`, and the unscoped commits came while no item was in progress.
- **The readme is accurate** about its tests, its Try it output, which I ran exactly as written, and its record of the fixes. One line is stale (process 1).

I found no new bug and no security issue. The findings that matter most:

- **Test gap 2:** no fixture holds a check record with `criterion`, so nothing checks that the shape TR-16 added comes back byte for byte, which is what slice 8's binary must read.
- **Finding 2:** after a reopen and a `criterion remove`, `previous_verification` names the wrong criterion for each check, and DESIGN.md doesn't say whether it should be renumbered.
- **Test gap 1:** TR-13's fix also stops a _passing_ check that leaves a detached process from hanging `tracker`, the more common case, but only the timed-out case has a test.

What was done about each finding, after the review, is under [Outcome](#outcome) at the end.

## Done when

| Check | Result | Tests |
| --- | --- | --- |
| A failing check shows its command, exit code and last lines of output | Pass | [checks.rs:93](../../tests/checks.rs#L93), through `gates --json` and a text `submit` |
| A check running longer than its timeout is stopped and fails | Pass | [checks.rs:141](../../tests/checks.rs#L141), and [checks.rs:437](../../tests/checks.rs#L437) for a process that left the group (TR-13) |
| A broken project check fails G4 even when every criterion check passes | Pass | [checks.rs:169](../../tests/checks.rs#L169) |
| `start` records a check that fails, as expected, and warns about one that already passes | Pass | [checks.rs:198](../../tests/checks.rs#L198), as JSON and as text |
| A passing check's last 20 lines are kept | Pass | [checks.rs:242](../../tests/checks.rs#L242), and [checks.rs:496](../../tests/checks.rs#L496) for a line cut at 500 characters (TR-15) |
| _Prompt:_ each check runs in the project root | Pass | [checks.rs:169](../../tests/checks.rs#L169), with `test -f marker` |
| _Prompt:_ the tool prints the command before running it | Pass | [checks.rs:93](../../tests/checks.rs#L93) and [checks.rs:178](../../tests/checks.rs#L178). Its output's escaping: [checks.rs:614](../../tests/checks.rs#L614) (TR-18) |
| _Prompt:_ the lock isn't held while checks run, and the item is re-read under it (§4.4) | Pass | [checks.rs:568](../../tests/checks.rs#L568) for `start` and [checks.rs:587](../../tests/checks.rs#L587) for `criterion add` after start (TR-18). `start`'s second G1: [gates.rs:287](../../tests/gates.rs#L287). `submit`'s re-read is slice 8's _Done when_ |
| TR-2: a check given after start runs once, is recorded, and is warned about if it already passes | Pass | [checks.rs:275](../../tests/checks.rs#L275), [checks.rs:305](../../tests/checks.rs#L305), [checks.rs:345](../../tests/checks.rs#L345). But see finding 1 |
| TR-3: `criterion remove` keeps `started.checks` on their criteria | Pass | [checks.rs:393](../../tests/checks.rs#L393). But see finding 2, for `previous_verification` |
| TR-13: a timed-out check whose escaped process holds its output fails soon after its timeout | Pass | [checks.rs:437](../../tests/checks.rs#L437). But see test gap 1 |
| TR-14: `criterion add` and `criterion set` refuse a newer tracker's file before running a check | Pass | [checks.rs:470](../../tests/checks.rs#L470) |
| TR-15: a kept line longer than the limit is cut and marked | Pass | [checks.rs:496](../../tests/checks.rs#L496) |
| TR-16: `verification.checks` entries name their criterion | Pass | [checks.rs:518](../../tests/checks.rs#L518). But see test gap 2 |
| TR-17: DESIGN.md says what `gates` runs, what `started.checks` holds, and that interrupting `tracker` can leave a check running | Pass, by reading | Manual criteria. [DESIGN.md:309](../DESIGN.md#L309), [DESIGN.md:434](../DESIGN.md#L434), [DESIGN.md:291](../DESIGN.md#L291), [DESIGN.md:681](../DESIGN.md#L681), [DESIGN.md:800](../DESIGN.md#L800), each quoted in its criterion's evidence |
| TR-18: the re-read after checks, output escaping and a timed-out check's leftover process are tested | Pass | [checks.rs:568](../../tests/checks.rs#L568), [checks.rs:587](../../tests/checks.rs#L587), [checks.rs:614](../../tests/checks.rs#L614), [checks.rs:640](../../tests/checks.rs#L640) |

## Where the code and DESIGN.md disagree, or the design is unclear

1. **§2.4 says a check given "after `start`" runs then, but the code runs it only for an item in progress.** [DESIGN.md:291](../DESIGN.md#L291) says "A criterion that gets a new check after `start`, from `criterion add` or `criterion set`, is run once at that moment as well". [`run_check_given_after_start`](../../src/main.rs#L1482) returns without running anything unless the item is in progress ([main.rs:1493](../../src/main.rs#L1493)). The readme gives the reasons ([slice-7-readme.md:488](../slice-7-readme.md#L488)), but DESIGN.md doesn't, and an item in review, from slice 9, is also "after start". There, a check I give a criterion from a terminal is never run before the work, and `started.checks` keeps the old check's result, or none for a criterion that was manual. After a reject, slice 11's T13 reads that entry as if it were the new check's.

   Found by reading the code. The review state doesn't exist yet, so nothing goes wrong today. Fix: **(a)** say in §2.4 that only an in-progress item runs a check given after start, because an item in review takes criterion changes only from my terminal and T5 holds it anyway, and say in slice 11's notes how T13 treats a check with no entry or an old one. **(b)** Run the check for an item in review too. I'd pick (a): it's my own change, and T5 already puts it in front of me.

2. **After a reopen, `previous_verification` names the wrong criterion for each check once a criterion is removed.** TR-16 gave each check record a `criterion` ([item.rs:120](../../src/item.rs#L120)), and TR-3 renumbers `started.checks` on `criterion remove` ([main.rs:1590-1598](../../src/main.rs#L1590-L1598)). A reopen moves the record to `previous_verification` ([main.rs:2208-2212](../../src/main.rs#L2208-L2212)), and nothing renumbers it there. §2.9 gives fixing a weak check as the reason to reopen, which is when criteria change.

   Reproduced: an item with criteria `one` (`true`) and `two` (`echo two`) was started, submitted, reopened, and given `criterion remove GO-1 1`. Its criteria are now `[1: two]`, but `previous_verification.checks` still says `(1, "true"), (2, "echo two")`: criterion 1 is now `two`, and criterion 2 doesn't exist. §4.3's new bullet ([DESIGN.md:682](../DESIGN.md#L682)) says each check names "its `criterion`, numbered from 1", without saying as of when. Slice 16's `audit` and digest read these records. Fix: **(a)** say in §4.3 that a verification record's `criterion` is the number at that submit, and that `previous_verification` is history, never renumbered, so a reader matches it by `cmd`. **(b)** Renumber `previous_verification.checks` on `criterion remove` as TR-3 does `started.checks`, dropping the removed one's number. I'd pick (a): rewriting a past record to fit today's criteria changes what it says was verified.

## Test gaps

1. **Only the timed-out half of TR-13's fix has a test.** [`run`](../../src/checks.rs#L145-L150) waits at most `LEFTOVER_OUTPUT` ([checks.rs:40](../../src/checks.rs#L40)) for the output once the group is stopped, whether the check timed out or ended. [checks.rs:437](../../tests/checks.rs#L437) tests only a check that times out. A test runner that exits 0 and leaves a detached server holding the pipe is the more common case, and nothing would fail if that path went back to waiting.

   Reproduced: the check `perl -e 'use POSIX; POSIX::setsid(); sleep 8' & echo ok` made `gates` take 8 seconds with `f7ef9dd`'s code, before the fix, and 1.1 seconds now, passing with G3's `secs` 1. Add a test with such a check, with `sleep 30` and the default timeout, that expects `gates` to pass within 10 seconds and kills the escaped process, as the existing test does.

2. **No fixture holds a check record with `criterion`.** `tests/fixtures/slice-7/XX-17.json` was written before TR-16, so its `verification.checks` entries have no `criterion`, and [`slice_7_wrote_two_fixtures_with_check_results`](../../tests/fixtures.rs#L208) doesn't look for one. PLAN.md's integration risks ask that items written by an older binary still load in the newer one, and from now on this binary writes `criterion`. A slice 8 change that renamed it, or moved it out of order, would pass every fixture test. Fix: **(a)** add a fixture written by this binary, such as XX-18, done with a criterion check and a project check, and check its `criterion` values in the slice 7 fixture test, keeping XX-17 as the shape before TR-16, which TR-1's record also has. **(b)** Rewrite XX-17 with this binary. I'd pick (a), since both shapes are in the real store.

## Process

1. **The readme's Status row says its update isn't committed.** [slice-7-readme.md:9](../slice-7-readme.md#L9) ends "This update of the readme isn't committed yet", but `464c239` committed it. Its Branch row ([slice-7-readme.md:10](../slice-7-readme.md#L10)) ends with "this readme's record of them" and doesn't name `464c239` or `30d2a4d`. Fix: drop the sentence, and name both commits.

2. **TR-16 is a `change`, though its code commit is `feat(TR-16)` and it adds a field to the item file.** Review 1's process 4 said that later out-of-scope items that add behaviour should be created as `capability`, and TR-13 to TR-18 were created at 07:08:13Z, after review 1 was saved at 07:07:26Z. A record that can now say which criterion each check came from is something it couldn't do before (§2.1). From slice 10, T1 holds a capability for my review, and a `change` only if it adds a file, which TR-16 didn't. A done item can't be edited, so there's nothing to fix for TR-16. Fix: **(a)** add a line to [CLAUDE.md](../../CLAUDE.md)'s conventions: an item whose work needs a `feat` commit is a `capability`. **(b)** Leave it as advice. I'd pick (a), since advice in a review was missed within a minute of being written.

3. **The order of the fix items' work checks out.** Each of TR-13 to TR-18 had its tests committed before its start: TR-13's `f7ef9dd` at 07:10:29 and its start at 07:10:34, TR-14's `fd61aac` and TR-16's `88bf66f` in the same second as their starts, TR-15's `06375a8` a second before, and TR-18's `02ef286` a minute before. TR-17 changes docs only. `065a308` came before TR-13's start, and `464c239` and `30d2a4d` after TR-18's submit at 07:38:47, while no item was in progress. TR-18's checks passed at its start, since they test code that already worked. Slice 11's T13 would hold such an item, which is right for it, and the readme says how each test was seen failing instead. No action needed.

## Still open, carried forward

- **Review 1, bug 1: the failure doesn't say that something the check started kept running.** TR-13 took option (a)'s wait but not its message, and neither the failure nor §7 says that `tracker` goes on after a second and leaves that process running. Only the readme's limits do ([slice-7-readme.md:552](../slice-7-readme.md#L552), [slice-7-readme.md:565](../slice-7-readme.md#L565)).
- **Review 1, bug 2: Ctrl-C on `tracker` leaves the running check going.** Won't fix in code. [DESIGN.md:800](../DESIGN.md#L800) records it as a limit.
- **Review 1, bug 3: a very large `check_timeout_secs` panics.** Won't fix. Still reproduced: `"check_timeout_secs": 18446744073709551615` makes `gates` panic at [checks.rs:142](../../src/checks.rs#L142).
- **Review 1, bug 5: the schemas' descriptions of `output`** still say "last 20 lines of output" and don't mention the 500-character cut ([item.schema.json:156](../../schemas/item.schema.json#L156), [error.schema.json:98](../../schemas/error.schema.json#L98)).
- **Review 1, finding 6: the shared test helper's check is still `true`** ([mod.rs:329-331](../../tests/common/mod.rs#L329-L331)), left for slice 11.
- **Review 1, process 3: no slice 7 check has been run by a binary other than the code it checks.** The installed `tracker` is still slice 6's: it verified TR-13 to TR-18 `unchecked` on G2 and G6 alone, with no `started` and no `verification.checks`, and `tracker show TR-1` still says it can't show `verification` in full. Install this slice's binary before TR-4 starts.

## Slice 8 must decide

- **Whether G5 runs again after the checks** (review 1's list, still open). A check that leaves an untracked file in the root passes this submit and fails G5 on the next one, or on a shelve.
- **Shelving runs project checks**, so it needs G4 on its own, and its re-read after those checks needs a test like TR-18's. It also inherits bug 2's §7 limit, and the escaped processes that TR-13 now leaves running without a word.
- **`show` prints `started` and each check's saved output**, escaped, with each check's `criterion`. It should say whether a `previous_verification`'s numbers are as at that submit (finding 2).
- **The first records slice 8 writes** carry `commit` and `ref` beside `criterion`. Add their fixture when the shape lands, as test gap 2 asks for this slice's.
- **TR-4 is the first item whose checks the installed `tracker` runs**, once this slice's binary is installed (process 3). Its `start` writes the first real `started` record, so its criteria need checks that fail first.

## Outcome

Added after the fixes. The findings above are left as written, so their line links point at `30d2a4d`, not at the fixed code. Each fix was built as a tracker item, TR-19 and TR-20, and the readme's [Fixes after review 2](../slice-7-readme.md#fixes-after-review-2) lists them too.

This round settled findings 1 and 2, both test gaps and process 1 and 2. Option (a) was taken wherever there was a choice. Findings 1 and 2 are settled in DESIGN.md and PLAN.md, with no code change, and the test gaps with a test and a fixture for code that already worked. Process 3 needed no action. Of the six items carried forward, one is fixed, three won't be fixed, one is left for slice 11, and one is planned for before slice 8 starts. Every item in "Slice 8 must decide" is left for slice 8, whose item is TR-4. The commits after this review (`83740e4`), in order:

- **Fixes:** `61cb404`, `8be2f4b` and `eb80e0f` (TR-19: findings 1 and 2, review 1's bug 5's schema wording, and process 2) and `1549d7d` (TR-20: test gaps 1 and 2).
- **Docs:** the readme's update and the commit that adds this Outcome.

| Finding | Resolved by |
| --- | --- |
| Finding 1: §2.4 says a check given "after `start`" runs then, but the code runs it only for an item in progress | TR-19, `61cb404`, option (a), docs only: §2.4's "Checks must fail first" says a check given after start runs only while the item is in progress. An item in review takes criterion changes only from my terminal, and T5 holds it anyway, so a check given then isn't run, and `started.checks` keeps the old check's result for that criterion, or none for one that was manual, until the item is started again. A note in PLAN.md's slice 11 prompt, under T13, says T13 reads a missing entry as a check that didn't pass, and an old one as it reads any other, since `started.checks` doesn't record the command |
| Finding 2: after a reopen, `previous_verification` names the wrong criterion for each check once a criterion is removed | TR-19, `61cb404`, option (a), docs only: §4.3's `verification.checks` bullet says each `criterion` is numbered as the criteria were at that submit, and that a record is history, so unlike `started.checks` it's never renumbered. After a reopen, a `criterion remove` leaves `previous_verification`'s numbers as they were, and a reader matches its checks to today's criteria by `cmd` |
| Test gap 1: only the timed-out half of TR-13's fix has a test | TR-20, `1549d7d`: `a_passing_check_with_an_escaped_process_finishes_soon_and_passes`, a check whose `perl` process calls `setsid`, writes its pid and sleeps 30 seconds, then `& echo ok`, under the default timeout. `gates --json` must pass within 10 seconds with G3 `ok`, and the test kills the escaped process, as the timed-out test does |
| Test gap 2: no fixture holds a check record with `criterion` | TR-20, `1549d7d`, option (a): XX-18, written by this binary, done with checks on criteria 1 and 3, a manual criterion 2 between them and a project check, so its records name 1, 3 and none. XX-17 stays as the shape before TR-16. Test: `slice_7s_xx_18_names_each_checks_criterion_and_comes_back_byte_for_byte`, which checks the `criterion` values and that the file comes back byte for byte with nothing lost. The existing `slice_7_wrote_two_fixtures_with_check_results` lists the folder's files exactly, so it's renamed `slice_7_wrote_three_fixtures_with_check_results` and expects XX-18 too. Nothing it checked was removed |
| Process 1: the readme's Status row says its update isn't committed | Done in the readme's update: the sentence is gone, and the Branch row names `464c239` and `30d2a4d`, then `83740e4` and this round's fixes. Its other mentions of `slice_7_wrote_two_fixtures_with_check_results` now use the new name |
| Process 2: TR-16 is a `change`, though its code commit is `feat(TR-16)` | TR-19, `eb80e0f`, option (a): CLAUDE.md's Conventions say an item whose work adds behaviour or a field, and so needs a `feat` commit, is created with `--kind capability` (§2.1). TR-16 stays a `change`, since a done item can't be edited |
| Process 3: the order of the fix items' work checks out | No action needed |
| Still open: review 1's bug 1, the failure doesn't say that something the check started kept running | Won't fix. The failure reports the timeout, which is what matters, and the note was an optional part of review 1's option (a). The readme's decisions and limits say the escaped process is left running |
| Still open: review 1's bug 2, Ctrl-C on `tracker` leaves the running check going | Won't fix in code, as review 1's Outcome says. §7's limit stays |
| Still open: review 1's bug 3, a very large `check_timeout_secs` panics | Won't fix, as review 1's Outcome says |
| Still open: review 1's bug 5, the schemas' descriptions of `output` | TR-19, `8be2f4b`: `item.schema.json`'s and `error.schema.json`'s descriptions add "Each kept line is cut at 500 characters, the last of them `…`, if it's longer." |
| Still open: review 1's finding 6, the shared test helper's check is still `true` | Left for slice 11. `Env::add_criterion` still adds `--check true` |
| Still open: review 1's process 3, no slice 7 check has been run by a binary other than the code it checks | Planned: the slice 7 binary is installed with `cargo install --path .` after this Outcome is committed, before TR-4 (slice 8) starts. Until then the slice 6 binary tracks, so TR-19 and TR-20 were verified as TR-13 to TR-18 were |
| Slice 8 must decide: whether G5 runs again after the checks | Left for slice 8 |
| Slice 8 must decide: shelving runs project checks | Left for slice 8. TR-18's two tests show how to test its re-read after checks |
| Slice 8 must decide: `show` prints `started` and each check's saved output | Left for slice 8. Finding 2's §4.3 bullet now says what a `previous_verification`'s numbers mean, so `show` only has to say so |
| Slice 8 must decide: the first records slice 8 writes, and their fixture | Left for slice 8. XX-18 shows the pattern for the shape before `commit` and `ref` |
| Slice 8 must decide: TR-4 is the first item whose checks the installed `tracker` runs | Left for slice 8. The install is process 3's, planned for before TR-4 starts |

Seen failing:

- **Test gap 1:** the new test was run with `run`'s wait for the output made unbounded, `ended.recv()` in place of `ended.recv_timeout(LEFTOVER_OUTPUT)`, then the code was restored. It took 30.03s, against a limit of under 10s. It now takes about 1.5s.
- **Test gap 2:** the XX-18 test was run with `criterion` moved after `cmd` in `CheckRecord`, and with the field renamed, then the code was restored. It failed both times.
- For the readme, the unbounded wait and the moved field were made again in a copy of `1549d7d`'s files outside the repo, and both tests failed again: the first after 30.03s, the second at its byte-for-byte check.
- **Findings 1 and 2, process 2 and bug 5's wording** change docs only, so there was nothing to see fail. TR-19's four criteria are manual, with evidence quoting each passage and naming its commit.

The installed slice 6 binary verified TR-19 and TR-20 as `unchecked`, on G2 and G6 alone, so neither ran its checks through G3. The full test command was run by hand instead. After both, `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo clippy --all-targets -- -D warnings` and all 269 tests pass, 2 more than at `30d2a4d`: 1 in `tests/checks.rs` and 1 in `tests/fixtures.rs`. The readme's Try it output is unchanged.

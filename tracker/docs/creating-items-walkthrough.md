# Walkthrough: creating the items for slices 8–16

This walks through creating `tracker`'s own items for slices 8 to 16, by hand, with the `tracker` CLI. It's step 2 of PLAN.md's [self-tracking checkpoint](PLAN.md#slice-6-criteria-and-simple-gates): each slice becomes one item in project `TR`, ready for its agent to pick up when the slice starts.

**In the real store the IDs are four higher than shown here, not two.** TR-2 and TR-3 were made first, as out-of-scope fixes found while building slice 7, so these nine items were created after them with the commands below unchanged, as TR-4 to TR-12: slice N is TR-(N−4). The IDs in this walkthrough's comments and output are from the throwaway store.

[The self-tracking walkthrough](self-tracking-walkthrough.md) tells the story before this one: registering `TR`, creating TR-1 for slice 7, and fixing one of its criteria by hand. This one gives every command in full, with what it prints. The output is from a throwaway store (`HOME` set to a temporary folder, so the store is its `~/.tracker`) set up the same way, so only the timestamps differ from a run in the real store at `~/.tracker`.

> **Tracker version.** These items were first created with the slice 6 binary. Every command and output here was re-run on 2026-11-24 with `tracker 0.1.0` built from commit `e5dbdd4` (TR-57). That build has all 16 slices of PLAN.md and their review fixes. The commands are unchanged. The one output that changed is `gates` in [step 4](#step-4-check-the-results), which now also reports G5.

## Contents

1. [Before you start](#before-you-start)
2. [How a slice becomes an item](#how-a-slice-becomes-an-item)
3. [Quote every text in single quotes](#quote-every-text-in-single-quotes)
4. [Step 1: Go to the project's root](#step-1-go-to-the-projects-root)
5. [Step 2: Check where you're starting from](#step-2-check-where-youre-starting-from)
6. [Step 3: Create one item per slice](#step-3-create-one-item-per-slice)
7. [Step 4: Check the results](#step-4-check-the-results)
8. [If something goes wrong](#if-something-goes-wrong)
9. [What happens to these items next](#what-happens-to-these-items-next)

## Before you start

- **The checkpoint's first step is done.** `tracker` is installed (from slice 6 at the time, though the current build takes the same commands), the store exists, and `TR` is registered with its checks in `config.json` ([self-tracking walkthrough, step 1](self-tracking-walkthrough.md#1-set-up-the-store-and-register-the-project)).
- **TR-1 exists**, for slice 7. So slice 8 becomes TR-2, and in general slice N becomes **TR-(N−6)**, as long as the items are created in order and nothing else is created in `TR` in between.
- **Each create is permanent in two ways:**
  - IDs are never reused, even after a delete. An item made by mistake uses up its number for good.
  - Each criterion's text is saved in the item's `written` list as well as in `criteria`. Fixing a text later means editing the item file by hand and then running `tracker criterion accept` from a terminal ([self-tracking walkthrough, step 5](self-tracking-walkthrough.md#5-fix-a-mistaken-criterion-with-my-approval)). It's much easier to get the text right the first time.

## How a slice becomes an item

| From PLAN.md                         | In the item                                                                                                   |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------- |
| The slice's outcome ("I can…")       | The title, without "I can" and with a capital first letter                                                    |
| Each _Done when_ bullet              | One `--done-when` criterion                                                                                   |
| A bullet with sub-bullets            | One criterion per sub-bullet, so each can get its own check                                                   |
| The slice adds new behaviour         | `--kind capability` (DESIGN.md §5). Every slice from 8 to 16 does                                             |
| The DESIGN.md section it builds most | `--design docs/DESIGN.md#<section>`, which G6 checks at submit                                                |
| The slice number                     | A short `--description` pointing back to PLAN.md, where the slice's _Prompt_ and _Done when_ stay the source of truth |

| Slice | Item  | Design section                    | Criteria |
| ----- | ----- | --------------------------------- | -------- |
| 8     | TR-2  | §2.4 Gates                        | 8        |
| 9     | TR-3  | §2.7 Human actions                | 8        |
| 10    | TR-4  | §2.5 Review triggers              | 5        |
| 11    | TR-5  | §2.5 Review triggers              | 5        |
| 12    | TR-6  | §2.3.1 Judges                     | 9        |
| 13    | TR-7  | §2.3.1 Judges                     | 4        |
| 14    | TR-8  | §2.4 Gates                        | 4        |
| 15    | TR-9  | §2.3.2 When a judge is wrong      | 5        |
| 16    | TR-10 | §2.6 Digest                       | 6        |

A few criteria are worded differently from PLAN.md's bullets:

- **Slice 8:** the G5 bullet ("In a throwaway repo: …") became two criteria, each starting "In a throwaway repo". The commit-matching bullet became one criterion for the headers that count and one for those that don't.
- **Slice 9:** "Until now the message says this version can't do either, and `allowed` is empty." is left out. It describes today's behaviour, so there's nothing to check.
- **Slice 10:** "the six triggers" names them, T1, T2, T3, T4, T7 and T8, so the criterion reads on its own.
- **Slice 12:** "Tests cover:" became five criteria, each starting "Tests cover".

## Quote every text in single quotes

Most criteria contain backticks, such as `` `start` `` or `` `git gc --prune=now` ``, and some contain double quotes or `!`. Inside **double** quotes the shell treats `` `…` `` as a command to run, and puts that command's output in its place. It can also treat `!` as a history expansion. That's how TR-1's criterion 4 lost its first word. Here's the same mistake in a throwaway store:

```text
$ tracker create "Quote test" --project TR --done-when "`start` records a check that fails"
command not found: start
Created TR-1: Quote test
1 manual criterion: give it a check with `tracker criterion set TR-1 1 --check "…"`.
$ tracker show TR-1
...
Done when:
  1. records a check that fails
       manual, no evidence yet
```

The shell's complaint is easy to miss, and `tracker` never sees the backticks, so it creates the item without the word.

Inside **single** quotes the shell changes nothing, so every command below puts each text in single quotes. The one character a single-quoted text can't contain is a single quote, so each apostrophe is written as `'\''`: close the quote, add an escaped `'`, and open a new quote. `doesn't` is typed as `doesn'\''t`, and `tracker` receives `doesn't`.

## Step 1: Go to the project's root

```sh
cd <guild-portfolio>/tracker
```

`--project TR` picks the project from anywhere, but `--design` names its file relative to the current folder. Run from `TR`'s root, `docs/DESIGN.md` is the right path.

## Step 2: Check where you're starting from

```text
$ tracker project list
KEY  NAME     OPEN  IN-PROGRESS  REVIEW  DONE  SHELVED  ROOT
TR   tracker  0     1            0       0     0        .../guild-portfolio/tracker
$ cat ~/.tracker/counters.json
{
  "TR": 2
}
```

`TR` is registered, with TR-1 in progress. `counters.json` holds the number the next item gets, so the next item is TR-2. If it says anything other than 2, stop: the IDs below won't match their slices.

## Step 3: Create one item per slice

Paste one command at a time, from its `# Slice` comment to its last `--done-when`, and check that it prints the ID in its comment. Don't paste them all at once. If one fails partway through a batch, the rest still run and get the wrong IDs.

When you type a command instead of pasting it, make sure each `\` is the last character on its line. A space after it ends the command there.

### Slice 8 → TR-2

```sh
# Slice 8 -> TR-2
tracker create 'Get items auto-verified against a commit, with saved output and a pinned ref' --project TR --kind capability \
  --design docs/DESIGN.md#2.4 \
  --description 'Slice 8 of docs/PLAN.md. Its Prompt and Done when are there.' \
  --done-when 'In a throwaway repo, an uncommitted change fails G5, and so does an untracked file.' \
  --done-when 'In a throwaway repo, an ignored file doesn'\''t fail G5, and neither does a dirty file outside a subfolder project'\''s root.' \
  --done-when '`feat(XX-1): …` and `fix(XX-1)!: …` count as XX-1'\''s commits.' \
  --done-when 'These don'\''t count as XX-1'\''s commits: a commit scoped to `XX-12`, one headed `XX-1: …`, one that names XX-1 only in its body, and a `feat(XX-1)` commit on another branch.' \
  --done-when 'A passing submit records the SHA and each check'\''s result, duration and output, and creates the ref. Deleting the item removes the ref.' \
  --done-when 'The commit is still readable after its branch is rebased away and `git gc --prune=now` runs. After the test is renamed, `show` still prints the saved output.' \
  --done-when 'Shelving an in-progress item is refused while the tree is dirty or a project check fails, and allowed once both are fixed.' \
  --done-when 'If a check edits the item file while the gates run, `submit` writes nothing and exits 1.'
```

It prints the new ID and title, and a reminder that every criterion is manual for now:

```text
Created TR-2: Get items auto-verified against a commit, with saved output and a pinned ref
8 manual criteria: give each a check with `tracker criterion set TR-2 <n> --check "…"`.
```

Each command below prints the same two lines, with its own ID, title and count.

### Slice 9 → TR-3

```sh
# Slice 9 -> TR-3
tracker create 'See held items in `review` and approve or reject them' --project TR --kind capability \
  --design docs/DESIGN.md#2.7 \
  --description 'Slice 9 of docs/PLAN.md. Its Prompt and Done when are there.' \
  --done-when 'A manual criterion with evidence is held under T6. A `needs-review` item is held under T9. An item with neither still auto-verifies.' \
  --done-when '`submit --json` matches the held shape in §3.' \
  --done-when '`review` lists held items with their reasons and the `show --diff` hint, and prints "Nothing needs you." when it'\''s empty.' \
  --done-when '`echo XX-1 | tracker approve XX-1` is refused. Typing the ID in a real terminal moves the item to done (checked by hand).' \
  --done-when 'An approve whose gate re-run fails leaves the item in review. A reject puts its reason in history.' \
  --done-when 'Every move refused for an item in review, through a command or through `status`, names `approve` and `reject`, and its `allowed` lists both.' \
  --done-when '`show XX-1 --diff` includes XX-1'\''s commits and none of XX-12'\''s.' \
  --done-when 'A refused move from in-progress lists `submit` twice in `allowed`, once to done and once to review, with the same command.'
```

### Slice 10 → TR-4

```sh
# Slice 10 -> TR-4
tracker create 'Have new capabilities, new dependencies, risky paths, edited tests, big diffs and design changes held' --project TR --kind capability \
  --design docs/DESIGN.md#2.5 \
  --description 'Slice 10 of docs/PLAN.md. Its Prompt and Done when are there.' \
  --done-when 'Each of the six triggers (T1, T2, T3, T4, T7 and T8) has one test where it fires and one where it stays quiet.' \
  --done-when 'Adding a test case to an existing test file doesn'\''t fire T4. Changing a line in one does, and so does deleting one.' \
  --done-when 'In a subfolder project, `source_paths: ["src/**"]` matches `<root>/src/…` and not a `src/` folder elsewhere in the repo.' \
  --done-when '`"disabled": ["T8"]` silences T8, but `["T6"]` doesn'\''t silence T6.' \
  --done-when 'After a reject, a resubmit is held again for every trigger that still applies.'
```

### Slice 11 → TR-5

```sh
# Slice 11 -> TR-5
tracker create 'Have moved goalposts, checks that pass before the work, and unscoped commits held' --project TR --kind capability \
  --design docs/DESIGN.md#2.5 \
  --description 'Slice 11 of docs/PLAN.md. Its Prompt and Done when are there.' \
  --done-when 'T13 fires for a `cargo test` filter that matched no tests at start.' \
  --done-when 'T5 fires when a non-interactive command changes or removes a written criterion'\''s text before start, and stays quiet when one adds a criterion or gives one a check.' \
  --done-when '`edit --kind` after start fires T5.' \
  --done-when 'T14 fires for an unscoped commit after start, including one headed `XX-1: …`. It doesn'\''t fire for one made before start, or for an unscoped commit that only touches files outside the project root.' \
  --done-when 'A reopened item resubmitted with unchanged code is held by T13.'
```

### Slice 12 → TR-6

```sh
# Slice 12 -> TR-6
tracker create 'Have yes/no judges grade qualities at submit, using a fake judge command' --project TR --kind capability \
  --design docs/DESIGN.md#2.3.1 \
  --description 'Slice 12 of docs/PLAN.md. Its Prompt and Done when are there.' \
  --done-when 'Tests cover a pass, a fail and an unsure answer.' \
  --done-when 'Tests cover a `"pass_when": "no"` question.' \
  --done-when 'Tests cover malformed output and a timeout.' \
  --done-when 'Tests cover an oversized excerpt, refused before the judge is called.' \
  --done-when 'Tests cover a cache hit, where the fake judge isn'\''t called again.' \
  --done-when '`--judge` refuses invalid JSON, an invalid regex and an `each` that matches nothing. `set` refuses a file whose `text` differs.' \
  --done-when 'G3 fails a judge after a later commit removes every match. Judges don'\''t run when a script check failed.' \
  --done-when 'The fake judge receives the excerpts with comments stripped, and refs look like `c2:file:line:id`.' \
  --done-when '`--json` failures carry `ref`, `answer` and `reason`. A done item'\''s verification record has one judge entry per question.'
```

### Slice 13 → TR-7

```sh
# Slice 13 -> TR-7
tracker create 'Have judges answered by `claude-haiku-5-5`, with judge-eval running the same binary' --project TR --kind capability \
  --design docs/DESIGN.md#2.3.1 \
  --description 'Slice 13 of docs/PLAN.md. Its Prompt and Done when are there.' \
  --done-when 'A fake `claude` on `PATH` that records what it'\''s given confirms that tracker-judge sends the same arguments and prompt as `judge-eval/adapters/claude.mjs`. It also refuses a reply from a different model.' \
  --done-when '`cargo install --path .` installs both binaries.' \
  --done-when '`node run.mjs claude-haiku-5-5` runs tracker-judge.' \
  --done-when 'By hand: one real item is judged from inside a Claude Code session, with the judge seeing no CLAUDE.md and having no tools.'
```

### Slice 14 → TR-8

```sh
# Slice 14 -> TR-8
tracker create 'Have judges that pass before the work, or get retried until they pass, held' --project TR --kind capability \
  --design docs/DESIGN.md#2.4 \
  --description 'Slice 14 of docs/PLAN.md. Its Prompt and Done when are there.' \
  --done-when '`start` refuses a `diff` judge before the item has a commit.' \
  --done-when 'A judge whose answers all pass at start is recorded as already passing and held by T13.' \
  --done-when 'T11 holds an item after `max_failures` different failed requests.' \
  --done-when 'An unchanged failure that `gates` repeats counts once, and fails at start don'\''t count.'
```

### Slice 15 → TR-9

```sh
# Slice 15 -> TR-9
tracker create 'Dispute a judge'\''s wrong fail and decide it myself' --project TR --kind capability \
  --design docs/DESIGN.md#2.3.2 \
  --description 'Slice 15 of docs/PLAN.md. Its Prompt and Done when are there.' \
  --done-when 'A dispute holds the item under T12, showing the excerpt, the model'\''s answer and reason, and the agent'\''s reason.' \
  --done-when 'Disputing a question that passed or got an unsure answer exits 1.' \
  --done-when 'A dispute doesn'\''t stop another failing gate from refusing the submit.' \
  --done-when 'Approving a T12 hold moves the item to done.' \
  --done-when 'Resubmitting unchanged code after a reject needs the dispute again. T12 can'\''t be disabled.'
```

### Slice 16 → TR-10

```sh
# Slice 16 -> TR-10
tracker create 'Audit auto-verified and shelved items, the store, and each judge question'\''s record' --project TR --kind capability \
  --design docs/DESIGN.md#2.6 \
  --description 'Slice 16 of docs/PLAN.md. Its Prompt and Done when are there.' \
  --done-when '`digest` lists every item auto-verified since that project'\''s last ack, judge-passed first. `digest ack --project XX` empties only XX'\''s digest.' \
  --done-when 'From a terminal, `digest ack` asks for `ack`. Typing the project key, or anything else, cancels with exit 1. An item auto-verified in that project while the question waits makes it exit 1, record nothing, and leave that item in the next digest.' \
  --done-when 'A shelve from piped stdin and one run under `script` (skipped where `script` isn'\''t installed) are both listed, marked not from a terminal and from a terminal. So are a delete and an approve made since the last ack, an edit that removes `needs-review`, and one that changes kind from `capability` to `change`. An edit that only adds a label isn'\''t listed.' \
  --done-when '`audit` reports a hand-edited status, a done item with no verification record, and an ID used twice.' \
  --done-when '`audit` reports a `deleted.json` line whose ID is at or above its project'\''s counter in `counters.json`. Such a line can only have been written by hand, and `create`'\''s tripwire refuses that project until the counter is raised (slice 4 review 1, finding 7).' \
  --done-when '`judges` shows the approve and reject counts for each question.'
```

## Step 4: Check the results

`list` should show TR-1 in progress and the nine new items open, in ID order:

```text
$ tracker list --project TR
  ID     PRIORITY  KIND        STATUS       TITLE
TR  tracker
  TR-1   medium    capability  in-progress  See checks run at start and submit, and fail with their output
  TR-2   medium    capability  open         Get items auto-verified against a commit, with saved output and a pinned ref
  TR-3   medium    capability  open         See held items in `review` and approve or reject them
  TR-4   medium    capability  open         Have new capabilities, new dependencies, risky paths, edited tests, big diffs and design changes held
  TR-5   medium    capability  open         Have moved goalposts, checks that pass before the work, and unscoped commits held
  TR-6   medium    capability  open         Have yes/no judges grade qualities at submit, using a fake judge command
  TR-7   medium    capability  open         Have judges answered by `claude-haiku-5-5`, with judge-eval running the same binary
  TR-8   medium    capability  open         Have judges that pass before the work, or get retried until they pass, held
  TR-9   medium    capability  open         Dispute a judge's wrong fail and decide it myself
  TR-10  medium    capability  open         Audit auto-verified and shelved items, the store, and each judge question's record

TR only. `--all` for every project.
Open, in-progress and review items. `--status done,shelved` for the others.
$ cat ~/.tracker/counters.json
{
  "TR": 11
}
```

Then read each item back with `show`, and compare its criteria with the commands above:

```sh
for i in 2 3 4 5 6 7 8 9 10; do tracker show TR-$i; done
```

The ones most likely to go wrong are those with backticks and double quotes, such as TR-2's 3rd, TR-4's 3rd and 4th, and TR-6's 2nd. TR-4's should read like this:

```text
$ tracker show TR-4
TR-4  Have new capabilities, new dependencies, risky paths, edited tests, big diffs and design changes held
  project   TR (tracker)
  kind      capability
  status    open
  priority  medium
  design    docs/DESIGN.md#2.5
  created   2026-11-21T05:46:00Z
  updated   2026-11-21T05:46:00Z

Description:
  Slice 10 of docs/PLAN.md. Its Prompt and Done when are there.

Done when:
  1. Each of the six triggers (T1, T2, T3, T4, T7 and T8) has one test where it fires and one where it stays quiet.
       manual, no evidence yet
  2. Adding a test case to an existing test file doesn't fire T4. Changing a line in one does, and so does deleting one.
       manual, no evidence yet
  3. In a subfolder project, `source_paths: ["src/**"]` matches `<root>/src/…` and not a `src/` folder elsewhere in the repo.
       manual, no evidence yet
  4. `"disabled": ["T8"]` silences T8, but `["T6"]` doesn't silence T6.
       manual, no evidence yet
  5. After a reject, a resubmit is held again for every trigger that still applies.
       manual, no evidence yet

Verification: none.

History: none yet.
```

Every criterion says "manual, no evidence yet", and that's expected. A `gates` dry run shows the same thing from the gates' side: G2 refuses each manual criterion that has no evidence. G5 refuses too, because nothing has been committed for TR-4 yet. The design section passes G6, since no G6 line appears. G6 checks that `docs/DESIGN.md` has a heading numbered 2.5 with text under it.

```text
$ tracker gates TR-4
error: TR-4 would be refused at submit: G2 and G5 failed. This was a dry run, so nothing was changed.
  G2  Criterion 1, "Each of the six triggers (T1, T2, T3, T4, T7 and T8) has one test where it fires and one where it stays quiet.", is manual and has no evidence. Record where to look with `tracker evidence TR-4 1 "…"`, or give it a check with `tracker criterion set TR-4 1 --check "…"`.
  ...
  G5  TR-4 has no commits. Commit its work with a message headed `<type>(TR-4): …`, such as `feat(TR-4): …`, so the proof matches a real commit (§2.1).
  It's open, and only an in-progress item can be submitted.
```

It exits 2, as a refused submit would. Both refusals go away as the slice is built: its tests are committed as `test(TR-4): …` before `start`, and each criterion gets a check.

## If something goes wrong

**A command ran twice.** `tracker` doesn't know two items are the same, so a second run makes a second item:

```text
Created TR-11: Get items auto-verified against a commit, with saved output and a pinned ref
8 manual criteria: give each a check with `tracker criterion set TR-11 <n> --check "…"`.
```

From then on, slice N is no longer TR-(N−6). Note which ID each slice really got. To get rid of the copy, run `tracker delete TR-11` in a real terminal. It says what will be lost and asks you to type the ID:

```text
$ tracker delete TR-11
TR-11 "Get items auto-verified against a commit, with saved output and a pinned ref" is open.
Deleting it removes its file and its 8 criteria.
Any commits scoped to it stay in git, the ID won't be used again, and deleted.json records the delete for the digest.
Type TR-11 to confirm, or anything else to cancel: TR-11
Deleted TR-11. Its line in deleted.json records the delete, and TR-11 won't be used again.
```

Delete only works from an interactive terminal. Run by an agent, or with stdin piped, it's refused with exit 1 and changes nothing. The refusal points to `shelve` instead. If you'd rather keep a record, `tracker shelve TR-11 --reason "duplicate of TR-2"` does that, and prints `TR-11: open → shelved`. Either way, its number is used up: the next item created is TR-12. The next `tracker digest` lists the delete or the shelve.

**A text came out wrong.** Fix it before the item starts. Edit that criterion's `text` in `~/.tracker/items/TR-n.json`, leaving `written` alone, then run `tracker criterion accept TR-n` in a terminal and type the ID. `criterion accept` shows what it records and what it drops before asking, as in the [self-tracking walkthrough](self-tracking-walkthrough.md#5-fix-a-mistaken-criterion-with-my-approval). It refuses a done item.

**A command was refused** with exit 1: a typo in a flag, or a design file it can't find because the command ran outside `TR`'s root. Look at `counters.json` before running it again, to see whether the number was used.

## What happens to these items next

The items stay open until their slice comes up. `TR` works on one item at a time (G1), and PLAN.md's [build loop](PLAN.md#build-loop) runs them in order. For each slice:

1. Install the binary from the merged slice before, `cargo install --path .` on `main`, between items and never while one is in progress ([PLAN.md's integration risks](PLAN.md#integration-risks)).
2. The slice's agent runs `tracker show TR-n`, writes and commits the tests for each criterion, and gives each criterion a check that runs its test:

   ```sh
   tracker criterion set TR-2 1 --check 'cargo test --test <file> <test_name>'
   ```

   Each check runs once by hand first, and its output says `1 failed`. Output saying `running 0 tests` means the filter matched nothing, and `cargo test` exits 0 then, so the check would always pass.
3. `tracker start TR-n`, build with commits scoped to the item, `tracker gates TR-n`, then `tracker submit TR-n`.

Slice 13's last criterion ("By hand: one real item is judged…") can't be a script check. It stays manual and gets evidence instead, `tracker evidence TR-7 4 "…"`, saying where to look. A manual criterion with evidence holds its item for review (T6, built in slice 9).

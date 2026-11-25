# Introduction

I built a bookmark manager using html, css, and javascript. The bookmark manager lets you add bookmarks (url, title, notes, tags), validates the form and saves to localStorage.The bookmarks can be edited and deleted. There is a search feature and it works in conjunction with any selected tag. Supports keyboard navigation and animations respect reduced motion settings.

## Build process

NOTE: **"First try"** means the first agent run produced code that met the slice's "Done when" list without me asking for a correction. Where a slice was run more than once, that is noted.

### 0. Planning: design document and slices

- **Asked:** For a design doc for a simple personal bookmark manager (what it does, what it looks like, what tech). I then ran my [`decompose` skill](../.claude/skills/decompose/SKILL.md) on it to get ordered vertical slices (the skill implements suggestions in the "Breaking Problems Down" chapter). The skill had several trial runs before I used its output.
- **First try?** Yes for the design doc: plain HTML/CSS/JS with `localStorage`, no accounts. The decomposition also came out in a usable shape: a "layer" approach, with a shared-context block for every slice's agent.
- **Adjusted:** The decomposition assumed manual checks only, because the design doc says "no tooling". I later added Playwright anyway. My first local slice prompts didn't follow the order and scope of the Navigators-Guild exercise, with Live Search as layer 4 and Tag Filter as layer 5. I renamed the layers to match the Guild ordering but missed the dependencies inside the prompts. I didn't discover this until I started work on slices 4 and 5 (see those sections). I then edited the prompts in several commits: changed the slice order, reworded slices 4 and 5, and expanded slice 6.
- **Verification failures:** None. There was nothing to run yet.

### 1. Add & list (persisted)

- **Asked:** The full [`prompt-slice-1.md`](./docs/prompts/prompt-slice-1.md): `index.html`, `style.css`, `app.js`, the add form (URL, Title, Notes, Tags), a newest-first list, validation and `localStorage` persistence.
- **First try?** The code was written in one pass. It was **not verified** that way. The agent only syntax-checked `app.js` and tested the tag and URL helper functions in Node, and said it hadn't opened a browser.
- **Adjusted:**
  - I asked how to preview it. The agent suggested Live Server (which wasn't running), then `python3 -m http.server` or `npx serve .`. I settled on `npm run start`, which serves on :3000.
  - Running `npx @playwright/mcp@latest` by hand looked stalled. The agent confirmed that is normal: it is an MCP server waiting for a client on stdin.
  - I had the agent write `bookmark-manager/CLAUDE.md` so future sessions know about the dev server, the Playwright tests and the Playwright MCP.
  - After that I asked for Playwright tests covering the "Done when" list.
  - Later, in a separate session, I asked for stricter URL validation: parse with the `URL` API, allow only `http:` and `https:`, reject missing hostnames, and show an accessible error.
- **Verification failures:** None failed. The 9 tests passed in Chromium, Firefox and WebKit (27 runs). The URL-validation change was checked only against the existing 9 tests. No new tests were added for it, so the new error messages and the screen-reader behaviour weren't exercised.

### 2. Delete

- **Asked:** [`prompt-slice-2.md`](./docs/prompts/prompt-slice-2.md): a Delete button on each bookmark that asks for confirmation, then removes it, saves, and re-renders.
- **First try?** Yes. Three new tests (cancel, confirm then refresh, delete everything) and all 12 passed on the first run.
- **Adjusted:** I asked for the text "Delete" button to become a trash can icon with an accessible label. The agent used an inline SVG with `aria-hidden` and an `aria-label` of "Delete \<title>".
- **Verification failures:** None. Tests only ran on Chromium.

### 3. Edit

- **Asked:** [`prompt-slice-3.md`](./docs/prompts/prompt-slice-3.md): an Edit icon button that fills the same form, switches the button to "Update", saves in place (keeping `id` and `createdAt`), and handles deleting the bookmark being edited.
- **First try?** Yes. 15 tests passed, 3 of them new.
- **Adjusted:** One thing was fixed by the agent. The Edit button inherited the red Delete styling, so it was changed to the accent colour before the tests ran.
- **Verification failures:** None. The new tests covered an in-place update that survives a refresh, invalid input followed by a fresh add, and deleting mid-edit.

### 4. Tag filter

- **Asked:** [`prompt-slice-4.md`](./docs/prompts/prompt-slice-4.md): clickable tag chips that filter the list, combined with search.
- **First try?** No. The feature worked, but the run built more than the slice asked for.
  - My local slice prompts had been generated without following the order and scope of the Navigators-Guild exercise ([Step 5: add layers](https://github.com/Navigators-Guild/apprentice-onboarding/blob/main/01-talking-to-agents/05-first-build.md#step-5-add-layers)). Mine had Live Search as layer 4 and Tag Filter as layer 5.
  - I renamed the layers to match the Guild ordering but didn't check the dependencies inside the prompts. The Tag Filter prompt still assumed Live Search was finished ("combined with search").
  - I ran the Tag Filter prompt without noticing that. The agent built the chips, plus a search box and a permanent "Tag filter" test block (19 tests passed). The slice was too large, but I didn't see that at the time.
  - I only found out while running slice 5 (see below). I then fixed the prompts, rolled back the code and regenerated this slice. The regenerated run built the chips only, verified with a throwaway Playwright spec (15 existing tests passed) and left search for slice 5. It then deleted that spec, so no permanent tag-filter test remained.
- **Adjusted:** After finding the problem in slice 5, I fixed the dependency in the prompts ("change slice order" and "modify the slice prompts" commits), rolled back and regenerated.
- **Verification failures:** None failed. The problem was scope: the prompt pulled in the next slice's work. The gap after regenerating was coverage: no permanent tag-filter test.

### 5. Live search

- **Asked:** [`prompt-slice-5.md`](./docs/prompts/prompt-slice-5.md): a search bar next to the tag filters that narrows the list on every input event and works within the active tag filter. It shows "No bookmarks match your search." when nothing matches.
- **First try?** No. I ran this prompt five times and the runs behaved differently.
  - **Runs 1 and 2 (on top of the oversized slice 4):** the agent said search was already built and only added tests (23 and 25 tests passed). That is how I discovered slice 4 had built more than it should have. It made me read the prompts for layers 4 and 5 and fix them (see slice 4), then roll back.
  - **Runs 3 to 5 (fresh builds after the rollback):** the first searched tags and left no test behind. The other two followed the Goal and did not search tags, and both flagged that the prompt contradicted itself.
  - The contradiction: the Goal said not to search tags, but "Done when" said typing "reference" (a tag) should find the MDN bookmark.
- **Adjusted:**
  - Reading the prompts, I found they differed from the Navigators-Guild spec in two ways. The Guild spec has search work in conjunction with the selected tag, searching only within the tagged bookmarks, but my version searched all text and ignored the selected tag. It also requires the search field to sit next to the tags.
  - I rewrote the Search prompt ("modify the search slice prompt" commits) and updated `DESIGN.md` to match. The Goal now says search must not match tags, searches within the active tag filter, and sits next to the tag chips. I removed the "reference" bullet from "Done when".
  - The agent then added a "Live search" block to the tests (22 tests passed in Chromium) covering title, URL and notes matching, no-match, clearing, tags not matching, search within a tag filter, and zero bookmarks.
  - `DESIGN.md`: I adjusted the ASCII layout mockup so search is next to the tags.
- **Verification failures:**
  - One throwaway check failed on its last assertion. It searched "x" expecting no match, but "x" appears in `example.com`. The bug was in my test, not the app, and the agent deleted the file.
  - The contradiction in the prompt only surfaced because the agents flagged it. Nothing failed.

### 6. Style & phone polish

- **Asked:** [`prompt-slice-6.md`](./docs/prompts/prompt-slice-6.md), which I extended before running it: dark mode, smooth transitions, a collapsible add form, a domain label per bookmark, and keyboard, focus ring, label, contrast and reduced-motion criteria.
- **First try?** The implementation was done in one pass, and 20 of the 22 existing tests passed straight away.
- **Adjusted:** Two tests failed because the form now starts collapsed. The agent added an `openForm` helper to the spec and used it in `addBookmark` and before the two "Save button is back" checks. No test logic changed.
- **Verification failures:** The two failures above. The agent also ran a throwaway check at 375px in light and dark mode with a 300-character URL and a 120-character title, and there was no sideways scroll. Gaps: the screenshot was taken mid-fade, nothing ran a contrast checker, nothing was tested with a screen reader, and only Chromium was run.

### 7. Follow-up polish (after slice 6)

I made these changes after the slice, one request at a time:

1. **The "+" was 1px too low.** The agent drew it with two CSS bars instead of the font glyph, so font metrics no longer affect it. I still have to check it by eye.
2. **Drawer animation.** I asked for a slide with ease-in-out and rows that fade in step with the drawer height. The agent drove it from JavaScript and sampled it frame by frame. That sampling caught a bug in its first version where the lower rows never faded in, and the agent fixed it.
3. **Alarm-clock roll on the toggle text.** The agent added a 350ms roll between "Add a bookmark" and "Close form", with the outgoing text hidden from screen readers.
4. **Same curve for the "+".** The agent changed one timing line to match.
5. **"Add a bookmark" truncated to "Add a".** This was a bug from step 3: the clipping window was sized to the new, shorter text. The old and new text now share one grid cell.

Verification here was mostly scripted sampling rather than looking at the result, and the agent noted each time that it hadn't watched it in a browser.

## Review 1

An adversarial review of the finished build against `DESIGN.md`, written to [`REVIEW.md`](./REVIEW.md) (2026-11-05). The reviewer read `DESIGN.md`, `PROCESS.md`, `index.html`, `app.js`, `style.css` and the tests in full, ran the suite (66 passed, 22 tests × 3 browsers), and probed the app with throwaway Playwright scripts (URL edge cases, keyboard focus, malformed storage, 320/375px widths, two tabs, `file://`) and a contrast calculation. It did not change any project files.

It raised 3 defects (D1–D3), 6 risks (R1–R6), 5 questions (Q1–Q5) and 9 optional improvements (O1–O9, plus one unnumbered "Stricter URL check").

### Triage rules

- **Sorted:** I read `REVIEW.md` and put each finding in one of three groups: a bug to fix now, a real issue that could wait, or an edge case I could ignore.
- **Fixed:** For the bugs, I asked the agent for a plan that follows red/green: write a failing test first, then make the fix.
- **Logged:** For anything I chose not to fix yet, I wrote the decision into the triage tables below, with the date and a revisit trigger.

### Triage: defects

| #   | Critique                                                                                                                              | Assessment                                                                                                                                            | Action          |
| --- | ------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- | --------------- |
| D1  | Keyboard focus falls to `<body>` after using a tag chip, a bookmark's tag, Delete or Update, because `render()` rebuilds the DOM.     | **Valid.** Reproduced by the reviewer in all four cases, and it breaks the slice 6 Done-when "can navigate by keyboard-only".                         | **Fix planned** |
| D2  | One unbroken tag of ~50+ characters makes the page scroll sideways on phones (`scrollWidth` 406 at 320px).                            | **Valid.** `DESIGN.md` §3 says "No sideways scrolling" with no exceptions, even though it's an edge case.                                             | **Fix planned** |
| D3  | URL validation accepts `https:example.com`, `http:/example.com` and (Chromium only) `https://exa mple.com`, and stores them as typed. | **Valid.** `DESIGN.md` §2.1 requires a link "starting with `http://` or `https://`", and the check looked at the parsed protocol, not the typed text. | **Fix planned** |

### Triage: risks

| #   | Critique                                                                                                                | Assessment                                                                                                                                                       | Action                                                                                                                                          |
| --- | ----------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | With two tabs open, the last tab to save overwrites the other's bookmarks (stored titles were `["Two"]`).               | **Valid.** Reproduced, but it only bites when one person leaves the app open in two tabs, and `DESIGN.md` §4 asks for "write the whole array back".              | **Defer.** Acknowledged 2026-11-05. Revisit if sync or multi-tab use becomes a real need (O4 is the ~5-line `storage` listener).                |
| R2  | One stored record missing `tags` or `notes` throws inside `render()` and blanks the whole list with no error.           | **Valid.** Reproduced, but today only my own code writes to storage, so a bad record is unlikely.                                                                | **Fixed later (2026-11-12, commit `37e2a0f`).** First deferred until before JSON import. `loadBookmarks()` now runs every record through `normaliseRecord()`: it defaults `notes` and `tags` and drops unusable records. Tests: the "R1 malformed stored data" block (named by a later review's numbering). |
| R3  | A failed `localStorage` write (full or blocked) fails silently and leaves the form open.                                | **Valid, not reproduced.** It comes from reading the code, and the trigger (quota or blocked storage) is rare for a few hundred small records.                   | **Defer.** Acknowledged. Revisit if I ever see it happen (O5).                                                                                  |
| R4  | Stored URLs become `href`s without re-validation, so an imported `javascript:` URL would be a clickable script link.    | **Partially valid.** Not exploitable today, because only the form writes to storage and the form validates, but it becomes real once import exists.              | **Fixed later (2026-11-12, commit `37e2a0f`), together with R2.** `normaliseRecord()` runs `validateUrl()` on each stored URL and drops records that fail, so a stored `javascript:` URL never becomes a link. Test: "javascript: URL does not break the app". |
| R5  | Light-mode input borders are below the 3:1 non-text contrast guideline (WCAG 1.4.11).                                   | **Valid.** The reviewer measured 2.56:1 against the panel and my red test measured 2.37:1 against the field background, and the slice 6 criteria named contrast. | **Fix planned**                                                                                                                                 |
| R6  | `role="alert"` starts `hidden`, and some screen reader and browser pairs don't announce live regions that start hidden. | **Unverified.** The `aria-invalid`, `aria-describedby` and focus move are in place, but I haven't run screen reader testing.                                     | **Defer.** Acknowledged. Needs a manual VoiceOver or NVDA check; I haven't done one, so this stays open.                                        |

### Triage: questions

| #   | Critique                                                                                                                                                                                         | Assessment                                                                                              | Action                                                                                                                  |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Q1  | `DESIGN.md` and the build have drifted: the design shows an always-open form and lists dark mode as a future idea, but the build has a collapsed form, animations, dark mode and a domain label. | **Valid.** The extras were added in slice 6 after `DESIGN.md` was written, so the doc was out of date.  | **Acknowledged:** the extras are intentional version 1 features. `DESIGN.md` now ends with an Addendum describing them. |
| Q2  | In edit mode the toggle reads "Editing bookmark" but pressing it closes the form and discards the edit, and the form's `aria-label` stays "Add a bookmark".                                      | **Valid.** A screen-reader user hears a name that doesn't match the action.                             | **Fix planned**                                                                                                         |
| Q3  | Escape, and clicking Edit on another bookmark, drop unsaved edits without asking.                                                                                                                | **Invalid as a defect.** It's the intended quick way out.                                               | **Reject.** Intentional. The "Discard changes" label makes it visible.                                                  |
| Q4  | The same URL can be saved more than once.                                                                                                                                                        | **Invalid as a defect.** I allow it on purpose, for example the same page with different notes or tags. | **Reject.** Intentional for version 1.                                                                                  |
| Q5  | Each card shows both the domain label and the full URL, but `DESIGN.md` §2.2 asks for the URL only.                                                                                              | **Invalid.** The duplication is wanted, and the Addendum (A.4) already describes it.                    | **Reject.** Intentional.                                                                                                |

### Triage: optional improvements

| #            | Critique                                                                                                                                                            | Assessment                                                                                                                  | Action                                                                                    |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| O1           | Restore focus after `render()`.                                                                                                                                     | **Valid.** It's the same work as D1.                                                                                        | **Agree, fix planned**                                                                    |
| O2           | Let tags wrap.                                                                                                                                                      | **Valid.** It's the same work as D2.                                                                                        | **Agree, fix planned**                                                                    |
| (unnumbered) | Stricter URL check before parsing.                                                                                                                                  | **Valid.** It's the same work as D3.                                                                                        | **Agree, fix planned**                                                                    |
| O3           | Add `normalizeBookmark()` on load (fixes R2).                                                                                                                       | **Valid.** Small, and import will need it.                                                                                  | **Done with R2 (2026-11-12).** Built as `normaliseRecord()`.                              |
| O4           | Handle the `storage` event (fixes R1).                                                                                                                              | **Valid.** About five lines.                                                                                                | **Defer** with R1.                                                                        |
| O5           | Wrap `saveBookmarks` in try/catch and show the error (fixes R3).                                                                                                    | **Valid.**                                                                                                                  | **Defer** with R3.                                                                        |
| O6           | Use `form.elements.title` rather than `form.title`.                                                                                                                 | **Partially valid.** The shadowing works, but it is a readability trap for my "understand every line" goal. It isn't a bug. | **Defer.**                                                                                |
| O7           | Every bookmark replays `fade-in` on every keystroke and filter click.                                                                                               | **Valid.** It's visual noise, and slice 6 asked for smooth transitions, not constant ones.                                  | **Agree, fix planned.**                                                                   |
| O8           | A bookmark's tag buttons are named just "recipes" and don't show the selected state.                                                                                | **Valid.** They should match the chips, and screen-reader users get no pressed state.                                       | **Agree, fix planned**: `aria-label="Filter by <tag>"`, `aria-pressed`, `selected` class. |
| O9           | Test gaps: no permanent tag-filter tests, no tests for error messages, phone layout, focus or reduced motion, a flaky-prone `openForm`, and a stale CLAUDE.md line. | **Valid.** `PROCESS.md` had already listed several of these gaps, and slice 4 deliberately left no tag-filter test.         | **Agree, fix planned**                                                                    |

### Result of the triage

| Outcome                          | Findings                               |
| -------------------------------- | -------------------------------------- |
| Fixed, with tests                | D1, D2, D3, R5, Q2, O1, O2, O7, O8, O9 |
| Fixed later, with tests (2026-11-12) | R2, R4, O3                         |
| Decided, with documentation      | Q1 (Addendum)                          |
| Rejected: intentional            | Q3, Q4, Q5                             |
| Deferred, with a revisit trigger | R1, R3, R6, O4, O5, O6                 |

**Revisit triggers.** Before I call accessibility done: R6 needs a screen reader run. R1, R3, O4, O5 and O6 wait for a real problem. R2, R4 and O3 were first deferred until before JSON import, but were fixed on 2026-11-12, so import no longer waits on them.

## Refactor

### Refactor Phase 1: markup into HTML (2026-11-09)

- **Asked:** Phase 1 of [`prompt-app-refactor-1.md`](./docs/prompts/prompt-app-refactor-1.md): a pure refactor that moves the bookmark markup and icons out of JavaScript. The existing suite was the safety net.
- **`index.html`:** a hidden SVG sprite with `icon-edit` and `icon-trash` symbols (path data copied unchanged from `app.js`, `viewBox` on each symbol), and `<template id="bookmark-template">` holding the full `<li class="bookmark">`. The stroke and fill attributes sit on the outer `<svg>` and the icons use `currentColor`. The template has no `id`s, so cloning it can't create duplicates.
- **`app.js`:** `createTrashIcon` and `createEditIcon` are deleted, so no `createElementNS` or path data is left in JS. `createBookmarkItem` clones the template, fills it with `textContent` and attributes, removes the empty notes and tags nodes, then sets the `aria-label`s and listeners. User data still never goes through `innerHTML`.
- **Decision:** I kept a small `createTagPill` helper, not a second template. The plan allowed either, and the helper is shorter. The filter chips are untouched.
- **`style.css`:** no change needed. No selectors target `svg` or `path`, and the buttons set `color`.
- **Result:** the suite passed on Chromium (91 tests) with no test changes. Not done: a baseline run before the change, the other two browsers, and a look at the icons in light and dark and in hover and focus states.

## Guide deviations fixed (2026-11-24)

I checked the build against the explicit instructions in the guild's [first-build guide](https://github.com/Navigators-Guild/apprentice-onboarding/blob/main/01-talking-to-agents/05-first-build.md), because the exercise tests following instructions as written. Each fix below is its own commit, with its tests written and seen failing first.

### Dark mode by default (Layer 6)

- **Deviation:** Layer 6 asks for "a subtle color scheme (dark background with light text, dark mode)". The page followed the OS `prefers-color-scheme` setting, so it was light for most people.
- **Fix:** `style.css` now has one dark palette on `:root` with `color-scheme: dark`. The light palette is gone. `DESIGN.md` A.3 says so.
- **Verification:** a new "Guide: dark colour scheme" block checks the background, panel and text luminance under light, dark and no-preference OS settings. It failed 3 of 3 before the change and passes after. The full suite passed in Chromium, Firefox and WebKit (303 tests). The R5 border-contrast test still passes for both OS settings, because both now get the dark palette.

### Inline edit (Layer 3)

- **Deviation:** Layer 3 asks that "Clicking edit should let me change the title, URL, note, and tags inline." My slice 3 prompt reused the add form at the top instead, and slice 6 and the Q2 review fix then built an "edit mode" into that form.
- **Fix:** Edit now swaps the bookmark's card for an inline edit form (a new `<template id="edit-template">` in `index.html`), with Save changes and Discard changes. The add form at the top is add-only, so its edit-mode heading and button names are gone (`setFormMode` is deleted). Add and edit share one `readFields()` check, so validation and messages are the same in both. Unsaved edits are kept in a draft, so a search, a tag click or deleting another bookmark doesn't lose them. `DESIGN.md` A.8 describes it.
- **Tests:** this changed behaviour that existing tests encoded, so I replaced the "Edit" and "Edit mode names" blocks with one "Edit (inline)" block (15 tests). It keeps their intent where it still applies: in-place update keeping `id` and `createdAt`, validation, the heading, switching bookmarks, reduced motion, a long title at 375px, and keyboard use with focus rings. It drops the tests about the add form's edit mode. It adds the guide's own check, "Try editing and then canceling. Does it revert correctly?", for both Discard changes and Escape. Two other tests that edited through the top form were pointed at the inline form: D1 "a successful update", which now expects focus on the bookmark's Edit button rather than the toggle, and the Tag filter "edited to drop it" test.
- **Verification:** before the change, 9 of the new and updated tests failed in Chromium, including "Edit opens filled-in fields inside that bookmark's card, not the add form". After it, the full suite passed in Chromium, Firefox and WebKit (297 tests). Not done: checking the inline form by eye, or with a screen reader.

### Single `index.html` (Step 3)

- **Deviation:** Step 3 asks for "Plain HTML, CSS, and JavaScript in a single file called index.html". My design doc planned three files (`index.html`, `style.css`, `app.js`) and I built it that way. I didn't edit the design doc to match the guide, as Step 1 says to.
- **Fix:** `style.css` is now a `<style>` block in the `<head>` of `index.html`, and `app.js` is a `<script>` block at the end of the `<body>`. The contents were moved unchanged, only indented, and the two files are deleted. `DESIGN.md` §4, `CLAUDE.md` and a note in `REVIEW.md` (whose line links point at the old files) say so.
- **Verification:** a new "Guide: single index.html" block checks that the page has no `<link rel="stylesheet">` or `<script src>`, and that it requests nothing but `index.html`. It failed in all three browsers before the merge and passes after. A second test opens `index.html` over `file://`, as the guide does, and checks that the styles apply and the form opens. It passed both before and after, as a regression guard. The full suite passed in Chromium, Firefox and WebKit (303 tests).

## Learnings

Generating a design document worked quite well. Applying the `/decomposition` skill against that document provided me with vertical slices that I could work on in phases. Scope can creep very easily in the slices with a single sentence.

The adversarial review found real defects my own tests and checks had missed (focus loss, long tags, loose URL validation), and it also raised findings I rejected or deferred. Writing the triage down made each decision explicit. Tests for behaviour that already worked passed straight away, so I had to break the code on purpose to show they could fail, and that exposed two weak tests.

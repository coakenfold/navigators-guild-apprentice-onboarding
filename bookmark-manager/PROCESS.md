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

- **Asked:** The full [`prompt-slice-1.md`](./prompt-slice-1.md): `index.html`, `style.css`, `app.js`, the add form (URL, Title, Notes, Tags), a newest-first list, validation and `localStorage` persistence.
- **First try?** The code was written in one pass. It was **not verified** that way. The agent only syntax-checked `app.js` and tested the tag and URL helper functions in Node, and said it hadn't opened a browser.
- **Adjusted:**
  - I asked how to preview it. The agent suggested Live Server (which wasn't running), then `python3 -m http.server` or `npx serve .`. I settled on `npm run start`, which serves on :3000.
  - Running `npx @playwright/mcp@latest` by hand looked stalled. The agent confirmed that is normal: it is an MCP server waiting for a client on stdin.
  - I had the agent write `bookmark-manager/CLAUDE.md` so future sessions know about the dev server, the Playwright tests and the Playwright MCP.
  - After that I asked for Playwright tests covering the "Done when" list.
  - Later, in a separate session, I asked for stricter URL validation: parse with the `URL` API, allow only `http:` and `https:`, reject missing hostnames, and show an accessible error.
- **Verification failures:** None failed. The 9 tests passed in Chromium, Firefox and WebKit (27 runs). The URL-validation change was checked only against the existing 9 tests. No new tests were added for it, so the new error messages and the screen-reader behaviour weren't exercised.

### 2. Delete

- **Asked:** [`prompt-slice-2.md`](./prompt-slice-2.md): a Delete button on each bookmark that asks for confirmation, then removes it, saves, and re-renders.
- **First try?** Yes. Three new tests (cancel, confirm then refresh, delete everything) and all 12 passed on the first run.
- **Adjusted:** I asked for the text "Delete" button to become a trash can icon with an accessible label. The agent used an inline SVG with `aria-hidden` and an `aria-label` of "Delete \<title>".
- **Verification failures:** None. Tests only ran on Chromium.

### 3. Edit

- **Asked:** [`prompt-slice-3.md`](./prompt-slice-3.md): an Edit icon button that fills the same form, switches the button to "Update", saves in place (keeping `id` and `createdAt`), and handles deleting the bookmark being edited.
- **First try?** Yes. 15 tests passed, 3 of them new.
- **Adjusted:** One thing was fixed by the agent. The Edit button inherited the red Delete styling, so it was changed to the accent colour before the tests ran.
- **Verification failures:** None. The new tests covered an in-place update that survives a refresh, invalid input followed by a fresh add, and deleting mid-edit.

### 4. Tag filter

- **Asked:** [`prompt-slice-4.md`](./prompt-slice-4.md): clickable tag chips that filter the list, combined with search.
- **First try?** No. The feature worked, but the run built more than the slice asked for.
  - My local slice prompts had been generated without following the order and scope of the Navigators-Guild exercise ([Step 5: add layers](https://github.com/Navigators-Guild/apprentice-onboarding/blob/main/01-talking-to-agents/05-first-build.md#step-5-add-layers)). Mine had Live Search as layer 4 and Tag Filter as layer 5.
  - I renamed the layers to match the Guild ordering but didn't check the dependencies inside the prompts. The Tag Filter prompt still assumed Live Search was finished ("combined with search").
  - I ran the Tag Filter prompt without noticing that. The agent built the chips, plus a search box and a permanent "Tag filter" test block (19 tests passed). The slice was too large, but I didn't see that at the time.
  - I only found out while running slice 5 (see below). I then fixed the prompts, rolled back the code and regenerated this slice. The regenerated run built the chips only, verified with a throwaway Playwright spec (15 existing tests passed) and left search for slice 5. It then deleted that spec, so no permanent tag-filter test remained.
- **Adjusted:** After finding the problem in slice 5, I fixed the dependency in the prompts ("change slice order" and "modify the slice prompts" commits), rolled back and regenerated.
- **Verification failures:** None failed. The problem was scope: the prompt pulled in the next slice's work. The gap after regenerating was coverage: no permanent tag-filter test.

### 5. Live search

- **Asked:** [`prompt-slice-5.md`](./prompt-slice-5.md): a search bar next to the tag filters that narrows the list on every input event and works within the active tag filter. It shows "No bookmarks match your search." when nothing matches.
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

- **Asked:** [`prompt-slice-6.md`](./prompt-slice-6.md), which I extended before running it: dark mode, smooth transitions, a collapsible add form, a domain label per bookmark, and keyboard, focus ring, label, contrast and reduced-motion criteria.
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

### 8. Review fixes: D1, D2, D3 and R5 (2026-11-07)

- **Asked:** [`prompt-review-fixes-3.md`](./prompt-review-fixes-3.md): red/green tests first, then fixes for keyboard focus (D1), long tags on phones (D2), URL validation (D3) and light-mode input border contrast (R5).
- **Red:** 19 new tests per browser. In the three browsers, 40 failed and 17 passed. Each failure matched its finding: focus landed on `BODY` (D1); `scrollWidth` was 406 against 320 (D2); the light border measured 2.37:1 (R5); and the error message never appeared for `https:example.com` and `http:/example.com` (D3). Chromium also failed on the space-in-host URL; Firefox and WebKit already reject it, as the prompt expected. The tests that passed before the fix were the dismissed-delete case (nothing re-renders), `ftp://`, the two valid URLs and dark-mode contrast.
- **Green:** `render()` now remembers the focused chip or bookmark tag (via `data-tag` and `data-id`) and refocuses it. Delete and save move focus explicitly. `validateUrl` checks the typed text first. `.tag` got `max-width: 100%` and `overflow-wrap: anywhere`. Light `--field-border` is now `#64748b`.
- **Adjusted:** one test needed a browser-specific key. Playwright's macOS WebKit does not Tab onto buttons (checked with a plain page of three buttons), so the chip Tab test uses Option+Tab there. No existing test changed.
- **Result:** the new tests pass in all three browsers, and the full suite passed: 162 tests across Chromium, Firefox and WebKit. Not done: a manual keyboard run-through or a check for console errors; the tests don't assert on the console.

## Learnings

Generating a design document worked quite well. Applying the `/decomposition` skill against that document provided me with vertical slices that I could work on in phases. Scope can creep very easily in the slices with a single sentence.

# Review: Personal Bookmark Manager vs. DESIGN.md

**Date:** 2026-11-05
**Scope:** `DESIGN.md`, `PROCESS.md`, `index.html`, `app.js`, `style.css`, `tests/bookmarks.spec.js` (plus `prompt-slice-6.md` where PROCESS.md relies on its criteria). All read in full. No project files were changed.

## How this was checked

- **Test suite:** `npx playwright test` gave **66 passed** (22 tests × Chromium, Firefox, WebKit).
- **Browser probes:** throwaway Playwright scripts, kept outside the project, ran against `npm run start` on :3000. They covered URL edge cases, keyboard focus, malformed storage, 320/375px widths, two tabs open at once, and opening `index.html` over `file://`.
- **Contrast:** colour ratios computed from the CSS custom properties with the WCAG formula.
- **Not done:** screen-reader testing, comparing the layout to the §3 mockup by eye, and checking on a real phone. Wherever a finding depends on one of these, it says so.

## Summary

The core of the design works. Add, list, edit, delete, search, tag filter and both empty states all match §2. Data persists in `localStorage` under `bookmarks` with the exact §4 data model, and the app runs from `file://` with no setup. The code is small, readable, and builds the DOM with `textContent` throughout, so user data can't inject HTML.

I found **3 confirmed defects**, all low to medium severity. Two break a stated requirement: URL validation and no sideways scrolling on phones. The third breaks the keyboard-navigation criterion that PROCESS.md says was met. I also found **6 risks**. The biggest one is real data loss when two tabs are open. **The design document and the build have drifted apart** on the collapsible form and dark mode. That needs a decision more than a fix.

| #   | Type   | Finding                                                                | Severity                         |
| --- | ------ | ---------------------------------------------------------------------- | -------------------------------- |
| D1  | Defect | Keyboard focus is lost after tag, Delete and Update actions (fixed)    | Medium                           |
| D2  | Defect | A long tag causes sideways scrolling on phones                         | Low–Medium                       |
| D3  | Defect | URL validation accepts URLs that don't start with `http://`/`https://` | Low                              |
| R1  | Risk   | Two open tabs overwrite each other's bookmarks (reproduced)            | Medium                           |
| R2  | Risk   | One malformed stored record blanks the whole list (reproduced)         | Medium (High once import exists) |
| R3  | Risk   | A failed `localStorage` write fails silently                           | Low                              |
| R4  | Risk   | Stored URLs are used as links without re-validation                    | Low now, higher with import      |
| R5  | Risk   | Light-mode input borders are below 3:1 contrast                        | Low                              |
| R6  | Risk   | Error announcements to screen readers are unverified                   | Unknown                          |

---

## Confirmed defects

### D1. Keyboard focus is lost after tag, Delete and Update actions

- **Status:** fixed (2026-11-07) via [prompt-review-fixes-3.md](prompt-review-fixes-3.md), with red/green Playwright tests in the `Review fixes` block.
- **Requirement:** PROCESS.md says the app "Supports keyboard navigation". `prompt-slice-6.md` sets "Can navigate by keyboard-only" as a Done-when item. DESIGN.md has no keyboard requirement of its own, so this is measured against the slice criterion.
- **Location:**
  - [app.js:508-510](app.js#L508-L510) and [app.js:493](app.js#L493): `render()` clears the list and the chip row with `innerHTML = ""` and builds them again.
  - [app.js:464-467](app.js#L464-L467): `toggleTag`.
  - [app.js:308-318](app.js#L308-L318): `deleteBookmark`.
  - [app.js:130-133](app.js#L130-L133): the end of `handleSubmit`, which closes the form around the focused Update button.
- **Impact:** the button the user just pressed is destroyed or hidden, so focus falls to `<body>`. A keyboard or screen-reader user loses their place after every filter, delete or edit. After a chip, the next Tab went back to "All" in Chromium rather than to the next chip. Filtering by keyboard works, but it's disorienting.
- **Reproduce:** add two bookmarks with different tags. Tab to a tag chip and press Enter, then run `document.activeElement` in DevTools: it is `BODY`. Do the same with a bookmark's own tag button, a Delete button (accept the dialog), and Update while editing. The probe saw `BODY` in all four cases.

### D2. A long tag causes sideways scrolling on phones

- **Status:** fixed (2026-11-07) via [prompt-review-fixes-3.md](prompt-review-fixes-3.md), with red/green Playwright tests in the `Review fixes` block.
- **Requirement:** DESIGN §3, Phones: "No sideways scrolling."
- **Location:** [style.css:313-326](style.css#L313-L326) (`.tag` / `button.tag`) and [style.css:346-349](style.css#L346-L349) (`.tag-filter`). Long URLs and titles wrap correctly thanks to `overflow-wrap: anywhere`, but tag buttons have no wrapping or width limit.
- **Impact:** one unbroken tag of about 50+ characters makes the whole page scroll sideways. Typical tags are fine, so this is an edge case, but the requirement has no exceptions.
- **Reproduce:** at a 320px or 375px viewport, add a bookmark with the tag `averyveryverylongtagnamewithoutanyspacesatallforthetest`. Then run `document.documentElement.scrollWidth` in DevTools. It returned **406** against a viewport of 320 or 375, and the overflowing element was `BUTTON.tag.chip` inside `.tag-filter`. With a 200-character URL and a 120-character title but short tags, there was no overflow.

### D3. URL validation accepts URLs that don't start with `http://`/`https://`

- **Status:** fixed (2026-11-07) via [prompt-review-fixes-3.md](prompt-review-fixes-3.md), with red/green Playwright tests in the `Review fixes` block.
- **Requirement:** DESIGN §2.1: URL "must be a valid link starting with `http://` or `https://`."
- **Location:** [app.js:55-69](app.js#L55-L69) (`validateUrl`), which checks `parsed.protocol` rather than the text the user typed.
- **Impact:** the `URL` parser forgives some typos, so these are accepted and **stored exactly as typed**:
  - `https:example.com` (no slashes)
  - `http:/example.com` (one slash)
  - `https://exa mple.com` (a space in the host), **in Chromium only**. Firefox and WebKit reject it, so validation differs between browsers.

  The links still open, because the browser normalizes them when it follows them. But the list shows malformed URLs, and URL search runs against the malformed text. A smaller point: the `!parsed.hostname` check at [app.js:65](app.js#L65) can never fire for `http:`/`https:`, because the parser throws first. Its error message is unreachable.

- **Reproduce:** enter each value above with any title and press Save. All three save in Chromium. Then run `JSON.parse(localStorage.bookmarks).map(b => b.url)` to see the stored strings. Compare `new URL("https://exa mple.com")` in Chrome and in Firefox DevTools.
- **Note:** PROCESS.md §1 says this stricter validation "was checked only against the existing 9 tests". That is still true. No test covers these messages or cases.

---

## Risks

### R1. Two open tabs overwrite each other's bookmarks (reproduced)

- **Status:** acknowledged (2026-11-05). No fix is planned.
- **Requirement:** DESIGN §4: "After every add, edit, or delete: write the whole array back." The code follows this literally. The risk comes from following it with no other safeguard.
- **Location:** [app.js:19](app.js#L19) loads the array only once. [app.js:33-35](app.js#L33-L35) overwrites storage on every change. Nothing listens for the `storage` event.
- **Impact:** with the app open in two tabs, the last tab to save replaces everything the other tab saved. The bookmark is lost silently. This is an easy mistake for one person to make with a tool they leave open.
- **Reproduce:** open http://localhost:3000 in two tabs. Add "One" in tab 1, then add "Two" in tab 2. Reload either tab: only "Two" is left. The probe confirmed that stored titles were `["Two"]`.

### R2. One malformed stored record blanks the whole list (reproduced)

- **Status:** acknowledged (2026-11-05). No fix is planned.
- **Requirement:** DESIGN §4, On page load: "read and parse it. If nothing is there, use an empty array." This is met. DESIGN §7 names import as "the first thing to add", and import will feed this same path.
- **Location:** [app.js:24-31](app.js#L24-L31) checks only that the data is an array. [app.js:424](app.js#L424) (`bookmark.tags.length`) and [app.js:503-504](app.js#L503-L504) (`text.toLowerCase()` on `notes`) assume every field is present.
- **Impact:** one record without `tags` or `notes` throws inside `render()`. **No bookmarks render at all**, and the user sees no error.
- **Reproduce:** run `localStorage.bookmarks = JSON.stringify([{id:"1",url:"https://x.com",title:"X",createdAt:"2026-01-01T00:00:00.000Z"}])` and reload. The probe got 0 items and the error `Cannot read properties of undefined (reading 'length')`.

### R3. A failed `localStorage` write fails silently

- **Status:** acknowledged (2026-11-05). No fix is planned.
- **Location:** [app.js:33-35](app.js#L33-L35) and [app.js:119-133](app.js#L119-L133).
- **Impact:** `setItem` can throw, for example when storage is full or blocked. The bookmark has already been pushed into memory by then, so `resetForm`/`render` never run and the form stays open with no message. The unsaved bookmark appears on the next render and disappears on reload.
- **Evidence:** this comes from reading the code. I didn't reproduce it. To verify, in DevTools run `Storage.prototype.setItem = () => { throw new DOMException("full", "QuotaExceededError") }` and then try to save.

### R4. Stored URLs are used as links without re-validation

- **Status:** acknowledged (2026-11-05). No fix is planned.
- **Location:** [app.js:392](app.js#L392) (`link.href = bookmark.url`).
- **Impact:** the form validates URLs, but anything already in storage is trusted. Today only you can write to storage, so the risk is small. Once JSON import (DESIGN §7) exists, an imported `javascript:` URL would become a clickable script link.
- **Verify:** store a record with `url: "javascript:alert(1)"` through DevTools, reload, and inspect the title link's `href`.

### R5. Light-mode input borders are below 3:1 contrast

- **Status:** fixed (2026-11-07) via [prompt-review-fixes-3.md](prompt-review-fixes-3.md), with red/green Playwright tests in the `Review fixes` block.
- **Requirement:** `prompt-slice-6.md`, "Text has adequate contrast". Text passes. Control boundaries are a related WCAG 1.4.11 concern that PROCESS.md notes were never checked.
- **Location:** [style.css:10](style.css#L10) (`--field-border: #94a3b8`) and [style.css:179-180](style.css#L179-L180).
- **Measured:** the field border against the white panel is **2.56:1** (WCAG 1.4.11 asks for 3:1). Dark mode is 3.33:1. All text pairs pass at 5.17:1 or better, for example muted text 5.98:1, link 5.17:1 and Delete 6.47:1.
- **Verify:** check `#94a3b8` against `#ffffff` in any contrast checker.

### R6. Error announcements to screen readers are unverified

- **Status:** acknowledged (2026-11-05). No fix is planned.
- **Location:** [index.html:33](index.html#L33) and [app.js:72-74](app.js#L72-L74). The `role="alert"` paragraph starts `hidden` and is revealed with its text in one step.
- **Impact:** some screen-reader and browser combinations don't announce live regions that start hidden. The `aria-invalid` and `aria-describedby` wiring and the focus move are good, and they may cover this.
- **Evidence is insufficient.** No screen reader was run, and PROCESS.md §6 says the same. To verify, use VoiceOver (macOS) or NVDA and submit an empty form.

---

## Questions

1. **Resolved (see the update at the end).** **The design document and the build have drifted apart.** DESIGN §3 shows the add form always open, and §7 lists dark mode as a future idea. The build ships a collapsed form, a JavaScript drawer animation, a rolling label, dark mode and a domain label. `prompt-slice-6.md` says "Don't change any behavior" and also asks for the collapsing form. Should DESIGN.md be updated to describe what was built? Or are these extras outside version 1? Note that the collapsed form adds a click to the "save a link in a few seconds" goal.
2. **Resolved (see the update at the end).** **Accessible names in edit mode.** During an edit, the toggle reads "Editing bookmark" ([app.js:259-266](app.js#L259-L266)), but pressing it closes the form and discards the edit. The form's `aria-label` also stays "Add a bookmark" ([index.html:20](index.html#L20)). Is that what you want a screen-reader user to hear?
3. **Resolved: intentional (see the update at the end).** **Escape discards edits.** Escape anywhere in the form ([app.js:284-286](app.js#L284-L286)) and clicking Edit on a different bookmark both drop unsaved changes without asking. Is that intended?
4. **Resolved: intentional (see the update at the end).** **Duplicate URLs.** The same URL can be saved more than once. Is that fine for version 1?
5. **Resolved: intentional (see the update at the end).** **URL shown twice.** Each card shows both the domain label and the full URL. DESIGN §2.2 asks for "the URL in smaller text" only. Is the duplication wanted?

---

## Optional improvements

- O1. **Covered by [prompt-review-fixes-3.md](prompt-review-fixes-3.md) (D1); fixes-4 confirms it.** **Restore focus after render (fixes D1).** Give chips a `data-tag` attribute and refocus the matching chip after `render()`. After a delete, focus the next bookmark, or the list. After an update, focus the toggle or the edited item.
- O2. **Covered by [prompt-review-fixes-3.md](prompt-review-fixes-3.md) (D2); fixes-4 confirms it.** **Let tags wrap (fixes D2).** Add `overflow-wrap: anywhere` and `max-width: 100%` to `.tag`, then add a 375px `scrollWidth` test.
- **Stricter URL check (fixes D3).** Before parsing, test `/^https?:\/\//i.test(text)`. Either remove the unreachable hostname branch or add a comment explaining why it stays.
- O3. **Acknowledged (2026-11-05). No change is planned.** **Normalize on load (R2).** A small `normalizeBookmark()` that defaults `tags` to `[]` and `notes` to `""`, and drops records with no `url`, `title` or `id`. Import will need it too.
- O4. **Acknowledged (2026-11-05). No change is planned.** **Handle the `storage` event (R1).** Add `window.addEventListener("storage", …)` to reload `bookmarks` and re-render. It's about five lines.
- O5. **Acknowledged (2026-11-05). No change is planned.** **Wrap `saveBookmarks` in try/catch (R3)** and show the error through `showError`.
- O6. **Acknowledged (2026-11-05). No change is planned.** **Readability.** `form.title` and `form.url` ([app.js:94-95](app.js#L94-L95)) only work because form controls shadow built-in properties. `form.title` would normally be the form's own `title` attribute. `form.elements.title` is clearer, which supports the "understand every line" goal.
- O7. **Planned in [prompt-review-fixes-4.md](prompt-review-fixes-4.md).** **Animation.** Every item replays the `fade-in` animation on every keystroke and filter click, because the list is rebuilt each time ([style.css:246](style.css#L246), [app.js:535](app.js#L535)). Consider animating only newly added items.
- O8. **Planned in [prompt-review-fixes-4.md](prompt-review-fixes-4.md).** **Tag buttons on bookmarks.** Their accessible name is only the tag (for example "recipes"), and they don't show the selected state. `aria-label="Filter by recipes"` and the `selected` class would match the chips.
- O9. **Planned in [prompt-review-fixes-4.md](prompt-review-fixes-4.md).** **Test gaps** (PROCESS.md already names some of these):
  - There is no permanent tag-filter test block. Cover: a chip filters, clicking it again clears, "All" clears, clicking a bookmark's tag filters, and the filter resets when its last tag disappears ([app.js:491](app.js#L491)).
  - There are no tests for the URL error messages, the phone layout, keyboard focus or reduced motion.
  - The `openForm` helper uses `isVisible()`, which doesn't retry ([tests/bookmarks.spec.js:18-21](tests/bookmarks.spec.js#L18-L21)), and fields are filled while the drawer is still animating. Everything passes today, but this pattern tends to become flaky.
  - CLAUDE.md still says the spec "covers the slice 1 Done-when list". It now covers slices 1–5, except tag filtering.

---

## Verified as matching the design

- **§2.1:** URL and Title are required. Tags are trimmed, lowercased and de-duplicated (`JavaScript, reference , javascript` → `["javascript","reference"]`). `createdAt` is set automatically.
- **§2.2:** newest first, the title opens in a new tab with `rel="noopener noreferrer"`, and the URL, notes, tags and date are shown.
- **§2.3:** Edit fills the form and updates in place, keeping `id` and `createdAt`. Delete confirms with exactly "Are you sure?".
- **§2.4 and §2.5:** search is live, ignores case, covers title, URL and notes but not tags, sits next to the chips, and combines with the tag filter. "All" and clicking a chip again both clear the filter.
- **§2.6:** both empty-state messages appear with the exact wording, and in the right cases.
- **§3:** 800px max width, `system-ui, sans-serif`, one accent colour, pill tags, bookmarks separated by a thin line, and a stacked layout under 500px. There's no sideways scroll with long URLs or titles at 320px and 375px; long tags are the exception (see D2).
- **§4:** three files, no framework, `localStorage` key `bookmarks`, and the exact data model. Opening `index.html` from `file://` works and persists in Chromium, Firefox and WebKit.
- **Reduced motion:** honoured in both the CSS and the JavaScript animations.

---

## Update (2026-11-05): design drift resolved

The author confirmed that the collapsible form, the drawer and label animations, dark mode, the domain label and the icon buttons are **intentional** version 1 features, added after DESIGN.md was written. DESIGN.md now ends with an **Addendum** that describes them, so Question 1 is closed.

This changes the review in three ways:

- **Mismatches with the design:** differences from the §3 mockup (collapsed form, dark mode) and the §2.2 card (domain label shown as well as the URL) are no longer mismatches. Question 5 is answered by Addendum A.4.
- **Question 3** (Escape discarding edits) was left open by this update. It has since been resolved (see the update at the end).
- **Defects:** D1 and D2 affect these added features directly, so they still apply. No defect or risk was removed by this update.

## Update (2026-11-05): Question 2 decided

The author chose to keep the toggle's name about its action and move the edit status into the form:

- **Toggle:** always reads "Add a bookmark" when the form is closed and "Close form" when it is open, in both add and edit mode. "Editing bookmark" is removed. `aria-expanded` still announces the state.
- **Form heading:** a visible `<h2 id="form-heading">` inside the form reads "Add a bookmark" or "Edit bookmark: <title>". The form uses `aria-labelledby="form-heading"` instead of its fixed `aria-label`, so screen readers hear the same name that sighted users see.
- **Buttons in edit mode:** the submit button reads "Save changes" (replacing "Update") and the cancel button reads "Discard changes". They read "Save" and "Cancel" in add mode.

The work is described in [prompt-review-fixes-2.md](prompt-review-fixes-2.md), which keeps Escape behaviour (Question 3) and focus handling (D1) out of scope. **It has not been built yet.** Question 2 is marked "Decided, fix pending" until that prompt has run and its tests pass. Then it can be marked resolved.

- **Built (2026-11-07):** prompt-review-fixes-2.md has run and Question 2 is resolved. The toggle, form heading and buttons now use the names above, DESIGN.md Addendum A.1 and A.4 describe them, and new `Edit mode names` Playwright tests cover them (105 tests pass in Chromium, Firefox and WebKit).

## Update (2026-11-05): Questions 3, 4 and 5 resolved

The author confirmed that all three behaviours are intentional for version 1:

- **Question 3, Escape and switching edits discard unsaved changes:** this is intended. Escape is a quick way out of the form, and choosing Edit on another bookmark replaces the current edit without asking. This doesn't affect [prompt-review-fixes-2.md](prompt-review-fixes-2.md), which already leaves Escape behaviour unchanged. Once that fix lands, the cancel button in edit mode reads "Discard changes", which makes this behaviour visible.
- **Question 4, duplicate URLs:** allowed on purpose. Saving the same URL twice, for example with different notes or tags, is not treated as an error.
- **Question 5, URL shown twice:** the domain label and the full URL line are both intended, as described in DESIGN.md Addendum A.4.

All the review's questions now have an answer. Question 2 was waiting on its fix at the time; it has since been built (see the Question 2 update above). The defects (D1–D3) and risks (R1–R6) are unchanged.

## Update (2026-11-05): fix planned for D1, D2, D3 and R5

[prompt-review-fixes-3.md](prompt-review-fixes-3.md) addresses these four findings test-first. For each finding, a Playwright test that reproduces it is added and run first, and must fail (red). Then the code is fixed until the test passes (green), and finally the full suite runs in Chromium, Firefox and WebKit.

- **D1:** focus goes back to the re-rendered chip or tag after filtering. After a delete it moves to the next bookmark's title (or the form toggle if the list is empty), and after a save it moves to the form toggle.
- **D2:** tag pills are allowed to wrap. Tests check for no sideways scrolling at 320px and 375px with a 56-character tag.
- **D3:** a URL must start with `http://` or `https://`, ignoring case, and contain no spaces. The unreachable hostname check is removed or explained with a comment. The space-in-host test is expected to fail before the fix only in Chromium.
- **R5:** the light-mode `--field-border` is darkened to at least 3:1 against both the panel and the field background. For example, `#64748b` measures 4.76:1 and 4.40:1. A test measures the contrast in both themes.

The prompt finds the submit button with a selector rather than by its label, so it works whether it runs before or after [prompt-review-fixes-2.md](prompt-review-fixes-2.md). R1–R4 and R6 are still open and out of scope. The four findings stay marked "fix planned" until the prompt has run. The prompt then marks them fixed and records the red and green results in PROCESS.md.

## Update (2026-11-05): plan for O7, O8 and O9

[prompt-review-fixes-4.md](prompt-review-fixes-4.md) covers these improvements test-first and **must run after [prompt-review-fixes-3.md](prompt-review-fixes-3.md)**. O1 and O2 are the same work as D1 and D2, which fixes-3 already does, so fixes-4 only checks that those tests pass. If they don't, it stops.

- **O7:** the fade-in plays only for bookmarks that **enter** the visible list: a new bookmark, or one revealed when a filter or search is cleared. Bookmarks that stay in view no longer replay it on every keystroke or chip press. Nothing animates on first load or with reduced motion. This keeps the "smooth transitions when filtering and searching" that slice 6 asked for.
- **O8:** a bookmark's tag buttons are named "Filter by <tag>" and show their pressed state with `aria-pressed` and the `selected` class, matching the chips. The visible text and the filter-row chip names don't change.
- **O9:**
  - New tests: a `Tag filter` test block, form-error message and ARIA tests, phone-layout checks at 375px, keyboard open and close with visible focus rings, and reduced-motion tests.
  - A more reliable `openForm` helper, checked with `--repeat-each=5`.
  - An updated line in CLAUDE.md.

Tests for behaviour that already works can't fail first, so each one has to be **shown to fail against a temporary, undone break** of the code it covers. The unnumbered "Stricter URL check" item stays with fixes-3 (D3). O3–O6 are still open and out of scope. These items stay marked "planned" until the prompt has run. It then marks them done and records the results in PROCESS.md.

## Update (2026-11-05): remaining risks acknowledged

The author has acknowledged risks **R1, R2, R3, R4 and R6** and accepted them for now. **No fixes are planned.** R5 is being handled separately in [prompt-review-fixes-3.md](prompt-review-fixes-3.md).

The matching optional improvements (O3 for R2, O4 for R1, O5 for R3) stay listed in case this changes. Two risks are worth revisiting before JSON import (DESIGN §7) is built, because import would make them more likely to happen:

- **R2:** a malformed record blanks the whole list.
- **R4:** stored URLs are used as links without being checked again.

## Update (2026-11-05): remaining optional improvements acknowledged

The author has acknowledged optional improvements **O3, O4, O5 and O6**. **No changes are planned.** The others are already covered:

- **O1 and O2:** done by [prompt-review-fixes-3.md](prompt-review-fixes-3.md) and confirmed by [prompt-review-fixes-4.md](prompt-review-fixes-4.md).
- **O7–O9:** planned in [prompt-review-fixes-4.md](prompt-review-fixes-4.md).
- **The unnumbered "Stricter URL check":** covered by fixes-3, as part of D3.

Every finding in this review now has a status: planned, decided, resolved or acknowledged.

## Update (2026-11-07): D1, D2, D3 and R5 fixed

D1, D2, D3 and R5 are fixed and marked as such above. The new tests are in the `Review fixes` block of `tests/bookmarks.spec.js`: `D1 keyboard focus` (9 tests), `D2 long tags on phones` (320px and 375px), `D3 URL validation` (4 rejected inputs, 2 accepted) and `R5 input border contrast` (light and dark). All pass in Chromium, Firefox and WebKit.

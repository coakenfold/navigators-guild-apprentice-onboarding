# Review 4: Bookmark Manager against DESIGN.md

**Date:** 2026-11-12
**Reviewed:** `index.html`, `app.js`, `style.css`, `tests/bookmarks.spec.js`, against `DESIGN.md` (§1–§7 and the 2026-11-05 Addendum, which wins where it differs)
**Not read:** `REVIEW.md`, `REVIEW-2.md`, `REVIEW-3.md` (on purpose, so this review is independent)

## Summary

The app meets nearly all of the design. Add, list, edit, delete, live search, tag filtering, empty states, validation, the collapsible form, motion, dark mode and the phone layout all behave as the spec says. The test suite passes in full: **282/282 on Chromium, Firefox and WebKit** (`npx playwright test`, 3.6 min).

I found **2 confirmed defects in the app** and **2 in the tests**. The larger exposure is in what the design does not cover: bad data in `localStorage`, failed writes, two open tabs, and render speed at the "few thousand bookmarks" size that §5 promises. I reproduced each of these in a browser. They are listed as risks, not defects, because no requirement in the design covers them.

| ID | Type | Severity | Finding |
| --- | --- | --- | --- |
| D1 | Defect | Medium | A long domain without hyphens causes sideways scrolling at 320px |
| D2 | Defect | Low | The Delete button's focus ring is red, not the accent colour |
| T1 | Test defect | Low | The Edit test says it checks position but never does |
| T2 | Test defect | Low | A test name describes behaviour the test doesn't exercise |
| R1 | Risk | High | One malformed stored record blanks the whole app |
| R2 | Risk | High | Two open tabs overwrite each other's bookmarks (data loss) |
| R3 | Risk | Medium | A failed `localStorage` write is silent and leaves memory out of sync |
| R4 | Risk | Medium | Typing a search with 3,000 bookmarks takes 130–160ms per keystroke |
| R5 | Risk | Low | The "Saved" status may not be announced by screen readers |
| R6 | Risk | Low | Some tests depend on timing |
| R7 | Risk | Low | Gaps in test coverage |
| Q1–Q5 | Question | — | Behaviour the design doesn't decide |

**How findings are classed.** A *confirmed defect* breaks a written requirement and was reproduced. A *risk* is a problem I either reproduced or can show in the code, where no requirement covers the behaviour. A *question* is a product decision for the owner.

---

## Confirmed defects

### D1. A long domain without hyphens causes sideways scrolling on narrow phones

- **Requirement:** §3 "Phones: … No sideways scrolling." A.6 (phone layout). A.4 (domain label next to the title).
- **Location:** [style.css:301-305](style.css#L301-L305). `.bookmark-domain` has no `overflow-wrap: anywhere`. The title ([style.css:290-295](style.css#L290-L295)), URL ([313-317](style.css#L313-L317)) and notes ([319-322](style.css#L319-L322)) all have it. The `body` rule's `overflow-wrap: break-word` ([style.css:43](style.css#L43)) doesn't help here. `break-word` doesn't shrink a flex item's min-content width, and the domain `<span>` is a flex item inside `.bookmark-heading`.
- **Impact:** Dots aren't line-break opportunities, so a domain without hyphens is one unbreakable word. At 320px wide, a domain of about 42 characters or more pushes the page sideways. The label also runs past the card's padding before that point. Hyphenated domains wrap at the hyphens and are fine.
- **Reproduced (all three engines):**

  | Domain | Width | Page overflow |
  | --- | --- | --- |
  | `internationalcompanyname.lightning.force.com` | 320px | 10–11px (Chromium, Firefox, WebKit) |
  | `averyveryverylongsubdomainnamewithnobreaks.example.com` | 320px | 93px (Chromium) |
  | `engineering-handbook.mycompany.atlassian.net` | 320px | 0 (breaks at the hyphen) |

- **Steps:** Set the viewport to 320×640. Add a bookmark with URL `https://internationalcompanyname.lightning.force.com/`. Then compare `document.documentElement.scrollWidth` with `clientWidth`.
- **Test to add:** Extend the existing "D2 long tags on phones" test ([tests/bookmarks.spec.js:886-913](tests/bookmarks.spec.js#L886-L913)) with a long unhyphenated domain. Assert that no overflow occurs and that the `.bookmark-domain` right edge is within `clientWidth`.
- **Suggested fix:** Add `overflow-wrap: anywhere;` (and `min-width: 0`) to `.bookmark-domain`.

### D2. The Delete button's focus ring uses the error colour, not the accent colour

- **Requirement:** A.6 "Every element you can focus shows a visible focus ring **in the accent colour**."
- **Location:** [style.css:391-393](style.css#L391-L393), `.delete-button:focus-visible { outline-color: var(--error); }`
- **Impact:** Small. The ring is still visible. But it is the only control whose ring isn't the accent colour, which breaks the stated rule. This may have been a deliberate choice that never made it into the design. See Q4.
- **Reproduced:** Focus a Delete button by keyboard (Shift+Tab then Tab). `getComputedStyle(document.activeElement).outlineColor` is `rgb(185, 28, 28)` in light mode, while `--accent` is `#2563eb`.
- **Test to add:** The existing focus-ring test ([tests/bookmarks.spec.js:1544-1560](tests/bookmarks.spec.js#L1544-L1560)) checks only that `outlineStyle` isn't `none`, so it can't catch this. Tab through every focusable control and assert that `outlineColor` equals the resolved `--accent`, in both colour schemes.
- **Suggested fix:** Remove the rule, or change A.6 to allow the exception.

### T1. The Edit test says it checks position but never does

- **Requirement:** §2.3 "Saving updates it instead of creating a new one". §2.2 "newest first".
- **Location:** [tests/bookmarks.spec.js:285-313](tests/bookmarks.spec.js#L285-L313). The test is named "updates in place, keeping id, createdAt **and position**". It only asserts the count (`toHaveCount(2)`), never the order.
- **Impact:** Suppose a change made editing re-sort the list, for example sorting by an `updatedAt` or moving the edited item to the top. This test would still pass, while its name says position is covered.
- **Test fix:** After the reload, assert `page.locator(".bookmark .bookmark-title")` has the text `[EXAMPLE.title, "MDN Renamed"]`.

### T2. A test name describes behaviour the test doesn't exercise

- **Location:** [tests/bookmarks.spec.js:315-343](tests/bookmarks.spec.js#L315-L343), "rejects invalid input, and a later submit creates a new bookmark".
- **Impact:** The "later submit" in this test saves the *edit* ("Changed"). The new bookmark comes from a separate `addBookmark` call after the form has been reopened. A reader would think the test proves that a submit after a rejected edit creates a new bookmark, which would actually be a bug. Nothing is wrong with the behaviour. The name is misleading.
- **Fix:** Rename it to something like "a rejected edit keeps the old values; fixing it saves the edit, then the form returns to add mode". Also assert that the edited bookmark's title is `"Changed"` in storage.

---

## Risks

### R1. One malformed stored record blanks the whole app (High)

- **Requirement touched:** §4 Data model and "On page load: read and parse it. If nothing is there, use an empty array." The code handles *missing* data and invalid JSON. It doesn't handle a valid array that contains a malformed record.
- **Location:** [app.js:31-38](app.js#L31-L38) checks only `Array.isArray`. Rendering then assumes every field exists: `bookmark.tags.length` ([app.js:417](app.js#L417)), `b.tags` ([464](app.js#L464)), `text.toLowerCase()` on `notes` ([477-478](app.js#L477-L478)), `b.createdAt.localeCompare` ([509](app.js#L509)).
- **Impact:** The first `render()` throws, so the list is empty, *no* empty-state message shows, and nothing can be found. Adding still saves, but it throws again on render. The data is still in storage but invisible, and fixing it needs DevTools. Data like this can come from an earlier build, a hand edit, or the planned JSON import (§7, "the first thing to add"). Also, `javascript:` URLs are only blocked at form entry ([app.js:63](app.js#L63)). A stored `javascript:alert(1)` is rendered as a clickable `href`, which becomes a real XSS path once import exists.
- **Reproduced:**
  - `localStorage.bookmarks = '[{"id":"1","url":"https://a.com","title":"A","createdAt":"2026-10-01T00:00:00.000Z"}]'` then reload. Result: `TypeError: Cannot read properties of undefined (reading 'length')`, 0 items, no empty message.
  - `[<valid bookmark>, null]` then reload. Result: `TypeError: Cannot read properties of null (reading 'tags')`.
  - A stored `url: "javascript:alert(1)"` renders `href="javascript:alert(1)"`.
- **Test to add:** Seed `localStorage` before load with (a) invalid JSON, (b) a record missing `tags`/`notes`, (c) a `null` entry, (d) a `javascript:` URL. Assert no `pageerror`, that valid records render, and that no `href` starts with `javascript:`. The suite has no seeded-storage tests today. The only one clears storage ([tests/bookmarks.spec.js:685](tests/bookmarks.spec.js#L685)).
- **Suggested fix:** Normalise in `loadBookmarks()`. Drop non-objects and records without a string `id`/`url`/`title`. Default `notes` to `""`, `tags` to `[]` and `createdAt` to now. Drop URLs that fail `validateUrl`.

### R2. Two open tabs overwrite each other's bookmarks (High, data loss)

- **Requirement touched:** §4 "After every add, edit, or delete: write the whole array back." §5 lists known limitations but doesn't mention this one.
- **Location:** [app.js:24](app.js#L24) reads storage once at load. [app.js:40-42](app.js#L40-L42) writes the in-memory array over whatever is stored. Nothing listens for the `storage` event.
- **Impact:** Each tab saves its own copy of the array over whatever the other tab wrote, so the last tab to save wins. Bookmarks added, edited or deleted in the other tab are silently lost.
- **Reproduced:** Open tabs A and B. Add "FromTabA" in A, then "FromTabB" in B. Storage then holds only `["FromTabB"]`.
- **Test to add:** In one browser context, open two pages, add a bookmark in each, and assert that storage contains both.
- **Suggested fix:** Listen for `window.addEventListener("storage", …)` and reload `bookmarks` and re-render. At minimum, re-read storage just before each write. If this is accepted as out of scope, add it to §5 Known Limitations.

### R3. A failed `localStorage` write is silent and leaves memory out of sync (Medium)

- **Location:** [app.js:40-42](app.js#L40-L42) has no `try/catch`. [app.js:125-136](app.js#L125-L136) changes the in-memory array *before* the write.
- **Impact:** When the quota is full or storage is blocked (some privacy modes, managed browsers), `setItem` throws. The submit handler stops after `bookmarks.push`. The form stays open, no error appears, and the list isn't redrawn. Submitting again pushes a second copy into memory, so if a later write succeeds, duplicates are saved. The same failure in `deleteBookmark` removes the item from memory but not from storage, so it comes back on reload.
- **Reproduced:** Stub `Storage.prototype.setItem` to throw `QuotaExceededError`, then add a bookmark. Result: an uncaught error, the form still open, `#form-error` hidden, 0 items listed. A second submit throws again and has pushed a second entry.
- **Test to add:** The same stub, then assert that a user-visible error appears in `#form-error` and that nothing changes in memory (a later successful save produces exactly one new record).
- **Suggested fix:** Write first and change memory after (or roll back on failure). Catch the error and show "Couldn't save. Your browser storage may be full or disabled." through `showError`.

### R4. Typing a search with 3,000 bookmarks takes 130–160ms per keystroke (Medium)

- **Requirement touched:** §5 "Fine for hundreds or even a few thousand bookmarks". §2.4 "Results update as I type". §1 Goal "Find any saved link again quickly".
- **Location:** Every `input` event calls `render()` ([app.js:530](app.js#L530)), which rebuilds every list item from the template, re-attaches listeners, and calls `formatDate` per item ([app.js:53-59](app.js#L53-L59), [498-524](app.js#L498-L524)).
- **Measured:** 3,000 bookmarks in desktop Chromium on this Mac took **130–158ms per keystroke** when the query matches all items. That is past the ~100ms point where typing feels laggy, and a phone will be several times slower. `toLocaleDateString` is about **84ms** of that, because it builds a new formatter on each call. A shared `Intl.DateTimeFormat` does the same 3,000 dates in **3ms**.
- **Test to add:** A performance check that seeds 3,000 bookmarks and asserts a single `input` dispatch renders in less than a budget (for example 50ms on CI). Or at least log the time so slowdowns are visible.
- **Suggested fix:** Create one `Intl.DateTimeFormat` at module scope. Append items through a `DocumentFragment`. If that's not enough, debounce the search briefly or cache the rendered `<li>` per bookmark id.

### R5. The "Saved" status may not be announced by screen readers (Low)

- **Requirement touched:** A.1 "a short 'Saved' message (`role="status"`) appears … so the message confirms the save."
- **Location:** [index.html:50](index.html#L50) has the live region `hidden` until it's used. [app.js:146-154](app.js#L146-L154) fills it and un-hides it in the same tick, then hides it again after 3s.
- **Impact:** Some screen reader and browser pairs only announce changes to a live region that was already in the accessibility tree. A region that becomes visible with its text already set is often missed. A second save within 3s sets the same text again (a DOM mutation was observed, but the text didn't change), which some screen readers also skip. Focus moves to the toggle at the same moment ([app.js:140](app.js#L140)), which can also interrupt the announcement. This needs a manual check. Automated tests can only show that the DOM changed.
- **Manual test:** VoiceOver + Safari and NVDA + Chrome/Firefox. Type a search, add a bookmark that doesn't match, and listen for "Saved". Repeat twice within 3 seconds.
- **Suggested fix:** Keep the region always in the DOM and visible (an empty, zero-height region is fine). Clear its text, then set "Saved" on the next frame. Hide it by clearing the text rather than with `hidden`.

### R6. Some tests depend on timing (Low)

- **Location:**
  - [tests/bookmarks.spec.js:484-496](tests/bookmarks.spec.js#L484-L496) assumes the toggle click lands "well inside the 450ms close".
  - [tests/bookmarks.spec.js:739](tests/bookmarks.spec.js#L739) and [745](tests/bookmarks.spec.js#L745) use `waitForTimeout(5)` so `createdAt` values differ.
  - [tests/bookmarks.spec.js:880](tests/bookmarks.spec.js#L880) uses a fixed 600ms wait.
- **Impact:** Everything passes locally, but on a slow or busy CI machine these can fail at random, or pass without testing what they claim. For example, if the close has already finished, the "still closing" test runs the ordinary reopen path.
- **Suggested fix:** Before the second click, check that the form still has an inline `height` style, so the test is truly mid-animation. Use `page.clock` to control `Date.now()` in `seedThree` instead of sleeping.

### R7. Gaps in test coverage (Low)

These requirements have no test:

- A.1: the "Saved" message when a **tag filter** (not a search) is active, and that it goes away after about 3 seconds.
- A.2: interrupting the drawer reverses it from where it is.
- A.4: tooltips (`title`) on the Edit and Delete buttons.
- R1–R3 above: seeded or corrupted storage, two tabs, and failed writes.
- D2 above: the colour of the focus ring.

---

## Questions for the owner

**Q1. Should notes keep their line breaks?** The Notes field is a 3-row `<textarea>` ([index.html:30](index.html#L30)), which invites multi-line notes. They are shown in a `<p>` with the default `white-space` ([app.js:412-414](app.js#L412-L414)), so "line one⏎line two" displays as "line one line two" (reproduced). If line breaks should be kept, add `white-space: pre-line` to `.bookmark-notes`.

**Q2. Should Edit silently discard unsaved form input?** If you type a new bookmark in the open form and then press Edit on an existing bookmark, the draft is replaced with no warning (reproduced). The same happens when you switch from one edit to another. A.1 asks for the switch but doesn't mention unsaved input. Is losing the draft acceptable?

**Q3. Should A.2 describe the list animation as built?** A.2 says list items "fade in briefly when the list is drawn". The code fades in only bookmarks that weren't in the previous render, and nothing on first page load ([app.js:27-28](app.js#L27-L28), [514-519](app.js#L514-L519)). The tests (O7) treat this as intended. If it is, update A.2 so the design matches.

**Q4. Is the red focus ring on Delete intentional?** If it is, change A.6 to allow it and close D2.

**Q5. How should international domain names display?** `getDomain` returns `URL.hostname`, so an international domain such as `https://пример.com` shows as `xn--e1afmkfd.com` ([app.js:372-378](app.js#L372-L378)). Rare for personal use, but if it matters, show the URL as the user typed it, or decode the punycode.

---

## How this was checked

- Read the design, all three app files and the full spec file. Ran `npx playwright test` on all three browsers: 282 passed.
- Ran one-off Playwright scripts (not added to the repo) against `npx serve` on port 3000 for: malformed and `null` stored records, a stored `javascript:` URL, two tabs, a `setItem` that throws, long domains at 320 and 375px in all three engines, multi-line notes, focus-ring colour, render time with 3,000 bookmarks, scrolling during the Edit drawer animation (no problem found), the "Saved" region mutations, use from `file://` (works), and a draft being discarded by Edit.
- These were checked and found to match the design: §2.1–§2.6, §3 layout and style, A.1 toggle and form states (heading, button names, close paths, focus on open), A.2 reduced motion in CSS and JS, A.3 dark mode, A.4 domain label and icon-button names, A.5 validation and error wiring, A.6 44px targets and hidden search label, A.7 tooling.

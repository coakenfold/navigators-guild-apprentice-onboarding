# Review 5: Bookmark Manager against DESIGN.md

**Date:** 2026-11-12
**Scope:** `index.html`, `app.js`, `style.css`, `tests/bookmarks.spec.js`, checked against `DESIGN.md` (sections 1–7 and the 2026-11-05 addendum, which takes priority where the two differ).
**Not read:** `REVIEW.md`, `REVIEW-2.md`, `REVIEW-3.md`, `REVIEW-4.md` (as instructed). Some findings may repeat earlier reviews.

## How this was checked

1. I read the design and every line of the three app files, and looked at test names and selected tests in the spec.
2. I ran the full suite: `npx playwright test --project=chromium`. **All 98 tests pass.** I did not run Firefox or WebKit.
3. I checked each suspected defect in Chromium with a throwaway Playwright script against `npm run start` (not added to the repo). Every item under "Confirmed defects" was reproduced, and the observed result is given. Items under "Risks" either depend on data or conditions the app does not create by itself, or are behaviour the design doesn't cover.

Severity: **High** = data loss or a core feature broken; **Medium** = a requirement visibly fails in a realistic case; **Low** = an edge case or a small mismatch with the spec.

## Summary

| # | Finding | Severity | Requirement |
|---|---|---|---|
| D1 | A failed storage write throws an uncaught error with no message shown, and saving again creates a duplicate | Medium | §4 Data model, A.5 |
| D2 | Stored records that the loader rejects are erased from storage on the next save | Medium | §4 "On page load", §5 |
| D3 | A long domain label causes sideways scrolling on phones | Medium | §3 Phones, A.4 |
| D4 | Tags loaded from storage are not normalised, which gives duplicate and empty chips | Low | §2.1 tag rules, §2.5 |
| D5 | When a submit has two problems, only one field is flagged | Low | A.5 |
| D6 | The Delete button's focus ring is red, not the accent colour | Low | A.6 |

---

## Confirmed defects

### D1. A failed storage write throws an uncaught error, shows no message, and saving again creates a duplicate (Medium)

**Requirement:** §4 Data model: "After every add, edit, or delete: write the whole array back." A.5: "Errors appear in a message area under the fields (`role="alert"`)."

**Code:** `saveBookmarks()` at [app.js:64-66](app.js#L64-L66) calls `localStorage.setItem` without a `try/catch`. In `handleSubmit` ([app.js:149-160](app.js#L149-L160)) the in-memory array is changed *before* the save, so when the save throws, the array has already changed. `deleteBookmark` ([app.js:381-382](app.js#L381-L382)) has the same problem.

**Impact:** If the storage quota is full, or storage is blocked (for example in some private-browsing modes), Save appears to do nothing. The form stays open, no error appears, and the list doesn't update. The bookmark exists only in memory. If the user presses Save again, a second copy is pushed. Both copies are written as soon as a later save succeeds. A delete that fails the same way disappears from the list but comes back on reload.

**Reproduce (observed):** Stub `Storage.prototype.setItem` to throw `QuotaExceededError` on the first call only. Open the form, enter a valid URL and title, and press Save twice.
- After the first press: `pageerror: quota`, the form is still open, `#form-error` is empty, and the list has 0 items.
- After the second press: `localStorage.bookmarks` holds two bookmarks titled `X`.

**Suggested test:** Use `page.addInitScript` to make `setItem` throw. Assert that `#form-error` shows a storage message, that the form stays open, that `bookmarks` in memory is unchanged (only one item after the retry succeeds), and that no `pageerror` occurs. Suggested fix: build the new array, try to save it, and only replace `bookmarks` if the save succeeds. Otherwise call `showError(...)`.

### D2. Stored records that the loader rejects are erased from storage on the next save (Medium)

**Requirement:** §4: "On page load: read and parse it." §5 Known Limitations names only "Clearing browser data deletes the bookmarks" as a way to lose data. §7 makes JSON import "the first thing to add", which will put data from outside the app into storage.

**Code:** `loadBookmarks()` → `normaliseRecord()` at [app.js:34](app.js#L34) and [app.js:41-47](app.js#L41-L47) drops any record whose `id` is not a string or whose URL fails `validateUrl` (which includes a URL containing a space). The next `saveBookmarks()` ([app.js:65](app.js#L65)) writes only the records that were kept, so the dropped ones are gone for good, without any warning.

**Impact:** Quiet, permanent data loss. Two realistic triggers:
- an `id` stored as a number (the data model's own comment says "timestamp works fine", and `1730000000000` is a timestamp);
- a URL saved by an earlier build that didn't reject spaces.

Hiding a broken record is a sensible way to protect the app. Deleting it from storage is a separate effect that the design doesn't call for.

**Reproduce (observed):** Seed `localStorage.bookmarks` with three records: `{id: 1730000000000, …}`, `{id: "2", url: "https://example.com/a b", …}` and one valid record `{id: "3", …}`. Reload. Only "ok" is listed. Delete it. `localStorage.bookmarks` is now `[]`, and the other two records are gone too.

**Suggested test:** Seed one valid record and one record with a numeric id, add a new bookmark, then assert that the raw stored JSON still contains the numeric-id record. Fix options:
- keep the rejected raw records and write them back with the rest;
- repair records that can be repaired (e.g. `String(id)`);
- back up the original value (e.g. under `bookmarks.backup`) before the first save that drops anything.

### D3. A long domain label causes sideways scrolling on phones (Medium)

**Requirement:** §3 Phones: "the layout should stack and still work on a narrow screen. No sideways scrolling." A.4 adds the domain label.

**Code:** `.bookmark-domain` ([style.css:301-305](style.css#L301-L305)) has no `overflow-wrap: anywhere`, but the title, URL and notes all do ([style.css:294](style.css#L294), [316](style.css#L316), [321](style.css#L321)). The label is a flex item inside `.bookmark-heading` ([style.css:282-288](style.css#L282-L288)), so its minimum width is the whole hostname. The `overflow-wrap: break-word` it inherits from `body` doesn't lower that minimum.

**Impact:** Any hostname wider than about 315px at 0.8rem with no hyphen to break on pushes the page wider than the screen. The whole page then scrolls sideways at 375px.

**Reproduce (observed):** At a 375×640 viewport, seed one bookmark with the URL `https://engineeringdocumentation.internalwikiplatform.example.com/` (57-character host). `document.documentElement.scrollWidth` is **408** against a `clientWidth` of 375. Hostnames that contain hyphens wrap and don't overflow. A 63-character single-label host gives a scrollWidth of 516.

**Suggested test:** Add a long-hostname case to the existing "D2 long tags on phones" / "nothing scrolls sideways" tests, asserting `scrollWidth <= clientWidth` at 320 and 375px. Fix: add `overflow-wrap: anywhere` (or `min-width: 0` plus `anywhere`) to `.bookmark-domain`.

### D4. Tags loaded from storage are not normalised (Low)

**Requirement:** §2.1: "Tags are trimmed, lowercased, and de-duplicated, so `Recipes` and `recipes ` count as the same tag." §2.5: "A list of all tags in use."

**Code:** `normaliseRecord` keeps any string tag as-is ([app.js:54-56](app.js#L54-L56)). Only the form path calls `parseTags` ([app.js:146](app.js#L146)). An empty-string tag also gets `data-tag=""`, the same value the "All" chip uses ([app.js:471](app.js#L471)). As a result, `focusedTagSelector` ([app.js:507-513](app.js#L507-L513)) can't tell the two apart when it restores focus.

**Impact:** Data from a hand edit, an older build, or the planned import (§7) shows `Recipes`, `recipes` and `recipes ` as three separate chips, plus a blank chip. Each chip filters to a different subset, so the "same tag" rule fails.

**Reproduce (observed):** Seed `tags: ["Recipes", "recipes ", ""]` on one record and `["recipes"]` on another. The filter row shows `All`, `""`, `Recipes`, `recipes`, `recipes `.

**Suggested test:** Seed tags that aren't normalised and assert the chip names are exactly `["All", "recipes"]`. Fix: in `normaliseRecord`, run the tags through the same trim/lowercase/dedupe/non-empty steps as `parseTags`. For example, `parseTags(record.tags.filter(isString).join(","))` would work, except for tags that contain commas.

### D5. When a submit has two problems, only one field is flagged (Low)

**Requirement:** A.5: "The field with the problem is marked `aria-invalid` and linked to the message, and it receives focus."

**Code:** [app.js:129-140](app.js#L129-L140) returns at the first problem. If both fields are empty, only the URL field gets `aria-invalid`. If the URL is invalid and the title is empty, the message only says "Please fill in both URL and Title." The URL is not flagged, even though it is also wrong.

**Impact:**
- A screen-reader user who fixes the flagged field and submits again then gets a second, different error.
- In the both-empty case, Title is missing but announced as valid.
- In the `ftp://` + empty title case, the message suggests the URL is fine.

**Reproduce (observed):**
- Open the form and press Save with both fields empty: `url` has `aria-invalid="true"`, `title` has `null`.
- Enter `ftp://x` with an empty title and press Save: the message is "Please fill in both URL and Title.", `url` has `null`, `title` has `"true"`.

**Suggested test:** Extend the "Form errors" block with both-empty and bad-URL + empty-title cases, asserting that every problem field has `aria-invalid` and `aria-describedby="form-error"`, and that focus goes to the first one. Fix: collect all field errors, mark each field, and focus the first.

### D6. The Delete button's focus ring is red, not the accent colour (Low; may be intentional)

**Requirement:** A.6: "Every element you can focus shows a visible focus ring in the accent colour."

**Code:** [style.css:391-393](style.css#L391-L393) sets `.delete-button:focus-visible { outline-color: var(--error); }`.

**Impact:** A small mismatch with the spec. The red ring is visible and may be a deliberate choice to match the red button. If so, the spec should say that. If not, remove the rule.

**Reproduce (observed):** Focus the Edit button and press Tab. The computed `outline-color` of `.delete-button` is `rgb(185, 28, 28)` (`--error`), while `--accent` is `#2563eb`.

**Suggested test:** The existing "a control focused by keyboard has a visible outline" test could assert `outline-color` equals `--accent` for each control type, including Delete.

---

## Risks

These are confirmed behaviours or likely failures that the design doesn't explicitly rule out. Each one needs a decision rather than an obvious fix.

### R1. Two open tabs overwrite each other's saves (data loss)

**Requirement:** §4 "write the whole array back"; §5 assumes one browser. The design doesn't mention multiple tabs.
**Code:** `bookmarks` is read once at startup ([app.js:24](app.js#L24)), and every save writes the whole array ([app.js:65](app.js#L65)). Nothing listens for the `storage` event.
**Impact:** With the app open in two tabs, each save in one tab erases bookmarks added, edited or deleted in the other since it loaded. For a personal app that often stays open in a pinned tab, this is the most likely way to lose data.
**Reproduce (observed):** Open two tabs, add "From A" in tab A, then add "From B" in tab B. `localStorage.bookmarks` contains only `["From B"]`.
**Suggested test / fix:** Use two pages in one context, add one bookmark in each, and assert that both are stored. Fix: listen for `window.addEventListener("storage", …)` and reload `bookmarks` and re-render, or re-read storage just before each change.

### R2. Ids can collide, and colliding ids make delete remove more than one bookmark

**Requirement:** §4 data model: "unique id (timestamp works fine)."
**Code:** `id: Date.now().toString()` ([app.js:154](app.js#L154)). Edit uses `find` (the first match only, [app.js:149](app.js#L149)) and delete uses `filter` (every match, [app.js:381](app.js#L381)). `normaliseRecord` doesn't check for duplicates.
**Impact:** Two adds in the same millisecond can't happen by hand, but they can from a script or a future bulk import, and duplicate ids can also arrive in imported data. Deleting one bookmark then quietly deletes its twin, and editing the second one changes the first.
**Reproduce (observed):** Seed two records with `id: "dup"`, then delete the top one. The list is empty and both are gone.
**Suggested test / fix:** Seed duplicate ids and assert that delete removes exactly one. Fix: use `crypto.randomUUID()` (or check for collisions), and give duplicates new ids on load.

### R3. `createdAt` values that aren't ISO are accepted on load but sort wrongly

**Requirement:** §2.2 "newest first"; the data model uses ISO strings.
**Code:** `normaliseRecord` accepts any string `Date` can parse ([app.js:57-60](app.js#L57-L60)), but `renderAll` sorts with `localeCompare` on the raw string ([app.js:532-534](app.js#L532-L534)). An unparsable `createdAt` is replaced with *now*, and that replacement is saved on the next write, so the bookmark's real date is lost.
**Impact:** Imported or hand-edited dates show in the wrong order. Records with a broken date jump to the top and get a new date.
**Reproduce (observed):** Seed `createdAt: "2026-10-27T14:30:00.000Z"` and `createdAt: "1 Nov 2026"`. The list shows Oct 27 above Nov 1.
**Suggested test / fix:** Assert the order for mixed formats. Fix: store `new Date(record.createdAt).toISOString()` in `normaliseRecord`, or sort by `Date.parse`.

### R4. The "Saved" message may not be announced reliably

**Requirement:** A.1: a short "Saved" message (`role="status"`) confirms the save.
**Code:** `#save-status` starts `hidden` ([index.html:50](index.html#L50)). `showSaved` sets the text and un-hides it in the same step ([app.js:170-178](app.js#L170-L178)). Some screen-reader and browser combinations ignore live regions that only enter the accessibility tree at the moment their content changes. A second save within 3 seconds sets the same text "Saved" again, which often isn't re-announced.
**Impact:** The message exists for people who can't see that the bookmark was saved, and it may be silent for exactly the screen-reader users who need it.
**Suggested test:** A manual check with VoiceOver (Safari) and NVDA (Firefox/Chrome). Save with a search active, then save twice in quick succession. Safer pattern: keep the region in the tree (visually empty rather than `hidden`), and clear the text before setting it again.

### R5. Multi-line notes are shown on one line

**Requirement:** §2.1 Notes are "free text"; the §3 mock-up shows a two-line Notes field, and the form uses a 3-row `<textarea>` ([index.html:30](index.html#L30)).
**Code:** `.bookmark-notes` ([style.css:319-322](style.css#L319-L322)) uses the default `white-space: normal`.
**Impact:** Line breaks the user typed are kept in storage but collapsed in the list.
**Reproduce (observed):** Seed `notes: "line one\nline two"`. The rendered paragraph is a single 24px line.
**Fix if wanted:** `white-space: pre-line`.

### R6. `form.url` / `form.title` rely on a little-known browser rule

**Requirement:** §1 goal: "Keep the code small enough that I understand every line."
**Code:** [app.js:124-125](app.js#L124-L125), [132](app.js#L132), [145-146](app.js#L145-L146), [366-369](app.js#L366-L369). On any other element, `form.title` means the `title` attribute. It returns the input here only because `HTMLFormElement` lets named controls override built-in properties.
**Impact:** It works today, but it is easy to misread, and renaming or removing the `name` attributes would break it without any error. Pointing the code at `form.elements.title` or `document.getElementById("title")` would make the intent clear.

---

## Questions

1. **Delete focus ring (D6):** Is the red ring intentional? If so, update A.6. If not, drop [style.css:391-393](style.css#L391-L393).
2. **Escape outside the form:** A.1 says Escape closes the form "inside the form". With the form open and focus on the toggle, Escape does nothing (observed). Is that the intended behaviour?
3. **Notes formatting (R5):** Should line breaks in notes be kept in the list?
4. **"Saved" scope:** The code shows "Saved" after edits as well as adds when a filter is active ([app.js:167](app.js#L167)). A.1's wording ("The new bookmark may be hidden") reads as add-only, but "when a save succeeds" covers both. Please confirm that both are intended.
5. **Known gaps:** The addendum's "Known gaps" points to `REVIEW.md` instead of listing the gaps. Should the design list its open issues itself, so it stays accurate when reviews are replaced?

---

## Test coverage gaps

The suite is thorough for the main flows, and all 98 Chromium tests pass. Nothing in the spec covers:

- the domain label (`.bookmark-domain`, `www.` stripping) or long hostnames on phones (D3);
- storage write failures (D1), records being dropped on load and then saved (D2), or non-normalised stored tags (D4);
- combined validation errors (D5) and the focus-ring colour per control (D6);
- two tabs (R1), duplicate ids (R2), or mixed `createdAt` formats (R3);
- "Saved" when only a **tag filter** is active, and the message disappearing after about 3 seconds;
- the Edit/Delete tooltips (`title`), and `aria-controls` on the toggle;
- dark mode beyond the input border contrast check.

`playwright.config.js` also defines Firefox and WebKit projects. I only ran Chromium. The A.2 animation and the `inert` behaviour would benefit from a WebKit run.

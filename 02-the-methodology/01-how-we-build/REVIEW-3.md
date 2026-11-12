# Review 3: Bookmark Manager against DESIGN.md

Date: 2026-11-11
Scope: `index.html`, `app.js`, `style.css`, `tests/bookmarks.spec.js`, checked against `DESIGN.md` (sections 2–4 and the 2026-11-05 addendum, A.1–A.7).
Earlier reviews (`REVIEW.md`, `REVIEW-2.md`) were not read, so some findings may repeat them.

## How this was checked

1. Read the design and all of the app source.
2. Ran the full Playwright suite (`npx playwright test`). **276 of 276 passed** in Chromium, Firefox and WebKit.
3. Wrote a separate Playwright script that drives the running app (http://localhost:3000, Chromium) through the cases below. Every finding marked **Reproduced** was seen in that run. Findings marked **From code** were worked out by reading the code and were not run.

## Summary

The app meets the main requirements in sections 2.1–2.6 and the addendum. The test suite is thorough and passes everywhere. All of the problems found are at the edges: what happens when `localStorage` fails or holds data the app did not write, and two small differences from the addendum.

| ID | Type | Severity | Finding |
|----|------|----------|---------|
| D1 | Defect | Medium | A failed `localStorage` write fails silently, and the screen stops matching what is saved |
| D2 | Defect | Low | The Delete button's focus ring is red, but A.6 says the accent colour |
| R1 | Risk | Medium | Two open tabs overwrite each other's bookmarks |
| R2 | Risk | Medium | One badly shaped stored bookmark blanks the whole app |
| R3 | Risk | Low | Editing a stored bookmark that has no `notes` saves the text "undefined" |
| R4 | Risk | Low | Stored URLs go into `href` without being checked again |
| Q1 | Question | — | With both fields empty, should both be marked invalid? |
| Q2 | Question | — | Should a new bookmark be visible after saving while a search or filter is active? |
| Q3 | Question | — | Should Escape check for an open autocomplete list or IME input first? |

---

## Confirmed defects

### D1. A failed `localStorage` write fails silently, and the screen stops matching what is saved

- **Requirement:** §4 Data model, "After every add, edit, or delete: write the whole array back." A.5, "Errors appear in a message area under the fields (`role="alert"`)."
- **Code:** [app.js:38-40](app.js#L38-L40) `saveBookmarks()` has no `try/catch`. In [app.js:123-137](app.js#L123-L137) `handleSubmit` changes the in-memory array *before* calling `saveBookmarks()`, so when the save throws, `resetForm()`, `setFormOpen(false)` and `render()` never run. The same order is used for delete in [app.js:342-343](app.js#L342-L343).
- **What happens (Reproduced):** With `setItem` made to throw `QuotaExceededError` and a bookmark saved:
  - the form stays open, no error message appears, and the list does not change;
  - the only sign is an uncaught exception in the console;
  - the bookmark *is* in memory, so it shows up on the next render (typing in search made it appear), even though it was never saved and is gone after a refresh.
- **Also, from code:** pressing Save again pushes a second copy with a new id. Deleting while storage fails takes the item out of memory but leaves its row on screen until the next render, and it comes back after a refresh.
- **Impact:** The user can believe a bookmark is saved when it is not. This happens when storage is full, in some private browsing modes, or when site data is blocked. The design's main promise is that saved links survive a refresh, and here that silently fails.
- **Fix:** Wrap `setItem` in `try/catch` and return success or failure. Change the array only after a successful write, or roll it back on failure. Show the failure through `showError()`.
- **Test:**
  ```js
  await page.evaluate(() => {
    Storage.prototype.setItem = () => { throw new DOMException('full', 'QuotaExceededError'); };
  });
  // fill and submit a valid bookmark
  await expect(page.locator('#form-error')).toBeVisible();
  await page.fill('#search', 'x'); await page.fill('#search', '');
  await expect(page.locator('.bookmark')).toHaveCount(0); // nothing that wasn't saved
  ```

### D2. The Delete button's focus ring is red, but A.6 says the accent colour

- **Requirement:** A.6, "Every element you can focus shows a visible focus ring in the accent colour."
- **Code:** [style.css:386-388](style.css#L386-L388) sets `.delete-button:focus-visible { outline-color: var(--error); }`.
- **What happens (Reproduced):** With keyboard focus on Delete, the computed `outline-color` is `rgb(185, 28, 28)` (`--error`). `--accent` is `#2563eb`.
- **Impact:** This is a small visual difference. The ring is still visible and has good contrast. It may well be a deliberate choice so the ring matches the red button. If so, the addendum should say so, so that the spec and the code agree.
- **Fix:** Remove the rule, or add "Delete uses the error colour" to A.6.
- **Test:** Focus `.delete-button` with Tab, then check that `getComputedStyle(el).outlineColor` equals the computed `--accent` value. Do this in both colour schemes.

---

## Risks

These do not break a written requirement, but they can cause data loss or a broken page in realistic use.

### R1. Two open tabs overwrite each other's bookmarks

- **Requirement:** §1 Goals ("Find any saved link again"). §5 Known Limitations does not mention more than one tab.
- **Code:** [app.js:22](app.js#L22) reads storage once at startup. [app.js:38-40](app.js#L38-L40) writes the whole array back. There is no `storage` event listener.
- **What happens (Reproduced):** Open the app in tabs A and B. Add "From tab A" in A, then add "From tab B" in B. `localStorage.bookmarks` then holds only `["From tab B"]`, because tab A's bookmark was overwritten.
- **Impact:** Bookmarks are lost without any warning. This is easy to trigger, for example by leaving one tab open and opening another later.
- **Fix:** Listen for `window.addEventListener('storage', …)` and reload plus re-render when the `bookmarks` key changes. At the very least, re-read storage before each write and merge by `id`. Or list this as a known limitation in §5.
- **Test:** Use two pages in one context, as in the steps above, and expect both titles in storage.

### R2. One badly shaped stored bookmark blanks the whole app

- **Requirement:** §4, "On page load: read and parse it. If nothing is there, use an empty array."
- **Code:** [app.js:29-36](app.js#L29-L36) only checks `Array.isArray`. The render code then assumes every field exists: `bookmark.tags.length` ([app.js:402](app.js#L402)), `b.tags` ([app.js:449](app.js#L449)), `text.toLowerCase()` ([app.js:463](app.js#L463)), `b.createdAt.localeCompare` ([app.js:494](app.js#L494)).
- **What happens (Reproduced):**
  - With one item missing `tags`: `TypeError: Cannot read properties of undefined (reading 'length')`. Nothing is listed, not even the good item, and no empty-state message shows.
  - With a `null` item: the same blank page.
  - With an item missing `notes`: it loads, but typing any search throws (`reading 'toLowerCase'`) and the list goes blank.
- **Impact:** The app is unusable until the user clears site data, which deletes every bookmark. The likely triggers are data from an earlier build slice, a hand edit in DevTools, or a future import feature (§7 lists import as "the first thing to add").
- **Fix:** Clean each item on load: drop anything that isn't an object, and give missing fields defaults (`notes: ''`, `tags: []`, and `createdAt` from `id`). Building this now also prepares for import.
- **Test:** Seed storage with `addInitScript`, using items that lack `tags` / `notes` / `createdAt` plus a `null` item. Expect no `pageerror`, expect the good items to be listed, and expect search to work.

### R3. Editing a stored bookmark that has no `notes` saves the text "undefined"

- **Code:** [app.js:329](app.js#L329), `form.notes.value = bookmark.notes;`. Setting `.value = undefined` writes the string `"undefined"`.
- **What happens (Reproduced):** Seed a bookmark without `notes`, press Edit, and the Notes box shows `undefined`. Press Save changes, and the stored `notes` is now `"undefined"`, which is then shown in the list.
- **Impact:** Silent data corruption, caused by the same kind of stored data as R2. The R2 fix also fixes this. Otherwise use `bookmark.notes ?? ''` (and the same for `tags`).
- **Test:** As in R2, then Edit, then expect `#notes` to have the value `''`.

### R4. Stored URLs go into `href` without being checked again

- **Code:** [app.js:387](app.js#L387) `link.href = bookmark.url;`. The `http(s)` check happens only on form submit ([app.js:60-73](app.js#L60-L73)).
- **What happens (Reproduced):** A stored `url` of `javascript:alert(1)` is rendered as a working `javascript:` link.
- **Impact:** Low for a single-user app, because someone would need to write to this origin's storage first. It becomes real once import (§7) exists, because an imported file could contain such links.
- **Fix:** Run `validateUrl` on each item during the R2 load-time cleanup, or render the title as plain text when the scheme is not `http:`/`https:`.
- **Test:** Seed the item above and expect the title link to have no `href` with `javascript:`.

---

## Questions for the designer

### Q1. With both fields empty, should both be marked invalid?

- **Requirement:** A.5, "The field with the problem is marked `aria-invalid` and linked to the message."
- **Code:** [app.js:103-108](app.js#L103-L108). When both URL and Title are empty, the message is "Please fill in both URL and Title.", but only URL gets `aria-invalid` (Reproduced). The test at [tests/bookmarks.spec.js:1097](tests/bookmarks.spec.js#L1097) requires exactly one invalid field, so this looks deliberate.
- **Question:** The message names two fields but only one is flagged. A screen reader user who moves to Title will not hear that it is invalid. Is one field the intent, or should both be marked and the first focused?

### Q2. Should a new bookmark be visible after saving while a search or filter is active?

- **Requirement:** §1, "Save a link in a few seconds". §2.2 "newest first". §2.5 says search and filter combine, but not what happens on save.
- **What happens (Reproduced):** With the search "pasta" active, saving "Soup" stores it (2 items in storage), but the list still shows only "Pasta". The form closes and focus moves to the toggle. Nothing on screen confirms the save.
- **Question:** Should saving clear the search and filter, show a short "Saved" message, or keep things as they are? Whatever is chosen should be added to §2.4 or A.1.

### Q3. Should Escape check for an open autocomplete list or IME input first?

- **Requirement:** A.1, Escape inside the form closes it and clears it.
- **Code:** [app.js:302-304](app.js#L302-L304) acts on every `Escape` keydown in the form. It does not check `event.isComposing`, and it doesn't leave the key to the browser's autocomplete list first.
- **Question:** The behaviour matches the spec. But someone pressing Escape to dismiss URL autocomplete, or to cancel IME input (for example Japanese), also loses everything typed into the form, with no undo. That includes unsaved edits, which was checked: Escape after typing a note left the stored note empty. Should Escape ignore `isComposing`, or should Discard changes ask for confirmation when the form has changed?

---

## Gaps in the tests

The suite covers the features in the design well. It does not cover these areas, which line up with the findings above:

- `localStorage` failures (D1). No test makes `setItem` throw.
- Stored data the app did not write itself (R2, R3, R4). No test seeds storage with partial or unusual items.
- More than one tab (R1).
- Dark mode is only checked for input border contrast ([tests/bookmarks.spec.js:772](tests/bookmarks.spec.js#L772)). Focus rings, selected chips and error text are not checked in dark mode.
- The focus-ring colour on Delete (D2).

## Checked and found correct

These were checked, and they match the design: tag trimming, lowercasing and de-duplication; newest-first order; title links open in a new tab with `noopener`; the exact `confirm("Are you sure?")` text; search across title, URL and notes ignoring case, without tags; search and tag filter combined; the filter resetting when its tag disappears; both empty-state messages; drawer open and close, Escape, and switching edit mode; heading and button names in both modes; reduced motion in CSS and JS; the 44px tap targets and no sideways scrolling at 320px and 375px; long hostnames wrap instead of overflowing; domain shown without `www.`; and DOM building uses only `textContent` (no `innerHTML` with user data).

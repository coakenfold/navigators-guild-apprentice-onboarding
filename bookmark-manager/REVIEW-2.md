# Review 2: Bookmark Manager

**Date:** 2026-11-11
**Scope:** `index.html`, `app.js`, `style.css` and `tests/bookmarks.spec.js`, checked against `DESIGN.md`, including its addendum. The previous review (`REVIEW.md`) was not read.

**Method:** I read the code against each requirement, ran the Playwright suite, and confirmed each finding in Chromium with throwaway Playwright scripts that loaded the app from `npm run start`.

**Test suite:** `npx playwright test --project=chromium`: 91 passed. Firefox and WebKit were not run.

## Summary

| #   | Type    | Finding                                                    | Severity |
| --- | ------- | ---------------------------------------------------------- | -------- |
| D1  | Defect  | Long domain names cause sideways scrolling on phones       | Medium   |
| D2  | Defect  | Form fields stay focusable while the form closes; focus is lost | Medium   |
| D3  | Defect  | Delete button focus ring is red, not the accent colour     | Low      |
| R1  | Risk    | Two open tabs overwrite each other's data                  | High     |
| R2  | Risk    | A failed `localStorage` write is silent                    | Medium   |
| R3  | Risk    | Stored records are not checked when the page loads         | Medium   |
| R4  | Risk    | IDs from `Date.now()` can repeat; Delete removes every record with that id | Low      |
| R5  | Risk    | Adding while a filter is active gives no feedback          | Low      |
| Q1–Q4 | Question | Multi-line notes, list fade-in, Escape discards edits, empty-state announcement | — |

No test in the suite covers any of the defects or risks.

---

## Confirmed defects

### D1. Long domain names cause sideways scrolling on phones

- **Requirement:** §3 "Phones: … No sideways scrolling" and A.6.
- **Code:** [style.css:296-300](style.css#L296-L300). `.bookmark-domain` is a flex item inside `.bookmark-heading` and has no `overflow-wrap: anywhere`. A flex item can't shrink below its min-content width, so an unbroken hostname keeps its full length. The title, URL and notes lines all set `overflow-wrap: anywhere`; the domain label is the one that was missed.
- **Impact:** at 375px wide, a bookmark for `https://averyveryveryverylonghostnamewithoutanyhyphensatallforthistest.example.com/` makes the page 527px wide.
- **Reproduce:** set a 375×640 viewport, add that bookmark, then compare `document.documentElement.scrollWidth` (527) with `clientWidth` (375).
- **Test:** add a long-hostname case to the "D2 long tags on phones" tests and assert `scrollWidth <= clientWidth`.
- **Suggested fix:** add `overflow-wrap: anywhere; min-width: 0;` to `.bookmark-domain`.

### D2. Form fields stay focusable while the form closes, and focus is then lost

- **Requirement:** A.1 says the toggle uses `aria-expanded` so its state is announced. A.6 covers keyboard accessibility.
- **Code:** [app.js:281](app.js#L281) sets `aria-expanded="false"` as soon as the close starts. `form.hidden` is only set at the end of the 450ms animation ([app.js:195](app.js#L195)). Until then the fields stay in the tab order, even though the toggle reports the form as closed.
- **Impact:** if you press Tab right after closing, focus moves into the URL field. When the animation ends, that field is hidden and focus falls back to `<body>`. Keyboard and screen-reader users lose their place.
- **Reproduce:** open the form, press the toggle to close it, then press Tab straight away. `document.activeElement` is `#url`. After 600ms it is `BODY`.
- **Test:** do the same steps in Playwright and assert that focus never enters `#bookmark-form` while `aria-expanded="false"`.
- **Suggested fix:** set `form.inert = true` when a close animation starts, and clear it when the form opens.

### D3. Delete button focus ring is red, not the accent colour

- **Requirement:** A.6 says "Every element you can focus shows a visible focus ring in the accent colour."
- **Code:** [style.css:386-388](style.css#L386-L388) sets `.delete-button:focus-visible { outline-color: var(--error); }`.
- **Impact:** minor. The ring is still visible, but it doesn't match the written requirement. Either the CSS or the design needs to change.
- **Reproduce:** Tab to a Delete button. Its computed `outlineColor` is `rgb(185, 28, 28)`, while `--accent` is `#2563eb`.
- **Test:** add `.delete-button` to the "a control focused by keyboard has a visible outline" test and assert that its outline colour matches `--accent`. If the red ring is intended, update A.6 instead.

---

## Risks

### R1. Two open tabs overwrite each other's data

- **Requirement:** §4 "After every add, edit, or delete: write the whole array back."
- **Code:** [app.js:22](app.js#L22) reads storage only once, at startup. [app.js:38-40](app.js#L38-L40) writes the whole in-memory array back, and nothing listens for the `storage` event.
- **Impact:** bookmarks are lost without any warning. Having the app open in two tabs is an easy mistake to make.
- **Reproduce:** open the app in tab 1 and tab 2. Add "From tab 1" in tab 1, then add "From tab 2" in tab 2. `localStorage.bookmarks` now holds only "From tab 2".
- **Test:** use two pages in one Playwright browser context and follow the steps above. Assert that both titles are stored.
- **Suggested fix:** listen for `window`'s `storage` event and reload and re-render when `bookmarks` changes, or re-read storage before each write.

### R2. A failed `localStorage` write is silent

- **Requirement:** §4 Storage, and A.5, which says errors appear in the message area.
- **Code:** nothing catches an exception from `saveBookmarks()` ([app.js:38-40](app.js#L38-L40), called at [app.js:134](app.js#L134)). `setItem` can throw when storage is full or blocked.
- **Impact:** the bookmark has already been added to the array in memory, but the save throws before `render()`. The form stays open, no error is shown and the list doesn't change. After a reload the bookmark is gone. If you press Save again, a second copy goes into memory, and both are written by the next save that works.
- **Reproduce:** in the console, run `Storage.prototype.setItem = () => { throw new DOMException('quota', 'QuotaExceededError') }`, then add a bookmark. The page shows an uncaught error, no error message and no new list item.
- **Test:** stub `setItem` the same way in Playwright and assert that `#form-error` is visible and the array was not changed.
- **Suggested fix:** wrap the save in `try/catch`. On failure, undo the change in memory and call `showError("Couldn't save…")`.

### R3. Stored records are not checked when the page loads

- **Requirement:** §4 "On page load: read and parse it", and the data model. §7 makes import "the first thing to add", which will make malformed data more likely.
- **Code:** [app.js:29-36](app.js#L29-L36) only checks that the stored value is an array, not what each record contains.
- **Impact:**
  - **Missing `tags`:** the render crashes at [app.js:400](app.js#L400). The list is blank and the empty-state message is also hidden, so the panel shows nothing.
  - **Missing `notes`:** any search that doesn't match on the title or URL first throws at [app.js:461](app.js#L461) and blanks the list.
  - **Missing `notes`, then Edit:** the Notes field is filled with the text `undefined` ([app.js:327](app.js#L327)). Saving would store that text.
  - **Wider exposure:** the key name `bookmarks` is generic. Other projects served on `localhost:3000` share that storage, and in Chrome all `file://` pages share one storage area.
- **Reproduce:**
  - Seed `localStorage.bookmarks` with `[{"id":"1","url":"https://example.com","title":"X","notes":"","createdAt":"2026-10-01T00:00:00.000Z"}]` (no `tags`) and reload. The list is blank.
  - Seed a record without `notes`, then search for `zzz`. The console shows `Cannot read properties of undefined (reading 'toLowerCase')`.
- **Test:** seed a record without each optional field. Assert that it renders, that a search for `zzz` shows the no-match message, and that Edit leaves Notes empty.
- **Suggested fix:** fill in defaults for each record when loading (`notes: ""`, `tags: []`, and a `createdAt` if missing), and drop records without a `url` or `title`.

### R4. IDs from `Date.now()` can repeat; Delete removes every record with that id

- **Requirement:** §4 data model ("unique id (timestamp works fine)").
- **Code:** ids are made at [app.js:128](app.js#L128). Delete filters out every record with the matching id ([app.js:340](app.js#L340)). Edit updates only the first match ([app.js:123](app.js#L123)).
- **Impact:** deleting one of two records that share an id removes both. Through the UI this needs two saves in the same millisecond, so it is unlikely, but an import or a clock change could also cause it.
- **Reproduce:** seed two records with `"id": "2"` and delete one. Both are removed from storage. Alternatively, freeze `Date.now` and add two bookmarks: both get the same id.
- **Test:** do the seeded case in Playwright and assert that one record remains.
- **Suggested fix:** use `crypto.randomUUID()` for new ids.

### R5. Adding while a filter is active gives no feedback

- **Requirement:** §2.1 (Add) together with §2.4/§2.5 (search and the tag filter work together).
- **Code:** [app.js:134-138](app.js#L134-L138) saves, closes the form and re-renders with the current filter still applied.
- **Impact:** a new bookmark that doesn't match the current filter is saved, but the form closes and the list doesn't change. It looks as though nothing was saved, which invites a duplicate.
- **Reproduce:** select the `recipes` chip, then add an untagged bookmark. It is in storage but not on screen.
- **Test:** do those steps and assert that the new title is visible, or that a status message is shown.
- **Suggested fix:** clear the search and tag filter after an add, or show a short status message such as "Saved. Hidden by the current filter."

---

## Questions

**Q1. Should notes keep their line breaks?**
Notes are typed into a multi-line textarea (§2.1, "Free text"), but `.bookmark-notes` uses `white-space: normal` ([style.css:314-317](style.css#L314-L317)). As a result, "line one⏎line two" is shown on one line. If the line breaks should stay, use `white-space: pre-line`.

**Q2. Should the list fade in on the first page load?**
A.2 says "List items fade in briefly when the list is drawn." The code ([app.js:25-26](app.js#L25-L26) and [app.js:497-499](app.js#L497-L499)) and the O7 tests deliberately skip the first load and only fade in bookmarks entering the list. If that change is intended, A.2 should be updated to match.

**Q3. Should Escape discard unsaved edits without asking?**
Pressing Escape in the form, including the Notes textarea, closes the form and throws away what was typed, in edit mode too ([app.js:300-302](app.js#L300-L302)). A.1 allows this. Is that acceptable for a long note or a half-finished edit?

**Q4. Should the empty-state messages be announced to screen readers?**
`#empty-message` ([index.html:51](index.html#L51)) is not a live region. When typing a search empties the list, screen-reader users hear nothing. One fix is `role="status"`, but the text would then also need to change only when the state changes, so it isn't announced again on every keystroke.

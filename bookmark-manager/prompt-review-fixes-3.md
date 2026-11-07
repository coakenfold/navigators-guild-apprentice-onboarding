# Review fixes: keyboard focus, phone overflow, URL validation and field contrast

## Overview

Fix four findings from REVIEW.md using red/green tests. For each one, first write a Playwright test that reproduces the problem and watch it fail (red). Then fix the code and watch the test pass (green).

## Context

- `bookmark-manager/REVIEW.md`: findings **D1** (keyboard focus is lost), **D2** (a long tag causes sideways scrolling), **D3** (URL validation is too loose) and **R5** (light-mode input borders are below 3:1). Each finding lists its location and the steps that reproduce it.
- `bookmark-manager/DESIGN.md`: §2.1 (URL must start with `http://` or `https://`), §3 Phones ("No sideways scrolling"), and the Addendum.
- `bookmark-manager/CLAUDE.md`: how to run the dev server and the Playwright tests.
- `bookmark-manager/prompt-review-fixes-2.md` changes the edit-mode button labels. It may run before or after this prompt. Don't rely on the button texts "Update", "Save changes" or "Discard changes". Find the submit button as `#bookmark-form button[type="submit"]`.

## Goal

### Process (follow this order)

1. **Red.** Add the tests below to `tests/bookmarks.spec.js` in a new `test.describe('Review fixes', …)` block. Don't change any code yet. Run the new tests in all three browsers. Report which tests failed and confirm that each failed **for the reason the finding describes**, not because of a typo or a wrong selector. If a test passes before the fix, say so and explain why. One case is expected: the space-in-host case in D3 passes in Firefox and WebKit, because those engines already reject it.
2. **Green.** Fix the code, one finding at a time. Rerun the new tests after each fix.
3. **Regression check.** Run the full suite in Chromium, Firefox and WebKit. Don't change any existing test except where a fix genuinely changes what it checks, and explain any such change.

### D1. Keyboard focus after actions

Tests: use the keyboard (`focus()` followed by `press('Enter')`) and check `document.activeElement`. Write one test per case.

- **Tag chip:** after pressing a chip, focus is on the re-rendered chip for the same tag. That includes "All". Pressing Tab then moves to the next chip.
- **Tag on a bookmark:** after pressing a bookmark's tag, focus is on that same tag button on the same bookmark. That bookmark is always still in the list.
- **Delete (accept the dialog):** focus moves to the title link of the next bookmark in the list, or the previous one if the deleted bookmark was last. If the list is now empty, focus moves to the form toggle.
- **Delete (dismiss the dialog):** focus stays on the Delete button.
- **Save or update:** after a successful submit closes the form, focus is on the form toggle, as it already is after Cancel.

Fix: `render()` rebuilds the list and the chip row, so record what had focus before rendering and restore it afterwards. Identify buttons with data attributes (for example `data-tag` on chips, and `data-id` plus `data-tag` on bookmark tags) rather than holding element references. Keep the code small and readable, with no framework or helper library.

### D2. Long tags on phones

Tests: at viewports of 320×640 and 375×640, add a bookmark with the tag `averyveryverylongtagnamewithoutanyspacesatallforthetest`. Check that `document.documentElement.scrollWidth <= document.documentElement.clientWidth`. Also check that both the filter chip and the tag on the bookmark fit inside the viewport, using `getBoundingClientRect().right`.

Fix: in `style.css`, let tag pills wrap or shrink (for example `max-width: 100%` and `overflow-wrap: anywhere` on `.tag`). Tags must still look like pills, and normal tags must not change visually.

### D3. URL validation

Tests: each of these inputs shows the error message, saves nothing, and leaves `localStorage` unchanged, in all three browsers:

- `https:example.com`
- `http:/example.com`
- `https://exa mple.com`
- `ftp://example.com`, which is already covered and must keep passing.

Add positive cases too. `HTTPS://Example.com` and `https://example.com/path?q=1#x` must still save.

Fix: in `validateUrl`, before parsing, require the trimmed text to start with `http://` or `https://`, ignoring case, and to contain no whitespace. Keep using the `URL` parser for everything else. Remove the `!parsed.hostname` branch, which can never run, or keep it with a one-line comment saying why. Error messages must still explain what to type.

### R5. Input border contrast

Tests: with `page.emulateMedia({ colorScheme: 'light' })` and again with `'dark'`, read the computed `border-color` of the URL input, the computed `background-color` of the input, and the computed `background-color` of its `.panel`. Compute the WCAG contrast ratio in the test. Assert that the border is at least **3:1** against both backgrounds. Put the contrast function in the test file.

Fix: change only `--field-border` in the light theme in `style.css`. For reference, `#64748b` gives 4.76:1 on `#fff` and 4.40:1 on `#f5f6f8`. The dark theme already passes and should not change unless the test shows otherwise. Hover and focus borders must stay visible and distinct from the default border.

### Out of scope

The REVIEW.md risks R1–R4 and R6, the edit-mode labels (handled by `prompt-review-fixes-2.md`), and Escape behaviour. Don't change them.

### Documents

When everything is green:

- In REVIEW.md, mark D1, D2, D3 and R5 as fixed, with the date.
- Add a line to the latest Update section naming the new tests.
- In `PROCESS.md`, add a short entry for this fix with the red and green results: which tests failed first, and the final pass count.

## Done when

- The red run is reported, with each new test failing for its stated reason (allowing for the expected D3 exception).
- All the new tests pass in Chromium, Firefox and WebKit.
- The full existing suite still passes in all three browsers, with no console errors.
- Using only the keyboard, you can filter by a chip, clear the filter, delete a bookmark, and add one without focus ever landing on `<body>`.
- At 375px nothing scrolls sideways, even with a very long tag.
- The URL inputs listed in D3 are rejected and the valid URLs are accepted.
- Light-mode input borders measure at least 3:1, and dark mode is unchanged.
- REVIEW.md and PROCESS.md are updated.

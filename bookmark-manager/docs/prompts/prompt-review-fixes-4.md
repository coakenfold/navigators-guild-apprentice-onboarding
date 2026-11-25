# Review fixes: list animation, tag button names and test coverage

## Overview

Implement REVIEW.md optional improvements O7, O8 and O9 using red/green tests. For each one, first write a Playwright test that reproduces the problem or the missing coverage and watch it fail (red). Then change the code and watch the test pass (green). O1 and O2 were already delivered by `prompt-review-fixes-3.md`. This prompt only confirms they are in place.

## Context

- `bookmark-manager/REVIEW.md`: the "Optional improvements" section, items **O1, O2, O7, O8 and O9**.
- `bookmark-manager/prompt-review-fixes-3.md` must have been run first. It fixed D1 (focus after render, which is O1) and D2 (tag wrapping, which is O2), and added a `Review fixes` test block.
- `bookmark-manager/prompt-review-fixes-2.md` changes the edit-mode button labels and may run before or after this prompt. Don't rely on the texts "Update", "Save changes" or "Discard changes". Find the submit button as `#bookmark-form button[type="submit"]`.
- `bookmark-manager/DESIGN.md`: §2.5 (tag filter) and the Addendum, especially A.2 (motion) and A.6 (accessibility and phone layout).
- `bookmark-manager/CLAUDE.md`: how to run the dev server and the Playwright tests.

## Goal

### Process (follow this order)

0. **Prerequisite.** Check that the `Review fixes` block from `prompt-review-fixes-3.md` exists and passes, and that REVIEW.md marks D1 and D2 as fixed. If not, **stop** and report it. Don't rebuild O1 or O2 here.
1. **Red.** Add the new tests below to `tests/bookmarks.spec.js` and change no app code. Run them in all three browsers. Report which tests failed, and confirm each one failed for the stated reason, not because of a typo or a wrong selector.
2. **Coverage tests: red by temporary break.** Some O9 tests cover behaviour that already works, so they will pass straight away. For each of these, prove it can fail:
   - Make a small temporary change to `app.js` or `style.css` that breaks the behaviour the test covers.
   - Run the test and confirm it fails.
   - Undo the change.

   Report what you broke and which tests caught it. Leave no temporary change behind. Run `git diff` or compare the files to confirm.
3. **Green.** Change the code one item at a time, rerunning the new tests after each change.
4. **Regression check.** Run the full suite in Chromium, Firefox and WebKit. Don't change any existing test unless a change below genuinely alters what it checks, and explain any such change.

### O1 and O2: confirm only

Confirm that the fixes-3 tests for focus after render and for long tags at 320 and 375px pass. Don't add code for them.

### O7. Animate only items that newly appear

Today every bookmark replays the `fade-in` animation on every keystroke and filter click, because `render()` rebuilds the whole list. Keep the smooth transitions that slice 6 asked for, but only for items that **enter** the visible list.

Tests:

- **Search:** add two bookmarks and wait for any animation to finish. Then type one search character that keeps both bookmarks visible. Right after that keystroke, every `.bookmark` has `getAnimations().length === 0`. This fails today.
- **Narrowing a filter:** pressing a chip that narrows the list leaves the bookmarks that remain unanimated.
- **Widening a filter:** clearing the filter, or clearing a search, animates only the bookmarks that come back into view, not the ones that were already visible.
- **Adding:** a newly added bookmark does animate. The other bookmarks don't.
- **Deleting:** the remaining bookmarks don't animate.
- **First load:** no bookmark animates when the page first loads.
- **Reduced motion:** with `page.emulateMedia({ reducedMotion: 'reduce' })`, no bookmark ever animates.

Fix: `render()` remembers the ids that were visible last time. It starts as `null`, so nothing animates on first load. Only bookmarks that weren't visible last time get an `entering` class. In `style.css`, move the `animation` from `.bookmark` to `.bookmark.entering`. Keep it small and readable, with no framework or helper library.

### O8. Tag buttons on bookmarks

Tests:

- **Name and state:** a bookmark's tag button has the accessible name `Filter by recipes` and `aria-pressed="false"`. The visible text is still just `recipes`.
- **Selected:** after pressing it, the `recipes` button on **every** visible bookmark has `aria-pressed="true"` and the `selected` class. The other tag buttons stay `false`, and the `recipes` chip in the filter row is selected.
- **Pressed again:** every `recipes` button returns to `aria-pressed="false"`, and the "All" chip is selected.
- **Filter row unchanged:** the chips in the filter row keep their current names, such as `recipes` and `All`. Their group label already says "Filter by tag".

Fix: in `createBookmarkItem`, set `aria-label` to `Filter by <tag>`, and set `aria-pressed` and the `selected` class from `selectedTag`. The visible text must stay inside the accessible name (WCAG 2.5.3, Label in Name). Check that the fixes-3 focus tests still pass, because they also locate these buttons.

### O9. Test gaps

Each of these is new coverage. Use the red-by-temporary-break step above where a test passes straight away.

- **Tag filter**, in a new `test.describe('Tag filter', …)` block. Use the break step for each test:
  - A chip shows only the bookmarks with that tag.
  - Pressing the same chip again clears the filter.
  - "All" clears the filter.
  - Pressing a tag on a bookmark filters the list.
  - Chips are sorted A–Z and include "All".
  - The filter resets to "All" when the selected tag disappears, because its last bookmark was deleted or edited to remove the tag.
- **URL and form errors:**
  - Each error message appears with its exact text. The empty-field message is "Please fill in both URL and Title.", plus the invalid-URL and wrong-scheme messages.
  - The field with the problem gets `aria-invalid="true"` and `aria-describedby="form-error"`, and receives focus.
  - After a successful save, the error is hidden and both attributes are removed.
  - Don't duplicate the D3 URL cases from fixes-3.
- **Phone layout**, at 375×640:
  - The form is a single column, with each label above its field.
  - The search box sits above the chips and spans the panel's width.
  - The date in each bookmark header sits below the title.
  - Buttons, chips and inputs are at least 44px tall.
  - Nothing scrolls sideways with typical content.
- **Keyboard and focus rings:**
  - Pressing Enter on the toggle opens the form, sets `aria-expanded="true"` and focuses the URL field.
  - Escape closes the form, sets `aria-expanded="false"` and returns focus to the toggle.
  - A control focused by keyboard has a visible outline. For example, the computed `outline-style` of a Tab-focused button is not `none`.
- **Reduced motion:** with `reducedMotion: 'reduce'`:
  - Opening the form shows it at full height straight away, with no inline `height` style once the click returns.
  - The toggle has a single `.roll-text` span, with no leftover rolling text.
  - The `+` icon has no transition (computed `transition-duration` of `0s`).
- **Reliable `openForm` helper:** rewrite it to wait instead of checking `isVisible()` once.
  - Read the toggle's `aria-expanded` state and click only when it is `false`.
  - Wait until the drawer animation has finished, meaning the form has no inline `height` style.
  - Have `addBookmark` wait for the form to close after saving.

  To prove it's stable, run the full suite with `--repeat-each=5` in all three browsers and report the result.
- **CLAUDE.md:** update the line saying the spec "covers the slice 1 Done-when list" so it describes what the tests cover now.

### Out of scope

REVIEW.md O3–O6, the unnumbered "Stricter URL check" item (covered by fixes-3, D3), the risks R1–R4 and R6, the edit-mode labels (fixes-2) and Escape behaviour. Don't change them.

### Documents

When everything is green:

- In REVIEW.md, mark O7, O8 and O9 as done, with the date, and mark O1 and O2 as confirmed.
- Add a line to the latest Update section naming the new test blocks.
- In `PROCESS.md`, add a short entry for this work with the red and green results:
  - which tests failed first;
  - what each temporary break was and which tests caught it;
  - the final pass count;
  - the `--repeat-each` result.

## Done when

- The prerequisite check passed, so the fixes-3 tests exist and are green.
- The red run is reported. Every O7 and O8 test failed for its stated reason, and every O9 coverage test was shown to fail against a temporary break, which was then undone.
- All the new tests pass in Chromium, Firefox and WebKit.
- The full suite passes in all three browsers with no console errors, including with `--repeat-each=5`.
- Typing in search or pressing a chip no longer replays the fade on bookmarks that stay in view. New and revealed bookmarks still fade in, and nothing animates with reduced motion on.
- A screen reader announces a bookmark's tag as "Filter by <tag>" along with its pressed state.
- CLAUDE.md, REVIEW.md and PROCESS.md are updated.

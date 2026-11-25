# Review fix: clear names for the form in edit mode

## Overview

Make the toggle, the form and its buttons say what they do while a bookmark is being edited, so a screen-reader user hears the right thing. This addresses REVIEW.md Question 2.

## Context

- `bookmark-manager/REVIEW.md`: Question 2, "Accessible names in edit mode".
- `bookmark-manager/DESIGN.md`: Addendum A.1 (collapsible add form) and A.4 (bookmark display).
- `bookmark-manager/CLAUDE.md`: how to run the dev server and the Playwright tests.
- The current behaviour lives in `app.js`: `setFormOpen`, `resetForm`, `startEdit` and `rollToggleLabel`. The form is in `index.html`.

## Goal

The toggle button should always describe its action, and the form should carry the edit status. The only behaviour that changes is the text below.

- **Toggle:** reads "Add a bookmark" when the form is closed and "Close form" when it is open, in add mode and edit mode alike. Remove the "Editing bookmark" label. Keep `aria-expanded`, the rolling-label animation and its reduced-motion handling.
- **Form heading:** add a visible `<h2 id="form-heading">` as the first child of the form, spanning the full width of the grid. It reads "Add a bookmark" in add mode and "Edit bookmark: <title>" in edit mode, using the bookmark's title from when the edit started. Set it with `textContent`. Give the form `aria-labelledby="form-heading"` and remove its hard-coded `aria-label`.
- **Submit button:** "Save" in add mode and "Save changes" in edit mode. This replaces "Update".
- **Cancel button:** "Cancel" in add mode and "Discard changes" in edit mode.
- **Back to add mode:** every path that returns to add mode resets the heading and both buttons. That means saving, cancelling, Escape, the toggle, and deleting the bookmark being edited. Clicking Edit on a different bookmark updates the heading to that bookmark's title.
- **Out of scope:** what Escape does (Question 3), focus handling (D1), and any other REVIEW.md finding. Don't change them.
- **Style:** the heading should look like a quiet form title rather than a page section, and it must wrap long titles without sideways scrolling at 375px. Check that the drawer animation still fades the heading row in and out cleanly.
- **Documents:** update DESIGN.md Addendum A.1 and A.4 to describe the new labels. In REVIEW.md, mark Question 2 as resolved, the same way Question 1 is marked, and add a line to the Update section.

## Done when

- Opening the form shows the heading "Add a bookmark", the buttons "Save" and "Cancel", and a toggle named "Close form" with `aria-expanded="true"`.
- After pressing Edit on "MDN Web Docs":
  - `getByRole('form', { name: 'Edit bookmark: MDN Web Docs' })` finds the form.
  - The toggle is still named "Close form".
  - The buttons are "Save changes" and "Discard changes".
- After each of save, Cancel/Discard, Escape, the toggle, and deleting mid-edit, reopening the form shows add mode again.
- Clicking Edit on a second bookmark while editing changes the heading to the second title.
- New Playwright tests cover the points above. Use `exact: true` for button names, because "Save" also matches "Save changes".
- The existing Edit tests are updated from "Update" to "Save changes". Only change test logic where a label changed.
- All tests pass in Chromium, Firefox and WebKit, and there are no console errors.
- The form can be opened, edited and closed by keyboard only, with focus rings visible.
- With reduced motion on, the labels switch with no animation.
- At 375px, a 120-character title in the heading doesn't cause sideways scrolling.

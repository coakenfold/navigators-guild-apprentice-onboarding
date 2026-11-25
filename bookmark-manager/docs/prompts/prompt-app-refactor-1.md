# Refactor plan: `app.js`

Goal: a pure refactor. No visible behavior change. The existing Playwright suite (`tests/bookmarks.spec.js`) is the safety net and must pass after every step.

## Current problems

1. **SVGs built in JS.** `createTrashIcon()` and `createEditIcon()` (app.js:341-392) are about 50 lines of `createElementNS` and `setAttribute` calls. They duplicate each other, and the markup is hard to read, edit or preview. Most of `createBookmarkItem` (app.js:394-475) is the same kind of imperative DOM building.
2. **Everything is global.** The file has top-level `const` and `let` declarations: `bookmarks`, `editingId`, `selectedTag`, `formOpen`, `formAnimation`, all the DOM refs, and about 30 functions. A classic `<script>` puts the `function` declarations on `window` and the top-level `let` and `const` in the shared global lexical scope. Other scripts can collide with them or overwrite them.
3. **Functions read and write shared state directly.** Examples: `render()` reads `bookmarks`, `selectedTag`, `searchInput` and `list`; `handleSubmit` mutates `bookmarks` and `editingId`. There are no parameters and no return values, so nothing can be unit-tested without faking globals and a full DOM.

## Target design

Do it in three phases. Each phase is shippable on its own, and the suite must be green before the next one starts.

### Phase 1: Move markup into HTML (concern 1)

- Add an inline SVG sprite to `index.html`: a hidden `<svg>` containing `<symbol id="icon-edit">` and `<symbol id="icon-trash">`. The path data is copied from app.js:356-360 and :385. Put `viewBox` on each symbol.
- Add `<template id="bookmark-template">` to `index.html` holding the full `<li class="bookmark">` structure:
  - header, heading, title link, domain and date
  - url line, notes `<p>`, tags container
  - actions with the edit and delete buttons, each containing `<svg width="18" height="18" aria-hidden="true" focusable="false"><use href="#icon-edit"/></svg>`
  - the stroke and fill attributes (`fill="none" stroke="currentColor" stroke-width="2"` and so on) on the `<svg>`, or in CSS on `.bookmark-actions svg`
- Add `<template id="tag-pill-template">` for the tag pill and the filter chip, or keep one tiny `createElement` helper for those buttons. Pick whichever is shorter.
- Rewrite `createBookmarkItem` to:
  1. `template.content.firstElementChild.cloneNode(true)`
  2. fill in values using `querySelector` and `textContent`, and set `href` and `data-*`
  3. remove the notes and tags nodes when empty
  4. set the `aria-label` and attach listeners
- User data still goes in only via `textContent` or attributes, never `innerHTML` (per CLAUDE.md).
- Delete `createTrashIcon` and `createEditIcon`.
- Check the icons: `currentColor`, hover and focus styles in `style.css`, and any selectors that target `svg` or `path`. A `<use>` shadow tree is not matched by `.delete-button path` selectors, so adjust them if needed.
- Tests: the selectors the suite relies on (`.bookmark`, `.delete-button`, `.edit-button`, `[data-tag]`, aria-labels) must not change.

### Phase 2: Contain the code in an IIFE (concern 2, minimum)

Quick win that ships before the larger restructure:

```js
(() => {
  "use strict";
  // ...entire current file...
})();
```

- Nothing leaks to `window`.
- Verify in the browser that `typeof bookmarks`, `typeof render` and similar all return `"undefined"`.
- Check that nothing, including the tests, depends on a global. Grep `tests/` for `evaluate` calls and `window.`.

### Phase 3: Split into modules with explicit state (concerns 2 and 3)

Replace the IIFE with ES modules, loaded via `<script type="module" src="app.js">`. The project has no build step and `npx serve` serves modules fine. `file://` will no longer work, but CLAUDE.md already mandates the dev server. The single IIFE from Phase 2 is the fallback if module loading is unwanted.

Suggested layout (flat, no build step):

| File | Contents | Dependencies |
|---|---|---|
| `lib/bookmarks.js` | **Pure logic, no DOM.** `parseTags`, `validateUrl`, `getDomain`, `formatDate`, `matchesSearch`, `filterBookmarks(bookmarks, {tag, query})`, `sortNewestFirst`, `collectTags`, `createBookmark(fields, {now, id})`, `updateBookmark`, `removeBookmark`. All return new values and never mutate their inputs. | none |
| `lib/storage.js` | `createStorage(backend = localStorage, key = "bookmarks")` returning `{ load, save }`. | injected backend |
| `lib/store.js` | `createStore(initialState)` returning `{ getState, setState, subscribe }`. It holds `bookmarks`, `editingId` and `selectedTag`. Optional: a plain closure is enough. | none |
| `lib/form-animation.js` | `animateForm`, `rollToggleLabel`, `easeInOutCubic`, `measureFormRows`. Takes elements as arguments and owns `formAnimation` in a closure via `createFormAnimator({form, toggle, toggleLabel})`. | injected elements |
| `lib/view.js` | `createView(root)` finds the DOM refs once and exposes `renderList`, `renderTagFilter`, `showError`, `clearError`, `setFormMode` and so on. Templates are cloned here. | `root` element |
| `app.js` | Composition root: builds storage, store, view and animator, wires event listeners, calls the first render. About 60 lines. | all of the above |

Rules that address concern 3:

- **No module-level mutable state.** State lives inside a `createX()` closure or in the store. Modules export functions and factories, never `let` bindings.
- **Dependencies are parameters.** Pure functions take data and return data. Anything that touches the DOM takes its elements, or a `root`, as an argument. Storage takes its backend. Time and ID generation (`Date.now`, `new Date()`) are injectable, which makes `createBookmark` deterministic.
- **One-way flow:** event → handler calls a pure function → `store.setState` → subscribed `render(state)`. `render` receives state as an argument and never reads globals.
- **`confirm()` is injected** into the delete handler so tests can stub it.
- **Event delegation** on `#bookmark-list` and `#tag-filter` (via `data-id` and `data-tag`) replaces per-element `addEventListener` calls and the closures over `bookmark`.
- The focus-restore logic (`focusedTagSelector`, `render`) moves into the view. It is easy to break, so keep it covered by tests.

### Phase 4 (optional): Unit tests for the pure logic

- With the logic out of the DOM, add fast tests for `lib/bookmarks.js` using `node --test` (no new dependency), with no globals to mock.
- Cover: `parseTags` dedupe and lowercase, each `validateUrl` branch, `matchesSearch` fields and case, `filterBookmarks` with tag and query, sort order, `createBookmark` with an injected clock and id, and `storage.load` with bad JSON.
- Keep Playwright for the integration behavior.

## Order of work

1. Run `npx playwright test --project=chromium` and record the baseline. Fix or note any pre-existing failures first.
2. Phase 1. Run the tests, then check the icons visually in light and dark and in the focus and hover states.
3. Phase 2. Run the tests and check the globals in the console.
4. Phase 3, in small steps. Extract pure functions first, then storage, then the view and animator, then the store and composition root. Run the tests after each extraction.
5. Phase 4 if wanted.
6. Update `CLAUDE.md` ("Files:" line and the module layout) and mention the module script.

## Acceptance checklist

- [ ] No `createElementNS` and no SVG path data in JS.
- [ ] `Object.keys(window)` shows no new app globals.
- [ ] No function in `lib/` reads a variable it did not receive as an argument or close over from its own factory.
- [ ] Playwright suite passes unchanged (chromium, then all projects).
- [ ] Manual check: add, edit, delete, tag filter, search, form open and close animation, reduced-motion, focus restoration after render, and Escape to cancel.
- [ ] No console errors on load.

## Risks and open questions

- **Modules need http(s).** Opening `index.html` directly from disk stops working in Phase 3. Is that acceptable? If not, stop at Phase 2.
- **`<use>` and CSS.** Styling inside the sprite is limited. Keep the stroke attributes on the outer `<svg>` and use `currentColor`.
- **Template cloning and IDs.** Don't put `id`s inside the templates, or they'll be duplicated per item.
- **Scope creep.** Don't change behavior, class names or the data shape (`localStorage` key `bookmarks`) during the refactor.

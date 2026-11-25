# Bookmark Manager

**A one-file bookmark manager for the links you want to find again.**

Bookmark Manager is a small web app for saving links with a title, a note on why you saved them, and tags. You can find them again by searching or by picking a tag. The whole app is one file, [`index.html`](index.html): plain HTML, CSS and JavaScript, with no framework and no build step. Bookmarks are saved in your browser's `localStorage`. There are no accounts and no server, and nothing is sent anywhere.

It's the guided first project of the guild's apprentice onboarding ([Your First Build](https://github.com/Navigators-Guild/apprentice-onboarding/blob/main/01-talking-to-agents/05-first-build.md)). So alongside the app, this folder keeps a record of how it was built: the design doc, a log of every layer, an adversarial review, and the triage of each finding.

## Contents

- [What it does](#what-it-does)
- [How it follows the guide](#how-it-follows-the-guide)
- [Run it](#run-it)
- [A quick tour](#a-quick-tour)
- [How it works](#how-it-works)
- [Keyboard reference](#keyboard-reference)
- [Testing](#testing)
- [Project files](#project-files)
- [Limits](#limits)
- [Further reading](#further-reading)

## What it does

- **Add** a bookmark with a URL and a title (both required), an optional note, and optional comma-separated tags. Tags are trimmed, lowercased and de-duplicated, so `Recipes` and `recipes ` are the same tag.
- **List** every bookmark, newest first. Each shows its title as a link that opens in a new tab, its domain, the full URL, the note, its tags and the date it was added.
- **Edit** a bookmark inline, in its own card, and save or discard the changes.
- **Delete** a bookmark, after an "Are you sure?" confirmation.
- **Filter** by tag. Click a tag chip above the list, or a tag on any bookmark, to show only bookmarks with that tag. Click it again, or click **All**, to clear the filter.
- **Search** as you type. Search matches the title, URL and notes, ignoring case, within the active tag filter. It doesn't match tags.
- **Persist** everything in `localStorage`, so bookmarks survive a refresh.

The add form starts collapsed behind an **Add a bookmark** button, and the page always uses a dark colour scheme.

## How it follows the guide

The guide's point is the workflow: design doc first, build in layers, verify each one, then an adversarial review. Each step has its record here.

| The guide asks for                                                                        | Where it is                                                                                                                           |
| ----------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| A design document, written before any code                                                | [DESIGN.md](DESIGN.md), with an Addendum for features added on purpose during the build                                               |
| A build in layers, each one verified                                                      | [PROCESS.md](PROCESS.md#build-process): what was asked, whether it worked first time, what was fixed, and what failed verification    |
| Plain HTML, CSS and JavaScript in a single `index.html`                                   | [index.html](index.html). A test checks that the page loads no separate stylesheet or script.                                         |
| URL validation with the `URL` API, `http:`/`https:` only, and an accessible error         | `validateUrl()` and `readFields()`, used by both the add form and the edit form                                                       |
| User text rendered with `textContent`, and new-tab links with `rel="noopener noreferrer"` | Throughout. No user data goes through `innerHTML`.                                                                                    |
| Keyboard use, narrow screens, visible focus, labels, contrast and reduced motion          | Covered by the Playwright suite ([Testing](#testing))                                                                                 |
| An adversarial review, and a `## Review 1` triage of every finding                        | [REVIEW.md](REVIEW.md), and [PROCESS.md](PROCESS.md#review-1): each finding marked valid, invalid or partly valid, with what was done |
| Known issues, stated honestly                                                             | [PROCESS.md](PROCESS.md#known-issues), including where the build still differs from the guide                                         |

## Run it

Open `index.html` in a browser. That's all: there's nothing to install. For example, double-click it, or on macOS:

```sh
open index.html
```

To have it reload as you edit, use VS Code's Live Server extension: right-click `index.html` and choose **Open with Live Server**. Or serve it with npm, which is what the tests use:

```sh
npm run start      # serves this folder at http://localhost:3000
```

`localStorage` is kept per origin. Bookmarks saved from `file://`, from Live Server (port 5500) and from port 3000 are three separate lists.

## A quick tour

1. **Add a bookmark.** Press **Add a bookmark**. The form slides open and the URL field gets focus. Enter `https://developer.mozilla.org`, the title `MDN Web Docs`, a note, and the tags `JavaScript, reference`, then press **Save**. The form closes and the bookmark appears at the top of the list, with the tags `javascript` and `reference`.
2. **Try a bad URL.** Enter `ftp://example.com`, or leave the title empty, and press **Save**. Nothing is saved. A message under the fields says what's wrong, such as "URL must start with http:// or https://", and the field with the problem is marked and focused.
3. **Filter by tag.** Click `reference` in the chips above the list, or on the bookmark itself. Only bookmarks with that tag stay. The active chip is highlighted, and screen readers hear it as pressed. Click **All** to clear the filter.
4. **Search.** Type in **Search bookmarks**. The list narrows on every keystroke, within the active tag. When nothing matches, it says "No bookmarks match your search."
5. **Edit.** Press the pencil on a bookmark. Its card becomes a form headed "Edit bookmark: MDN Web Docs", filled with its values. Change anything and press **Save changes**. Or press **Discard changes** or Escape to put it back as it was. An unsaved edit survives searching, filtering and deleting another bookmark.
6. **Delete.** Press the trash can and confirm. Focus moves to the next bookmark.
7. **Refresh.** Everything is still there.

If a search or filter hides a bookmark you just saved, a short "Saved" message confirms it was saved.

## How it works

### Data

Each bookmark is one object. All of them are kept in an array, saved as JSON under the `localStorage` key `bookmarks`:

```json
{
  "id": "1730000000000",
  "url": "https://developer.mozilla.org",
  "title": "MDN Web Docs",
  "notes": "Great reference for HTML and CSS.",
  "tags": ["javascript", "reference"],
  "createdAt": "2026-10-27T14:30:00.000Z"
}
```

- **On load,** the array is read and each record is checked by `normaliseRecord()`. A missing note or tags list gets a default. A record with no usable `id`, `url` or `title`, or with a URL that fails validation, is dropped. So one bad record can't blank the list, and a stored `javascript:` URL never becomes a link.
- **After every add, edit or delete,** the whole array is written back.

### Code

Everything is in `index.html`, in this order:

| Part                                | What it holds                                                                                                                                                          |
| ----------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `<style>`                           | All the CSS. Colours are custom properties on `:root`. Below 500px wide the layout stacks, and controls are at least 44px tall.                                        |
| The page markup                     | The add-form toggle, the add form, the search box, the tag chips and the list.                                                                                         |
| An SVG sprite and two `<template>`s | The pencil and trash icons, `#bookmark-template` (one bookmark's card) and `#edit-template` (the inline edit form). The script clones these rather than building HTML. |
| `<script>`                          | All the behaviour, in one strict-mode function (see below).                                                                                                            |

The main functions in the script:

- `loadBookmarks()` and `normaliseRecord()` read storage, and `saveBookmarks()` writes it.
- `readFields()` checks either form, using `validateUrl()` and `parseTags()`.
- `render()` rebuilds the list and the tag chips from the current search text and selected tag, then puts focus back on the tag button you pressed. `matchesSearch()` does the search.
- `startEdit()`, `handleEditSubmit()` and `cancelEdit()` run the inline edit.
- `deleteBookmark()` confirms, removes, saves and moves focus.
- `animateForm()` and `rollToggleLabel()` animate the add form and its button. Both skip the animation when the system asks for reduced motion.

## Keyboard reference

Every control can be reached with Tab and shows a focus ring.

| Key            | Where              | What it does                                                   |
| -------------- | ------------------ | -------------------------------------------------------------- |
| Enter or Space | **Add a bookmark** | Opens the add form and focuses URL. Pressed again, closes it.  |
| Enter          | In a form          | Saves it.                                                      |
| Escape         | In the add form    | Closes and clears it, and focuses **Add a bookmark**.          |
| Escape         | In an edit form    | Discards the changes, and focuses that bookmark's Edit button. |
| Enter or Space | A tag chip or tag  | Filters by that tag, or clears the filter. Focus stays on it.  |

## Testing

The tests are for development only. The app doesn't need them, or npm, to run.

They're Playwright end-to-end tests in [tests/bookmarks.spec.js](tests/bookmarks.spec.js), run in Chromium, Firefox and WebKit. Playwright starts `npm run start` itself, or reuses a server already running on port 3000. Set up once:

```sh
npm install
npx playwright install      # download the three browsers
```

Then:

```sh
npx playwright test                     # all 303 tests (101 in each browser)
npx playwright test --project=chromium  # one browser, for a quicker run
npx playwright test -g "Edit (inline)"  # one block
```

The suite covers adding and listing, edit and delete, live search, the tag filter, form errors and their ARIA attributes, keyboard use and focus rings, the phone layout, reduced motion, list animation, malformed stored data, the dark colour scheme, and the single-file rule. It also has a block for each fix from the adversarial review.

Before changing the code, read [CLAUDE.md](CLAUDE.md), which has the notes for agents. In particular, keep the app in one file, build DOM with `createElement` and `textContent`, and use the spec's `openForm` and `addBookmark` helpers, which wait for the form's animation to finish.

## Project files

```
bookmark-manager/
├── index.html            # the whole app
├── DESIGN.md             # the design doc, plus an Addendum for later changes
├── PROCESS.md            # the build log, review triage, learnings and known issues
├── REVIEW.md             # the adversarial review, with each finding's status
├── CLAUDE.md             # notes for agents working on this folder
├── docs/prompts/         # the prompt used for each slice, review and fix
├── tests/                # the Playwright suite
├── playwright.config.js
└── package.json          # `npm run start` and the test dependencies
```

## Limits

- **One browser, one device.** Bookmarks live in one browser's `localStorage`. Clearing site data deletes them, and there's no export or sync yet.
- **Two open tabs overwrite each other.** The last tab to save wins.
- **A failed save is silent.** If storage is full or blocked, the change isn't saved and no message says so.
- **Dark only.** There is no light mode, even when the system is set to light.
- **No screen reader testing yet.** The ARIA attributes, labels and focus moves are tested, but nothing has been checked with VoiceOver or NVDA.

[PROCESS.md § Known issues](PROCESS.md#known-issues) has the full list. That includes where the build still differs from the guide: the layer contents, search also matching the URL, the scope, the tooling, and some bundled commits.

## Further reading

- [DESIGN.md](DESIGN.md): what the app should do and look like, and the Addendum for changes made during the build.
- [PROCESS.md](PROCESS.md): the build layer by layer, Review 1's triage, the fixes for guide deviations, learnings and known issues.
- [REVIEW.md](REVIEW.md): the adversarial review, with each defect, risk, question and improvement, and its current status.
- [docs/prompts/](docs/prompts/): the prompt for each slice (`prompt-slice-1.md` to `prompt-slice-6.md`), for the review, and for each round of fixes.
- [Your First Build](https://github.com/Navigators-Guild/apprentice-onboarding/blob/main/01-talking-to-agents/05-first-build.md): the guild guide this project follows.

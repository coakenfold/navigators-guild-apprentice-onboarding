# Design Document: Personal Bookmark Manager

## 1. Overview

A small web app, just for me, for saving links I want to come back to. Each bookmark has a URL, a title, an optional note, and some tags. I can search and filter to find things again.

It runs in the browser and saves data on my own computer. It has no accounts, no login, and no server.

### Goals

- Save a link in a few seconds.
- Find any saved link again quickly by searching or picking a tag.
- Keep the code small enough that I understand every line.

### Non-goals (not in version 1)

- User accounts, login, or sharing
- Syncing between devices
- Browser extension or "bookmarklet"
- Automatically fetching page titles or previews
- Folders or nested tags

## 2. What It Should Do (Features)

### 2.1 Add a bookmark

- A form with these fields:
  - **URL** (required). It must be a valid link starting with `http://` or `https://`.
  - **Title** (required). This is the text shown in the list.
  - **Notes** (optional). Free text, such as why I saved it.
  - **Tags** (optional). Typed as a comma-separated list, e.g. `recipes, dinner`.
- Tags are trimmed, lowercased, and de-duplicated, so `Recipes` and `recipes ` count as the same tag.
- Each bookmark also stores the date it was created. The app adds this automatically.

### 2.2 View bookmarks

- All bookmarks are shown in a list, newest first.
- Each item shows the title (as a clickable link that opens in a new tab), the URL in smaller text, the notes, its tags, and the date added.

### 2.3 Edit and delete

- Each bookmark has **Edit** and **Delete** buttons.
- **Edit** fills the form with that bookmark's values. Saving updates it instead of creating a new one.
- **Delete** asks "Are you sure?" before removing the bookmark.

### 2.4 Search

- One search box next to the tags above the list.
- Results update as I type.
- It matches text in the title, URL, and notes. It ignores upper and lower case.

### 2.5 Filter by tag

- A list of all tags in use, each shown as a clickable "chip".
- Clicking a tag shows only the bookmarks with that tag. Clicking it again, or clicking "All", clears the filter.
- Clicking a tag on a bookmark does the same thing.
- Search and the tag filter work together. For example, I can search for "pasta" within the `recipes` tag.

### 2.6 Empty states

- With no bookmarks yet, show: "No bookmarks yet. Add your first one above."
- When a search finds nothing, show: "No bookmarks match your search."

## 3. What It Should Look Like

One page with a clean, simple layout:

```
+--------------------------------------------------------------+
|  My Bookmarks                                                |
+--------------------------------------------------------------+
|  Add a bookmark                                              |
|  URL    [ https://...                                     ]  |
|  Title  [                                                 ]  |
|  Notes  [                                                 ]  |
|         [                                                 ]  |
|  Tags   [ comma, separated                                ]  |
|                                              [ Save ]        |
+--------------------------------------------------------------+
|  Search [ 🔍 type to search... ]  Tags:  (All) (recipes)     |
+--------------------------------------------------------------+
|  MDN Web Docs                                 Oct 27, 2026   |
|  developer.mozilla.org                                       |
|  Great reference for HTML and CSS.                           |
|  (javascript) (reference)               [ Edit ] [ Delete ]  |
|--------------------------------------------------------------|
|  Easy Weeknight Pasta                         Oct 25, 2026   |
|  example.com/pasta                                           |
|  (recipes) (dinner)                     [ Edit ] [ Delete ]  |
+--------------------------------------------------------------+
```

### Style guidelines

- **Width:** content centered, max about 800px wide, with comfortable padding.
- **Font:** the system font stack (`system-ui, sans-serif`). No custom fonts needed.
- **Colors:** white or light grey background, dark text, and one accent color (e.g. blue) for links, buttons, and the selected tag.
- **Tags:** small rounded "pill" shapes. The selected tag uses the accent color.
- **Bookmarks:** each one in a light card or separated by a thin line.
- **Phones:** the layout should stack and still work on a narrow screen. No sideways scrolling.

## 4. Technology

The goal is the fewest moving parts: no frameworks, no build tools, nothing to install.

| Part       | Choice                             | Why                                                                                         |
| ---------- | ---------------------------------- | ------------------------------------------------------------------------------------------- |
| Structure  | **HTML**                           | The page and the form.                                                                      |
| Styling    | **CSS**                            | One stylesheet for the look.                                                                |
| Behavior   | **Plain JavaScript**               | Handles adding, editing, searching, and filtering. No framework to learn first.             |
| Storage    | **Browser `localStorage`**         | Saves data in the browser. It survives page refreshes, and no server or database is needed. |
| Running it | **Open `index.html` in a browser** | No setup required. Optionally use VS Code's "Live Server" extension for auto-reload.        |

### File structure

```
bookmark-manager/
├── index.html    # page layout and form
├── style.css     # all styling
└── app.js        # all behavior and data handling
```

### Data model

Each bookmark is a JavaScript object:

```js
{
  id: "1730000000000",          // unique id (timestamp works fine)
  url: "https://developer.mozilla.org",
  title: "MDN Web Docs",
  notes: "Great reference for HTML and CSS.",
  tags: ["javascript", "reference"],
  createdAt: "2026-10-27T14:30:00.000Z"
}
```

All bookmarks are kept in one array. It's saved to `localStorage` under the key `bookmarks` as JSON:

- **On page load:** read and parse it. If nothing is there, use an empty array.
- **After every add, edit, or delete:** write the whole array back.

### How the code fits together

1. `loadBookmarks()` reads from `localStorage`.
2. `saveBookmarks()` writes to `localStorage`.
3. `render()` clears the list and redraws it based on the current search text and selected tag. It also redraws the tag chips.
4. Event listeners call these functions: form submit, search input, tag click, edit, and delete.

## 5. Known Limitations

- Data lives in one browser on one computer. Clearing browser data deletes the bookmarks.
- Fine for hundreds or even a few thousand bookmarks, which is plenty for personal use.

## 6. Build Plan (step by step)

Each step should work before moving on to the next.

1. **Static page:** create `index.html` and `style.css` with the form and a hard-coded sample bookmark, to get the look right.
2. **Add & list:** in `app.js`, submitting the form adds a bookmark to the array and renders the list.
3. **Save data:** store bookmarks in `localStorage` so they survive a refresh.
4. **Delete:** add the Delete button with a confirmation.
5. **Edit:** add the Edit button, which reuses the form.
6. **Search:** add live search.
7. **Tags:** show tag chips and filtering.
8. **Polish:** empty-state messages, URL validation, and checking the layout on a phone-sized window.

## 7. Future Ideas (maybe later)

- Export and import bookmarks as a JSON file, as a backup. This is the first thing to add.
- Sort options (A–Z, oldest first)
- Dark mode
- Auto-fetching the page title from the URL

## Addendum: features added after this document (2026-11-05)

These features were added on purpose during the build (slice 6 and the follow-up polish in `PROCESS.md`), after this document was written. They are part of version 1. Where they differ from the sections above, this addendum wins.

### A.1 Collapsible add form (changes §3)

- The add form starts **collapsed**. A full-width toggle button with a round "+" icon sits where the form was, labelled "Add a bookmark".
- Activating the toggle opens the form and moves focus to the URL field. While the form is open, the "+" turns into an "×" and the label reads "Close form", in add mode and edit mode alike. The toggle always names its action.
- The form starts with a visible heading that carries the edit status: "Add a bookmark" in add mode, or "Edit bookmark: <title>" while editing, using the title the bookmark had when the edit started. The form takes its accessible name from this heading (`aria-labelledby`).
- The buttons read **Save** and **Cancel** in add mode, and **Save changes** and **Discard changes** in edit mode.
- The form closes after a successful save, and also when you press the toggle again, press **Cancel** / **Discard changes**, or press **Escape** inside the form. Deleting the bookmark being edited closes it too. Closing the form clears it and returns it to add mode; the heading and buttons switch back to their add-mode names once the drawer has closed.
- **Edit** on a bookmark opens the form if it is closed. Pressing Edit on another bookmark while editing switches the form, and its heading, to that bookmark.
- The toggle uses `aria-expanded` and `aria-controls`, so its state is announced.
- If a search or tag filter is active when a save succeeds, a short "Saved" message (`role="status"`) appears above the list for about 3 seconds. The new bookmark may be hidden by the search or filter, so the message confirms the save. The search and filter are left as they are. No message shows when neither is active.

### A.2 Motion

- **Drawer animation:** the form slides open and closed over 450ms with an ease-in-out curve. Each row of fields fades in as the drawer edge passes it. If you interrupt it, the animation reverses from where it is.
- **Rolling label:** the toggle text rolls between its states over 350ms, like an alarm-clock drum. The outgoing text is hidden from screen readers.
- **List items** fade in briefly when the list is drawn.
- **Reduced motion:** with `prefers-reduced-motion: reduce`, all of these animations are skipped, in both the CSS and the JavaScript.

### A.3 Dark mode (moved from §7 into version 1)

- The colours follow the operating system's `prefers-color-scheme` setting. All colours are CSS custom properties on `:root`, with a dark set: a dark navy background, light text and a lighter blue accent.
- There is no manual toggle. The page follows the system setting.

### A.4 Bookmark display (adds to §2.2)

- Each bookmark shows its **domain** (for example `developer.mozilla.org`, with `www.` removed) as a small muted label next to the title, as well as the full URL line.
- **Edit** and **Delete** are icon buttons: a pencil and a trash can. Their accessible names are "Edit <title>" and "Delete <title>", and they show tooltips.
- In edit mode the form's heading reads **Edit bookmark: <title>**, the submit button reads **Save changes** instead of Save, and the cancel button reads **Discard changes** instead of Cancel (see A.1).

### A.5 Validation and errors (adds to §2.1)

- URLs are checked with the browser's `URL` parser and must use `http:` or `https:`.
- Errors appear in a message area under the fields (`role="alert"`). The field with the problem is marked `aria-invalid` and linked to the message, and it receives focus.

### A.6 Accessibility and phone layout (adds to §3)

- Every element you can focus shows a visible focus ring in the accent colour.
- Below 500px wide, buttons, tag chips and fields are at least 44px tall, so they're easy to tap. The search box and the tag chips stack.
- The search box has a visually hidden label, "Search bookmarks".

### A.7 Developer tooling (adds to §4)

- The app still needs nothing installed to run. You can open `index.html` directly.
- For development only, `package.json` provides `npm run start`, which serves the app at http://localhost:3000. It also provides a Playwright test suite in `tests/`. Neither is needed to use the app.

### Known gaps

`REVIEW.md` lists open issues with these features, such as keyboard focus after actions and long tags on phones.

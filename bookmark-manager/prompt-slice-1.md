# Add & list (persisted)

## Overview

Save a bookmark and see it in the list, still there after a refresh

## Context

`bookmark-manager/DESIGN.md`

## Goal

Create index.html, style.css and app.js with the header, the add form (URL, Title, Notes textarea, Tags) and a bookmark list, following the layout in section 3. Submitting the form validates that URL and Title are filled in and that the URL starts with http:// or https://. It then creates a bookmark with a timestamp id and a createdAt date. Tags are split on commas, trimmed, lowercased and de-duplicated, and blank tags are dropped. The bookmark is saved to localStorage (bookmarks) and the form is cleared. render() shows bookmarks newest first: the title as a link opening in a new tab (rel="noopener"), the URL in smaller text, notes, tags as pills, and the date in the form 'Oct 27, 2026'. When there are no bookmarks, it shows 'No bookmarks yet. Add your first one above.' Add basic, clean styling, but no detailed polish yet.

## Done when

- On first load the page shows the empty-state message and no console errors.
- Adding https://developer.mozilla.org, "MDN Web Docs", tags JavaScript, reference , javascript shows one item with tags javascript and reference. The form clears.
- A second bookmark appears above the first.
- After a refresh, both are still there in the same order.
- Submitting ftp://x.com or an empty title shows an error and saves nothing.
- Clicking a title opens the link in a new tab.
- In DevTools, localStorage.bookmarks holds JSON that matches the data model.

# Edit

## Overview

Change an existing bookmark using the same form

## Context

`bookmark-manager/DESIGN.md`

## Goal

Add an Edit button to each bookmark. It fills the form with that bookmark's values (tags joined as a, b) and switches the form into edit mode: track the id being edited and show the button as 'Update'. Saving in edit mode replaces that bookmark in place, keeping its id and createdAt, and uses the same validation and tag normalization as adding. Afterwards the form returns to add mode and is cleared. If the bookmark being edited is deleted, the form also resets to add mode.

## Done when

- Editing a bookmark's title and tags updates it in place without creating a duplicate.
- Its position and date are unchanged, and the edit survives a refresh.
- Invalid input in edit mode is rejected.
- After saving, a new submit creates a new bookmark.
- Deleting a bookmark mid-edit, then submitting, creates a new bookmark rather than throwing an error.

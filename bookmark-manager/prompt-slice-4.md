# Tag filter

## Overview

Click a tag chip to show only bookmarks with that tag, combined with search

## Context

`bookmark-manager/DESIGN.md`

## Goal

Below the search box, render an 'All' chip followed by one chip for each distinct tag in use, sorted A–Z. render() rebuilds them every time. Clicking a chip sets it as the selected tag. Clicking it again, or clicking 'All', clears the selection. The selected chip uses the accent style. Clicking a tag pill on a bookmark does the same as clicking its chip. The list shows bookmarks that match both the search text and the selected tag. If the selected tag no longer exists after an edit or delete, clear the selection.

## Done when

- The chips match the tags in use.
- Clicking `recipes` shows only recipe bookmarks and highlights the chip. Clicking it again, or clicking All, shows everything.
- Clicking a tag on a bookmark filters the same way.
- Searching "pasta" with `recipes` selected shows only bookmarks that match both.
- Deleting the only bookmark tagged `travel` while `travel` is selected removes the chip and shows all bookmarks.

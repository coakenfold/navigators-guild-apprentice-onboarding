# Live search

## Overview

Narrow the list by typing

## Context

`bookmark-manager/DESIGN.md`

## Goal

Add a search bar _next_ to the tag filters. On every input event, render() shows only bookmarks where the search text appears (ignoring case) in the title, URL, notes. Search should not match against the tags. Search and tag filters should work together. If I have a tag filter active and then search, it should search within the filtered results. If bookmarks exist but none match, show 'No bookmarks match your search.' instead of the no-bookmarks message. An empty search shows everything.

## Done when

- Typing "mdn", "MOZILLA", "reference" (a tag) or a word from the notes each finds the MDN bookmark while you type.
- Typing "zzzz" shows the no-match message.
- Clearing the box shows everything again.
- With zero bookmarks, you still see the no-bookmarks message.

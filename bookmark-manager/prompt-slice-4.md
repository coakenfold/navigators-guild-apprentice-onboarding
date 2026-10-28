# Live search

## Overview

Narrow the list by typing

## Context

`bookmark-manager/DESIGN.md`

## Goal

Add a search box above the list. On every input event, render() shows only bookmarks where the search text appears (ignoring case) in the title, URL, notes or any tag. An empty search shows everything. If bookmarks exist but none match, show 'No bookmarks match your search.' instead of the no-bookmarks message.

## Done when

- Typing "mdn", "MOZILLA", "reference" (a tag) or a word from the notes each finds the MDN bookmark while you type.
- Typing "zzzz" shows the no-match message.
- Clearing the box shows everything again.
- With zero bookmarks, you still see the no-bookmarks message.

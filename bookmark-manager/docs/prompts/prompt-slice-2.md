# Delete

## Overview

Remove a bookmark after confirming

## Context

`bookmark-manager/DESIGN.md`

## Goal

Add a Delete button to each bookmark. Clicking it shows confirm('Are you sure?'). If you confirm, the bookmark is removed, the array is saved, and the list is re-rendered. If you cancel, nothing changes. Deleting the last bookmark shows the empty-state message again.

## Done when

- Cancelling the dialog leaves the bookmark in place.
- Confirming removes it, and it stays gone after a refresh.
- Deleting every bookmark brings back the empty-state message.

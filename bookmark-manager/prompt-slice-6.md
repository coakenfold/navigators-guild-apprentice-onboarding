# Style & phone polish

## Overview

Use a clean layout that matches the style guide and works on a phone

## Context

`bookmark-manager/DESIGN.md`

## Goal

Bring style.css in line with section 3's style guidelines: content centered with a max width of about 800px and comfortable padding; the system-ui, sans-serif font; a light background with one accent color for links, buttons and the selected chip; rounded pill tags; each bookmark in a light card or separated by thin lines. Add a narrow-screen layout where the form, the search area and the bookmark rows stack. Long URLs must wrap or truncate so there's never sideways scrolling. Don't change any behavior.

I want: better spacing and typography, a subtle color scheme (dark background with light text, dark mode), smooth transitions when filtering and searching, the add form should collapse to a button when not in use to keep the interface clean, and bookmarks should show the domain name extracted from the URL as a subtle label.

## Done when

- The page visually matches the section 3 mockup.
- In DevTools at 375px wide, nothing scrolls sideways, even with a very long URL. Buttons and chips are still tappable.
- All checks from slices 1–5 still pass.
- Can navigate by keyboard-only
- Focus rings remain visible
- Labels are announced
- Text has adequate contrast
- Reduced-motion preferences are respected
- Collapsed form can be opened an closed without a mouse

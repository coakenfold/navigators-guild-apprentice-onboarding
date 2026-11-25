# Bookmark Manager

Plain HTML/CSS/JS app (no framework, no build step). Spec: `DESIGN.md`. Build steps: `docs/prompts/prompt-slice-*.md`.
Files: `index.html`, `style.css`, `app.js`. Data lives in `localStorage` under the key `bookmarks`.

## Checking progress

Run everything from this directory (`bookmark-manager/`).

- **Dev server:** `npm run start` serves the app at http://localhost:3000/ (uses `npx serve .`). Check whether it is already running before starting another (`curl -s -o /dev/null -w "%{http_code}" http://localhost:3000/`). Run it in the background; stop it when finished.
- **Playwright tests:** `@playwright/test` is installed locally. Config is `playwright.config.js` (`baseURL` is http://localhost:3000 and `webServer` runs `npm run start`, reusing a server that is already up), specs are in `tests/`. Run `npx playwright test` (add `--project=chromium` for a quick run). `tests/bookmarks.spec.js` covers add, list, edit, delete, live search, the tag filter, form errors, the phone layout, keyboard and focus rings, reduced motion, list animation and the tag buttons. Use the `openForm` and `addBookmark` helpers at the top of the spec: they wait for the drawer to finish, so don't click the toggle by hand.
- **Playwright MCP:** the `@playwright/mcp` browser tools can drive the running app to check behaviour by hand (add a bookmark, refresh, inspect `localStorage.bookmarks`, check the console for errors). If the tools are not available, register with `claude mcp add playwright -- npx @playwright/mcp@latest` and restart the session. Running it directly in a terminal looks stalled; that is normal, it waits for an MCP client.
- Prefer verifying a slice's "Done when" list in the browser or with a test over assuming it works.

## Notes

- `localStorage` is per origin, so data saved on another port (e.g. Live Server on 5500) will not appear on port 3000. Tests should start from a clean browser context.
- Match the existing style in `app.js`: small functions, DOM built with `createElement` and `textContent` (never `innerHTML` with user data).

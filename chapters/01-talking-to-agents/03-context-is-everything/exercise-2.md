> Start a conversation with an agent. Give it only "add a dark mode toggle" and note which assumptions it makes. Then start a new session, provide a specific project's relevant context, and make the same request. Compare the assumptions, questions, and result.

---

## Without any context

The repo had no app code, only the exercise write-ups, so there was nothing to add a toggle to. The agent said so and listed what it would have had to guess.

### Assumptions

- **Stack:** React or plain HTML/CSS/JS, but the repo has neither.
- **Styling:** CSS variables with a `data-theme` attribute, or Tailwind's `dark:` classes, depending on what already exists.
- **Default theme:** follows `prefers-color-scheme` on first load.
- **Persistence:** the choice is saved in `localStorage`. It's unclear whether there's a backend or user settings to use instead.
- **Placement:** a button in a header or navbar, though it's unknown whether a header exists.
- **Control type:** a sun/moon icon button. A switch or a three-way light/dark/system choice are also possible.
- **Palette:** the dark colors would be invented, since no brand colors are defined.
- **Scope:** every page and component gets themed. Embedded content and images might need special handling.
- **Accessibility:** an aria-label, keyboard support and sufficient contrast.

### Questions

1. What's the project, and what does it use?
2. Where should the toggle live?
3. Should it follow the system setting or just be manual?
4. Is there an existing color palette?

### Result

No code and no files changed. The agent offered either to record these assumptions or to scaffold an app and add the toggle.

## With context

There was no code to change yet, because the project is still at the planning stage. The agent said so and gave a design, its assumptions and its questions instead.

### Assumptions

- **Plain web stack.** There's no backend and no framework is mentioned, so it would use HTML, CSS and vanilla JS with CSS custom properties for theming.
- **Saved on the device.** The preference goes in `localStorage`, next to the recipe data. There are no accounts, so nothing syncs.
- **Starts from the phone's setting.** On first visit the app follows `prefers-color-scheme`, and the user's manual choice overrides it after that.
- **Easy to tap.** The toggle is a button of at least 48px in the header, reachable with one thumb. It has a visible label (☀️/🌙 plus text) and an `aria-pressed` state.
- **Still bright and playful in dark mode.** The look is colorful and lively, so dark mode would not be grey. It would use deep navy or plum backgrounds with the same saturated accent colors, kept bright enough to read.
- **Large text stays readable.** Contrast should be at least WCAG AA in both themes, especially on the cooking view, where the recipe is read from a distance.
- **No flash on load.** A tiny inline script sets the theme before the page paints, so the screen doesn't flash white first.

### Questions

1. Three states (Light / Dark / Auto) or a simple two-state toggle?
2. Should the cooking view have its own always-dark or always-high-contrast option, for example for a dim kitchen?
3. Is there a framework or build tool in mind, or should it stay static files?
4. Are there brand colors yet? If not, pick a palette for both themes.

### Result

A design and a short list of questions, with no code yet. The agent offered to scaffold the app (header, theme CSS variables, toggle and persistence).

### Comparison

Without context, the agent had to guess the stack, the device, where to store the setting and the visual style. With context, it made specific choices (phone-first tap targets, `localStorage`, a playful dark palette, large readable text) and asked a few focused questions instead of broad ones.

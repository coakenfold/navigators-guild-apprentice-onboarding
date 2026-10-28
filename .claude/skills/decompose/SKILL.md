---
name: decompose
description: Break a large or ambiguous task into small, independent, dependency-ordered vertical slices that can each be built and verified in one focused agent conversation. Invoke explicitly with /decompose <task description>.
disable-model-invocation: true
---

# Decompose a task into vertical slices

Turn a big ask into a short, ordered list of slices. Each slice is a **vertical slice of user behavior** — something a person can see or do when it's finished — not a tiny code edit or a horizontal layer like "set up the database". Small, well-defined slices are easier to inspect, retry, and test; but over-fragmenting hides integration failures, so aim for coherent slices with observable outcomes, not the most slices. A typical feature lands at roughly 3–8 slices; well outside that range, re-check your sizing.

The task to decompose: `$ARGUMENTS` (if empty, use the task from the conversation; if there is none, ask for it).

This is the design phase. **Produce the plan and stop for review — do not start building.**

## Step 1: Decide whether it needs decomposing

Rule of thumb: if the task can be described **completely** in two or three sentences, it doesn't need decomposing. "Completely" is the test, not length: the description must say exactly what changes and what done looks like. Small, concrete asks ("add a footer with my name and the year", "change the background to dark blue", "fix the bug where save doesn't save") should be given whole.

If the task is that small and concrete, say so in one line, suggest doing it directly, and stop.

A short but vague ask ("improve the app", "make it faster") is **not** small — it's incomplete. Don't exit early; use the "what changes" approach in Step 3 to turn it into specific changes.

## Step 2: Gather just enough context

- If this is an existing codebase, look at what already exists (structure, existing features, how tests are run) so slices build on what's there and don't re-plan finished work.
- If a decision would change the shape of the core (who the user is, what "useful" means), ask one focused question. Otherwise make a sensible assumption and list it in the output.
- Collect the **shared context** every slice's agent will need: tech stack, where key code lives, conventions to follow, and the test command. For a new project, decide these (stack, storage) and record them as assumptions.
- If there is **no test suite**, note it. Slice 1 must then either set up a minimal test harness as part of its outcome, or every slice's "Done when" must list manual steps with expected results.

## Step 3: Pick the approach

Use whichever fits; combining them is fine.

**Layer approach (new projects).**
1. Find the **core**: the absolute simplest version that would still be useful. Strip every nice-to-have. For a recipe app the core is "display a list of recipes and add a new one with a name and ingredients" — not a searchable, categorized, photo-equipped database.
2. Add **layers** one at a time, most important first (e.g. categories + filter → search → edit/delete → photos → visual polish). Polish usually comes last.

**User story approach (behavior-heavy products).** Walk the user's journey and write each step as a one-sentence story in the user's voice:
- "I open the app and see today's habits."
- "I tap a habit to mark it complete."
- "I want to see how I did this week, so I tap 'weekly view' and see a grid."
Each story is a candidate slice.

**"What changes" approach (modifying an existing project, or any vague ask).** Replace vague goals ("improve the app") with specific, independent changes ("the add button is bigger", "the weekly view shows streak counts", "completed habits move to the bottom"). Each change is one slice, so if one causes a problem you know exactly which. If you can't name specific changes from the context, ask the user what's wrong today before planning.

## Step 4: Shape each slice

Every slice must have:
- **Outcome** — what the user can see or do afterward, in plain language.
- **Done when** — concrete, observable checks (a test, or manual steps with expected results).
- **Depends on** — the slices that must exist first (or "none").
- **Prompt** — an instruction that could be handed to an agent as-is: specific enough to describe in two or three sentences, naming the data, behavior, and constraints that matter. Don't repeat the shared context in each prompt; prompts assume it is provided alongside them.

Sizing check for each slice:
- **Too big** if the prompt needs a paragraph, it spans several user behaviors, or a failure wouldn't point to one cause → split it.
- **Too small** if it has no observable outcome on its own (e.g. "create the Task model") → merge it into the slice that first makes it visible.

## Step 5: Order by dependencies

For each slice ask: **"Does this require anything else to exist first?"** If yes, that thing comes earlier. Example:
- Display recipes → no dependencies, first
- Add recipes → needs display (otherwise you add into the void)
- Categories → needs recipes; filter by category → needs categories
- Search, edit/delete, photos → need existing recipes

Two slices can be marked **parallel** only if **both** hold:
1. Neither depends on the other, directly or through another slice.
2. They don't change the same parts of the code (same files, components, or data shape), so building them at once won't cause conflicts.

Sharing a dependency is not enough: categories and filter-by-category both depend on recipes, but filter also needs categories, so they are not parallel. When in doubt, leave slices sequential. No formal diagram needed; the table's "Depends on" and "Parallel with" columns are enough.

## Step 6: Critically review the plan before presenting it

- Does slice 1 alone deliver something useful?
- Is any slice too big, or so small it has no visible outcome?
- Any missed dependencies (e.g. filtering before the data has the field)?
- Are any "parallel" slices actually touching the same code?
- Is anything a nice-to-have that crept into the core?
- Is the slice count reasonable for the task's size?
- Where could slices fail to integrate? Note those seams.

Fix issues before showing the plan.

## Output format

```markdown
## Decomposition: <task name>

**Approach:** <layer / user story / what changes> — <one line on why>
**Core (smallest useful version):** <one sentence>
**Assumptions:** <bullets, or "none">

### Shared context (give this to every slice's agent)
- **Stack:** …
- **Where things live:** …
- **Conventions:** …
- **Test command:** <command, or "none yet — slice 1 sets it up" / "manual checks only">

| # | Slice | Outcome (user can…) | Depends on | Parallel with |
|---|-------|---------------------|------------|---------------|
| 1 | …     | …                   | —          | —             |

### Slice 1: <name>
- **Prompt:** "<instruction for the agent; assumes shared context is provided>"
- **Done when:** <observable checks>

### Slice 2: …

**Deferred / out of scope:** <nice-to-haves intentionally left out>
**Integration risks:** <seams worth an end-to-end check>

**Build loop:** build one slice → run its new checks **and** the existing suite → commit → next slice. A regression in slice 3 should surface before later slices depend on it.
```

## Finish: review, then offer to save

End by asking the user to review: does the order make sense, is any slice too big, are dependencies missing? Adjust the plan with them before any building starts.

Then ask whether to save the plan to a file (suggest `PLAN.md` at the project root, or a path they prefer). Each slice runs in its own conversation, so a saved plan lets later conversations read the shared context and the next slice. If they agree, write the plan with a **Status** column added to the table (`todo` / `in progress` / `done`), and tell them to update it as slices finish. Don't write the file without asking.

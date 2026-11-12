> Take one critique from the adversarial review and fix it. Then submit the fix for another round of critique. Do this three times. Notice how the quality of the critiques changes as the code improves.

## Round 1

### Prompt 1

Model: Sonnet 5.5 Medium

```
Implement `D2. Form fields stay focusable while the form closes, and focus is then lost`
```

### Prompt 2

Model: Opus 5.5 High

```
## Context

Design: `guild-portfolio/bookmark-manager/DESIGN.md`

App:
- `guild-portfolio/bookmark-manager/index.html`
- `guild-portfolio/bookmark-manager/app.js`
- `guild-portfolio/bookmark-manager/style.css`
- Tests: `guild-portfolio/bookmark-manager/tests`

## Goal

Review this code against the attached design and acceptance criteria. Report confirmed defects separately from risks and questions. For each finding, cite the relevant requirement and code location, explain impact, and suggest a way to reproduce or test it.

Save the review to `guild-portfolio/bookmark-manager/REVIEW-3.md`

## Important

There are previously created REVIEW documents. Ignore them and do not read.

- `guild-portfolio/bookmark-manager/REVIEW.md`
- `guild-portfolio/bookmark-manager/REVIEW-2.md`
```

---

## Round 2

### Prompt 1

Model: Sonnet 5.5 Medium

```
Read: `guild-portfolio/bookmark-manager/REVIEW-3.md`

Re: > Q2. Should a new bookmark be visible after saving while a search or filter is active?

Yes, show a short "Saved" message after saving while a search or filter is active
```

### Prompt 2

Model: Opus 5.5 High

```
## Context

Design: `guild-portfolio/bookmark-manager/DESIGN.md`

App:
- `guild-portfolio/bookmark-manager/index.html`
- `guild-portfolio/bookmark-manager/app.js`
- `guild-portfolio/bookmark-manager/style.css`
- Tests: `guild-portfolio/bookmark-manager/tests`

## Goal

Review this code against the attached design and acceptance criteria. Report confirmed defects separately from risks and questions. For each finding, cite the relevant requirement and code location, explain impact, and suggest a way to reproduce or test it.

Save the review to `guild-portfolio/bookmark-manager/REVIEW-4.md`

## Important

There are previously created REVIEW documents. Ignore them and do not read.

- `guild-portfolio/bookmark-manager/REVIEW.md`
- `guild-portfolio/bookmark-manager/REVIEW-2.md`
- `guild-portfolio/bookmark-manager/REVIEW-3.md`
```

## Round 3

### Prompt 1

Model: Sonnet 5.5 Medium

```
Read: `guild-portfolio/bookmark-manager/REVIEW-4.md`

Fix: `R1. One malformed stored record blanks the whole app`
```

### Prompt 2

Model: Opus 5.5 High

```
## Context

Design: `guild-portfolio/bookmark-manager/DESIGN.md`

App:
- `guild-portfolio/bookmark-manager/index.html`
- `guild-portfolio/bookmark-manager/app.js`
- `guild-portfolio/bookmark-manager/style.css`
- Tests: `guild-portfolio/bookmark-manager/tests`

## Goal

Review this code against the attached design and acceptance criteria. Report confirmed defects separately from risks and questions. For each finding, cite the relevant requirement and code location, explain impact, and suggest a way to reproduce or test it.

Save the review to `guild-portfolio/bookmark-manager/REVIEW-5.md`

## Important

There are previously created REVIEW documents. Ignore them and do not read.

- `guild-portfolio/bookmark-manager/REVIEW.md`
- `guild-portfolio/bookmark-manager/REVIEW-2.md`
- `guild-portfolio/bookmark-manager/REVIEW-3.md`
- `guild-portfolio/bookmark-manager/REVIEW-4.md`
```

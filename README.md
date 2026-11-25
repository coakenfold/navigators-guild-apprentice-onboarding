# guild-portfolio

**My apprentice portfolio for the Navigators Guild: projects built with coding agents, and the record of how each one was built.**

This repository holds my work through the guild's [apprentice onboarding](https://github.com/Navigators-Guild/apprentice-onboarding) curriculum. I build each project with a coding agent (Claude Code), using the guild's methodology:

1. Write a design doc first.
2. Build in small layers or slices.
3. Verify each one against what "done" means.
4. Put the result through adversarial review.

So each project folder holds the design, the build log, the reviews and how each finding was triaged, as well as the code. The process is part of the portfolio, not just the result.

## Projects

| Project                                  | What it is                                                                          | Built with                        | Start with                                   |
| ---------------------------------------- | ----------------------------------------------------------------------------------- | --------------------------------- | -------------------------------------------- |
| [bookmark-manager](bookmark-manager/)    | Portfolio project #1: a one-file web app for saving links with notes and tags       | Plain HTML, CSS and JavaScript    | [README](bookmark-manager/README.md)         |
| [tracker](tracker/)                      | Portfolio project #2: a work tracker that won't mark anything done until it's proven | Rust                              | [README](tracker/README.md)                  |

### bookmark-manager

The guided first build from [Your First Build](https://github.com/Navigators-Guild/apprentice-onboarding/blob/main/01-talking-to-agents/05-first-build.md). It's a single `index.html` with no build step: add, edit inline, delete, search, and filter by tag, all saved in `localStorage`. Open the file in a browser to run it.

The folder's other files hold the record of how it was built:

- [DESIGN.md](bookmark-manager/DESIGN.md): the design.
- [PROCESS.md](bookmark-manager/PROCESS.md): the build layer by layer, the review triage, learnings and known issues.
- [REVIEW.md](bookmark-manager/REVIEW.md): the adversarial review.
- [docs/prompts/](bookmark-manager/docs/prompts/): the prompt behind each slice and fix.

A Playwright suite tests it in Chromium, Firefox and WebKit.

### tracker

Portfolio project #2, from [Tracking Your Work](https://github.com/Navigators-Guild/apprentice-onboarding/blob/main/02-the-methodology/02-tracking-your-work.md), and my first Rust project. The chapter asks for a simple issue tracker CLI. Mine goes further: agents can't mark work done. An agent submits an item, and the tool runs the item's checks, the project's tests and a set of gates. It then either verifies the item automatically or holds it for a human, with the reasons.

- It was built in 16 vertical slices ([PLAN.md](tracker/docs/PLAN.md)). Each slice was reviewed twice ([docs/reviews/](tracker/docs/reviews/)) and has its own readme.
- From slice 7 on, `tracker` tracked its own build.
- Install it with `cargo install --path tracker`.

## Chapter exercises

[chapters/](chapters/) holds my answers to the exercises in each curriculum chapter. Its folders follow the curriculum's.

| Folder                                                                                       | What's in it                                                                                     |
| -------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| [01-talking-to-agents/02-the-art-of-intent](chapters/01-talking-to-agents/02-the-art-of-intent/)         | Prompts written to state intent clearly                                                          |
| [01-talking-to-agents/03-context-is-everything](chapters/01-talking-to-agents/03-context-is-everything/) | Context for a prompt, the assumptions an agent makes without it, and a context summary           |
| [01-talking-to-agents/04-breaking-problems-down](chapters/01-talking-to-agents/04-breaking-problems-down/) | Project ideas broken into ordered pieces                                                         |
| [01-talking-to-agents/05-first-build](chapters/01-talking-to-agents/05-first-build/)                     | Pointers to the bookmark manager's design and process docs                                       |
| [01-talking-to-agents/06-when-things-break](chapters/01-talking-to-agents/06-when-things-break/)         | Bugs introduced on purpose, and debugging them with an agent                                     |
| [01-talking-to-agents/07-finding-answers](chapters/01-talking-to-agents/07-finding-answers/)             | Finding answers in crate docs, version mismatches, and searching error messages before asking     |
| [02-the-methodology/01-how-we-build](chapters/02-the-methodology/01-how-we-build/)                       | Further adversarial reviews of the bookmark manager (Reviews 2 to 5), and fixes for their findings |

## Agent setup

[.claude/](.claude/) holds the Claude Code skills and agents I wrote for this work. They're shared by every project in the repo:

| Name                                                        | What it does                                                                                                         |
| ----------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| [decompose](.claude/skills/decompose/SKILL.md)              | Breaks a large task into small, dependency-ordered vertical slices, following the "Breaking Problems Down" chapter.  |
| [slice-cycle](.claude/skills/slice-cycle/SKILL.md)          | Runs `tracker`'s remaining slices end to end: build, readme, two reviews with fixes, as stacked pull requests.         |
| [slice-review](.claude/skills/slice-review/SKILL.md)        | Reviews how faithfully one `tracker` slice implements the plan and the design.                                       |
| [slice-readme](.claude/skills/slice-readme/SKILL.md)        | Writes a slice's readme, for developers who weren't there when it was built.                                         |
| `tracker-docs` and `tracker-fixer` agents                   | Write slice docs, and build small review-fix items through the `tracker` workflow.                                   |

Each project also has its own `CLAUDE.md`, with the notes an agent needs to work in that folder.

## Layout

```
guild-portfolio/
├── bookmark-manager/   # portfolio project #1 (HTML, CSS, JavaScript)
├── tracker/            # portfolio project #2 (Rust)
├── chapters/           # chapter exercises, in curriculum order
└── .claude/            # shared Claude Code skills and agents
```

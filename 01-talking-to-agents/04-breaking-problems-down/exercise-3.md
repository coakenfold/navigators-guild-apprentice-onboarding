> Open a conversation with an agent and describe a project idea you're interested in. Ask the agent to help you decompose it. Review the agent's breakdown critically. Is anything too big? Too small? In the wrong order? Revise it.

---

## Decomposition: Family Household-Admin Agent

**Approach:** Layer approach (this is a new project), using user stories to find the layers. The morning summary is the core, and each later feature builds on the same calendar-reading pipeline.

**Core (smallest useful version):** I run one command and get a readable summary of today's events from the shared family calendar, grouped by family member.

**Assumptions:**
- The shared calendar is a **shared iCloud calendar**, accessed over **CalDAV** (`https://caldav.icloud.com`) with an Apple ID and an **app-specific password**. That Apple ID owns the calendar or has accepted the invite, and it has **edit** permission (needed in slice 5). Nothing writes to the calendar until slice 5.
- The **family is defined in `family.yaml`**: each member's name, plus keywords or attendee emails that link an event to that person.
- **Contact details come from `providers.yaml`**, which the family maintains: provider name, phone, email, and which member it serves. The agent doesn't look providers up online.
- Briefings are **delivered by email** through iCloud Mail SMTP (`smtp.mail.me.com`, app-specific password). Reviewing proposed entries happens in the **CLI** for now (no web UI).
- **Claude is used where judgment is needed**: writing summaries, normalizing messy event titles into appointment types, and drafting emails. Date math and cadence detection are plain deterministic code.
- It runs on one machine and is scheduled with cron. It doesn't sync phones directly; iCloud handles that.

### Shared context (give this to every slice's agent)
- **Stack:** Python 3.12, `anthropic` SDK (model `claude-sonnet-5-5`), `caldav` + `icalendar`, `pyyaml`, `pytest`. CLI built with `typer`.
- **Where things live:** `household/` package. `calendar_source.py` has a `CalendarSource` interface with an iCloud CalDAV implementation and a JSON-fixture fake. Also `family.yaml`, `providers.yaml`, `state/` (learned cadences, pending proposals), `tests/fixtures/*.json` (sample calendars).
- **Conventions:** All calendar access goes through `CalendarSource` so tests never touch iCloud. Repeating events are always expanded into individual occurrences (`expand=True` on date searches). LLM calls are wrapped in one module so tests can stub them. Times are timezone-aware, in the family's local zone. Credentials come from environment variables, never committed. Nothing writes to the real calendar without explicit approval.
- **Test command:** `pytest`. None exists yet, so slice 1 sets it up along with the fake calendar fixtures.

| # | Slice | Outcome (user can…) | Depends on | Parallel with | Status |
|---|-------|---------------------|------------|---------------|--------|
| 1 | Today's summary | run `household today` and read today's events grouped by person | — | — | todo |
| 2 | Monday week overview | run `household week` and see the week ahead, with busy days and clashes flagged | 1 | — | todo |
| 3 | Automatic delivery | get the daily summary by email every morning, plus the week overview on Mondays, without running anything | 2 | — | todo |
| 4 | Learned cadences | run `household cadences` and see recurring appointment patterns per person, and correct them | 1 | 2, 3 | todo |
| 5 | Proposed entries + approval | review suggested calendar entries for overdue or due-soon appointments, approve or reject each, and see approved ones appear in the shared calendar | 4 | — | todo |
| 6 | Action helpers | for anything I need to do (book, renew, call), see the phone number, email and a ready-to-send draft, in the briefing and in the review | 3, 5 | — | todo |

### Slice 1: Today's summary
- **Prompt:** "Build `household today [--date YYYY-MM-DD]`. It reads that day's events through `CalendarSource` (implement the iCloud CalDAV version, with repeating events expanded into individual occurrences), assigns each event to family members using `family.yaml` (or 'Everyone' if no match), and prints a short summary grouped by person: time, title, location, and anything to bring or prepare if the event description mentions it. Set up pytest with a JSON fake calendar and test the grouping, a weekly repeating event, and the empty-day case ('Nothing scheduled today')."
- **Done when:** `pytest` passes. Running against the fixture for a busy day lists every event under the right person, in time order, including an instance of a repeating event. An empty day prints the 'nothing scheduled' message. Running against the real iCloud calendar prints today's actual events.

### Slice 2: Monday week overview
- **Prompt:** "Add `household week [--start DATE]`. It shows Monday–Sunday events day by day, grouped by person, and opens with a 3–5 line 'week at a glance' that flags the busiest days, overlapping events for the same person, and times when two people need to be in different places at once. Reuse slice 1's grouping. Don't duplicate it."
- **Done when:** A fixture week with a deliberate clash gets that clash flagged. A quiet week says so. The day sections match what `household today` produces for each day.

### Slice 3: Automatic delivery
- **Prompt:** "Add `household send-briefing`. It emails today's summary every day through iCloud Mail SMTP, and on Mondays it puts the week overview first in the same email. Recipients come from `family.yaml`. Provide a cron entry, plus a `--dry-run` flag that prints the email instead of sending it. If the calendar can't be reached, send a short failure notice instead of nothing."
- **Done when:** `--dry-run` on a Monday date shows the week overview followed by today's summary, and on a Tuesday shows only today. A real send arrives in the inbox. A forced calendar error produces the failure email.

### Slice 4: Learned cadences
- **Prompt:** "Add `household cadences`. It scans the past 24 months of the calendar, normalizes event titles into appointment types per person (e.g. 'Dr. Lee cleaning' becomes 'Dentist'), and detects recurring intervals (median gap, with at least 2 occurrences). Events that already repeat on a calendar rule are treated as one series, not as a learned cadence. It saves results to `state/cadences.yaml` showing last date, typical interval and next due date. It also lets the user edit or ignore a cadence (`--set`, `--ignore`), and those edits survive re-runs."
- **Done when:** A fixture with dentist visits 6 months apart for one child yields 'Dentist, every ~6 months, next due <date>'. One-off events produce no cadence. A weekly repeating event isn't reported as a learned cadence. A cadence the user marked ignored stays ignored after re-running.

### Slice 5: Proposed entries + approval
- **Prompt:** "Add `household propose` and `household review`. `propose` looks for cadences that are due within 4 weeks or overdue with no matching future event, and writes draft entries to `state/proposals.yaml` (a placeholder date and time slot that doesn't clash with the person's existing events). `review` walks through each draft: approve, change the date, or reject. Approved drafts are created on the shared iCloud calendar over CalDAV and tagged (e.g. a marker in the event notes) so they're recognizable; rejected ones aren't proposed again until the next cycle."
- **Done when:** On the fixture, an overdue dentist visit produces one proposal and an already-booked visit produces none. Approving it writes the event to the fake calendar, it then appears in `household today` and `household week` for that date, and running `propose` again doesn't re-propose it. One manual test against a throwaway iCloud calendar creates a real event that shows up in Apple Calendar on a phone.

### Slice 6: Action helpers
- **Prompt:** "For proposals and calendar events that require the user to act (book an appointment, renew a prescription, call someone), attach an action card: provider name, phone and email from `providers.yaml`, and a short draft email or call script written by Claude using the person, the reason and preferred dates. Show these cards in `review` and in an 'Actions for today/this week' section of the briefing. If a provider is missing from `providers.yaml`, say so instead of guessing."
- **Done when:** A due prescription renewal for a known provider shows the correct phone and email plus a sensible draft. An unknown provider shows 'add this provider to providers.yaml'. The Monday briefing email lists the week's actions.

**Deferred / out of scope:** Sending emails or placing calls on the user's behalf, two-way chat with the agent, a web or mobile review UI, calendar services other than iCloud, school-email parsing, reminders other than the briefing, booking directly with providers, reading the Mac Calendar app directly (EventKit).

**Integration risks:**
- **Who an event belongs to (slices 1, 4, 5):** keyword matching on titles is fragile, and wrong assignments break both the summaries and the cadences. Test with real, messy titles early.
- **Repeating events (slices 1, 2, 4):** CalDAV returns a repeating event as one event plus a repeat rule. If it isn't expanded, weekly events go missing from summaries; if it's expanded naively, the cadence learner mistakes them for learned patterns.
- **Recognizing proposals the agent created itself (slices 4 and 5):** once approved entries are on the calendar, the cadence learner must count them as real appointments without creating feedback loops. The tag added in slice 5 exists for this.
- **First write to the real calendar (slice 5):** test against a throwaway iCloud calendar first, and confirm the agent's Apple ID has edit (not view-only) access to the shared one.
- **Briefing layout (slices 3 and 6):** the actions section is added to an email format that already exists. Re-check the Monday layout end to end.

**Build loop:** build one slice → run its new checks **and** the existing suite → commit → next slice. A regression in slice 3 should surface before later slices depend on it. Update the Status column as slices finish.

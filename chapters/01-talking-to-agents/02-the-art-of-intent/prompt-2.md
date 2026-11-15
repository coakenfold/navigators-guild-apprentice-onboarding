## What I'm Building

A simple timer web app where I can create named timers, like "Laundry" or "Meeting in 30 min", so I can tell at a glance what each countdown is for.

## Who It's For

Just me. It's a single-user tool for my own day-to-day reminders.

## What It Should Do

- Create a timer with a name and a duration
- Run multiple timers at the same time
- Pause, resume, and reset each timer individually
- Alert me when a timer finishes (sound or notification)
- Save presets for timers I use often (e.g. "Laundry, 45 min")
- Keep timers running accurately if I close or reload the page
- Delete timers I no longer need

## What It Should Look Like

Clean and minimal: lots of whitespace, with the name and countdown as the focus. It should be mobile-first, with large readable numbers and easy tap targets.

## What It Should NOT Do

- No accounts or login
- No ads or paywalls
- No social features (sharing, collaboration)
- No complex scheduling; it's a timer, not a calendar or alarm clock

## Additional Context

- Runs as a web app in the browser
- Data is stored locally on the device (local storage), with no backend or database
- Phone is the primary device, so it should work well on small screens
- Timers should be based on end timestamps rather than ticking counters, so they stay accurate after a reload

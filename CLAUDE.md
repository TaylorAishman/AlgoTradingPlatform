# Project guide for agents

Keep this file short. It is a map; details live in the files it points to.

## Commands

- Check a change: run the ticket's Proof command. The lint, format, type and test commands are requirements R-246 to R-249 in `docs/design/trading-platform/requirements.md`.
- Run the app: `TODO`
- Run one test: `TODO`

## Where things are

- `docs/reference/` - seed documents chosen by a human. Agents read them and never edit them.
- `docs/design/<slug>/` - `architecture.md`, `requirements.md`, `plan.md`. Read them before changing a component.
- `REVIEW.md` - what the reviewer checks. Agents do not edit it.

## How work happens here

1. Design: `/devteam:design <slug>` runs one stage per call: architecture, then requirements, then plan. A human approves each by merging its PR.
2. Tickets: `/devteam:tickets docs/design/<slug>/plan.md` creates one GitHub issue per plan task.
3. Build: `/devteam:dispatch` runs the current wave. One ticket, one branch, one PR.
4. Review: a clean-context reviewer comments with a verdict pinned to a commit.
5. Merge: a human merges. There is no CI workflow; the ticket's Proof command is the check.

## Rules

- One PR does one thing. Stay inside the ticket's Scope; report anything else you find.
- Fix the code, not the check. Never skip or weaken a test to get green.
- Knowledge that matters goes in the repo (design docs, this file), not in chat.

## Conventions

TODO: language version, formatter, naming, error handling, anything a new contributor gets wrong.

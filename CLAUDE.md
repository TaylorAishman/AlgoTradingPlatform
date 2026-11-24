# Project guide for agents

Keep this file short. It is a map; details live in the files it points to.

## Commands

- Check everything: `CHECK` in `.devteam/config` (lint, type check, tests, build). `/devteam:check` runs it.
- Run the app: `cargo run -p pt-cli -- <args>` (binary `trader`). Needs `~/.cargo/bin` on `PATH`.
- Run one test: `cargo test -p <crate> --test <name>` for an integration test file, or `cargo test -p <crate> <filter>` for a name filter.

## Where things are

- `docs/reference/` - seed documents chosen by a human. Agents read them and never edit them.
- `docs/design/<slug>/` - `architecture.md`, `requirements.md`, `plan.md`. Read them before changing a component.
- `REVIEW.md` - what the reviewer checks. Agents do not edit it.
- `.devteam/config` and `.github/workflows/` - CI/CD. Only the integrator agent edits them.
- `.devteam/worktrees/` - agent worktrees, one per chain. Not committed.
- `.devteam/state/` - landing digests, flaky log, proposed review rules. Not committed.

## How work happens here

1. Design: `/devteam:design <slug> <goal>` writes architecture, requirements and a plan of chains as three commits on `design/<slug>`. A human reads it and runs `/devteam:land <slug>`.
2. Build: `/devteam:run <slug>` builds the ready chains. Each chain is one branch, `chain/<slug>/<chain>`, with one commit per step: tests from a tester, code from an implementer.
3. Review: a clean-context reviewer reads each finished chain commit by commit.
4. Integrate: `/devteam:integrate <slug>` merges reviewed chains into `integration/<slug>` one at a time and runs CHECK after each.
5. Land: chains that pass every gate land automatically (`AUTO_LAND`); the rest wait for `/devteam:land`. One merge per chain, one commit per step, so any of it can be reverted.

`/devteam:auto <slug> <goal>` runs all five, wave after wave.

## Rules

- One step, one commit. Stay inside the step's Files; report anything else you find.
- Every commit leaves CHECK passing, so it can be reverted on its own.
- Fix the code, not the check. Never skip or weaken a test to get green.
- Knowledge that matters goes in the repo (design docs, this file), not in chat.

## Conventions

- Rust 2021, toolchain pinned to 1.100.0 in `rust-toolchain.toml`.
- `cargo fmt` with default settings; `cargo clippy ... -D warnings` must be clean.
- `unsafe` is forbidden.
- Money and prices are decimals (`rust_decimal::Decimal`), never floats.
- One error enum per crate in `error.rs`, built with `thiserror`.
- CHECK runs the Rust checks automatically once a root `Cargo.toml` exists.

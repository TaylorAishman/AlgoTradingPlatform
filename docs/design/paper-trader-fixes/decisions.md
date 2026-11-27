# paper-trader-fixes: decisions

Goal: implement the fixes to the platform listed in
`docs/design/paper-trader/requirements-gaps.md`.

## Scope

Everything (owner, 2026-11-26): every missing-behaviour item (G-10 to G-42,
G-70 to G-86, G-95 to G-100), the test-tightness follow-ups, the housekeeping
items, the accepted confirmations (G-01 to G-04, G-50 to G-52, G-57 to G-59,
G-61 to G-63, G-90, G-91, G-93, pinned as the code behaves now), and the
decided items below. G-23 is already fixed on main.

## Decided before the questions (owner, 2026-11-26)

Recorded in `docs/design/paper-trader/requirements-gaps.md`, "Decisions made
on 2026-11-26": G-53, G-92, G-55, G-56, G-54, G-94, G-05, G-60.

## Questions and answers (owner, 2026-11-26)

1. **What uses up the one first order, and what does an edited `[first_order]` do?**
   Only an order the broker accepted uses it up. An approval rejected on all six
   attempts is dead and later runs do not retry it, but the owner may edit
   `[first_order]` and the new approval gets its own six attempts. Once the
   broker has accepted a first order, an edited approval is ignored with a
   warning alert and the run carries on. R-268 and R-320 are amended.
2. **G-60: keys set but stage `backtest`.** The stage gate wins: account equity
   is used only when keys are set and the stage is `ingest` or higher. In stage
   `backtest` the config value is used and the log says why. The backtest stage
   stays network-free with or without keys.
3. **Is starting capital part of a trial's identity?** No. The same strategy,
   parameters, period and costs at a different capital is the same trial. The
   capital and its source are stored with the run. Hashes already registered
   stay valid. R-367 and D-26 are amended.
4. **G-81: should `trader flatten --yes` cancel, settle and close in one
   command?** Yes. `flatten` cancels every open order the system created,
   settles, then sends the closing orders. `cancel-open` stays as the lighter
   command. The output gains a cancelled count next to G-54's accepted and
   rejected counts.
5. **G-26 and G-27 (fail-closed risk checks).** Keep both and pin them with
   tests. No behaviour change; the kill switch does not go through the
   pre-trade checks and bypasses halts.
6. **G-20: webhook redirects.** Do not follow redirects. Any 3xx is a failed
   delivery, the alert is stored with `delivered == false`, and exactly one
   request is made.
7. **G-17: a universe symbol with no stored bars.** Report it: `data check`
   lists the symbol as having no bars and exits 1. R-126 and R-383 are amended.
8. **G-53 scope.** Every stage, as decided: `cancel-open` and `flatten` run in
   all five stages and need only keys and `--yes`. They bypass the flat
   confirmation and the allowlist; each kill-switch order is checked against
   the broker's position (opposite side, quantity no larger).

## Architect's assumptions (accepted with the answers above)

- General rule: where today's behaviour is a defect (swallowed errors,
  overflow, NaN, leaks, non-atomic writes) it is fixed; where it is only
  unspecified it is pinned by a requirement and a test.
- G-53: the last sentence of R-384, the DryRun clause of R-321, R-269 and the
  Q5 statement "refuses to send orders until the owner's approval is recorded"
  are amended to except kill-switch orders. The gate gets a separate
  kill-switch authorisation that checks the order against the broker's
  position. Fills from a flatten made before the opening balance exists are
  kept out of the paper ledger.
- G-05: launchd cannot schedule in another zone and macOS cron ignores
  `CRON_TZ`. The schedule fires every 15 minutes, and a new `trader run` flag
  makes the binary do nothing unless one of the three New York slots (08:00,
  09:45, 17:30, weekdays) has come due since the last started run. Adds the
  `chrono-tz` dependency (MIT or Apache-2.0). R-400 is amended.
- G-92 mechanics: `qty` is required whenever `[first_order]` is present, so an
  existing owner file without it fails to parse until the owner adds it.
  Retries happen inside one run with waits of 1, 2, 4, 8 and 16 seconds through
  the injected sleeper, under ids `-r1` to `-r5`. Only a 403 or 422 answer to
  the order POST counts as a broker reject. After the last retry: a critical
  alert, no halt, and the run ends failed.
- G-55 and G-92 share one helper in `pt-live`: look up the spent id, adopt if
  the broker holds it. Adoption needs one new allowed transition, Rejected to
  Submitted, so R-017 is amended.
- G-73: a 401 is never an order rejection; it fails the run and leaves the row
  as Intent.
- G-56: alert code `incomplete_fill` for leg 1 and `order_abandoned` for leg 2;
  no halt.
- G-54: `flatten` prints accepted and rejected counts and exits 1 when any
  order was rejected.
- G-60 details: the config key is `[backtest] initial_capital` (default
  100000), left out of `config_hash`. If the account read fails, the config
  value is used with a warning. `backtest crosscheck` also gets `--capital` but
  never reads the account.
- Store: migrations are additive and an existing database must still open.
  G-16 is pinned, not migrated (second precision, insertion order as the
  tie-break). G-10 and G-11 become typed errors that store nothing. G-12 makes
  `asset_snapshots` append-only (identical repeat is a no-op, a different one
  is `Duplicate`). G-13 ignores only a repeated `activity_id`. G-14 requires
  `filled_qty` to be at most the order quantity and never to fall. G-15 stays
  an error, with its own "already cleared" message.
- Gate checklist: G-21 makes `alert_webhook` see `PT_ALERT_WEBHOOK_URL`. G-25
  uses `first_order_sent` as a thirteenth checklist item, so R-270 and R-382
  change from twelve lines to thirteen.
- Smaller calls: G-22 P&L from a zero base is absent, not 0. G-33 keeps `>= 2`.
  G-36 to G-40 and G-42 become ineligibility or typed errors. G-72 keeps three
  alert levels and pins each code's level. G-74 turns the silent skip into an
  error. G-75 and G-76 use one transaction each. G-78 adds a warning alert.
  G-80 counts release events. G-82 counts only `planned` decisions as
  rebalances and pins the last-session rule. G-95 moves `StoredTsmom` into a
  shared module and fills the fidelity section. G-96 to G-100 are fixed as the
  gap table suggests. G-31 is checked against Alpaca's documentation during
  design.
- Housekeeping inside the repository (unused dependencies, duplicated code,
  stale docs, shared test fixtures, a per-route reject hook in
  `pt-mock-alpaca`) becomes chains. Items outside what a chain may touch
  (`check.sh` logging and its shared log, per-worktree target directories, the
  stale remote `integration/paper-trader`, process notes) are recorded as
  notes for the owner or integrator, not as steps.
- Flaky tests are in scope only where a cause is found in the test code;
  otherwise they are recorded as a note.
- Documents: the new design lives under `docs/design/paper-trader-fixes/`; the
  old three files are not edited. Amended requirements keep their R-number and
  are restated in full; new requirements start after R-402; new step IDs
  continue each component's numbering.
- All tests use fake keys, the mock broker and loopback only; no secrets file
  is read.

## Revision 2026-11-27: R-512 date (owner)

- Question: R-512's acceptance check named TLT's bar for 2024-02-29, which is the signal date in the late setup; readiness is checked on the reference date (2024-03-06), the behaviour the owner accepted as G-58. Correct the requirement, or change the code?
- Answer: correct R-512 to the reference date, 2024-03-06. O74 stays a test-only pin (Arch F-21).
- New gap for a later design (not in this design's scope): in a late rebalance, a missing bar on the signal date is not caught by the readiness check. The strategy then treats that symbol as ineligible and the decision goes ahead without it. For a late first paper rebalance this can plan real orders with the symbol left out. On-time rebalances are not affected.

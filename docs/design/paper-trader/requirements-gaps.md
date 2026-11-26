# paper-trader: requirement gaps found during the build

Reviewers and testers found these while building the waves. Waves 1 to 4 are in the first tables; waves 5 to 9 and waves 10 to 15 are in their own sections at the end. Each is behaviour that `requirements.md` does not pin down, so the code does something reasonable but untested, or something that may not be what we want. Resolve them after the current `/devteam:auto` run by adding or amending requirements and re-planning the affected chains.

Each entry gives the area, what is unspecified, what the code does today, and a suggested direction. "Decide" means a human choice is needed.

## Needs a decision

| # | Area | Gap | Today | Suggested |
|---|------|-----|-------|-----------|
| G-01 | store-orders (D14) | Timestamp of an order's creation event | Stamped at `trade_date` 00:00 UTC so the store never reads the wall clock | Decide: keep the synthetic time, or pass the real submit instant in from the caller. The D14 commit body says `Utc::now()`, which no longer matches the code |
| G-02 | engine-decide (K09) | How a `Rejection` is written in decision JSON | `format!("{r:?}")`: renaming a variant silently changes stored JSON that shadow-replay reads | Add a requirement for a stable string per reason, such as `order_notional` |
| G-03 | engine-decide (K08) | A held position whose symbol is outside the universe | Gets no reference-price check, so `plan_orders` fails with `MissingPrice` instead of `DataNotReady` | Decide whether held symbols outside the universe must be priced and flattened, and which error applies |
| G-04 | engine-decide (K08) | A missing bar for an unheld, zero-weight symbol | Not an error (the tester's reading of R-196) | Confirm in R-196 |
| G-05 | ops / launchd | Schedule time zone | launchd uses local time | State the zone (US/Eastern market time, or UTC) and test it |

## Missing behaviour (add a requirement and a test)

| # | Area | Gap | Today |
|---|------|-----|-------|
| G-10 | store-market (D07) | Two different bars for the same series, date and `fetched_at` in one batch | The primary key is violated and the whole batch fails with a SQL error |
| G-11 | store-market (D07) | A `fetched_at` earlier than the stored latest version | Compared against the later version and inserted as an older row |
| G-12 | store-market (D11) | A second asset snapshot for the same symbol and instant | `INSERT OR REPLACE` silently overwrites the first one, and `asset_snapshots` is not append-only (R-057) |
| G-13 | store-orders (D15) | `insert_fill` and constraint failures | `INSERT OR IGNORE` swallows every constraint failure, not only a repeated `activity_id` |
| G-14 | store-orders | `filled_qty` | Not validated (for example, against the order quantity) |
| G-15 | store-ops | `clear_halt` on a halt that is already cleared | Returns `NotFound` |
| G-16 | store | Instant precision | Second precision only; sub-second order is lost |
| G-17 | data-quality (G08) | A symbol with no stored bars at all | Silently skipped by `check_gaps`; expected to be covered by data-universe (G18), so confirm that |
| G-18 | data-quality (G07) | The shape of the bad-bar error | Tests only check `is_err()`, but R-125 later relies on `DataError::BadBar` naming the symbol and date |
| G-19 | ops-alerts (Q07) | Scrubbing the alert `code` | Only `message` is scrubbed, so a code built from error text could leak a secret |
| G-20 | ops-alerts (Q05) | Webhook redirects | ureq follows redirects, so a 307 gives a second POST, against "one attempt" in R-352 |
| G-21 | ops | `alert_webhook` and the environment | Ignores the environment |
| G-22 | ops-pnl | Zero equity | Returns 0 |
| G-23 | cli-base (R03) | `gates` keys item | Counts only the `APCA_*` variables, not `PT_SECRETS_FILE`, which `load_keys` accepts |
| G-24 | cli-base (R06) | JSON logging for `trader run` | R-401 covers the live run, but `run` is still a stub, so only the error and refusal paths are tested. Later chains must keep every stderr line of `run` JSON |
| G-25 | gate | `first_order_sent` | Unused |
| G-26 | risk | A reducing order on an unpriced symbol | Blocked by the risk checks |
| G-27 | risk / halts | The first order after a halt | Refused |
| G-28 | alpaca | Asset symbol in URL paths | Not percent-encoded |
| G-29 | alpaca | `cancel_order` id in URL paths | Not percent-encoded |
| G-30 | alpaca | Notional in a foreign format | Gives a Parse error |
| G-31 | alpaca (E20) | Pagination `after` | Exclusive; confirm this matches the API |
| G-32 | engine-sim | `start > end` | Not rejected explicitly |
| G-33 | engine-sim | `margin_multiplier` threshold | Uses `>= 2`; confirm the boundary |
| G-34 | engine (vector backtest) | First-day weights | Ignored |
| G-35 | engine (I01) | All weights zero | Behaviour not specified |
| G-36 | strategy (H05) | NaN volatility | Not made ineligible |
| G-37 | strategy | Empty lookbacks | Give signal 0 |
| G-38 | stats | NaN inputs to the stats functions | Not specified |
| G-39 | stats | OLS with a singular design | Can return inf |
| G-40 | stats | `n_combinations` | Can overflow |
| G-41 | stats (PBO) | Even split rule | Confirm the rule for an even number of partitions |
| G-42 | alpaca limiter | A limit of 0 | Turned into 1 |

## Test-tightness follow-ups (requirement exists, test is loose)

- R-003 licence test is loose.
- K12 compares with a 1e-4 tolerance.
- E23 tests queue only one reply.
- G06 `import_calendar` accepts either an error or an empty calendar after a bad row. `store.calendar()` returns `NotFound`, so assert that.
- K09 originally checked R-197 only through `decide_with`. Fixed in review; listed so the pattern is avoided elsewhere.

## Housekeeping (not requirements)

- Unused manifest dependencies: `pt-stats` in `pt-shadow`, `pt-config` in `pt-ops`, `csv` in `pt-data`, `pt-validate` in `pt-cli`, `pt-core` in `pt-strategy`, `serde_json` in `pt-engine` before K09.
- EWMA code is duplicated between crates. `positions_at` and `cash_at` repeat the fold from `positions` and `cash`. `stage_name()` exists in both `wiring.rs` and `cmd_config.rs`. `decide` calls `target_weights` twice.
- `wiring::config_path()` re-parses argv. Pass the resolved path from `dispatch` instead.
- Stale docs: `ops.rs` module doc; `MarketView::adv_shares` doc says "before asof" where R-138 says "on or before".
- Several steps landed as test-only commits because an earlier step had already implemented them (H07, H09, H12, H13, K13, K16, K18, D09, G05). H08 and K13 overreached their step's scope.
- C14 and C13 fixture changes.
- The remote `integration/paper-trader` is stale. Each integration rebuilds it from `main`, so pushing it would need a force push, which the workflow forbids.

## Found in waves 5 to 9

These came from reviewers, testers and implementers while building waves 5 to 9 (ops, live, shadow, validate and cli chains).

### Needs a decision

| # | Area | Gap | Today | Suggested |
|---|------|-----|-------|-----------|
| G-50 | ops-report | Shape of `status.json` | Fields chosen by the implementer; no requirement pins the keys | Fix the schema in a requirement so cli-ops and any dashboard can rely on it |
| G-51 | live-rebalance (O29) | Id of a dry-run order | Prefixed `dry-` so it can never collide with a real client order id | Confirm, and state it in R-30x |
| G-52 | live-rebalance | `FirstOrder` stage | `rebalance` treats it like Paper (`dry` depends on the stage only) | live-cycle must exclude rebalancing in FirstOrder (R-319); confirm and test there |
| G-53 | gate / live-killswitch | `flatten` in the `FirstOrder` stage | The gate refuses every order except the approved first order, so the kill switch cannot flatten in that stage | Decide whether risk-reducing kill-switch orders bypass the first-order rule |
| G-54 | live-killswitch (O46) | Return value of `flatten` | Counts orders sent and accepted by this call; rejected and adopted orders are not counted, and only an `order_rejected` alert signals a partial failure | Decide what the CLI reports; cli-ops should surface the alerts |
| G-55 | live-killswitch (O46) | A stored Rejected row whose order the broker still holds | Not looked up before a retry, so a second order could be sent under `-r1` (needs the broker to reject and keep the order) | Look up the spent id before taking a retry id, if it is worth one extra request |
| G-56 | live-legs (O16) | Leg 1 that ends Expired or Canceled after a partial fill | Leg 2 is neither released nor abandoned and stays `Intent` forever | Decide: abandon, release for the filled part, or alert |
| G-57 | live-rebalance-timing (O31) | What counts as "the first paper rebalance" | `decisions(mode).is_empty()`: a skipped_late row counts as a prior decision, and DryRun mode gets its own exemption | Confirm R-306 means this |
| G-58 | live-rebalance-timing (O30) | A late rebalance whose bars are missing | Returns DataNotReady; it is skipped as late on a later call once data arrives | Confirm |
| G-59 | live-rebalance-timing (O33) | Level of the `not_tradable` alert, and alerts on a skipped_late decision | Warn; a skipped_late decision writes no `not_tradable` alerts | Confirm R-308 |
| G-60 | cli-backtest (R13) | Initial capital and strategy choice | Capital hard-coded to 100000; strategy defaults to `tsmom`. Architecture 5.8 lists a `--capital` flag that `cli.rs` does not have | Add `--capital` (or a config field) and a requirement for it |
| G-61 | validate (V07, V10) | Borrow cost on the last session, and which dates enter the return comparison | Last-session borrow day is ambiguous; returns are compared on common dates only | State both in requirements |
| G-62 | validate-overfit | DSR and benchmark config vs the stored run | The config passed in is not checked against the run's stored `config_hash` | Decide whether a mismatch is an error |
| G-63 | architecture 5.4 | `Store::backtest_family` (D27, cli-validate replan 1) | Added by a replan; not listed in architecture 5.4 | Add the line at the next architecture revision |

### Missing behaviour (add a requirement and a test)

| # | Area | Gap | Today |
|---|------|-----|-------|
| G-70 | ops-report | Timestamp used for the pre-open status | Not pinned by a test |
| G-71 | live-snapshots | Anchor of "five sessions" | Not pinned by a test |
| G-72 | live-reconcile (O09) | Alert level for reconciliation breaks | Warn, because there is no Error level |
| G-73 | live-reconcile (O09) | Which broker errors count as Rejected | Catches Rejected too broadly, including 401 and 403 |
| G-74 | live-reconcile (O08) | Staleness check | Can be silently disabled by config |
| G-75 | live-submit (O12) | Fill and ledger entry | Not written atomically; a re-add fallback covers a crash between them |
| G-76 | live-rebalance (O27) | Decision row and its orders | Not written atomically; a backfill from `outputs_json` covers a crash between them |
| G-77 | live-rebalance (O26) | `data_asof` vs the fixture's bar time | They differ in the tests; confirm which instant is stored |
| G-78 | live-rebalance | An asset with no stored snapshot | Silently dropped from the universe, with no alert |
| G-79 | live-settle (O19, O20) | Incomplete-fill alert and broker paging | Alert recorded before the state change, with a mock-fix exception; paging of order lists not covered |
| G-80 | live-legs (O15) | `legs_released` | A level count recomputed every settle, not an event count |
| G-81 | live-killswitch (O45) | Ordering of cancel, settle and flatten | `flatten` skips a symbol whose row is still Submitted, so settle must run between `cancel_open` and `flatten` (documented, not enforced) |
| G-82 | shadow-fidelity (P10, P11) | Counting | The last session is not counted as missed; `n_rebalances` includes skipped_late decisions |
| G-83 | shadow-daily (P03) | Order states considered | Chosen by the implementer; not pinned by R-333 |
| G-84 | shadow-slippage (P02) | Fixture size | Larger than the step guideline |
| G-85 | cli-data (R08, R10, R11, R12) | Dispatch | Several steps needed small dispatch changes outside their Files |
| G-86 | cli-validate (R16) | `validate dsr` on a benchmark run | No test shows it counts only its own family's trials |

### Test-tightness follow-ups

- shadow-signoff (P14): `signoff.md` format is asserted loosely (verdict word, rows by id); R-343 does not fix the layout.
- cli-validate (R17, R18): output labels are matched loosely because R-391 and R-392 do not name them.
- live-killswitch (O46): rejection is tested through a loopback proxy in `flatten.rs`, because `pt-mock-alpaca` has no per-route reject hook. Add one and move the test onto it.

### Flaky tests

- `pt-validate` `run_and_record`, and one test around c04e8fb.
- A `pt-shadow` test failed once at P13 (c2a635e, 33.9 s) and passed on retry; not reproduced in five runs at the tip.
- `flaky.log` often records only cargo's summary line ("0 passed; 0 failed"), not the failing test's name. `check.sh` should log the name.

### Housekeeping (not requirements)

- More unused manifest dependencies: `pt-live` and `pt-shadow` in `pt-cli`.
- Duplicated code: the halt helper (`reconcile.rs` private `halt()` and `settle.rs` `halt_unexpected`); `crosscheck.rs` repeats setup from `runner.rs`; `cmd_backtest.rs` `family()` repeats strategy ids (`tsmom-v1`, `bh-volscaled-v1`); the cli `Env` and fixture helpers (about 150 lines) are copied into `validate_dsr.rs`, `validate_pbo.rs` and `validate_benchmark.rs`.
- `check.sh` writes one shared `.devteam/state/last-check.log`; concurrent checks overwrite each other's log.
- Process: O46 landed at 449 changed lines, over the 400-line step guideline, after review fixes (mostly test code). live-killswitch took five review rounds, two past the usual cap of three. One fix agent reported a per-commit proof that had not actually compiled (the missing `AlpacaError` import in O45).

## Found in waves 10 to 15

These came from the live-first-order, live-cycle-failures, cli-live, cli-ops and cli-e2e chains.

### Decisions to confirm

| # | Area | Question | Today |
|---|------|----------|-------|
| G-90 | live-rebalance (O37) | Do Deferred and SkippedLate outcomes submit nothing? | They skip submit, but the doc comment says otherwise |
| G-91 | live-cycle-failures (O40) | Which errors become a Failed run with `run_failed` | Caught broadly; the test does not check that no order went out |
| G-92 | live-first-order | The approval changes after the order was sent | A hard error; no retry after a broker reject; `max_qty` is used as the order quantity |
| G-93 | gate (N12) | `Gate::first_order()` getter | Added by a replan; not in architecture section 5.3 |
| G-94 | cli-ops (R24) | Owner sets all three sign-off minimums to 0 | An empty store with no calendar would give PASS; it gives NOT_READY today only because the defaults are above 0 |

### Missing behaviour (add a requirement and a test)

| # | Area | Gap | Today |
|---|------|-----|-------|
| G-95 | cli-live (R21), cli-ops (R23) | Fidelity section of `status.json` | Always empty; filling it needs the private `StoredTsmom` wrapper in `cmd_run.rs` moved somewhere shared |
| G-96 | cli-live (R19, R21) | A store failure at the start or end of a run | `run_cycle` returns an error before the report is written; the plan says the report is written on every path except a held lock |
| G-97 | cli-live (R20) | "A locked run writes nothing" | The store is opened (WAL, migrations) before the lock is taken; take the lock first |
| G-98 | cli-ops (R25) | `--yes` checked first | `main.rs` loads keys and builds a client before `cancel-open` and `flatten` check `--yes`. Without keys the error is about keys, and below the stage minimum the exit is 4, not 1. No request is ever made. |
| G-99 | shadow-fidelity (R24) | `fidelity()` on a store with no calendar | Errors with NotFound; `signoff report` special-cases it with an all-zero `Fidelity` |
| G-100 | cli-ops (R25) | Kill-switch output | No test checks that no secret appears in the output, or the exit-4 path when the stage is too low |

### Test-tightness follow-ups

- cli-ops (R23): `status` "exits 0 right after a run" seeds a finished `ok` run row instead of running `trader run`. R27's e2e covers the real sequence.
- cli-e2e (R27): the help-text check uses substring needles, so a flag spelled another way would pass.

### Housekeeping (not requirements)

- Duplicated code in `pt-cli`: `open_store`, a `Tsmom` placeholder for the strategy id and the webhook env lookup are copied from `cmd_run.rs` into `cmd_ops.rs`, because each step's Files allowed only `cmd_ops.rs`. The live fixture (`sessions`, `calendar_sessions`, `env()`) is copied into `run_ok.rs`, `run_exit_codes.rs`, `run_report.rs`, `killswitch.rs` and `e2e_paper.rs`; move it to `tests/common`.
- Stale text: the `cmd_ops.rs` module doc still says "stubs"; `main.rs` says "the client is not used by the stub command modules yet"; the R25 doc comments say "before any client is built".
- `pt-shadow` declares an unused `pt-stats` dependency (a cargo warning on every CHECK).
- Shared `CARGO_TARGET_DIR` between worktrees left test binaries with another worktree's absolute path baked in, which failed CHECK until `cargo clean -p pt-cli`. Give each worktree, reviewer and integrator its own target dir.
- Process:
  - R19's implementer fixed the tester's mock calendar (a setup bug: no month end before the trade session); disclosed in the commit, no assertion changed.
  - R26 and R27 were test-only: the commands already met R-393, R-399 and R-402.
  - R27 landed at 448 lines, over the 400-line guideline (one test file with a shared fixture).
  - cli-live took three review rounds.
  - Some agent reports (V04, V10, R26) came back as the bare word "placeholder", so the orchestrator checked the work itself.

## Decisions made on 2026-11-26

The owner decided these. Each needs a requirement, a test and a change in the next design.

| # | Decision |
|---|----------|
| G-53 | The kill switch works in every stage. Risk-reducing kill-switch orders (`cancel-open`, `flatten`) bypass the stage gate, including in `first_order`. |
| G-92 | The `[first_order]` approval gains a `qty`, which must be at least 1 and at most `max_qty`; the order is sent for `qty`. A broker reject is retried with exponential back-off, at most 5 retries. Before each retry, look up the previous client order id at the broker (as in G-55) so a held order is adopted, never duplicated. After the last retry, alert and stop. |
| G-55 | Before a kill-switch retry under a new `-rN` id, look up the spent id at the broker; adopt the order if the broker still holds it. |
| G-56 | When leg 1 ends Expired or Canceled after a partial fill: alert, and abandon leg 2. The shares leg 1 bought are kept. |
| G-54 | `flatten` prints both counts: orders accepted and orders rejected. |
| G-94 | Config validation rejects 0 for each sign-off minimum (`min_sessions`, `min_rebalances`, `min_fills`); each must be at least 1. |
| G-05 | The daily schedule runs on US/Eastern market time, not the Mac's local zone. |
| G-60 | `backtest run` gains `--capital`. Without it, use the paper account's current equity when keys are set; otherwise use a config value (default 100000). Log which source was used. The offline path stays network-free when no keys are set (R-393). |

Accepted as they are (owner, 2026-11-26): G-01 to G-04, G-50 to G-52, G-57 to G-59, G-61 to G-63, G-90, G-91 and G-93. The next design writes the current behaviour into requirements and pins it with tests.

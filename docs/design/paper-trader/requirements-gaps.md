# paper-trader: requirement gaps found during the build

Reviewers and testers found these while building waves 1 to 4. Each is behaviour that `requirements.md` does not pin down, so the code does something reasonable but untested, or something that may not be what we want. Resolve them after the current `/devteam:auto` run by adding or amending requirements and re-planning the affected chains.

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

# paper-trader-fixes: architecture

This design extends `docs/design/paper-trader/`. It changes only what is listed
here; everything else in the old architecture, requirements and plan stands.
Section and decision numbers written as "old 5.3" or "old D-26" point into
`docs/design/paper-trader/architecture.md`.

## 1. Problem

The paper trader was built in fifteen waves, and its reviewers, testers and
implementers recorded about ninety places where the requirements were silent,
loose or wrong (`docs/design/paper-trader/requirements-gaps.md`). This design
closes every one of them for the owner: defects are fixed, unspecified
behaviour is pinned by a requirement and a test, and the eight owner decisions
of 2026-11-26 (kill switch in every stage, first-order quantity and retry,
market-time schedule, backtest capital and the rest) are built.

## 2. Sources

| Source | What it contributed |
|---|---|
| `docs/design/paper-trader-fixes/decisions.md` | Scope (everything), the owner's eight answers and the accepted assumptions. It overrides any preference of this document. |
| `docs/design/paper-trader/requirements-gaps.md` | The gap list G-01 to G-100, the test-tightness and housekeeping lists, and the decisions table of 2026-11-26. |
| `docs/design/paper-trader/architecture.md`, `requirements.md`, `plan.md` | The components and their letters, the interfaces of old section 5, the error enums, decisions D-01 to D-31, the text of R-001 to R-402 and the highest step ID of each component. |
| The code on `main` at `b3f76e6` | What each gap does today. Every "today" statement in this design was checked against the source, not taken from the gap table. |
| `docs/reference/` | Nothing new. The old design's use of the references stands; no fix here changes a formula taken from them. |
| Alpaca API reference, "Get All Orders" (`docs.alpaca.markets/reference/getallorders-1`, read 2026-11-26) | G-31: the `after` parameter of `GET /v2/orders` is documented as "submitted after this timestamp (exclusive)", `limit` has default 50 and maximum 500. The paging in `orders_since` matches the API. |

### Conflicts

These are places where two sources disagree. Each is resolved in section 6 and
named there; none is resolved silently.

1. **R-003 against the dependency tree.** R-003 says no licence string may
   contain `GPL`. The package `r-efi` declares
   `MIT OR Apache-2.0 OR LGPL-2.1-or-later`, and the existing test passes it
   because one alternative of an `OR` expression is enough. The test is right
   and the requirement text is wrong. See F-27.
2. **G-72 against the code.** The gap table says reconciliation breaks alert
   at `warn`. The code raises `halt_set` at `critical` in `reconcile.rs`,
   `settle.rs` and `snapshot.rs`. The code is pinned. See F-20.
3. **G-71 names the wrong area.** It is filed under live-snapshots, but the
   only "five sessions" rule is `RAW_OVERLAP_SESSIONS` in
   `crates/data/src/ingest.rs`. It is pinned there. See F-24.
4. **Decisions, G-92: "ids `-r1` to `-r5`" against "a new approval gets its
   own six attempts".** An approval edited on the same day for the same symbol
   shares the id stem `pt-first-<valid_on>-<symbol>`, so its attempts cannot
   reuse `-r1` to `-r5`. They continue with the next unused suffix. See F-08.
5. **Housekeeping list against today's manifests.** Most of the dependencies
   listed as unused (`pt-config` in `pt-ops`, `csv` in `pt-data`,
   `pt-validate`, `pt-live` and `pt-shadow` in `pt-cli`) are used now. A scan
   of the manifests against the source finds a different set, among them
   `pt-stats` and `pt-calendar` in `pt-shadow` and five crates in `pt-cli`.
   The scan, not the list, is the requirement. See F-28.

### Gaps

- The Alpaca reference does not say whether `after` on the account activities
  endpoint is inclusive. Nothing here depends on it: fills are deduplicated by
  activity id.
- `orders_since` pages by the last `submitted_at` of a page. Two orders with
  the same `submitted_at` on either side of a page boundary would lose the
  second one. It needs more than 500 orders and an exact tie, and is recorded
  as accepted in F-25.
- `crates/cli/src/wiring.rs` builds the client with a constant of 180 requests
  per minute and ignores `data.max_requests_per_minute`. It is not in the gap
  list and is recorded under Out of scope in `requirements.md`.
- No cause was found in test code for the three flaky tests of the gap list.
  They stay a note for the owner.
- `chrono-tz` is not in this machine's cargo cache. The step that adds it
  needs network access to crates.io once.

## 3. Constraints

- **Additive store changes.** One new migration, `0005`. A database written by
  migrations 0001 to 0004 must open and keep every row.
- **Trial hashes stay valid.** A hash registered before this design must be
  produced again for the same request.
- **`config_hash` stays valid.** Adding `[backtest]` must not change it.
- **No change to a formula** from the reference papers, to the decision JSON
  layout, to client order id formats, or to the order of run phases.
- **One new dependency:** `chrono-tz` (MIT or Apache-2.0), in `pt-live` only.
- **The safety boundary keeps its shape.** `OrderPermit` is still created only
  inside `pt-gate`; the kill switch gets a second, narrower door, not a way
  round the gate.
- **The backtest stage stays network-free**, with or without keys, except for
  the two kill-switch commands the owner asked for.
- **Tests** use fake keys, `pt-mock-alpaca` and loopback only. No test reads a
  secrets file, and the live trading host appears nowhere in source.
- **Repository conventions** hold: Rust 2021 on toolchain 1.100.0, decimals
  for money, one `thiserror` enum per crate, no `unsafe`, `cargo fmt` and
  clippy clean, integration tests under `crates/<name>/tests/`.
- **The old design files are not edited.** An amended requirement keeps its
  number and is restated in full in the new `requirements.md`; a new one
  starts at R-403; a new step continues its component's numbering.

## 4. Components

The components and their letters are those of the old design. The table lists
what each one gains or changes and the files this design touches.

| Prefix | Component | Change in this design | Files touched |
|---|---|---|---|
| W | Workspace and `pt-archtest` | Licence rule in SPDX terms; a scan for unused dependencies; a scan that duplicated helpers stay single; unused dependencies removed | `Cargo.lock`, `crates/*/Cargo.toml`, `crates/archtest/tests/` |
| A | `pt-core` | One new order-state transition, Rejected to Submitted; one doc fix | `crates/core/src/order_state.rs`, `crates/core/src/view.rs` |
| C | `pt-config` | `[backtest] initial_capital`; sign-off minimums at least 1 | `crates/config/src/config.rs`, `config/paper-trader.example.toml` |
| D | `pt-store` | Migration 0005; typed errors for bad bar batches; append-only asset snapshots; strict fills; `filled_qty` checks; off-ledger orders; atomic fill and decision writes; backtest capital | `crates/store/migrations/0005_fixes.sql`, `crates/store/src/{db,error,market,trading,ops}.rs` |
| E | `pt-alpaca` | Path segments percent-encoded; limiter refuses 0; `is_order_reject`; `HttpClient::sleep` | `crates/alpaca/src/{error,limiter,http,reference,orders_write}.rs` |
| F | `pt-mock-alpaca` | Two per-route reject hooks; a settable margin multiplier | `crates/mock-alpaca/src/{server,routes,state}.rs` |
| G | `pt-data` | Symbols with no bars are reported | `crates/data/src/quality.rs` |
| H | `pt-strategy` | Non-finite volatility is ineligible; empty lookbacks are an error; one EWMA weights function; strategy ids as constants | `crates/strategy/src/{estimate,tsmom,vol,benchmark}.rs` |
| I | `pt-portfolio` | Pins only | tests |
| J | `pt-risk` | Pins only | tests |
| K | `pt-engine` | `start > end` refused; `decide` asks the strategy once | `crates/engine/src/{simulate,decision}.rs` |
| L | `pt-vector` | Pins only | tests |
| M | `pt-stats` | Overflow, odd partitions, non-finite inputs and singular OLS become typed errors; `max_drawdown` propagates NaN | `crates/stats/src/{pbo,ols,dsr,normal,perf}.rs` |
| N | `pt-gate` | `qty` in the first-order approval; kill-switch authorisation; `CancelOrder` in every stage; thirteenth checklist item; `Stage::as_str` | `crates/gate/src/{owner,gate,status,error}.rs`, `config/owner.example.toml` |
| O | `pt-live` | Kill switch in every stage that cancels, settles and closes; adoption of held orders; first-order retry; strict reject classification; atomic writes; schedule slots; shared halt helper | `crates/live/src/{adopt,halts,slots,killswitch,first_order,cycle,settle,reconcile,snapshot,submit,rebalance,error,lib}.rs` |
| P | `pt-shadow` | `n_rebalances` counts planned decisions only; fidelity of a store with no calendar is all zero | `crates/shadow/src/fidelity.rs` |
| Q | `pt-ops` | Alert code scrubbed; no redirects; P&L absent on a zero base | `crates/ops/src/{alert,pnl,report}.rs` |
| V | `pt-validate` | Capital leaves the trial hash and is stored with the run; crosscheck shares the runner's setup | `crates/validate/src/{runner,crosscheck}.rs` |
| R | `pt-cli` and `deploy/` | Kill-switch commands in every stage; `run --if-due`; `backtest --capital`; fidelity in the report; thirteen checklist lines; shared helpers and test fixtures; fifteen-minute schedules | `crates/cli/src/*.rs`, `crates/cli/tests/`, `deploy/` |

`pt-calendar` (B) is not touched.

New files: `crates/store/migrations/0005_fixes.sql`,
`crates/live/src/adopt.rs`, `crates/live/src/halts.rs`,
`crates/live/src/slots.rs`, `crates/cli/src/strategies.rs`,
`crates/cli/src/reporting.rs`, `crates/cli/src/cmd_kill.rs`,
`crates/cli/tests/common/live.rs`, `crates/cli/tests/common/offline.rs`.

## 5. Interfaces

Only new or changed interfaces are listed. Anything not listed is as in old
section 5.

### 5.1 `pt-core`

`transition_allowed` gains one pair: Rejected to Submitted. It is used only
when the broker turns out to hold an order the system had marked rejected.

### 5.2 `pt-config`

```toml
[backtest]
initial_capital = 100000   # a decimal above 0; default 100000
```

```rust
pub struct BacktestCfg { pub initial_capital: Decimal }
pub struct Config { /* existing fields */ pub backtest: BacktestCfg }
```

`config_hash` does not include `backtest.initial_capital`. `validate` refuses
`signoff.min_sessions`, `signoff.min_rebalances` or `signoff.min_fills` of 0.

### 5.3 `pt-gate`

```rust
pub struct FirstOrderApproval {
    pub symbol: String,
    pub side: String,
    pub qty: u64,        // new, required: 1 <= qty <= max_qty
    pub max_qty: u64,
    pub valid_on: NaiveDate,
}

impl Stage { pub fn as_str(self) -> &'static str }   // backtest, ingest, dry_run, first_order, paper

impl Gate {
    /// Existing; added by a replan and listed here for the first time (G-93).
    pub fn first_order(&self) -> Option<&FirstOrderApproval>;

    /// New. The only way besides `authorize` to obtain a permit.
    pub fn authorize_kill_switch(
        &self,
        order: &OrderSummary,
        broker_position: i64,
    ) -> Result<OrderPermit, GateError>;
}

pub enum GateError { /* existing */ NotRiskReducing }
```

`authorize_kill_switch` checks no stage, flat confirmation, allowlist or halt.
It returns a permit only when all of these hold, and `NotRiskReducing`
otherwise:

- `order.client_order_id` starts with `pt-flatten-`;
- `broker_position` is not 0;
- `order.side` is `sell` for a positive position and `buy` for a negative one;
- `1 <= order.qty <= |broker_position|`.

`authorize` for a first order now requires `order.qty == approval.qty`.

Capability table (replaces the `CancelOrder` column of old 5.3; the other
columns are unchanged):

| Stage | MarketDataRead | TradingRead | RecordDryRun | SubmitOrder | CancelOrder |
|---|---|---|---|---|---|
| Backtest | no | no | no | no | yes |
| Ingest | yes | yes | no | no | yes |
| DryRun | yes | yes | yes | no | yes |
| FirstOrder | yes | yes | yes | yes | yes |
| Paper | yes | yes | yes | yes | yes |

Checklist ids, in order (thirteen): `api_keys`, `calendar`, `bars`,
`stage_ingest`, `stage_dry_run`, `alert_webhook`, `account_flat`,
`first_order_approval`, `stage_first_order`, `first_order_sent`,
`strategy_allowlist`, `stage_paper`, `validation_signoff`. `first_order_sent`
is done when `Facts::first_order_sent` is true. `Facts` is unchanged.

### 5.4 `pt-store`

Migration `0005_fixes.sql`, additive:

```sql
ALTER TABLE orders ADD COLUMN off_ledger INTEGER NOT NULL DEFAULT 0 CHECK (off_ledger IN (0, 1));
ALTER TABLE backtest_runs ADD COLUMN initial_capital TEXT;
ALTER TABLE backtest_runs ADD COLUMN capital_source TEXT;
CREATE TRIGGER asset_snapshots_no_update BEFORE UPDATE ON asset_snapshots ...;
CREATE TRIGGER asset_snapshots_no_delete BEFORE DELETE ON asset_snapshots ...;
```

```rust
pub enum StoreError { /* existing */ Invalid(String) }

impl Store {
    // market.rs: same signatures, new errors
    //   upsert_bars: two different bars for one key in a batch -> Duplicate;
    //                a different bar with an earlier fetched_at -> Invalid.
    //   put_asset_snapshot: identical repeat -> Ok, nothing written;
    //                       different repeat -> Duplicate.

    // trading.rs
    pub fn mark_off_ledger(&self, client_order_id: &str) -> Result<(), StoreError>;
    pub fn is_off_ledger(&self, client_order_id: &str) -> Result<bool, StoreError>;
    /// The fill and its ledger entry in one transaction. False, and nothing
    /// written, when the activity id is already stored.
    pub fn record_fill(&self, fill: &FillRow, entry: &LedgerEntry) -> Result<bool, StoreError>;
    /// The decision and its orders in one transaction.
    pub fn insert_decision_with_orders(&self, d: &DecisionRow, orders: &[OrderRow]) -> Result<i64, StoreError>;
    //   set_order_state: filled_qty above the order quantity, or below the
    //                    stored filled_qty -> Invalid.
    //   insert_fill: false only for a repeated activity id.

    // ops.rs
    pub fn save_backtest_with_capital(
        &self, family: &str, trial_hash: &str, config_hash: &str,
        initial_capital: Decimal, capital_source: &str,
        equity: &[(NaiveDate, Decimal)], at: DateTime<Utc>,
    ) -> Result<i64, StoreError>;
    pub fn backtest_capital(&self, id: i64) -> Result<Option<(Decimal, String)>, StoreError>;
    /// Existing; added by a replan and listed here for the first time (G-63).
    pub fn backtest_family(&self, id: i64) -> Result<String, StoreError>;
    //   clear_halt: an already cleared halt -> NotFound with "already cleared".
}
```

`OrderRow` does not gain a field: `off_ledger` is read and written only
through the two methods, so no existing order literal changes.

### 5.5 `pt-alpaca`

```rust
pub enum AlpacaError { /* existing */ Invalid(String) }

impl AlpacaError {
    /// True only for `Rejected` with status 403 or 422.
    pub fn is_order_reject(&self) -> bool;
}

impl RateLimiter {
    pub fn new(max: u32, clock: Arc<dyn MonoClock>, sleeper: Arc<dyn Sleeper>)
        -> Result<RateLimiter, AlpacaError>;          // 0 -> Invalid
}

impl HttpClient {
    /// Sleeps through the injected sleeper, so callers that wait are testable.
    pub fn sleep(&self, d: Duration);
}
```

`asset(http, symbol)` and `cancel_order(http, id)` percent-encode the path
segment with the helper `orders_read.rs` already has.

### 5.6 `pt-mock-alpaca`

```rust
impl MockAlpaca {
    /// The next `n` requests with this method whose path starts with `path`
    /// are answered with `status` and a JSON message. They are recorded and
    /// change no state. Other routes are not affected.
    pub fn reject_next(&self, method: &str, path: &str, n: u32, status: u16);
    /// The next `POST /v2/orders` is answered with `status`, but the order is
    /// kept as if it had been accepted.
    pub fn reject_but_keep_next_order(&self, status: u16);
    /// The `multiplier` the account reports; `2` until set.
    pub fn seed_account_multiplier(&self, multiplier: &str);
}
```

### 5.7 `pt-data`

```rust
/// Universe symbols with no stored bar at all, in universe order.
pub fn symbols_without_bars(store: &Store, cfg: &Config) -> Result<Vec<Symbol>, DataError>;
```

### 5.8 `pt-strategy`

```rust
/// `n` weights, newest first, proportional to delta^i with
/// delta = com / (1 + com), normalised to sum to 1.
pub fn ewma_weights(n: usize, com: f64) -> Vec<f64>;

pub const ID: &str = "tsmom-v1";          // in tsmom.rs
pub const ID: &str = "bh-volscaled-v1";   // in benchmark.rs
```

`estimate_risk` gains the ineligibility reason `non_finite`. `Tsmom` with no
lookback returns `StrategyError::Invalid` from `target_weights`.

### 5.9 `pt-stats`

```rust
pub fn n_combinations(partitions: usize) -> Result<u64, StatsError>;
```

Odd or zero `partitions` is `InvalidPartition`; a result above `u64::MAX` is
`OutOfDomain`. `ols`, `psr`, `deflated_sharpe`, `pbo_cscv` and `norm_inv_cdf`
return `OutOfDomain` for a non-finite input, and `ols` also when its result
would not be finite. The plain functions (`mean`, `std_sample`, `skewness`,
`kurtosis`, `sharpe`, `annualized_sharpe`, `annualized_vol`, `max_drawdown`,
`cagr`) return NaN when an input is NaN.

### 5.10 `pt-live`

```rust
// adopt.rs
/// For a stored `Rejected` order: asks the broker for that client order id.
/// If the broker holds it in any status but `rejected`, the row becomes
/// `Submitted` with the broker's id and true is returned. Sends no order.
pub fn adopt_if_held(ctx: &LiveContext, client_order_id: &str) -> Result<bool, LiveError>;

// halts.rs
/// Sets a halt and raises the critical `halt_set` alert. Shared by
/// reconcile, settle and the equity checks.
pub fn set_halt_with_alert(ctx: &LiveContext, reason: &str, detail: &str) -> Result<(), LiveError>;

// slots.rs
/// True when a weekday slot (08:00, 09:45 or 17:30 in America/New_York) lies
/// in (last_started, now], or when there is no earlier run.
pub fn slot_due(last_started: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool;

// killswitch.rs
pub struct FlattenReport { pub cancelled: usize, pub accepted: usize, pub rejected: usize }
pub fn cancel_open(ctx: &LiveContext) -> Result<usize, LiveError>;          // now in every stage
pub fn flatten(ctx: &LiveContext) -> Result<FlattenReport, LiveError>;      // cancel, settle, close

// first_order.rs
/// Whether a first order has been accepted by the broker: a stored order
/// whose id starts with `pt-first-` in Submitted, PartiallyFilled, Filled,
/// Canceled or Expired.
pub fn first_order_accepted(store: &Store) -> Result<bool, LiveError>;

pub enum LiveError { /* existing */ FirstOrderExhausted }
```

`SettleReport::legs_released` becomes the number of leg-2 intents released by
this call. `LiveContext` is unchanged.

Client order ids: a first order is `pt-first-<valid_on>-<symbol>`, and each
further attempt for that day and symbol takes the next unused
`-r1`, `-r2`, ... A flatten order is `pt-flatten-<yyyymmdd>-<symbol>` with the
same suffix rule (unchanged).

Alert codes and their one level:

| Level | Codes |
|---|---|
| `critical` | `halt_set`, `broker_error`, `feed_error`, `run_failed`, `first_order_exhausted` (new) |
| `warn` | `order_rejected`, `order_abandoned`, `incomplete_fill`, `data_not_ready`, `late_rebalance_skipped`, `not_tradable`, `missed_run`, `first_order_ignored` (new), `asset_missing` (new) |

### 5.11 `pt-shadow`

`Fidelity::n_rebalances` counts paper decisions with status `planned`.
`fidelity` on a store with no calendar returns a `Fidelity` whose every field
is zero.

### 5.12 `pt-ops`

```rust
pub struct Pnl {
    pub day: Option<Decimal>,
    pub mtd: Option<Decimal>,
    pub ytd: Option<Decimal>,
    pub inception: Option<Decimal>,
}
```

A value is `None` when its base equity is 0, and `null` in `status.json`.
The alerter scrubs `code` as it scrubs `message`, and its HTTP agent follows
no redirect.

`status.json` top-level keys, now a contract: `generated_at`, `stage`, `feed`,
`basis`, `last_run`, `equity`, `cash`, `gross_exposure`, `pnl`, `positions`,
`last_rebalance`, `active_halts`, `trial_count`, `fidelity`,
`validation_signoff`, `disclaimer`. Nested keys: `last_run` has `id`, `stage`,
`status`, `started_at`, `finished_at`; `pnl` has `day`, `mtd`, `ytd`,
`inception`; each of `positions` has `symbol`, `qty`, `value`;
`last_rebalance` has `signal_date`, `trade_date`, `status`, `created_at`; each
of `active_halts` has `reason`, `detail`, `set_at`; each of `fidelity` has
`label`, `value`. The `fidelity` labels are the ten field names of `Fidelity`
in declaration order.

### 5.13 `pt-validate`

```rust
pub enum CapitalSource { Flag, Account, Config }      // as_str: flag, account, config

pub fn run_and_record_with(
    store: &Store, cfg: &Config, req: &BacktestRequest,
    source: CapitalSource, clock: &dyn Clock,
) -> Result<BacktestOutcome, ValidateError>;
```

`run_and_record` stays and is `run_and_record_with` with
`CapitalSource::Config`. `trial_hash` no longer depends on
`req.initial_capital`.

### 5.14 CLI

| Command | Change |
|---|---|
| `trader run [--if-due]` | With `--if-due`, exits 0 without a request, a run row or a report when no slot is due. Takes the run lock before it opens the store. Writes the report even when the run log cannot be written. |
| `trader cancel-open --yes` | Runs in every stage. `--yes` is checked before keys are loaded. Prints the number of cancels sent. |
| `trader flatten --yes` | Runs in every stage. Prints `cancelled N`, `accepted N` and `rejected N`, one per line. Exits 1 when `rejected` is above 0. |
| `trader backtest run [--capital N]` | Capital from the flag, else the account's equity when keys are set and the stage is `ingest` or higher, else `[backtest] initial_capital`. Prints `capital N` and `source flag\|account\|config`. |
| `trader backtest crosscheck [--capital N]` | Capital from the flag, else the config value. Never reads the account. |
| `trader gates` | Thirteen lines. `first_order_sent` comes from the store; `alert_webhook` also sees `PT_ALERT_WEBHOOK_URL`. |
| `trader data check` | Also prints `SYMBOL no-bars` for a universe symbol with no stored bar, and exits 1. |
| `trader report`, `trader run` | The `fidelity` section of `status.json` is filled. |

Exit codes are unchanged: 0 success, 1 failure, 2 halted, 3 lock held, 4 stage
forbids. The kill-switch commands no longer use exit 4.

### 5.15 Deploy examples

`deploy/cron.example` has one entry, `*/15 * * * *`, invoking
`trader run --if-due`, and no `CRON_TZ` line.
`deploy/launchd/com.paper-trader.run.plist.example` passes `run` and
`--if-due` and has four `StartCalendarInterval` entries, at minutes 0, 15, 30
and 45.

## 6. Decisions

Decisions are numbered F-01 onward so they cannot be confused with the old
D-numbers. "Owner" marks a choice the owner made in `decisions.md`; the rest
are this design's.

**F-01. The kill switch gets its own authorisation (owner, G-53).**
`Gate::authorize_kill_switch` issues a permit for an order that can only
shrink a position the broker reports. Rejected: a flag on `AuthContext` that
skips the stage check, because the caller would then assert "risk reducing"
without proof, and the same `authorize` path would serve both the daily cycle
and the emergency command. `CancelOrder` becomes allowed in every stage in the
capability table instead of being skipped in the kill switch, so the table
stays the one place that says what a stage may do. The `pt-flatten-` prefix
check keeps the daily cycle from ever using this door.

**F-02. `flatten` reads positions from the broker, in every stage.** It
already did; in stages below `first_order` there is no ledger to read. If the
ledger and the broker disagree, `flatten` closes what the broker holds and the
next run's reconciliation reports the difference. Accepted: the kill switch is
for emergencies and must not depend on the books being right.

**F-03. Off-ledger orders are a column, not a timing rule.** A flatten order
sent while the paper ledger is empty is marked `off_ledger`. `settle` updates
its state and records neither a fill row nor a ledger entry for it, so the
opening balance written later from the flat account is correct, and the
slippage and fidelity statistics do not count an order from before the paper
book existed. Rejected: comparing fill times with the opening balance, because
the broker's activity feed can lag. Rejected: a field on `OrderRow`, because
every order literal in the tests would change.

**F-04. `flatten` is cancel, settle, close in one call (owner, G-81) and
reports three counts (owner, G-54).** `accepted` counts orders the broker
accepted or was found already holding in this call; `rejected` counts orders
answered with 403 or 422. The return type changes from `usize` to
`FlattenReport`; the one caller is the CLI.

**F-05. Only 403 and 422 on the order POST are a broker reject (G-73).**
`AlpacaError::is_order_reject` is the single definition, used by submit,
flatten and the first order. Any other error, a 401 included, fails the call
and leaves the row `Intent`, so the next run looks the id up and sends or
adopts it. Rejected: a new `AlpacaError::Unauthorized` variant, which would
change every match on `Rejected` for no gain.

**F-06. Adoption before a retry (owner, G-55) needs Rejected to Submitted.**
`adopt_if_held` is the one helper for flatten and the first order. It needs
the mock to answer a POST with an error and still keep the order, hence the
second mock hook.

**F-07. First-order approval identity is (symbol, side, qty, valid_on)
(owner, G-92 and answer 1).** `qty` is required; an owner file without it
fails to parse. The order is planned for `qty`, and the gate authorises
exactly `qty`. Attempts are counted from stored rows: the rows whose id has
the stem `pt-first-<valid_on>-<symbol>` and whose side and quantity equal the
approval's. Six rejected rows make the approval dead.

**F-08. Retry ids take the next unused suffix (conflict 4).** The first
attempt of the first approval is the bare stem and its retries are `-r1` to
`-r5`, as decided. An approval edited on the same day for the same symbol
continues at the next unused suffix. Rejected: putting side and quantity in
the id, which would change the id format existing tests and stored rows use.

**F-09. Retries wait 1, 2, 4, 8 and 16 seconds through the HTTP client's
sleeper.** `HttpClient::sleep` exposes the sleeper the client already holds.
Rejected: a sleeper field on `LiveContext`, which is built by literal in
several tests and both CLI commands.

**F-10. An exhausted approval fails the run through the existing failure
path.** `first_order` returns `LiveError::FirstOrderExhausted`; `run_cycle`'s
failure handler maps it to the code `first_order_exhausted`, so exactly one
critical alert is raised, the run is `Failed`, and no halt is set. A later run
with the same dead approval sends nothing and raises no further alert.

**F-11. "Sent" means accepted (owner, answer 1).** A first order is used up
by a row in Submitted, PartiallyFilled, Filled, Canceled or Expired.
`first_order_accepted` is that one definition, shared by the cycle and by
`trader gates`. Once it is true, a `[first_order]` valid for the trade session
that differs from the accepted order raises the warning `first_order_ignored`
and the run carries on.

**F-12. The thirteenth checklist item uses the existing `Facts` field
(G-25).** The CLI fills `first_order_sent` from the store, as it already fills
`calendar_loaded` and `bars_loaded`. For G-21 the CLI applies
`PT_ALERT_WEBHOOK_URL` to its copy of the owner file before it asks for the
checklist; `pt-gate` still reads no environment.

**F-13. The schedule fires every fifteen minutes and the binary decides
(owner, G-05).** `slot_due` is a pure function of two instants, so daylight
saving is tested with fixed instants. "Last started run" is the newest row of
`runs`, failed runs included: a failed 08:00 run is not retried until 09:45.
With `--if-due` and nothing due, the command takes the lock, opens the store,
logs one JSON line and exits 0; a held lock is still exit 3. Slots are
weekdays, not sessions: a run on a market holiday behaves as it does today.
launchd uses `StartCalendarInterval`, which fires once after a wake, not
`StartInterval`, which does not.

**F-14. Capital is not part of a trial (owner, answer 3; amends old D-26).**
`trial_hash` writes the fixed line `initial_capital=100000` where it used to
write the request's capital. Every hash registered so far was made at 100000,
the only value the CLI could pass, so all of them stay valid, and capital no
longer changes the hash. Rejected: dropping the line, which would change every
registered hash. The capital and its source are stored on the run in two
nullable columns; runs saved before this design read back as `None`.

**F-15. Capital source order (owner, G-60 and answer 2).** Flag, then account
equity when keys are set and the stage is `ingest` or higher, then the config
value. A failed account read falls back to the config value with a warning.
`run_and_record_with` carries the source; `BacktestRequest` is unchanged so no
request literal changes. `backtest crosscheck` never reads the account.

**F-16. Store defects become typed errors that store nothing** (G-10, G-11,
G-14), with one new variant `StoreError::Invalid`. `Duplicate` is kept for a
key that is already taken. G-12 makes `put_asset_snapshot` a plain insert
after a comparison; the triggers are a second line of defence, since
`INSERT OR REPLACE` does not fire delete triggers. G-15 keeps `NotFound` with
a distinct message, so callers that match on the variant are unaffected.

**F-17. Two store writes become one transaction each (G-75, G-76).** With
`record_fill` and `insert_decision_with_orders` the re-add fallback in
`settle` and the backfill from `outputs_json` in `rebalance` are dead code and
are removed.

**F-18. Leg 1 that ends short abandons leg 2 (owner, G-56).** When a leg-1
order ends Expired or Canceled with any shares unfilled, the `incomplete_fill`
warning is raised (when some shares did fill), the leg-2 intents of that
decision become Abandoned with `order_abandoned`, and no halt is set. The
shares leg 1 traded stay in the ledger.

**F-19. `legs_released` counts events (G-80).** It is the number of leg-2
intents released by this `settle` call, so a second call with nothing new
reports 0.

**F-20. Three alert levels, one level per code (G-72, conflict 2).** The
table in 5.10 is the contract. No `error` level is added.

**F-21. Pins, not changes.** These behave today as the owner accepted, and get
a requirement and a test only: G-01 (creation event at 00:00 UTC of the trade
date), G-02 (rejection reasons written as their variant names), G-03, G-04,
G-16 (second precision, insertion order breaks ties), G-26, G-27, G-30, G-33
(`>= 2`), G-34, G-35, G-50, G-51, G-52, G-57, G-58, G-59, G-61, G-62 (the
stored `config_hash` is not checked), G-63, G-77 (`data_asof` is the clock
instant of the decision), G-79, G-83, G-90, G-91, G-93.

**F-22. Smaller fixes.** G-17: `check_gaps` is unchanged and a new function
reports symbols with no bars, so the gap type does not grow an optional date.
G-22: P&L from a zero base is absent. G-32: `start > end` is
`EngineError::Invalid`. G-36: non-finite volatility is ineligible. G-37: no
lookback is `StrategyError::Invalid`. G-38 to G-41: section 5.9; a perfect fit
in `ols` is `OutOfDomain`, because its t-statistic is not finite. G-42: a
limit of 0 is `AlpacaError::Invalid`. G-74: a calendar with no session before
the trade session is `LiveError::Invalid`, where the staleness check used to
be skipped. G-78: the symbol is still left out, with the warning
`asset_missing`. G-82: only `planned` decisions are rebalances. G-94: the
three sign-off minimums must be at least 1. G-99: all-zero fidelity; the
special case in `signoff report` goes.

**F-23. `trader run` report and lock order (G-96, G-97).** The lock is taken
before the store is opened, so a locked-out run does not migrate or write. The
report is written after `run_cycle` whatever it returned, and the error still
decides the exit code.

**F-24. G-71 is pinned in `pt-data` (conflict 3).** The raw refetch starts
five sessions before the earliest of the symbols' latest stored raw bars, and
at `data.start` when a symbol has none.

**F-25. G-31 is pinned; the tie at a page boundary is accepted** (see Gaps).

**F-26. Plan shape: small scaffold chains first.** Changes that cross crate
boundaries at compile time (a required field, a changed signature, a new
module line, the migration) go into per-crate scaffold chains in the first
wave, so the chains behind them touch disjoint files and run in parallel.
`pt-cli` gets a refactoring chain that moves each command's arguments and
shared helpers into their own modules, so later CLI chains do not meet in
`main.rs` or `cli.rs`.

**F-27. R-003 is restated in SPDX terms (conflict 1).** A licence expression
is allowed when it is an `OR` (or `/`) list with at least one allowed
alternative, where an alternative is an `AND` list whose every part is
allowed, and a part is allowed when it contains neither `GPL` nor `SSPL`. A
package with a licence file and no expression must be on an explicit list in
the test, which is empty today.

**F-28. Hygiene is tested by scanning source (conflict 5).** `pt-archtest`
already holds policy tests of this kind. One scan checks that every manifest
dependency is used; another checks that the helpers this design deduplicates
stay single. Rejected: leaving housekeeping untested, which is how the
duplicates came back in each wave.

**F-29. Not built: notes for the owner or integrator.** `check.sh` should log
the failing test's name and write one log per run; each worktree, reviewer and
integrator should have its own `CARGO_TARGET_DIR`; the remote
`integration/paper-trader` is stale and cannot be updated without a force
push; the three flaky tests have no known cause; the client ignores
`data.max_requests_per_minute`. None of these is inside what a chain may
touch, or inside the scope the owner set.

## 7. Testing

Every requirement has an acceptance check, and every step's Proof runs the
test file that asserts it. Levels:

| Component | How it is proven |
|---|---|
| W | `pt-archtest` policy tests over `cargo metadata`, the manifests and the source tree. |
| A, C, H, I, J, K, L, M | Unit-level integration tests on pure functions with literal inputs and expected values. |
| D | Integration tests on an in-memory store; the migration test builds a version-4 database from the first four migration files and opens it. |
| E | Integration tests against a loopback server with a recording sleeper. |
| F | Integration tests of the mock itself over loopback. |
| G | Integration tests with an in-memory store and the mock. |
| N | Unit-level tests of `authorize`, `authorize_kill_switch`, `allows` and `gate_status`; the `compile_fail` doctest for the permit stays. |
| O | Integration tests with the live fixture: in-memory store, mock broker, fake clock, recording sleeper where waits are asserted. `slot_due` is tested with fixed instants on both sides of both daylight-saving changes. |
| P, Q, V | Integration tests with an in-memory or temporary store; the alerter against a loopback listener. |
| R | End-to-end tests that run the built `trader` binary with a scrubbed environment, a temporary store and the mock broker. |

Pins are tests of today's behaviour with no source change; their steps touch
test files only.

The integration check is CHECK from `.devteam/config`: the format check,
clippy with warnings denied, every workspace test and the build. It must pass
after every step and after every merge into the integration branch. The licence and
unused-dependency tests run inside it, so the new dependency is checked the
moment it is added.

## 8. Open questions

none

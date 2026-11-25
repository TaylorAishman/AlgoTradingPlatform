# paper-trader: architecture

Stage 1 of the design. It is built from `docs/reference/`, `decisions.md` and the repository as it stands (no product code yet). `requirements.md` and `plan.md` are built from this file.

## 1. Problem

Build a self-hosted systematic trading system in Rust for the repository owner: its own event-driven daily-bar engine, a time-series momentum strategy on a wide set of US-listed ETFs, and a validation path that ends in full paper trading on a dedicated Alpaca paper account. The system must be able to show that its backtest, its paper orders and its paper fills agree, because a few months of paper trading can validate a system but not a strategy. Everything that needs a human is an owner step that the binary checks at run time, so no build work waits on the owner.

## 2. Sources

### 2.1 Authority order

`docs/reference/README.md` still has its authority table as TODO, so the order is recorded here.

1. Owner decisions: `paper-trader-source-brief.md` section 1 and `docs/design/paper-trader/decisions.md`. These win over everything.
2. Primary sources for method detail: the PDFs in `docs/reference/trading-papers/` and the Alpaca documentation pages. Where the brief or the review disagrees with a PDF on a formula or a procedure, the PDF wins.
3. `paper-trader-source-brief.md` sections 2 to 8: findings and external facts, each with a confidence label.
4. `literature-review-for-builders.md`: guidance. Its architecture, stack and roadmap sections are advice, not decisions, because the brief leaves those to the team.

The PDFs are gitignored and absent from build worktrees. Every formula the build needs is transcribed in section 5.10 of this file.

### 2.2 What each source contributed

| Source | Contribution to this design |
| --- | --- |
| Brief section 1 | Rust, own engine, Alpaca paper, SQLite, time-series momentum first, phased roadmap, finest task grain, owner steps isolated, no live trading |
| `decisions.md` Q1 to Q8 | Basic data plan and end-of-day design; long/short with gross at most 1.0x and whole shares; dedicated $100k account with exclusive control; daily-bar ETFs only; all phases in one plan behind runtime gates; no Python, a second Rust calculator; Alpaca data from 2016; local report only |
| `decisions.md` assumptions | Universe shape, strategy parameters, sizing and risk limits, validation set, engine rules, process shape, order handling, dividends, own REST client, storage, layout, secrets, owner gates |
| Moskowitz, Ooi and Pedersen (2012), PDF pages 13 to 19 | Signal: sign of the past 12-month excess return, held one month. Volatility: exponentially weighted variance with a 60-day centre of mass, annualised by 261 (equation 1). Position proportional to 1 / volatility (equation 5) |
| Yin et al. (2026), PDF sections 3.3, 6 and appendix A | Algorithm 1 (proportional cost on traded notional); exact agreement at zero cost as the test of an accounting harness; the seven defects: commission mis-scaling, fills that reject buys funded by later sells, two calendar instances, hedging instead of netting positions, silent truncation, double-charged costs, arithmetic overflow; the advice to validate with an independent vectorised implementation |
| Bailey and Lopez de Prado (2014), PDF pages 7 to 10 | Expected maximum Sharpe ratio (equation 1), Deflated Sharpe Ratio (equation 2), and a worked example used as a golden test |
| Bailey, Borwein, Lopez de Prado and Zhu, PDF pages 11 to 13 | Combinatorially symmetric cross-validation (algorithm 2.3) and the definition of the Probability of Backtest Overfitting |
| Sato and Kanazawa (2025), PDF page 1 | Impact form I(Q) / sigma_D = c (Q / V_D)^(1/2) |
| Brief sections 2 and 3 (Kim, Tse and Wald; Huang et al.; McLean and Pontiff; Harvey, Liu and Zhu) | A volatility-scaled buy-and-hold benchmark separates trend from volatility scaling; parameters are fixed in advance and every trial is counted |
| Brief section 4 and Alpaca docs opened 2026-11-23 | Paper behaviour, plan limits, endpoints and schemas listed in section 5.9 |
| Review, "paper trading and validation" | Pass condition is agreement with a shadow backtest: signals and orders must match exactly, fill prices and end-of-day positions are reconciled, the paper-minus-shadow daily return must be small and unbiased; failure drills; heartbeat and kill switch |
| Review, "architecture" and "Rust-first stack" | One strategy core for backtest and paper; a risk gate no strategy can bypass; an append-only ledger as the source of P&L; crate boundaries, an exhaustive order-state enum, decimal money, an injected clock |

### 2.3 Conflicts

None of these is resolved silently. Each row says which side the design takes and under which rule.

| # | Conflict | Resolution |
| --- | --- | --- |
| 1 | Review proposes Parquet with Polars or DuckDB for research and PostgreSQL for the ledger. Brief section 1 and `decisions.md` say SQLite for everything | SQLite. Owner decision wins |
| 2 | Review says adopting an existing engine saves the hardest work and suggests Nautilus or Barter as a second engine. Brief section 1 says the team writes its own; Q6 says the cross-check is a second Rust calculator | Own engine plus own vectorised calculator. Owner decision wins |
| 3 | Review recommends HAR on realised volatility for the volatility target. `decisions.md` fixes the Moskowitz estimator | Exponentially weighted estimator. HAR needs intraday data the Basic plan design does not use; it is on the roadmap |
| 4 | Review recommends tokio at the edges and sqlx. The design has no websocket and one process run to completion | Blocking HTTP (`ureq`) and `rusqlite`. Team decision; the review is guidance (decision D-07) |
| 5 | Review lists fractional Kelly sizing among the four risk controls. `decisions.md` lists a volatility target, a weight cap, a gross cap, a maximum order notional and a daily-loss halt | The `decisions.md` set, plus the review's drawdown halt. No Kelly sizing: there is no expected-return estimate to feed it |
| 6 | Review roadmap starts by replicating a Kenneth French series and puts the trial log last. Q7 drops the AQR comparison; `decisions.md` records trials from the start | No replication target. The trial registry is built with the first backtest |
| 7 | Review's cost model is half the quoted spread plus fees plus impact. The Basic plan design uses no quotes | The half-spread is a configured number of basis points, checked against measured paper slippage |
| 8 | Yin et al. Algorithm 1 trades at the same close it marks at, with fractional shares and cost deducted before allocation. Review says orders must fill at the next available price; Q2 and Q3 require whole shares | Fill at the next session's open, whole shares, cost charged per fill against cash. The arithmetic of Algorithm 1 (cost = rate x traded notional) is kept. Both of our implementations follow the adapted specification in 5.10.5 |
| 9 | Moskowitz sizes each instrument to 40% volatility with futures leverage. `decisions.md` sets a 10% portfolio target and Q2 caps gross exposure at 1.0x | Portfolio target with caps. The caps will often bind, so realised volatility will sit below the target. This is an adaptation, not a replication, as the brief already says |
| 10 | The overfitting paper prints 12,780 combinations for S = 16. The binomial coefficient C(16, 8) is 12,870 | 12,870, the computed value |
| 11 | Annualisation constants differ: 261 in Moskowitz, 250 in the Deflated Sharpe example | 261 inside the volatility estimator only (fixed by `decisions.md`). Performance statistics use 252 periods per year, a config value. The Deflated Sharpe Ratio itself is computed on per-period returns and needs no constant |
| 12 | Alpaca's asset schema defines `borrow_status`; its example shows a boolean `easy_to_borrow` | The parser accepts either and treats an asset as easy to borrow if either says so |
| 13 | Brief 4.1 says paper-only accounts "receive and use only IEX market data". The bars reference defaults `feed` to `sip`, and the plan table limits Basic historical data only by "latest 15 minutes" | Not settled by documents. `data.feed` is a config value (default `sip`), every stored bar records its feed, feeds are never mixed in one run, and `trader doctor` probes with real keys. If only `iex` works, the consequences in 3.3 apply |
| 14 | Q2 requires a borrow-cost model. Alpaca's margin page says easy-to-borrow shares carry $0 borrow fees for Trading API users, and paper charges none | The research cost profile charges a configured annual borrow rate so backtests are not flattered. The shadow profile charges none, to match what paper does |
| 15 | Brief 4.3 says fractional orders are market-only | Outdated per `decisions.md`. It does not matter: there is no fractional path |

### 2.4 Gaps

- The brief cites Hurst et al., Lemperiere et al., Kim et al., Huang et al., Moreira and Muir, Faber and DeMiguel et al. Their PDFs are not in `trading-papers/`. Nothing in the design depends on a formula from them.
- No fixture can be recorded from the real API, because agents never hold keys. Fixtures are written by hand from the documented schemas. `trader doctor` runs the same parsers against the real API once the owner supplies keys.
- Six facts can only be confirmed with real keys. Each is guarded by a hard error or a conservative rule, so none blocks the build: which feed Basic returns for daily bars; the timestamp convention of daily bars; which candidate ETFs are tradable, easy to borrow and have history from 2016; whether one order may cross from long to short (forum reports say it is rejected, so the design never sends one); when a market day order sent before the open fills on paper; whether `easy_to_borrow` is still present.

## 3. Constraints

### 3.1 Fixed by the owner

- Rust only. No Python anywhere, including tests and tooling.
- SQLite is the only store.
- Alpaca paper trading is the only venue. The binary cannot be configured to reach a live endpoint and never handles live keys.
- Daily bars of US-listed ETFs only. No crypto, options, single stocks, intraday bars or order-book data.
- Long and short, gross exposure at most 1.0x equity, whole shares, shorts only in instruments that are shortable and easy to borrow.
- The paper account is dedicated to this system. Any order or position the system did not create halts it.
- The trading machine holds no push credentials. Reports are local files.
- The repository is public and MIT licensed. No GPL, LGPL or AGPL dependency (this rules out `apca`). No PDF is ever staged.

### 3.2 External limits

- Basic plan: 200 historical data requests per minute, history from 2016, the latest 15 minutes excluded. The client limits itself to 180 per minute.
- Paper fills are simulated from quotes, ignore quoted size, and are randomly partial 10% of the time. Paper simulates no dividends and charges no borrow fees.
- Alpaca numeric fields on the trading API are JSON strings. Bar prices on the data API are JSON numbers.

### 3.3 If Basic returns only IEX bars

- Closes are the last IEX trade, not the consolidated close, and volume is IEX volume only.
- Config validation then requires `impact_coeff = 0`, because the impact term needs consolidated volume.
- Thinly traded ETFs can have sessions with no IEX bar. `trader universe verify` fails any candidate with a gap, and the engine never fills a gap silently.
- Paper fills still come from consolidated quotes, so measured slippage against an IEX open will be noisier. The sign-off report states the feed.

### 3.4 Build prerequisites

- Rust 1.100.0, pinned in `rust-toolchain.toml` (`rusqlite` 0.43 needs 1.89 or later). The machine this design was written on has rustup with 1.83.0 installed and `~/.cargo/bin` missing from the non-interactive `PATH`. Both must be fixed before the first chain runs; rustup installs the pinned toolchain on first use if it can reach the network.
- A C compiler, for the bundled SQLite and for `ring`.
- `CHECK` in `.devteam/config` runs the Rust checks (section 7.4) as soon as a root `Cargo.toml` exists. Until then it only guards README.md and tracked PDFs, so each step's Proof command is the real gate.

### 3.5 Must not change

- `docs/reference/`, `REVIEW.md`, `.devteam/` and `.github/workflows/` are not touched by any chain.
- `CHECK` keeps its rule that no PDF is tracked.

## 4. Components

One Cargo workspace, `members = ["crates/*"]`. Each component is one crate and owns everything under its directory. A crate's `lib.rs` holds only `pub mod` lines and is written once by the scaffold, so chains that run side by side own separate module files. Items are addressed by module path (`pt_core::types::Symbol`).

| Prefix | Crate (directory) | Responsibility | Depends on |
| --- | --- | --- | --- |
| W | workspace root, `pt-archtest` (`crates/archtest`) | `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.gitignore`, `.gitattributes`, crate skeletons. Policy tests: member list, licences, dependency boundaries, forbidden strings | none |
| A | `pt-core` (`crates/core`) | Domain types: symbol, decimal price and money, bar, order intent, cost specification, order states, signal and risk-estimate types, market-view traits, clock | none |
| B | `pt-calendar` (`crates/calendar`) | The one trading calendar type, month-end logic, rebalance schedule | none |
| C | `pt-config` (`crates/config`), `config/` | Typed TOML config with validation and a stable hash, the example config and candidate universe, API key loading | core |
| D | `pt-store` (`crates/store`) | SQLite schema, migrations, repositories for market data, trading records and operations records | core, calendar |
| E | `pt-alpaca` (`crates/alpaca`) | Thin blocking REST client: endpoint lock, auth, rate limiter, retries, market data, trading reads, order submission | gate (permit type only) |
| F | `pt-mock-alpaca` (`crates/mock-alpaca`) | In-process mock of the Alpaca data and trading APIs for tests and failure drills | none |
| G | `pt-data` (`crates/data`) | Bar ingest, CSV import, calendar and asset sync, data quality checks, universe verification, the real-API smoke test, the store-backed market view | core, calendar, config, store, alpaca |
| H | `pt-strategy` (`crates/strategy`) | `Strategy` trait, volatility and covariance estimators, time-series momentum, the buy-and-hold benchmark | core, calendar |
| I | `pt-portfolio` (`crates/portfolio`) | Target weights from signals and risk estimates; whole-share targets and the order plan | core |
| J | `pt-risk` (`crates/risk`) | Pre-trade order checks; equity-based halts | core |
| K | `pt-engine` (`crates/engine`) | Cost functions, the book, the decision function shared by backtest and paper, the simulated broker, the backtest loop | core, calendar, config, strategy, portfolio, risk |
| L | `pt-vector` (`crates/vector`) | The second, deliberately simple calculator, written from the specification in 5.10.5 only | core |
| M | `pt-stats` (`crates/stats`) | Descriptive statistics, the normal distribution, Deflated Sharpe Ratio, Probability of Backtest Overfitting, simple regression | none |
| N | `pt-gate` (`crates/gate`) | Owner file, operating stages, capability checks, the order permit, the owner checklist | core |
| O | `pt-live` (`crates/live`) | Order identifiers and state mapping, idempotent submission, settlement from fills, reconciliation, equity snapshots, rebalance decisions, the run cycle, the first-order flow, kill switch | core, calendar, config, store, alpaca, gate, strategy, engine, risk, data, ops |
| P | `pt-shadow` (`crates/shadow`) | Decision replay, the daily shadow simulation, slippage, fidelity metrics, the sign-off evaluation | core, calendar, config, store, data, strategy, engine, stats |
| Q | `pt-ops` (`crates/ops`) | Alerts (log, database, webhook), structured logging, the status and P&L report | core, config, store |
| V | `pt-validate` (`crates/validate`) | Backtest runner with trial registration, cross-check of engine against vector, DSR and PBO over the registry, benchmark comparison | core, calendar, config, store, data, strategy, engine, vector, stats |
| R | `pt-cli` (`crates/cli`), `deploy/` | The `trader` binary: argument parsing and wiring only. Example schedules | everything |

Module files. The scaffold creates every file below, empty except for `lib.rs` (the `pub mod` lines) and `error.rs` (the full enum from 5.11). Later chains fill them in and add tests; they do not edit `lib.rs`, `error.rs`, any `Cargo.toml` or `Cargo.lock`.

| Crate | Modules under `src/` |
| --- | --- |
| `pt-core` | `types`, `money`, `cost`, `order_state`, `signal`, `view`, `clock`, `error` |
| `pt-calendar` | `calendar`, `schedule`, `error` |
| `pt-config` | `config`, `hash`, `secrets`, `error` |
| `pt-store` | `db`, `codec`, `market`, `trading`, `shadow`, `ops`, `error`; SQL in `crates/store/migrations/` |
| `pt-alpaca` | `endpoints`, `auth`, `limiter`, `http`, `data`, `account`, `reference`, `orders_read`, `activities`, `orders_write`, `error` |
| `pt-mock-alpaca` | `server`, `state`, `routes` |
| `pt-data` | `ingest`, `quality`, `csv_import`, `reference`, `universe`, `doctor`, `view`, `error` |
| `pt-strategy` | `vol`, `estimate`, `strategy`, `tsmom`, `benchmark`, `error` |
| `pt-portfolio` | `weights`, `orders`, `error` |
| `pt-risk` | `pretrade`, `halts` |
| `pt-engine` | `cost`, `book`, `decision`, `broker`, `simulate`, `schedule`, `error` |
| `pt-vector` | `vector`, `error` |
| `pt-stats` | `descriptive`, `perf`, `normal`, `dsr`, `pbo`, `ols`, `error` |
| `pt-gate` | `owner`, `gate`, `status`, `error` |
| `pt-live` | `ids`, `status`, `context`, `submit`, `settle`, `reconcile`, `snapshot`, `rebalance`, `cycle`, `lock`, `first_order`, `killswitch`, `error` |
| `pt-shadow` | `replay`, `daily`, `slippage`, `fidelity`, `signoff`, `error` |
| `pt-ops` | `alert`, `logging`, `pnl`, `report`, `error` |
| `pt-validate` | `runner`, `crosscheck`, `overfit`, `error` |
| `pt-cli` | `main.rs`, `cli` (argument definitions), `wiring` (builds store, config, gate and client), and one module per command group: `cmd_config`, `cmd_data`, `cmd_backtest`, `cmd_validate`, `cmd_run`, `cmd_ops` |
| `pt-archtest` | no library code; tests only |

External crates. The `workspace` chain declares all of these, with versions in `[workspace.dependencies]`, so that no later chain edits a `Cargo.toml` or `Cargo.lock`. A crate also lists the workspace crates in its "Depends on" cell above. Declaring a crate that ends up unused is harmless; a missing one stops a later chain, so the scaffold errs on the side of declaring.

| Crate | Dependencies | Dev-dependencies |
| --- | --- | --- |
| `pt-core` | `rust_decimal`, `chrono`, `serde`, `thiserror` | `serde_json` |
| `pt-calendar` | `chrono`, `thiserror` | none |
| `pt-config` | `serde`, `toml`, `rust_decimal`, `chrono`, `sha2`, `thiserror` | `tempfile` |
| `pt-store` | `rusqlite` (feature `bundled`), `rust_decimal`, `chrono`, `thiserror` | `tempfile`, `rusqlite` |
| `pt-alpaca` | `ureq`, `serde`, `serde_json`, `rust_decimal`, `chrono`, `thiserror` | `pt-mock-alpaca`, `tiny_http` |
| `pt-mock-alpaca` | `tiny_http`, `serde_json`, `chrono` | `ureq`, `serde_json` |
| `pt-data` | `csv`, `rust_decimal`, `chrono`, `thiserror` | `pt-mock-alpaca`, `tempfile` |
| `pt-strategy` | `chrono`, `thiserror` | none |
| `pt-portfolio` | `rust_decimal`, `thiserror` | none |
| `pt-risk` | `rust_decimal` | none |
| `pt-engine` | `rust_decimal`, `chrono`, `serde`, `serde_json`, `thiserror` | none |
| `pt-vector` | `rust_decimal`, `chrono`, `thiserror` | none |
| `pt-stats` | `thiserror` | none |
| `pt-gate` | `serde`, `toml`, `chrono`, `thiserror` | `tempfile` |
| `pt-live` | `rust_decimal`, `chrono`, `serde_json`, `thiserror` | `pt-mock-alpaca`, `tempfile` |
| `pt-shadow` | `rust_decimal`, `chrono`, `serde_json`, `thiserror` | `tempfile` |
| `pt-ops` | `ureq`, `serde_json`, `rust_decimal`, `chrono`, `thiserror` | `tiny_http`, `tempfile` |
| `pt-validate` | `sha2`, `rust_decimal`, `chrono`, `thiserror` | none |
| `pt-cli` | `clap` (feature `derive`), `anyhow`, `rust_decimal`, `chrono`, `serde_json` | `pt-mock-alpaca`, `tempfile`, `serde_json` |
| `pt-archtest` | none | `serde_json`, `toml` |

Version requirements: `rust_decimal` 1, `chrono` 0.4, `serde` 1 (feature `derive`), `serde_json` 1, `toml` 1, `thiserror` 2, `anyhow` 1, `rusqlite` 0.43, `ureq` 3, `tiny_http` 0.12, `clap` 4, `sha2` 0.11, `csv` 1, `tempfile` 3. All are MIT, Apache-2.0 or Unlicense (checked on crates.io on 2026-11-23). There is no random-number, async or logging-framework crate.

Dependency rules that are tested (`pt-archtest`):

- `pt-strategy` cannot reach `pt-alpaca`, `pt-store`, `pt-live`, `pt-gate`, `ureq` or `rusqlite`. Strategy code cannot touch a broker or a database.
- `pt-vector` depends on `pt-core` and nothing else in the workspace. It shares no accounting code with the engine.
- `pt-stats` depends on no workspace crate.
- `pt-engine` does not depend on `pt-vector`. Only `pt-validate` sees both.

## 5. Interfaces

Signatures are Rust. `Decimal` is `rust_decimal::Decimal`; `NaiveDate` and `DateTime<Utc>` are from `chrono`. Maps are `BTreeMap` and sets `BTreeSet` so iteration order is deterministic. Each crate has one error enum in `error.rs`, built with `thiserror` and listed in 5.11.

### 5.1 `pt-core`

```rust
// types.rs
pub struct Symbol(/* private */);
impl Symbol { pub fn new(s: &str) -> Result<Symbol, CoreError>; pub fn as_str(&self) -> &str; }
pub enum Side { Buy, Sell }
impl Side { pub fn sign(&self) -> i64; }            // Buy = +1, Sell = -1
pub enum Adjustment { Raw, All }                     // Alpaca `adjustment` values "raw" and "all"
pub enum PriceBasis { Raw, Adjusted }                // which series the engine fills and marks on
pub struct Bar { pub symbol: Symbol, pub date: NaiveDate, pub open: Price, pub high: Price,
                 pub low: Price, pub close: Price, pub volume: u64 }
pub struct OrderIntent { pub symbol: Symbol, pub side: Side, pub qty: u64, pub leg: u8,
                         pub reference_price: Price }
impl OrderIntent { pub fn new(symbol: Symbol, side: Side, qty: u64, leg: u8, reference_price: Price)
                       -> Result<OrderIntent, CoreError>; }
pub struct Fill { pub symbol: Symbol, pub side: Side, pub qty: u64, pub price: Price, pub cost: Money }
pub type Weights = BTreeMap<Symbol, Decimal>;        // 6 decimal places
pub type Positions = BTreeMap<Symbol, i64>;          // signed whole shares, one entry per symbol

// money.rs
pub struct Price(/* private Decimal, > 0 */);
pub struct Money(pub Decimal);
impl Price { pub fn new(d: Decimal) -> Result<Price, CoreError>;
             pub fn from_f64_shortest(x: f64) -> Result<Price, CoreError>;
             pub fn value(&self) -> Decimal;
             pub fn notional(&self, shares: i64) -> Money; }
pub fn round6(d: Decimal) -> Decimal;                // 6 dp, midpoint away from zero

// cost.rs
pub struct CostSpec { pub spread_bps: Decimal, pub fee_bps: Decimal,
                      pub impact_coeff: Decimal, pub borrow_bps_annual: Decimal }
impl CostSpec { pub fn zero() -> CostSpec; pub fn validate(&self) -> Result<(), CoreError>; }

// order_state.rs
pub enum OrderState { Intent, DryRun, Submitted, PartiallyFilled, Filled, Canceled, Expired,
                      Rejected, Abandoned }
pub fn transition_allowed(from: OrderState, to: OrderState) -> bool;

// signal.rs
pub struct SignalSet { pub asof: NaiveDate, pub signals: BTreeMap<Symbol, f64>,
                       pub ineligible: BTreeMap<Symbol, String> }
pub struct RiskEstimates { pub asof: NaiveDate, pub vols: BTreeMap<Symbol, f64>,
                           pub cov: BTreeMap<(Symbol, Symbol), f64>,
                           pub ineligible: BTreeMap<Symbol, String> }

// view.rs
pub trait HistoryView {
    /// Adjusted closes for sessions on or before `asof`, oldest first, at most `max` of them.
    fn adj_closes(&self, symbol: &Symbol, asof: NaiveDate, max: usize) -> Vec<(NaiveDate, f64)>;
}
pub trait MarketView: HistoryView {
    fn exec_bar(&self, symbol: &Symbol, date: NaiveDate) -> Option<Bar>;   // execution basis
    fn adv_shares(&self, symbol: &Symbol, asof: NaiveDate, sessions: usize) -> Option<f64>;
}
pub struct MemoryView { /* in-memory MarketView for tests and the vector cross-check */ }
impl MemoryView { pub fn new() -> MemoryView;
                  pub fn push_exec(&mut self, bar: Bar);
                  pub fn push_adj_close(&mut self, symbol: &Symbol, date: NaiveDate, close: f64); }

// clock.rs
pub trait Clock { fn now_utc(&self) -> DateTime<Utc>; }
pub struct SystemClock; pub struct FixedClock(pub DateTime<Utc>);
```

Allowed order-state transitions: `Intent` to `Submitted`, `Rejected` or `Abandoned`; `Submitted` to `PartiallyFilled`, `Filled`, `Canceled`, `Expired` or `Rejected`; `PartiallyFilled` to `PartiallyFilled`, `Filled`, `Canceled` or `Expired`. `DryRun`, `Filled`, `Canceled`, `Expired`, `Rejected` and `Abandoned` are terminal.

### 5.2 `pt-calendar`

```rust
// calendar.rs
pub struct TradingCalendar { /* private */ }
impl TradingCalendar {
    pub fn new(sessions: Vec<NaiveDate>) -> Result<TradingCalendar, CalendarError>;
    pub fn is_session(&self, d: NaiveDate) -> bool;
    pub fn next_session(&self, d: NaiveDate) -> Option<NaiveDate>;      // strictly after d
    pub fn prev_session(&self, d: NaiveDate) -> Option<NaiveDate>;      // strictly before d
    pub fn sessions_between(&self, from: NaiveDate, to: NaiveDate) -> &[NaiveDate];  // inclusive
    pub fn calendar_days_to_next(&self, d: NaiveDate) -> Option<i64>;
    pub fn month_end_sessions(&self, from: NaiveDate, to: NaiveDate) -> Vec<NaiveDate>;
    pub fn month_end_back(&self, d: NaiveDate, months: u32) -> Option<NaiveDate>;
}
// schedule.rs
pub struct Rebalance { pub signal_date: NaiveDate, pub trade_date: NaiveDate }
pub fn monthly_schedule(cal: &TradingCalendar, from: NaiveDate, to: NaiveDate) -> Vec<Rebalance>;
```

A session is a month end only if the calendar holds a later session and that session falls in a different calendar month. `month_end_back(d, k)` is the last session of the calendar month `k` months before the month of `d`.

### 5.3 `pt-config`, `pt-gate`

Config file (TOML, unknown keys rejected). Values shown are the defaults; struct fields carry the TOML key names. The committed examples are `config/paper-trader.example.toml` and `config/owner.example.toml`.

```toml
[run]
db_path = "var/paper-trader.sqlite"
output_dir = "var/reports"

[data]
feed = "sip"                    # "sip", "iex", or "csv:<source>" for imported bars
start = "2016-01-04"
max_requests_per_minute = 180

[strategy]
id = "tsmom-v1"
lookback_months = [12]          # [1, 3, 12] gives the equal-weighted blend
risk_free_symbol = "BIL"
vol_com_days = 60
vol_annualization = 261
vol_min_obs = 126
vol_max_obs = 504

[sizing]
target_vol = 0.10
max_weight = 0.10
max_gross = 1.0                 # values above 1.0 are rejected

[risk]
max_order_notional = 25000
daily_loss_limit = 0.03
max_drawdown = 0.15
max_rebalance_delay_sessions = 5
min_minutes_before_close = 30
cash_tolerance = 0.01

[cost.research]                 # historical backtests, total-return basis
spread_bps = 5
fee_bps = 0
impact_coeff = 1.0
borrow_bps_annual = 50

[cost.shadow]                   # comparison with paper, price-return basis
spread_bps = 5
fee_bps = 0
impact_coeff = 0
borrow_bps_annual = 0

[stats]
periods_per_year = 252
pbo_partitions = 16

[signoff]
min_sessions = 63
min_rebalances = 3
min_fills = 30
max_replay_mismatches = 0
max_reconciliation_breaks = 0
max_missed_sessions = 2
max_late_rebalances = 0
slippage_tolerance_bps = 5
max_mean_diff_bps = 1
max_abs_diff_bps = 25

[[universe]]
symbol = "SPY"
asset_class = "equity"          # equity, treasury, credit, commodity, currency, real_estate
shortable = true                # research-backtest assumption; paper uses the broker's answer
```

```rust
// pt-config
// config.rs
pub struct Config { pub run: RunCfg, pub data: DataCfg, pub strategy: StrategyCfg, pub sizing: SizingCfg,
                    pub risk: RiskCfg, pub cost: CostCfg, pub stats: StatsCfg, pub signoff: SignoffCfg,
                    pub universe: Vec<Instrument> }
pub fn load(path: &Path) -> Result<Config, ConfigError>;
pub fn parse(toml_text: &str) -> Result<Config, ConfigError>;     // parse + validate
// hash.rs
pub fn config_hash(cfg: &Config) -> String;                       // lowercase hex SHA-256
// secrets.rs
pub struct Secret(/* private */);                                 // Debug and Display print "***"
impl Secret { pub fn expose(&self) -> &str; }
pub struct ApiKeys { pub key_id: Secret, pub secret_key: Secret }
pub fn load_api_keys(env: &BTreeMap<String, String>) -> Result<ApiKeys, ConfigError>;

// pt-gate
// owner.rs
pub enum Stage { Backtest, Ingest, DryRun, FirstOrder, Paper }    // ordered, serialised snake_case
pub struct FirstOrderApproval { pub symbol: String, pub side: String, pub max_qty: u64,
                                pub valid_on: NaiveDate }
pub struct OwnerFile { pub stage: Stage, pub strategy_allowlist: Vec<String>,
                       pub account_flat_confirmed_on: Option<NaiveDate>,
                       pub first_order: Option<FirstOrderApproval>,
                       pub validation_signoff: Option<String>,
                       pub alert_webhook_url: Option<String> }
pub fn load_owner_file(path: &Path, repo_root: &Path) -> Result<OwnerFile, GateError>;
// gate.rs
pub enum Capability { MarketDataRead, TradingRead, RecordDryRun, SubmitOrder, CancelOrder }
pub struct OrderSummary { pub client_order_id: String, pub symbol: String, pub side: String, pub qty: u64 }
pub struct AuthContext { pub strategy_id: String, pub today: NaiveDate, pub halted: bool,
                         pub risk_reducing: bool, pub is_first_order: bool,
                         pub first_order_already_sent: bool }
pub struct OrderPermit { /* private fields; no public constructor */ }
impl OrderPermit { pub fn matches(&self, o: &OrderSummary) -> bool; }
pub struct Gate { /* private */ }
impl Gate { pub fn new(owner: OwnerFile) -> Gate;
            pub fn stage(&self) -> Stage;
            pub fn allows(&self, cap: Capability) -> Result<(), GateError>;
            pub fn authorize(&self, order: &OrderSummary, ctx: &AuthContext) -> Result<OrderPermit, GateError>; }
// status.rs
pub struct Facts { pub keys_present: bool, pub calendar_loaded: bool, pub bars_loaded: bool,
                   pub first_order_sent: bool }
pub struct ChecklistItem { pub id: &'static str, pub done: bool, pub instruction: &'static str }
pub fn gate_status(owner: &OwnerFile, facts: &Facts) -> Vec<ChecklistItem>;
```

Files and environment:

- `PT_CONFIG` or `--config`: path of the config file.
- `PT_OWNER_FILE`: path of the owner file, default `$HOME/.config/paper-trader/owner.toml`. It must lie outside the repository. A missing file means stage `backtest`. The binary never writes it.
- `APCA_API_KEY_ID` and `APCA_API_SECRET_KEY`, or `PT_SECRETS_FILE` naming a TOML file with `key_id` and `secret_key` whose mode grants nothing to group or other.
- `PT_ALERT_WEBHOOK_URL`: overrides `alert_webhook_url` from the owner file.
- `PT_ALPACA_TRADING_URL`, `PT_ALPACA_DATA_URL`: accepted only when the host is a loopback address. They exist for tests.

Stage capabilities:

| Stage | MarketDataRead | TradingRead | RecordDryRun | SubmitOrder | CancelOrder |
| --- | --- | --- | --- | --- | --- |
| `backtest` | no | no | no | no | no |
| `ingest` | yes | yes | no | no | no |
| `dry_run` | yes | yes | yes | no | no |
| `first_order` | yes | yes | yes | the one approved order | yes |
| `paper` | yes | yes | yes | yes | yes |

The `GateError` variants are listed in 5.11.

### 5.4 `pt-store`

One SQLite file. Money and prices are stored as TEXT holding the canonical decimal string, quantities as INTEGER, dates as `YYYY-MM-DD`, instants as RFC 3339 UTC. `PRAGMA user_version` is the schema version.

| Table | Key | Notes |
| --- | --- | --- |
| `calendar_sessions` | `date` | The single calendar source |
| `bars` | `(symbol, date, adjustment, feed, fetched_at)` | Append-only. A new row is written only when values differ from the latest version |
| `asset_snapshots` | `(symbol, fetched_at)` | tradable, shortable, easy to borrow |
| `decisions` | `(strategy_id, signal_date, mode)` | mode `dry_run` or `paper`; inputs and outputs as JSON |
| `orders` | `client_order_id` | state, decision id, leg, broker order id |
| `order_events` | autoincrement | every state change |
| `fills` | `activity_id` | Append-only |
| `ledger_entries` | autoincrement | Append-only. `book`, `kind`, `symbol`, `qty`, `price`, `cash_delta` |
| `equity_snapshots` | `(book, date)` | `recorded_at` keeps its first value on replace |
| `shadow_daily` | `date` | paper return, shadow return, difference |
| `slippage` | `client_order_id` | reference price, average fill price, basis points |
| `runs` | autoincrement | stage, status, config hash, code version, phases |
| `halts` | autoincrement | reason, detail, set and cleared instants |
| `alerts` | autoincrement | level, code, message, delivered |
| `trials` | `(family, trial_hash)` | per-period Sharpe ratio, observations, run count |
| `backtest_runs`, `backtest_equity` | run id; `(run_id, date)` | stored results of research backtests |

```rust
// db.rs
pub struct Store { /* private */ }
impl Store { pub fn open(path: &Path) -> Result<Store, StoreError>;
             pub fn open_in_memory() -> Result<Store, StoreError>;
             pub fn schema_version(&self) -> Result<i64, StoreError>;
             pub fn execute_raw(&self, sql: &str) -> Result<usize, StoreError>; }   // tests and triggers
// codec.rs
pub fn dec_to_text(d: Decimal) -> String; pub fn text_to_dec(s: &str) -> Result<Decimal, StoreError>;
// market.rs
pub struct BarRow { pub bar: Bar, pub adjustment: Adjustment, pub feed: String }
pub struct UpsertStats { pub inserted: usize, pub unchanged: usize, pub restated: usize }
impl Store {
    pub fn upsert_bars(&self, rows: &[BarRow], fetched_at: DateTime<Utc>) -> Result<UpsertStats, StoreError>;
    pub fn bars_asof(&self, symbol: &Symbol, adjustment: Adjustment, feed: &str, from: NaiveDate,
                     to: NaiveDate, asof: Option<DateTime<Utc>>) -> Result<Vec<Bar>, StoreError>;
    pub fn put_sessions(&self, sessions: &[NaiveDate]) -> Result<(), StoreError>;
    pub fn calendar(&self) -> Result<TradingCalendar, StoreError>;
    pub fn put_asset_snapshot(&self, s: &AssetSnapshot, fetched_at: DateTime<Utc>) -> Result<(), StoreError>;
    pub fn asset_asof(&self, symbol: &Symbol, asof: Option<DateTime<Utc>>) -> Result<Option<AssetSnapshot>, StoreError>;
}
pub struct AssetSnapshot { pub symbol: Symbol, pub tradable: bool, pub shortable: bool, pub easy_to_borrow: bool }
// trading.rs
pub enum Book { Paper, Shadow }
pub enum Mode { DryRun, Paper }
pub struct DecisionRow { pub id: i64, pub strategy_id: String, pub signal_date: NaiveDate,
                         pub trade_date: Option<NaiveDate>, pub mode: Mode, pub status: String,
                         pub inputs_json: String, pub outputs_json: String,
                         pub created_at: DateTime<Utc> }
pub struct OrderRow { pub client_order_id: String, pub decision_id: Option<i64>, pub symbol: Symbol,
                      pub side: Side, pub qty: u64, pub leg: u8, pub state: OrderState,
                      pub broker_order_id: Option<String>, pub filled_qty: u64,
                      pub reference_price: Decimal, pub trade_date: NaiveDate }
pub struct FillRow { pub activity_id: String, pub broker_order_id: String, pub symbol: Symbol,
                     pub side: Side, pub qty: u64, pub price: Decimal, pub at: DateTime<Utc> }
pub struct LedgerEntry { pub book: Book, pub date: NaiveDate, pub kind: String, pub symbol: Option<Symbol>,
                         pub qty: i64, pub price: Option<Decimal>, pub cash_delta: Decimal,
                         pub ref_id: Option<String> }
pub struct EquitySnapshot { pub book: Book, pub date: NaiveDate, pub cash: Decimal,
                            pub positions_value: Decimal, pub equity: Decimal,
                            pub recorded_at: DateTime<Utc> }
impl Store {
    pub fn insert_decision(&self, d: &DecisionRow) -> Result<i64, StoreError>;
    pub fn decision_for(&self, strategy_id: &str, signal_date: NaiveDate, mode: Mode) -> Result<Option<DecisionRow>, StoreError>;
    pub fn decisions(&self, mode: Mode) -> Result<Vec<DecisionRow>, StoreError>;
    pub fn insert_order(&self, o: &OrderRow) -> Result<(), StoreError>;
    pub fn set_order_state(&self, client_order_id: &str, to: OrderState, filled_qty: u64,
                           broker_order_id: Option<&str>, at: DateTime<Utc>) -> Result<(), StoreError>;
    pub fn order(&self, client_order_id: &str) -> Result<Option<OrderRow>, StoreError>;
    pub fn orders_in_states(&self, states: &[OrderState]) -> Result<Vec<OrderRow>, StoreError>;
    pub fn insert_fill(&self, f: &FillRow) -> Result<bool, StoreError>;        // false if already present
    pub fn fills(&self) -> Result<Vec<FillRow>, StoreError>;                   // insertion order
    pub fn append_ledger(&self, e: &LedgerEntry) -> Result<i64, StoreError>;
    pub fn ledger(&self, book: Book) -> Result<Vec<LedgerEntry>, StoreError>;  // insertion order
    pub fn positions(&self, book: Book) -> Result<Positions, StoreError>;
    pub fn cash(&self, book: Book) -> Result<Decimal, StoreError>;
    pub fn positions_at(&self, book: Book, through: NaiveDate) -> Result<Positions, StoreError>;   // entries dated on or before
    pub fn cash_at(&self, book: Book, through: NaiveDate) -> Result<Decimal, StoreError>;
    pub fn put_equity_snapshot(&self, s: &EquitySnapshot) -> Result<(), StoreError>;
    pub fn equity_series(&self, book: Book) -> Result<Vec<EquitySnapshot>, StoreError>;
}
// shadow.rs
pub struct ShadowDay { pub date: NaiveDate, pub paper_return: f64, pub shadow_return: f64, pub diff: f64 }
pub struct SlippageRow { pub client_order_id: String, pub reference_price: Decimal,
                         pub avg_fill_price: Decimal, pub qty: u64, pub bps: f64 }
impl Store { pub fn put_shadow_day(&self, d: &ShadowDay) -> Result<(), StoreError>;
             pub fn shadow_days(&self) -> Result<Vec<ShadowDay>, StoreError>;
             pub fn put_slippage(&self, s: &SlippageRow) -> Result<(), StoreError>;
             pub fn slippage_rows(&self) -> Result<Vec<SlippageRow>, StoreError>; }
// ops.rs
pub struct RunRow { pub id: i64, pub started_at: DateTime<Utc>, pub finished_at: Option<DateTime<Utc>>,
                    pub stage: String, pub status: String, pub config_hash: String,
                    pub code_version: String, pub phases: String }
pub struct HaltRow { pub id: i64, pub reason: String, pub detail: String, pub set_at: DateTime<Utc>,
                     pub cleared_at: Option<DateTime<Utc>>, pub clear_note: Option<String> }
pub struct TrialRow { pub family: String, pub trial_hash: String, pub sharpe: f64, pub n_obs: u32, pub runs: u32 }
impl Store {
    pub fn start_run(&self, stage: &str, config_hash: &str, code_version: &str, at: DateTime<Utc>) -> Result<i64, StoreError>;
    pub fn finish_run(&self, id: i64, status: &str, phases: &str, at: DateTime<Utc>) -> Result<(), StoreError>;
    pub fn runs(&self) -> Result<Vec<RunRow>, StoreError>;
    pub fn set_halt(&self, reason: &str, detail: &str, at: DateTime<Utc>) -> Result<i64, StoreError>;
    pub fn clear_halt(&self, id: i64, note: &str, at: DateTime<Utc>) -> Result<(), StoreError>;
    pub fn active_halts(&self) -> Result<Vec<HaltRow>, StoreError>;
    pub fn halts(&self) -> Result<Vec<HaltRow>, StoreError>;
    pub fn record_alert(&self, level: &str, code: &str, message: &str, delivered: bool, at: DateTime<Utc>) -> Result<i64, StoreError>;
    pub fn alerts(&self) -> Result<Vec<(String, String, String, bool)>, StoreError>;   // level, code, message, delivered
    pub fn register_trial(&self, t: &TrialRow) -> Result<(), StoreError>;
    pub fn trials(&self, family: &str) -> Result<Vec<TrialRow>, StoreError>;
    pub fn save_backtest(&self, family: &str, trial_hash: &str, config_hash: &str,
                         equity: &[(NaiveDate, Decimal)], at: DateTime<Utc>) -> Result<i64, StoreError>;
    pub fn backtest_equity(&self, run_id: i64) -> Result<Vec<(NaiveDate, Decimal)>, StoreError>;
}
```


### 5.5 `pt-alpaca`, `pt-mock-alpaca`

```rust
// pt-alpaca
// endpoints.rs
pub const PAPER_TRADING_URL: &str = "https://paper-api.alpaca.markets";
pub const MARKET_DATA_URL: &str = "https://data.alpaca.markets";
pub struct Endpoints { /* private */ }
impl Endpoints { pub fn production() -> Endpoints;
                 pub fn with_overrides(trading: Option<&str>, data: Option<&str>) -> Result<Endpoints, AlpacaError>;
                 pub fn trading(&self) -> &str; pub fn data(&self) -> &str; }
// auth.rs
pub struct Credentials { /* private; Debug prints "***" */ }
impl Credentials { pub fn new(key_id: String, secret_key: String) -> Credentials; }
// limiter.rs
pub trait Sleeper { fn sleep(&self, d: std::time::Duration); }
pub trait MonoClock { fn now(&self) -> std::time::Duration; }      // monotonic time since start
pub struct RateLimiter { /* private */ }
impl RateLimiter { pub fn new(max_per_minute: u32, clock: Arc<dyn MonoClock>, sleeper: Arc<dyn Sleeper>) -> RateLimiter;
                   pub fn acquire(&self); }
// http.rs
pub struct HttpClient { /* private */ }
impl HttpClient { pub fn new(endpoints: Endpoints, creds: Credentials, limiter: RateLimiter,
                             sleeper: Arc<dyn Sleeper>) -> HttpClient; }
/// For tests: dummy credentials, no rate limit, a sleeper that returns at once. Loopback URLs only.
pub fn loopback_client(url: &str) -> Result<HttpClient, AlpacaError>;
// data.rs
pub struct WireBar { pub symbol: String, pub date: NaiveDate, pub open: Decimal, pub high: Decimal,
                     pub low: Decimal, pub close: Decimal, pub volume: u64 }
pub fn daily_bars(http: &HttpClient, symbols: &[String], start: NaiveDate, end: NaiveDate,
                  adjustment: &str, feed: &str) -> Result<Vec<WireBar>, AlpacaError>;
// account.rs
pub struct WireAccount { pub status: String, pub cash: Decimal, pub equity: Decimal, pub last_equity: Decimal,
                         pub buying_power: Decimal, pub multiplier: Decimal, pub shorting_enabled: bool,
                         pub trading_blocked: bool, pub account_blocked: bool }
pub struct WirePosition { pub symbol: String, pub qty: i64 }
pub fn account(http: &HttpClient) -> Result<WireAccount, AlpacaError>;
pub fn positions(http: &HttpClient) -> Result<Vec<WirePosition>, AlpacaError>;
// reference.rs
pub struct WireClock { pub timestamp: DateTime<FixedOffset>, pub is_open: bool,
                       pub next_open: DateTime<FixedOffset>, pub next_close: DateTime<FixedOffset> }
pub struct WireAsset { pub symbol: String, pub tradable: bool, pub shortable: bool, pub easy_to_borrow: bool }
pub fn clock(http: &HttpClient) -> Result<WireClock, AlpacaError>;
pub fn calendar(http: &HttpClient, start: NaiveDate, end: NaiveDate) -> Result<Vec<NaiveDate>, AlpacaError>;
pub fn asset(http: &HttpClient, symbol: &str) -> Result<Option<WireAsset>, AlpacaError>;
// orders_read.rs
pub struct WireOrder { pub id: String, pub client_order_id: String, pub symbol: String, pub side: String,
                       pub qty: u64, pub filled_qty: u64, pub status: String,
                       pub submitted_at: DateTime<Utc> }
pub fn order_by_client_id(http: &HttpClient, client_order_id: &str) -> Result<Option<WireOrder>, AlpacaError>;
pub fn orders_since(http: &HttpClient, after: DateTime<Utc>) -> Result<Vec<WireOrder>, AlpacaError>;
// activities.rs
pub struct WireFill { pub id: String, pub order_id: String, pub symbol: String, pub side: String,
                      pub qty: u64, pub price: Decimal, pub transaction_time: DateTime<Utc> }
pub fn fill_activities(http: &HttpClient, after: DateTime<Utc>) -> Result<Vec<WireFill>, AlpacaError>;
// orders_write.rs
pub struct NewOrder { pub client_order_id: String, pub symbol: String, pub side: String, pub qty: u64 }
pub fn submit_order(http: &HttpClient, permit: OrderPermit, order: &NewOrder) -> Result<WireOrder, AlpacaError>;
pub fn cancel_order(http: &HttpClient, broker_order_id: &str) -> Result<(), AlpacaError>;
```

```rust
// pt-mock-alpaca, server.rs
pub struct MockAlpaca { /* private */ }
pub struct Recorded { pub method: String, pub path: String, pub query: String, pub body: String }
impl MockAlpaca {
    pub fn start() -> MockAlpaca;                       // binds 127.0.0.1 on a free port
    pub fn url(&self) -> String;                        // serves trading and data routes on one base URL
    pub fn requests(&self) -> Vec<Recorded>;
    pub fn seed_account(&self, cash: &str);
    pub fn seed_bars(&self, adjustment: &str, feed: &str, bars: Vec<(String, String, f64, f64, f64, f64, u64)>);
                                                        // symbol, date, open, high, low, close, volume
    pub fn seed_sessions(&self, dates: &[&str]);
    pub fn seed_asset(&self, symbol: &str, tradable: bool, shortable: bool, easy_to_borrow: bool);
    pub fn set_clock(&self, timestamp: &str, is_open: bool, next_open: &str, next_close: &str);
    pub fn seed_foreign_order(&self, client_order_id: &str, symbol: &str, side: &str, qty: u64);
    pub fn seed_position(&self, symbol: &str, qty: i64);
    pub fn fill(&self, client_order_id: &str, qty: u64, price: &str);
    pub fn expire(&self, client_order_id: &str);
    pub fn set_order_status(&self, client_order_id: &str, status: &str);   // any string, for failure drills
    pub fn fail_next(&self, n: usize, status: u16);
    pub fn deny_feed(&self, feed: &str);                // bars requests for this feed answer 403
}
```

### 5.6 `pt-data`

```rust
// ingest.rs
pub struct IngestReport { pub raw: UpsertStats, pub adjusted: UpsertStats, pub complete: bool }
pub fn ingest(store: &Store, http: &HttpClient, cfg: &Config, through: NaiveDate,
              fetched_at: DateTime<Utc>) -> Result<IngestReport, DataError>;
// quality.rs
pub struct Gap { pub symbol: Symbol, pub date: NaiveDate }
pub fn check_bar(b: &WireBar) -> Result<(), String>;
pub fn check_gaps(store: &Store, cfg: &Config, through: NaiveDate) -> Result<Vec<Gap>, DataError>;
// csv_import.rs
pub fn import_bars_csv(store: &Store, csv_text: &str, adjustment: Adjustment, source: &str,
                       fetched_at: DateTime<Utc>) -> Result<usize, DataError>;
pub fn import_calendar_csv(store: &Store, csv_text: &str) -> Result<usize, DataError>;
// reference.rs
pub fn sync_calendar(store: &Store, http: &HttpClient, from: NaiveDate, to: NaiveDate) -> Result<usize, DataError>;
pub fn sync_assets(store: &Store, http: &HttpClient, cfg: &Config, fetched_at: DateTime<Utc>)
    -> Result<Vec<AssetSnapshot>, DataError>;
// universe.rs
pub struct UniverseCheck { pub symbol: Symbol, pub pass: bool, pub reasons: Vec<String> }
pub fn verify_universe(store: &Store, http: &HttpClient, cfg: &Config, through: NaiveDate)
    -> Result<Vec<UniverseCheck>, DataError>;
// doctor.rs
pub struct DoctorLine { pub check: &'static str, pub pass: bool, pub detail: String }
pub fn doctor(http: &HttpClient, cfg: &Config) -> Vec<DoctorLine>;
// view.rs
pub struct StoreView { /* private; loads the requested series into memory */ }
impl StoreView { pub fn load(store: &Store, symbols: &[Symbol], feed: &str, basis: PriceBasis,
                             asof: Option<DateTime<Utc>>) -> Result<StoreView, DataError>; }   // impl MarketView
```

`ingest` is `complete` when every universe symbol and the risk-free symbol has a raw and an adjusted bar for `through`. A CSV import stores bars under the feed name `csv:<source>`. To backtest on imported bars, set `data.feed` to that name; the config accepts `sip`, `iex`, or a name that starts with `csv:`.

### 5.7 Strategy, portfolio, risk, engine, vector

```rust
// pt-strategy
// vol.rs
pub fn ewma_vol(returns: &[f64], com_days: f64, annualization: f64) -> Option<f64>;      // oldest first
pub fn ewma_cov(a: &[f64], b: &[f64], com_days: f64, annualization: f64) -> Option<f64>;
// estimate.rs
pub struct VolParams { pub com_days: f64, pub annualization: f64, pub min_obs: usize, pub max_obs: usize }
pub fn estimate_risk(symbols: &[Symbol], asof: NaiveDate, view: &dyn HistoryView, p: &VolParams) -> RiskEstimates;
// strategy.rs
pub trait Strategy { fn id(&self) -> &str; fn params_canonical(&self) -> String;
                     fn signals(&self, universe: &[Symbol], asof: NaiveDate, view: &dyn HistoryView)
                         -> Result<SignalSet, StrategyError>; }
// tsmom.rs
pub struct TsmomParams { pub lookback_months: Vec<u32>, pub risk_free_symbol: Symbol }
pub struct Tsmom { /* private */ }
impl Tsmom { pub fn new(p: TsmomParams, cal: Arc<TradingCalendar>) -> Tsmom; }   // impl Strategy, id "tsmom-v1"
// benchmark.rs
pub struct LongOnly { /* private */ }                                             // impl Strategy, id "bh-volscaled-v1"
impl LongOnly { pub fn new(cal: Arc<TradingCalendar>) -> LongOnly; }

// pt-portfolio
// weights.rs
pub struct SizingParams { pub target_vol: f64, pub max_weight: f64, pub max_gross: f64 }
pub fn target_weights(signals: &BTreeMap<Symbol, f64>, risk: &RiskEstimates,
                      shortable: &BTreeSet<Symbol>, p: &SizingParams) -> Weights;
// orders.rs
pub fn target_shares(weights: &Weights, equity: Money, ref_prices: &BTreeMap<Symbol, Price>)
    -> Result<Positions, PortfolioError>;
pub fn plan_orders(current: &Positions, target: &Positions, ref_prices: &BTreeMap<Symbol, Price>)
    -> Result<Vec<OrderIntent>, PortfolioError>;

// pt-risk
// pretrade.rs
pub struct RiskLimits { pub max_order_notional: Decimal, pub max_weight: Decimal, pub max_gross: Decimal,
                        pub daily_loss_limit: Decimal, pub max_drawdown: Decimal }
pub enum Rejection { OrderNotional, WeightCap, Gross, NoEquity }
pub struct GateOutcome { pub approved: Vec<OrderIntent>, pub rejected: Vec<(OrderIntent, Rejection)> }
pub fn check_orders(orders: &[OrderIntent], current: &Positions, equity: Money,
                    ref_prices: &BTreeMap<Symbol, Price>, limits: &RiskLimits) -> GateOutcome;
// halts.rs
pub enum HaltReason { DailyLoss, Drawdown }
pub fn check_equity(equity: &[Decimal], limits: &RiskLimits) -> Option<HaltReason>;       // oldest first

// pt-engine
// cost.rs
pub struct ImpactInputs { pub daily_vol: f64, pub adv_shares: f64 }
pub fn fill_cost(spec: &CostSpec, qty: u64, price: Price, impact: Option<ImpactInputs>) -> Result<Money, EngineError>;
pub fn borrow_cost(spec: &CostSpec, short_market_value: Money, days: i64) -> Money;
// book.rs
pub struct Book { pub cash: Decimal, pub positions: Positions }
impl Book { pub fn new(cash: Decimal) -> Book;
            pub fn apply_fill(&mut self, f: &Fill);
            pub fn equity(&self, closes: &BTreeMap<Symbol, Price>) -> Result<Decimal, EngineError>; }
// decision.rs
pub struct DecisionInputs { pub signal_date: NaiveDate, pub trade_date: NaiveDate, pub ref_date: NaiveDate,
                            pub equity: Money, pub positions: Positions, pub universe: Vec<Symbol>,
                            pub shortable: BTreeSet<Symbol> }
pub struct Decision { pub signals: SignalSet, pub risk: RiskEstimates, pub weights: Weights,
                      pub target: Positions, pub orders: Vec<OrderIntent>,
                      pub rejected: Vec<(OrderIntent, Rejection)> }
pub struct DecisionParams { pub vol: VolParams, pub sizing: SizingParams, pub limits: RiskLimits }
pub fn params_from_config(cfg: &Config) -> DecisionParams;     // the one place config becomes parameters
pub fn decide_with(signals: SignalSet, risk: RiskEstimates, inputs: &DecisionInputs,
                   ref_prices: &BTreeMap<Symbol, Price>, p: &DecisionParams) -> Result<Decision, EngineError>;
pub fn decide(inputs: &DecisionInputs, strategy: &dyn Strategy, view: &dyn MarketView,
              p: &DecisionParams) -> Result<Decision, EngineError>;
pub fn decision_to_json(d: &Decision) -> String;       // canonical: sorted keys, decimals as strings
pub struct DecisionRecord { pub inputs: DecisionInputs, pub data_asof: DateTime<Utc>, pub feed: String,
                            pub config_hash: String, pub code_version: String, pub bootstrap: bool }
pub fn record_to_json(r: &DecisionRecord) -> String;
pub fn record_from_json(s: &str) -> Result<DecisionRecord, EngineError>;
// broker.rs
pub struct SimBroker { /* private */ }
impl SimBroker { pub fn new(cost: CostSpec) -> SimBroker;
                 pub fn execute(&self, date: NaiveDate, orders: &[OrderIntent], view: &dyn MarketView)
                     -> Result<Vec<Fill>, EngineError>; }
// simulate.rs
pub struct SimConfig { pub start: NaiveDate, pub end: NaiveDate, pub initial_capital: Decimal,
                       pub cost: CostSpec, pub limits: RiskLimits, pub universe: Vec<Symbol> }
pub struct SimResult { pub equity: Vec<(NaiveDate, Decimal)>, pub fills: Vec<(NaiveDate, Fill)>,
                       pub rejected: Vec<(NaiveDate, OrderIntent, Rejection)>,
                       pub would_halt: Vec<(NaiveDate, HaltReason)>,
                       pub cost_total: Decimal, pub borrow_total: Decimal, pub traded_notional: Decimal }
impl SimResult { pub fn clean(&self) -> bool; }        // true when nothing was rejected
pub fn simulate(cfg: &SimConfig, weights: &BTreeMap<NaiveDate, Weights>, view: &dyn MarketView,
                cal: &TradingCalendar) -> Result<SimResult, EngineError>;      // weights keyed by trade date
// schedule.rs
pub fn weights_schedule(schedule: &[Rebalance], universe: &[Symbol], shortable: &BTreeSet<Symbol>,
                        strategy: &dyn Strategy, view: &dyn HistoryView, p: &DecisionParams)
    -> Result<BTreeMap<NaiveDate, Weights>, EngineError>;
pub fn run_backtest(cfg: &SimConfig, schedule: &[Rebalance], shortable: &BTreeSet<Symbol>,
                    strategy: &dyn Strategy, view: &dyn MarketView, cal: &TradingCalendar,
                    p: &DecisionParams) -> Result<SimResult, EngineError>;

// pt-vector, vector.rs
pub struct VectorInputs { pub dates: Vec<NaiveDate>, pub symbols: Vec<Symbol>,
                          pub open: Vec<Vec<Decimal>>, pub close: Vec<Vec<Decimal>>,   // [date][symbol]
                          pub weights: BTreeMap<NaiveDate, Vec<Decimal>>,              // by trade date
                          pub days_to_next: Vec<i64>, pub initial_capital: Decimal, pub cost: CostSpec }
pub fn vector_equity(inp: &VectorInputs) -> Result<Vec<Decimal>, VectorError>;
pub struct CrossCheck { pub equal: bool, pub max_abs_diff: Decimal, pub first_diff_date: Option<NaiveDate> }
pub fn compare(dates: &[NaiveDate], a: &[Decimal], b: &[Decimal], tolerance: Decimal) -> Result<CrossCheck, VectorError>;
```

### 5.8 Statistics, validation, live, shadow, operations

```rust
// pt-stats
// descriptive.rs
pub fn mean(x: &[f64]) -> f64; pub fn std_sample(x: &[f64]) -> f64;                 // ddof = 1
pub fn skewness(x: &[f64]) -> f64; pub fn kurtosis(x: &[f64]) -> f64;               // population moments; normal = 3
pub fn returns_from_equity(e: &[f64]) -> Vec<f64>;
// perf.rs
pub fn sharpe(returns: &[f64]) -> f64;                                               // per period; 0 if std is 0
pub fn annualized_sharpe(returns: &[f64], periods_per_year: f64) -> f64;
pub fn annualized_vol(returns: &[f64], periods_per_year: f64) -> f64;
pub fn max_drawdown(equity: &[f64]) -> f64; pub fn cagr(equity: &[f64], periods_per_year: f64) -> f64;
// normal.rs
pub fn norm_cdf(x: f64) -> f64; pub fn norm_inv_cdf(p: f64) -> Result<f64, StatsError>;
// dsr.rs
pub fn expected_max_sharpe(var_sr: f64, n_trials: u32) -> f64;
pub fn psr(sr: f64, t: usize, skew: f64, kurt: f64, sr_benchmark: f64) -> Result<f64, StatsError>;
pub fn deflated_sharpe(sr: f64, t: usize, skew: f64, kurt: f64, var_sr: f64, n_trials: u32) -> Result<f64, StatsError>;
// pbo.rs
pub struct PboResult { pub pbo: f64, pub logits: Vec<f64>, pub n_combinations: usize, pub rows_used: usize }
pub fn n_combinations(s: usize) -> usize;
pub fn pbo_cscv(m: &[Vec<f64>], s: usize) -> Result<PboResult, StatsError>;          // m[row][trial]
// ols.rs
pub struct Ols { pub alpha: f64, pub beta: f64, pub alpha_t: f64, pub r2: f64, pub n: usize }
pub fn ols(y: &[f64], x: &[f64]) -> Result<Ols, StatsError>;

// pt-validate
// runner.rs
pub struct BacktestRequest { pub strategy: String /* "tsmom" or "benchmark" */, pub from: NaiveDate,
                             pub to: NaiveDate, pub initial_capital: Decimal }
pub struct Metrics { pub sharpe: f64, pub cagr: f64, pub vol: f64, pub max_drawdown: f64,
                     pub turnover: f64, pub cost_total: Decimal, pub borrow_total: Decimal, pub n_obs: usize }
pub struct BacktestOutcome { pub run_id: i64, pub trial_hash: String, pub metrics: Metrics, pub result: SimResult }
pub fn run_and_record(store: &Store, cfg: &Config, req: &BacktestRequest, clock: &dyn Clock)
    -> Result<BacktestOutcome, ValidateError>;
pub fn trial_hash(cfg: &Config, req: &BacktestRequest, strategy_params: &str) -> String;
// crosscheck.rs
pub fn crosscheck(store: &Store, cfg: &Config, req: &BacktestRequest, zero_cost: bool, tolerance: Decimal)
    -> Result<CrossCheck, ValidateError>;
// overfit.rs
pub struct DsrReport { pub dsr: f64, pub sr: f64, pub sr0: f64, pub n_trials: u32, pub skew: f64, pub kurt: f64, pub t: usize }
pub fn dsr_for_run(store: &Store, cfg: &Config, run_id: i64, family: &str) -> Result<DsrReport, ValidateError>;
pub fn pbo_for_runs(store: &Store, run_ids: &[i64], s: usize) -> Result<PboResult, ValidateError>;
pub struct BenchmarkReport { pub strategy_sharpe: f64, pub benchmark_sharpe: f64, pub ols: Ols }
pub fn compare_to_benchmark(store: &Store, cfg: &Config, run_id: i64, benchmark_run_id: i64, periods_per_year: f64)
    -> Result<BenchmarkReport, ValidateError>;

// pt-live
// ids.rs
pub fn client_order_id(strategy_id: &str, signal_date: NaiveDate, symbol: &str, leg: u8) -> String;
pub fn first_order_id(valid_on: NaiveDate, symbol: &str) -> String;
// status.rs
pub fn map_status(alpaca_status: &str, filled_qty: u64) -> Result<OrderState, LiveError>;
// context.rs
pub struct LiveContext<'a> { pub store: &'a Store, pub cfg: &'a Config, pub gate: &'a Gate,
                             pub http: &'a HttpClient, pub clock: &'a dyn Clock,
                             pub alerter: &'a Alerter, pub strategy: &'a dyn Strategy,
                             pub code_version: &'a str }
// submit.rs
pub fn submit_planned(ctx: &LiveContext, auth: &AuthContext) -> Result<SubmitReport, LiveError>;
pub struct SubmitReport { pub submitted: usize, pub adopted: usize, pub rejected: usize, pub abandoned: usize }
// settle.rs
pub fn settle(ctx: &LiveContext) -> Result<SettleReport, LiveError>;
pub struct SettleReport { pub orders_updated: usize, pub fills_recorded: usize, pub legs_released: usize }
// reconcile.rs
pub fn reconcile(ctx: &LiveContext) -> Result<ReconcileReport, LiveError>;
pub struct ReconcileReport { pub clean: bool, pub halt_reason: Option<String> }
// snapshot.rs
pub fn expected_last_session(clock: &WireClock, cal: &TradingCalendar) -> Option<NaiveDate>;
pub fn trade_session(clock: &WireClock) -> NaiveDate;
pub fn snapshot_equity(ctx: &LiveContext, through: NaiveDate) -> Result<usize, LiveError>;
pub fn check_halts(ctx: &LiveContext) -> Result<Option<HaltReason>, LiveError>;
// rebalance.rs
pub enum RebalanceOutcome { NotDue, Deferred, DataNotReady, SkippedLate, Recorded { decision_id: i64, orders: usize } }
pub fn rebalance(ctx: &LiveContext, wire_clock: &WireClock) -> Result<RebalanceOutcome, LiveError>;
// cycle.rs
pub enum RunStatus { Ok, Halted, DataNotReady, Failed }
pub struct RunOutcome { pub run_id: i64, pub status: RunStatus, pub phases: Vec<String> }
pub fn run_cycle(ctx: &LiveContext) -> Result<RunOutcome, LiveError>;
// lock.rs
pub struct RunLock { /* private; released on drop */ }
pub fn acquire_run_lock(db_path: &Path) -> Result<RunLock, LiveError>;      // `File::try_lock` on `<db_path>.lock`; the OS frees it if the process dies
// first_order.rs
pub fn first_order(ctx: &LiveContext, wire_clock: &WireClock) -> Result<bool, LiveError>;   // true if it planned the order
// killswitch.rs
pub fn cancel_open(ctx: &LiveContext) -> Result<usize, LiveError>;
pub fn flatten(ctx: &LiveContext) -> Result<usize, LiveError>;

// pt-shadow
// replay.rs
pub struct ReplayResult { pub decision_id: i64, pub matches: bool, pub diffs: Vec<String> }
pub fn replay_decision(store: &Store, cfg: &Config, strategy: &dyn Strategy, decision_id: i64) -> Result<ReplayResult, ShadowError>;
// daily.rs
pub fn shadow_day(store: &Store, cfg: &Config, date: NaiveDate) -> Result<ShadowDay, ShadowError>;
pub fn update(store: &Store, cfg: &Config) -> Result<usize, ShadowError>;     // every session not yet computed
// slippage.rs
pub fn slippage_bps(side: Side, avg_fill_price: Decimal, reference_price: Decimal) -> f64;
pub fn record_slippage(store: &Store, cfg: &Config) -> Result<usize, ShadowError>;
// fidelity.rs
pub struct Fidelity { pub n_sessions: usize, pub n_rebalances: usize, pub n_fills: usize,
                      pub replay_mismatches: usize, pub reconciliation_breaks: usize,
                      pub missed_sessions: usize, pub late_rebalances: usize,
                      pub mean_slippage_bps: f64, pub mean_diff_bps: f64, pub max_abs_diff_bps: f64 }
pub fn fidelity(store: &Store, cfg: &Config, strategy: &dyn Strategy) -> Result<Fidelity, ShadowError>;
// signoff.rs
pub enum Verdict { Pass, Fail, NotReady }
pub struct Criterion { pub id: &'static str, pub value: f64, pub limit: f64, pub pass: bool }
pub struct SignoffReport { pub verdict: Verdict, pub criteria: Vec<Criterion>, pub feed: String }
pub fn evaluate(f: &Fidelity, cfg: &Config) -> SignoffReport;
pub fn write_signoff(report: &SignoffReport, out_dir: &Path) -> Result<PathBuf, ShadowError>;

// pt-ops
// alert.rs
pub enum Level { Info, Warn, Critical }
pub struct Alerter { /* private */ }
impl Alerter { pub fn new(webhook_url: Option<String>, scrub: Vec<String>) -> Alerter;
               pub fn alert(&self, store: &Store, level: Level, code: &str, message: &str,
                            run_id: Option<i64>, at: DateTime<Utc>) -> bool; }     // true if the webhook accepted it
// logging.rs
pub fn format_event(level: &str, message: &str, run_id: Option<i64>, at: DateTime<Utc>) -> String;   // one JSON line
// pnl.rs
pub struct Pnl { pub day: Decimal, pub mtd: Decimal, pub ytd: Decimal, pub inception: Decimal }
pub fn pnl(series: &[(NaiveDate, Decimal)], asof: NaiveDate) -> Option<Pnl>;
// report.rs
pub struct ReportInputs<'a> { pub store: &'a Store, pub cfg: &'a Config, pub stage: &'a str,
                              pub fidelity: &'a [(String, String)],     // label, value; empty if none
                              pub validation_signoff: Option<&'a str>,  // from the owner file
                              pub now: DateTime<Utc> }
pub fn write_status(inp: &ReportInputs) -> Result<(PathBuf, PathBuf), OpsError>;       // status.md, status.json
pub fn last_ok_run_age_hours(store: &Store, now: DateTime<Utc>) -> Result<Option<f64>, OpsError>;
```

Command line (`trader`). Global options: `--config <path>` (or `PT_CONFIG`).

| Command | Minimum stage | Effect |
| --- | --- | --- |
| `config check` | none | Validate the config and print its hash |
| `gates` | none | Print the stage and the owner checklist |
| `calendar import --csv <file>` | none | Load sessions from a CSV with a `date` column |
| `data import-csv --csv <file> --adjustment <raw or all> --source <name>` | none | Load bars from CSV |
| `data check` | none | Report gaps and bad bars |
| `backtest run [--strategy tsmom or benchmark] --from <date> --to <date> [--capital <n>]` | none | Run, store and register a research backtest |
| `backtest crosscheck --from <date> --to <date> [--with-costs]` | none | Compare engine and vector calculator |
| `validate dsr --run <id>` | none | Deflated Sharpe Ratio from the trial registry |
| `validate pbo --runs <id,id,...>` | none | Probability of Backtest Overfitting |
| `validate benchmark --run <id> --benchmark-run <id>` | none | Compare with volatility-scaled buy-and-hold |
| `calendar sync` | `ingest` | Fetch the Alpaca calendar |
| `data ingest` | `ingest` | Fetch bars |
| `universe verify` | `ingest` | Check each candidate ETF |
| `doctor` | `ingest` | Real-API smoke test |
| `run` | `ingest` | One idempotent daily cycle |
| `halt status`, `halt set --reason <text>`, `halt clear --id <n> --note <text>` | none | Inspect and change halts |
| `cancel-open --yes` | `first_order` | Cancel every open order the system created |
| `flatten --yes` | `paper` | Close every position and halt |
| `report` | none | Write the status and P&L report |
| `status --max-age-hours <n>` | none | Exit non-zero if the last good run is older |
| `signoff report` | none | Write the fidelity and sign-off report |

Exit codes: 0 success; 1 error; 2 completed but halted; 3 another run holds the lock; 4 the stage forbids the command.

The dispatcher in `main.rs` holds this table. It checks the minimum stage, and for commands that need the network loads the keys and builds the client, before any command module runs. A refused command therefore makes no request, whatever the command module does.

Identifiers: a rebalance order is `pt-<strategy id>-<signal date yyyymmdd>-<symbol>-<leg>`; the first order is `pt-first-<valid_on yyyymmdd>-<symbol>`; a flatten order is `pt-flatten-<yyyymmdd>-<symbol>`.

### 5.9 Alpaca facts the client is built on

Opened in Alpaca's documentation on 2026-11-23 unless marked.

| Item | Value |
| --- | --- |
| Bars | `GET {data}/v2/stocks/bars` with `symbols` (comma separated), `timeframe=1Day`, `start`, `end`, `limit` (1 to 10000, counted across symbols), `adjustment` (`raw`, `split`, `dividend`, `spin-off`, `all`; default `raw`), `feed` (`sip`, `iex`, `otc`, `boats`; default `sip`), `sort`, `page_token`. Response `{"bars": {"SPY": [{"t","o","h","l","c","v","n","vw"}]}, "next_page_token": string or null}` with prices as JSON numbers |
| Daily bar time | From memory, to be confirmed by `doctor`: `t` is midnight New York time, so 04:00:00Z or 05:00:00Z. The parser takes the UTC date and rejects any other time of day |
| Account | `GET {trading}/v2/account`: `status`, `cash`, `equity`, `last_equity`, `buying_power`, `multiplier` as strings; `shorting_enabled`, `trading_blocked`, `account_blocked` as booleans |
| Positions | `GET /v2/positions`: `symbol`, `qty` (string), `side` |
| Clock | `GET /v2/clock`: `timestamp`, `is_open`, `next_open`, `next_close` (RFC 3339 with offset) |
| Calendar | `GET /v2/calendar?start=&end=`: `date`, `open`, `close` (New York local `HH:MM`); covers 1970 to 2029 |
| Asset | `GET /v2/assets/{symbol}`: `tradable`, `shortable`, `borrow_status` (`easy_to_borrow` or `hard_to_borrow`); the example also shows `easy_to_borrow`; 404 if unknown |
| Submit | `POST /v2/orders` body `symbol`, `qty` (string), `side`, `type`, `time_in_force`, `client_order_id` (at most 128 characters). 403 means buying power or shares are insufficient; 422 means the input was not accepted |
| Order lookup | `GET /v2/orders:by_client_order_id?client_order_id=` (LISTING in the brief); `GET /v2/orders?status=all&after=&direction=asc&limit=500` |
| Order statuses | `new`, `partially_filled`, `filled`, `done_for_day`, `canceled`, `expired`, `replaced`, `pending_cancel`, `pending_replace`, `accepted`, `pending_new`, `accepted_for_bidding`, `stopped`, `rejected`, `suspended`, `calculated`, `held` |
| Fills | `GET /v2/account/activities/FILL?direction=asc&page_size=100&after=&page_token=<last id>`: `id`, `order_id`, `symbol`, `side`, `qty`, `price`, `transaction_time`, `type` (`fill` or `partial_fill`) |
| Cancel | `DELETE /v2/orders/{id}` |
| Day orders | Sent while the market is closed, they are queued for the next session's open |
| Rate limit | 429 with `X-RateLimit-Reset` (Unix seconds). The client does not read the header: it waits 15, 30, 45 then 60 seconds between attempts, so it needs no wall clock |
| Auth | Headers `APCA-API-KEY-ID` and `APCA-API-SECRET-KEY` |
| Position flips | Forum reports (LISTING): a sell larger than the long position is rejected with "insufficient qty available". The design never sends one |

### 5.10 Specifications transcribed from the papers

#### 5.10.1 Volatility and covariance (Moskowitz et al., equation 1)

Daily simple returns come from adjusted closes: `r_j = c_j / c_(j-1) - 1`. With centre of mass `com` (60), `delta = com / (1 + com)`. Take the most recent `n` returns, `n` capped at `vol_max_obs`, and index them `i = 0` for the newest. Weights `w_i = (1 - delta) * delta^i`, `W = sum(w_i)`.

- `rbar = sum(w_i * r_i) / W`
- `var = annualization * sum(w_i * (r_i - rbar)^2) / W`, `vol = sqrt(var)`
- `cov(a, b) = annualization * sum(w_i * (a_i - abar) * (b_i - bbar)) / W`, over sessions both series have.

The paper's sum is infinite, so its weights add to 1. Dividing by `W` is the finite-sample form of the same estimator. A symbol with fewer than `vol_min_obs` returns, zero volatility, or a missing session inside the window is ineligible at that date.

#### 5.10.2 Signal (Moskowitz et al., section 4)

At a month-end session `T`, for each lookback `k` in `lookback_months`: `R_k = adj(T) / adj(T_k) - 1` where `T_k = month_end_back(T, k)`; `F_k` is the same for the risk-free proxy; `s_k = sign(R_k - F_k)` with `sign(0) = 0`. The signal is the mean of `s_k` over the lookbacks. A symbol with no adjusted close at `T` or at any `T_k` is ineligible. A missing risk-free price is an error.

The benchmark strategy (`LongOnly`) sets the signal to +1 for every symbol with an adjusted close at `T`. It is the paper's "replace sign with 1" passive position, sized the same way.

#### 5.10.3 Weights

Inputs: signals `s_i`, volatilities `sigma_i`, covariance `C`, the shortable set, `target_vol`, `max_weight`, `max_gross`.

1. If `s_i < 0` and `i` is not shortable, set `s_i = 0`.
2. `u_i = s_i / sigma_i`.
3. `sigma_u = sqrt(u' C u)`. If it is zero, every weight is zero.
4. `w_i = u_i * target_vol / sigma_u`.
5. Clamp each `w_i` to `[-max_weight, +max_weight]`.
6. If `g = sum(|w_i|) > max_gross`, multiply every `w_i` by `max_gross / g`.
7. Round each `w_i` to 6 decimal places, midpoint away from zero. Ineligible symbols get 0.

#### 5.10.4 Orders

At trade date `D` with equity `E` and reference prices `p_i` (execution-basis close of the last completed session before `D`):

- Target shares `q*_i = trunc(w_i * E / p_i)`, truncation toward zero. A symbol held but absent from the weights has target 0.
- If the current position `q_i` and `q*_i` have opposite signs, two orders: leg 1 closes `q_i`, leg 2 opens `q*_i`. Otherwise one leg-1 order for `q*_i - q_i`, or none if it is zero.
- Order: leg-1 sells by symbol, leg-1 buys by symbol, leg-2 sells by symbol, leg-2 buys by symbol.
- Pre-trade checks, with exposure measured at reference prices. An order whose notional exceeds `max_order_notional` is rejected. An order that increases the absolute position is rejected if the resulting position exceeds `max_weight * E`, or if gross exposure after all orders would exceed `max_gross * E`. An order that only reduces a position is never rejected by the weight or gross check. If `E <= 0`, everything is rejected.

#### 5.10.5 Accounting (adapted from Yin et al., Algorithm 1)

State: cash, and one signed whole-share position per symbol (net positions; a sell reduces a long before it creates a short). For each session `D` from `start` to `end`:

1. If `D` is a trade date: `E` is the equity at the close of the previous session, or the initial capital on the first session. Build orders as in 5.10.4 from the weights for `D`. Each approved order fills completely at the execution-basis open of `D`: `cash -= sign * qty * open`.
2. Each fill costs `round6(qty * open * (spread_bps + fee_bps) / 10000) + round6(qty * open * impact)`, where `impact = impact_coeff * daily_vol * sqrt(qty / adv_shares)`, `daily_vol` is the sample standard deviation of the 20 adjusted daily returns ending at the session before `D`, and `adv_shares` is the mean volume of the 20 sessions before `D`. The impact term is skipped when `impact_coeff` is 0. Cost is subtracted from cash.
3. At the close: `borrow = round6(sum over shorts(|q_i| * close_i) * borrow_bps_annual / 10000 / 360 * days)`, where `days` is the number of calendar days to the next session (1 on the last session). It is subtracted from cash.
4. `equity(D) = cash + sum(q_i * close_i)`.

The sum of per-fill proportional costs equals Algorithm 1's `c * ||delta||_1` over the trades actually executed. The vector calculator implements steps 1 to 4 without the impact term and without the pre-trade checks; it returns an error if `impact_coeff` is not zero. A missing bar for any held or traded symbol is an error, never a carried-forward price. The result must hold exactly one equity value per calendar session in the range.

Bases. A research backtest fills and marks on adjusted prices (total return) with `cost.research`. The shadow simulation fills and marks on raw prices (price return) with `cost.shadow`, because paper pays no dividends and charges no borrow. Signals always use adjusted closes.

#### 5.10.6 Deflated Sharpe Ratio (Bailey and Lopez de Prado, equations 1 and 2)

With per-period Sharpe ratio `SR`, `T` returns, skewness `g3`, kurtosis `g4` (normal is 3), `N` trials whose per-period Sharpe ratios have sample variance `V`, Euler-Mascheroni constant `gamma = 0.5772156649`, and `Z` the standard normal distribution function:

- `SR0 = sqrt(V) * ((1 - gamma) * Zinv(1 - 1/N) + gamma * Zinv(1 - 1/(N * e)))`, and `SR0 = 0` when `N = 1`.
- `DSR = Z((SR - SR0) * sqrt(T - 1) / sqrt(1 - g3 * SR + (g4 - 1) / 4 * SR^2))`.

The paper's example: `N = 100`, `V = 1/500`, `T = 1250`, `SR = 2.5 / sqrt(250)`, `g3 = -3`, `g4 = 10` gives `SR0 = 0.1132` and `DSR = 0.9004`; with `N = 46` it gives 0.9505.

#### 5.10.7 Probability of Backtest Overfitting (Bailey et al., algorithm 2.3)

`M` is a `T x N` matrix of returns, one column per trial, rows aligned in time. `S` is even.

1. Drop the earliest `T mod S` rows. Cut the rest into `S` consecutive blocks of equal length.
2. For each of the `C(S, S/2)` ways to choose `S/2` blocks, in lexicographic order of block indices: the chosen blocks are the in-sample set, the others the out-of-sample set, both in original order.
3. Compute each trial's Sharpe ratio (mean over sample standard deviation) in-sample and out-of-sample. `n*` is the trial with the highest in-sample value, lowest index on ties.
4. Rank the out-of-sample values ascending, 1 for the worst, mean rank on ties. `omega = rank(n*) / (N + 1)`, `lambda = ln(omega / (1 - omega))`.
5. `PBO` is the share of combinations with `lambda <= 0`.

#### 5.10.8 Impact (Sato and Kanazawa)

`I(Q) / sigma_D = c * (Q / V_D)^(1/2)`, with `Q` the order size, `V_D` the daily volume and `sigma_D` the daily volatility. The design uses it as the fractional cost in step 2 of 5.10.5 with `c = impact_coeff`.

### 5.11 Error enums

Written in full by the scaffold. `String` payloads carry a human-readable detail. A variant that wraps another crate's error converts with `From`.

| Enum | Variants |
| --- | --- |
| `CoreError` | `InvalidSymbol(String)`, `NonPositivePrice`, `NotFinite`, `InvalidOrder(String)`, `InvalidCost(String)` |
| `CalendarError` | `Empty`, `NotSorted`, `Duplicate(NaiveDate)` |
| `ConfigError` | `Io(String)`, `Parse(String)`, `Invalid(String)`, `MissingCredentials`, `InsecureSecretsFile(String)` |
| `StoreError` | `Sql(String)`, `Duplicate(String)`, `IllegalTransition { from: String, to: String }`, `SchemaTooNew { found: i64, supported: i64 }`, `NotFound(String)`, `Codec(String)` |
| `AlpacaError` | `EndpointNotAllowed(String)`, `RateLimited`, `Server { status: u16 }`, `Rejected { status: u16, message: String }`, `FeedNotPermitted { feed: String, message: String }`, `PermitMismatch`, `UnexpectedBarTime(String)`, `Parse(String)`, `Transport(String)` |
| `DataError` | `Store(StoreError)`, `Alpaca(AlpacaError)`, `Csv { line: usize, message: String }`, `BadBar { symbol: String, date: NaiveDate, message: String }`, `Invalid(String)` |
| `StrategyError` | `MissingRiskFree(NaiveDate)`, `NotASession(NaiveDate)`, `Invalid(String)` |
| `PortfolioError` | `MissingPrice(String)`, `Invalid(String)` |
| `EngineError` | `MissingBar { symbol: String, date: NaiveDate }`, `DataNotReady { symbol: String, date: NaiveDate }`, `MissingImpactInputs(String)`, `Strategy(StrategyError)`, `Portfolio(PortfolioError)`, `Core(CoreError)`, `Invalid(String)` |
| `VectorError` | `Shape(String)`, `NonPositivePrice`, `UnknownTradeDate(NaiveDate)`, `ImpactNotSupported`, `LengthMismatch` |
| `StatsError` | `OutOfDomain(String)`, `TooFewObservations`, `InvalidPartition(String)` |
| `GateError` | `StageForbids`, `StrategyNotAllowed`, `Halted`, `FlatNotConfirmed`, `FirstOrderMismatch`, `FirstOrderAlreadySent`, `OwnerFileInRepo`, `Parse(String)` |
| `LiveError` | `AlreadyRunning`, `UnknownStatus(String)`, `UnexpectedStatus(String)`, `Gate(GateError)`, `Store(StoreError)`, `Alpaca(AlpacaError)`, `Data(DataError)`, `Engine(EngineError)`, `Invalid(String)` |
| `ShadowError` | `Store(StoreError)`, `Data(DataError)`, `Engine(EngineError)`, `NotFound(String)`, `Io(String)`, `Invalid(String)` |
| `OpsError` | `Store(StoreError)`, `Io(String)` |
| `ValidateError` | `Store(StoreError)`, `Data(DataError)`, `Engine(EngineError)`, `Vector(VectorError)`, `Stats(StatsError)`, `NotComparable(String)`, `TooFewTrials`, `Invalid(String)` |

A fractional broker position is an `AlpacaError::Parse`: the run fails and alerts on every attempt until the owner removes it, so no separate halt reason exists for it.

Halt reasons are strings: `reconciliation_position`, `reconciliation_cash`, `foreign_order`, `account_not_flat`, `unexpected_order_status`, `daily_loss`, `drawdown`, `manual`, `flattened`.

Alert codes are strings: `data_not_ready`, `feed_error`, `broker_error`, `order_rejected`, `order_abandoned`, `incomplete_fill`, `late_rebalance_skipped`, `halt_set`, `missed_run`, `not_tradable`.

## 6. Decisions

| ID | Decision | Alternative rejected | Why |
| --- | --- | --- | --- |
| D-01 | One run-to-completion command, `trader run`, does every phase that is due and is safe to repeat. Recommended schedule: 08:00, 09:45 and 17:30 New York time on weekdays | A daemon | `decisions.md`; no websocket is needed for end-of-day trading; a crash leaves nothing running |
| D-02 | Every phase is driven by stored state: ingest what is missing, settle what is open, snapshot sessions without a snapshot, rebalance if a signal date has no decision | A phase argument per cron entry | The same command catches up after a missed day and cannot run phases out of order |
| D-03 | Signals at the month-end close, market day orders for the next session, queued before the open | Market-on-open or market-on-close orders | Forum reports say closing-auction orders on paper are partly filled and then expire; a day order keeps working through random partial fills |
| D-04 | A position that changes sign is traded as two orders, the opening leg sent only after the closing leg is filled | One crossing order | Reported as rejected by Alpaca; the two-leg plan is correct whether or not that is still true |
| D-05 | Unfilled remainders are not resent. An alert is raised and the next rebalance corrects the position | Automatic resend | Keeps one decision equal to one set of orders, which the replay check depends on |
| D-06 | Two bar series per symbol, `raw` and `all`. Signals use `all`. Research backtests fill and mark on `all`; paper comparison uses `raw` | Computing adjustments ourselves from corporate actions | Less code and less risk. A split in a held ETF during paper trading shows up as a reconciliation break and halts the system; automatic handling is out of scope |
| D-07 | Blocking HTTP with `ureq`, `rusqlite` with bundled SQLite, no async runtime | tokio, reqwest, sqlx | No concurrent I/O exists in this design; fewer dependencies and simpler tests |
| D-08 | Bars are append-only and versioned by fetch time, and each decision records the instant its data was fetched | Overwriting bars | Adjusted history is restated at every dividend. Replaying a decision needs the bars as they were. The cost is storage: a dividend rewrites one symbol's history |
| D-09 | The adjusted series is fetched in full on every ingest; the raw series from five sessions before the last stored bar | Incremental adjusted ingest | Restatements cannot be seen incrementally. About 25 requests a day, far below the limit |
| D-10 | Money is `Decimal`; statistics and signals are `f64`. Weights cross the boundary rounded to 6 places; costs are rounded to 6 places before they touch cash | Floats throughout | Two independent implementations can then agree exactly, with or without costs |
| D-11 | Bar prices are converted from `f64` through the shortest round-trip decimal string | `serde_json` arbitrary precision | Avoids a feature that changes number handling in every crate; the shortest form reproduces the literal Alpaca printed |
| D-12 | The vectorised calculator receives the weight schedule and re-implements share rounding, fills, costs, borrow and marking | Feeding it the engine's fills | Otherwise it would check nothing. It does not re-implement the pre-trade checks, so a run with rejections is declared not comparable |
| D-13 | Weights depend only on data up to the signal date, never on equity or positions | Path-dependent sizing | The weight schedule can be computed once and given to both calculators |
| D-14 | Full exponentially weighted covariance, same decay as the volatility, no shrinkage | Ignoring correlation; Ledoit-Wolf | A portfolio volatility target needs a portfolio volatility. Shrinkage is a parameter choice the owner has not made; it is on the roadmap |
| D-15 | Trading halts from equity (daily loss, drawdown) are enforced in paper trading and only recorded in research backtests | Enforcing them in backtests | A halt needs an owner to clear it. A backtest has no owner |
| D-16 | The owner file lives outside the repository and is only ever read | Approvals stored in SQLite by a command | Anyone who can run the binary could run that command. A file the binary cannot write keeps the approval with the owner |
| D-17 | `submit_order` consumes an `OrderPermit` that only `Gate::authorize` can create and that is bound to one order | A stage check at each call site | One choke point, enforced by the compiler |
| D-18 | The trading host is a constant. Overrides are accepted only for loopback addresses | A base URL in config | Makes "talks only to paper" a property of the binary, not of its configuration |
| D-19 | Fills come from the account activities endpoint, keyed by activity id; order polling only updates state | Deriving fills from order `filled_qty` | Each execution is recorded once with its own price, and re-reading is harmless |
| D-20 | Orders are written to the database as `Intent` before the POST, with a deterministic `client_order_id` | Submitting first | After a crash the next run looks the order up by id and adopts it instead of sending it twice |
| D-21 | The ledger is the source of P&L. Positions must equal the broker's exactly and cash within `cash_tolerance`, or the system halts | Trusting broker equity | Review guidance; it is also how foreign activity is detected |
| D-22 | The shadow comparison restarts each day from the paper book at the previous close | One continuous shadow account | Differences do not compound, and each day is comparable on its own |
| D-23 | Decision replay recomputes each stored decision from its stored inputs and the bars as of its data instant and requires equality | Comparing with an independently evolving simulation | Whole-share orders match exactly only from identical equity and positions |
| D-24 | Sign-off criteria measure fidelity only, need at least 63 sessions, 3 rebalances and 30 fills by default, and the report never treats return as a criterion | A profit target | Brief section 3: a few months cannot validate a strategy |
| D-25 | The first rebalance after entering stage `paper` is exempt from the lateness limit | Waiting for the next month end | Otherwise up to a month of the validation window is idle |
| D-26 | A trial is a distinct hash of strategy, parameters, universe, period and cost profile. Re-running the same one increments a counter, not the trial count | Counting every run | Repeating a run is not a new attempt at finding a result |
| D-27 | PBO counts `lambda <= 0` as overfit and needs at least two registered trials | Strict inequality | The paper's integral is ambiguous at zero; this is the conservative reading. With one fixed configuration PBO is undefined and the command says so |
| D-28 | `Cargo.lock` is committed and marked `-diff` in `.gitattributes`. The `workspace` chain declares every dependency of every crate | Ignoring the lock file; adding dependencies chain by chain | Builds are reproducible, the generated file does not count against step size, and parallel chains never touch `Cargo.toml` or `Cargo.lock` |
| D-29 | Idle cash earns nothing and short proceeds earn nothing in the simulation | Crediting the bill rate | Conservative, and paper does not credit it either. Research Sharpe ratios are computed on returns in excess of the risk-free proxy |
| D-30 | No no-trade band. Every non-zero share difference is traded at the monthly rebalance | Partial rebalancing | Garleanu and Pedersen's rule needs a cost-aversion parameter nobody has chosen; it is on the roadmap |
| D-31 | The candidate universe is a list in the example config, checked by `trader universe verify` | A hard-coded universe | Tradability, borrow status and history can only be confirmed with keys |

## 7. Testing

### 7.1 Levels

| Level | What | Where |
| --- | --- | --- |
| Unit | Pure functions against hand-computed golden values: volatility, signal, weights, orders, costs, statistics | `crates/<name>/tests/` of core, calendar, strategy, portfolio, risk, engine, vector, stats, gate |
| Integration, in memory | Store repositories against `Store::open_in_memory()`; engine against `MemoryView` | store, engine, validate, shadow, ops |
| Integration, mock HTTP | Client, ingest and the live cycle against `pt-mock-alpaca` on a loopback port. No test reaches the internet and none needs keys | alpaca, data, live |
| Cross-implementation | Engine against the vector calculator on a seeded synthetic data set: equal to the last digit at zero cost, within $0.01 with costs | `crates/validate/tests/` |
| End to end | The `trader` binary driven as a process against temporary directories: offline import to backtest to validation; a mock paper session from pre-open run to report | `crates/cli/tests/` |
| Policy | Member list, licences, dependency boundaries, no live endpoint string | `crates/archtest/tests/` |
| Owner smoke | `trader doctor` with real keys | run by the owner, not by CI |

### 7.2 How each component is proven

| Component | Acceptance |
| --- | --- |
| W | R-001 to R-007 |
| A, B | Validation and arithmetic checks with exact decimal results; the 2024 exchange calendar as fixture |
| C, N | Parsing, defaults, rejections; the stage capability table; a `compile_fail` doctest proving a permit cannot be forged |
| D | Versioned bars and as-of reads; append-only triggers; transition checks; idempotent fills |
| E, F | Request shape, pagination, retries with a fake sleeper, loopback-only overrides; the mock's own behaviour |
| G | Ingest against the mock, request budget, gap and sanity reports, CSV import atomicity |
| H, I, J | Golden values in 5.10 and in the requirement rows; look-ahead tests that change later bars and expect identical output |
| K, L | The worked three-session example (equity 100000, 101000, 101802 at zero cost; 100000, 100922.60, 101628.2602 with costs); truncation and missing-bar errors |
| M | The Deflated Sharpe paper example; a 12 x 4 golden matrix for PBO |
| V | Trial registration, cross-check verdicts, registry-driven DSR and PBO |
| O | Each failure drill from the review as a test: crash between POST and record, data API down, restart with a partly filled order, foreign order, foreign position |
| P | Replay equality and tamper detection; zero difference on non-trade days; slippage sign convention |
| Q | Webhook delivery and failure tolerance against a local listener; report contents and atomic write |
| R | Exit codes, stage refusals with zero HTTP requests, the two end-to-end runs |

### 7.3 Test rules

- No sleeps and no wall-clock waits. Time comes from `Clock`, `MonoClock` and `Sleeper` fakes.
- No network beyond loopback. No credentials.
- Synthetic prices are generated in the test from a fixed seed with a small linear congruential generator written in the test, so no random-number crate is needed.
- Fixture JSON is written by hand from section 5.9 and kept under `crates/alpaca/tests/fixtures/`.

### 7.4 What the integration check must run

`CHECK` already runs the Rust checks below once a root `Cargo.toml` exists, so no swap is needed when the `workspace` chain lands. The full command is:

```
test -f README.md && ! git ls-files | grep -qi "\.pdf$" && cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test --workspace --locked && cargo build --workspace --locked
```

CI needs the pinned toolchain with `rustfmt` and `clippy`, a cargo cache and a C compiler. It needs no secrets.

## 8. Owner steps

The owner performs each step once. The binary checks every one at run time and `trader gates` prints what is pending. No chain waits on any of them.

| # | Owner step | When it is needed | How the binary sees it |
| --- | --- | --- | --- |
| 1 | Fix the build machine: `~/.cargo/bin` on `PATH`, network for rustup | Before the first chain | Not a runtime gate |
| 2 | Generate paper API keys and export them or write the secrets file | Before stage `ingest` | `load_api_keys` |
| 3 | Confirm the data plan (Basic assumed) and run `trader doctor`; set `data.feed` to what it reports | Before the first ingest | `doctor` output; `FeedNotPermitted` |
| 4 | Run `trader universe verify`; remove failing candidates from the config | Before the first real backtest | Verify report |
| 5 | Set `stage = "ingest"`, then `"dry_run"` in the owner file | To start ingest; to start dry runs | `Gate::allows` |
| 6 | Choose the machine, install the schedule from `deploy/`, set the alert webhook URL | Before unattended runs | Owner file or environment |
| 7 | Reset the paper account to $100k or confirm it is flat; set `account_flat_confirmed_on` | Before the first order | `FlatNotConfirmed`; the binary also checks that the account is flat |
| 8 | Approve the first order: `[first_order]` with symbol, side, `max_qty`, `valid_on`; set `stage = "first_order"` | Once, after dry runs look right | `Gate::authorize` |
| 9 | Add the strategy id to `strategy_allowlist`; set `stage = "paper"` | To start full paper trading | `StrategyNotAllowed` |
| 10 | Read `signoff report`; record the outcome in `validation_signoff` | After the validation window | Shown in the status report |
| 11 | Clear halts after investigating: `trader halt clear` | Whenever the system halts | `halts` table |
| 12 | Copy what they want from `status.md` into the README | Any time | Not seen by the binary |

## 9. Roadmap

| Phase | Operating stage | What it delivers | Chains (see `plan.md`) | Exit check |
| --- | --- | --- | --- | --- |
| 0 Foundation | none | Workspace, types, calendar, config, store, statistics, mock broker | workspace, core-*, calendar, config, config-hash, secrets, store-*, stats-*, mock-* | Policy tests pass |
| 1 Offline backtest | `backtest` | Strategy, sizing, risk checks, engine, vector cross-check, trial registry, DSR, PBO, benchmark, CSV import | strategy-*, portfolio-*, risk, engine-*, vector, validate-*, data-import, data-view, data-quality, cli-base, cli-import, cli-backtest, cli-validate | Engine and vector calculator agree exactly at zero cost |
| 2 Live data | `ingest` | Alpaca client, ingest, calendar and asset sync, universe verification, smoke test | alpaca-*, data-ingest, data-reference, data-universe, data-doctor, cli-data | `doctor` and `universe verify` pass with the owner's keys |
| 3 Dry run | `dry_run` | Gates, decisions recorded without orders, alerts, report, schedule | gate-*, ops-*, live-ids, live-lock, live-context, live-snapshots, live-rebalance, live-rebalance-timing, cli-deploy | A dry-run decision appears after each month end and replays exactly |
| 4 One order | `first_order` | Submission, settlement from fills, ledger, reconciliation | live-submit, live-settle, live-legs, live-reconcile, live-first-order | The approved order fills and the ledger equals the broker |
| 5 Full paper | `paper` | Monthly rebalances, shadow comparison, fidelity metrics, sign-off report, kill switch | live-cycle, live-cycle-failures, live-killswitch, shadow-*, cli-live, cli-ops, cli-e2e | Sign-off criteria pass over the window; the owner signs off |

Every phase is built and tested before any owner step, against the mock broker. The stages are how the owner turns the phases on. A chain is listed under the first phase that needs it; the order chains are built in is the wave table in `plan.md`, which follows code dependencies, not phases.

Later work, each needing its own design: intraday bars and realised-volatility (HAR) forecasts; the `trade_updates` websocket; order-book strategies; options; crypto; machine-learning signal combination; LLM text features and agents; covariance shrinkage and optimisation with `clarabel`; partial rebalancing; automatic corporate-action handling; a Parquet research store; publishing the report.

## 10. Conventions for implementers

`CLAUDE.md` leaves conventions as TODO. Until it is filled in, these apply.

- Rust 2021 edition, toolchain 1.100.0, `cargo fmt` defaults, `clippy` with warnings denied, `unsafe` forbidden.
- Library crates return their own `thiserror` enum. Only `pt-cli` uses `anyhow`. No `unwrap` or `expect` outside tests.
- Tests are integration tests under `crates/<name>/tests/`, one file per topic, test names stating the behaviour, with the requirement ID in a comment.
- No function reads the wall clock or the environment directly except in `pt-cli` and `SystemClock`.
- Secrets are never logged, stored or put in an error message.

## 11. Open questions

none

# paper-trader: plan

Stage 3 of the design, built from `requirements.md` and `architecture.md`. A chain is one story and becomes the branch `chain/paper-trader/<chain>`; a step is one commit on it. Steps run in table order within their chain.

How to read the tables:

- `After` lists the chains whose code a chain calls. The `workspace` chain declares every crate, dependency, module file and error enum, so no later chain edits a `lib.rs`, an `error.rs`, a `Cargo.toml` or `Cargo.lock`. That is what lets chains in one wave run side by side.
- Each step fills in module files that already exist (empty) and adds one test file of its own. A step implements the signatures of architecture section 5 for what its text names, and nothing else.
- A `tests/common/mod.rs` (or `*_fixture/mod.rs`) file is a shared test fixture. The step that lists it writes it; later chains in the same crate read it and do not change it.
- Tier `opus` marks migrations, performance work, and steps that cross several crates or carry the safety argument; the rest are `sonnet`.
- If a step needs a dependency the scaffold did not declare, that is a replan of the `workspace` chain, not an edit in place.
- The Steps table uses no code formatting, because step text becomes a commit subject and the other cells are read by scripts.

## Chains

| Chain | Story | After |
| --- | --- | --- |
| workspace | The repository is a Cargo workspace of twenty crates with every dependency, module and error enum declared, guarded by policy tests. | - |
| core-types | Symbols, prices, money, order intents and cost specifications are validated value types with exact decimal arithmetic. | workspace |
| core-state | Order states follow one transition table and time is injectable. | workspace |
| core-view | Strategies read prices through view traits that cannot return data dated after the date asked for. | core-types |
| calendar | There is one trading calendar type that answers session, neighbour, range and day-count questions. | workspace |
| calendar-months | The calendar knows month ends and produces the monthly rebalance schedule. | calendar |
| secrets | API keys load from the environment or a private file and never print. | workspace |
| config | The typed config parses with defaults, rejects what the owner ruled out, and ships with the candidate universe. | core-types |
| config-hash | A config has a stable hash that depends on its values only. | config |
| stats-descriptive | Returns can be summarised: moments, Sharpe ratio, drawdown, growth and a simple regression. | workspace |
| stats-normal | The normal distribution function and its inverse are available to the stated accuracy. | workspace |
| stats-dsr | The Deflated Sharpe Ratio reproduces the worked example in its paper. | stats-normal |
| stats-pbo | The Probability of Backtest Overfitting is computed by combinatorially symmetric cross-validation, fast enough for sixteen partitions. | workspace |
| vector | A second, deliberately simple calculator turns a weight schedule into an equity series. | core-types |
| alpaca-endpoints | The client has endpoints locked to paper and loopback, credentials that never print, and a rate limiter with injected time. | workspace |
| alpaca-http | One HTTP client sends authenticated requests and retries only what is safe to retry. | alpaca-endpoints |
| alpaca-data | Daily bars are fetched, paged and parsed into exact decimals. | alpaca-http |
| alpaca-account | Account, positions, calendar and asset status are read from the trading API. | alpaca-http |
| alpaca-orders-read | Orders and fill activities can be read back, to the last page. | alpaca-http |
| alpaca-write | An order can be sent only by spending a permit issued for exactly that order. | alpaca-orders-read, gate-authorize |
| mock-reference | Tests can start a loopback Alpaca double that serves account, clock, calendar, asset and bar data and injects failures. | workspace |
| mock-orders | The double accepts orders and lets a test script fills, expiries, odd statuses and foreign activity. | mock-reference |
| store-codec | Decimals survive the database unchanged. | workspace |
| store-schema | Opening the database creates the whole schema, versions it and refuses a newer one. | workspace |
| store-market | Bars are stored as versions readable as of any instant, next to the calendar and asset snapshots. | store-schema, store-codec, core-types, calendar |
| store-orders | Decisions, orders, their state history and fills are recorded once each. | store-schema, store-codec, core-types, core-state |
| store-ledger | Positions, cash and equity snapshots come from an append-only ledger. | store-orders |
| store-shadow | Shadow comparison rows and per-order slippage are stored. | store-schema, store-codec |
| store-ops | Runs, halts, alerts, trials and backtest results are recorded. | store-schema, store-codec |
| store-append-only | The database itself refuses to change or delete bars, fills and ledger entries. | store-market, store-ledger |
| gate-owner | The operating stage comes from an owner file outside the repository that the binary only reads. | workspace |
| gate-authorize | Every order needs a permit that only the gate can issue, under the rules of the current stage. | gate-owner |
| gate-status | The owner can see which of their steps are done and which are pending. | gate-owner |
| strategy-vol | Volatility and covariance are the exponentially weighted estimates of the momentum paper. | workspace |
| strategy-risk | Risk estimates are built per rebalance from adjusted closes, with ineligible symbols named. | strategy-vol, core-view |
| strategy-tsmom | Time-series momentum signals are the sign of the excess return over one or several lookbacks. | core-view, calendar-months |
| strategy-benchmark | A long-only strategy gives the volatility-scaled buy-and-hold benchmark. | strategy-tsmom |
| portfolio-weights | Signals become weights scaled to a portfolio volatility target under weight and gross caps. | core-view |
| portfolio-orders | Weights become whole-share targets and an ordered plan of orders, with sign changes split in two legs. | core-types |
| risk | Orders pass pre-trade checks, and equity is checked against the loss and drawdown limits. | core-types |
| engine-cost | Trading cost, impact, borrow cost and the net-position book are exact decimal functions. | core-types |
| engine-decide | One pure function turns signals and risk estimates into weights, targets and checked orders, for backtest and paper alike. | strategy-risk, strategy-tsmom, portfolio-weights, portfolio-orders, risk, config |
| engine-sim | The event-driven simulation reproduces the worked example to the last digit, with and without costs. | engine-cost, portfolio-orders, risk, core-view, calendar |
| engine-sim-guards | The simulation fails loudly instead of guessing, and records what the risk checks did. | engine-sim |
| engine-backtest | A backtest is the simulation of a weight schedule computed from data up to each signal date. | engine-sim-guards, engine-decide, calendar-months |
| data-view | The engine reads stored bars through a view fixed to one feed, one price basis and one instant. | store-market, core-view |
| data-import | Bars and the calendar can be loaded from CSV files, all or nothing. | store-market |
| data-quality | Bad bars and missing sessions are detected and reported, never repaired. | store-market, config, alpaca-data |
| data-ingest | One call brings raw and adjusted bars up to date and says whether the data is complete. | data-quality, mock-reference |
| data-reference | The calendar and asset status are synced from Alpaca into the store. | store-market, config, alpaca-account, mock-reference |
| data-universe | Each candidate instrument is checked for tradability, borrow status and usable history. | data-quality, alpaca-account, mock-reference |
| data-doctor | A smoke test reports, line by line, whether the real account and data plan behave as the design assumes. | alpaca-account, alpaca-data, mock-reference, config |
| ops-logging | A log event is one JSON line. | workspace |
| ops-pnl | Day, month, year and inception returns are computed from the snapshot series. | workspace |
| ops-alerts | An alert is always stored and logged, and sent to the webhook when one is set. | store-ops, ops-logging |
| ops-report | A local status report is written atomically as JSON and Markdown, whatever state the system is in. | ops-pnl, store-ops, store-ledger, store-market, config |
| validate-backtest | A research backtest is run from the store, saved, measured and registered as a trial. | engine-backtest, strategy-benchmark, data-view, store-ops, config-hash, stats-descriptive, core-state |
| validate-crosscheck | The engine and the vector calculator agree exactly at zero cost and within a cent with costs. | validate-backtest, vector |
| validate-overfit | Overfitting statistics and the benchmark comparison are computed from stored runs and the trial registry. | validate-backtest, stats-dsr, stats-pbo |
| live-ids | Client order ids are deterministic and every Alpaca order status maps to an internal state or an error. | core-state |
| live-lock | Only one run can work on a database at a time. | workspace |
| live-context | The live code has its context type, its reading of the broker clock, and one shared test fixture. | gate-authorize, ops-alerts, alpaca-account, alpaca-orders-read, alpaca-data, mock-orders, store-ledger, store-market, store-ops, config, strategy-tsmom, core-state |
| live-submit | Planned orders are written before they are sent, and a retry adopts what the broker already has. | live-context, alpaca-write, live-ids |
| live-settle | Order states and fills are pulled from the broker and each fill becomes one ledger entry. | live-submit |
| live-legs | An opening leg is sent only after its closing leg has filled, and unexpected broker states stop trading. | live-settle |
| live-reconcile | The ledger is compared with the broker on every run, and any difference or foreign activity halts trading. | live-context |
| live-snapshots | The paper book is valued at each stored close, missed sessions are caught up, and equity halts are set. | live-context, risk |
| live-rebalance | A due rebalance is decided once, recorded with its inputs, and planned as orders, or as a dry run. | live-reconcile, live-snapshots, live-ids, engine-decide, data-view, data-ingest, data-reference, calendar-months |
| live-rebalance-timing | A rebalance respects lateness, the close and what the broker says can be traded and shorted. | live-rebalance |
| live-cycle | One idempotent run does every phase the stage allows, in a fixed order, and logs itself. | live-rebalance-timing, live-legs |
| live-cycle-failures | A halted, failing or data-starved run ends cleanly and sends nothing. | live-cycle |
| live-first-order | In the first-order stage the one approved order is sent once and nothing else. | live-cycle-failures |
| live-killswitch | The owner can cancel every open order and flatten the account, even while halted. | live-settle |
| shadow-slippage | Each filled order has a recorded slippage against the open of its trade date. | store-shadow, store-ledger, store-market, config |
| shadow-daily | Each session of paper trading is compared with a shadow simulation restarted from the paper book. | shadow-slippage, engine-cost |
| shadow-replay | Every stored decision can be recomputed from its stored inputs and the bars as they were. | engine-decide, data-view, store-orders, config-hash |
| shadow-fidelity | The paper period is summarised as counts of sessions, fills, breaks, misses and differences. | shadow-replay, shadow-daily, store-ops |
| shadow-signoff | A sign-off report judges fidelity only, and says not-ready until the sample is large enough. | shadow-fidelity |
| cli-base | The `trader` binary parses every command, loads config, owner file and keys, and refuses what the stage forbids before any command runs. | config-hash, gate-status, secrets, store-orders, store-market, alpaca-http, ops-logging |
| cli-deploy | Example schedules for cron and launchd are in the repository. | workspace |
| cli-import | Calendar and bars can be imported and checked from the command line with no keys and no network. | cli-base, data-import, data-quality |
| cli-data | Calendar sync, ingest, universe verification and the smoke test run from the command line. | cli-import, data-ingest, data-reference, data-universe, data-doctor |
| cli-backtest | A research backtest and the engine cross-check run from the command line. | cli-import, validate-crosscheck, validate-overfit |
| cli-validate | The overfitting statistics and the benchmark comparison run from the command line. | cli-backtest |
| cli-live | `trader run` runs one cycle under the run lock, updates the shadow comparison and the report, and reports the outcome by exit code. | cli-base, live-first-order, live-lock, shadow-fidelity, ops-report |
| cli-ops | The owner can inspect and clear halts, use the kill switch, and write the status and sign-off reports. | cli-live, live-killswitch, shadow-signoff |
| cli-e2e | The offline research path and a mock paper session work end to end through the binary. | cli-ops, cli-validate, cli-data |

## Steps

| ID | Chain | Step | Reqs | Tier | Files | Proof |
| --- | --- | --- | --- | --- | --- | --- |
| W01 | workspace | Add the root manifest with every workspace dependency and lint, the toolchain pin, the ignore and attribute rules, and the pt-archtest crate | R-002, R-006 | opus | Cargo.toml, Cargo.lock, rust-toolchain.toml, .gitignore, .gitattributes, crates/archtest/ | cargo test -p pt-archtest --test repo_files |
| W02 | workspace | Scaffold pt-core, pt-calendar, pt-stats and pt-vector: manifest, lib.rs, empty modules, error enum | R-007 | opus | Cargo.lock, crates/core/, crates/calendar/, crates/stats/, crates/vector/, crates/archtest/tests/lints.rs | cargo build --workspace && cargo test -p pt-archtest --test lints |
| W03 | workspace | Scaffold pt-config, pt-gate and pt-store | R-003 | opus | Cargo.lock, crates/config/, crates/gate/, crates/store/, crates/archtest/tests/licences.rs | cargo build --workspace && cargo test -p pt-archtest --test licences |
| W04 | workspace | Scaffold pt-alpaca, pt-mock-alpaca and pt-data | R-005 | opus | Cargo.lock, crates/alpaca/, crates/mock-alpaca/, crates/data/, crates/archtest/tests/forbidden_strings.rs | cargo build --workspace && cargo test -p pt-archtest --test forbidden_strings |
| W05 | workspace | Scaffold pt-strategy, pt-portfolio, pt-risk, pt-engine and pt-validate | R-004 | opus | Cargo.lock, crates/strategy/, crates/portfolio/, crates/risk/, crates/engine/, crates/validate/, crates/archtest/tests/boundaries.rs | cargo build --workspace && cargo test -p pt-archtest --test boundaries |
| W06 | workspace | Scaffold pt-ops, pt-live, pt-shadow and pt-cli (a trader binary that only exits 0) | R-001 | opus | Cargo.lock, crates/ops/, crates/live/, crates/shadow/, crates/cli/, crates/archtest/tests/members.rs | cargo build --workspace && cargo test -p pt-archtest |
| A01 | core-types | Add Symbol with validation | R-010 | sonnet | crates/core/src/types.rs, crates/core/tests/symbol.rs | cargo test -p pt-core --test symbol |
| A02 | core-types | Add Price, Money and exact notional | R-011, R-014 | sonnet | crates/core/src/money.rs, crates/core/tests/price.rs | cargo test -p pt-core --test price |
| A03 | core-types | Add round6, midpoint away from zero | R-013 | sonnet | crates/core/src/money.rs, crates/core/tests/round6.rs | cargo test -p pt-core --test round6 |
| A04 | core-types | Add Price::from_f64_shortest | R-012 | sonnet | crates/core/src/money.rs, crates/core/tests/price_f64.rs | cargo test -p pt-core --test price_f64 |
| A05 | core-types | Add Side, Adjustment, PriceBasis, Bar, Fill, Weights, Positions and the validated OrderIntent | R-015 | sonnet | crates/core/src/types.rs, crates/core/tests/order_intent.rs | cargo test -p pt-core --test order_intent |
| A06 | core-types | Add CostSpec with zero and validate | R-016 | sonnet | crates/core/src/cost.rs, crates/core/tests/cost_spec.rs | cargo test -p pt-core --test cost_spec |
| A07 | core-state | Add OrderState and transition_allowed | R-017 | sonnet | crates/core/src/order_state.rs, crates/core/tests/order_state.rs | cargo test -p pt-core --test order_state |
| A08 | core-state | Add Clock, SystemClock and FixedClock | R-019 | sonnet | crates/core/src/clock.rs, crates/core/tests/clock.rs | cargo test -p pt-core --test clock |
| A09 | core-view | Add SignalSet, RiskEstimates and the HistoryView and MarketView traits | R-018 | sonnet | crates/core/src/signal.rs, crates/core/src/view.rs, crates/core/tests/view_traits.rs | cargo test -p pt-core --test view_traits |
| A10 | core-view | Add MemoryView adjusted closes: on or before the date, oldest first, capped | R-018 | sonnet | crates/core/src/view.rs, crates/core/tests/memory_view_history.rs | cargo test -p pt-core --test memory_view_history |
| A11 | core-view | Add MemoryView execution bars and average volume | R-018 | sonnet | crates/core/src/view.rs, crates/core/tests/memory_view_exec.rs | cargo test -p pt-core --test memory_view_exec |
| B01 | calendar | Add TradingCalendar::new with validation, and is_session | R-020 | sonnet | crates/calendar/src/calendar.rs, crates/calendar/tests/common/mod.rs, crates/calendar/tests/calendar_new.rs | cargo test -p pt-calendar --test calendar_new |
| B02 | calendar | Add next_session and prev_session | R-021 | sonnet | crates/calendar/src/calendar.rs, crates/calendar/tests/next_prev.rs | cargo test -p pt-calendar --test next_prev |
| B03 | calendar | Add sessions_between, inclusive at both ends | R-022 | sonnet | crates/calendar/src/calendar.rs, crates/calendar/tests/sessions_between.rs | cargo test -p pt-calendar --test sessions_between |
| B04 | calendar | Add calendar_days_to_next | R-023 | sonnet | crates/calendar/src/calendar.rs, crates/calendar/tests/days_to_next.rs | cargo test -p pt-calendar --test days_to_next |
| B05 | calendar-months | Add month_end_sessions | R-024 | sonnet | crates/calendar/src/calendar.rs, crates/calendar/tests/month_end.rs | cargo test -p pt-calendar --test month_end |
| B06 | calendar-months | Add month_end_back | R-025 | sonnet | crates/calendar/src/calendar.rs, crates/calendar/tests/month_end_back.rs | cargo test -p pt-calendar --test month_end_back |
| B07 | calendar-months | Add Rebalance and monthly_schedule | R-026 | sonnet | crates/calendar/src/schedule.rs, crates/calendar/tests/schedule.rs | cargo test -p pt-calendar --test schedule |
| C01 | secrets | Add Secret and ApiKeys with redacted Debug and Display | R-043 | sonnet | crates/config/src/secrets.rs, crates/config/tests/secret_redaction.rs | cargo test -p pt-config --test secret_redaction |
| C02 | secrets | Load API keys from the environment map | R-041 | sonnet | crates/config/src/secrets.rs, crates/config/tests/api_keys_env.rs | cargo test -p pt-config --test api_keys_env |
| C03 | secrets | Load API keys from the file named by PT_SECRETS_FILE | R-041 | sonnet | crates/config/src/secrets.rs, crates/config/tests/api_keys_file.rs | cargo test -p pt-config --test api_keys_file |
| C04 | secrets | Refuse a secrets file readable by group or others | R-042 | sonnet | crates/config/src/secrets.rs, crates/config/tests/api_keys_mode.rs | cargo test -p pt-config --test api_keys_mode |
| C12 | config | Add parse for a Config that holds only the universe of Instruments with the six asset classes | R-031 | sonnet | crates/config/src/config.rs, crates/config/tests/parse_universe.rs | cargo test -p pt-config --test parse_universe |
| C13 | config | Add the run, data, strategy and stats sections (strings, integers and one date) with their architecture 5.3 defaults | R-031 | sonnet | crates/config/src/config.rs, crates/config/tests/defaults_plain.rs | cargo test -p pt-config --test defaults_plain |
| C14 | config | Add the sizing and risk sections, holding fractional values as exact Decimals read from TOML numbers | R-031 | sonnet | crates/config/src/config.rs, crates/config/tests/defaults_limits.rs | cargo test -p pt-config --test defaults_limits |
| C15 | config | Add the cost section as two CostSpec values whose research and shadow keys default separately | R-031 | sonnet | crates/config/src/config.rs, crates/config/tests/defaults_cost.rs | cargo test -p pt-config --test defaults_cost |
| C05 | config | Add the signoff section, completing the defaults of architecture 5.3 | R-031 | sonnet | crates/config/src/config.rs, crates/config/tests/defaults.rs | cargo test -p pt-config --test defaults |
| C06 | config | Reject unknown keys by name | R-032, R-033 | sonnet | crates/config/src/config.rs, crates/config/tests/unknown_keys.rs | cargo test -p pt-config --test unknown_keys |
| C07 | config | Validate the gross cap, the feed name and impact-off-for-IEX | R-034, R-035, R-036 | sonnet | crates/config/src/config.rs, crates/config/tests/validation_limits.rs | cargo test -p pt-config --test validation_limits |
| C08 | config | Validate the universe | R-037 | sonnet | crates/config/src/config.rs, crates/config/tests/validation_universe.rs | cargo test -p pt-config --test validation_universe |
| C09 | config | Range-check strategy, sizing, risk and statistics parameters | R-038 | sonnet | crates/config/src/config.rs, crates/config/tests/validation_ranges.rs | cargo test -p pt-config --test validation_ranges |
| C10 | config | Add load and the example config with the 37-symbol candidate universe | R-030, R-039 | sonnet | crates/config/src/config.rs, config/paper-trader.example.toml, crates/config/tests/example_config.rs | cargo test -p pt-config --test example_config |
| C11 | config-hash | Add config_hash | R-040 | sonnet | crates/config/src/hash.rs, crates/config/tests/config_hash.rs | cargo test -p pt-config --test config_hash |
| M01 | stats-descriptive | Add mean, std_sample and returns_from_equity | R-230 | sonnet | crates/stats/src/descriptive.rs, crates/stats/tests/mean_std.rs | cargo test -p pt-stats --test mean_std |
| M02 | stats-descriptive | Add skewness and kurtosis | R-231 | sonnet | crates/stats/src/descriptive.rs, crates/stats/tests/moments.rs | cargo test -p pt-stats --test moments |
| M03 | stats-descriptive | Add sharpe, annualized_sharpe and annualized_vol | R-232 | sonnet | crates/stats/src/perf.rs, crates/stats/tests/sharpe.rs | cargo test -p pt-stats --test sharpe |
| M04 | stats-descriptive | Add max_drawdown and cagr | R-233 | sonnet | crates/stats/src/perf.rs, crates/stats/tests/drawdown_cagr.rs | cargo test -p pt-stats --test drawdown_cagr |
| M05 | stats-descriptive | Add ols with the intercept t-statistic | R-245 | sonnet | crates/stats/src/ols.rs, crates/stats/tests/ols.rs | cargo test -p pt-stats --test ols |
| M06 | stats-normal | Add norm_cdf | R-234 | sonnet | crates/stats/src/normal.rs, crates/stats/tests/norm_cdf.rs | cargo test -p pt-stats --test norm_cdf |
| M07 | stats-normal | Add norm_inv_cdf with its domain check | R-235 | sonnet | crates/stats/src/normal.rs, crates/stats/tests/norm_inv_cdf.rs | cargo test -p pt-stats --test norm_inv_cdf |
| M08 | stats-dsr | Add expected_max_sharpe | R-236 | sonnet | crates/stats/src/dsr.rs, crates/stats/tests/expected_max_sharpe.rs | cargo test -p pt-stats --test expected_max_sharpe |
| M09 | stats-dsr | Add psr | R-238 | sonnet | crates/stats/src/dsr.rs, crates/stats/tests/psr.rs | cargo test -p pt-stats --test psr |
| M10 | stats-dsr | Add deflated_sharpe and prove the paper example | R-237, R-238 | sonnet | crates/stats/src/dsr.rs, crates/stats/tests/dsr_paper.rs | cargo test -p pt-stats --test dsr_paper |
| M11 | stats-dsr | Refuse inputs the formula cannot evaluate | R-239 | sonnet | crates/stats/src/dsr.rs, crates/stats/tests/dsr_invalid.rs | cargo test -p pt-stats --test dsr_invalid |
| M12 | stats-pbo | Add n_combinations | R-244 | sonnet | crates/stats/src/pbo.rs, crates/stats/tests/pbo_combinations.rs | cargo test -p pt-stats --test pbo_combinations |
| M13 | stats-pbo | Add pbo_cscv input validation | R-242 | sonnet | crates/stats/src/pbo.rs, crates/stats/tests/pbo_validation.rs | cargo test -p pt-stats --test pbo_validation |
| M14 | stats-pbo | Compute logits and PBO over every combination | R-240 | sonnet | crates/stats/src/pbo.rs, crates/stats/tests/pbo_golden.rs | cargo test -p pt-stats --test pbo_golden |
| M15 | stats-pbo | Drop the earliest rows that do not fill the partitions | R-241 | sonnet | crates/stats/src/pbo.rs, crates/stats/tests/pbo_truncation.rs | cargo test -p pt-stats --test pbo_truncation |
| M16 | stats-pbo | Prove a dominant trial gives zero | R-243 | sonnet | crates/stats/src/pbo.rs, crates/stats/tests/pbo_dominant.rs | cargo test -p pt-stats --test pbo_dominant |
| M17 | stats-pbo | Compute Sharpe ratios from per-block sums so sixteen partitions run in time | R-244 | opus | crates/stats/src/pbo.rs, crates/stats/tests/pbo_speed.rs | cargo test -p pt-stats --test pbo_speed |
| L01 | vector | Add VectorInputs and input validation | R-222 | sonnet | crates/vector/src/vector.rs, crates/vector/tests/validation.rs | cargo test -p pt-vector --test validation |
| L02 | vector | Compute equity at zero cost: whole-share targets, fills at the open, marks at the close | R-220 | sonnet | crates/vector/src/vector.rs, crates/vector/tests/zero_cost.rs | cargo test -p pt-vector --test zero_cost |
| L03 | vector | Add proportional cost and borrow | R-221 | sonnet | crates/vector/src/vector.rs, crates/vector/tests/costed.rs | cargo test -p pt-vector --test costed |
| L04 | vector | Add compare with a tolerance | R-223 | sonnet | crates/vector/src/vector.rs, crates/vector/tests/compare.rs | cargo test -p pt-vector --test compare |
| E01 | alpaca-endpoints | Add Endpoints::production with the two host constants | R-080 | sonnet | crates/alpaca/src/endpoints.rs, crates/alpaca/tests/endpoints.rs | cargo test -p pt-alpaca --test endpoints |
| E02 | alpaca-endpoints | Accept endpoint overrides for loopback hosts only | R-081 | sonnet | crates/alpaca/src/endpoints.rs, crates/alpaca/tests/endpoint_overrides.rs | cargo test -p pt-alpaca --test endpoint_overrides |
| E03 | alpaca-endpoints | Add Credentials with redacted Debug | R-083 | sonnet | crates/alpaca/src/auth.rs, crates/alpaca/tests/credentials.rs | cargo test -p pt-alpaca --test credentials |
| E04 | alpaca-endpoints | Add RateLimiter with MonoClock and Sleeper | R-084 | sonnet | crates/alpaca/src/limiter.rs, crates/alpaca/tests/limiter.rs | cargo test -p pt-alpaca --test limiter |
| E05 | alpaca-http | Add HttpClient with authenticated GET, proven through the clock endpoint | R-082, R-095 | sonnet | crates/alpaca/src/http.rs, crates/alpaca/src/reference.rs, crates/alpaca/tests/common/mod.rs, crates/alpaca/tests/http_auth.rs | cargo test -p pt-alpaca --test http_auth |
| E06 | alpaca-http | Add loopback_client for tests | R-103 | sonnet | crates/alpaca/src/http.rs, crates/alpaca/tests/loopback_client.rs | cargo test -p pt-alpaca --test loopback_client |
| E07 | alpaca-http | Retry 429 after fixed waits | R-085 | sonnet | crates/alpaca/src/http.rs, crates/alpaca/tests/retry_429.rs | cargo test -p pt-alpaca --test retry_429 |
| E08 | alpaca-http | Retry 5xx with doubling waits | R-086 | sonnet | crates/alpaca/src/http.rs, crates/alpaca/tests/retry_5xx.rs | cargo test -p pt-alpaca --test retry_5xx |
| E09 | alpaca-http | Report other 4xx without retrying | R-087 | sonnet | crates/alpaca/src/http.rs, crates/alpaca/tests/no_retry_4xx.rs | cargo test -p pt-alpaca --test no_retry_4xx |
| E25 | alpaca-data | Add HttpClient::get_data, a GET on the market data base URL | R-082 | sonnet | crates/alpaca/src/http.rs, crates/alpaca/tests/http_data_host.rs | cargo test -p pt-alpaca --test http_data_host |
| E10 | alpaca-data | Send the documented daily-bars query | R-088 | sonnet | crates/alpaca/src/data.rs, crates/alpaca/tests/bars_query.rs | cargo test -p pt-alpaca --test bars_query |
| E11 | alpaca-data | Parse the bars response into WireBar | R-089 | sonnet | crates/alpaca/src/data.rs, crates/alpaca/tests/fixtures/bars.json, crates/alpaca/tests/bars_parse.rs | cargo test -p pt-alpaca --test bars_parse |
| E12 | alpaca-data | Refuse a daily bar that is not at New York midnight | R-091 | sonnet | crates/alpaca/src/data.rs, crates/alpaca/tests/bar_time.rs | cargo test -p pt-alpaca --test bar_time |
| E13 | alpaca-data | Follow next_page_token to the end | R-090 | sonnet | crates/alpaca/src/data.rs, crates/alpaca/tests/bars_pagination.rs | cargo test -p pt-alpaca --test bars_pagination |
| E14 | alpaca-data | Report a refused feed as FeedNotPermitted | R-092 | sonnet | crates/alpaca/src/data.rs, crates/alpaca/tests/feed_refusal.rs | cargo test -p pt-alpaca --test feed_refusal |
| E15 | alpaca-account | Add account | R-093 | sonnet | crates/alpaca/src/account.rs, crates/alpaca/tests/fixtures/account.json, crates/alpaca/tests/account.rs | cargo test -p pt-alpaca --test account |
| E16 | alpaca-account | Add positions as signed whole shares | R-094 | sonnet | crates/alpaca/src/account.rs, crates/alpaca/tests/fixtures/positions.json, crates/alpaca/tests/positions.rs | cargo test -p pt-alpaca --test positions |
| E17 | alpaca-account | Add calendar | R-095 | sonnet | crates/alpaca/src/reference.rs, crates/alpaca/tests/fixtures/calendar.json, crates/alpaca/tests/calendar.rs | cargo test -p pt-alpaca --test calendar |
| E18 | alpaca-account | Add asset, reading borrow status from either documented field | R-096 | sonnet | crates/alpaca/src/reference.rs, crates/alpaca/tests/fixtures/asset.json, crates/alpaca/tests/asset.rs | cargo test -p pt-alpaca --test asset |
| E19 | alpaca-orders-read | Add order_by_client_id | R-097 | sonnet | crates/alpaca/src/orders_read.rs, crates/alpaca/tests/fixtures/order.json, crates/alpaca/tests/order_lookup.rs | cargo test -p pt-alpaca --test order_lookup |
| E20 | alpaca-orders-read | Add orders_since | R-098 | sonnet | crates/alpaca/src/orders_read.rs, crates/alpaca/tests/fixtures/orders.json, crates/alpaca/tests/orders_since.rs | cargo test -p pt-alpaca --test orders_since |
| E21 | alpaca-orders-read | Add fill_activities with paging | R-099 | sonnet | crates/alpaca/src/activities.rs, crates/alpaca/tests/fixtures/activities.json, crates/alpaca/tests/fill_activities.rs | cargo test -p pt-alpaca --test fill_activities |
| E22 | alpaca-write | Add submit_order, which consumes a matching OrderPermit | R-100 | opus | crates/alpaca/src/orders_write.rs, crates/alpaca/tests/submit_order.rs | cargo test -p pt-alpaca --test submit_order |
| E23 | alpaca-write | Report a rejected order without retrying | R-101 | sonnet | crates/alpaca/src/orders_write.rs, crates/alpaca/tests/submit_rejected.rs | cargo test -p pt-alpaca --test submit_rejected |
| E24 | alpaca-write | Add cancel_order | R-102 | sonnet | crates/alpaca/src/orders_write.rs, crates/alpaca/tests/cancel_order.rs | cargo test -p pt-alpaca --test cancel_order |
| F01 | mock-reference | Start a loopback server on a free port and record every request | R-110 | sonnet | crates/mock-alpaca/src/server.rs, crates/mock-alpaca/src/state.rs, crates/mock-alpaca/src/routes.rs, crates/mock-alpaca/tests/listen.rs | cargo test -p pt-mock-alpaca --test listen |
| F02 | mock-reference | Answer 401 without the auth headers | R-111 | sonnet | crates/mock-alpaca/src/server.rs, crates/mock-alpaca/src/state.rs, crates/mock-alpaca/src/routes.rs, crates/mock-alpaca/tests/auth.rs | cargo test -p pt-mock-alpaca --test auth |
| F03 | mock-reference | Serve the seeded account and clock | R-112 | sonnet | crates/mock-alpaca/src/server.rs, crates/mock-alpaca/src/state.rs, crates/mock-alpaca/src/routes.rs, crates/mock-alpaca/tests/account_clock.rs | cargo test -p pt-mock-alpaca --test account_clock |
| F04 | mock-reference | Serve the seeded calendar and assets | R-112 | sonnet | crates/mock-alpaca/src/server.rs, crates/mock-alpaca/src/state.rs, crates/mock-alpaca/src/routes.rs, crates/mock-alpaca/tests/calendar_assets.rs | cargo test -p pt-mock-alpaca --test calendar_assets |
| F05 | mock-reference | Serve seeded bars filtered by symbol, date, adjustment and feed, with paging | R-113 | sonnet | crates/mock-alpaca/src/server.rs, crates/mock-alpaca/src/state.rs, crates/mock-alpaca/src/routes.rs, crates/mock-alpaca/tests/bars.rs | cargo test -p pt-mock-alpaca --test bars |
| F06 | mock-reference | Add deny_feed and fail_next | R-113, R-116 | sonnet | crates/mock-alpaca/src/server.rs, crates/mock-alpaca/src/state.rs, crates/mock-alpaca/src/routes.rs, crates/mock-alpaca/tests/failures.rs | cargo test -p pt-mock-alpaca --test failures |
| F07 | mock-orders | Accept an order, refuse a duplicate client order id, look it up by that id | R-114 | sonnet | crates/mock-alpaca/src/server.rs, crates/mock-alpaca/src/state.rs, crates/mock-alpaca/src/routes.rs, crates/mock-alpaca/tests/orders_post.rs | cargo test -p pt-mock-alpaca --test orders_post |
| F08 | mock-orders | List orders and cancel one | R-114 | sonnet | crates/mock-alpaca/src/server.rs, crates/mock-alpaca/src/state.rs, crates/mock-alpaca/src/routes.rs, crates/mock-alpaca/tests/orders_list_cancel.rs | cargo test -p pt-mock-alpaca --test orders_list_cancel |
| F09 | mock-orders | Script fills that update order, position, cash and activities | R-115 | sonnet | crates/mock-alpaca/src/server.rs, crates/mock-alpaca/src/state.rs, crates/mock-alpaca/src/routes.rs, crates/mock-alpaca/tests/fills.rs | cargo test -p pt-mock-alpaca --test fills |
| F10 | mock-orders | Add expire and set_order_status | R-115, R-119 | sonnet | crates/mock-alpaca/src/server.rs, crates/mock-alpaca/src/state.rs, crates/mock-alpaca/src/routes.rs, crates/mock-alpaca/tests/order_status.rs | cargo test -p pt-mock-alpaca --test order_status |
| F11 | mock-orders | Refuse a sell above the long position and a short of a non-shortable asset | R-117 | sonnet | crates/mock-alpaca/src/server.rs, crates/mock-alpaca/src/state.rs, crates/mock-alpaca/src/routes.rs, crates/mock-alpaca/tests/refusals.rs | cargo test -p pt-mock-alpaca --test refusals |
| F12 | mock-orders | Seed a foreign order and a foreign position | R-118 | sonnet | crates/mock-alpaca/src/server.rs, crates/mock-alpaca/src/state.rs, crates/mock-alpaca/src/routes.rs, crates/mock-alpaca/tests/foreign.rs | cargo test -p pt-mock-alpaca --test foreign |
| D01 | store-codec | Add dec_to_text and text_to_dec | R-053 | sonnet | crates/store/src/codec.rs, crates/store/tests/codec.rs | cargo test -p pt-store --test codec |
| D02 | store-schema | Add Store::open, open_in_memory, schema_version, execute_raw and the migration runner with the market tables | R-050 | opus | crates/store/src/db.rs, crates/store/migrations/0001_market.sql, crates/store/tests/open.rs | cargo test -p pt-store --test open |
| D03 | store-schema | Add the trading tables migration | R-050 | opus | crates/store/src/db.rs, crates/store/migrations/0002_trading.sql, crates/store/tests/schema_trading.rs | cargo test -p pt-store --test schema_trading |
| D04 | store-schema | Add the shadow and operations tables migration | R-050 | opus | crates/store/src/db.rs, crates/store/migrations/0003_ops.sql, crates/store/tests/schema_ops.rs | cargo test -p pt-store --test schema_ops |
| D05 | store-schema | Refuse a database whose version is newer than the binary | R-051 | sonnet | crates/store/src/db.rs, crates/store/tests/schema_too_new.rs | cargo test -p pt-store --test schema_too_new |
| D06 | store-schema | Turn on write-ahead logging and foreign keys at open | R-052 | sonnet | crates/store/src/db.rs, crates/store/tests/pragmas.rs | cargo test -p pt-store --test pragmas |
| D07 | store-market | Add upsert_bars: insert, leave unchanged, or add a restated version | R-054 | opus | crates/store/src/market.rs, crates/store/tests/bars_versions.rs | cargo test -p pt-store --test bars_versions |
| D08 | store-market | Add bars_asof | R-055 | sonnet | crates/store/src/market.rs, crates/store/tests/bars_asof.rs | cargo test -p pt-store --test bars_asof |
| D09 | store-market | Prove raw, adjusted and per-feed series stay separate | R-056 | sonnet | crates/store/src/market.rs, crates/store/tests/bars_keys.rs | cargo test -p pt-store --test bars_keys |
| D10 | store-market | Add put_sessions and calendar | R-058 | sonnet | crates/store/src/market.rs, crates/store/tests/calendar.rs | cargo test -p pt-store --test calendar |
| D11 | store-market | Add asset snapshots readable as of an instant | R-059 | sonnet | crates/store/src/market.rs, crates/store/tests/assets.rs | cargo test -p pt-store --test assets |
| D12 | store-orders | Add insert_decision, decision_for and decisions | R-060 | sonnet | crates/store/src/trading.rs, crates/store/tests/decisions.rs | cargo test -p pt-store --test decisions |
| D13 | store-orders | Add insert_order, order and orders_in_states | R-061, R-052 | sonnet | crates/store/src/trading.rs, crates/store/tests/orders_insert.rs | cargo test -p pt-store --test orders_insert |
| D14 | store-orders | Add set_order_state with the transition check and the event log | R-062 | sonnet | crates/store/src/trading.rs, crates/store/tests/order_state.rs | cargo test -p pt-store --test order_state |
| D15 | store-orders | Add insert_fill, idempotent by activity id, and fills | R-063, R-073 | sonnet | crates/store/src/trading.rs, crates/store/tests/fills.rs | cargo test -p pt-store --test fills |
| D16 | store-ledger | Add append_ledger, ledger, positions and cash | R-064, R-073 | sonnet | crates/store/src/trading.rs, crates/store/tests/ledger.rs | cargo test -p pt-store --test ledger |
| D17 | store-ledger | Add positions_at and cash_at | R-065 | sonnet | crates/store/src/trading.rs, crates/store/tests/ledger_asof.rs | cargo test -p pt-store --test ledger_asof |
| D18 | store-ledger | Add equity snapshots that keep their first recorded_at | R-066 | sonnet | crates/store/src/trading.rs, crates/store/tests/equity_snapshots.rs | cargo test -p pt-store --test equity_snapshots |
| D19 | store-shadow | Add put_shadow_day, shadow_days, put_slippage and slippage_rows | R-067 | sonnet | crates/store/src/shadow.rs, crates/store/tests/shadow_rows.rs | cargo test -p pt-store --test shadow_rows |
| D20 | store-ops | Add start_run, finish_run and runs | R-068 | sonnet | crates/store/src/ops.rs, crates/store/tests/runs.rs | cargo test -p pt-store --test runs |
| D21 | store-ops | Add set_halt, clear_halt, active_halts and halts | R-069 | sonnet | crates/store/src/ops.rs, crates/store/tests/halts.rs | cargo test -p pt-store --test halts |
| D22 | store-ops | Add record_alert and alerts | R-070 | sonnet | crates/store/src/ops.rs, crates/store/tests/alerts.rs | cargo test -p pt-store --test alerts |
| D23 | store-ops | Add register_trial and trials | R-071 | sonnet | crates/store/src/ops.rs, crates/store/tests/trials.rs | cargo test -p pt-store --test trials |
| D24 | store-ops | Add save_backtest and backtest_equity | R-072 | sonnet | crates/store/src/ops.rs, crates/store/tests/backtests.rs | cargo test -p pt-store --test backtests |
| D25 | store-append-only | Add the append-only triggers migration | R-057 | opus | crates/store/src/db.rs, crates/store/migrations/0004_append_only.sql, crates/store/tests/append_only.rs | cargo test -p pt-store --test append_only |
| N01 | gate-owner | Add Stage, OwnerFile and the default for a missing file | R-260 | sonnet | crates/gate/src/owner.rs, crates/gate/tests/owner_missing.rs | cargo test -p pt-gate --test owner_missing |
| N02 | gate-owner | Parse the owner file strictly | R-261 | sonnet | crates/gate/src/owner.rs, crates/gate/tests/owner_parse.rs | cargo test -p pt-gate --test owner_parse |
| N03 | gate-owner | Refuse an owner file inside the repository | R-262 | sonnet | crates/gate/src/owner.rs, crates/gate/tests/owner_in_repo.rs | cargo test -p pt-gate --test owner_in_repo |
| N04 | gate-owner | Add the example owner file, which grants nothing | R-271 | sonnet | crates/gate/src/owner.rs, config/owner.example.toml, crates/gate/tests/owner_example.rs | cargo test -p pt-gate --test owner_example |
| N05 | gate-authorize | Add Capability and Gate::allows per the stage table | R-263 | sonnet | crates/gate/src/gate.rs, crates/gate/tests/capabilities.rs | cargo test -p pt-gate --test capabilities |
| N06 | gate-authorize | Add authorize for the paper stage and a permit bound to one order | R-264 | opus | crates/gate/src/gate.rs, crates/gate/tests/authorize_paper.rs | cargo test -p pt-gate --test authorize_paper |
| N07 | gate-authorize | Prove with a compile_fail doctest that a permit cannot be forged | R-265 | sonnet | crates/gate/src/gate.rs | cargo test -p pt-gate --doc |
| N08 | gate-authorize | Block orders while halted, except risk-reducing ones | R-266 | sonnet | crates/gate/src/gate.rs, crates/gate/tests/authorize_halted.rs | cargo test -p pt-gate --test authorize_halted |
| N09 | gate-authorize | Require the flat confirmation, and refuse everything below the first-order stage | R-267, R-269 | sonnet | crates/gate/src/gate.rs, crates/gate/tests/authorize_preconditions.rs | cargo test -p pt-gate --test authorize_preconditions |
| N10 | gate-authorize | Authorise only the approved first order, once | R-268 | sonnet | crates/gate/src/gate.rs, crates/gate/tests/authorize_first_order.rs | cargo test -p pt-gate --test authorize_first_order |
| N11 | gate-status | Add gate_status with the twelve checklist items | R-270 | sonnet | crates/gate/src/status.rs, crates/gate/tests/checklist.rs | cargo test -p pt-gate --test checklist |
| H01 | strategy-vol | Add ewma_vol | R-140 | sonnet | crates/strategy/src/vol.rs, crates/strategy/tests/ewma_vol.rs | cargo test -p pt-strategy --test ewma_vol |
| H02 | strategy-vol | Handle fewer than two returns and constant returns | R-141 | sonnet | crates/strategy/src/vol.rs, crates/strategy/tests/ewma_vol_edges.rs | cargo test -p pt-strategy --test ewma_vol_edges |
| H03 | strategy-vol | Add ewma_cov | R-142 | sonnet | crates/strategy/src/vol.rs, crates/strategy/tests/ewma_cov.rs | cargo test -p pt-strategy --test ewma_cov |
| H04 | strategy-risk | Add estimate_risk with minimum and maximum observations | R-143 | sonnet | crates/strategy/src/estimate.rs, crates/strategy/tests/estimate_window.rs | cargo test -p pt-strategy --test estimate_window |
| H05 | strategy-risk | Mark zero-volatility and gapped symbols ineligible | R-144 | sonnet | crates/strategy/src/estimate.rs, crates/strategy/tests/estimate_ineligible.rs | cargo test -p pt-strategy --test estimate_ineligible |
| H06 | strategy-risk | Report covariance for every ordered pair of eligible symbols | R-145 | sonnet | crates/strategy/src/estimate.rs, crates/strategy/tests/estimate_cov.rs | cargo test -p pt-strategy --test estimate_cov |
| H07 | strategy-risk | Prove estimates ignore data after the date | R-146 | sonnet | crates/strategy/src/estimate.rs, crates/strategy/tests/estimate_lookahead.rs | cargo test -p pt-strategy --test estimate_lookahead |
| H08 | strategy-tsmom | Add the Strategy trait and Tsmom with one lookback | R-147 | sonnet | crates/strategy/src/strategy.rs, crates/strategy/src/tsmom.rs, crates/strategy/tests/tsmom_sign.rs | cargo test -p pt-strategy --test tsmom_sign |
| H09 | strategy-tsmom | Blend several lookbacks with equal weight | R-148 | sonnet | crates/strategy/src/tsmom.rs, crates/strategy/tests/tsmom_blend.rs | cargo test -p pt-strategy --test tsmom_blend |
| H10 | strategy-tsmom | Mark a symbol without an anchor price ineligible | R-149 | sonnet | crates/strategy/src/tsmom.rs, crates/strategy/tests/tsmom_ineligible.rs | cargo test -p pt-strategy --test tsmom_ineligible |
| H11 | strategy-tsmom | Return errors for a missing risk-free price and a non-session date | R-150, R-151 | sonnet | crates/strategy/src/tsmom.rs, crates/strategy/tests/tsmom_errors.rs | cargo test -p pt-strategy --test tsmom_errors |
| H12 | strategy-tsmom | Add the strategy id and canonical parameters | R-152 | sonnet | crates/strategy/src/tsmom.rs, crates/strategy/tests/tsmom_identity.rs | cargo test -p pt-strategy --test tsmom_identity |
| H13 | strategy-tsmom | Prove signals ignore data after the date | R-146 | sonnet | crates/strategy/src/tsmom.rs, crates/strategy/tests/tsmom_lookahead.rs | cargo test -p pt-strategy --test tsmom_lookahead |
| H14 | strategy-benchmark | Add LongOnly | R-153 | sonnet | crates/strategy/src/benchmark.rs, crates/strategy/tests/benchmark.rs | cargo test -p pt-strategy --test benchmark |
| I01 | portfolio-weights | Add target_weights: inverse-volatility positions scaled to the target | R-160 | sonnet | crates/portfolio/src/weights.rs, crates/portfolio/tests/weights_target.rs | cargo test -p pt-portfolio --test weights_target |
| I02 | portfolio-weights | Clamp each weight | R-161 | sonnet | crates/portfolio/src/weights.rs, crates/portfolio/tests/weights_cap.rs | cargo test -p pt-portfolio --test weights_cap |
| I03 | portfolio-weights | Scale all weights down to the gross cap | R-162 | sonnet | crates/portfolio/src/weights.rs, crates/portfolio/tests/weights_gross.rs | cargo test -p pt-portfolio --test weights_gross |
| I04 | portfolio-weights | Flatten short signals in non-shortable symbols before scaling | R-163 | sonnet | crates/portfolio/src/weights.rs, crates/portfolio/tests/weights_shortable.rs | cargo test -p pt-portfolio --test weights_shortable |
| I05 | portfolio-weights | Return zero weights for no signal and for ineligible symbols | R-164 | sonnet | crates/portfolio/src/weights.rs, crates/portfolio/tests/weights_zero.rs | cargo test -p pt-portfolio --test weights_zero |
| I06 | portfolio-orders | Add target_shares, truncating toward zero | R-165 | sonnet | crates/portfolio/src/orders.rs, crates/portfolio/tests/target_shares.rs | cargo test -p pt-portfolio --test target_shares |
| I07 | portfolio-orders | Return an error for a non-zero weight without a price | R-166 | sonnet | crates/portfolio/src/orders.rs, crates/portfolio/tests/target_shares_price.rs | cargo test -p pt-portfolio --test target_shares_price |
| I08 | portfolio-orders | Add plan_orders for same-sign changes | R-167 | sonnet | crates/portfolio/src/orders.rs, crates/portfolio/tests/plan_diff.rs | cargo test -p pt-portfolio --test plan_diff |
| I09 | portfolio-orders | Close held symbols that are absent from the target | R-170 | sonnet | crates/portfolio/src/orders.rs, crates/portfolio/tests/plan_absent.rs | cargo test -p pt-portfolio --test plan_absent |
| I10 | portfolio-orders | Split a sign change into a closing leg and an opening leg | R-168 | sonnet | crates/portfolio/src/orders.rs, crates/portfolio/tests/plan_flip.rs | cargo test -p pt-portfolio --test plan_flip |
| I11 | portfolio-orders | Order the plan: sells before buys, leg 1 before leg 2 | R-169 | sonnet | crates/portfolio/src/orders.rs, crates/portfolio/tests/plan_order.rs | cargo test -p pt-portfolio --test plan_order |
| J01 | risk | Add RiskLimits and check_orders with the no-equity and order-notional checks | R-184, R-180 | sonnet | crates/risk/src/pretrade.rs, crates/risk/tests/order_notional.rs | cargo test -p pt-risk --test order_notional |
| J02 | risk | Add the position weight cap | R-181 | sonnet | crates/risk/src/pretrade.rs, crates/risk/tests/weight_cap.rs | cargo test -p pt-risk --test weight_cap |
| J03 | risk | Add the projected gross-exposure check | R-182 | sonnet | crates/risk/src/pretrade.rs, crates/risk/tests/gross.rs | cargo test -p pt-risk --test gross |
| J04 | risk | Exempt reducing orders from the weight and gross checks | R-183 | sonnet | crates/risk/src/pretrade.rs, crates/risk/tests/reducing.rs | cargo test -p pt-risk --test reducing |
| J05 | risk | Add check_equity with the daily-loss halt | R-185 | sonnet | crates/risk/src/halts.rs, crates/risk/tests/daily_loss.rs | cargo test -p pt-risk --test daily_loss |
| J06 | risk | Add the drawdown halt | R-186 | sonnet | crates/risk/src/halts.rs, crates/risk/tests/drawdown.rs | cargo test -p pt-risk --test drawdown |
| K01 | engine-cost | Add fill_cost for spread and fee | R-190 | sonnet | crates/engine/src/cost.rs, crates/engine/tests/fill_cost.rs | cargo test -p pt-engine --test fill_cost |
| K02 | engine-cost | Add the square-root impact term | R-191 | sonnet | crates/engine/src/cost.rs, crates/engine/tests/impact.rs | cargo test -p pt-engine --test impact |
| K03 | engine-cost | Add borrow_cost | R-192 | sonnet | crates/engine/src/cost.rs, crates/engine/tests/borrow.rs | cargo test -p pt-engine --test borrow |
| K04 | engine-cost | Add Book with net positions | R-193 | sonnet | crates/engine/src/book.rs, crates/engine/tests/book.rs | cargo test -p pt-engine --test book |
| K05 | engine-decide | Add DecisionParams and params_from_config | R-212 | sonnet | crates/engine/src/decision.rs, crates/engine/tests/params.rs | cargo test -p pt-engine --test params |
| K06 | engine-decide | Add decide_with | R-194 | opus | crates/engine/src/decision.rs, crates/engine/tests/decide_with.rs | cargo test -p pt-engine --test decide_with |
| K07 | engine-decide | Record the orders the risk checks rejected | R-195 | sonnet | crates/engine/src/decision.rs, crates/engine/tests/decide_rejections.rs | cargo test -p pt-engine --test decide_rejections |
| K08 | engine-decide | Add decide, which reads the view and refuses a missing reference price | R-196 | sonnet | crates/engine/src/decision.rs, crates/engine/tests/decide_view.rs | cargo test -p pt-engine --test decide_view |
| K09 | engine-decide | Add decision_to_json in canonical form | R-197 | sonnet | crates/engine/src/decision.rs, crates/engine/tests/decision_json.rs | cargo test -p pt-engine --test decision_json |
| K10 | engine-decide | Add DecisionRecord with its JSON round trip | R-197 | sonnet | crates/engine/src/decision.rs, crates/engine/tests/record_json.rs | cargo test -p pt-engine --test record_json |
| K11 | engine-sim | Add SimBroker, filling at the open of the trade date | R-198 | sonnet | crates/engine/src/broker.rs, crates/engine/tests/common/mod.rs, crates/engine/tests/sim_broker.rs | cargo test -p pt-engine --test sim_broker |
| K12 | engine-sim | Take impact inputs from the view | R-211 | sonnet | crates/engine/src/broker.rs, crates/engine/tests/sim_broker_impact.rs | cargo test -p pt-engine --test sim_broker_impact |
| K13 | engine-sim | Add simulate: targets from weights, fills, marks, one equity point per session | R-199 | opus | crates/engine/src/simulate.rs, crates/engine/tests/simulate_zero_cost.rs | cargo test -p pt-engine --test simulate_zero_cost |
| K14 | engine-sim | Charge fill costs and daily borrow | R-200 | sonnet | crates/engine/src/simulate.rs, crates/engine/tests/simulate_costed.rs | cargo test -p pt-engine --test simulate_costed |
| K15 | engine-sim | Charge borrow by calendar days to the next session | R-201 | sonnet | crates/engine/src/simulate.rs, crates/engine/tests/simulate_weekend.rs | cargo test -p pt-engine --test simulate_weekend |
| K16 | engine-sim-guards | Stop on a missing bar | R-202 | sonnet | crates/engine/src/simulate.rs, crates/engine/tests/simulate_missing_bar.rs | cargo test -p pt-engine --test simulate_missing_bar |
| K17 | engine-sim-guards | Validate the range and the weight dates | R-203, R-204 | sonnet | crates/engine/src/simulate.rs, crates/engine/tests/simulate_range.rs | cargo test -p pt-engine --test simulate_range |
| K18 | engine-sim-guards | Record rejected orders and add clean | R-205 | sonnet | crates/engine/src/simulate.rs, crates/engine/tests/simulate_rejections.rs | cargo test -p pt-engine --test simulate_rejections |
| K19 | engine-sim-guards | Record equity halts without enforcing them | R-206 | sonnet | crates/engine/src/simulate.rs, crates/engine/tests/simulate_would_halt.rs | cargo test -p pt-engine --test simulate_would_halt |
| K20 | engine-backtest | Add weights_schedule | R-207 | sonnet | crates/engine/src/schedule.rs, crates/engine/tests/weights_schedule.rs | cargo test -p pt-engine --test weights_schedule |
| K21 | engine-backtest | Add run_backtest | R-209 | sonnet | crates/engine/src/schedule.rs, crates/engine/tests/run_backtest.rs | cargo test -p pt-engine --test run_backtest |
| K22 | engine-backtest | Hold cash when no instrument is eligible | R-208 | sonnet | crates/engine/src/schedule.rs, crates/engine/tests/schedule_empty.rs | cargo test -p pt-engine --test schedule_empty |
| K23 | engine-backtest | Prove a ten-year backtest runs within the time limit | R-210 | opus | crates/engine/src/schedule.rs, crates/engine/src/simulate.rs, crates/engine/tests/backtest_speed.rs | cargo test -p pt-engine --test backtest_speed |
| G01 | data-view | Add StoreView adjusted closes as of an instant | R-136 | sonnet | crates/data/src/view.rs, crates/data/tests/view_asof.rs | cargo test -p pt-data --test view_asof |
| G02 | data-view | Serve the execution bar from the series named by the basis | R-137 | sonnet | crates/data/src/view.rs, crates/data/tests/view_basis.rs | cargo test -p pt-data --test view_basis |
| G03 | data-view | Read one feed only, and add average volume | R-138 | sonnet | crates/data/src/view.rs, crates/data/tests/view_feed.rs | cargo test -p pt-data --test view_feed |
| G04 | data-import | Add import_bars_csv | R-129 | sonnet | crates/data/src/csv_import.rs, crates/data/tests/import_bars.rs | cargo test -p pt-data --test import_bars |
| G05 | data-import | Import nothing from a malformed file and name the line | R-130 | sonnet | crates/data/src/csv_import.rs, crates/data/tests/import_bad.rs | cargo test -p pt-data --test import_bad |
| G06 | data-import | Add import_calendar_csv | R-131 | sonnet | crates/data/src/csv_import.rs, crates/data/tests/import_calendar.rs | cargo test -p pt-data --test import_calendar |
| G07 | data-quality | Add check_bar | R-124 | sonnet | crates/data/src/quality.rs, crates/data/tests/check_bar.rs | cargo test -p pt-data --test check_bar |
| G08 | data-quality | Add check_gaps | R-126 | sonnet | crates/data/src/quality.rs, crates/data/tests/check_gaps.rs | cargo test -p pt-data --test check_gaps |
| G09 | data-ingest | Add ingest for the raw and adjusted series of the universe and the risk-free symbol | R-120 | opus | crates/data/src/ingest.rs, crates/data/tests/ingest_common/mod.rs, crates/data/tests/ingest_full.rs | cargo test -p pt-data --test ingest_full |
| G10 | data-ingest | Report incomplete data | R-121 | sonnet | crates/data/src/ingest.rs, crates/data/tests/ingest_incomplete.rs | cargo test -p pt-data --test ingest_incomplete |
| G11 | data-ingest | Refetch raw bars from five sessions back and adjusted bars in full | R-122 | sonnet | crates/data/src/ingest.rs, crates/data/tests/ingest_incremental.rs | cargo test -p pt-data --test ingest_incremental |
| G12 | data-ingest | Prove a restated history is kept as a new version | R-123 | sonnet | crates/data/src/ingest.rs, crates/data/tests/ingest_restated.rs | cargo test -p pt-data --test ingest_restated |
| G13 | data-ingest | Store nothing from a response with a bad bar | R-125 | sonnet | crates/data/src/ingest.rs, crates/data/tests/ingest_bad_bar.rs | cargo test -p pt-data --test ingest_bad_bar |
| G14 | data-ingest | Stay inside the request budget and surface a feed refusal | R-127, R-128 | sonnet | crates/data/src/ingest.rs, crates/data/tests/ingest_budget.rs | cargo test -p pt-data --test ingest_budget |
| G15 | data-reference | Add sync_calendar | R-132 | sonnet | crates/data/src/reference.rs, crates/data/tests/sync_calendar.rs | cargo test -p pt-data --test sync_calendar |
| G16 | data-reference | Add sync_assets | R-133 | sonnet | crates/data/src/reference.rs, crates/data/tests/sync_assets.rs | cargo test -p pt-data --test sync_assets |
| G17 | data-universe | Add verify_universe with the asset-status reasons | R-134 | sonnet | crates/data/src/universe.rs, crates/data/tests/universe_assets.rs | cargo test -p pt-data --test universe_assets |
| G18 | data-universe | Add the history-start and gap reasons | R-134 | sonnet | crates/data/src/universe.rs, crates/data/tests/universe_history.rs | cargo test -p pt-data --test universe_history |
| G19 | data-doctor | Add doctor with the account, margin and clock checks | R-135 | sonnet | crates/data/src/doctor.rs, crates/data/tests/doctor_account.rs | cargo test -p pt-data --test doctor_account |
| G20 | data-doctor | Add the feed probe and the asset-field check | R-135 | sonnet | crates/data/src/doctor.rs, crates/data/tests/doctor_feed.rs | cargo test -p pt-data --test doctor_feed |
| Q01 | ops-logging | Add format_event | R-354 | sonnet | crates/ops/src/logging.rs, crates/ops/tests/format_event.rs | cargo test -p pt-ops --test format_event |
| Q02 | ops-pnl | Add pnl | R-355 | sonnet | crates/ops/src/pnl.rs, crates/ops/tests/pnl.rs | cargo test -p pt-ops --test pnl |
| Q03 | ops-pnl | Return nothing when there is no snapshot for the date | R-356 | sonnet | crates/ops/src/pnl.rs, crates/ops/tests/pnl_missing.rs | cargo test -p pt-ops --test pnl_missing |
| Q04 | ops-alerts | Add Alerter that stores and logs an alert | R-350 | sonnet | crates/ops/src/alert.rs, crates/ops/tests/alert_store.rs | cargo test -p pt-ops --test alert_store |
| Q05 | ops-alerts | Post the alert to the webhook as JSON | R-351 | sonnet | crates/ops/src/alert.rs, crates/ops/tests/alert_webhook.rs | cargo test -p pt-ops --test alert_webhook |
| Q06 | ops-alerts | Tolerate a failing webhook | R-352 | sonnet | crates/ops/src/alert.rs, crates/ops/tests/alert_failure.rs | cargo test -p pt-ops --test alert_failure |
| Q07 | ops-alerts | Scrub secret strings | R-353 | sonnet | crates/ops/src/alert.rs, crates/ops/tests/alert_scrub.rs | cargo test -p pt-ops --test alert_scrub |
| Q08 | ops-report | Add write_status with the book keys of status.json: stage, feed, basis, equity, cash, positions, gross exposure | R-357 | sonnet | crates/ops/src/report.rs, crates/ops/tests/status_json_book.rs | cargo test -p pt-ops --test status_json_book |
| Q09 | ops-report | Add the run, P&L, rebalance, halt, trial, fidelity and sign-off keys | R-357 | sonnet | crates/ops/src/report.rs, crates/ops/tests/status_json_ops.rs | cargo test -p pt-ops --test status_json_ops |
| Q10 | ops-report | Write status.md with the disclaimer | R-358 | sonnet | crates/ops/src/report.rs, crates/ops/tests/status_md.rs | cargo test -p pt-ops --test status_md |
| Q11 | ops-report | Replace both files atomically | R-359 | sonnet | crates/ops/src/report.rs, crates/ops/tests/status_atomic.rs | cargo test -p pt-ops --test status_atomic |
| Q12 | ops-report | Report an empty store and a halted system | R-360 | sonnet | crates/ops/src/report.rs, crates/ops/tests/status_empty.rs | cargo test -p pt-ops --test status_empty |
| Q13 | ops-report | Add last_ok_run_age_hours | R-361 | sonnet | crates/ops/src/report.rs, crates/ops/tests/heartbeat.rs | cargo test -p pt-ops --test heartbeat |
| V01 | validate-backtest | Add BacktestRequest and trial_hash | R-367 | sonnet | crates/validate/src/runner.rs, crates/validate/tests/trial_hash.rs | cargo test -p pt-validate --test trial_hash |
| V02 | validate-backtest | Add run_and_record: build the view and strategy, run on adjusted prices with research costs, save the equity | R-365 | opus | crates/validate/src/runner.rs, crates/validate/tests/common/mod.rs, crates/validate/tests/run_and_record.rs | cargo test -p pt-validate --test run_and_record |
| V03 | validate-backtest | Compute Metrics from the result | R-368 | sonnet | crates/validate/src/runner.rs, crates/validate/tests/metrics.rs | cargo test -p pt-validate --test metrics |
| V04 | validate-backtest | Register the trial; count a repeat as a run, not a trial | R-365, R-366 | sonnet | crates/validate/src/runner.rs, crates/validate/tests/trials.rs | cargo test -p pt-validate --test trials |
| V05 | validate-backtest | Store and register nothing when the run fails | R-369 | sonnet | crates/validate/src/runner.rs, crates/validate/tests/run_failure.rs | cargo test -p pt-validate --test run_failure |
| V06 | validate-crosscheck | Add crosscheck and prove exact agreement at zero cost | R-370 | opus | crates/validate/src/crosscheck.rs, crates/validate/tests/crosscheck_zero.rs | cargo test -p pt-validate --test crosscheck_zero |
| V07 | validate-crosscheck | Cross-check with proportional and borrow costs | R-371 | opus | crates/validate/src/crosscheck.rs, crates/validate/tests/crosscheck_costs.rs | cargo test -p pt-validate --test crosscheck_costs |
| V08 | validate-crosscheck | Declare a run with rejected orders not comparable | R-372 | sonnet | crates/validate/src/crosscheck.rs, crates/validate/tests/crosscheck_rejections.rs | cargo test -p pt-validate --test crosscheck_rejections |
| V09 | validate-overfit | Add dsr_for_run | R-373 | sonnet | crates/validate/src/overfit.rs, crates/validate/tests/dsr.rs | cargo test -p pt-validate --test dsr |
| V10 | validate-overfit | Add pbo_for_runs over common dates | R-374 | sonnet | crates/validate/src/overfit.rs, crates/validate/tests/pbo.rs | cargo test -p pt-validate --test pbo |
| V11 | validate-overfit | Add compare_to_benchmark | R-375 | sonnet | crates/validate/src/overfit.rs, crates/validate/tests/benchmark.rs | cargo test -p pt-validate --test benchmark |
| O01 | live-ids | Add client_order_id and first_order_id | R-280 | sonnet | crates/live/src/ids.rs, crates/live/tests/ids.rs | cargo test -p pt-live --test ids |
| O02 | live-ids | Add map_status | R-281 | sonnet | crates/live/src/status.rs, crates/live/tests/map_status.rs | cargo test -p pt-live --test map_status |
| O03 | live-lock | Add acquire_run_lock | R-310 | sonnet | crates/live/src/lock.rs, crates/live/tests/lock.rs | cargo test -p pt-live --test lock |
| O04 | live-context | Add trade_session and expected_last_session | R-282 | sonnet | crates/live/src/snapshot.rs, crates/live/tests/session_clock.rs | cargo test -p pt-live --test session_clock |
| O05 | live-context | Add LiveContext and the live test fixture (seeded mock, store, config, gate, strategy) | R-283 | sonnet | crates/live/src/context.rs, crates/live/tests/common/mod.rs, crates/live/tests/fixture.rs | cargo test -p pt-live --test fixture |
| O06 | live-submit | Add submit_planned: look up, authorise, POST, mark submitted, in plan order | R-283 | opus | crates/live/src/submit.rs, crates/live/tests/submit_basic.rs | cargo test -p pt-live --test submit_basic |
| O07 | live-submit | Adopt an order the broker already has | R-284 | sonnet | crates/live/src/submit.rs, crates/live/tests/submit_adopt.rs | cargo test -p pt-live --test submit_adopt |
| O08 | live-submit | Abandon an intent whose session has passed | R-285 | sonnet | crates/live/src/submit.rs, crates/live/tests/submit_stale.rs | cargo test -p pt-live --test submit_stale |
| O09 | live-submit | Mark a broker rejection and carry on with the other orders | R-286 | sonnet | crates/live/src/submit.rs, crates/live/tests/submit_rejected.rs | cargo test -p pt-live --test submit_rejected |
| O10 | live-submit | Stop at a gate refusal without touching the order | R-287 | sonnet | crates/live/src/submit.rs, crates/live/tests/submit_gate.rs | cargo test -p pt-live --test submit_gate |
| O11 | live-settle | Add settle: update the state of every open order | R-288 | sonnet | crates/live/src/settle.rs, crates/live/tests/settle_states.rs | cargo test -p pt-live --test settle_states |
| O12 | live-settle | Record each fill once with its ledger entry | R-289 | opus | crates/live/src/settle.rs, crates/live/tests/settle_fills.rs | cargo test -p pt-live --test settle_fills |
| O13 | live-settle | Accumulate partial fills | R-290 | sonnet | crates/live/src/settle.rs, crates/live/tests/settle_partial.rs | cargo test -p pt-live --test settle_partial |
| O14 | live-settle | Alert on an order that ends short, and never resend it | R-291 | sonnet | crates/live/src/settle.rs, crates/live/tests/settle_incomplete.rs | cargo test -p pt-live --test settle_incomplete |
| O15 | live-legs | Hold a leg-2 intent until settle sees leg 1 filled | R-292 | sonnet | crates/live/src/submit.rs, crates/live/src/settle.rs, crates/live/tests/legs_release.rs | cargo test -p pt-live --test legs_release |
| O16 | live-legs | Abandon leg 2 when leg 1 ends unfilled | R-293 | sonnet | crates/live/src/settle.rs, crates/live/tests/legs_abandon.rs | cargo test -p pt-live --test legs_abandon |
| O17 | live-legs | Halt on an order status the design never causes | R-294 | sonnet | crates/live/src/settle.rs, crates/live/tests/settle_unexpected.rs | cargo test -p pt-live --test settle_unexpected |
| O18 | live-reconcile | Add reconcile, recording the opening balance of a flat account | R-295 | sonnet | crates/live/src/reconcile.rs, crates/live/tests/reconcile_open.rs | cargo test -p pt-live --test reconcile_open |
| O19 | live-reconcile | Halt when the account is not flat at the start | R-296 | sonnet | crates/live/src/reconcile.rs, crates/live/tests/reconcile_not_flat.rs | cargo test -p pt-live --test reconcile_not_flat |
| O20 | live-reconcile | Halt on an order the system did not create | R-297 | sonnet | crates/live/src/reconcile.rs, crates/live/tests/reconcile_foreign_order.rs | cargo test -p pt-live --test reconcile_foreign_order |
| O21 | live-reconcile | Halt on a position break | R-298 | sonnet | crates/live/src/reconcile.rs, crates/live/tests/reconcile_position.rs | cargo test -p pt-live --test reconcile_position |
| O22 | live-reconcile | Halt on a cash break beyond the tolerance | R-299 | sonnet | crates/live/src/reconcile.rs, crates/live/tests/reconcile_cash.rs | cargo test -p pt-live --test reconcile_cash |
| O23 | live-snapshots | Add snapshot_equity for the sessions without a snapshot | R-300 | sonnet | crates/live/src/snapshot.rs, crates/live/tests/snapshot_one.rs | cargo test -p pt-live --test snapshot_one |
| O24 | live-snapshots | Catch up over missed sessions and alert | R-301 | sonnet | crates/live/src/snapshot.rs, crates/live/tests/snapshot_catchup.rs | cargo test -p pt-live --test snapshot_catchup |
| O25 | live-snapshots | Add check_halts | R-302 | sonnet | crates/live/src/snapshot.rs, crates/live/tests/check_halts.rs | cargo test -p pt-live --test check_halts |
| O26 | live-rebalance | Add rebalance: find the due signal date and record the decision with its inputs | R-303 | opus | crates/live/src/rebalance.rs, crates/live/tests/rebalance_record.rs | cargo test -p pt-live --test rebalance_record |
| O27 | live-rebalance | Write the planned orders as intents, and report not-due afterwards | R-303 | sonnet | crates/live/src/rebalance.rs, crates/live/tests/rebalance_intents.rs | cargo test -p pt-live --test rebalance_intents |
| O28 | live-rebalance | Decide nothing from incomplete data | R-304 | sonnet | crates/live/src/rebalance.rs, crates/live/tests/rebalance_data.rs | cargo test -p pt-live --test rebalance_data |
| O29 | live-rebalance | Record a dry-run decision and dry-run orders below the first-order stage | R-309 | sonnet | crates/live/src/rebalance.rs, crates/live/tests/rebalance_dry_run.rs | cargo test -p pt-live --test rebalance_dry_run |
| O30 | live-rebalance-timing | Skip and record a rebalance later than the limit | R-305 | sonnet | crates/live/src/rebalance.rs, crates/live/tests/rebalance_late.rs | cargo test -p pt-live --test rebalance_late |
| O31 | live-rebalance-timing | Exempt the first paper rebalance | R-306 | sonnet | crates/live/src/rebalance.rs, crates/live/tests/rebalance_bootstrap.rs | cargo test -p pt-live --test rebalance_bootstrap |
| O32 | live-rebalance-timing | Defer when too close to the close | R-307 | sonnet | crates/live/src/rebalance.rs, crates/live/tests/rebalance_deferred.rs | cargo test -p pt-live --test rebalance_deferred |
| O33 | live-rebalance-timing | Take the shortable set from asset snapshots and leave out untradable assets | R-308 | sonnet | crates/live/src/rebalance.rs, crates/live/tests/rebalance_assets.rs | cargo test -p pt-live --test rebalance_assets |
| O34 | live-cycle | Add run_cycle, refusing the backtest stage | R-312 | sonnet | crates/live/src/cycle.rs, crates/live/tests/cycle_backtest.rs | cargo test -p pt-live --test cycle_backtest |
| O35 | live-cycle | Run the read-only phases in the ingest stage and log the run | R-311, R-317 | sonnet | crates/live/src/cycle.rs, crates/live/tests/cycle_ingest.rs | cargo test -p pt-live --test cycle_ingest |
| O36 | live-cycle | Add the dry-run rebalance phase | R-323 | sonnet | crates/live/src/cycle.rs, crates/live/tests/cycle_dry_run.rs | cargo test -p pt-live --test cycle_dry_run |
| O37 | live-cycle | Add settle, reconcile, snapshot, halts, rebalance and submit for the paper stage | R-313, R-317 | opus | crates/live/src/cycle.rs, crates/live/tests/cycle_paper.rs | cargo test -p pt-live --test cycle_paper |
| O38 | live-cycle | Prove a repeated run changes nothing | R-314 | sonnet | crates/live/src/cycle.rs, crates/live/tests/cycle_repeat.rs | cargo test -p pt-live --test cycle_repeat |
| O39 | live-cycle-failures | Settle and reconcile but send nothing while halted | R-315 | sonnet | crates/live/src/cycle.rs, crates/live/tests/cycle_halted.rs | cargo test -p pt-live --test cycle_halted |
| O40 | live-cycle-failures | Finish the run as failed, with an alert, on a broker or data error | R-316 | sonnet | crates/live/src/cycle.rs, crates/live/tests/cycle_failed.rs | cargo test -p pt-live --test cycle_failed |
| O41 | live-cycle-failures | Finish as data-not-ready after settling and reconciling | R-318 | sonnet | crates/live/src/cycle.rs, crates/live/tests/cycle_data.rs | cargo test -p pt-live --test cycle_data |
| O42 | live-first-order | Add first_order, planning the approved order on its day | R-319, R-320 | sonnet | crates/live/src/first_order.rs, crates/live/tests/first_order_plan.rs | cargo test -p pt-live --test first_order_plan |
| O43 | live-first-order | Send it from the cycle in the first-order stage, with no rebalance | R-319 | opus | crates/live/src/cycle.rs, crates/live/tests/cycle_first_order.rs | cargo test -p pt-live --test cycle_first_order |
| O44 | live-first-order | Prove it is never repeated | R-320 | sonnet | crates/live/src/cycle.rs, crates/live/src/first_order.rs, crates/live/tests/cycle_first_order_repeat.rs | cargo test -p pt-live --test cycle_first_order_repeat |
| O45 | live-killswitch | Add cancel_open | R-321 | sonnet | crates/live/src/killswitch.rs, crates/live/tests/cancel_open.rs | cargo test -p pt-live --test cancel_open |
| O46 | live-killswitch | Add flatten | R-322 | opus | crates/live/src/killswitch.rs, crates/live/tests/flatten.rs | cargo test -p pt-live --test flatten |
| P01 | shadow-slippage | Add slippage_bps | R-336 | sonnet | crates/shadow/src/slippage.rs, crates/shadow/tests/slippage_bps.rs | cargo test -p pt-shadow --test slippage_bps |
| P02 | shadow-slippage | Add record_slippage and the shadow test fixture | R-337 | sonnet | crates/shadow/src/slippage.rs, crates/shadow/tests/common/mod.rs, crates/shadow/tests/record_slippage.rs | cargo test -p pt-shadow --test record_slippage |
| P03 | shadow-daily | Add shadow_day for a session with orders | R-333 | opus | crates/shadow/src/daily.rs, crates/shadow/tests/shadow_trade_day.rs | cargo test -p pt-shadow --test shadow_trade_day |
| P04 | shadow-daily | Prove zero difference on a session without orders | R-334 | sonnet | crates/shadow/src/daily.rs, crates/shadow/tests/shadow_quiet_day.rs | cargo test -p pt-shadow --test shadow_quiet_day |
| P05 | shadow-daily | Add update for every session not yet compared | R-335 | sonnet | crates/shadow/src/daily.rs, crates/shadow/tests/shadow_update.rs | cargo test -p pt-shadow --test shadow_update |
| P06 | shadow-replay | Add replay_decision and the replay test fixture | R-330 | opus | crates/shadow/src/replay.rs, crates/shadow/tests/replay_fixture/mod.rs, crates/shadow/tests/replay_match.rs | cargo test -p pt-shadow --test replay_match |
| P07 | shadow-replay | Report what differs when a decision does not reproduce | R-331 | sonnet | crates/shadow/src/replay.rs, crates/shadow/tests/replay_mismatch.rs | cargo test -p pt-shadow --test replay_mismatch |
| P08 | shadow-replay | Prove replay ignores later restatements | R-332 | sonnet | crates/shadow/src/replay.rs, crates/shadow/tests/replay_asof.rs | cargo test -p pt-shadow --test replay_asof |
| P09 | shadow-fidelity | Add fidelity with the counts, mean slippage and return differences | R-338 | sonnet | crates/shadow/src/fidelity.rs, crates/shadow/tests/fidelity_counts.rs | cargo test -p pt-shadow --test fidelity_counts |
| P10 | shadow-fidelity | Count missed sessions | R-339 | sonnet | crates/shadow/src/fidelity.rs, crates/shadow/tests/fidelity_missed.rs | cargo test -p pt-shadow --test fidelity_missed |
| P11 | shadow-fidelity | Count late rebalances | R-340 | sonnet | crates/shadow/src/fidelity.rs, crates/shadow/tests/fidelity_late.rs | cargo test -p pt-shadow --test fidelity_late |
| P12 | shadow-signoff | Add evaluate with the seven fidelity criteria | R-341 | sonnet | crates/shadow/src/signoff.rs, crates/shadow/tests/criteria.rs | cargo test -p pt-shadow --test criteria |
| P13 | shadow-signoff | Add the verdict: not ready, pass or fail | R-342 | sonnet | crates/shadow/src/signoff.rs, crates/shadow/tests/verdict.rs | cargo test -p pt-shadow --test verdict |
| P14 | shadow-signoff | Add write_signoff | R-343 | sonnet | crates/shadow/src/signoff.rs, crates/shadow/tests/signoff_file.rs | cargo test -p pt-shadow --test signoff_file |
| R01 | cli-base | Add the full command tree, help, and a dispatcher with a stub per command group | R-380 | sonnet | crates/cli/src/main.rs, crates/cli/src/cli.rs, crates/cli/src/cmd_config.rs, crates/cli/src/cmd_data.rs, crates/cli/src/cmd_backtest.rs, crates/cli/src/cmd_validate.rs, crates/cli/src/cmd_run.rs, crates/cli/src/cmd_ops.rs, crates/cli/tests/common/mod.rs, crates/cli/tests/help.rs | cargo test -p pt-cli --test help |
| R02 | cli-base | Resolve the config path and add config check | R-381 | sonnet | crates/cli/src/wiring.rs, crates/cli/src/cmd_config.rs, crates/cli/tests/config_check.rs | cargo test -p pt-cli --test config_check |
| R03 | cli-base | Load the owner file and add gates | R-382 | sonnet | crates/cli/src/wiring.rs, crates/cli/src/cmd_config.rs, crates/cli/tests/gates.rs | cargo test -p pt-cli --test gates |
| R04 | cli-base | Check the minimum stage in the dispatcher and exit 4 | R-384 | opus | crates/cli/src/main.rs, crates/cli/src/wiring.rs, crates/cli/tests/stage_refusal.rs | cargo test -p pt-cli --test stage_refusal |
| R05 | cli-base | Load the keys and build the client for network commands | R-386 | sonnet | crates/cli/src/main.rs, crates/cli/src/wiring.rs, crates/cli/tests/missing_keys.rs | cargo test -p pt-cli --test missing_keys |
| R06 | cli-base | Log JSON lines to stderr and keep stdout for results | R-401 | sonnet | crates/cli/src/main.rs, crates/cli/src/wiring.rs, crates/cli/tests/logging.rs | cargo test -p pt-cli --test logging |
| R07 | cli-deploy | Add the cron and launchd example schedules | R-400 | sonnet | deploy/cron.example, deploy/launchd/com.paper-trader.run.plist.example, crates/cli/tests/deploy_files.rs | cargo test -p pt-cli --test deploy_files |
| R08 | cli-import | Add calendar import and data import-csv | R-383 | sonnet | crates/cli/src/cmd_data.rs, crates/cli/tests/import.rs | cargo test -p pt-cli --test import |
| R09 | cli-import | Add data check | R-383 | sonnet | crates/cli/src/cmd_data.rs, crates/cli/tests/data_check.rs | cargo test -p pt-cli --test data_check |
| R10 | cli-data | Add calendar sync and data ingest | R-385 | sonnet | crates/cli/src/cmd_data.rs, crates/cli/tests/ingest.rs | cargo test -p pt-cli --test ingest |
| R11 | cli-data | Add universe verify | R-387 | sonnet | crates/cli/src/cmd_data.rs, crates/cli/tests/universe_verify.rs | cargo test -p pt-cli --test universe_verify |
| R12 | cli-data | Add doctor | R-387 | sonnet | crates/cli/src/cmd_data.rs, crates/cli/tests/doctor.rs | cargo test -p pt-cli --test doctor |
| R13 | cli-backtest | Add backtest run, printing metrics, the Deflated Sharpe Ratio and the trial count | R-388 | sonnet | crates/cli/src/cmd_backtest.rs, crates/cli/tests/backtest_run.rs | cargo test -p pt-cli --test backtest_run |
| R14 | cli-backtest | Exit 1 and name the missing bar when a backtest fails | R-389 | sonnet | crates/cli/src/cmd_backtest.rs, crates/cli/tests/backtest_failure.rs | cargo test -p pt-cli --test backtest_failure |
| R15 | cli-backtest | Add backtest crosscheck | R-390 | sonnet | crates/cli/src/cmd_backtest.rs, crates/cli/tests/crosscheck.rs | cargo test -p pt-cli --test crosscheck |
| R16 | cli-validate | Add validate dsr | R-391 | sonnet | crates/cli/src/cmd_validate.rs, crates/cli/tests/validate_dsr.rs | cargo test -p pt-cli --test validate_dsr |
| R17 | cli-validate | Add validate pbo | R-391 | sonnet | crates/cli/src/cmd_validate.rs, crates/cli/tests/validate_pbo.rs | cargo test -p pt-cli --test validate_pbo |
| R18 | cli-validate | Add validate benchmark | R-392 | sonnet | crates/cli/src/cmd_validate.rs, crates/cli/tests/validate_benchmark.rs | cargo test -p pt-cli --test validate_benchmark |
| R19 | cli-live | Add run: build the context, run the cycle, exit 0 or 1 | R-394 | opus | crates/cli/src/cmd_run.rs, crates/cli/tests/run_ok.rs | cargo test -p pt-cli --test run_ok |
| R20 | cli-live | Take the run lock (exit 3) and report a halted run (exit 2) | R-394 | sonnet | crates/cli/src/cmd_run.rs, crates/cli/tests/run_exit_codes.rs | cargo test -p pt-cli --test run_exit_codes |
| R21 | cli-live | After the cycle, update the shadow comparison and slippage and write the report | R-394, R-401 | sonnet | crates/cli/src/cmd_run.rs, crates/cli/tests/run_report.rs | cargo test -p pt-cli --test run_report |
| R22 | cli-ops | Add halt status, halt set and halt clear | R-395 | sonnet | crates/cli/src/cmd_ops.rs, crates/cli/tests/halt.rs | cargo test -p pt-cli --test halt |
| R23 | cli-ops | Add report and status --max-age-hours | R-397 | sonnet | crates/cli/src/cmd_ops.rs, crates/cli/tests/report_status.rs | cargo test -p pt-cli --test report_status |
| R24 | cli-ops | Add signoff report | R-398 | sonnet | crates/cli/src/cmd_ops.rs, crates/cli/tests/signoff.rs | cargo test -p pt-cli --test signoff |
| R25 | cli-ops | Add cancel-open and flatten behind --yes | R-396 | sonnet | crates/cli/src/cmd_ops.rs, crates/cli/tests/killswitch.rs | cargo test -p pt-cli --test killswitch |
| R26 | cli-e2e | Prove the offline path end to end: import, backtest, cross-check, DSR, report | R-393 | sonnet | crates/cli/src/, crates/cli/tests/e2e_offline.rs | cargo test -p pt-cli --test e2e_offline |
| R27 | cli-e2e | Prove a mock paper session end to end, with a read-only owner file | R-399, R-402 | opus | crates/cli/src/, crates/cli/tests/e2e_paper.rs | cargo test -p pt-cli --test e2e_paper |
## Waves

Output of `chains.sh --check docs/design/paper-trader/plan.md`. The number after each chain is its step count.

Wave | Chains that can run together
--- | ---
1 | workspace (6)
2 | core-types (6), core-state (2), calendar (4), secrets (4), stats-descriptive (5), stats-normal (2), stats-pbo (6), alpaca-endpoints (4), mock-reference (6), store-codec (1), store-schema (5), gate-owner (4), strategy-vol (3), ops-logging (1), ops-pnl (2), live-lock (1), cli-deploy (1)
3 | core-view (3), calendar-months (3), config (10), stats-dsr (4), vector (4), alpaca-http (5), mock-orders (6), store-market (5), store-orders (4), store-shadow (1), store-ops (5), gate-authorize (6), gate-status (1), portfolio-orders (6), risk (6), engine-cost (4), live-ids (2)
4 | config-hash (1), alpaca-data (6), alpaca-account (4), alpaca-orders-read (3), store-ledger (3), strategy-risk (4), strategy-tsmom (6), portfolio-weights (5), engine-sim (5), data-view (3), data-import (3), ops-alerts (4)
5 | alpaca-write (3), store-append-only (1), strategy-benchmark (1), engine-decide (6), engine-sim-guards (4), data-quality (2), data-reference (2), data-doctor (2), ops-report (6), live-context (2), shadow-slippage (2), cli-base (6)
6 | engine-backtest (4), data-ingest (6), data-universe (2), live-submit (5), live-reconcile (5), live-snapshots (3), shadow-daily (3), shadow-replay (3), cli-import (2)
7 | validate-backtest (5), live-settle (4), live-rebalance (4), shadow-fidelity (3), cli-data (3)
8 | validate-crosscheck (3), validate-overfit (3), live-legs (3), live-rebalance-timing (4), live-killswitch (2), shadow-signoff (3)
9 | live-cycle (5), cli-backtest (3)
10 | live-cycle-failures (3), cli-validate (3)
11 | live-first-order (3)
12 | cli-live (3)
13 | cli-ops (4)
14 | cli-e2e (2)

87 chains, 318 steps. Widest wave: 17 chains. Longest chain: 10 steps.

What the waves mean in practice:

- Wave 1 is one chain and everything waits for it. It is the only chain that touches `Cargo.toml`, `Cargo.lock`, `lib.rs` and `error.rs` files.
- `CHECK` in `.devteam/config` already runs the architecture 7.4 Rust checks once a root `Cargo.toml` exists, so no swap is needed after the `workspace` chain lands. Implementers may not edit `.devteam/`.
- The offline research path (import, backtest, cross-check, DSR, PBO) is complete at wave 10 (`cli-validate`). The paper-trading path is complete at wave 13 (`cli-ops`). Wave 14 proves both end to end.
- No chain waits for an owner step. Keys, stages and approvals are checked by the binary at run time (architecture section 8).

Before the first chain: the build machine needs `~/.cargo/bin` on the non-interactive `PATH` and network access so rustup can install the pinned toolchain (architecture 3.4).

## Reading order

One line per chain: what to look for when reviewing it.

- `workspace`: Check each manifest against the dependency table in architecture section 4 and each `error.rs` against 5.11; nothing but `lib.rs` and `error.rs` should hold code.
- `core-types`: Look for any `f64` in money arithmetic and any constructor that skips validation.
- `core-state`: Compare the transition function with the table under architecture 5.1, cell by cell.
- `core-view`: The in-memory view is the fixture for every later look-ahead test: check its date filter is on or before, never after.
- `calendar`: Check strictness (next and previous exclude the date itself) and that a bad session list cannot construct a calendar.
- `calendar-months`: A month end exists only when a later session in another month is known; check the last month of the calendar is not treated as ended.
- `secrets`: Search the diff for any path by which a secret value could reach `Debug`, `Display` or an error message.
- `config`: Check the defaults against the TOML block in architecture 5.3 and that no key can name an endpoint. C12 to C05 build `parse` one section at a time, then C06 to C10 add the rules; no step should parse a number by hand (see Replans).
- `config-hash`: The hash must come from the parsed values in a fixed order, not from the file text.
- `stats-descriptive`: Check divisors: sample standard deviation uses n - 1, skewness and kurtosis use population moments.
- `stats-normal`: Check accuracy in the tails, where the Deflated Sharpe Ratio uses the inverse.
- `stats-dsr`: Compare the formula with architecture 5.10.6 term by term; the Sharpe ratio is per period, not annualised.
- `stats-pbo`: Check the tie rules (lowest index in-sample, mean rank out-of-sample) and that `lambda <= 0` counts as overfit.
- `vector`: This crate must share no accounting code with the engine. Read it against architecture 5.10.5 only.
- `alpaca-endpoints`: There must be no way to obtain an `Endpoints` value for any non-loopback host other than the two constants.
- `alpaca-http`: Check that waits go through `Sleeper` (no real sleeping) and that no 4xx other than 429 is retried.
- `alpaca-data`: Check that bars are asked of the market data base URL and never the trading one (E25, E10), the query against architecture 5.9, and that each price reaches `Decimal` through its shortest round-trip text, inside `data.rs` (see Replans).
- `alpaca-account`: Numeric fields arrive as strings; check none is parsed through a float.
- `alpaca-orders-read`: Check paging: fills page by the last activity id, orders by `after`.
- `alpaca-write`: This is the only code that POSTs an order. Check the permit is consumed and compared before any request is made.
- `mock-reference`: The mock is the fixture for every broker test: check its JSON shapes against architecture 5.9, strings for trading numbers, numbers for bar prices.
- `mock-orders`: Check that a scripted fill moves order state, position, cash and the activity list together.
- `store-codec`: Text in, text out: no float may appear between a `Decimal` and the column.
- `store-schema`: Read the three SQL files against the table list in architecture 5.4: keys, TEXT for money, foreign keys.
- `store-market`: The as-of read is what makes decision replay possible: check it returns the latest version at or before the instant, per date.
- `store-orders`: Uniqueness is the idempotency mechanism: check each duplicate is reported as `Duplicate`, not as a generic SQL error.
- `store-ledger`: Positions and cash must be sums over ledger rows, never stored totals.
- `store-shadow`: Small; check replace-by-key semantics.
- `store-ops`: Check that re-registering a trial increments `runs` instead of adding a row: the trial count feeds the Deflated Sharpe Ratio.
- `store-append-only`: Triggers, not application code: check both UPDATE and DELETE on all three tables.
- `gate-owner`: A missing file must mean `backtest`, and nothing here may write to the path.
- `gate-authorize`: Try to construct an `OrderPermit` any other way. Then read `authorize` as a decision table: stage, halt, flat confirmation, allowlist, first-order approval.
- `gate-status`: Compare the checklist with the owner steps in architecture section 8.
- `strategy-vol`: Check the weights against architecture 5.10.1: newest return has the largest weight, and the sum is normalised.
- `strategy-risk`: An ineligible symbol must be reported with a reason and never silently given a number.
- `strategy-tsmom`: Excess over the risk-free proxy, not raw return; anchors are month-end sessions from the calendar, not day counts.
- `strategy-benchmark`: It must go through the same `Strategy` trait, so it is sized by the same code.
- `portfolio-weights`: Follow the seven numbered steps of architecture 5.10.3 in order; caps come after scaling, rounding comes last.
- `portfolio-orders`: Truncation is toward zero for both signs. Order of the plan: leg-1 sells, leg-1 buys, leg-2 sells, leg-2 buys.
- `risk`: A reducing order must never be blocked by the weight or gross check; boundaries are inclusive as the requirement rows state.
- `engine-cost`: Each cost is rounded to six places before it touches cash; that is what lets two implementations agree.
- `engine-decide`: This is the function paper trading and replay share. Check it reads nothing but its arguments and that its JSON form is canonical.
- `engine-sim`: Work the three-session example by hand from architecture 5.10.5 and compare with the test values.
- `engine-sim-guards`: These are the defects from the implementation-risk paper: carried prices, truncated ranges, silent rejections.
- `engine-backtest`: Weights must not depend on equity or positions; that is what lets the vector calculator check the engine.
- `data-view`: Signals always read the adjusted series; only the execution bar follows the basis.
- `data-import`: A bad row must leave the store untouched: check the import is one transaction.
- `data-quality`: Nothing here may fill, interpolate or carry a price forward.
- `data-ingest`: The adjusted series is refetched in full so restatements are seen; check the request starts recorded by the mock.
- `data-reference`: An unknown asset becomes an all-false snapshot, not an error.
- `data-universe`: Every failure reason must be listed, not only the first.
- `data-doctor`: It must keep going after a failed check and must suggest `iex` when the feed is refused.
- `ops-logging`: Check escaping of quotes and newlines in the message.
- `ops-pnl`: Month and year bases are the last snapshot of the previous period, not the first of the current one.
- `ops-alerts`: A webhook failure must never propagate; secrets must be scrubbed before storing and before sending.
- `ops-report`: The report is what the owner reads: check the disclaimer and the price-return note are present and that a halted or empty system still reports.
- `validate-backtest`: Check what goes into the trial hash, and that a failed run leaves no row behind.
- `validate-crosscheck`: The headline correctness proof. Check the vector inputs are built from stored bars and the weight schedule only, never from the engine result.
- `validate-overfit`: The trial count and Sharpe variance must come from the registry, not from arguments.
- `live-ids`: Count the statuses against the seventeen in architecture 5.9; nothing may fall through to a default state.
- `live-lock`: The lock must be released by the operating system if the process dies; no stale-file logic.
- `live-context`: Every later live test builds on `tests/common/mod.rs`: check it matches the live fixture in the requirements exactly.
- `live-submit`: The crash-safety story: for each order, a lookup by client order id comes before any POST. Check no path POSTs without it.
- `live-settle`: Fills come from the activities endpoint keyed by activity id; running settle twice must change nothing.
- `live-legs`: Check a leg-2 order can never be POSTed while leg 1 is anything but filled.
- `live-reconcile`: Positions must match exactly and cash within the tolerance. Check each break sets a halt and an alert, and writes nothing else.
- `live-snapshots`: Snapshots use stored raw closes and the ledger as of that date, never the broker equity.
- `live-rebalance`: The decision row is written before any order row, and holds everything replay needs: inputs, data instant, feed, config hash.
- `live-rebalance-timing`: The first paper rebalance is exempt from the lateness limit and must be marked `bootstrap`.
- `live-cycle`: Read the phase order against requirement R-313 and check each stage runs only its phases.
- `live-cycle-failures`: These are the failure drills. In each, count the POSTs the mock recorded: it must be zero.
- `live-first-order`: Check the order can be sent only on `valid_on`, only once, and that no rebalance happens in this stage.
- `live-killswitch`: Flatten orders must be authorised as risk-reducing and must leave the system halted.
- `shadow-slippage`: Sign convention: positive means the fill was worse than the reference, for buys and sells.
- `shadow-daily`: On a day without orders the difference must be exactly zero; if it is not, the two books are valued differently.
- `shadow-replay`: Replay must load bars as of the decision data instant; a later restatement must not change the verdict.
- `shadow-fidelity`: Check what counts as a missed session and a late rebalance against requirements R-339 and R-340.
- `shadow-signoff`: No criterion may mention return, profit or Sharpe ratio.
- `cli-base`: The stage table lives in the dispatcher. Check a refused command exits 4 before a client is even built.
- `cli-deploy`: Three weekday runs in New York time; no key and no webhook URL in either file.
- `cli-import`: These commands must work in the backtest stage.
- `cli-data`: Exit code 1 when any universe symbol or doctor check fails.
- `cli-backtest`: The trial count is printed with every backtest so nobody can forget how many were tried.
- `cli-validate`: With one trial, PBO must say it is undefined instead of printing a number.
- `cli-live`: Check the exit codes 0, 1, 2 and 3 and that the report is written on every path except a held lock.
- `cli-ops`: Destructive commands need `--yes`; `halt status` exits 2 while a halt is active.
- `cli-e2e`: These two tests are the acceptance of the whole design. Any source change in this chain is a bug fix elsewhere: read it as one.

## Replans

### config, replan 1: C05 was too big

C05 ("Add the Config structs and parse with the defaults of architecture 5.3") came out as one commit of 459 lines. It is now five steps, C12 to C15 and then C05, each adding one slice of the config to `parse`. C06 to C10 are unchanged. Requirements and architecture are unchanged.

Why it was big, and what is true instead:

- About half of the code was a hand-written reader for decimals, written in the belief that `rust_decimal` had no serde support here. It has: the `serde` feature is one of `rust_decimal`'s defaults and is in `Cargo.lock` (1.43.0). `Decimal` already implements `Deserialize` and reads a TOML integer, float or string. A plain `#[derive(Deserialize)]` field of type `Decimal` is all that is needed; write no `deserialize_with` helper for numbers.
- A TOML float reaches `Decimal` through its shortest round-trip decimal text, never through binary arithmetic, so `max_gross = 1.01` is exactly 1.01 and `target_vol = 0.10` is exactly 0.1. This is the rule of architecture D-11. Checked against the locked versions for the literals in architecture 5.3 and requirement R-034.
- `chrono` has no serde feature here, and none is needed. `data.start` is a quoted string in architecture 5.3: read it as a string and parse it as a `NaiveDate` in one small private helper.
- No step in this chain edits `Cargo.toml` or `Cargo.lock`. The rule that only the `workspace` chain touches them stands.
- The research and shadow cost tables differ only in their defaults. Read both through one private table whose four keys are optional, and lay it over the research or the shadow default. Do not write one reader per table.

Money in the config file is a bare TOML number, as in the block in architecture 5.3 and the checks of R-034 and R-040. It is not a quoted string.

Field types, which architecture 5.3 leaves open and the tests of these steps share:

- `db_path`, `output_dir`, `feed`, `id` and `risk_free_symbol` are `String`.
- `start` is a `NaiveDate`.
- Counts, day lengths and `lookback_months` entries are unsigned integers.
- Every value written with a fraction in architecture 5.3, every `_bps` value, and `max_order_notional` are `Decimal`. `params_from_config` (K05) converts to `f64` where the strategy wants floats.
- `cost.research` and `cost.shadow` are `pt_core` `CostSpec` values.
- Decimal equality ignores trailing zeros (`0.10 == 0.1`), but the printed text does not. Anything that prints a decimal to compare or hash it (C11) must normalise it first.

Tests for C12 to C15:

- Each goes through `pt_config::config::parse` with a config holding one `[[universe]]` entry, as R-031 states it, and asserts only the keys its step names.
- Compare values through their printed text, as `crates/config/tests/defaults.rs` does, so no test pins a type beyond the list above.
- C14 also proves exactness: `max_gross = 1.01`, `target_vol = 0.11` and `max_order_notional = 25000` read back equal to the decimals `1.01`, `0.11` and `25000`.
- C15 also proves that a table with one key set keeps the other three at that table's own defaults: `[cost.shadow]` with only `spread_bps = 10` leaves `impact_coeff` and `borrow_bps_annual` at 0, and leaves `cost.research` untouched.
- `crates/config/tests/defaults.rs` (written and locked before this replan) is the acceptance test of C05 and of R-031 as a whole. It cannot compile before C05, so it must stay out of the worktree until then.

The chain is ten steps long. Every step writes `crates/config/src/config.rs`, so no part of it can run beside another, and moving the later steps to a new chain would change what the chains listing `config` under After wait for.

### alpaca-data, replan 1: E10 was stuck

E10 ("Send the documented daily-bars query") could not be built inside its Files. Bars live on the market data host (`GET {data}/v2/stocks/bars`, architecture 5.9), and the `HttpClient` that the `alpaca-http` chain built can only reach the trading host: its one public request method is `get_trading`, and the rate limiter, auth and retry code behind it are private to `crates/alpaca/src/http.rs`. Calling `get_trading` from `data.rs` would send bar requests to the paper trading host in production, and a test that points both endpoint overrides at one listener would not notice.

The chain gets one new first step, E25, which adds the missing request method. E10 to E14 keep their rows. Requirements and architecture are unchanged.

E25:

- Add `pub fn get_data(&self, path_and_query: &str) -> Result<String, AlpacaError>` to `HttpClient`. It prefixes `endpoints.data()` and calls the same private `get` as `get_trading`. The limiter slot, the auth headers, the 429 and 5xx waits and the 4xx report then hold for the data host with no new code. Do not copy the retry loop.
- It must be `pub`, as `get_trading` is. A test file sees only `pub` items, and a `pub(crate)` method that nothing calls yet fails the dead-code lint.
- Its test builds the client with `HttpClient::new` and two different overrides: the data override is a `common::Listener`, the trading override is a loopback URL where nothing listens (for example `http://127.0.0.1:1`). It asserts that `get_data` returns the body served, and that the listener saw one GET with the path and query passed in and both auth headers (R-082).
- Assert the `Ok` before calling `Listener::finish`. A request sent to the wrong host then fails the test at once with a transport error. `finish` waits for a request that never comes.

E10 to E14, checked against what `data.rs` can reach after E25:

- E10: `daily_bars` builds the query and calls `get_data`. Its test uses the same two different overrides and asserts `daily_bars` returned `Ok` before `finish`, so it proves the host as well as the query. A `bars_query.rs` written before this replan points both overrides at one listener; change that before E10 is committed.
- E11: `pt-alpaca` does not depend on `pt-core` (architecture section 4) and this chain may not edit `Cargo.toml`, so `Price::from_f64_shortest` is out of reach. The rule of architecture D-11 is applied inside `data.rs`: each price becomes a `Decimal` by parsing the shortest round-trip text of the `f64`. A derived `Deserialize` with `Decimal` fields does exactly that for JSON numbers (rust_decimal 1.43.0, as locked), and so does `Decimal::from_str(&x.to_string())`. Nothing may do arithmetic on the `f64`.
- E12: the rule is the one in architecture 5.9: a time of day of 04:00:00Z or 05:00:00Z, and the UTC date. `chrono` alone is enough; no time zone database is declared and none is needed.
- E13: each page is one more `get_data` call with `page_token` added to the query, so each page takes a limiter slot.
- E14: a 403 comes back from `get_data` as `AlpacaError::Rejected { status: 403, message }` after one request, with the raw body as `message`. `data.rs` turns it into `FeedNotPermitted` and reads the server message from the body.

Other chains:

- Bars are the only call to the market data host in architecture 5.9. `pt-data` (G09 to G14, G20) and `pt-live` reach it only through `daily_bars`, so no other step needs `get_data` or `http.rs`.
- `MockAlpaca` serves trading and data routes on one base URL, so tests in other crates cannot see a request sent to the wrong host. E25 and E10 are where that is proven.
- `crates/alpaca/src/http.rs` now belongs to `alpaca-http` and then `alpaca-data`. `alpaca-account` and `alpaca-orders-read` run beside `alpaca-data` and must not touch it; `get_trading` is all they need.
- Not changed here, because it is another chain: `alpaca-write` has the same kind of gap. E22 needs a POST and E24 a DELETE on the trading host, `HttpClient` has neither, and neither step lists `http.rs`. A POST must also skip the 5xx retries (R-101). That chain needs its own replan before it starts. Two things to carry into it: the POST helper must be `pub(crate)`, because a public one would let any crate send an order without a permit (architecture D-17), so it has to arrive in the same step as `submit_order`; and `alpaca-write` then shares `http.rs` with this chain, so it needs `alpaca-data` under After.

Architecture 5.5 shows `HttpClient` with `new` only and names no request method. That is not the cause of this replan and is left as it is: the request methods are called only by `pt-alpaca`'s own modules, and 5.9 already says which host each endpoint is on. It did leave them without a home, since a step builds only the section 5 signatures its text names. The next revision of the architecture should add to 5.5: `pub fn get_trading(&self, path_and_query: &str) -> Result<String, AlpacaError>` and `pub fn get_data(&self, path_and_query: &str) -> Result<String, AlpacaError>`, with the note that write verbs stay `pub(crate)`.

The chain is six steps long. E25 is the only step that touches `http.rs`; E10 to E14 all write `data.rs` and build on one another, so none can move to a chain of its own.

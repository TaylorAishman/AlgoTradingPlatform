# paper-trader: decisions

Owner answers to the blocking questions, recorded 2026-11-23. These sit alongside the fixed owner decisions in `docs/reference/paper-trader-source-brief.md` section 1. Don't reopen them without asking the owner.

## Questions and answers

### Q1. Market data plan
Which Alpaca market data plan should the design assume?
- Options: Basic (free) · Algo Trader Plus ($99/month)
- **Answer: Basic, free.** End-of-day design only. Signals come from daily bars fetched after the close, orders go out the next session, real-time quotes are not used, and requests are throttled to 200/min. Still open, to check with real keys: whether Basic returns consolidated (SIP) daily bars or IEX-only bars. If they are IEX-only, the design must say so.

### Q2. Shorting and margin
May the strategy short and use margin on the paper account?
- Options: long/short with gross ≤ 1.0x · long/flat · long/short up to 2x
- **Answer: Long/short, gross exposure capped at 1.0x equity.** Needs a margin-enabled account, whole-share shorts (Alpaca has no fractional short sales), a shortable/easy-to-borrow check per instrument, and a borrow-cost model in the backtest, because paper charges no borrow fees.

### Q3. Paper account
What paper account does the system own, and at what size?
- Options: dedicated $100k whole shares · dedicated smaller balance · shared account
- **Answer: Dedicated account, reset flat at the default $100k, whole shares only.** The system assumes exclusive control and halts with an alert on any position or order it did not create. There is no fractional-share path. An owner step resets the account, or confirms it is flat, before the first order.

### Q4. Architecture scope beyond time-series momentum
- Options: daily-bar equities/ETFs only · multi-asset, intraday-ready core
- **Answer: Daily-bar equities and ETFs only.** An event-driven engine over bars with a strategy trait. The roadmap names intraday, order-book, options, crypto and LLM-agent strategies as needing their own designs.

### Q5. How much of the roadmap this design turns into buildable chains
- Options: all phases through full paper trading in one plan · offline phases only
- **Answer: All phases through full paper trading, in one plan.** Phases are sequenced with `After`. Owner steps are runtime gates plus a checklist, so they never block a chain. All code is tested against recorded fixtures and a mock Alpaca server, and the binary refuses to send orders until the owner's approval is recorded. Operating stages: backtest only → live data ingest → dry-run orders → one approved order → full paper.

### Q6. Python cross-check
- Options: no Python · dev-only Python check
- **Answer: No Python at all.** Validation uses a second, deliberately simple vectorized calculator in Rust, written from the same cost specification, plus hand-computed golden fixtures. The two must agree exactly at zero cost.

### Q7. Price history before 2016
- Options: Alpaca only, 2016 onward · owner supplies longer history
- **Answer: Alpaca data only, 2016 onward.** The AQR comparison is dropped from acceptance. A generic CSV bar importer is still built so longer history can be added later.

### Q8. Publishing status and P&L to the public README
- Options: local report only · automatic publish
- **Answer: Local report only.** The system writes a status and P&L report file on the run machine, and the owner pastes what they want into the README. The trading machine holds no push credentials.

## Architect's assumptions (from questions mode)

The team decides these. The owner may override any of them.

- **Companion literature review:** added later as `docs/reference/literature-review-for-builders.md`. It is guidance, not a decision; the authority order is in architecture.md section 2.1.
- **PDFs are local only:** they are gitignored and absent from build-agent worktrees, so every formula the build needs must be transcribed into the design docs.
- **Universe:** about 30-40 liquid US-listed ETFs covering equity regions, Treasuries by maturity, credit, commodities, currencies and real estate, kept as a candidate list in config. A command checks that each is tradable, shortable and has history since 2016, run against Alpaca once the owner supplies keys. No crypto, options or single stocks.
- **Strategy parameters:** they follow Moskowitz, Ooi and Pedersen and are fixed in advance, with no parameter search. 12-month lookback, 1-month hold, sign signal, EWMA volatility with a 60-day center of mass annualized with 261. The 1/3/12-month blend is a config option, off by default. A T-bill ETF stands in for the risk-free rate. The trial count is recorded in SQLite so the Deflated Sharpe Ratio stays honest.
- **Sizing and risk limits:** all in config, and the owner can override them. A portfolio volatility target of about 10% annualized (instead of 40% per instrument), a per-instrument weight cap, gross ≤ 1.0x (Q2), a maximum order notional, a daily-loss halt, and a hard lock so the binary talks only to the paper endpoint.
- **Validation:** the Deflated Sharpe Ratio, the Probability of Backtest Overfitting and a volatility-scaled buy-and-hold benchmark, all in Rust. Sign-off thresholds measure system fidelity (paper fills against simulated fills, reconciliation breaks, missed runs), not strategy returns. A few months of paper trading cannot validate a strategy.
- **Engine rules from the Implementation Risk paper:** one explicit cost specification, net-position semantics, sells before buys in a rebalance, a single calendar source, a hard failure on a truncated or short run, and decimal arithmetic for money (floats only for statistics).
- **Process shape:** a run-to-completion CLI driven by cron or launchd. It is idempotent and catches up after a missed day. There is no daemon. Alerts go to a log plus a generic webhook whose URL the owner supplies at deploy time.
- **Order handling:** signals on the close of day T, day orders in the regular session on T+1, and a deterministic `client_order_id` so retries are idempotent. Order status comes from REST polling in the first phases, and the `trade_updates` websocket is deferred. No market-on-close orders. Partial fills are handled as normal.
- **Dividends:** paper does not simulate them, so signals and backtest P&L use adjusted prices and the paper comparison uses price return.
- **Alpaca client:** the team writes its own thin REST client, because `apca` is GPL-3.0-or-later and this repo is MIT. Check the bars `feed` and `adjustment` parameters and the `trade_updates` schema against the docs during design.
- **Storage:** SQLite for everything (bars, signals, orders, fills, ledger, trial registry, run log). No Parquet or polars yet.
- **Layout:** one Cargo workspace with small crates along component lines. CHECK becomes fmt, clippy with warnings denied, tests and build. The integrator changes `.devteam/config` and CI; this is not a chain.
- **Secrets:** keys are read from environment variables or a file outside the repo, and are never committed or logged. Agents never hold them. Every test runs on fixtures or a mock server.
- **Owner steps as runtime gates:** keys before the first real-API smoke test; data plan before ingest config (default Basic); machine and alert destination before scheduling; first-order approval; validation sign-off; and a strategy allowlist that only the owner edits. No chain waits on any of them.
- **Live trading:** no live endpoint, no live key handling, and nothing configurable to enable it.

## Facts confirmed 2026-11-23 that update the brief

- A paper account can be reset to an arbitrary balance, and more than one paper account can be created.
- Fractional orders now support market, limit, stop and stop-limit with day time-in-force. The brief's "market orders only" note is outdated.
- Fractional short sales are not supported.

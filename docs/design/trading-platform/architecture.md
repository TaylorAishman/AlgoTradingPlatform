# Architecture: trading-platform

Stage 1 of 3 (architecture). Written 2026-11-21. Needs human approval before the requirements stage starts.

Conventions used in this document:

- Papers are cited by the leading words of their file name in `docs/reference/trading-papers/`, in italics, followed by a page: *Multi-Period Trading* p12. Section 2.1 maps every short title to its full file name.
- Page numbers are PDF page indices (page 1 is the first page of the file), not the printed page numbers.
- "Target" marks a number chosen by this design. "Fact" numbers always carry a citation.
- The repository holds no code yet (only `LICENSE` and `README.md` are tracked; the devteam scaffolding is untracked), so there are no existing code patterns to follow beyond what `.devteam/verify.sh` auto-detects: a root `pyproject.toml`, `ruff check .` and `pytest -q`.

## 1. Problem

Build a self-hosted, single-user platform that turns end-of-day data for US equities and ETFs into daily rebalanced portfolios, on an event-driven core that can later run minute bars. The owner uses it to research signals (classical and machine-learning first, LLM-derived signals as an optional plugin), test them in a cost-aware simulator behind statistical validation gates, and then run the survivors against Alpaca's paper-trading API. Its purpose is to tell real edge from backtest artefact before any capital is involved; live trading is not part of this design.

## 2. Sources

### 2.1 Reference documents

All 31 files in `docs/reference/trading-papers/` were read at least at the level of abstract, introduction and conclusions; the ones marked **Drives** were read in the sections cited. Role is one of: **Drives** (a decision in section 6 rests on it), **Informs** (shapes a default, a test or a caveat), **Not used** (with the reason).

| # | File | Role | What it contributed |
| --- | --- | --- | --- |
| 1 | `Multi-Period Trading via Convex Optimization.pdf` | Drives | The accounting model for holdings, trades, cash and the self-financing condition (p11, p17-18); the transaction cost form `a\|x\| + b*sigma*\|x\|^(3/2)/V^(1/2)` (p12-13, eq. 2.2); holding cost; the list of effects a simulator ignores (p18-20); simulation inputs and the advice to re-simulate the actually executed trades and compare (p20-21); performance metrics (p22-23); single-period optimization with holding and trading constraints, soft constraints and solve times (p26-41); hyper-parameter search by backtest (p44-45, p64-66); multi-period optimization as a later extension (p48-52). States plainly that it does not address forecasting (p4, p7). |
| 2 | `Markowitz Portfolio Construction at Seventy.pdf` | Drives | The default portfolio problem ("Markowitz++", eq. 19 on p24): robust return and risk terms (p22-24), weight, leverage, turnover and participation limits (p14-16), cost terms (p17, eq. 10), soft constraints and why infeasibility appears in production (p25-27), the data-versus-parameter split (p25), backtest with forecasts in the optimizer and realized values in the simulator (p29), parameter tuning discipline (p30), EWMA covariance (p22, p31). |
| 3 | `Building Diversified Portfolios that Outperform Out-of-Sample.pdf` | Drives | Hierarchical Risk Parity as an allocator that needs no return forecast and no covariance inversion (p2, p5-8); the instability argument against quadratic optimizers (p3-4). |
| 4 | `The Deflated Sharpe Ratio - Correcting for Selection Bias, Backtest Overfitting and Non-Normality.pdf` | Drives | The trial registry and the significance gate: a backtest without its number of trials "is worthless" (p3); expected maximum Sharpe ratio under N trials (p7, eq. 1); DSR with its inputs (p8-9, eq. 2); the 95% level used in the worked example (p9-10); effective number of independent trials from average correlation (p14-15, eq. 9); plan trials in advance (p10). |
| 5 | `The Probability of Backtest Overfitting.pdf` | Drives | PBO defined on the selection process (p10-11); the CSCV algorithm on a T x N matrix of per-period results (p12-13); the companion statistics performance degradation and probability of loss (p14-15); the customary rule to reject when PBO exceeds 0.05 (p15); choice of S (p23); limits: all trials must be disclosed, guided searches count once, PBO says nothing about backtest correctness (p25), and PBO must not become an optimization target (p26). Why a single hold-out is not enough (p8). |
| 6 | `Implementation Risk in Portfolio Backtesting - A Previously Unquantified Source of Error.pdf` | Drives | The simulator's verification design: engines agree exactly at zero cost and diverge through the cost model (p1, p41); failure taxonomy (p13); concrete defects to design against: commission rate silently divided by 100 (p32), buys rejected unless sells go first (p33), two calendar instances (p33), netting versus hedging, silent truncation after 62 of 1,258 days, double fee deduction (p34); a minimal auditable reference loop (Algorithm 1, p43-45); multi-engine comparison as the diagnostic (p2, p4). |
| 7 | `Qlib - An AI-oriented Quantitative Investment Platform.pdf` | Drives | The module chain data, features, model, portfolio generator, order executor, analysers (p3-4, Fig. 1); the executor as a responsive simulator, not a backtest function (p4); the data model `x[instrument, time, attribute]` with time-varying instrument pools and append-only updates (p4); flat columnar files with one shared calendar (p5); caching of derived data (p5). |
| 8 | `Time Series Momentum.pdf` | Drives | A baseline signal: sign of the trailing 12-month excess return, held one month, sized to constant ex-ante volatility (p3, p18); the EWMA volatility estimator with a 60-day centre of mass using only data up to t-1 (p13, eq. 1). Evidence is on 58 futures and forwards, not on US stocks or ETFs (p2). |
| 9 | `Statistical Arbitrage in the U.S. Equities Market.pdf` | Drives | A baseline signal: residual of each stock against its sector ETF (or PCA factors) modelled as mean-reverting, traded on the s-score (p1-4, p45-46, eq. 21-22); a universe defined point in time by size at the trade date (p4); parameters fixed once "to avoid data-mining" (p4); 5 bps per trade cost assumption (p4); Sharpe decay after 2002 and the August 2007 drawdown (p1). |
| 10 | `Empirical Asset Pricing via Machine Learning.pdf` | Drives | The machine-learning predictor design: penalized linear models, dimension reduction, trees and shallow networks compared (p5-6); OLS on many predictors fails out of sample (p6); train, validation and test periods in temporal order with annual recursive refit (p10, p26); Huber loss for heavy tails (p12); momentum, liquidity and volatility as the dominant predictor families (p2). |
| 11 | `Financial Machine Learning.pdf` | Informs | Why expectations must be modest: small data, weak signal-to-noise, patterns arbitraged away (p13-14); validation results are themselves selection-biased, hence a third test sample (p37-38); ML predictability concentrates in small illiquid stocks and often disappears net of costs, so a model without cost guidance cannot tell tradeable patterns from untradeable ones (p130-132); pointers to public characteristic data sets (p36). |
| 12 | `And the Cross-Section of Expected Returns.pdf` | Drives | The signal-level hurdle: with hundreds of factors tried, a new factor needs a t-ratio above 3.0, not 2.0 (p2, p34); Bonferroni, Holm and BHY adjustments (p15). |
| 13 | `Is There a Replication Crisis in Finance.pdf` | Informs | The counter-position to #12: most factors replicate, and cluster into 13 themes (p2-3). Supports counting correlated trials as fewer independent ones and prefers published, replicated signals as baselines. |
| 14 | `Does Academic Research Destroy Stock Return Predictability.pdf` | Informs | Predictor returns are 26% lower out of sample and 58% lower after publication (p1). Expected returns are calibrated on recent realized data, never on a paper's reported magnitude. |
| 15 | `The Virtue of Complexity in Return Prediction.pdf` | Informs | Claims models with more parameters than observations predict market returns better (p2). Model complexity is therefore allowed, and recorded as a trial dimension. |
| 16 | `Seemingly Virtuous Complexity in Return Prediction.pdf` | Drives | The rebuttal of #15: the complex model reduces to a volatility-timed momentum rule and behaves the same on artificial data with reversals (p2-3). Source of two validation checks: every predictor must beat simple named baselines, and must be run on placebo data. |
| 17 | `Optimal Execution of Portfolio Transactions.pdf` | Informs | The execution-schedule interface: a trade list worked over several slices trades expected cost against risk (p1, p3), the schedule can be fixed in advance (p4), and a risk-averse trader should not simply minimise expected cost (p35). Degenerates to one slice for daily auction orders; becomes active with minute bars. |
| 18 | `Strict Universality of the Square-Root Law in Price Impact Across Stocks - A Complete Survey of the Tokyo Stock Exchange.pdf` | Informs | Fixes the impact exponent: price impact scales with the square root of traded volume, exponent 1/2 within error for every liquid stock studied (p1-2). Evidence is from Tokyo, not the US. |
| 19 | `Assessing Look-Ahead Bias in Stock Return Predictions Generated By GPT Sentiment Analysis.pdf` | Drives | LLM safeguards: a backtest inside the model's training window is biased by look-ahead and by a "distraction" effect (p1-3); only the period after the training cutoff is out of sample (p2); anonymizing company identifiers measures and reduces the bias and lowers market beta (p3, p14). |
| 20 | `Look-Ahead-Bench - A Standardized Benchmark of Look-ahead Bias in Point-in-Time LLMs for Finance.pdf` | Drives | The alpha-decay diagnostic: compare performance inside and after the training window (p1-2); warns that masked entities can still be recognised from context (p1). |
| 21 | `LLM Trading Agents Earn Beta, Not Alpha - A Look-Ahead-Free Benchmark.pdf` | Drives | News must be visible only after its publication minute; joining by calendar date manufactures an edge (p1). Decide at the close, fill at the next open (p4). Carry each model's training cutoff and score contaminated periods apart (p4). The look-ahead check that replaces the future with random data and replays (p2, p4). Factor attribution, same-day same-sector random controls, news ablations, several seeds (p1-2, p6, p8). In 91 runs no agent had significant alpha (p6), so an LLM is treated as a signal source and rule executor, not as the edge. |
| 22 | `Can Blindfolded LLMs Still Trade - An Anonymization-First Framework for Portfolio Optimization.pdf` | Informs | The anonymization protocol (tickers, names, subsidiaries, products replaced by neutral identifiers, p1-2); negative-control experiments by random shuffling (p2); point-in-time index membership against survivorship bias (p2); results reported across 20 seeds (p1). Its GNN and RL policy are not adopted. |
| 23 | `Can ChatGPT Forecast Stock Price Movements - Return Predictability and Large Language Models.pdf` | Informs | The case that an LLM news-sentiment signal can exist: headline scores after the knowledge cutoff predict a one-to-two-day drift, strongest in small stocks and negative news (p1, p3); trades at the opening and closing auctions; the strategy survives 5 and 10 bps round trip and is unprofitable at 20 bps (p20); returns decline as adoption rises (p1). |
| 24 | `TradingAgents - Multi-Agents LLM Financial Trading Framework.pdf` | Informs | Structured, schema-checked model outputs instead of free text passed between steps (p2); a risk-management role between analysis and orders (p1). Its evidence is a three-month window chosen for cost reasons with Sharpe ratios the authors call beyond the expected range (p12), so the multi-agent debate architecture is not adopted. |
| 25 | `Agentic Trading - When LLM Agents Meet Financial Markets.pdf` | Drives | Safety layers must sit outside the model: pre-trade exposure checks, hard position limits, concentration caps, drawdown triggers and a kill switch, enforced as deterministic checks (p16, p26, p35); a budget on reflection loops so a system cannot retry its way to a good-looking result (p35). |
| 26 | `R&D-Agent-Quant - A Multi-Agent Framework for Data-Centric Factors and Model Joint Optimization.pdf` | Informs | Argues that LLM output should be an inspectable factor, not a direct trading decision (p2). Its automated factor-mining loop is not adopted: it multiplies trials, which #4 and #5 penalise. If such a loop is ever added, every generated candidate must be registered as a trial. |
| 27 | `Pretrained Time-Series Foundation Models for Financial Return Forecasting.pdf` | Informs | Pretrained time-series models win rankings but their gains over a random walk are "small and sparse" (p1). They fit the predictor plugin interface and get no special treatment. |
| 28 | `Re(Visiting) Time Series Foundation Models in Finance.pdf` | Informs | Off-the-shelf time-series foundation models "generally perform poorly"; public pre-training corpora may overlap the evaluation period, the same contamination problem as LLMs (p1, p20 footnote 10); judge by portfolio results, not forecast error (p2, p14). |
| 29 | `Recent Advances in Reinforcement Learning in Finance.pdf` | Not used (interface only) | Surveys RL for execution, portfolio choice, hedging, market making and order routing (p1). The user chose classical and ML signals as the core, so no RL component is designed. Its one effect: the simulator is steppable (as in #7), so an RL policy could later implement the allocator interface. |
| 30 | `High-Frequency Trading in a Limit Order Book.pdf` | Not used | Optimal bid and ask quoting for a dealer under inventory risk (p1). Needs order-book data and intraday quoting; excluded by the frequency decision. |
| 31 | `DeepLOB - Deep Convolutional Neural Networks for Limit Order Books.pdf` | Not used | Deep network on order-book levels predicting short-horizon price moves (p1). Needs level-2 order-book data; excluded by the frequency decision. |

### 2.2 Sources outside `docs/reference/`

The papers say nothing about Alpaca, so the following were checked on 2026-11-21. Everything Alpaca-specific in this document comes from these pages.

| Source | Used for |
| --- | --- |
| `docs.alpaca.markets/docs/paper-trading` | Paper base URL `https://paper-api.alpaca.markets`, separate keys, $100k default balance, fill rules (filled only when marketable, against NBBO, order size not checked against quote size, random partial fills 10% of the time), and what paper trading does not simulate: market impact, information leakage, latency slippage, queue position, price improvement, regulatory fees, dividends; borrow fees marked "coming soon". |
| `docs.alpaca.markets/docs/orders-at-alpaca` and `/reference/postorder` | Order types, time-in-force values, auction cut-offs (OPG orders rejected between 9:28am and 7:00pm ET, CLS between 3:50pm and 7:00pm ET; after 7:00pm they queue for the next session), fractional and notional rules (DAY only, never OPG or CLS), buying-power checks, order statuses, `client_order_id` up to 128 characters, HTTP 403 and 422 meanings. |
| `docs.alpaca.markets/docs/websocket-streaming` | The `trade_updates` stream, event types and fill fields. |
| `docs.alpaca.markets/docs/about-market-data-api`, `/docs/historical-stock-data-1`, `/reference/stockbars` | Free Basic plan: IEX real time, historical data since 2016 except the latest 15 minutes, 200 calls per minute; Algo Trader Plus at $99 per month: all exchanges, 10,000 calls per minute. IEX is about 2.5% of market volume. Bars endpoint parameters: `timeframe` (minute to month), `adjustment` (`raw` default, `split`, `dividend`, `spin-off`, `all`), `feed`, `asof`, 10,000 data points per page. |
| `docs.alpaca.markets/reference/news-3`, `/docs/historical-news-data` | News fields (`headline`, `summary`, `content`, `symbols`, `created_at`, `updated_at`, `source`), 50 articles per page. |
| `docs.alpaca.markets/reference/corporateactions-1`, `/reference/get-v2-assets-1` | Corporate action types and dates; the warning that actions "may not be available immediately after announcement"; asset flags `status`, `tradable`, `shortable`, `fractionable`, `borrow_status`. |
| `docs.alpaca.markets/docs/margin-and-short-selling` | Margin and shorting need $2,000 equity; 2x overnight buying power; easy-to-borrow names carry no borrow fee, hard-to-borrow names need a locate; margin interest formula. |
| Alpaca community forum threads (unofficial) | Delisted symbols remain in the database but are not reliably retrievable; the asset list can omit dead names; a reused ticker can mix two companies' histories. |
| PyPI metadata for `alpaca-py` (0.45.0, Python >= 3.10) and `cvxpy` (1.9.3, Python >= 3.11, ships Clarabel, OSQP, SCS, HiGHS) | Language floor and solver availability. |
| FINRA Regulatory Notice 26-10 | The pattern-day-trader rule was replaced in 2026; Alpaca's handling is unconfirmed. Daily rebalancing does not day-trade, so the design does not depend on it. |

### 2.3 Conflicts between references

None of these is resolved silently. The design either supports both sides behind one interface or picks a default and says so; the human is asked to confirm in section 8, because the "authoritative document" table in `docs/reference/README.md` is still empty.

| # | Topic | One side | Other side | What the design does |
| --- | --- | --- | --- | --- |
| C1 | Are mean-variance optimizers fit for use? | *Building Diversified Portfolios* p3-4: quadratic optimizers are unstable, concentrated and lose to naive portfolios out of sample. | *Markowitz at Seventy* p5-6: the deficiencies are "alleged"; regularization, robust terms and constraints fix them. | Both sit behind the `Allocator` interface. Markowitz++ is the default when a return forecast exists, HRP when it does not, and equal-weight and inverse-volatility are mandatory baselines. The validation gates decide per strategy. |
| C2 | Is published factor evidence trustworthy? | *And the Cross-Section* p2: most claimed findings are likely false; require t > 3.0. | *Is There a Replication Crisis* p2-3: most factors replicate; the large number of factors strengthens the evidence. | New, in-house signals face the t > 3.0 hurdle. Whether replicated published signals get a lower hurdle is open question Q6. Both papers agree that out-of-sample evidence counts and that correlated variants are not independent tests, which is how the trial registry counts. |
| C3 | Does model complexity help? | *Virtue of Complexity* p2: yes, even with more parameters than observations. | *Seemingly Virtuous Complexity* p2-3: the gain is a disguised volatility-timed momentum rule. | Complex predictors are allowed but must beat the simple baselines in the same harness and pass a placebo-data test. |
| C4 | Shape of price impact | *Optimal Execution* p1, p36: linear temporary and permanent impact. | *Strict Universality* p1 and the 3/2-power cost in *Multi-Period Trading* p12 and *Markowitz at Seventy* p17: square-root impact. | The cost model uses the 3/2-power form. The linear model survives only as the quadratic approximation an Almgren-Chriss schedule would use, which is not built in the first version. |
| C5 | Can an LLM find alpha in news? | *Can ChatGPT Forecast* p1, p3: yes, post-cutoff headline scores predict drift. *TradingAgents* p1 and *R&D-Agent-Quant* p1 report strong backtests. | *LLM Trading Agents Earn Beta* p1-2, p6: no significant alpha in 91 runs; news score rank correlation of 0.004 with next-five-day returns; agents did as well without headlines. | The LLM signal is optional, off by default, and must pass the same gates net of costs, on post-cutoff data only, with ablations. |
| C6 | Is anonymization enough against memorization? | *Assessing Look-Ahead Bias* p1-3: anonymization removes the bias and even helps out of sample. | *Look-Ahead-Bench* p1: models recognise masked entities from context. *LLM Trading Agents Earn Beta* p1: hiding tickers lowers recall but does not measure it. | Anonymization is applied but is not treated as sufficient: the training-cutoff rule is the binding safeguard. |
| C7 | Is a hold-out or validation split adequate? | *Empirical Asset Pricing via ML* p10 and *Financial Machine Learning* p37-38: temporal train, validation and test split is the standard design. | *Deflated Sharpe Ratio* p6 and *Probability of Backtest Overfitting* p8: a hold-out ignores the number of trials and cannot detect selection bias. | Both are used for different jobs: the temporal split fits and tunes models; DSR and PBO over the registered trials judge the strategy selection. Paper trading is the only sample treated as untouched. |
| C8 | Separate forecasting from portfolio construction? | *Multi-Period Trading* p4, p26 and *Markowitz at Seventy* p4: forecasts are inputs; the optimizer exploits them however they were made. | *Financial Machine Learning* p130-132: a predictor trained without cost guidance isolates untradeable patterns. *Qlib* p2 notes end-to-end RL as an alternative. | The pipeline is separated, but no signal is judged on forecast accuracy alone: every candidate is scored net of costs through the full portfolio and simulation path, as *Multi-Period Trading* p26 itself recommends. |
| C9 | How often to trade | *Time Series Momentum* p18 and *Empirical Asset Pricing via ML* p6: monthly horizons. | *Statistical Arbitrage* p4 and *Can ChatGPT Forecast* p3: daily. | The user fixed a daily decision cycle. Each signal declares its horizon, and the trade-aversion term (*Multi-Period Trading* p44) sets realized turnover, so a monthly signal does not trade daily. |
| C10 | What do costs amount to? | 5 bps per trade (*Statistical Arbitrage* p4); 5, 10 and 20 bps round trip (*Can ChatGPT Forecast* p20); 0, 18, 36, 60 bps (*Implementation Risk* p1); spread plus impact with coefficients fitted to realized costs (*Multi-Period Trading* p13). | - | No reference gives calibrated US costs for a small account. Cost parameters are explicit configuration with conservative defaults, and every gate is re-run at twice the cost. |

### 2.4 Gaps the references leave

1. **Broker, operations, order management.** No paper covers them. Covered from Alpaca's documentation (section 2.2) and from engineering judgement, marked as such.
2. **History length.** Alpaca history starts in 2016, about 2,740 sessions; the papers use decades (60 years in *Empirical Asset Pricing via ML* p5, 25 in *Time Series Momentum* p3). Statistical power is limited, which the DSR's sample-length term reflects but cannot cure.
3. **Survivorship.** Alpaca offers no point-in-time index membership and its coverage of delisted names is not documented. Two reference papers themselves use survivorship-biased samples and say so (*Markowitz at Seventy* p31; *Multi-Period Trading* p59).
4. **Fundamentals.** The 94 firm characteristics of *Empirical Asset Pricing via ML* need accounting data, and none of the Alpaca pages consulted offers it. Only price- and volume-derived features are assumed reachable.
5. **Score to expected return.** The references prescribe winsorizing and cross-sectional standardization (*Markowitz at Seventy* p22) but not how a score becomes an expected return.
6. **Thresholds.** Three are given: the DSR's 95% level (*Deflated Sharpe Ratio* p9-10), the t > 3.0 hurdle (*And the Cross-Section* p2) and the customary PBO limit of 0.05 (*Probability of Backtest Overfitting* p15). The alpha hurdle after factor attribution, paper-trading duration, risk limits and tracking tolerances are not.
7. **Cost calibration.** The impact coefficient is "around one" before fitting (*Multi-Period Trading* p13); paper trading simulates no impact, so it cannot be fitted there.
8. **Transfer of evidence.** Time-series momentum is shown on futures, the square-root law on Tokyo stocks; neither on the instruments traded here.
9. **Equations.** No PDF renderer was available while writing this document, so formulas were read from a text extraction that garbles them (notably *Deflated Sharpe Ratio* eq. 1-2 and its numerical example). Implementers must transcribe formulas from the PDFs, and tests must reproduce the papers' worked examples.
10. **Unverified Alpaca behaviour.** Whether a paper-only account may query delayed SIP history at all: the paper-trading page says such accounts are entitled to IEX data only, while the market-data pages describe 15-minute-delayed historical access on the free plan. If only IEX bars are available, daily opens, closes and volumes come from about 2.5% of the market and the liquidity filters and fill prices in this design are not usable as written. Also: how paper trading prices OPG orders; whether the daily bar's open equals the opening auction price; what a duplicate `client_order_id` returns; the trading API rate limit; calendar endpoint details; the news provider and history depth. Each is to be measured in the first paper sessions or confirmed from the API reference before the code depends on it.
11. **Taxes, wash sales, regulation.** Not covered by any reference; out of scope for a paper account.

## 3. Constraints

**Scope (decided by the user).** US equities and ETFs only. End-of-day signals and daily rebalancing; the core must not assume one bar per session. No tick or order-book strategies. LLM signals are an optional plugin behind point-in-time safeguards and the same gates as any signal. Validation target is Alpaca paper trading.

**Paper only.** The broker adapter must refuse any trading endpoint other than `https://paper-api.alpaca.markets`. There is no configuration switch for live trading in this design.

**Alpaca compatibility.**

- Auction orders (`opg`, `cls`) cannot be fractional or notional and have submission windows; fractional orders must be `day` orders.
- Buying power is reduced by open orders and is not replenished by a sell until it executes, so a rebalance submitted as one batch needs spare buying power for its buys.
- Short selling needs a margin account with at least $2,000 equity and is limited by this design to easy-to-borrow names.
- Free-plan data: history is queryable once it is 15 minutes old (whether that includes the SIP feed for a paper-only account is unverified, gap 10); 200 calls per minute; bars return at most 10,000 points per page; news 50 per page.
- IEX volume (about 2.5% of the market) must not be used for liquidity or participation estimates; use SIP bars.
- Paper trading simulates neither dividends nor impact, so paper results and simulated results differ by construction and must be reconciled on a like-for-like basis.

**Data.** Every stored record carries the time it became knowable. No component other than the data layer may read storage directly. Market data and news text are licensed content and the repository is public: neither they nor API keys may be committed. Tests run on synthetic data.

**Performance (targets).**

- Daily decision job, including ingestion, for a 500-asset universe: at most 10 minutes on one laptop core. For scale, *Multi-Period Trading* p40 reports a 1,500-asset, 50-factor single-period problem solving in under half a second.
- Ten-year daily backtest of a 500-asset universe: at most 10 minutes on one core.
- `./.devteam/verify.sh`: at most 5 minutes, offline, deterministic.
- Full backfill of daily bars for 3,000 symbols since 2016: within the free plan's 200 calls per minute (about 820 pages).

**Dependencies.** Python, floor 3.11 (imposed by `cvxpy`). Runtime: `numpy`, `pandas`, `pyarrow`, `scipy`, `scikit-learn`, `cvxpy` with its bundled Clarabel solver, `alpaca-py`, `pydantic`. Development: `pytest`, `hypothesis`, `ruff`, `mypy`. Gradient-boosted trees use scikit-learn's implementation, so no further ML dependency is needed. No database server, no message broker, no cloud service. An LLM provider is needed only if the optional plugin is enabled.

**Must not change.** `docs/reference/`, `REVIEW.md`, `.devteam/` and `.github/workflows/` are protected. The verification script and CI need changes that only a human can make (section 7.2).

## 4. Components

```
 Alpaca market data                                                    Alpaca paper trading API
 (bars, corporate actions, assets, calendar, news)                                 ^
        |                                                                          |
        v                                                                          |
 +-------------+   DataView(as_of)   +-----------+  Forecast  +-------------+  TargetPortfolio  +----------------------+
 |  B  Data    | ------------------> | C Signals | ---------> | E Portfolio | ----------------> | G Execution           |
 |  PIT store  |                     +-----------+            +-------------+                   | planner, risk gate,  |
 +-------------+                        ^     ^                 ^                               | paper broker,        |
        |                               |     |                 |                               | reconciliation       |
        |                      +--------+   +-----------+   +----------+                        +----------------------+
        |                      | I LLM  |   | (optional)|   | D Models |                                   ^
        |                      | plugin |   +-----------+   | risk,cost|---- same cost code ----+          | Broker protocol
        |                      +--------+                   +----------+                        v          |
        |                                                                         +---------------------------------+
        +-----------------------------------------------------------------------> | F Engine and simulation         |
                                                                                  | clock, events, ledger, SimBroker|
                                                                                  +---------------------------------+
                                                                                                 |
                                                                                                 v  runs, returns, trials
 +---------------------------------------------------+        +---------------------------------------------------------+
 | J Operations: CLI, jobs, config, reports, package |        | H Validation: registry, metrics, DSR, PBO, attribution, |
 +---------------------------------------------------+        |   leak and placebo tests, promotion gates               |
                                                              +---------------------------------------------------------+
 A Core contracts (types, events, clock, calendar, protocols, synthetic market) is imported by every component.
```

The chain follows *Qlib* p3-4 (data, features and model, portfolio generator, order executor, analysers) with the forecast and optimizer split of *Markowitz at Seventy* p4. One strategy pipeline runs unchanged in backtest and in paper trading; only the clock and the broker differ.

| Prefix | Component | Responsibility | Owns |
| --- | --- | --- | --- |
| A | Core contracts | Domain types, events, clock and trading calendar, the protocols in section 5, configuration loading, error types, and the synthetic market generator every test uses. No business logic. | `src/atp/core/`, `src/atp/testing/`, `tests/core/` |
| B | Data | Ingestion from Alpaca (bars, corporate actions, assets, calendar, news); the append-only point-in-time store; the `DataView`; price adjustment; point-in-time universe construction; data quality checks. | `src/atp/data/`, `tests/data/` |
| C | Signals | Feature functions; the baseline signals (time-series momentum, residual mean reversion, price and volume characteristics); ML predictors with walk-forward training; the calibrator that turns scores into a `Forecast`. | `src/atp/signals/`, `tests/signals/` |
| D | Risk and cost models | Volatility and covariance estimators (EWMA, statistical factor model); the transaction cost and holding cost models, each with one implementation that serves both the optimizer and the simulator. | `src/atp/models/`, `tests/models/` |
| E | Portfolio construction | Allocators: Markowitz++ single-period convex optimizer, HRP, inverse-volatility, equal-weight; constraint handling; rounding heuristics. | `src/atp/portfolio/`, `tests/portfolio/` |
| F | Engine and simulation | The event loop; the ledger (holdings, cash, costs); the simulated broker with its fill models; the minimal reference backtest loop used as a test oracle; run recording. | `src/atp/engine/`, `tests/engine/` |
| G | Execution | Order planning from targets; the deterministic pre-trade risk gate and kill switch; execution schedules; the Alpaca paper broker adapter; order state tracking; reconciliation of paper results against a shadow simulation. | `src/atp/execution/`, `tests/execution/` |
| H | Validation | The append-only trial registry; performance metrics; PSR and DSR; PBO by CSCV; multiple-testing adjustments; factor attribution; look-ahead, placebo, negative-control and cost-stress tests; promotion gates and reports. | `src/atp/validation/`, `tests/validation/` |
| I | LLM signal plugin (optional) | News-to-score pipeline: point-in-time news selection, anonymizer, prompt and output schema, provider adapter, response cache, model registry with training cutoffs, seed handling, ablation modes. Implements the `Signal` protocol and nothing else. | `src/atp/llm/`, `tests/llm/` |
| J | Operations | Packaging and tooling; the `atp` command line; the daily jobs; configuration files; logging and alerts; reports; the runbook; end-to-end tests. | `pyproject.toml`, `src/atp/cli/`, `src/atp/ops/`, `configs/`, `docs/runbook.md`, `tests/e2e/`, `.gitignore` |

Dependency rule: A depends on nothing. B, C, D, E, F, G, H depend on A's protocols and on each other only through them. I depends on A and on B's `DataView`. J wires concrete classes together and is the only component that imports them all. Implementations are named in configuration by dotted path (for example `atp.signals.tsmom:TimeSeriesMomentum`), so adding a signal or allocator never edits a shared registry file.

## 5. Interfaces

Signatures are Python. Times are timezone-aware UTC; a session is an exchange date in `America/New_York`. Money is USD. Weights are fractions of portfolio value with cash separate, as in *Markowitz at Seventy* p13 (`1'w + c = 1`).

### 5.1 Core types (A)

```python
AssetId = NewType("AssetId", str)        # stable internal id; a ticker is an attribute with a validity range, never a key
Frequency = Literal["1Day", "1Min"]      # "1Min" reserved; first version ingests "1Day" only

@dataclass(frozen=True)
class Bar:
    asset: AssetId; session: date; start: datetime; end: datetime; frequency: Frequency
    open: float; high: float; low: float; close: float; volume: float; vwap: float; trade_count: int
    adjustment: Literal["raw", "split", "all"]; available_at: datetime

@dataclass(frozen=True)
class NewsItem:
    news_id: str; assets: tuple[AssetId, ...]; headline: str; summary: str; source: str
    created_at: datetime; available_at: datetime      # available_at >= created_at, minute resolution or finer

@dataclass(frozen=True)
class Forecast:
    as_of: datetime; horizon_sessions: int
    mu: pd.Series            # expected return per asset over the horizon
    rho: pd.Series           # non-negative uncertainty half-width per asset (Markowitz at Seventy p22-23)
    sources: Mapping[str, float]   # signal name -> weight actually used

@dataclass(frozen=True)
class TargetPortfolio:
    as_of: datetime; weights: pd.Series; cash_weight: float
    status: Literal["optimal", "relaxed", "fallback_hold"]; diagnostics: Mapping[str, float]

@dataclass(frozen=True)
class OrderRequest:
    client_order_id: str     # deterministic, <= 128 chars: f"{strategy}-{session}-{asset}-{side}-{slice}"
    asset: AssetId; symbol: str; side: Literal["buy", "sell"]; qty: Decimal
    type: Literal["market", "limit"]; time_in_force: Literal["opg", "cls", "day"]
    limit_price: Decimal | None; decision_as_of: datetime

@dataclass(frozen=True)
class Fill:
    client_order_id: str; asset: AssetId; side: Literal["buy", "sell"]
    qty: Decimal; price: Decimal; filled_at: datetime; fees: Decimal

@dataclass(frozen=True)
class Position:        asset: AssetId; qty: Decimal; market_value: Decimal
@dataclass(frozen=True)
class AccountSnapshot: as_of: datetime; equity: Decimal; cash: Decimal; buying_power: Decimal
```

### 5.2 Events and decision timing (A, F)

Events: `SessionOpen`, `BarClosed(frequency)`, `DataReady(as_of)`, `Decision(as_of)`, `OrdersSubmitted`, `FillReceived`, `CorporateActionApplied`, `SessionClose`, `KillSwitch`. Handlers are the same in simulation and paper trading.

```python
class Clock(Protocol):
    def now(self) -> datetime: ...
class TradingCalendar(Protocol):                    # one instance per process, built from the stored calendar table
    def sessions(self, start: date, end: date) -> list[date]: ...
    def open_close(self, session: date) -> tuple[datetime, datetime]: ...
    def next_session(self, session: date) -> date: ...
```

Daily cycle, first version (all times ET):

| Time | Simulation | Paper trading |
| --- | --- | --- |
| Session T close, 16:00 | `BarClosed` for T | - |
| `decision_time` (default 19:05, after Alpaca's 7:00pm auction queue reopens) | `Decision(as_of = T 19:05)`: only data with `available_at <= as_of` is visible | `atp paper decide` ingests T's data, runs the same pipeline, submits orders for T+1 |
| Session T+1 open, 09:30 | Orders fill at T+1's open price plus modelled cost | Orders execute in the opening auction (`opg`) or at the open (`day`) |
| T+1, default 10:00 and again after the close | - | `atp paper reconcile` pulls fills and positions, updates the shadow simulation, writes the tracking report |

Deciding at the close and filling at the next open is the convention of *LLM Trading Agents Earn Beta* p4 and keeps the decision strictly before the fill.

### 5.3 Data (B)

```python
class DataView(Protocol):                           # the only data handle a strategy ever receives
    as_of: datetime
    def bars(self, assets: Sequence[AssetId], field: str, lookback: int,
             frequency: Frequency = "1Day", adjustment: str = "all") -> pd.DataFrame: ...   # index: session, columns: asset
    def universe(self, name: str) -> list[AssetId]: ...
    def asset_flags(self, assets: Sequence[AssetId]) -> pd.DataFrame: ...    # tradable, shortable, easy_to_borrow, fractionable
    def news(self, assets: Sequence[AssetId], since: datetime) -> list[NewsItem]: ...
    def corporate_actions(self, assets: Sequence[AssetId], since: date) -> pd.DataFrame: ...

class DataStore(Protocol):
    def view(self, as_of: datetime) -> DataView: ...
    def append(self, dataset: str, records: pd.DataFrame, ingested_at: datetime) -> int: ...  # append-only
    def snapshot_id(self) -> str: ...               # content hash recorded with every run

class DataSource(Protocol):                         # Alpaca is the first implementation
    def fetch_bars(self, symbols, start, end, frequency, adjustment, feed) -> pd.DataFrame: ...
    def fetch_corporate_actions(self, symbols, start, end) -> pd.DataFrame: ...
    def fetch_assets(self) -> pd.DataFrame: ...
    def fetch_calendar(self, start, end) -> pd.DataFrame: ...
    def fetch_news(self, symbols, start, end) -> pd.DataFrame: ...
```

Contract: `DataView` never returns a record whose `available_at` is later than `as_of`. Every record has three times: the period it describes, `available_at` (when it could have been known; for backfilled history this is modelled, for example session close plus a configured lag, and for news it is `created_at`), and `ingested_at` (when this system fetched it).

Datasets (Parquet, under `$ATP_HOME/data/`, partitioned as shown):

| Dataset | Key | Columns beyond the key | Partition |
| --- | --- | --- | --- |
| `bars` | asset, session, frequency, adjustment | open, high, low, close, volume, vwap, trade_count, available_at, ingested_at | frequency, adjustment, year |
| `corporate_actions` | asset, type, ex_date | ratio or cash amount, record_date, payable_date, process_date, available_at, ingested_at | year |
| `assets` | asset, valid_from | symbol, name, exchange, status, tradable, shortable, easy_to_borrow, fractionable, valid_to | none |
| `calendar` | session | open, close | none |
| `news` | news_id | assets, headline, summary, source, created_at, updated_at, available_at, ingested_at | year, month |
| `universe` | name, session | asset, rank, reason | name, year |

Raw and fully adjusted daily bars are both stored. Returns come from adjusted prices (*Multi-Period Trading* p17); share counts, price filters and order sizes come from raw prices. The default universe is rebuilt each month from data visible at that date: the 500 names with the highest trailing 60-session median dollar volume among assets with a raw close of at least $5 and at least 252 sessions of history, plus a configured ETF list whose members enter only once they have 252 sessions of history. It includes inactive assets wherever Alpaca returns their history.

### 5.4 Signals (C, I)

```python
@dataclass(frozen=True)
class SignalSpec:
    name: str; version: str; params: Mapping[str, object]
    horizon_sessions: int; lookback_sessions: int
    origin: Literal["published", "in_house", "pretrained_model"]
    knowledge_cutoff: date | None      # required when origin == "pretrained_model"

class Signal(Protocol):
    spec: SignalSpec
    def compute(self, view: DataView, universe: Sequence[AssetId]) -> pd.Series: ...   # score per asset; NaN = no opinion

class Predictor(Protocol):             # trainable model wrapped as a Signal
    def fit(self, view: DataView, train_end: date, validation_sessions: int) -> "ModelArtifact": ...
    def predict(self, view: DataView, artifact: "ModelArtifact", universe: Sequence[AssetId]) -> pd.Series: ...
    # ModelArtifact records train_end; predictions for sessions <= train_end are never used in a backtest

class Calibrator(Protocol):
    def forecast(self, view: DataView, scores: Mapping[str, pd.Series], specs: Sequence[SignalSpec]) -> Forecast: ...
```

A missing or failed signal yields NaN scores; the calibrator drops it and renormalises. A signal failure never blocks the daily job.

Shipped signals: `tsmom` (12-month lookback, volatility-scaled, *Time Series Momentum* p13, p18); `resid_reversion` (60-session regression on a sector ETF, s-score entry and exit bands, *Statistical Arbitrage* p4, p45-46); `pv_characteristics` (momentum, liquidity and volatility families, *Empirical Asset Pricing via ML* p2); predictors `elastic_net_huber` and `gbt` trained on those characteristics with the temporal scheme of *Empirical Asset Pricing via ML* p10, p26.

### 5.5 Risk and cost models (D)

```python
@dataclass(frozen=True)
class RiskEstimate:
    assets: list[AssetId]; sigma: pd.Series
    factor_loadings: pd.DataFrame | None; factor_cov: pd.DataFrame | None; idio_var: pd.Series | None
    cov: pd.DataFrame | None           # full matrix when no factor structure is used

class RiskModel(Protocol):
    def estimate(self, view: DataView, universe: Sequence[AssetId]) -> RiskEstimate: ...

@dataclass(frozen=True)
class CostParams:                      # all rates are fractions, never percent or bps
    half_spread: pd.Series; sigma: pd.Series; adv_dollars: pd.Series
    impact_coeff: float; short_fee: pd.Series; cash_borrow_rate: float

class CostModel(Protocol):
    def params(self, view: DataView, universe: Sequence[AssetId]) -> CostParams: ...
    def trade_cost(self, trade_dollars: pd.Series, p: CostParams) -> pd.Series: ...          # numpy path, used by F
    def trade_cost_expr(self, z: "cvxpy.Expression", nav: float, p: CostParams) -> "cvxpy.Expression": ...  # used by E
    def holding_cost(self, holdings_dollars: pd.Series, cash: float, p: CostParams) -> float: ...
```

`trade_cost` is `half_spread*|x| + impact_coeff*sigma*|x|^(3/2)/adv_dollars^(1/2)` (*Multi-Period Trading* p12, eq. 2.2). The optimizer calls it with forecast parameters at decision time; the simulator calls the same function with the realized parameters of the fill session (*Markowitz at Seventy* p29).

### 5.6 Portfolio construction (E)

```python
@dataclass(frozen=True)
class Constraints:
    long_only: bool; w_min: float; w_max: float; cash_min: float; cash_max: float
    leverage_max: float; turnover_max: float; participation_max: float
    risk_target_annual: float | None; restricted: frozenset[AssetId]
    soft: Mapping[str, float]          # constraint name -> priority; absent = hard

@dataclass(frozen=True)
class AllocationInput:
    as_of: datetime; nav: float; w_pre: pd.Series; cash_pre: float
    forecast: Forecast | None; risk: RiskEstimate; cost: CostParams; constraints: Constraints
    gamma_trade: float; gamma_hold: float; rho_scale: float; cov_uncertainty: float

class Allocator(Protocol):
    def allocate(self, x: AllocationInput) -> TargetPortfolio: ...
```

`MarkowitzAllocator` solves problem (19) of *Markowitz at Seventy* p24 in its softened form (p25-27). If the solver fails or the problem is infeasible after softening, it returns the pre-trade weights with `status="fallback_hold"` and the run raises an alert; it never raises into the order path.

### 5.7 Broker and execution (A defines `Broker`; F and G implement it)

```python
class Broker(Protocol):
    def account(self) -> AccountSnapshot: ...
    def positions(self) -> list[Position]: ...
    def submit(self, order: OrderRequest) -> "OrderState": ...      # idempotent on client_order_id
    def cancel(self, client_order_id: str) -> None: ...
    def orders(self, session: date) -> list["OrderState"]: ...
    def fills(self, since: datetime) -> list[Fill]: ...

class OrderPlanner(Protocol):          # G
    def plan(self, target: TargetPortfolio, account: AccountSnapshot, positions: Sequence[Position],
             prices: pd.Series, flags: pd.DataFrame, schedule: "ExecutionSchedule") -> list[OrderRequest]: ...

class ExecutionSchedule(Protocol):     # G; first version: SingleAuction("opg" | "cls") and DayMarket
    def slices(self, asset: AssetId, qty: Decimal, session: date) -> list[tuple[datetime, Decimal, str]]: ...

class RiskGate(Protocol):              # G; pure and deterministic
    def check(self, orders: Sequence[OrderRequest], account: AccountSnapshot, positions: Sequence[Position],
              prices: pd.Series, limits: "RiskLimits", state: "RiskState") -> "GateResult": ...
    # GateResult: approved orders, rejected orders with reason codes, kill: bool
```

`RiskLimits` fields: `max_order_notional`, `max_position_weight`, `max_gross_leverage`, `max_net_leverage`, `max_daily_turnover`, `max_orders`, `price_collar_pct`, `max_daily_loss_pct`, `max_drawdown_pct`, `restricted`. The gate also rejects orders for assets that are not tradable, short sales of assets that are not easy to borrow, and anything submitted while the kill switch is on. The kill switch is a row in the state database set by the gate or by `atp kill on`; clearing it is a manual command.

```python
def reconcile(session: date, broker: Broker, store: DataStore, run_id: str) -> "ReconciliationReport": ...   # G
```

`reconcile` compares broker positions and cash with the recorded state, then replays the broker's actual fills through the simulator's ledger on split-adjusted prices without dividends (paper trading pays none). The report gives the per-session return difference, the cumulative divergence, each fill price against the session open, and any break it cannot explain.

The planner emits sells before buys (*Implementation Risk* p33), rounds to whole shares for auction orders, and scales buys down when account buying power would not cover them. Before submitting, the adapter looks up each `client_order_id` so a re-run of a crashed job does not duplicate orders.

### 5.8 Engine and simulation (F)

```python
class Strategy(Protocol):
    name: str
    def decide(self, view: DataView, account: AccountSnapshot, positions: Sequence[Position]) -> TargetPortfolio: ...

class Engine:
    def __init__(self, clock: Clock, calendar: TradingCalendar, store: DataStore, broker: Broker,
                 strategy: Strategy, planner: OrderPlanner, gate: RiskGate, recorder: "RunRecorder"): ...
    def run(self, start: date, end: date) -> "RunResult": ...       # simulation
    def step(self, session: date) -> "StepResult": ...              # one decision cycle; used by paper trading and tests

def reference_backtest(weights: pd.DataFrame, prices: pd.DataFrame, capital: float, cost_rate: float) -> pd.Series: ...
    # minimal proportional-cost loop after Implementation Risk Algorithm 1 (p43-45); test oracle only
```

The standard `Strategy` is `PipelineStrategy`, owned by J in `src/atp/ops/pipeline.py`: it builds signals, calibrator, risk model, cost model and allocator from a configuration file and calls them in that order.

`RunResult` holds the run id, the equity series, per-session returns gross and net, turnover, costs by component, holdings, orders and fills, and the count of sessions processed against sessions in the calendar. The ledger works in dollar holdings and total returns (*Multi-Period Trading* p17-18); whole-share rounding uses raw prices.

### 5.9 Validation (H)

```python
class TrialRegistry(Protocol):         # append-only
    def open(self, family: str, config_hash: str, code_rev: str, data_snapshot: str, seed: int | None) -> str: ...  # called before a run starts
    def close(self, trial_id: str, returns: pd.Series, metrics: Mapping[str, float], status: str) -> None: ...
    def family_matrix(self, family: str) -> pd.DataFrame: ...       # sessions x trials net returns, for CSCV

def probabilistic_sharpe(returns: pd.Series, sr_benchmark: float) -> float: ...
def deflated_sharpe(returns: pd.Series, n_effective: float, sr_variance: float) -> float: ...   # DSR eq. 1-2
def effective_trials(returns_matrix: pd.DataFrame) -> float: ...                                 # DSR eq. 9
def pbo_cscv(returns_matrix: pd.DataFrame, n_blocks: int = 16) -> "PBOResult": ...               # PBO Algorithm 2.3; result: pbo, logits, degradation slope, probability of loss
def adjust_pvalues(p: pd.Series, method: Literal["bonferroni", "holm", "bhy"]) -> pd.Series: ...
def attribute(returns: pd.Series, factor_returns: pd.DataFrame) -> "Attribution": ...            # alpha, t-stat, betas, R^2
def leak_check(strategy: Strategy, store: DataStore, decision_days: Sequence[date], seed: int) -> "LeakReport": ...
def evaluate_gates(trial_id: str, gates: "GateConfig") -> "GateReport": ...
```

Factor returns for `attribute` default to proxies built from ETF returns in the same store (market, size, value, momentum) and are configurable; the choice of proxies is engineering judgement, since the references name a four-factor regression without a data source this platform can reach.

`leak_check` re-runs the decision on copies of the data in which everything after the decision time is replaced by random values; any change in the decision is a leak (*LLM Trading Agents Earn Beta* p4).

Promotion states of a strategy: `research`, `candidate` (passed gates 1 to 6), `paper` (running against Alpaca), `validated` (passed gate 7). There is no `live` state.

| Gate | Passes when | Basis |
| --- | --- | --- |
| 1 Integrity | Leak check clean on sampled decision days; data checks clean; sessions processed equals sessions in the calendar | *LLM Trading Agents Earn Beta* p2, p4; *Implementation Risk* p34 |
| 2 Baselines | Net Sharpe ratio above every baseline named in the strategy's configuration before the run (at least equal-weight, inverse-volatility, time-series momentum) | *Seemingly Virtuous Complexity* p2; *LLM Trading Agents Earn Beta* p2; *Building Diversified Portfolios* p4 |
| 3 Significance | DSR at least 0.95 using all registered trials of the family; for a new signal, t-statistic at least 3.0 | *Deflated Sharpe Ratio* p8-10; *And the Cross-Section* p2 |
| 4 Selection | PBO at most 0.05 over the family's trials; performance degradation and probability of loss reported alongside | *Probability of Backtest Overfitting* p11-15 |
| 5 Attribution | Alpha after market and style factors has t-statistic at least the configured limit (proposed 2.0); exposures reported | *LLM Trading Agents Earn Beta* p1, p6 |
| 6 Cost robustness | Net Sharpe stays positive at twice the modelled cost, and its sign survives a 20% change of each tuned parameter | *Markowitz at Seventy* p29-30; *Multi-Period Trading* p21 |
| 7 Paper | At least the configured number of paper sessions (proposed 60); shadow-simulation tracking difference within tolerance; no unexplained reconciliation breaks | *Multi-Period Trading* p21; *Implementation Risk* p4 |

For any signal with `origin="pretrained_model"`, gates 3 to 5 use only sessions after `knowledge_cutoff`, and the report must add: dispersion across at least the configured number of seeds, the with-and-without-text ablation, and performance before versus after the cutoff (*Assessing Look-Ahead Bias* p2; *LLM Trading Agents Earn Beta* p1, p4, p8; *Look-Ahead-Bench* p2). PBO is reported, never optimized (*Probability of Backtest Overfitting* p26).

### 5.10 LLM signal plugin (I)

```python
class LLMProvider(Protocol):
    model_id: str
    def complete(self, system: str, user: str, schema: Mapping[str, object], seed: int) -> Mapping[str, object]: ...

@dataclass(frozen=True)
class ModelCard: model_id: str; vendor: str; knowledge_cutoff: date; cutoff_source: str

class Anonymizer(Protocol):
    def anonymize(self, item: NewsItem, session: date) -> str: ...   # tickers, names, products -> neutral ids

class LLMNewsSignal(Signal): ...       # spec.origin = "pretrained_model"; refuses to construct without a ModelCard
```

Output schema per item: `{"score": -1 | 0 | 1, "confidence": number in [0, 1]}`; anything else is discarded. Responses are cached under the hash of model id, prompt version, anonymized text and seed. The plugin runs as a batch step before `decision_time`, reads only news with `available_at <= as_of`, and has no access to the broker. Ablation modes `none`, `no_news`, `swapped_news`, `calendar_date_join` exist for validation only.

### 5.11 Command line (J)

```
atp data ingest     --dataset {bars,corporate-actions,assets,calendar,news} [--start D] [--end D] [--symbols S,..] [--feed {sip,iex}]
atp data check      [--start D] [--end D]
atp universe build  --config PATH [--start D] [--end D]
atp backtest run    --config PATH [--start D] [--end D] [--family NAME] [--synthetic SEED] [--cost-scale X] [--out DIR]
atp validate        --family NAME [--select TRIAL_ID] [--gates PATH] [--report PATH]
atp trials {list,show} [--family NAME] [--trial TRIAL_ID]
atp paper decide    --config PATH [--session D] [--dry-run]
atp paper reconcile [--session D]
atp paper status
atp kill {on,off,status} [--flatten] [--reason TEXT]
atp report {run,paper} --id ID [--out PATH]
```

Exit codes: 0 success, 1 failure, 2 usage error, 3 a gate failed, 4 blocked by the risk gate or kill switch. Every `backtest run` opens a trial in the registry before it starts, so abandoned runs are counted.

### 5.12 Configuration and state (J)

Strategy configuration is TOML under `configs/`, with sections `[strategy]`, `[universe]`, `[[signals]]`, `[calibrator]`, `[risk_model]`, `[cost_model]`, `[allocator]`, `[constraints]`, `[execution]`, `[risk_limits]`, `[gates]`, `[llm]`. Secrets come only from the environment: `APCA_API_KEY_ID`, `APCA_API_SECRET_KEY`, and the provider key if the plugin is enabled. `ATP_HOME` (default `./var`, git-ignored) holds everything mutable:

```
$ATP_HOME/data/        Parquet datasets of section 5.3
$ATP_HOME/models/      predictor artifacts, one directory per predictor and train_end
$ATP_HOME/runs/<id>/   config snapshot, decisions, orders, fills, equity, metrics
$ATP_HOME/llm_cache/   cached model responses
$ATP_HOME/state.sqlite tables: trials, runs, orders, fills, position_snapshots, kill_switch, model_cards, job_log
$ATP_HOME/logs/        structured logs, one file per job run
```

## 6. Decisions

| # | Decision | Alternative rejected | Basis |
| --- | --- | --- | --- |
| D1 | Python package `atp` in a `src/` layout, tested with `pytest`, linted with `ruff`. | Another language or a multi-language split. | The reference tooling is Python (*Qlib* p2 calls it the mainstream scientific language, the library announced in *Multi-Period Trading* p8, every engine in *Implementation Risk* p45), Alpaca's official SDK is Python, and `.devteam/verify.sh` already detects `pyproject.toml`, `ruff` and `pytest`. |
| D2 | Build a small simulator in the repository instead of adopting an open-source backtesting engine. | Depend on an existing engine. | *Implementation Risk* found seven defects in three engines, including a silent hundredfold cost undercharge (p1, p32-34). A small engine with a single owner of cost accounting, checked against a reference loop, is easier to trust and must share code with the paper-trading path anyway. |
| D3 | One event-driven pipeline for simulation and paper trading; only `Clock` and `Broker` differ. | A vectorized backtester plus a separate live trading script. | *Qlib* p4 (the executor is a responsive simulator). Two code paths are the "specification divergence" failure of *Implementation Risk* p13. The vectorized loop survives only as a test oracle. |
| D4 | Decide after the close of session T, fill at the open of T+1; default order style is a whole-share market-on-open (`opg`) order. | Decide and fill at the same close; or fractional `day` orders as the default. | *LLM Trading Agents Earn Beta* p4; *Can ChatGPT Forecast* p20 trades the auctions. The simulator's fill price (the session open) then describes the same event as the paper order. Alpaca forbids fractional auction orders, so fractional `day` orders remain a configuration choice (Q5). |
| D5 | Point-in-time store: every record has `available_at`; strategies see data only through `DataView(as_of)`; news is admitted by publication time, not calendar date. | Trust each signal to lag its own inputs. | *LLM Trading Agents Earn Beta* p1 (one news rule earned +32.0 bps per trade when headlines were joined by calendar date and -6.7 bps when replayed by publication time and traded from the next open); *Qlib* p4 (append-only data, time-varying pools); *Can Blindfolded LLMs* p2. |
| D6 | Parquet files for market data and one SQLite file for operational state. | A database server; or Qlib's custom binary format and expression engine. | *Qlib* p4 argues against general-purpose and time-series databases for research data and for append-only columnar files. At daily frequency the whole data set fits in memory, so a standard columnar format is enough, and a self-hosted single user should not run a server. An expression engine invites mass feature search, which D12 discourages. |
| D7 | Universe chosen point in time by trailing dollar volume, including inactive assets where Alpaca has them; curated ETF list with a history requirement. | A fixed list of today's index members. | *Statistical Arbitrage* p4 selects by size at the trade date; *Can Blindfolded LLMs* p2; the survivorship admissions in *Markowitz at Seventy* p31 and *Multi-Period Trading* p59. Dollar volume replaces market capitalization because no share-count or capitalization field appears in the Alpaca pages consulted (reason, not reference). |
| D8 | Forecasting is separate from portfolio construction, joined by the `Forecast` type, but every candidate is evaluated through the whole pipeline net of costs. | End-to-end learned policy; or judging signals on forecast accuracy. | *Markowitz at Seventy* p4; *Multi-Period Trading* p4, p26; *Financial Machine Learning* p130-131; *Re(Visiting) TSFMs* p2. See conflict C8. |
| D9 | Markowitz++ single-period optimization is the default allocator, with robust return and covariance terms, soft constraints and a trade-aversion multiplier; HRP, inverse-volatility and equal-weight share the interface. | Plain mean-variance on sample estimates; HRP only. | *Markowitz at Seventy* p22-27; *Multi-Period Trading* p26-29, p44. Conflict C1 stays open to evidence because the alternatives run in the same harness. |
| D10 | Single-period optimization first; the `Forecast.horizon_sessions` field leaves room for multi-period planning. | Multi-period optimization now. | *Multi-Period Trading* p48-52 needs forecasts at several horizons; *Financial Machine Learning* p131-132 explains why ML signals do not supply them cheaply. The round-trip effect is approximated by trade aversion (*Multi-Period Trading* p44, p48). |
| D11 | One cost model, 3/2-power impact with exponent fixed and coefficient configurable (default 1), used by both optimizer and simulator; rates typed as fractions. | Flat basis-point costs only; separate cost code in each component. | *Multi-Period Trading* p12-13; *Markowitz at Seventy* p17; *Strict Universality* p1; *Implementation Risk* p32 (unit confusion) and p34 (double deduction). |
| D12 | An append-only trial registry records every backtest before it runs; DSR and PBO are computed over all trials of a strategy family; correlated trials count as fewer independent ones. | Report the best backtest; or rely on a hold-out period. | *Deflated Sharpe Ratio* p3, p6-9, p14; *Probability of Backtest Overfitting* p8, p12-13, p25. See conflict C7. |
| D13 | Paper trading is the only sample treated as untouched, and it is reconciled daily against a shadow simulation driven by the actual fills, treating Alpaca's paper engine as a second, independent engine. | Treat a passed backtest as validation; or compare paper equity to the original backtest directly. | *Multi-Period Trading* p21 (simulate the executed trades and compare); *Implementation Risk* p2, p4 (multi-engine comparison). Like-for-like comparison is needed because paper trading omits dividends and impact (Alpaca documentation). |
| D14 | Deterministic pre-trade risk gate and kill switch between every order and the broker, in both modes, for every signal source. | Risk handled inside the optimizer only; or by an LLM "risk agent". | *Agentic Trading* p16, p26, p35. Optimizer constraints can be violated by realized values (*Multi-Period Trading* p28). |
| D15 | The LLM is an optional `Signal` that scores anonymized, point-in-time news offline, with a mandatory training cutoff, cached structured outputs and several seeds. It never places orders. | An autonomous or multi-agent LLM trader; LLM signals exempt from the gates. | The user's scoping decision; *LLM Trading Agents Earn Beta* p1-2, p6; *Assessing Look-Ahead Bias* p1-2; *Look-Ahead-Bench* p1-2; *TradingAgents* p2, p12; *R&D-Agent-Quant* p2. See conflicts C5 and C6. |
| D16 | Every model carries a knowledge cutoff: the last training label for models trained here, the vendor's training cutoff for pretrained ones. Observations at or before the cutoff are excluded from gate statistics. | Separate rules for ML and LLM models. | One rule covers *Empirical Asset Pricing via ML* p10, *Assessing Look-Ahead Bias* p2 and *Re(Visiting) TSFMs* p20. |
| D17 | Scores become expected returns through a walk-forward cross-sectional regression of realized forward returns on winsorized, standardized scores, with the cross-signal dispersion as the uncertainty `rho`. | Use magnitudes reported in papers; or raw scores as returns. | Gap 5: the references give the preprocessing (*Markowitz at Seventy* p22-23) but not the mapping, so this is engineering judgement. Calibrating on recent realized data respects the decay in *Does Academic Research* p1. |
| D18 | Machine-learning predictors are limited to penalized linear models and gradient-boosted trees on price and volume characteristics in the first version. | Deep networks; time-series foundation models; the full 94-characteristic set. | *Empirical Asset Pricing via ML* p5-6 (shallow beats deep, trees and penalized models work); gap 4 (no fundamentals); *Pretrained TSFMs* p1 and *Re(Visiting) TSFMs* p1 (weak evidence). Other models can be added behind `Predictor`. |
| D19 | Paper trading runs as two idempotent scheduled jobs per session (`decide`, `reconcile`) with state in SQLite and REST polling. | A long-running process on the streaming API. | Daily frequency does not need a daemon, and a job that can be re-run safely is the simplest thing to self-host (reason, not reference). The event types and `Engine.step` are the seam for a streaming, minute-bar runner later. |
| D20 | Long-only is the default; long-short is enabled per strategy and restricted to easy-to-borrow names. | Long-short by default. | Alpaca's margin and locate rules; Much of the measured edge sits in small, illiquid stocks (*Financial Machine Learning* p130) and on the short side after negative news (*Can ChatGPT Forecast* p3), which a retail account cannot rely on borrowing. See Q4. |
| D21 | The broker adapter is hard-wired to the paper endpoint. | A live/paper switch. | The goal is validation on paper; removing the switch removes a class of accident (reason, not reference). |
| D22 | Implementations are selected by dotted path in configuration; there is no central registry module. | A registry file that every new signal edits. | Keeps unrelated plan tasks from sharing a file (pipeline rule), and keeps the trial's configuration hash a complete description of what ran. |

## 7. Verification strategy

### 7.1 Per component

| Component | How it is proven |
| --- | --- |
| A Core | Type checking under `mypy --strict`. Property tests: calendar sessions are strictly increasing and identical however the calendar is obtained (*Implementation Risk* p33); the synthetic market generator is deterministic for a seed and can plant a known signal, a pure-noise panel and a reversal panel. |
| B Data | Property test: for random `as_of`, no record with later `available_at` is returned. Append-only test: re-ingestion never changes a stored value silently. Adjustment test: raw and adjusted series agree on returns away from corporate action dates. Checks for gaps, duplicate bars, zero volume and reused tickers. Ingestion tested against hand-written HTTP fixtures; no network in `verify`. |
| C Signals | Known-answer tests on synthetic panels (a planted trend gives positive `tsmom` scores; a planted mean-reverting residual gives the expected s-score sign, with the formulas of *Statistical Arbitrage* p45-46). Predictor tests: training never reads beyond `train_end`; on the pure-noise panel the predictor does not pass gate 3. |
| D Models | Covariance estimates are positive semidefinite and match a closed-form case. `trade_cost` and `trade_cost_expr` agree numerically for random inputs; cost is zero at zero trade and scales as the 3/2 power. Unit test that a rate entered as a fraction is never rescaled. |
| E Portfolio | The solution satisfies every hard constraint. With one asset pair and no constraints the optimizer matches the analytic mean-variance solution (*Markowitz at Seventy* p4). Raising trade aversion never raises turnover. HRP weights are non-negative, sum to one and work on a singular covariance matrix (*Building Diversified Portfolios* p2). Infeasible input returns `fallback_hold`. |
| F Engine | At zero cost the engine and `reference_backtest` produce the same equity series to 1e-9 relative error for every allocator baseline (*Implementation Risk* p41). Net return is non-increasing in the cost rate. The self-financing identity holds each session (*Multi-Period Trading* p17-18). Sessions processed equals sessions in the calendar. A synthetic one-minute calendar runs through the same engine without code changes outside F. |
| G Execution | Planner: sells precede buys; auction orders are whole shares; total buys never exceed buying power. Risk gate: one test per limit, plus kill-switch persistence. Adapter: contract tests against hand-written fixtures of Alpaca responses; a test that any non-paper base URL is refused; re-running `decide` submits nothing new. Opt-in tests marked `alpaca` run against a real paper account and are not part of `verify`. |
| H Validation | DSR reproduces the worked example of *Deflated Sharpe Ratio* p9-10 (about 0.9004 at 100 trials, 0.9505 at 46; inputs to be transcribed from the PDF). PBO is high for a family of strategies fitted to a random walk and low when one configuration carries a planted signal (the paper's own examples give 74% and 0.04%, *Probability of Backtest Overfitting* p17, p26-27). The leak check catches planted leaks of one day (*LLM Trading Agents Earn Beta* p2). Multiple-testing adjustments reproduce the table of *And the Cross-Section* p15. |
| I LLM plugin | A fake provider makes every test offline. Anonymizer removes every ticker and company name in a fixture set. Construction fails without a `ModelCard`. Sessions at or before the cutoff are flagged. Identical inputs hit the cache. Malformed model output is discarded. News published after `as_of` never reaches a prompt. |
| J Operations | End-to-end test on synthetic data: ingest, build universe, backtest, validate, then two `paper decide` and `reconcile` cycles against the simulated broker. A repository hygiene test fails if a data file, a key-like string or a file over 1 MB is tracked. |

### 7.2 What `.devteam/verify.sh` must run

In this order, offline and deterministic:

1. `ruff check .`
2. `ruff format --check .`
3. `mypy src/atp`
4. `pytest -q` with the default marker expression excluding `alpaca` and `slow`. This includes the end-to-end synthetic test and the hygiene test.
5. `atp backtest run --config configs/smoke.toml --synthetic 7` followed by `atp validate --family smoke`, as the smoke test of the installed command.

The script as it stands cannot do this. It runs `ruff` and `pytest` only if they happen to be on the path, exits with "not configured" otherwise, and the CI workflow has no toolchain setup step. Agents may not edit either file, so a human must change `.devteam/verify.sh` and `.github/workflows/verify.yml` (a `judge-change` pull request) to install Python and the project with its development dependencies and to run the five steps. Until then the first build tasks cannot go green in CI. This is question Q1.

Not in `verify`: anything needing network access, Alpaca keys, an LLM provider or real market data. Those are run by hand from the runbook, and their results are recorded in the paper-trading reports.

## 8. Open questions

- **Q1. Toolchain and the verification script.** Which Python version should CI pin (floor 3.11 from `cvxpy`; the development machine has 3.15, for which dependency wheels were not verified)? Will a human update `.devteam/verify.sh` and `.github/workflows/verify.yml` as described in section 7.2 before build tasks start?
- **Q2. Data plan and history.** Stay on Alpaca's free Basic plan (history from 2016, 15 minutes delayed, 200 calls per minute; consolidated SIP access for a paper-only account is unverified, see gap 10) or pay for Algo Trader Plus ($99 per month)? Is a sample that starts in 2016 acceptable for the gates, or should a second vendor with longer history be added behind `DataSource`?
- **Q3. Survivorship.** Is a liquidity-ranked universe built from whatever delisted history Alpaca returns acceptable, with the residual bias measured and reported, or must a survivorship-free data set be in place before any gate result counts?
- **Q4. Short selling.** Should the first version support long-short strategies at all (needed for the residual mean-reversion signal as published; limited at Alpaca to easy-to-borrow names with a margin account), or be long-only with shorting deferred?
- **Q5. Account size and order style.** What notional paper account should be assumed (Alpaca's default is $100,000)? Is whole-share market-on-open the right default, or should fractional `day` orders be the default so small accounts can hold many names?
- **Q6. Gate thresholds.** Confirm or replace the values: DSR at least 0.95, t at least 3.0 and PBO at most 0.05 (from the papers); alpha t-statistic at least 2.0, 60 paper sessions and the tracking tolerance (proposed here, not from the papers). Should replicated published signals face a lower hurdle than new ones (conflict C2)?
- **Q7. Conflict defaults.** `docs/reference/README.md` names no authoritative document. Confirm the defaults taken for conflicts C1 to C10 in section 2.3, in particular Markowitz++ over HRP as the default allocator and square-root over linear impact.
- **Q8. LLM provider and news.** If the plugin is enabled, should it call a locally hosted open-weights model or a hosted API, and with what budget? Is Alpaca's news feed the intended source? Is it acceptable that an LLM signal can in practice only be validated forward in paper trading, because the period after any model's training cutoff is short?
- **Q9. Features without fundamentals.** Is a price-and-volume-only feature set acceptable for the first version, or should a public characteristics data set (those named in *Financial Machine Learning* p36) be evaluated for licence and point-in-time correctness?
- **Q10. Risk limits.** What values should the pre-trade gate enforce: maximum position weight, gross leverage, daily turnover, daily loss and drawdown that trip the kill switch?
- **Q11. Host and schedule.** Which machine runs the two daily jobs (a laptop asleep at 19:05 ET misses the run), with which scheduler, and where should alerts go?
- **Q12. Second engine.** Should a second open-source engine be added as a development-only cross-check of the simulator, as *Implementation Risk* recommends, or is the in-repository reference loop plus the Alpaca paper comparison enough?
- **Q13. Trial discipline.** Confirm that every backtest must go through `atp backtest run` so that it is registered, and that the registry is never reset for a strategy family.

This stage needs human approval before the requirements stage starts.

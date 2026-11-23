# Paper Trader Source Brief

Source material for the agent team building a self-hosted systematic trading system, validated on Alpaca paper trading.

- Date: 2026-11-22
- Owner: Taylor Aishman
- Companion: the literature review "Algorithmic and Systematic Trading: A Literature Review for Builders" and the PDFs in `trading-papers/`.

## 0. What this document is

This brief holds three things only: the owner's decisions, findings from the literature, and facts about external systems. It contains no architecture, requirements, parameters, thresholds or task breakdown. Those are the team's to decide.

Each fact carries a confidence label, explained in section 8. Check anything not marked OPENED against its primary source before depending on it.

## 1. Owner decisions

These are fixed. Do not reopen them without asking the owner.

| Topic | Decision |
| --- | --- |
| Language | Rust. Minimize Python as far as possible. |
| Engine | The team writes its own engine. |
| Paper venue | Alpaca paper trading. The account already exists. |
| Database | SQLite. |
| First strategy | Time-series momentum on a wide set of instruments. |
| Delivery | An extensive roadmap with a phased rollout. No one-shot build. |
| Task grain | Work broken down as finely as possible. |
| Owner steps | The owner handles everything that needs a human. The team says clearly at which step each is needed and isolates those steps from other work. |
| Live trading | Out of scope. |

Everything else is open: universe, parameters, data plan, order handling, risk limits, acceptance thresholds, crate choices, crate layout, interfaces, phases and tickets.

## 2. Time-series momentum: what the literature says

| Source | Finding |
| --- | --- |
| Moskowitz, Ooi and Pedersen (2012), "Time Series Momentum", Journal of Financial Economics 104(2) | An instrument's own past 12-month excess return predicts its next-month return across 58 liquid futures and forwards. Positions go long on a positive trend and short on a negative one, and are scaled by ex ante volatility. |
| Same paper, method detail | Ex ante volatility is an exponentially weighted average of squared daily returns with a 60-day center of mass, annualized with 261. Position size targets 40% annualized volatility per instrument; the authors say the choice of 40% does not matter for most reported statistics. |
| Hurst, Ooi and Pedersen (2017), "A Century of Evidence on Trend-Following Investing", Journal of Portfolio Management 44(1) | An equal-weighted blend of 1-month, 3-month and 12-month time-series momentum across 67 markets in four asset classes, 1880 to 2016. It performed well in eight of the ten largest drawdowns of a 60/40 stock and bond portfolio. |
| Lemperiere, Deremble, Seager, Potters and Bouchaud (2014), "Two Centuries of Trend Following", arXiv:1404.3274 | Trend excess returns have a t-statistic near 5 since 1960 and near 10 since 1800 across commodities, currencies, stock indices and bonds. Long trends show no statistical degradation in the recent period; shorter trends have weakened. |
| Kim, Tse and Wald (2016), "Time Series Momentum and Volatility Scaling", Journal of Financial Markets 30 | The Moskowitz results are largely driven by volatility scaling. Without it, time-series momentum and buy-and-hold have similar cumulative returns and alphas that do not differ significantly. |
| Huang, Li, Wang and Zhou (2020), "Time Series Momentum: Is It There?", Journal of Financial Economics 135(3) | Asset-by-asset regressions show little evidence of predictability in or out of sample. The strategy is profitable, but performs about the same as one based on the historical mean return that needs no predictability. |
| Moreira and Muir (2017), "Volatility-Managed Portfolios", Journal of Finance 72(4) | Scaling exposure down when recent variance is high raises Sharpe ratios. |
| Faber (2007, updated 2013), "A Quantitative Approach to Tactical Asset Allocation" | A related long-or-cash trend rule: hold each of five asset classes only above its 10-month moving average. About three to four round trips a year. Shows the idea working with ETFs-style asset-class exposures instead of futures. |

Points the team should weigh:

- The original studies use futures and forwards, long and short. An ETF implementation is an adaptation, not a replication.
- Two critiques attribute most of the measured advantage to volatility scaling, not to the trend signal. A volatility-scaled buy-and-hold comparison is the test that separates the two.
- AQR publishes the original paper's factor returns, monthly from January 1985 to December 2009, as a replication target.

## 3. Methodology findings that bear on the build

| Source | Finding |
| --- | --- |
| Yin, Miki, Lesnichenko and Gural (2026), "Implementation Risk in Portfolio Backtesting", arXiv:2603.20319 | Fifteen strategies run through five open-source backtest engines. With zero transaction costs all engines matched exactly. With costs they diverged, staying under 0.75 percentage points for most strategies and reaching 3.71% for high-turnover rotation strategies. Seven previously undocumented defects were found across three engines. All engines agreed on the sign of every metric. |
| McLean and Pontiff (2016), Journal of Finance 71(1) | Across 97 predictors, returns were 26% lower out of sample and 58% lower after publication. |
| Harvey, Liu and Zhu (2016), Review of Financial Studies 29(1) | With hundreds of factors tried, most claimed findings are likely false. A new factor needs a t-statistic near 3, not 2. |
| Bailey and Lopez de Prado (2014), Journal of Portfolio Management | The Deflated Sharpe Ratio corrects a Sharpe ratio for the number of trials and for non-normal returns. |
| Bailey, Borwein, Lopez de Prado and Zhu (2017), Journal of Computational Finance | Probability of Backtest Overfitting, estimated by combinatorially symmetric cross-validation. |
| DeMiguel, Garlappi and Uppal (2009), Review of Financial Studies 22(5) | Equal weighting is hard to beat out of sample. |
| Almgren and Chriss (2000), Journal of Risk; Sato and Kanazawa (2025), Physical Review Letters | Execution cost models; price impact scales with the square root of order size. |
| Wang (2026), "LLM Trading Agents Earn Beta, Not Alpha", arXiv:2611.19530 | In 91 runs of six models, market and style exposure explained a median 78% of returns and no alpha was significant. Same-model runs differing only in random seed ended 5.2 points apart on average. |

One statistical fact about validation by paper trading: a t-statistic is roughly the Sharpe ratio times the square root of years. A strategy with a true Sharpe ratio of 1 needs about four years of returns to reach a t-statistic of 2. A paper run of a few months can validate a system. It cannot validate a strategy.

## 4. Alpaca: facts

### 4.1 Paper trading behaviour

From Alpaca's "Paper Trading" docs page, opened 2026-11-22.

| Behaviour | Detail |
| --- | --- |
| Nature | Real-time simulation. Same as live end to end except orders are not routed to an exchange. |
| Fills | Simulated from real-time quotes. Orders fill only when marketable, against the NBBO. |
| Size | Order size is not checked against quoted size, so an order can fill far beyond real liquidity. |
| Partial fills | An eligible order receives a partial fill of random size 10% of the time; the remainder is re-evaluated. |
| Not simulated | Market impact, information leakage, latency slippage, queue position for non-marketable limit orders, price improvement, regulatory fees, dividends. |
| Borrow fees | Not charged on paper; listed as coming soon. Live accounts have them. |
| Available on paper | Margin trading, short selling, premarket and after-hours trading. |
| Starting balance | $100k by default. Accounts can be created and deleted from the dashboard. |
| Keys and endpoint | Paper has its own API keys. Base URL `https://paper-api.alpaca.markets`. The API spec is the same as live. |
| Paper-only accounts | Receive and use only IEX market data. |

From Alpaca staff replies on the community forum (LISTING):

- Paper and live fills depend on the latest quote from full-market SIP data, regardless of the account's market data plan.
- Paper deliberately injects more partial fills than live trading produces, so algorithms see them.
- A market-on-close order on paper can be partly filled and then expire because of those random partial fills.

### 4.2 Market data plans

From Alpaca's "About Market Data API" docs page, opened 2026-11-22.

| | Basic | Algo Trader Plus |
| --- | --- | --- |
| Price | Free | $99 per month |
| Real-time equities coverage | IEX only | All US stock exchanges |
| Websocket symbols | 30 | Unlimited |
| Historical data | Since 2016 | Since 2016 |
| Historical limitation | Excludes the latest 15 minutes | None |
| Historical API calls | 200 per minute | 10,000 per minute |

The page does not describe feed parameters or split and dividend adjustment for bars.

History since 2016 is about ten years. That is short for judging a strategy with a 12-month lookback.

### 4.3 API details

| Item | Value | Confidence |
| --- | --- | --- |
| Market data base URL | `https://data.alpaca.markets/v2/...` | OPENED (example on the docs page) |
| Submit order | `POST /v2/orders` | LISTING |
| Client order ID | `client_order_id`, string, max length 128; orders can be fetched by it | LISTING |
| Order and fill stream | `wss://paper-api.alpaca.markets/stream`, stream name `trade_updates`; covers fills, partial fills, cancels and rejects | LISTING |
| Stream framing | The paper trading stream uses binary frames; the data stream uses text frames | LISTING |
| Auth headers | `APCA-API-KEY-ID`, `APCA-API-SECRET-KEY` | LISTING (seen in the `apca` crate source) |
| Fractional shares | Older docs say fractional quantities work only with market orders and day time-in-force | LISTING, may be outdated |
| Docs index for agents | `https://docs.alpaca.markets/llms.txt` | LISTING |

## 5. Rust ecosystem: facts

| Fact | Confidence |
| --- | --- |
| Alpaca publishes no official Rust SDK. Community crates on docs.rs include `apca`, `alpaca-trade`, `alpacars`, `rpaca` and `alpaca_api_client`. | LISTING |
| `apca` is async and licensed GPL-3.0-or-later. | LISTING |
| `clarabel` is a Rust-native interior-point solver for linear, quadratic and second-order cone programs, Apache-2.0. It is the default solver in CVXPY for linear and second-order cone programs. | LISTING |
| `polars` supports lazy queries over Parquet and larger-than-memory streaming from Rust. | LISTING |
| NautilusTrader's Rust crates run strategies, backtests and live trading without Python, with no Alpaca adapter; its docs say the Rust API is still changing. LGPL-3.0. | OPENED |
| Barter is an MIT-licensed Rust engine for live, paper and backtest; its market data integrations are crypto exchanges and its license text says it is not intended for production use. | LISTING |
| `hftbacktest` is a Rust backtester that models latency and queue position from full order-book data. | LISTING |
| `ort` wraps ONNX Runtime for model inference in Rust and is still on 2.0 release candidates. | LISTING |

## 6. Owner-only steps

The owner performs these. The team's plan should place each at a specific step and keep other work from blocking on it.

- Creating the repository and its access rules.
- Generating Alpaca paper API keys and placing them where the system reads them.
- Choosing or changing the Alpaca market data plan.
- Choosing the machine the system runs on and where alerts go.
- Approving the first order sent to the paper account.
- Reviewing validation results and signing off.
- Approving any new strategy before it trades on paper.
- Anything involving a live account. Agents never hold live credentials.

## 7. Things to verify

1. Which ETFs are tradable on Alpaca and have history back to 2016.
2. The bars endpoint parameters for feed selection and for split and dividend adjustment.
3. Whether fractional shares are still limited to market day orders.
4. The websocket framing and message schema for `trade_updates`.
5. Current versions and licenses of every crate considered.
6. Current Alpaca data plan terms, which change.

## 8. Sources

Confidence key: OPENED means the page was fetched and read. LISTING means it was confirmed through a search listing or abstract only. MEMORY means it is cited from memory and should be checked.

### Strategy

| Source | Link | Confidence |
| --- | --- | --- |
| Moskowitz, Ooi, Pedersen (2012), Time Series Momentum | https://papers.ssrn.com/abstract=2089463 | LISTING |
| Hurst, Ooi, Pedersen (2017), A Century of Evidence on Trend-Following Investing | https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2993026 | LISTING |
| Lemperiere et al. (2014), Two Centuries of Trend Following | https://arxiv.org/abs/1404.3274 | LISTING |
| Kim, Tse, Wald (2016), Time Series Momentum and Volatility Scaling | https://doi.org/10.1016/j.finmar.2016.05.003 | LISTING |
| Huang, Li, Wang, Zhou (2020), Time Series Momentum: Is It There? | https://ink.library.smu.edu.sg/lkcsb_research/6521 | LISTING |
| Faber (2007, updated 2013), A Quantitative Approach to Tactical Asset Allocation | https://papers.ssrn.com/abstract=962461 | LISTING |
| Moreira, Muir (2017), Volatility-Managed Portfolios | Journal of Finance 72(4) | MEMORY |
| AQR time-series momentum factor data, 1985 to 2009 | https://www.aqr.com/Insights/Datasets/Time-Series-Momentum-Original-Paper-Data | LISTING |

### Methodology

| Source | Link | Confidence |
| --- | --- | --- |
| Yin, Miki, Lesnichenko, Gural (2026), Implementation Risk in Portfolio Backtesting | https://arxiv.org/abs/2603.20319 | OPENED |
| Wang (2026), LLM Trading Agents Earn Beta, Not Alpha | https://arxiv.org/abs/2611.19530 | OPENED |
| Harvey, Liu, Zhu (2016), ...and the Cross-Section of Expected Returns | https://www.nber.org/papers/w20592 | LISTING |
| McLean, Pontiff (2016), Does Academic Research Destroy Stock Return Predictability? | https://papers.ssrn.com/abstract=2156623 | LISTING |
| Bailey, Lopez de Prado (2014), The Deflated Sharpe Ratio | https://papers.ssrn.com/abstract=2460551 | LISTING |
| Bailey, Borwein, Lopez de Prado, Zhu (2017), The Probability of Backtest Overfitting | https://escholarship.org/uc/item/4w1110bb | LISTING |
| DeMiguel, Garlappi, Uppal (2009), Optimal Versus Naive Diversification | Review of Financial Studies 22(5) | MEMORY |
| Almgren, Chriss (2000), Optimal Execution of Portfolio Transactions | in `trading-papers/` | LISTING |
| Sato, Kanazawa (2025), Strict Universality of the Square-Root Law in Price Impact | https://arxiv.org/abs/2411.13965 | LISTING |

### Alpaca

| Source | Link | Confidence |
| --- | --- | --- |
| Paper Trading docs | https://docs.alpaca.markets/us/docs/paper-trading | OPENED |
| About Market Data API | https://docs.alpaca.markets/us/docs/about-market-data-api | OPENED |
| Working with /orders | https://docs.alpaca.markets/us/docs/working-with-orders | LISTING |
| Websocket Streaming | https://docs.alpaca.markets/us/docs/websocket-streaming | LISTING |
| Staff reply: fills use SIP quotes regardless of plan | https://forum.alpaca.markets/t/paper-trading-fill-delays-of-50-260-seconds-limit-orders-filled-minutes-after-price-crossed/18223 | LISTING |
| Staff reply: closing-auction orders and partial fills on paper | https://forum.alpaca.markets/t/paper-moc-orders-partially-fill-remainder-expires-same-setup-on-live-works/19155 | LISTING |

### Rust

| Source | Link | Confidence |
| --- | --- | --- |
| clarabel | https://docs.rs/crate/clarabel/latest | LISTING |
| polars | https://docs.rs/crate/polars | LISTING |
| apca | https://docs.rs/crate/apca/latest | LISTING |
| alpaca-trade | https://docs.rs/alpaca-trade | LISTING |
| NautilusTrader Rust docs | https://nautilustrader.io/docs/latest/concepts/rust/ | OPENED |
| Barter | https://docs.rs/crate/barter/latest | LISTING |
| hftbacktest | https://docs.rs/hftbacktest/ | LISTING |
| ort | https://docs.rs/crate/ort/latest | LISTING |

This brief is research guidance for a paper-trading system. It is not investment advice.

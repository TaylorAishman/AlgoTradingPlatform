# Algorithmic and Systematic Trading: A Literature Review for Builders

Nov 20, 2026 · @Taylor Aishman

## Summary

The literature supports a specific build: a small number of well-documented strategies, an event-driven engine whose strategy code is identical in backtest and paper trading, and more effort on cost modeling and overfitting control than on signal discovery. This review covers about 60 papers and 11 open books across theory, strategies, execution, portfolio construction, methodology, machine learning and current LLM research, followed by three implementation sections and a roadmap.

- Real edges are few, small and decay after publication. Returns to published predictors fell 58% after publication in the largest study, so plan around half of any reported Sharpe ratio.
- Trend following and a handful of cross-sectional factors are the best-evidenced starting strategies and the easiest to paper trade honestly.
- Backtest error comes mostly from cost handling and data leakage. A 2026 study found open-source engines agree exactly without costs and diverge by up to 3.71% with them.
- Machine learning helps combine signals; tree models on tabular features remain the benchmark. Claims for very large models are contested.
- LLM trading agents have not shown alpha under look-ahead-free testing as of November 2026. Their defensible uses are text features and research automation.
- Paper trading tests the system, not the strategy. Its pass condition is daily agreement with a shadow backtest.

If you read only ten items, read these in order:

1. Tidy Finance with Python (book)
2. Moskowitz, Ooi and Pedersen (2012), time-series momentum
3. McLean and Pontiff (2016), post-publication decay
4. Harvey, Liu and Zhu (2016), multiple testing
5. Bailey and Lopez de Prado (2014), Deflated Sharpe Ratio
6. Boyd et al. (2017), multi-period trading via convex optimization
7. Almgren and Chriss (2000), optimal execution
8. Hasbrouck, Securities Trading: Principles and Procedures (book)
9. Kelly and Xiu (2023), Financial Machine Learning
10. Wang (2026), LLM trading agents earn beta, not alpha

## Foundations: efficiency, factors and what an edge is

Every strategy is a bet against some form of market efficiency, so the first reading is about what efficiency does and does not rule out. Prices absorb most public information, but information is costly to gather, which leaves small, competed-for edges. Factor models then tell you how much of a backtest's return is a known premium and how much is new.

| Work | Contribution | Use in your build |
| --- | --- | --- |
| Fama (1970), "Efficient Capital Markets", Journal of Finance 25(2) | Defines weak, semi-strong and strong efficiency | The null hypothesis every signal has to beat |
| Grossman and Stiglitz (1980), American Economic Review 70(3) | Perfect efficiency is impossible when information costs money | Explains why edges exist and why they are small |
| Fama and French (1993), Journal of Financial Economics 33(1) | Three-factor model: market, size, value | Baseline regression for return attribution |
| Jegadeesh and Titman (1993), Journal of Finance 48(1) | Past 3 to 12 month winners keep beating losers | The first anomaly worth replicating end to end |
| Carhart (1997), Journal of Finance 52(1) | Adds momentum as a fourth factor | Standard four-factor attribution |
| Lo (2004), "The Adaptive Markets Hypothesis", Journal of Portfolio Management 30(5) | Edges appear, get crowded and decay as traders adapt | Reason to monitor live decay, not just backtests |
| Cont (2001), "Empirical properties of asset returns: stylized facts", Quantitative Finance 1(2) | Fat tails, volatility clustering, near-zero linear autocorrelation | Sanity checks for any simulator or risk model |

Free factor returns for these models are in the [Kenneth French data library](https://mba.tuck.dartmouth.edu/pages/faculty/ken.french/data_library.html). Matching its momentum series from raw prices is a good first test of a data pipeline.

## Seminal strategy families

Six families account for most published systematic returns, and each has one paper that defines it. Trend following and cross-sectional factors are the best first targets for a self-hosted system: they run on daily bars, trade liquid instruments, and their paper-trading fills are believable.

| Family | Defining paper | Core finding |
| --- | --- | --- |
| Trend (time-series momentum) | [Moskowitz, Ooi and Pedersen (2012)](https://papers.ssrn.com/abstract=2089463), Journal of Financial Economics | Across 58 liquid futures and forwards, an instrument's own past 12-month return predicts its next return; the effect partly reverses at longer horizons |
| Value and cross-sectional momentum | Asness, Moskowitz and Pedersen (2013), "Value and Momentum Everywhere", Journal of Finance 68(3) | Both premia show up across asset classes and are negatively correlated with each other, so they combine well |
| Carry | Koijen, Moskowitz, Pedersen and Vrugt (2018), "Carry", Journal of Financial Economics 127(2) | Carry predicts returns in equities, bonds, currencies and commodities |
| Pairs trading | Gatev, Goetzmann and Rouwenhorst (2006), Review of Financial Studies 19(3) | Distance-matched pairs earned excess returns, shrinking over the sample |
| Statistical arbitrage | [Avellaneda and Lee (2010)](https://math.nyu.edu/~avellane/AvellanedaLeeStatArb071008.pdf), Quantitative Finance | Mean reversion of residuals after removing PCA or sector-ETF factors; PCA strategies averaged a Sharpe ratio of 1.44 after costs over 1997 to 2007 |
| Low risk | Frazzini and Pedersen (2014), "Betting Against Beta", Journal of Financial Economics 111(1) | Leveraged low-beta assets beat high-beta assets on a risk-adjusted basis |
| Volatility management | Moreira and Muir (2017), "Volatility-Managed Portfolios", Journal of Finance 72(4) | Scaling exposure down when recent variance is high raises Sharpe ratios |

AQR publishes the [original time-series momentum factor data](https://www.aqr.com/Insights/Datasets/Time-Series-Momentum-Original-Paper-Data), monthly from January 1985 to December 2009, which gives you a target to replicate.

Published Sharpe ratios are ceilings, not forecasts. A [Stanford student replication](https://web.stanford.edu/class/msande448/2017/Final/Reports/gr5.pdf) of Avellaneda and Lee got a Sharpe ratio of 0.54 on its 2013 to 2016 out-of-sample period.

## Market microstructure and execution

Transaction costs decide whether a signal survives, and microstructure is the literature that prices them. At personal account size, spread and fees dominate and market impact is small. The impact papers still matter, because they tell you how capacity shrinks as size or turnover grows.

| Work | Contribution | Use in your build |
| --- | --- | --- |
| Kyle (1985), "Continuous Auctions and Insider Trading", Econometrica 53(6) | Price impact arises because order flow carries information | Why impact grows with order size |
| Glosten and Milgrom (1985), Journal of Financial Economics 14(1) | The bid-ask spread compensates dealers for trading against informed flow | Why you pay the spread and when it widens |
| [Almgren and Chriss (2000)](https://quantitativebrokers.com/s/Optimal-Execution-of-Portfolio-Transaction-_-AlmgrenChriss-1999.pdf), Journal of Risk | Optimal liquidation schedule trading off impact cost against volatility risk | Template for any order-slicing logic |
| [Avellaneda and Stoikov (2008)](https://math.nyu.edu/~avellane/HighFrequencyTrading.pdf), Quantitative Finance | Optimal bid and ask quotes for a dealer with inventory risk | Starting point for market-making research |
| Cont, Kukanov and Stoikov (2014), "The Price Impact of Order Book Events", Journal of Financial Econometrics 12(1) | Order flow imbalance explains short-horizon price moves almost linearly | The basic intraday feature |
| Garleanu and Pedersen (2013), "Dynamic Trading with Predictable Returns and Transaction Costs", Journal of Finance 68(6) | With costs, trade only part of the way toward the ideal portfolio | Rule for turnover control in rebalancing |
| [Sato and Kanazawa (2025)](https://arxiv.org/abs/2411.13965), Physical Review Letters | Impact scales with the square root of order size; the exponent is 1/2 within error for every liquid Tokyo stock over eight years of account-level data | A defensible functional form for the impact term in a cost model |

A workable cost model for backtests is half the quoted spread, plus fees, plus a square-root impact term that is near zero at small size. Paper brokers do not simulate queue position or impact, so limit-order and intraday strategies look better on paper than they will live.

For institutional detail on order types, auctions and US market structure, Hasbrouck's free [Securities Trading: Principles and Procedures](https://pages.stern.nyu.edu/~jhasbrou/STPP/STPPindex.html) is the best open text.

## Portfolio construction and risk

Sizing and risk control usually matter more to realized results than the signal, and the literature here is unusually open. The arc runs from Markowitz's optimizer, through decades of evidence that naive optimization is fragile, to modern convex formulations that handle costs and constraints directly.

| Work | Contribution | Use in your build |
| --- | --- | --- |
| Markowitz (1952), "Portfolio Selection", Journal of Finance 7(1) | Portfolio choice as a trade-off between expected return and variance | The base formulation everything else modifies |
| Kelly (1956), "A New Interpretation of Information Rate", Bell System Technical Journal 35(4) | Bet size that maximizes long-run growth | Upper bound on leverage; most practitioners run a fraction of it |
| Ledoit and Wolf (2004), "Honey, I Shrunk the Sample Covariance Matrix", Journal of Portfolio Management 30(4) | Shrinkage makes covariance estimates usable | Default covariance estimator |
| DeMiguel, Garlappi and Uppal (2009), "Optimal Versus Naive Diversification", Review of Financial Studies 22(5) | Equal weighting is hard to beat out of sample | The benchmark any optimizer must clear |
| [Lopez de Prado (2016)](https://papers.ssrn.com/abstract=2708678), Journal of Portfolio Management | Hierarchical Risk Parity: allocate down a correlation tree, no matrix inversion | Robust allocation across strategies or assets |
| [Boyd et al. (2017), "Multi-Period Trading via Convex Optimization"](https://arxiv.org/abs/1705.00109) | One framework for return, risk, transaction cost and holding cost, single and multi-period | The best open blueprint for a portfolio and simulation layer |
| [Boyd et al. (2024), "Markowitz Portfolio Construction at Seventy"](https://arxiv.org/abs/2401.05080) | Extends Markowitz with practical constraints and robustness to forecast error, still convex | Current reference design, with [code](https://github.com/cvxgrp/markowitz-reference) |

Four controls cover most of the risk layer for a first system: a portfolio volatility target, per-position and gross exposure caps, fractional Kelly sizing, and a drawdown limit that halts trading. Put them in code that sits between the strategy and the order manager, so no strategy can bypass them.

## Quantitative finance: derivatives pricing, volatility and risk measures

Classical quantitative finance matters to this build in three places: volatility forecasts drive position sizing, tail-risk measures set limits, and option prices open a further strategy family in the variance risk premium. Pricing theory itself is only needed once the system trades options.

| Area | Work | Contribution | Use in your build |
| --- | --- | --- | --- |
| Pricing | Black and Scholes (1973), Journal of Political Economy 81(3); Merton (1973), Bell Journal of Economics 4(1) | Option value by continuous hedging with the underlying; no expected return needed | Implied volatility and greeks for any option position |
| Pricing | Cox, Ross and Rubinstein (1979), Journal of Financial Economics 7(3) | Binomial tree; the discrete version of the same argument | Simplest pricer to implement and test against |
| Pricing | Heston (1993), Review of Financial Studies 6(2) | Stochastic volatility with a closed-form price | Standard model for the volatility smile |
| Pricing | Dupire (1994), "Pricing with a Smile", Risk 7(1) | Local volatility that fits every listed option price | Consistent pricing across strikes and expiries |
| Numerics | [Longstaff and Schwartz (2001)](https://www.escholarship.org/uc/item/43n1k4jb), Review of Financial Studies | American options by Monte Carlo with least-squares regression for the continuation value | Pricing early-exercise and path-dependent payoffs |
| Volatility | Engle (1982), Econometrica 50(4); Bollerslev (1986), Journal of Econometrics 31(3) | ARCH and GARCH: volatility clusters and is forecastable | Baseline daily volatility forecast |
| Volatility | Andersen, Bollerslev, Diebold and Labys (2003), Econometrica 71(2) | Realized volatility from intraday returns | Better volatility input than daily closes |
| Volatility | [Corsi (2009)](https://statmath.wu.ac.at/~hauser/LVs/FinEtricsQF/References/Corsi2009JFinEtrics_LMmodelRealizedVola.pdf), Journal of Financial Econometrics | HAR: regress volatility on its daily, weekly and monthly averages | The forecast to beat; three regressors, easy to run live |
| Volatility | [Gatheral, Jaisson and Rosenbaum (2014)](https://arxiv.org/abs/1410.3394), Quantitative Finance | Log-volatility behaves like fractional Brownian motion with Hurst exponent near 0.1; improves realized-volatility forecasts | Current research frontier in volatility modeling |
| Volatility | [Guyon and Lekeufack (2023)](https://papers.ssrn.com/abstract=4174589), Quantitative Finance | Past index returns explain up to 90% of implied volatility variance and up to 65% of future realized volatility | A simple, strong volatility feature built from prices alone |
| Risk premium | [Carr and Wu (2009)](https://econwpa.ub.uni-muenchen.de/econ-wp/fin/papers/0409/0409015.pdf), Review of Financial Studies | Measures the variance risk premium as realized variance minus an option-implied variance swap rate; it is negative on stock indexes | Basis of volatility-selling strategies, with crash risk attached |
| Risk | Artzner, Delbaen, Eber and Heath (1999), "Coherent Measures of Risk", Mathematical Finance 9(3) | Axioms a risk measure should satisfy; Value-at-Risk fails subadditivity | Reason to prefer expected shortfall for limits |
| Risk | [Rockafellar and Uryasev (2000)](https://sites.math.washington.edu/~rtr/papers/rtr179-CVaR1.pdf), Journal of Risk | Conditional Value-at-Risk can be minimized by linear programming | Tail-risk objective or constraint in the portfolio optimizer |
| ML | [Buehler, Gonon, Teichmann and Wood (2018), "Deep Hedging"](https://arxiv.org/abs/1802.03042) | Neural-network hedging strategies under transaction costs and risk limits | Reference for hedging when costs make textbook delta hedging wrong |

Privault's free [Notes on Stochastic Finance](https://personal.ntu.edu.sg/nprivault/indext.html) cover the mathematics behind this table, from Brownian motion through Black-Scholes, exotic and American options.

For a first system, implement HAR on realized volatility and use it in the volatility target. Leave option strategies until the paper venue's option fills have been checked against live quotes, since simulated option fills are less reliable than simulated stock fills.

## Backtesting methodology and overfitting

Most backtested edges are artifacts of search, and this literature gives you the tools to tell which. Read it before writing any strategy code, because the fixes are design decisions: a trial log, point-in-time data and a holdout you never touch.

| Work | Finding | Use in your build |
| --- | --- | --- |
| White (2000), "A Reality Check for Data Snooping", Econometrica 68(5) | Tests the best of many rules against chance | The original multiple-testing correction for trading rules |
| [Harvey, Liu and Zhu (2016)](https://www.nber.org/papers/w20592), Review of Financial Studies | With hundreds of factors tried, most claimed findings are likely false; a new factor needs a t-statistic near 3, not 2 | Raise your own significance bar with every variant tested |
| [McLean and Pontiff (2016)](https://papers.ssrn.com/abstract=2156623), Journal of Finance | Across 97 predictors, returns were 26% lower out of sample and 58% lower after publication | Haircut any published Sharpe ratio by about half before planning around it |
| [Bailey and Lopez de Prado (2014)](https://papers.ssrn.com/abstract=2460551), Journal of Portfolio Management | Deflated Sharpe Ratio corrects for number of trials and non-normal returns | Report this instead of raw Sharpe |
| [Bailey, Borwein, Lopez de Prado and Zhu (2017)](https://escholarship.org/uc/item/4w1110bb), Journal of Computational Finance | Probability of Backtest Overfitting via combinatorially symmetric cross-validation | Run it over your parameter grid |
| [Jensen, Kelly and Pedersen (2023)](https://research.cbs.dk/en/publications/is-there-a-replication-crisis-in-finance/), Journal of Finance | Counterpoint: most of 153 factors replicate, cluster into 13 themes and hold in 93 countries | Factors are real but few; think in themes, not hundreds of signals |
| Chen and Zimmermann (2022), "Open Source Cross-Sectional Asset Pricing", Critical Finance Review ([summary](https://alphaarchitect.com/2021/03/more-on-the-factor-investing-replication-debate/)) | Open code and data reproducing 319 stock characteristics | Free, audited signal definitions |
| [Yin, Miki, Lesnichenko and Gural (2026)](https://arxiv.org/abs/2603.20319) | Five open-source engines agree exactly at zero cost and diverge by up to 3.71% for high-turnover strategies once costs are on; seven engine defects found | Your cost code is where backtests go wrong; test it against a second engine |

A backtest is trustworthy only if all of these hold:

- Data is point-in-time: each record carries the time you could have known it, and delisted names are included.
- Signals at time t use data stamped before t, and orders fill at the next available price, not the signal bar's close.
- Costs are modeled per trade, including spread, fees and borrow.
- Every variant you run is logged, so trial counts feed the deflated Sharpe calculation.
- Parameters are chosen by walk-forward or purged cross-validation, and a final holdout period is used once.

## Machine learning in trading

Machine learning adds modest, real value in combining many weak signals, and far less in finding new ones from price history alone. The evidence favors tree ensembles and small neural networks on tabular features, evaluated with strict time ordering.

| Work | Finding | Use in your build |
| --- | --- | --- |
| [Gu, Kelly and Xiu (2020)](https://www.nber.org/papers/w25398), Review of Financial Studies | Trees and neural networks beat linear models at predicting stock returns by capturing interactions; all methods lean on momentum, liquidity and volatility signals | The benchmark study; start with gradient-boosted trees |
| [Kelly and Xiu (2023), "Financial Machine Learning"](https://papers.ssrn.com/abstract=4501707) | 159-page survey of return prediction, risk models and portfolio choice with ML | The single best open overview; read after the foundations |
| [Kelly, Malamud and Zhou (2024)](https://www.nber.org/papers/w30217), Journal of Finance | Argues heavily over-parameterized models improve market-timing | The case for large models |
| [Nagel (2025)](https://bfi.uchicago.edu/wp-content/uploads/2025/08/BFI_WP_2025-104.pdf) | Rebuttal: that result is an illusion; the model behaves like a volatility-timed momentum rule | Check what simple rule your complex model has rediscovered |
| [Zhang, Zohren and Roberts (2019), DeepLOB](https://arxiv.org/abs/1808.03668) | CNN plus LSTM predicts short-horizon moves from the order book and transfers to unseen stocks | Reference architecture for order-book data |
| Lim, Zohren and Roberts (2019), "Enhancing Time-Series Momentum Strategies Using Deep Neural Networks", Journal of Financial Data Science 1(4) | Networks trained directly on Sharpe ratio learn trend and sizing together | A natural ML upgrade to a trend system |
| [Hambly, Xu and Yang (2023)](https://arxiv.org/abs/2112.04553), Mathematical Finance | Survey of reinforcement learning for execution, market making, portfolio choice and hedging | Read before attempting RL; execution is its strongest use case |
| [Yang et al. (2020), Qlib](https://arxiv.org/abs/2009.11189) | Open platform for the ML research loop on equities | Reusable design for datasets, models and evaluation |

Signal-to-noise in returns is low enough that a model explaining well under 1% of monthly return variance can still be valuable, and a model that explains much more is usually leaking future data. Treat a surprisingly good ML backtest as a bug report first.

## Cutting edge, 2023 to 2026: LLMs, agents and foundation models

The newest work is strongest as a warning about evaluation: LLMs can read news usefully, but autonomous LLM trading agents and general time-series foundation models have not shown alpha under clean testing. The recurring flaw is look-ahead, since a model trained on text from the test period already knows what happened.

| Work | Finding |
| --- | --- |
| [Lopez-Lira and Tang (2023, revised 2025)](https://arxiv.org/abs/2304.07619) | ChatGPT scores of news headlines predict next-day stock returns, more for small stocks and bad news; strategy returns fall as LLM adoption rises |
| [Glasserman and Lin (2023)](https://datascience.columbia.edu/wp-content/uploads/2024/01/P021_FB_PosterSession_Fall2023.pdf) | Backtests of LLM sentiment are biased when training and test periods overlap; they test by stripping company identifiers from the text |
| [Xiao et al. (2024), TradingAgents](https://arxiv.org/abs/2412.20138) | Multi-agent framework with analyst, bull and bear researcher, trader and risk roles; reports gains over baselines in its own backtests |
| ["Agentic Trading: When LLM Agents Meet Financial Markets" (2026)](https://arxiv.org/html/2605.19337v1) | Survey of the agent literature; treats look-ahead and timestamp leakage as the field's central methodological risk |
| [Look-Ahead-Bench (2026)](https://arxiv.org/pdf/2601.13770) | Proposes a standard benchmark for measuring look-ahead bias in point-in-time financial LLMs |
| ["Can Blindfolded LLMs Still Trade?" (2026)](https://arxiv.org/html/2603.17692v1) | Anonymizes tickers and dates to test whether agent signals survive without memorized associations |
| [Wang (November 2026), "LLM Trading Agents Earn Beta, Not Alpha"](https://arxiv.org/abs/2611.19530) | 91 runs of six models over three US market regimes: market and style exposure explained a median 78% of returns, no alpha was significant, and picks did no better than random same-sector stocks |
| [Li et al. (2025), R&D-Agent-Quant](https://arxiv.org/abs/2505.15155) | LLM agents that propose and test factors and models inside a fixed research pipeline |
| [Rahimikia et al. (2025), "Re(Visiting) Time Series Foundation Models in Finance"](https://arxiv.org/abs/2511.18578) | Off-the-shelf pretrained models do poorly on return forecasting, zero-shot or fine-tuned; models pretrained from scratch on financial data do much better |
| [Noguer i Alonso and Pereira Franklin (2026)](https://arxiv.org/abs/2606.27100) | Benchmarks TimesFM, Moirai, Chronos and others on five US stocks; beating a zero-return forecast happens in a minority of cases |

Three practical conclusions follow. Use an LLM as a feature extractor over timestamped text, or as a research assistant that writes and tests code, with a deterministic engine doing all evaluation. Test any LLM signal only on dates after the model's training cutoff. Report every result as factor exposure plus alpha, across several random seeds, because the benchmark above found same-model runs differing only by seed ended 5.2 points apart on average.

## Open-access books and long-form texts

Eleven free texts cover the whole stack, from empirical finance to optimization and reinforcement learning. The first five are finance-specific and worth reading in full; the rest are references.

| Text | Covers | Access |
| --- | --- | --- |
| Scheuch, Voigt, Weiss and Frey, Tidy Finance with Python (also an R edition) | Empirical asset pricing from raw data: beta estimation, portfolio sorts, Fama-French replication, ML, portfolio optimization, all with code | [tidy-finance.org](https://www.tidy-finance.org/python/index.html) |
| Coqueret and Guida, Machine Learning for Factor Investing | Penalized regression, trees, neural networks, backtesting pitfalls and interpretability for equity factors | [mlfactor.com](http://www.mlfactor.com/) |
| Kelly and Xiu, Financial Machine Learning | Survey-length monograph on ML for return prediction and portfolios | [SSRN](https://papers.ssrn.com/abstract=4501707) and NBER working paper 31502 |
| Palomar, Portfolio Optimization: Theory and Application (Cambridge, 2025) | Heavy tails, robust and risk-parity portfolios, index tracking, pairs trading, backtesting, with R and Python code | [Online edition](https://bookdown.org/palomar/portfoliooptimizationbook/preface.html) |
| Boyd et al., Multi-Period Trading via Convex Optimization | A complete, implementable trading and simulation framework in 76 pages | [arXiv](https://arxiv.org/abs/1705.00109) |
| Hasbrouck, Securities Trading: Principles and Procedures | Market structure, order types, auctions, dealers, regulation | [NYU draft, updated December 2025](https://pages.stern.nyu.edu/~jhasbrou/STPP/STPPindex.html) |
| Hyndman and Athanasopoulos, Forecasting: Principles and Practice, 3rd ed. | Time-series forecasting and honest evaluation | [otexts.com/fpp3](https://otexts.com/fpp3/) |
| James, Witten, Hastie, Tibshirani and Taylor, An Introduction to Statistical Learning | Core supervised learning, resampling, regularization | [statlearning.com](https://www.statlearning.com/) |
| Boyd and Vandenberghe, Convex Optimization | The mathematics behind every portfolio optimizer | [Stanford](https://web.stanford.edu/~boyd/cvxbook/) |
| Sutton and Barto, Reinforcement Learning: An Introduction, 2nd ed. | Foundations for execution and market-making RL | [incompleteideas.net](http://incompleteideas.net/book/the-book-2nd.html) |
| Open Quant Live Book | Community introduction to financial data analysis and algo trading, CC BY-NC-SA | [Leanpub](https://leanpub.com/openquant) |

The standard practitioner books are not open access: Lopez de Prado's Advances in Financial Machine Learning, Carver's Systematic Trading, Isichenko's Quantitative Portfolio Management, and Cartea, Jaimungal and Penalva's Algorithmic and High-Frequency Trading. The papers above cover most of their core arguments.

## Technical implementation: architecture

Build an event-driven system in which the strategy code cannot tell whether it is in a backtest or connected to a broker. That single property is what makes paper trading a real validation step: any gap between backtest and paper results then comes from data, fills or costs, not from two diverging codebases.

&#91;embedded content: system architecture · backtest and paper paths sharing one strategy core\]

Only the two outer columns change between modes: the data source on the left and the venue on the right. Everything the strategy knows arrives as timestamped events, and everything it does leaves as orders.

Six components make up the core, each behind a narrow interface:

- Data handler: turns stored or streamed data into one ordered event stream.
- Strategy: maps events to target positions; holds no broker or storage code.
- Portfolio and sizing: converts targets to trades using the methods in the portfolio section.
- Risk gate: enforces exposure caps, order-size limits and the drawdown halt before any order leaves.
- Order manager: tracks each order's state, makes submissions idempotent with client order IDs, and recovers open orders after a restart.
- Ledger: an append-only record of events, orders, fills and positions, which is the only source of truth for P&L.

Boyd et al.'s [multi-period trading paper](https://arxiv.org/abs/1705.00109) specifies the simulation accounting precisely. NautilusTrader documents the same backtest-to-live parity design in a [production codebase](https://nautilustrader.io/docs/).

## Technical implementation: a Rust-first stack

The whole runtime can be Rust today: engine, data layer, optimizer, risk gate and broker connection, with no Python process anywhere in backtest or paper trading. Python is only hard to avoid in one offline step, training ML models, and that step can hand its result to Rust as an ONNX file. Where this section conflicts with the Python-leaning choices in the next two sections, this section wins.

The first decision is engine: adopt a Rust engine or write a thin one.

| Option | What you get | Limits |
| --- | --- | --- |
| [NautilusTrader Rust crates](https://nautilustrader.io/docs/latest/concepts/rust/) | Strategies, backtests and live trading in pure Rust; data, execution and risk engines, matching engine, portfolio, cache and indicators; Rust adapters for Interactive Brokers and Databento among 18 venues | No Alpaca adapter; the docs warn the Rust API is still changing between releases; LGPL-3.0 |
| [Barter](https://docs.rs/crate/barter/latest) | MIT-licensed engine for live, paper and backtest with pluggable Strategy and RiskManager traits and mock execution | Market-data integrations are crypto exchanges; its license text says it is not intended for production use |
| [hftbacktest](https://docs.rs/hftbacktest/) | Tick-level backtests that model feed latency, order latency and queue position; same code runs a live bot | Built for order-book strategies; needs full depth data; overkill for daily bars |
| Your own thin engine | Full control and the smallest dependency surface; a daily-bar event loop is a few hundred lines | You own the order state machine, reconnection and fill simulation |

Two combinations fit your goal. Nautilus Rust crates with an Interactive Brokers paper account gives the most engine for the least code. A thin engine of your own with an Alpaca crate gives the free paper venue and the simplest stack, and suits daily-bar ETF strategies well.

Crates by layer:

| Layer | Crate | Notes |
| --- | --- | --- |
| Async runtime | tokio | Use at the edges: feeds, broker sockets, timers |
| Research data | [polars](https://docs.rs/crate/polars) with the `lazy` and `parquet` features | Lazy queries over Parquet, larger-than-memory streaming; expect long compile times |
| SQL over files | duckdb, datafusion | Ad hoc queries on the same Parquet store |
| Ledger | sqlx with PostgreSQL or SQLite | Compile-time checked queries for orders, fills and positions |
| Money and time | rust\_decimal, chrono or jiff | No floats for prices, quantities or cash |
| Portfolio optimizer | [clarabel](https://docs.rs/crate/clarabel/latest) | Rust-native conic solver for LP, QP and SOCP, Apache-2.0; the same solver CVXPY uses by default, so Boyd's formulations port directly |
| Linear algebra and statistics | faer, nalgebra, ndarray, statrs | Covariance shrinkage, HAR regressions, HRP |
| Interactive Brokers | [ibapi (rust-ibapi)](https://docs.rs/crate/ibapi/3.1.0) | MIT; async Tokio client by default plus a blocking client; paper ports are 4002 on IB Gateway and 7497 on TWS |
| Alpaca | [apca](https://docs.rs/crate/apca/0.30.0/source/README.md), [alpaca-trade](https://docs.rs/alpaca-trade) | Community crates, no official Rust SDK; apca is async and GPL-3.0-or-later; alpaca-trade defaults to the paper endpoint |
| Model inference | [ort](https://docs.rs/crate/ort/latest) | ONNX Runtime wrapper; still on 2.0 release candidates |
| Model training in Rust | [burn](https://docs.rs/crate/burn/0.22.1), linfa, smartcore | Burn for neural networks; linfa and smartcore for classical models |
| Operations | tracing, serde, clap | Structured logs, typed config, command-line entry points |
| Testing | proptest, criterion | Property tests for accounting invariants; benchmarks for the hot path |

A workspace layout that enforces the architecture:

```text
trader/
  crates/
    core/        # events, orders, fills, Price and Qty newtypes, Clock trait
    data/        # Parquet readers, live feed adapters -> one event stream
    strategy/    # signals and target positions; depends only on core
    portfolio/   # sizing, clarabel optimizer, volatility target
    risk/        # pre-trade checks, exposure caps, drawdown halt
    execution/   # Venue trait; SimVenue, IbkrVenue, AlpacaVenue
    ledger/      # append-only store, reconciliation
  bins/
    backtest/    # wires data + SimVenue
    paper/       # wires live feed + broker venue
    research/    # one-off studies writing Parquet and plots
```

Rust gives this design four things the Python version only gets by discipline:

- Crate boundaries are checked by the compiler. If `strategy` does not list a broker crate in its Cargo.toml, strategy code cannot touch a broker.
- The order lifecycle is an enum with exhaustive matches, so an unhandled state such as a partial fill after a cancel request is a compile error.
- Newtypes for price, quantity and cash stop unit mix-ups, and fixed-point decimals remove float drift from the ledger.
- A `Clock` trait injected everywhere, a single-threaded core loop and a seeded random generator make backtests bit-for-bit repeatable, which the daily paper reconciliation depends on.

Keep the core loop synchronous and single-threaded, with async only for I/O. Run parameter sweeps as many independent single-threaded backtests in parallel with rayon.

Python remains useful in three offline places, none of them in the runtime:

| Task | Why Python is tempting | Rust-only alternative |
| --- | --- | --- |
| Training gradient-boosted trees or neural networks | Mature LightGBM, XGBoost and PyTorch tooling | Train in Python once and export ONNX for ort, or train with Burn or linfa and accept a smaller toolbox |
| Replicating published results | Tidy Finance, Qlib and the Boyd reference code are Python | Port the method to Polars and clarabel; check your numbers against the published tables |
| Cross-checking the backtest against a second engine | vectorbt and similar are quick | Use Nautilus or Barter as the second engine against your own, or the reverse |

The costs are slower research iteration and porting work, since almost all published replication code is Python. A practical rule is to prototype a study as a small Rust binary that writes Parquet, inspect it in the DuckDB command line, and promote it into the strategy crate only once it survives the overfitting checks.

## Technical implementation: data, engines and frameworks

The main decision is whether to adopt an existing event-driven engine or write your own, and for a system meant to reach a broker, adopting one saves the hardest work. Order state machines, reconnection handling and fill simulation are where home-built systems fail. A small vectorized research layer on top of an adopted engine is the common split.

| Framework | What it is | Fit |
| --- | --- | --- |
| [NautilusTrader](https://pypi.org/project/nautilus-trader) | Rust-native engine with a Python API; deterministic event-driven backtests and live trading from the same strategy code; LGPL-3.0 | Closest match to the architecture above; steepest learning curve |
| [QuantConnect LEAN](https://github.com/QuantConnect/Lean) | Open-source C# engine that runs Python or C# algorithms; runs locally through Docker and the LEAN CLI; many brokerage plugins including Interactive Brokers | Mature and broad; local data is your responsibility |
| [Qlib](https://github.com/microsoft/qlib/blob/main/README.md) | Microsoft's ML research platform for equities: datasets, model zoo, evaluation | Research loop for ML signals, not an execution system |
| [cvxgrp markowitz-reference](https://github.com/cvxgrp/markowitz-reference) | Reference implementation of the modern Markowitz problem | Drop-in portfolio construction layer |
| vectorbt, zipline-reloaded, backtrader | Python backtesters: vectorized sweeps (vectorbt) and older event-driven designs | Fast idea screening; cross-check results against your main engine |

Check each project's current adapters and license before committing, since both change between releases.

For data, the rule is to store what you received, when you received it, and never overwrite it. Corrections, restated fundamentals and adjusted prices are new records with their own timestamps, which is what makes point-in-time queries possible later.

| Layer | Reasonable default | Why |
| --- | --- | --- |
| Research store | Parquet files partitioned by date and symbol, queried with DuckDB or Polars | Columnar, cheap, fast for whole-history scans |
| Operational ledger | PostgreSQL | Transactions and constraints for orders, fills and positions |
| Free research data | [Kenneth French library](https://mba.tuck.dartmouth.edu/pages/faculty/ken.french/data_library.html), [AQR data sets](https://www.aqr.com/Insights/Datasets/Time-Series-Momentum-Original-Paper-Data), Open Source Asset Pricing, FRED, SEC EDGAR | Replication targets and macro or fundamental inputs |
| Price history and live feed | Your paper broker's API first; a paid vendor only once a strategy needs depth or history the broker lacks | The feed you paper trade on should be the feed you backtest on |

Survivorship is the usual gap in free price data: histories that drop delisted stocks overstate equity strategy returns. ETFs and futures largely avoid the problem, which is another reason to start there.

## Technical implementation: paper trading and validation

Paper trading validates plumbing and costs; it cannot validate alpha in any reasonable time. A strategy with a true Sharpe ratio of 1 needs about four years of returns to reach a t-statistic of 2, since the t-statistic is roughly the Sharpe ratio times the square root of years. So define success for the paper phase as agreement with the backtest, not profit.

| Paper venue | Access | Known limits |
| --- | --- | --- |
| [Alpaca](https://docs.alpaca.markets/docs/trading-api) | Free for all users; a real-time simulation with free market data that can be reset at will; the Python client switches with `paper=True` | Stocks, options and crypto only; fills are simulated |
| [Interactive Brokers](https://www.interactivebrokers.com/docs/tws-api/doc/notes-limitations/limitations/paper-trading) | Paper account alongside a funded IBKR Pro account; connect through TWS or IB Gateway logged in with paper credentials | IBKR states the paper environment relies on more simulated technology, so execution can differ from live |

For IBKR in Python, [ib\_async](https://github.com/ib-api-reloaded/ib_async) is the maintained successor to ib\_insync. The API does not distinguish paper from live, so make the account type an explicit startup check in your own code.

Neither venue models queue position or market impact. Favor marketable orders in liquid instruments during validation, and treat passive-order fill rates on paper as optimistic.

A validation protocol that makes the paper phase informative:

1. Freeze the strategy, parameters and cost model, and record the commit hash and the number of variants tried.
2. Each day, rerun the backtest over the same day using the data the live process actually received.
3. Reconcile four things: signals, orders, fill prices and end-of-day positions. Signals and orders should match exactly; any mismatch is a bug.
4. Compare realized slippage per trade with the cost model, and recalibrate the model, not the strategy.
5. Track the difference between paper and shadow-backtest daily returns. A small, unbiased difference is the pass condition.
6. Run failure drills: kill the process mid-order, drop the data feed, restart with open positions, and confirm the ledger and broker agree afterward.

Operationally, the system needs a heartbeat with alerting, a kill switch that flattens or freezes positions, synchronized clocks, and broker credentials kept out of the repository.

## Build roadmap

Build in five phases, each ending in a check that can fail, with the reading attached to the phase that needs it.

1. Data and replication. Stand up the Parquet store and reproduce a published factor from raw prices. Read Tidy Finance and Fama and French (1993). Exit check: your momentum series tracks the Kenneth French series closely.
2. First strategy in an event-driven backtest. Implement time-series momentum on a small set of liquid ETFs with a spread-plus-fees cost model. Read Moskowitz, Ooi and Pedersen (2012) and Boyd et al. (2017). Exit check: results agree with a second engine once costs are switched on.
3. Sizing and risk. Add volatility targeting, exposure caps and the drawdown halt. Read Moreira and Muir (2017), Lopez de Prado (2016) and Boyd et al. (2024). Exit check: the risk gate blocks a deliberately oversized test order.
4. Paper trading. Connect the same strategy core to Alpaca or IBKR paper and run the validation protocol above. Read Hasbrouck's notes on order handling. Exit check: signals and orders match the shadow backtest every day for a full month, and failure drills pass.
5. Research discipline, then extensions. Add the trial log, Deflated Sharpe Ratio and overfitting probability before testing new ideas. Read Bailey and Lopez de Prado (2014), Harvey, Liu and Zhu (2016), then Kelly and Xiu (2023). Only after that add ML signal combination or LLM text features, tested on post-cutoff dates.

## Sources and access notes

Links go to open versions where one exists: arXiv, SSRN, NBER, author pages or institutional repositories. Classics listed with journal, volume and year but no link are behind publisher access, though working-paper copies of most are easy to find by title. Findings quoted with numbers come from the linked abstracts or papers. The 2026 preprints in the cutting-edge section are not yet peer reviewed. This review is research and engineering guidance, not investment advice.

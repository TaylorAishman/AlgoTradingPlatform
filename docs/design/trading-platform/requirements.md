# Requirements: trading-platform

Stage 2 of 3 (requirements). Written 2026-11-22 from `architecture.md` and the papers in `docs/reference/trading-papers/`. Needs human approval before the plan stage starts.

## 1. How to read this document

- **IDs** run from R-001 to R-288 and are never reused or renumbered. A requirement that is dropped later keeps its number and is marked withdrawn.
- **Source** uses these short forms: "Arch 5.3" is a section of `architecture.md`; "Arch D5", "Arch C4", "Arch gap 10" and "Arch Q6" are its decisions, conflicts, gaps and open questions; papers are cited as the architecture cites them (short title in italics, PDF page index). "I4" points to an item in section 5 of this document, where the architecture had to be interpreted. "N" marks a number chosen by this document and listed in section 3.
- **Acceptance check** is a pytest node id unless it is written as a command. It is relative to the test directory named above each table (a path starting with `tests/` is relative to the repository root). A check passes when `pytest <node id>` exits 0. The test names are part of the requirement: the plan stage must create them under these names. Several test files share a base name across directories (`test_perf.py`, `test_contract.py`), so the test tree must be set up to allow that.
- Checks marked **(slow)** run with `OMP_NUM_THREADS=1 pytest -m slow <node id>` on the owner's machine and are not part of `./.devteam/verify.sh`. Checks marked **(alpaca)** need paper-account keys and network access and are not part of `verify` either. Everything else runs offline inside `verify`.
- "Synthetic panel" means data from the generator of R-018 to R-023. "Randomized after `as_of`" means every stored record whose `available_at` is later than `as_of` is replaced by random values, as in the look-ahead check of *LLM Trading Agents Earn Beta* p4.
- The papers were read from a plain-text extraction (no PDF renderer on this machine either). Prose is reliable; equations and tables are garbled. Where a check depends on a formula or a worked number, the row says what was legible and what must be confirmed against the PDF.

## 2. Answers assumed for the architecture's open questions

Pull request 1 was merged without answers to section 8 of the architecture. The requirements below assume the following. Each line is a decision a human should confirm or change; the last column shows what moves if the answer changes.

| Question | Assumed answer | Requirements that depend on it |
| --- | --- | --- |
| Q1 Toolchain and verify script | CI and development pin Python 3.12 (one above the 3.11 floor; wheels for 3.15 remain unverified); the package declares `requires-python >= 3.11`. A human rewrites `.devteam/verify.sh` and `.github/workflows/verify.yml` to run the five steps of Arch 7.2 before the first build task merges. | R-244 to R-252, R-257 |
| Q2 Data plan and history | Free Basic plan, Alpaca as the only vendor, history from 2016, SIP feed for historical bars. If Alpaca refuses SIP history to a paper-only account, ingestion stops with an error; it does not fall back to IEX. | R-043, R-044, R-046, R-047, R-190 |
| Q3 Survivorship | A liquidity-ranked universe built from whatever inactive-asset history Alpaca returns is acceptable, provided every universe build and every run report discloses how many members are inactive. | R-052, R-053, R-142, R-272 |
| Q4 Short selling | As Arch D20: long-only by default. Long-short paths exist in the optimizer, ledger and risk gate and are enabled per strategy, for easy-to-borrow names only and only when account equity is at least $2,000. | R-101, R-106, R-172 |
| Q5 Account size and order style | A $100,000 paper account. Whole-share market `opg` orders are the default; fractional `day` orders are a per-strategy option. With 500 names this leaves about $200 per name, so whole-share rounding will drop higher-priced names; that is accepted and reported. | R-155, R-158, R-159 |
| Q6 Gate thresholds | DSR at least 0.95, t at least 3.0, PBO at most 0.05, alpha t at least 2.0, 60 paper sessions. Tracking tolerance: 5 bps per session and 25 bps cumulative (proposed here). Published signals get no lower hurdle unless a human sets one. | R-217 to R-223 |
| Q7 Conflict defaults | The defaults of Arch 2.3 stand: Markowitz++ when a forecast exists, HRP and the naive allocators as baselines, 3/2-power impact, temporal splits for fitting and DSR/PBO for selection. | R-095, R-106, R-117, R-215 |
| Q8 LLM provider and news | The plugin is off by default and Alpaca news is its source. Validation forward in paper trading is accepted. No vendor is assumed: requirements cover the provider protocol, an offline fake provider and the safeguards. A concrete adapter is not ready to be specified (section 7). | R-229 to R-243 |
| Q9 Features | Price-and-volume features only. | R-069 |
| Q10 Risk limits | No values are assumed. Every `RiskLimits` field is mandatory configuration and the platform refuses to plan paper orders without all of them. | R-162 to R-175 |
| Q11 Host and schedule | No host or scheduler is assumed. Jobs are one-shot, idempotent commands for an external scheduler. A job that runs outside its submission window submits nothing. An alert is a record in the job log and the `job_log` table plus a non-zero exit code; delivery to a person is not specified. | R-160, R-265, R-268, R-269, R-277, R-278 |
| Q12 Second engine | No second open-source engine. The reference loop and the Alpaca paper comparison are the cross-checks. | R-133 to R-135, R-187 |
| Q13 Trial discipline | Confirmed. Gates can only be evaluated on registered trials, and the registry has no reset or delete. | R-191 to R-196 |

## 3. Numbers chosen by this document

Neither the architecture nor a reference fixes these. They are defaults or thresholds this document proposes so that the requirements are testable. All are configuration values unless marked "limit".

| Value | Used in | Basis |
| --- | --- | --- |
| Python 3.12 pinned for CI | R-244 | One above the floor in Arch 3. |
| 252 periods per year for annualization | R-198 | The references use 250 (*Deflated Sharpe Ratio* p10, *Multi-Period Trading* p23), 252 (*Can Blindfolded LLMs* p5) and 261 (*Time Series Momentum* p13). |
| Daily bars become available 15 minutes after the session close | R-026 | The free plan's delay (Arch 2.2). |
| A backfilled corporate action becomes available at the open of its ex-date session | R-027 | Conservative: never earlier than the market could act on it. |
| 5 attempts, backoff starting at 1 s and doubling | R-045 | Engineering judgement. |
| Half-spread 0.0005 (5 bps) for every asset | R-102 | *Statistical Arbitrage* p4 and *LLM Trading Agents Earn Beta* p4 both charge 5 bps per trade; no quote data is ingested (I12). |
| Average daily dollar volume over 60 sessions (median) | R-102 | Same window as the universe rule in Arch 5.3. |
| Covariance half-life 125 sessions | R-090 | *Markowitz at Seventy* p32. |
| 15 statistical factors | R-091 | *Statistical Arbitrage* p6 ("approximately 15 factors"). |
| Minimum history 60 sessions for a risk estimate | R-094 | Engineering judgement. |
| Winsorize standardized scores at plus and minus 3 | R-081 | *Markowitz at Seventy* p22 names the method, not the range. |
| Refit predictors every 252 sessions | R-074 | *Empirical Asset Pricing via ML* p26 ("refit once every year"). |
| Band state replayed over 60 sessions | R-067 | Same length as the estimation window. |
| Five price-and-volume characteristics | R-069 | The three families of *Empirical Asset Pricing via ML* p2; the list is this document's. |
| Solver time limit 10 s | R-116 | Engineering judgement. |
| Minimum trade notional 0 (off) | R-122 | Engineering judgement. |
| Median `allocate` time 0.15 s (limit) | R-123 | Derived from the ten-year backtest target (I20). |
| Simulated fills capped at `participation_max` of realized volume | R-140 | *Markowitz at Seventy* p16; Arch 4 says "fill models" without naming them. |
| Peak memory 4 GB for the ten-year backtest (limit) | R-153 | Engineering judgement. |
| Buying-power buffer 2% | R-156 | Covers an overnight gap between the sizing price and the open. |
| Cash break tolerance $0.01 | R-186 | Engineering judgement. |
| 20 sampled decision days for the leak check | R-212 | Engineering judgement. |
| Fewer than 20 candidate trials makes PBO not evaluable | R-219 | Relative ranks must resolve 0.05 (*Probability of Backtest Overfitting* p23). |
| CSCV in 60 s (limit) | R-207 | Engineering judgement. |
| Placebo fails when PSR against zero reaches 0.95 on noise | R-216 | Mirrors the gate 3 level. |
| 20 shuffles for the negative control | R-226 | Engineering judgement. |
| At least 5 seeds for a pretrained-model signal | R-225 | *LLM Trading Agents Earn Beta* uses 3, *Can Blindfolded LLMs* 20. |
| Tracking tolerance 5 bps per session, 25 bps cumulative | R-223 | Arch Q6 asks for a value; none is given. |
| Bars for at least 95% of the universe before a decision | R-266 | Engineering judgement. |
| LLM scoring time budget 300 s | R-242 | Half of the 10-minute job target. |
| Decision pipeline in 120 s (limit) | R-288 | Share of the 10-minute job target left after ingestion. |
| `--cost-scale` scales simulated costs only | R-256 | A stress test changes what happened, not what the strategy believed (*Markowitz at Seventy* p29). |

## 4. Requirements by component

### A. Core contracts

Owns `src/atp/core/`, `src/atp/testing/`, `tests/core/`. Test paths are under `tests/core/`.

| ID | Requirement | Source | Acceptance check |
| --- | --- | --- | --- |
| R-001 | Every type in Arch 5.1 must be a frozen dataclass with exactly the field names given there. | Arch 5.1 | `test_types.py::test_fields_and_frozen` |
| R-002 | Every datetime field of a core type must reject a naive or non-UTC value with `ValueError` at construction. | Arch 5 (intro) | `test_types.py::test_naive_datetime_rejected` |
| R-003 | One function must build `client_order_id` as `f"{strategy}-{session}-{asset}-{side}-{slice}"` and must raise `ValueError` when the result is longer than 128 characters. | Arch 5.1; Alpaca limit in Arch 2.2 | `test_types.py::test_client_order_id_format_and_length` |
| R-004 | Constructing an `OrderRequest` with `time_in_force` `opg` or `cls` and a non-integer `qty` must raise `ValueError`. | Arch 3 (Alpaca compatibility) | `test_types.py::test_auction_order_requires_whole_shares` |
| R-005 | Constructing an `OrderRequest` with `qty` not above zero, or `type="limit"` without `limit_price`, must raise `ValueError`. | Arch 5.1 | `test_types.py::test_order_request_validation` |
| R-006 | Every protocol in Arch 5 (`Clock`, `TradingCalendar`, `DataView`, `DataStore`, `DataSource`, `Signal`, `Predictor`, `Calibrator`, `RiskModel`, `CostModel`, `Allocator`, `Broker`, `OrderPlanner`, `ExecutionSchedule`, `RiskGate`, `Strategy`, `TrialRegistry`) must be defined in `atp.core` with the method names and parameters of the architecture. | Arch 4, 5 | `mypy --strict src/atp/core` exits 0; `test_protocols.py::test_signatures_match_architecture` |
| R-007 | `atp.core` and `atp.testing` must import no other `atp` package; each of `atp.data`, `atp.signals`, `atp.models`, `atp.portfolio`, `atp.engine`, `atp.execution`, `atp.validation` and `atp.llm` must import from `atp` only `atp.core`. | Arch 4 (dependency rule); I7 | `test_import_rules.py::test_component_imports` (AST scan of `src/atp`) |
| R-008 | Modules under `src/atp`, other than the wall-clock implementation of `Clock`, must not call `datetime.now`, `datetime.utcnow`, `date.today` or `time.time`. | Arch 5.2, D3 | `test_import_rules.py::test_no_wall_clock_calls` |
| R-009 | `TradingCalendar.sessions(start, end)` must return exactly the stored sessions in the inclusive range, strictly increasing. | Arch 5.2; *Implementation Risk* p33 | `test_calendar.py::test_sessions_strictly_increasing` (property test) |
| R-010 | The calendar factory must return the same `TradingCalendar` object for every call in a process for a given store. | Arch 5.2; *Implementation Risk* p33 | `test_calendar.py::test_single_instance_per_process` |
| R-011 | `open_close(session)` must return the stored open and close of that session in UTC, including early closes. | Arch 5.2, 5.3 | `test_calendar.py::test_open_close_early_close` |
| R-012 | `open_close` and `next_session` must raise `CalendarError` for a date that is not a stored session; they must not snap to a neighbouring date. | Arch 5.2; *Implementation Risk* p13 | `test_calendar.py::test_non_session_raises` |
| R-013 | The decision instant of a session must be `decision_time` (default 19:05) in `America/New_York` on the session date, converted to UTC, on both sides of a daylight-saving change. | Arch 5.2 | `test_calendar.py::test_decision_time_dst` (2026-01-15 gives 2026-01-16T00:05Z; 2026-07-15 gives 2026-07-15T23:05Z) |
| R-014 | The configuration loader must reject a TOML file containing an unknown section or key with a `ConfigError` that names the key. | Arch 5.12 | `test_config.py::test_unknown_key_rejected` |
| R-015 | An implementation named as `module:Class` in configuration must be loaded by that path, including a class defined outside the `atp` package, and a path that cannot be loaded must raise a `ConfigError` naming it. | Arch D22 | `test_config.py::test_dotted_path_loads_external_class` |
| R-016 | `config_hash` must be the SHA-256 of a canonical form of the fully resolved configuration (defaults filled in), unchanged by key order, comments or whitespace and changed by any change of value. | Arch D22, 5.9 | `test_config.py::test_config_hash_canonical` |
| R-017 | Configuration models must have no field for a credential; credentials must be read only from the environment variables named in Arch 5.12, and a missing variable must raise a `ConfigError` naming it. | Arch 3 (Data), 5.12 | `test_config.py::test_credentials_only_from_environment` |
| R-018 | The synthetic market generator must return identical data for identical seed and parameters and different data for different seeds. | Arch 7.1 A | `test_synthetic.py::test_deterministic_per_seed` |
| R-019 | The generator must be able to plant a signal of a requested strength: for a requested rank correlation of 0.05 between the planted score and the next-session return, the realized mean over 200 assets and 1,000 sessions must be within 0.01 of 0.05. | Arch 7.1 A | `test_synthetic.py::test_planted_signal_strength` |
| R-020 | The generator's pure-noise panel must show no predictability: over 200 assets and 1,000 sessions the mean rank correlation of its score with the next-session return must be below 0.01 in absolute value and the lag-1 autocorrelation of returns below 0.02 in absolute value. | Arch 7.1 A; *Seemingly Virtuous Complexity* p2 | `test_synthetic.py::test_noise_panel_has_no_signal` |
| R-021 | The generator's reversal panel must have a negative relation between the trailing 252-session return and the following 21-session return, with a t-statistic of -3 or lower. | Arch 7.1 A; *Seemingly Virtuous Complexity* p2 | `test_synthetic.py::test_reversal_panel` |
| R-022 | The generator must emit all six datasets of Arch 5.3 in the store's schema and, on request, a split, a cash dividend, a delisted asset, a reused ticker, an early-close session and a news item published after the close. | Arch 3 ("Tests run on synthetic data"), 7.1 | `test_synthetic.py::test_emits_all_datasets_and_events` |
| R-023 | The generator must emit `1Min` bars with a matching intraday calendar on request. | Arch 3 (Scope), 7.1 F | `test_synthetic.py::test_minute_bars` |

### B. Data

Owns `src/atp/data/`, `tests/data/`. Test paths are under `tests/data/`.

| ID | Requirement | Source | Acceptance check |
| --- | --- | --- | --- |
| R-024 | The methods of `DataView` (`bars`, `universe`, `asset_flags`, `news`, `corporate_actions`) must never return a record whose `available_at` is later than the view's `as_of`. | Arch 5.3 (contract), D5 | `test_view_pit.py::test_no_record_after_as_of` (property test over random `as_of`, all five methods) |
| R-025 | `DataStore.append` must reject any record that lacks a period, a timezone-aware `available_at` or a timezone-aware `ingested_at`. | Arch 5.3 | `test_store.py::test_append_requires_three_times` |
| R-026 | A backfilled daily bar must get `available_at` equal to its session close plus `bars_available_lag` (default 15 minutes). | Arch 5.3; N | `test_store.py::test_bar_available_at_model` |
| R-027 | A backfilled corporate action must get `available_at` equal to the open of its ex-date session. | Arch 5.3; I18; N | `test_store.py::test_corporate_action_available_at` |
| R-028 | A news item must get `available_at` equal to `created_at` at minute resolution or finer, and text changed by a later update must be visible only from its `updated_at`. | Arch 5.1, 5.3, D5; I18 | `test_view_pit.py::test_news_versions_by_time` |
| R-029 | `DataStore.append` must never modify or delete a row that is already stored. | Arch 5.3; *Qlib* p4 | `test_store.py::test_append_only` (hash of existing rows unchanged) |
| R-030 | When a re-ingested record differs from the stored version of the same key, a new version must be appended with `available_at` not earlier than its `ingested_at`, the revision must be counted in the ingest result, and a view with an earlier `as_of` must still return the old value. | Arch 5.3, 7.1 B ("never changes a stored value silently") | `test_store.py::test_revision_is_versioned` |
| R-031 | Re-ingesting unchanged data must append nothing: `append` returns 0. | Arch 5.3, D19 | `test_store.py::test_reingest_is_noop` |
| R-032 | `snapshot_id` must be a content hash: equal stored content gives an equal id and any appended row changes it. | Arch 5.3 | `test_store.py::test_snapshot_id` |
| R-033 | Each dataset must be written as Parquet under `$ATP_HOME/data/<dataset>/` with the key, columns and partitioning of Arch 5.3, and `bars` must also carry a `feed` column. | Arch 5.3; I5 | `test_store.py::test_dataset_schemas` (one case per dataset) |
| R-034 | `atp.data` must be the only package that reads or writes the Parquet datasets: the other packages under `src/atp` must not import `pyarrow` or call `read_parquet` or `to_parquet`. | Arch 3 (Data) | `test_isolation.py::test_only_data_layer_touches_parquet` (AST scan) |
| R-035 | A ticker used by two issuers over time must resolve to two `AssetId` values with non-overlapping validity ranges, and `bars` must never return both issuers' bars in one column. | Arch 5.1 (`AssetId`), 5.3; Alpaca forum note in Arch 2.2 | `test_assets.py::test_reused_ticker_two_assets` |
| R-036 | `asset_flags` must return, for each asset, the ticker valid at `as_of` together with `tradable`, `shortable`, `easy_to_borrow` and `fractionable`. | Arch 5.3; I6 | `test_assets.py::test_flags_include_symbol_as_of` |
| R-037 | Returns computed from adjusted prices must equal returns from raw prices, to 1e-9, on every session that is not a corporate-action ex-date. | Arch 5.3, 7.1 B | `test_adjustment.py::test_adjusted_equals_raw_off_action_dates` |
| R-038 | Across a split ex-date the adjusted return must equal the raw return corrected by the split ratio, to 1e-9. | Arch 5.3; *Multi-Period Trading* p17 | `test_adjustment.py::test_split` |
| R-039 | On a cash-dividend ex-date the adjusted return must equal (close plus dividend) divided by the previous close, minus one, to 1e-9. | Arch 5.3; *Multi-Period Trading* p17 | `test_adjustment.py::test_cash_dividend` |
| R-040 | Adjusted bars returned by a view must not change when a corporate action with `available_at` later than the view's `as_of` is added to the store. | Arch 5.3 (contract); I4 | `test_adjustment.py::test_invariant_to_future_actions` |
| R-041 | `bars` must return a frame indexed by the most recent `lookback` calendar sessions at or before `as_of`, with one column per requested asset in the requested order and NaN where no bar exists; it must not forward-fill. | Arch 5.3; *Implementation Risk* p13 (stale data) | `test_view_pit.py::test_bars_shape_and_no_forward_fill` |
| R-042 | The Alpaca source must follow page tokens until none is returned and must yield every row exactly once. | Arch 3 (10,000 points and 50 articles per page) | `test_alpaca_source.py::test_pagination` (three-page fixtures for bars and news) |
| R-043 | The Alpaca source must send at most 200 requests in any rolling 60 seconds, the limit being configurable. | Arch 3 (Performance, Alpaca) | `test_alpaca_source.py::test_rate_limit` (injected clock) |
| R-044 | A bar backfill of S symbols over D sessions must use at most 1.1 x ceil(S x D / 10,000) + 1 requests. | Arch 3 (Performance: about 820 pages for 3,000 symbols) | `test_alpaca_source.py::test_backfill_request_count` |
| R-045 | An HTTP 429 or 5xx response must be retried up to 5 attempts with a backoff starting at 1 second and doubling; after the last attempt the ingest must raise `IngestError` and store nothing from the failed batch. | Arch 3; N | `test_alpaca_source.py::test_retry_then_fail_atomically` |
| R-046 | Bar ingestion must request the `sip` feed by default, and when the source refuses that feed it must raise an error naming the feed and must not substitute another feed. | Arch 3, gap 10; Q2 | `test_alpaca_source.py::test_sip_refusal_is_an_error` |
| R-047 | `DataView.bars` must raise `DataError` when asked for `volume`, `vwap` or `trade_count` on a session for which only IEX-feed bars are stored. | Arch 3 ("IEX volume must not be used"); I5 | `test_view_pit.py::test_iex_volume_refused` |
| R-048 | Each of the five `DataSource` fetch methods of the Alpaca implementation must map a hand-written response fixture to the dataset schema of Arch 5.3 with timestamps in UTC. | Arch 5.3, 7.1 B | `test_alpaca_source.py::test_fetch_maps_to_schema` (five cases) |
| R-049 | The default universe effective from the first session of a month must be the 500 assets with the highest median dollar volume (raw close times raw volume) over the trailing 60 sessions, among assets with a raw close of at least $5 and at least 252 sessions of history. | Arch 5.3, D7 | `test_universe.py::test_selection_rule` (against an independent computation on a synthetic panel) |
| R-050 | A member of the configured ETF list must enter the universe only once it has 252 sessions of history. | Arch 5.3 | `test_universe.py::test_etf_history_requirement` |
| R-051 | A universe must be computed only from data available at the decision instant of the last session of the previous month, and a stored universe must not change when later data is added. | Arch 5.3, D7; *Statistical Arbitrage* p4 | `test_universe.py::test_point_in_time_and_stable` |
| R-052 | An asset that is inactive today must be a member of every past universe it qualified for on the data visible at that time. | Arch 5.3, D7; Q3 | `test_universe.py::test_delisted_asset_included_before_delisting` |
| R-053 | Every universe build must record, per rebuild date, the number and share of members that are inactive in the data snapshot used. | Arch gap 3; Q3 | `test_universe.py::test_survivorship_disclosure` |
| R-054 | The data check must report every session on which an active universe member has no bar. | Arch 7.1 B | `test_checks.py::test_gap_detected` |
| R-055 | The data check must report duplicate bars for one key and bars dated on a non-session date. | Arch 7.1 B; *Implementation Risk* p13 | `test_checks.py::test_duplicates_and_non_session_bars` |
| R-056 | The data check must report as errors a non-positive price, a high below the low, and a negative volume, and as a warning a zero volume. | Arch 7.1 B | `test_checks.py::test_ohlcv_sanity` |
| R-057 | The data check must report a ticker whose validity ranges overlap for two assets. | Arch 7.1 B | `test_checks.py::test_overlapping_ticker_ranges` |
| R-058 | The data check must return each finding with a level (error or warning), the dataset, the asset, the session and a check code. | Arch 5.11, 7.1 B | `test_checks.py::test_finding_fields` |

### C. Signals

Owns `src/atp/signals/`, `tests/signals/`. Test paths are under `tests/signals/`.

| ID | Requirement | Source | Acceptance check |
| --- | --- | --- | --- |
| R-059 | `Signal.compute` must return a float series indexed by exactly the assets of `universe`, with NaN meaning no opinion. | Arch 5.4 | `test_contract.py::test_output_shape` (every shipped signal) |
| R-060 | Every shipped signal must be a pure function of the view: two calls with the same view, on the same or a new instance, must return identical output. | Arch 5.4; I8 | `test_contract.py::test_pure_function_of_view` |
| R-061 | The output of every shipped signal and predictor at `as_of` must be unchanged when the store is randomized after `as_of`. | Arch D5, 5.9 (`leak_check`) | `test_contract.py::test_no_lookahead` |
| R-062 | A signal must return NaN for an asset with fewer than `lookback_sessions` bars and must not request more than `lookback_sessions` sessions from the view. | Arch 5.4 (`SignalSpec`) | `test_contract.py::test_insufficient_history_is_nan` |
| R-063 | The `tsmom` score must have the sign of the asset's trailing 252-session return in excess of the configured cash rate. | Arch 5.4; *Time Series Momentum* p3, p18; I12 | `test_tsmom.py::test_sign_follows_planted_trend` |
| R-064 | The absolute `tsmom` score must be inversely proportional to the asset's volatility from the estimator of R-089. | Arch 5.4; *Time Series Momentum* p13, p18 | `test_tsmom.py::test_volatility_scaling` (doubling volatility halves the score) |
| R-065 | `resid_reversion` must regress the 60 most recent session returns of an asset on those of its factor ETF and compute the s-score from the cumulative residual by the appendix of *Statistical Arbitrage* (eq. 21-22, with the mean centred across assets). | Arch 5.4; *Statistical Arbitrage* p45-46 | `test_resid_reversion.py::test_sscore_matches_hand_computed_fixture` (to 1e-9; fixture transcribed from the PDF) |
| R-066 | `resid_reversion` must return NaN for an asset whose estimated mean-reversion speed is below 252/30 per year. | *Statistical Arbitrage* p46 | `test_resid_reversion.py::test_slow_reversion_rejected` |
| R-067 | `resid_reversion` must return a non-zero score, equal to minus the s-score, only while the asset's band state is open, where the state opens when the absolute s-score exceeds 1.25, closes when it falls below 0.5, and is replayed from the s-score path over the last 60 sessions. | Arch 5.4; *Statistical Arbitrage* p4; I8; N | `test_resid_reversion.py::test_band_state_replay` |
| R-068 | `resid_reversion` must take an asset's factor ETF from a configured asset-to-ETF map when one is given, and otherwise use the ETF of the configured list with the highest return correlation over the estimation window. | *Statistical Arbitrage* p13-14; I8 | `test_resid_reversion.py::test_factor_etf_choice` |
| R-069 | `pv_characteristics` must expose one characteristic per configured instance, and must offer at least: return from t-252 to t-21, return from t-21 to t, 60-session EWMA volatility, 60-session median dollar volume, and 60-session mean of absolute return over dollar volume. | Arch 5.4; *Empirical Asset Pricing via ML* p2; I9; Q9; N | `test_pv_characteristics.py::test_each_characteristic_matches_reference` |
| R-070 | `Predictor.fit` must not read any data dated after `train_end`: the artifact must be identical when all later data is randomized. | Arch 5.4, D16 | `test_predictors.py::test_fit_blind_after_train_end` |
| R-071 | `Predictor.fit` must choose hyper-parameters on the last `validation_sessions` label dates at or before `train_end` and estimate parameters only on earlier dates, without shuffling. | Arch 5.4; *Empirical Asset Pricing via ML* p10 | `test_predictors.py::test_temporal_split_order` |
| R-072 | A training or validation row must be dropped when its forward-return label extends past `train_end`. | Arch 5.4, D16 | `test_predictors.py::test_labels_realized_by_train_end` |
| R-073 | `Predictor.predict` must raise `LookaheadError` for a view whose session is at or before the artifact's `train_end`. | Arch 5.4 | `test_predictors.py::test_predict_refuses_in_sample_session` |
| R-074 | Predictors must be refitted every 252 sessions (configurable) on an expanding window, and a prediction for a session must use the newest artifact whose `train_end` is earlier than that session. | *Empirical Asset Pricing via ML* p26; N | `test_predictors.py::test_refit_schedule_and_artifact_choice` |
| R-075 | `elastic_net_huber` must fit a linear model under Huber loss with an elastic-net penalty. | Arch 5.4; *Empirical Asset Pricing via ML* p12-13 | `test_predictors.py::test_elastic_net_huber` (a large penalty gives all-zero coefficients; with planted outliers the coefficient error is below that of least squares) |
| R-076 | `gbt` must use scikit-learn's gradient-boosted trees with hyper-parameters chosen from a grid given in configuration, so that the grid is part of `config_hash`. | Arch 3 (Dependencies), D18; Arch C3 | `test_predictors.py::test_gbt_grid_from_config` |
| R-077 | Fitting twice with the same view, `train_end` and seed must give predictions equal to 1e-12. | Arch 3 (deterministic verify) | `test_predictors.py::test_deterministic_fit` |
| R-078 | Each artifact must be stored under `$ATP_HOME/models/<predictor>/<train_end>/` with `train_end`, feature list, chosen hyper-parameters, seed, code revision and data snapshot id. | Arch 5.12, D16 | `test_predictors.py::test_artifact_layout_and_metadata` |
| R-079 | On the pure-noise panel, a backtest of each shipped predictor must fail gate 3. | Arch 7.1 C; Arch C3 | `test_predictors.py::test_noise_panel_fails_gate3` |
| R-080 | On a panel with a planted rank correlation of 0.05, each shipped predictor's out-of-sample mean rank correlation with the next-session return must be positive with a t-statistic of at least 3. | Arch 7.1 C | `test_predictors.py::test_planted_signal_recovered` |
| R-081 | The calibrator must standardize each signal's scores across assets to mean 0 and standard deviation 1 per session and then clip them at plus and minus 3 (configurable). | Arch D17; *Markowitz at Seventy* p22; N | `test_calibrator.py::test_standardize_and_winsorize` |
| R-082 | The calibrator must estimate each signal's coefficient by a walk-forward cross-sectional regression that uses only forward returns fully realized by `as_of`. | Arch D17 | `test_calibrator.py::test_uses_only_realized_returns` (unchanged when randomized after `as_of`) |
| R-083 | On a synthetic panel where the forward return is b times the standardized score plus noise, the calibrator's coefficient must be within 20% of b. | Arch D17 | `test_calibrator.py::test_recovers_planted_coefficient` |
| R-084 | The calibrator must return `Forecast.horizon_sessions` equal to 1 and must scale the coefficient of a signal with horizon h by 1/h. | Arch 5.1, C9; I10 | `test_calibrator.py::test_common_one_session_horizon` |
| R-085 | The calibrator must combine per-signal forecasts with the weights given in configuration (equal by default) and must record the weights actually used in `Forecast.sources`. | Arch 5.1, 5.4 | `test_calibrator.py::test_weights_recorded` |
| R-086 | `Forecast.rho` must be non-negative and equal to the standard deviation across signals of the per-signal forecasts, or, when only one signal is usable, the 20th percentile of the absolute values of `mu` for every asset. | Arch D17; *Markowitz at Seventy* p23 | `test_calibrator.py::test_rho_dispersion_and_fallback` |
| R-087 | A signal whose scores are all NaN must be dropped and the remaining weights renormalised to sum to 1. | Arch 5.4 | `test_calibrator.py::test_nan_signal_dropped` |
| R-088 | When no signal has a usable score the calibrator must raise `NoForecastError`. | Arch 5.4, 5.6 | `test_calibrator.py::test_no_usable_signal_raises` |

### D. Risk and cost models

Owns `src/atp/models/`, `tests/models/`. Test paths are under `tests/models/`.

| ID | Requirement | Source | Acceptance check |
| --- | --- | --- | --- |
| R-089 | The volatility estimator must be the exponentially weighted variance of daily returns with a centre of mass of 60 sessions (decay 60/61), using returns up to the last session visible in the view. | *Time Series Momentum* p13 (eq. 1) | `test_risk.py::test_ewma_volatility_closed_form` (to 1e-12) |
| R-090 | The EWMA covariance estimator (default half-life 125 sessions) must return a symmetric matrix whose smallest eigenvalue is not below -1e-10 times the largest. | Arch 4 (D), 7.1 D; *Markowitz at Seventy* p22, p32 | `test_risk.py::test_ewma_covariance_psd_and_closed_form` |
| R-091 | The statistical factor model (default 15 factors) must return loadings, a factor covariance and non-negative idiosyncratic variances whose implied covariance is positive semidefinite and, for 100 assets and 2,000 sessions drawn from a known 3-factor model and fitted with 3 factors, within 25% of the true covariance in Frobenius norm. | Arch 5.5; *Markowitz at Seventy* p20; *Statistical Arbitrage* p6 | `test_risk.py::test_factor_model_recovers_known_structure` |
| R-092 | A `RiskEstimate` must carry either a full covariance or the factor triple, never both and never neither, and `sigma` must equal the square root of the implied diagonal to 1e-9. | Arch 5.5 | `test_risk.py::test_risk_estimate_consistent` |
| R-093 | The output of every `RiskModel.estimate` and of `CostModel.params` at `as_of` must be unchanged when the store is randomized after `as_of`. | Arch D5; *Time Series Momentum* p13 | `test_risk.py::test_no_lookahead` |
| R-094 | `RiskModel.estimate` must raise `ModelError` naming every asset of `universe` with fewer than 60 sessions of returns. | Arch 5.5; N | `test_risk.py::test_short_history_raises` |
| R-095 | `trade_cost` must equal `half_spread * abs(x) + impact_coeff * sigma * abs(x)**1.5 / adv_dollars**0.5` per asset, for dollar trade x, to a relative error of 1e-12. | Arch 5.5, D11; *Multi-Period Trading* p12 (eq. 2.2) | `test_cost.py::test_trade_cost_formula` |
| R-096 | `trade_cost` must be zero for a zero trade, equal for a buy and a sell of the same size, and never negative. | *Multi-Period Trading* p12 | `test_cost.py::test_zero_symmetric_nonnegative` (property test) |
| R-097 | With zero half-spread, `trade_cost` of four times a trade must be eight times the cost of the trade, to 1e-9. | Arch 7.1 D; *Strict Universality* p1 | `test_cost.py::test_three_halves_power` |
| R-098 | A rate given as a fraction must be applied as given: a half-spread of 0.0018 must cost exactly 0.0018 times the absolute trade. | Arch D11; *Implementation Risk* p32 | `test_cost.py::test_fraction_not_rescaled` |
| R-099 | `trade_cost_expr(z, nav, p)` must be the cost as a fraction of portfolio value, so that its value times `nav` equals the sum of `trade_cost(z * nav, p)` to a relative error of 1e-8 for random inputs. | Arch 5.5, D11; *Multi-Period Trading* p13 (eq. 2.3) | `test_cost.py::test_expression_matches_numeric` (property test) |
| R-100 | `trade_cost_expr` must be a convex expression accepted by cvxpy's DCP rules. | *Markowitz at Seventy* p24 | `test_cost.py::test_expression_is_dcp_convex` |
| R-101 | `holding_cost` must equal the sum of `short_fee` times the short dollar holdings plus `cash_borrow_rate` times the negative part of cash, with per-session rates, and must be zero for a long-only portfolio with non-negative cash. | Arch 5.5; *Markowitz at Seventy* p15 (eq. 5) | `test_cost.py::test_holding_cost` |
| R-102 | `CostModel.params` must return `sigma` from the estimator of R-089, `adv_dollars` as the median dollar volume of the trailing 60 sessions, `half_spread` from configuration (default 0.0005) and `impact_coeff` from configuration (default 1.0). | Arch 5.5, D11, C10; I12; N | `test_cost.py::test_params` |
| R-103 | For an asset with `adv_dollars` not above zero, `trade_cost` must return infinity for a non-zero trade and zero for a zero trade, never NaN. | Arch 5.5 | `test_cost.py::test_zero_volume_asset` |
| R-104 | A cost scale X must multiply every cost component by exactly X: for fixed trades and holdings, `trade_cost` and `holding_cost` at scale X must equal X times their values at scale 1. | Arch 5.9 (gate 6), 5.11 (`--cost-scale`) | `test_cost.py::test_cost_scale` |

### E. Portfolio construction

Owns `src/atp/portfolio/`, `tests/portfolio/`. Test paths are under `tests/portfolio/`.

| ID | Requirement | Source | Acceptance check |
| --- | --- | --- | --- |
| R-105 | Every allocator must return weights indexed by the assets of its input with the sum of weights plus `cash_weight` equal to 1 to 1e-8, and `diagnostics` holding solve time and every soft-constraint violation. | Arch 5.1, 5.6; *Markowitz at Seventy* p13 | `test_contract.py::test_budget_and_diagnostics` (every allocator) |
| R-106 | `MarkowitzAllocator` must satisfy, to 1e-6, every constraint of `Constraints` that is not listed in `soft`: long-only, weight bounds, cash bounds, leverage, turnover, participation and risk target. | Arch 5.6; *Markowitz at Seventy* p24 (eq. 19) | `test_markowitz.py::test_hard_constraints_hold` (property test over random feasible problems) |
| R-107 | Turnover must be half the sum of absolute trades in weight terms, and the participation limit must bound each asset's absolute dollar trade by `participation_max` times its forecast `adv_dollars`. | *Markowitz at Seventy* p16 (eq. 8-9) | `test_markowitz.py::test_turnover_and_participation_definitions` |
| R-108 | The trade of every asset in `Constraints.restricted` must be zero. | Arch 5.6 | `test_markowitz.py::test_restricted_assets_not_traded` |
| R-109 | With two assets, no costs, no robustness terms and only a risk target, the solution must match the analytic one, `sigma_target * inv(Sigma) @ mu / sqrt(mu @ inv(Sigma) @ mu)`, to 1e-5. | Arch 7.1 E | `test_markowitz.py::test_analytic_two_asset_case` |
| R-110 | The return term must be `mu @ w - rho_scale * rho @ abs(w)`: when `rho` exceeds `abs(mu)` for every asset and `rho_scale` is 1, the solution must hold only cash. | *Markowitz at Seventy* p23 (eq. 16) | `test_markowitz.py::test_robust_return_all_cash` |
| R-111 | The risk term must be the worst-case risk of *Markowitz at Seventy* eq. 17 with uncertainty `cov_uncertainty`, and the `sigma_wc` diagnostic must equal that formula evaluated at the solution to 1e-8. | *Markowitz at Seventy* p23-24 (eq. 17) | `test_markowitz.py::test_worst_case_risk` |
| R-112 | Raising `gamma_trade` with everything else fixed must never raise turnover. | Arch 7.1 E; *Multi-Period Trading* p44 | `test_markowitz.py::test_trade_aversion_monotone` (grid of five values, tolerance 1e-6) |
| R-113 | A constraint listed in `soft` must be replaced by a penalty of its priority times its violation; a problem that is infeasible with that constraint hard must be solved with `status="relaxed"` and the violation reported in `diagnostics`. | Arch 5.6; *Markowitz at Seventy* p26 | `test_markowitz.py::test_soft_constraint_relaxed` |
| R-114 | Listing the budget, long-only or minimum-cash constraint in `soft` must raise `ConfigError`. | *Markowitz at Seventy* p26 | `test_markowitz.py::test_hard_only_constraints` |
| R-115 | `status` must be `optimal` when no soft constraint is violated by more than 1e-6, `relaxed` when one is, and `fallback_hold` when no solution was obtained, in which case the weights and cash returned must equal `w_pre` and `cash_pre` exactly. | Arch 5.1, 5.6 | `test_markowitz.py::test_status_semantics` |
| R-116 | `MarkowitzAllocator.allocate` must never raise: a solver error, an infeasible problem, a solve longer than 10 seconds or NaN in its inputs must each produce `fallback_hold` with a non-zero `fallback_code` in `diagnostics`. | Arch 5.6; N | `test_markowitz.py::test_never_raises` (four cases) |
| R-117 | `HRPAllocator` must return non-negative weights that sum to 1, including for a singular covariance matrix (50 assets estimated from 30 sessions). | Arch 7.1 E; *Building Diversified Portfolios* p2, p5 | `test_baselines.py::test_hrp_properties` |
| R-118 | For a diagonal covariance matrix `HRPAllocator` must return inverse-variance weights to 1e-9. | *Building Diversified Portfolios* p7-8 | `test_baselines.py::test_hrp_diagonal_case` |
| R-119 | The inverse-volatility allocator must return weights proportional to the reciprocal of `sigma`, summing to 1. | Arch 4 (E), C1 | `test_baselines.py::test_inverse_volatility` |
| R-120 | The equal-weight allocator must return the same weight for every asset that is not restricted, summing to 1. | Arch 4 (E), C1 | `test_baselines.py::test_equal_weight` |
| R-121 | The HRP, inverse-volatility and equal-weight allocators must accept `forecast=None` and return the same weights whatever forecast is passed. | Arch C1, D9 | `test_baselines.py::test_forecast_ignored` |
| R-122 | After solving, a trade whose absolute notional is below `min_trade_notional` (default 0) must be set to zero with the difference absorbed by cash. | Arch 4 (E: rounding heuristics); *Markowitz at Seventy* p27; I21 | `test_markowitz.py::test_small_trades_removed` |
| R-123 | The median wall time of `MarkowitzAllocator.allocate` over 100 consecutive synthetic sessions with 500 assets, 15 factors and default constraints must be at most 0.15 seconds on one core. | Arch 3 (Performance); *Multi-Period Trading* p40; I20; N | (slow) `test_perf.py::test_allocate_median_time` |

### F. Engine and simulation

Owns `src/atp/engine/`, `tests/engine/`. Test paths are under `tests/engine/`.

| ID | Requirement | Source | Acceptance check |
| --- | --- | --- | --- |
| R-124 | For each session the engine must emit events in this order: `SessionOpen`, `CorporateActionApplied` (if any), `FillReceived` (for orders decided the session before), `BarClosed`, `SessionClose`, `DataReady`, `Decision`, `OrdersSubmitted`. | Arch 5.2 (daily cycle) | `test_events.py::test_event_order` (three-session run) |
| R-125 | The engine must give the strategy a view whose `as_of` is the session's decision instant (R-013), and nothing else that reads data. | Arch 5.2, 5.8, D5 | `test_events.py::test_decision_view_as_of` |
| R-126 | The simulated broker must fill an order decided after the close of session T at the raw open price of session T+1, with `filled_at` equal to that open and therefore later than the order's `decision_as_of`. | Arch 5.2, D4; *LLM Trading Agents Earn Beta* p4 | `test_sim_broker.py::test_fill_at_next_open` |
| R-127 | The simulated broker must take every cost from the injected `CostModel` and from nowhere else: the costs recorded in `RunResult` must equal the sum of the `trade_cost` and `holding_cost` values returned during the run to 1e-9. | Arch D11; *Implementation Risk* p34 (double deduction) | `test_sim_broker.py::test_single_cost_owner` |
| R-128 | The simulated broker must compute a fill's cost with `adv_dollars` equal to the realized dollar volume of the fill session, not the forecast used by the optimizer. | Arch 5.5; *Markowitz at Seventy* p29 | `test_sim_broker.py::test_realized_volume_in_fill_cost` |
| R-129 | On every session the ledger must satisfy the self-financing identity: the change in portfolio value equals the gain on holdings minus trading cost minus holding cost, to a relative error of 1e-9. | Arch 7.1 F; *Multi-Period Trading* p17-18 | `test_ledger.py::test_self_financing_identity` (property test) |
| R-130 | On a split ex-date the ledger must scale the share quantity by the split ratio so that equity is unchanged by the split itself. | Arch 5.2 (`CorporateActionApplied`), 5.8 | `test_ledger.py::test_split` |
| R-131 | With dividends enabled, the equity of a buy-and-hold position in one asset must follow that asset's fully adjusted close to 1e-9 across a cash dividend. | Arch 5.8 ("total returns"); *Multi-Period Trading* p17 | `test_ledger.py::test_dividend_total_return` |
| R-132 | With dividends disabled, the equity of a buy-and-hold position must follow the split-adjusted close to 1e-9 across a cash dividend. | Arch 5.7 (`reconcile`), D13 | `test_ledger.py::test_dividends_disabled` |
| R-133 | With fractional shares, zero cost and a synthetic panel whose open equals the previous close, the engine's equity series must equal `reference_backtest` on the same target weights and prices to a relative error of 1e-9 on every session, for the equal-weight, inverse-volatility and HRP allocators. | Arch 7.1 F; *Implementation Risk* p41; I11 | `test_reference.py::test_engine_matches_reference_at_zero_cost` (three allocators) |
| R-134 | `reference_backtest` must implement Algorithm 1 of *Implementation Risk* line by line. | Arch 5.8; *Implementation Risk* p43-44 | `test_reference.py::test_hand_computed_example` (weights [[0.5, 0.5], [1, 0]], prices [[10, 20], [11, 20]], capital 1000, cost rate 0.001 give equity [999.0, 1047.951] to 1e-9) |
| R-135 | The module that defines `reference_backtest` must import nothing but `numpy` and `pandas`. | Arch D2, D3 (test oracle); *Implementation Risk* p45 | `test_reference.py::test_reference_is_independent` (AST scan) |
| R-136 | For a fixed schedule of target weights, final equity must not increase when the cost scale increases. | Arch 7.1 F; *Implementation Risk* p1 | `test_reference.py::test_net_return_monotone_in_cost` (scales 0, 0.5, 1, 2) |
| R-137 | `RunResult` must record the sessions processed and the sessions in the calendar for the run's range, and a run in which they differ must end with status `failed` and raise. | Arch 5.8; *Implementation Risk* p34 (62 of 1,258 days) | `test_engine.py::test_completeness_check` |
| R-138 | The simulated broker must apply the sells of a batch before its buys, so that a rebalance from one fully invested asset to another fills completely. | *Implementation Risk* p32-33 | `test_sim_broker.py::test_sells_fund_buys` |
| R-139 | `SimBroker.submit` must be idempotent on `client_order_id`: a second submit of the same id returns the first order's state and creates no second fill. | Arch 5.7 | `test_sim_broker.py::test_submit_idempotent` |
| R-140 | The simulated broker must fill at most `participation_max` times the fill session's realized volume of an asset and record the remainder as unfilled. | Arch 4 (F: fill models); *Markowitz at Seventy* p16; N | `test_sim_broker.py::test_participation_cap` |
| R-141 | An order for an asset with no bar on the fill session must expire unfilled and be recorded as such. | *Multi-Period Trading* p20 (trading freeze) | `test_sim_broker.py::test_no_bar_no_fill` |
| R-142 | When a held asset has no further bars and is inactive, the ledger must close the position at its last available raw close on the next session, credit the cash, emit an event and count the closure in `RunResult`. | Arch D7; *Multi-Period Trading* p20; I17 | `test_ledger.py::test_delisted_position_closed` |
| R-143 | When the execution schedule is an auction, every simulated order and position quantity must be a whole number of shares computed from raw prices. | Arch 5.8, D4 | `test_engine.py::test_whole_share_mode` |
| R-144 | The engine must depend only on the `Broker` protocol: for identical data and account state it must produce identical `OrderRequest` lists against the simulated broker and against a fixture-backed paper broker. | Arch D3, 5.2 | `tests/e2e/test_parity.py::test_same_orders_sim_and_paper` |
| R-145 | Calling `Engine.step` twice for the same session must submit no order the first call did not submit. | Arch 5.8, D19 | `test_engine.py::test_step_idempotent` |
| R-146 | The engine must run a synthetic `1Min` calendar and bar set with a change of configuration only, processing exactly as many bars as the calendar holds. | Arch 3 (Scope), 7.1 F | `test_engine.py::test_minute_bars_run` |
| R-147 | `RunResult` must hold the fields listed in Arch 5.8, and the run must be written to `$ATP_HOME/runs/<id>/` with the configuration snapshot, decisions, orders, fills, equity and metrics. | Arch 5.8, 5.12 | `test_recorder.py::test_run_result_and_directory` |
| R-148 | Every recorded run must carry `config_hash`, the code revision (with a marker when the working tree has uncommitted changes), the data snapshot id and the seed. | Arch 5.9, D12 | `test_recorder.py::test_run_metadata` |
| R-149 | Two runs with the same configuration, data snapshot and seed must produce identical orders and an identical equity series. | Arch 3 (deterministic) | `test_engine.py::test_deterministic_run` |
| R-150 | `RunResult` must report costs split into spread, impact, short fee and cash borrow, and the parts must sum to the total to 1e-9. | Arch 5.8 | `test_recorder.py::test_cost_components` |
| R-151 | The engine must pass every planned order through the `RiskGate` and submit to the broker only orders the gate approved. | Arch D14, 5.8 | `test_engine.py::test_only_approved_orders_submitted` |
| R-152 | A ten-year daily backtest of a 500-asset synthetic universe with the default pipeline (`tsmom` and two characteristics, factor risk model, `MarkowitzAllocator`) must finish in at most 600 seconds on one core. | Arch 3 (Performance); I20 | (slow) `test_perf.py::test_ten_year_backtest_time` |
| R-153 | The backtest of R-152 must use at most 4 GB of peak resident memory. | Arch D6 ("fits in memory"); N | (slow) `test_perf.py::test_ten_year_backtest_memory` |

### G. Execution

Owns `src/atp/execution/`, `tests/execution/`. Test paths are under `tests/execution/`.

| ID | Requirement | Source | Acceptance check |
| --- | --- | --- | --- |
| R-154 | The planner must return every sell before any buy. | Arch 5.7; *Implementation Risk* p33 | `test_planner.py::test_sells_before_buys` |
| R-155 | For an auction schedule the planner must size each order in whole shares from the raw reference price, rounding toward zero, and must emit no order that rounds to zero shares. | Arch 5.7, D4; I21 | `test_planner.py::test_whole_share_rounding` |
| R-156 | The total notional of the planned buys at reference prices must not exceed `account.buying_power` times (1 minus `buying_power_buffer`, default 0.02), without counting proceeds of the planned sells; when it would, all buys must be scaled down by one common factor before rounding. | Arch 3 (buying power), 5.7; N | `test_planner.py::test_buys_fit_buying_power` (property test) |
| R-157 | The planner must be deterministic: the same inputs must give the same orders with the same `client_order_id` values, and each order's `symbol` must be the ticker valid at the decision instant. | Arch 5.1, 5.7 | `test_planner.py::test_deterministic_ids_and_symbols` |
| R-158 | With the `DayMarket` schedule the planner must emit fractional quantities only for assets flagged `fractionable` and whole shares for the others. | Arch 3 (Alpaca compatibility), Q5 | `test_planner.py::test_fractional_only_when_fractionable` |
| R-159 | `SingleAuction("opg")` and `SingleAuction("cls")` must each return one slice for the next session with that time-in-force, and `DayMarket` one slice with time-in-force `day`. | Arch 5.7 | `test_schedule.py::test_first_version_schedules` |
| R-160 | The paper adapter must refuse to submit an `opg` order between 09:28 and 19:00 ET and a `cls` order between 15:50 and 19:00 ET, raising `SubmissionWindowError` without calling the API. | Arch 2.2 (auction cut-offs), 3 | `test_alpaca_broker.py::test_submission_windows` (injected clock) |
| R-161 | `RiskGate.check` must be pure: the same inputs must give the same `GateResult`, and it must perform no file, network or database access. | Arch 5.7, D14; *Agentic Trading* p35 | `test_risk_gate.py::test_pure_and_deterministic` |
| R-162 | The gate must reject an order whose notional at the reference price exceeds `max_order_notional`. | Arch 5.7; I22 | `test_risk_gate.py::test_max_order_notional` |
| R-163 | The gate must reject an order that would raise the absolute weight of a position above `max_position_weight`, and must approve an order that only reduces such a position. | Arch 5.7; *Agentic Trading* p26; I22 | `test_risk_gate.py::test_max_position_weight` |
| R-164 | When the batch, taken as fully filled at reference prices, would leave gross leverage above `max_gross_leverage`, the gate must reject every order that increases an absolute position and approve those that reduce one. | Arch 5.7; I22 | `test_risk_gate.py::test_max_gross_leverage` |
| R-165 | When the batch would leave net leverage outside plus or minus `max_net_leverage`, the gate must reject every order that moves net exposure further from zero. | Arch 5.7; I22 | `test_risk_gate.py::test_max_net_leverage` |
| R-166 | The gate must reject the whole batch when its turnover (half the sum of absolute order notionals over equity) exceeds `max_daily_turnover`. | Arch 5.7; I22 | `test_risk_gate.py::test_max_daily_turnover` |
| R-167 | The gate must reject the whole batch when it holds more than `max_orders` orders. | Arch 5.7; I22 | `test_risk_gate.py::test_max_orders` |
| R-168 | The gate must reject an order whose reference price differs from the asset's previous-session close by more than `price_collar_pct`, or whose limit price differs from the reference price by more than it. | Arch 5.7; I22 | `test_risk_gate.py::test_price_collar` |
| R-169 | When account equity is below the previous session's closing equity by more than `max_daily_loss_pct`, the gate must reject every order and return `kill=True`. | Arch 5.7, Q10; *Agentic Trading* p26 | `test_risk_gate.py::test_daily_loss_trips_kill` |
| R-170 | When account equity is below the high-water mark in `RiskState` by more than `max_drawdown_pct`, the gate must reject every order and return `kill=True`. | Arch 5.7, Q10; *Agentic Trading* p26 | `test_risk_gate.py::test_drawdown_trips_kill` |
| R-171 | The gate must reject an order for an asset that is in `restricted` or is not flagged `tradable`. | Arch 5.7 | `test_risk_gate.py::test_restricted_and_untradable` |
| R-172 | The gate must reject a sell that would open or increase a short position unless the asset is flagged `shortable` and `easy_to_borrow` and account equity is at least $2,000. | Arch 3 (Alpaca compatibility), D20 | `test_risk_gate.py::test_short_sale_rules` |
| R-173 | While the kill switch is on, the gate must reject every order except orders produced by the flatten path of R-178. | Arch 5.7, D14 | `test_risk_gate.py::test_kill_switch_blocks_orders` |
| R-174 | The gate must fail closed: a missing or NaN price, a missing account snapshot or any internal error must reject the affected orders, never approve them. | Arch D14; *Agentic Trading* p35 | `test_risk_gate.py::test_fail_closed` |
| R-175 | Every field of `RiskLimits` must be given in configuration; a strategy configuration that omits one must be rejected with a `ConfigError` naming the field. | Arch 5.7; Q10 | `test_risk_gate.py::test_limits_are_mandatory` |
| R-176 | The kill switch must be a row in `state.sqlite` that survives a process restart, is set by the gate or by an explicit engage operation, and is cleared only by an explicit manual clear. | Arch 5.7 | `test_kill_switch.py::test_persistence_and_manual_clear` |
| R-177 | Engaging the kill switch must cancel every open order at the broker. | Arch 5.7, 5.11; *Agentic Trading* p26 ("halt order flow") | `test_kill_switch.py::test_kill_cancels_open_orders` |
| R-178 | The flatten operation of the kill switch must submit, for every open position, one order that closes it and no order that opens or increases a position. | Arch 5.11 | `test_kill_switch.py::test_flatten_only_reduces` |
| R-179 | The Alpaca broker adapter must refuse to be constructed with any trading base URL other than `https://paper-api.alpaca.markets`, whatever the configuration or environment says. | Arch 3 (Paper only), D21 | `test_alpaca_broker.py::test_non_paper_url_refused`; `grep -rn "//api.alpaca.markets" src` prints nothing |
| R-180 | Before submitting an order the adapter must look up its `client_order_id` and, when the order already exists, return its state without submitting again. | Arch 5.7 | `test_alpaca_broker.py::test_submit_idempotent` |
| R-181 | When the API rejects a submit because the `client_order_id` already exists, the adapter must treat the order as already submitted and fetch its state. | Arch 5.7, gap 10 (behaviour unverified; fixture hand-written) | `test_alpaca_broker.py::test_duplicate_id_rejection` |
| R-182 | An HTTP 403 on submit must be recorded as a rejection for insufficient buying power or shares and an HTTP 422 as an invalid order; in both cases the remaining orders of the batch must still be submitted. | Arch 2.2 | `test_alpaca_broker.py::test_rejections_do_not_stop_batch` |
| R-183 | Every order and every change of its status must be stored in the `orders` table of `state.sqlite`. | Arch 5.12, D19 | `test_order_state.py::test_orders_persisted` |
| R-184 | Polling fills twice must store each fill once in the `fills` table, and the fills of a partially filled order must sum to its filled quantity. | Arch 2.2 (partial fills), 5.7 | `test_order_state.py::test_fills_deduplicated_and_partial` |
| R-185 | In paper mode order sizes must be computed from the positions and account reported by the broker at decision time, not from earlier targets. | Arch 5.7, 5.8 (`Strategy.decide`) | `test_planner.py::test_sizes_from_broker_state` |
| R-186 | `reconcile` must report a break for every asset whose broker quantity differs from the recorded quantity and for a cash difference above $0.01. | Arch 5.7; N | `test_reconcile.py::test_position_and_cash_breaks` |
| R-187 | `reconcile` must replay the broker's actual fills through a ledger received by injection, on split-adjusted prices, with dividends disabled and no modelled trading cost. | Arch 5.7, D13; *Multi-Period Trading* p21; I7 | `test_reconcile.py::test_shadow_replay` |
| R-188 | The reconciliation report must give, per session, the paper return, the shadow return, their difference and the cumulative divergence, and, per fill, the difference between the fill price and the session open in basis points. | Arch 5.7 | `test_reconcile.py::test_report_fields` |
| R-189 | Running `reconcile` twice for one session must produce the same report and no duplicate rows in `state.sqlite`. | Arch D19 | `test_reconcile.py::test_idempotent` |
| R-190 | An opt-in test module must check against a real paper account the behaviours of Arch gap 10 that the code relies on: SIP history access, the response to a duplicate `client_order_id`, and the calendar endpoint. | Arch gap 10, 7.1 G | (alpaca) `pytest -m alpaca tests/execution/test_alpaca_contract.py` exits 0 |

### H. Validation

Owns `src/atp/validation/`, `tests/validation/`. Test paths are under `tests/validation/`.

| ID | Requirement | Source | Acceptance check |
| --- | --- | --- | --- |
| R-191 | The trial registry must offer no operation that deletes a trial or changes one other than closing it once, and the `trials` table must reject an SQL `DELETE` and any `UPDATE` of a closed trial. | Arch 5.9, D12; Q13 | `test_registry.py::test_append_only` |
| R-192 | A trial must be opened before its run starts, so that a run that crashes leaves an open trial in the registry. | Arch 5.11, D12; *Deflated Sharpe Ratio* p3 | `test_registry.py::test_crashed_run_stays_registered` |
| R-193 | A trial record must hold family, role (`candidate`, `baseline` or `diagnostic`), `config_hash`, code revision, data snapshot id, seed, opening time and status. | Arch 5.9; I2 | `test_registry.py::test_trial_record_fields` |
| R-194 | Opening a trial on a synthetic data snapshot in a family that already holds real-data trials, or the reverse, must raise `RegistryError`. | Arch 5.11 (`--synthetic`), D12 | `test_registry.py::test_synthetic_and_real_not_mixed` |
| R-195 | `close` must store the trial's per-session net returns, metrics and final status, and a second `close` of the same trial must raise. | Arch 5.9 | `test_registry.py::test_close_once` |
| R-196 | `family_matrix` must return the net returns of the closed `candidate` trials of a family over the sessions common to all of them, with no missing value. | Arch 5.9; *Probability of Backtest Overfitting* p12 (condition i); I2 | `test_registry.py::test_family_matrix_synchronous` |
| R-197 | The metrics functions must compute mean return, volatility, Sharpe ratio, maximum drawdown, turnover, leverage and total cost, matching hand-computed values on a fixture to 1e-12. | Arch 4 (H); *Multi-Period Trading* p22-23; *Markowitz at Seventy* p29 | `test_metrics.py::test_known_values` |
| R-198 | The Sharpe ratio must be computed on returns in excess of the configured cash-rate series (zero when none is configured, which the report must state) and annualized with 252 periods per year unless configured otherwise. | *Multi-Period Trading* p23; I12; N | `test_metrics.py::test_sharpe_excess_and_annualization` |
| R-199 | `probabilistic_sharpe` must implement the statistic of *Deflated Sharpe Ratio* eq. 2 for a given threshold, using the sample length and the skewness and kurtosis of the returns. | Arch 5.9; *Deflated Sharpe Ratio* p8 | `test_dsr.py::test_psr_formula` (to 1e-10 against the formula evaluated from sample moments) |
| R-200 | `deflated_sharpe` must reproduce the worked example of *Deflated Sharpe Ratio* p9-10: an annualized Sharpe ratio of 2.5 on 1,250 daily observations gives 0.9505 at 46 independent trials and about 0.90 at the paper's own trial count. | Arch 7.1 H; *Deflated Sharpe Ratio* p9-10; I19 | `test_dsr.py::test_paper_example` (0.9505 within 0.0005 at N = 46 and 0.9004 within 0.0005 at N = 100, with variance of annualized Sharpe ratios 0.5, skewness -3, kurtosis 10; these inputs must be confirmed against the PDF, RQ2) |
| R-201 | `effective_trials` must return `rho + (1 - rho) * M`, where `rho` is the mean off-diagonal correlation of the M trial return columns, limited to the range 1 to M. | Arch 5.9; *Deflated Sharpe Ratio* p14 (eq. 9) | `test_dsr.py::test_effective_trials` (identical columns give 1; 20 independent columns of 5,000 rows give a value within 1 of 20) |
| R-202 | The trial count used for gate 3 must be `effective_trials` of the closed `candidate` trials plus one for every `candidate` trial without a return series, and when that count is 1 the deflated Sharpe ratio must equal the probabilistic Sharpe ratio against zero. | Arch 5.11 ("abandoned runs are counted"); I3 | `test_gates.py::test_trial_count_and_single_trial_case` |
| R-203 | `pbo_cscv` with S blocks must evaluate all C(S, S/2) splits (12,870 for S = 16) and return as PBO the share of splits in which the logit of the out-of-sample relative rank of the in-sample best trial is zero or negative, ranking by Sharpe ratio. | Arch 5.9; *Probability of Backtest Overfitting* p12-14; I19 | `test_pbo.py::test_split_count_and_definition` (hand-computed case at S = 4; count at S = 16) |
| R-204 | For 50 independent pure-noise trials of 1,024 sessions, `pbo_cscv` must return a PBO of at least 0.2, four times the gate limit. | Arch 7.1 H; *Probability of Backtest Overfitting* p26-27 | `test_pbo.py::test_noise_family_is_overfit` (five fixed seeds; a trial computation over ten seeds gave 0.29 to 0.76) |
| R-205 | For the same family with one trial given a mean of 0.3 standard deviations per session, `pbo_cscv` must return a PBO of at most 0.05. | Arch 7.1 H; *Probability of Backtest Overfitting* p17, p27 | `test_pbo.py::test_planted_signal_is_not_overfit` (five fixed seeds; a trial computation gave 0.00) |
| R-206 | `PBOResult` must hold the PBO, the logits, the slope of out-of-sample on in-sample performance of the selected trials, and the share of splits in which the selected trial loses out of sample; `pbo_cscv` must raise `ValueError` for fewer than 2 trials or fewer than 2 x S rows. | Arch 5.9; *Probability of Backtest Overfitting* p14-15 | `test_pbo.py::test_result_fields_and_input_validation` |
| R-207 | `pbo_cscv` on a matrix of 2,740 sessions and 50 trials with S = 16 must finish in at most 60 seconds on one core. | Arch 3 (Performance); N | (slow) `test_perf.py::test_pbo_time` |
| R-208 | `adjust_pvalues` must reproduce Table 4 of *And the Cross-Section*: for p-values (in percent) 4.66, 0.85, 2.71, 0.05, 3.00, 0.84, 0.00, 0.00, 0.60, 1.28 at the 5% level, Bonferroni must find tests 4, 7, 8, Holm tests 4, 7, 8, 9 and BHY tests 2, 4, 6, 7, 8, 9. | Arch 7.1 H; *And the Cross-Section* p15-18 | `test_multiple_testing.py::test_table_4` |
| R-209 | `attribute` must return the intercept, its t-statistic, the factor loadings and R-squared of an ordinary least-squares regression of returns on the factor returns. | Arch 5.9; *LLM Trading Agents Earn Beta* p6 | `test_attribution.py::test_matches_least_squares` (to 1e-10 against `numpy.linalg.lstsq`) |
| R-210 | Factor returns for attribution must be built from the ETFs named in configuration (long leg, optional short leg) using returns from the store, and gate 5 must be not evaluable when none are configured. | Arch 5.9 (factor proxies) | `test_attribution.py::test_factor_proxies_from_config` |
| R-211 | `leak_check` must report a leak for each of these planted defects: a signal that reads one session ahead, news joined by calendar date, and timestamps shifted four hours early. | Arch 5.9, 7.1 H; *LLM Trading Agents Earn Beta* p2, p13 | `test_leak_check.py::test_planted_leaks_caught` (three cases) |
| R-212 | `leak_check` must place its cuts on decision days (by default 20 drawn with the given seed, or all when fewer exist) and must report no leak for the shipped pipeline on synthetic data. | *LLM Trading Agents Earn Beta* p2, p13; N | `test_leak_check.py::test_cuts_on_decision_days_and_clean_pipeline` |
| R-213 | `evaluate_gates` must return and store a report giving, for each gate, a status (`pass`, `fail`, `not_evaluable` or `not_applicable`), the measured values, the thresholds and the trial counts; the strategy passes only when every gate is `pass` or `not_applicable`. | Arch 5.9 | `test_gates.py::test_report_structure_and_overall_result` |
| R-214 | Gate 1 must pass only when the leak check is clean, the data check has no error-level finding in the run's range, and sessions processed equal sessions in the calendar. | Arch 5.9 (gate 1) | `test_gates.py::test_gate_1` |
| R-215 | Gate 2 must pass only when the selected trial's net Sharpe ratio exceeds that of every baseline named in its configuration snapshot, which must include equal-weight, inverse-volatility and time-series momentum, each run as a `baseline` trial on the same data snapshot, sessions and cost settings; a missing baseline makes the gate not evaluable. | Arch 5.9 (gate 2); *Seemingly Virtuous Complexity* p2 | `test_gates.py::test_gate_2` |
| R-216 | For a strategy that uses a `Predictor` or a signal with `origin="pretrained_model"`, gate 2 must also fail when the same configuration run on the pure-noise panel has a probabilistic Sharpe ratio against zero of 0.95 or more. | Arch C3; *Seemingly Virtuous Complexity* p2-3; N | `test_gates.py::test_placebo_run` |
| R-217 | Gate 3 must require a deflated Sharpe ratio of at least 0.95, computed with the trial count of R-202 and the variance of the Sharpe ratios of the closed `candidate` trials. | Arch 5.9 (gate 3); *Deflated Sharpe Ratio* p8-10 | `test_gates.py::test_gate_3_dsr` |
| R-218 | Gate 3 must also require a t-statistic of at least 3.0 for the mean of the selected trial's per-session net excess returns, the threshold for a strategy built only from `published` signals being a separate configuration value that defaults to 3.0. | Arch 5.9 (gate 3), C2; *And the Cross-Section* p2; I13 | `test_gates.py::test_gate_3_t_statistic` |
| R-219 | Gate 4 must require a PBO of at most 0.05 over the family matrix and report the degradation slope and probability of loss; it must be not evaluable for 2 to 19 `candidate` trials and not applicable for exactly one. | Arch 5.9 (gate 4); *Probability of Backtest Overfitting* p15, p23; I3; N | `test_gates.py::test_gate_4` |
| R-220 | Gate 5 must require that the intercept of R-209 has a t-statistic of at least the configured value (default 2.0), and the report must list every factor loading. | Arch 5.9 (gate 5) | `test_gates.py::test_gate_5` |
| R-221 | Gate 6 must require a positive net Sharpe ratio in a rerun at cost scale 2, registered as a `diagnostic` trial. | Arch 5.9 (gate 6), C10; *Markowitz at Seventy* p29 | `test_gates.py::test_gate_6_double_cost` |
| R-222 | Gate 6 must also require that, for each parameter listed as tuned in the configuration, reruns at 0.8 and 1.2 times its value have a net Sharpe ratio of the same sign as the selected trial, each registered as a `diagnostic` trial. | Arch 5.9 (gate 6); *Markowitz at Seventy* p30 | `test_gates.py::test_gate_6_parameter_sensitivity` |
| R-223 | Gate 7 must require at least 60 reconciled paper sessions, an absolute difference between paper and shadow return of at most 5 bps on every session and at most 25 bps cumulatively, and no unexplained reconciliation break. | Arch 5.9 (gate 7), D13; N | `test_gates.py::test_gate_7` |
| R-224 | Gates 3 to 5 must exclude every session at or before the `knowledge_cutoff` of any `pretrained_model` signal in the strategy and every session without an out-of-sample prediction from a `Predictor` in it. | Arch 5.9, D16; *Assessing Look-Ahead Bias* p2 | `test_gates.py::test_cutoff_sessions_excluded` |
| R-225 | For a strategy with a `pretrained_model` signal the report must add the mean and spread of the net Sharpe ratio over at least 5 seeds, the result of the `no_news` ablation, and the result before against after the cutoff; when any of these is missing, gates 3 to 5 must be not evaluable. | Arch 5.9; *LLM Trading Agents Earn Beta* p1, p8; *Look-Ahead-Bench* p1-2; N | `test_gates.py::test_pretrained_model_additions` |
| R-226 | The validation report must include a negative control: the percentile of the selected trial's net Sharpe ratio among 20 `diagnostic` reruns in which each session's scores are shuffled across assets. | Arch 4 (H); *Can Blindfolded LLMs* p2; N | `test_gates.py::test_negative_control_reported` |
| R-227 | A strategy's promotion state must be one of `research`, `candidate`, `paper`, `validated`; it must move to `candidate` only through a report passing gates 1 to 6, to `paper` on its first submitted paper order, and to `validated` only through gate 7. | Arch 5.9 (promotion states); I15 | `test_gates.py::test_promotion_transitions` |
| R-228 | `evaluate_gates` must take baselines and tuned parameters from the selected trial's stored configuration snapshot, take thresholds from the gate configuration it is given, and store that gate configuration with its hash in the report. | Arch 5.9 (gate 2: "before the run"), 5.11; I14 | `test_gates.py::test_threshold_and_baseline_provenance` |

### I. LLM signal plugin (optional)

Owns `src/atp/llm/`, `tests/llm/`. Test paths are under `tests/llm/`. Every test uses the plugin's deterministic fake provider.

| ID | Requirement | Source | Acceptance check |
| --- | --- | --- | --- |
| R-229 | With no `[llm]` section in the configuration, running the pipeline must not import `atp.llm`, and installing the package must not require any model-provider library. | Arch 3 (Scope, Dependencies), D15 | `test_plugin.py::test_not_imported_when_disabled` |
| R-230 | `LLMNewsSignal` must refuse to be constructed without a `ModelCard`, and its `spec` must carry `origin="pretrained_model"` and the card's `knowledge_cutoff`. | Arch 5.10, D16 | `test_plugin.py::test_model_card_required` |
| R-231 | A `ModelCard` must be stored in the `model_cards` table, and one with an empty `cutoff_source` must be rejected. | Arch 5.10, 5.12; *LLM Trading Agents Earn Beta* p4 | `test_plugin.py::test_model_card_persisted` |
| R-232 | A prompt must contain only news whose `available_at` is at or before the view's `as_of`. | Arch 5.10, D5; *LLM Trading Agents Earn Beta* p1 | `test_plugin.py::test_no_future_news_in_prompt` (prompts recorded by the fake provider) |
| R-233 | The anonymizer's output must contain, case-insensitively, no ticker and no company name of any asset known at the session, and no entry of the configured alias dictionary; each must be replaced by a neutral identifier. | Arch 5.10; *Assessing Look-Ahead Bias* p2; *Can Blindfolded LLMs* p1-2; I16 | `test_anonymizer.py::test_fixture_set_fully_masked` |
| R-234 | The mapping from asset to neutral identifier must be fixed by the seed: the same seed must give the same identifiers. | *LLM Trading Agents Earn Beta* p4 | `test_anonymizer.py::test_aliases_fixed_by_seed` |
| R-235 | A model response must be accepted only when it is exactly an object with `score` in -1, 0, 1 and `confidence` between 0 and 1; anything else must be discarded and counted. | Arch 5.10; *TradingAgents* p2 | `test_plugin.py::test_malformed_output_discarded` |
| R-236 | Responses must be cached under the hash of model id, prompt version, anonymized text and seed, and an identical request must be answered from the cache without calling the provider. | Arch 5.10 | `test_plugin.py::test_cache_hit` |
| R-237 | A backtest run on a warm cache must make no provider call and must return the same scores as the run that filled the cache. | Arch 5.10, D15 | `test_plugin.py::test_warm_cache_replay` |
| R-238 | Scores computed for a session at or before the model's `knowledge_cutoff` must be flagged as contaminated in the run record. | Arch 7.1 I, D16; *LLM Trading Agents Earn Beta* p4 | `test_plugin.py::test_pre_cutoff_sessions_flagged` |
| R-239 | An asset's score must be the confidence-weighted mean of the accepted item scores for its news made available after the previous decision instant and at or before `as_of`, and NaN when there is none. | Arch 5.10; I16 | `test_plugin.py::test_item_scores_aggregated` |
| R-240 | The ablation modes must behave as named: `no_news` sends no news; `swapped_news` gives each asset that has news the items of another such asset, by a seeded permutation that leaves no asset its own; `calendar_date_join` admits every item of the calendar date whatever its time. | Arch 5.10; *LLM Trading Agents Earn Beta* p8 | `test_ablation.py::test_modes` (three cases) |
| R-241 | Constructing the plugin for a paper run with an ablation mode other than `none` must raise `ConfigError`. | Arch 5.10 ("validation only") | `test_ablation.py::test_refused_in_paper_run` |
| R-242 | A provider error, a timeout, reaching the configured maximum number of calls, or exceeding the scoring time budget (default 300 seconds) must leave the unscored items as NaN and must not raise. | Arch 5.4 ("never blocks the daily job"); *Agentic Trading* p35; N | `test_plugin.py::test_provider_failure_and_budgets` (four cases) |
| R-243 | A prompt must contain only the anonymized headline and summary of news items, and the prompt version must be part of both the cache key and `config_hash`. | Arch 5.10, D22 | `test_plugin.py::test_prompt_content_and_version` |

### J. Operations

Owns `pyproject.toml`, `src/atp/cli/`, `src/atp/ops/`, `configs/`, `docs/runbook.md`, `tests/e2e/`, `.gitignore`. Test paths are under `tests/e2e/`.

| ID | Requirement | Source | Acceptance check |
| --- | --- | --- | --- |
| R-244 | `pyproject.toml` must define the package `atp` in a `src/` layout with `requires-python = ">=3.11"`, a `dev` extra and a console script `atp`. | Arch 3 (Dependencies), D1; Q1; N | On Python 3.12, `pip install -e ".[dev]"` exits 0 and `atp --help` exits 0 |
| R-245 | Runtime dependencies must be exactly `numpy`, `pandas`, `pyarrow`, `scipy`, `scikit-learn`, `cvxpy`, `alpaca-py` and `pydantic`, and development dependencies exactly `pytest`, `hypothesis`, `ruff` and `mypy`. | Arch 3 (Dependencies) | `test_packaging.py::test_dependency_allow_list` |
| R-246 | The repository must pass the linter. | Arch 7.2 | `ruff check .` exits 0 |
| R-247 | The repository must pass the formatter check. | Arch 7.2 | `ruff format --check .` exits 0 |
| R-248 | The package must pass the type checker with its configuration in `pyproject.toml`. | Arch 7.2 | `mypy src/atp` exits 0 |
| R-249 | A plain `pytest -q` must deselect tests marked `alpaca` or `slow`. | Arch 7.2 | `test_packaging.py::test_default_markers_exclude_alpaca_and_slow` |
| R-250 | The five steps of Arch 7.2 must together finish in at most 300 seconds on the CI runner. | Arch 3 (Performance) | Duration of the `Verify` step in CI is at most 300 s |
| R-251 | Tests selected by a plain `pytest -q` must not open a network connection. | Arch 3 (Performance: offline), 7.2 | `test_offline.py::test_socket_connect_blocked` (an autouse fixture makes any connect raise) |
| R-252 | Two runs of the smoke sequence of R-257 in separate fresh `ATP_HOME` directories must give identical equity series and identical gate values. | Arch 3 (Performance: deterministic) | `test_smoke.py::test_smoke_is_deterministic` |
| R-253 | The `atp` command must provide every command and flag of Arch 5.11. | Arch 5.11 | `test_cli.py::test_commands_and_flags` (each command's `--help` exits 0 and lists its flags) |
| R-254 | `atp` must exit 0 on success, 1 on failure (including a data check with an error-level finding), 2 on a usage or configuration error, 3 when a gate did not pass and 4 when blocked by the risk gate or kill switch. | Arch 5.11 | `test_cli.py::test_exit_codes` (five cases) |
| R-255 | `atp backtest run --synthetic SEED` must run on generated data with no stored data, no credentials and no network. | Arch 5.11, 7.2 | `test_cli.py::test_backtest_synthetic_offline` |
| R-256 | `--cost-scale X` must scale the costs charged by the simulator by X and leave the cost parameters seen by the optimizer unchanged. | Arch 5.11; *Markowitz at Seventy* p29; N | `test_cli.py::test_cost_scale_simulator_only` |
| R-257 | In a fresh `ATP_HOME`, `atp backtest run --config configs/smoke.toml --synthetic 7` must exit 0 and `atp validate --family smoke` must then exit 3 with a report in which gate 1 passes and gate 2 is not evaluable. | Arch 7.2 (step 5); I1 | `test_smoke.py::test_smoke_sequence` |
| R-258 | `PipelineStrategy` must build its parts from configuration by dotted path and call signals, calibrator, risk model, cost model and allocator in that order. | Arch 5.8, D22 | `test_pipeline.py::test_build_and_call_order` |
| R-259 | A signal that raises must be treated as returning NaN for every asset, must produce an alert, and must not stop the decision. | Arch 5.4 | `test_pipeline.py::test_signal_failure_does_not_block` |
| R-260 | When the calibrator raises `NoForecastError` and the allocator needs a forecast, the strategy must return the pre-trade weights with `status="fallback_hold"` and produce an alert. | Arch 5.4, 5.6 | `test_pipeline.py::test_no_forecast_holds` |
| R-261 | A configuration that names `MarkowitzAllocator` and no signal must be rejected with a `ConfigError`. | Arch C1 ("when a return forecast exists") | `test_pipeline.py::test_markowitz_requires_a_signal` |
| R-262 | `atp paper decide` must ingest the session's data, run the pipeline once, pass the planned orders through the risk gate and submit the approved orders for the next session. | Arch 5.2, 5.11 | `test_paper_jobs.py::test_decide_flow` (simulated broker) |
| R-263 | `atp paper decide --dry-run` must submit nothing and must write the planned orders and the gate result to the run directory. | Arch 5.11 | `test_paper_jobs.py::test_dry_run_submits_nothing` |
| R-264 | `atp paper decide` must submit nothing and exit 4 while the kill switch is on. | Arch 5.7, 5.11 | `test_paper_jobs.py::test_decide_blocked_by_kill_switch` |
| R-265 | A decide job started before its session's decision instant or after the submission cut-off for the next session must submit nothing, record the session as missed in `job_log` and exit 1. | Arch 5.2, 2.2 (auction cut-offs); Q11 | `test_paper_jobs.py::test_decide_outside_window` |
| R-266 | A decide job must submit nothing, produce an alert and exit 1 when bars for its session exist for less than 95% of the universe. | Arch 2.2 (data "may not be available immediately"), 7.1 B; *Implementation Risk* p13 (stale data); N | `test_paper_jobs.py::test_decide_refuses_incomplete_data` |
| R-267 | `atp paper decide` without `--dry-run` must exit 3 unless the configuration's `config_hash` belongs to a trial whose strategy has reached the `candidate` state. | Arch 1 ("run the survivors"), 5.9; I15 | `test_paper_jobs.py::test_decide_requires_candidate` |
| R-268 | A second `atp paper decide` for a session already decided must exit 0 and submit nothing. | Arch 5.7, 7.1 G, D19 | `test_paper_jobs.py::test_decide_rerun_submits_nothing` |
| R-269 | A job started while another job holds the lock on the same `ATP_HOME` must exit 1 without contacting the broker. | Arch D19; Q11 | `test_paper_jobs.py::test_concurrent_job_refused` |
| R-270 | `atp paper status` must print, as JSON, the account, positions, open orders, kill-switch state and the result of the last decide and reconcile jobs. | Arch 5.11 | `test_cli.py::test_paper_status_output` |
| R-271 | `atp kill on`, `atp kill off` and `atp kill status` must set, clear and print the kill switch of R-176, with `--reason` stored and `--flatten` invoking R-178. | Arch 5.7, 5.11 | `test_cli.py::test_kill_commands` |
| R-272 | `atp report run` must write a report with the metrics of R-197, costs by component, the gate report if one exists, the universe's inactive-member counts and the number of positions closed by delisting. | Arch 5.11; Q3 | `test_reports.py::test_run_report_sections` |
| R-273 | `atp report paper` must write a report with the per-session tracking table, cumulative divergence, fill-price differences, breaks and the count of reconciled sessions. | Arch 5.11, 5.7 | `test_reports.py::test_paper_report_sections` |
| R-274 | `atp trials list` and `atp trials show` must show every trial of a family, including open and failed ones, with role and status. | Arch 5.11, D12; *Probability of Backtest Overfitting* p25 | `test_cli.py::test_trials_list_shows_all` |
| R-275 | Every job run must write one JSON-lines log file under `$ATP_HOME/logs/`, each line holding a UTC timestamp, a level, the job id and an event name. | Arch 5.12 | `test_logging.py::test_structured_job_log` |
| R-276 | Credential values must never be written to a file: after an end-to-end run with sentinel credentials, no file under `ATP_HOME` contains a sentinel. | Arch 3 (Data), 5.12 | `test_logging.py::test_no_secret_in_any_file` |
| R-277 | Each of these conditions must write a record with level `alert`: a `fallback_hold`, a failed signal, an order rejected by the gate, the kill switch being set, a reconciliation break, a failed ingest, a missed session, and data coverage below the floor of R-266. | Arch 4 (J), 5.6; Q11 | `test_alerts.py::test_alert_conditions` (eight cases) |
| R-278 | Every job run must add a row to `job_log` with job name, session, start, end, status and exit code. | Arch 5.12 | `test_paper_jobs.py::test_job_log_rows` |
| R-279 | Everything mutable must be written under `ATP_HOME` (default `./var`) in the layout of Arch 5.12, and `state.sqlite` must hold the tables listed there plus `gate_reports` and `promotions`. | Arch 5.12; I15 | `test_layout.py::test_atp_home_layout` (also asserts nothing else in the working tree changed) |
| R-280 | `.gitignore` must ignore the default `ATP_HOME`. | Arch 5.12 | `git check-ignore -q var/x` exits 0 |
| R-281 | A hygiene test must fail when a tracked file is a data file (`.parquet`, `.sqlite`, `.feather`, `.csv`), is larger than 1 MB, or contains a key-like string (an Alpaca key id pattern, or a literal of 20 or more characters assigned to a name containing KEY, SECRET or TOKEN). | Arch 3 (Data), 7.1 J | `test_hygiene.py::test_no_data_secrets_or_large_files` (and a self-test with planted offenders in a temporary repository) |
| R-282 | Test fixtures must use only symbols from a reserved fictitious list, so that no licensed market data or news is committed as a fixture. | Arch 3 (Data: "Tests run on synthetic data") | `test_hygiene.py::test_fixture_symbols_are_fictitious` |
| R-283 | `docs/runbook.md` must contain sections titled Setup, Backfill, Daily schedule, Kill switch, Recovery, Paper-account tests and Unverified Alpaca behaviour. | Arch 4 (J), 7.2, gap 10 | `test_hygiene.py::test_runbook_sections` |
| R-284 | Every TOML file under `configs/`, including `configs/smoke.toml`, must load and validate. | Arch 5.12, 7.2 | `test_configs.py::test_all_configs_load` |
| R-285 | An end-to-end run on synthetic data must complete with the expected exit codes: ingest from a fixture source, build the universe, run backtests, validate, and then, for a configuration that reached `candidate`, two decide and reconcile cycles against the simulated broker. | Arch 7.1 J | `test_end_to_end.py::test_full_cycle` |
| R-286 | On a synthetic panel with a planted signal, a family of one planted-signal configuration and 19 noise configurations, with its baselines, must reach the `candidate` state. | Arch 5.9 (gates 1 to 6); 7.1 H | `test_end_to_end.py::test_planted_signal_reaches_candidate` |
| R-287 | A decide job for a 500-asset universe, with a fixture data source in place of the network, must finish in at most 600 seconds on one core. | Arch 3 (Performance) | (slow) `test_perf.py::test_decide_job_time` |
| R-288 | Within that job, the steps from signals to planned orders must take at most 120 seconds. | Arch 3 (Performance); N | (slow) `test_perf.py::test_pipeline_time` |

## 5. Where the architecture had to be interpreted

The architecture is not rewritten here. These are the places where it was silent, ambiguous or inconsistent, and what the requirements assume. The plan stage should treat each as part of the requirement it is cited from.

| # | Issue in `architecture.md` | What the requirements do |
| --- | --- | --- |
| I1 | Section 7.2 step 5 ends `verify` with `atp validate --family smoke`. By section 5.11 that command exits 3 unless every gate passes, and a one-trial family cannot pass gate 2. With the default `ATP_HOME` the append-only registry would also gain a smoke trial on every run. | R-257 fixes the expected result (0, then 3) in a fresh `ATP_HOME`. The human-owned script must be written to expect it (RQ1). |
| I2 | Section 5.11 says every backtest opens a trial and gates 3 and 4 use all trials of the family, but `TrialRegistry.open` cannot tell a candidate from a baseline, a cost-stress rerun, a seed, an ablation or a placebo run. Counting those as candidates would distort DSR and PBO (*Probability of Backtest Overfitting* p25). | R-193 adds a role to the trial record; only `candidate` trials enter the count and the matrix (R-196, R-202). This extends the signature in section 5.9. |
| I3 | Nothing says how an unclosed trial is counted, or what gates 3 and 4 do with one trial: eq. 1 of *Deflated Sharpe Ratio* is undefined at N = 1 and PBO needs many trials. | R-202 and R-219. |
| I4 | Section 5.3 stores Alpaca's fully adjusted bars. Those change for all past dates whenever a new dividend or split occurs, which conflicts with the append-only rule and lets a later corporate action alter what a view returns for an earlier `as_of`. | R-040 requires views to be invariant to later actions. In practice adjustment must be computed at view time from raw bars and the actions visible at `as_of` (RQ10). |
| I5 | The `bars` schema has no feed column, so the constraint that IEX volume is never used for liquidity cannot be enforced. | R-033 adds the column; R-047 enforces the rule. |
| I6 | `asset_flags` returns no ticker, yet the planner must fill `OrderRequest.symbol`. | R-036. |
| I7 | `reconcile` (G) replays fills through the simulator's ledger (F), but the dependency rule forbids G from importing F and section 5 has no ledger protocol. | R-007 keeps the rule; R-187 has the ledger injected, which adds a parameter to `reconcile` and a ledger protocol to A. |
| I8 | `resid_reversion` needs (a) memory for its entry and exit bands, while `Signal.compute` is stateless, and (b) a sector ETF per stock, while no listed Alpaca data set gives a sector. | R-067 replays the band state from the view; R-068 takes a configured map or the most correlated ETF (RQ8). |
| I9 | `pv_characteristics` is listed as one signal but is a family of characteristics, and a `Signal` returns one score per asset. The characteristics are not named. | R-069: one characteristic per configured instance, five proposed (RQ9). |
| I10 | `Forecast` has one horizon; signals declare different horizons. | R-084. |
| I11 | Section 7.1 F claims engine and reference loop agree to 1e-9 at zero cost. The engine sizes orders at the close of T and fills at the open of T+1, so the claim holds only with fractional shares and when the fill price equals the sizing price. | R-133 states those conditions. |
| I12 | Two inputs have no data source: the half-spread (no quotes in `DataSource`) and a cash or risk-free rate (needed for Sharpe ratios and for the excess return in `tsmom`). | R-102 uses a configured constant; R-198 and R-063 use a configured series or zero (RQ6, RQ7). |
| I13 | Gate 3 asks for "a t-statistic" without naming the series, and for "a new signal" without saying how a multi-signal strategy is classed. | R-218 (RQ5). |
| I14 | `atp validate --gates PATH` supplies gate settings at validation time, while gate 2 says baselines are named before the run. | R-228 separates the two. |
| I15 | Promotion states and gate reports have no table in section 5.12, and section 5.11 does not say whether `atp paper decide` checks the state. | R-227, R-267, R-279 (RQ13). |
| I16 | The plugin "runs as a batch step before `decision_time`" but section 5.11 has no command for it. There is no rule for turning item scores into an asset score, and masking products and subsidiaries needs a dictionary no listed data set provides. | R-242 runs scoring inside the decide job under a time budget; R-239 and R-233 fix the other two (RQ17). |
| I17 | Section D7 puts inactive assets in the universe, but section 5.8 has no rule for a held asset that delists or undergoes a merger or spin-off. | R-142; other corporate actions are out of scope (RQ11). |
| I18 | `available_at` is not defined for corporate actions or for news revised after publication. | R-027, R-028. |
| I19 | Citations in section 7.1 H. (a) "0.9004 at 100 trials" is not legible in the text extraction of *Deflated Sharpe Ratio*; only "a 90% chance" and 0.9505 at N = 46 are. The inputs in R-200 reproduce both when computed. (b) The PBO figures 74% and 0.04% are both on p17; the random-walk example on p26-27 reports 55%. (c) The paper prints 12,780 splits for S = 16 (p12, p23); C(16, 8) is 12,870, and the paper's other counts (924 and 2,704,156) match the binomial coefficient. | R-200, R-203 to R-205 use the legible or computed values. |
| I20 | The target of a ten-year, 500-asset backtest in 10 minutes on one core leaves 0.24 seconds per session for data access, signals, one convex solve and accounting. The figure cited for scale (*Multi-Period Trading* p40) is an estimate from operation counts for a custom solve, not a measurement of cvxpy. | R-123 and R-152 keep the target; feasibility is a risk (RQ15). |
| I21 | Rounding is assigned twice: E owns "rounding heuristics" and G's planner "rounds to whole shares". | Whole-share rounding is G's (R-155); E only removes sub-threshold trades (R-122). |
| I22 | Section 5.7 names the fields of `RiskLimits` without saying what is rejected when one is breached. | R-162 to R-174 (RQ12). |
| I23 | The hygiene test of section 7.1 J fails for any tracked file over 1 MB; 19 of the 31 reference PDFs exceed that. The papers are untracked today, so a reader of the repository cannot open the documents the design cites. | R-281 keeps the rule (RQ19). |

Three statements in the architecture could not be turned into a requirement because nothing observable follows from them: that trials should be planned in advance (*Deflated Sharpe Ratio* p10), that PBO must never become an optimization target (covered only by having no search command, section 6), and that an LLM is treated as a signal source and not as the edge.

## 6. Out of scope

- Live trading: no live endpoint, no configuration switch, no `live` promotion state.
- Tick, order-book and market-making strategies; level-2 data.
- Ingestion of minute bars and any strategy on them. Only the engine seam is required (R-023, R-146).
- A long-running or streaming runner. Jobs are one-shot commands.
- Multi-period optimization, Almgren-Chriss execution schedules and the linear impact model.
- Reinforcement-learning policies, deep networks, time-series foundation models.
- Fundamental or accounting data, the 94-characteristic set, a second data vendor.
- Quote ingestion and per-asset spread estimates.
- An autonomous or multi-agent LLM trader, and automated factor mining.
- Any command or API that searches or tunes configurations automatically, and any use of PBO to select a configuration.
- A second open-source backtesting engine as a cross-check.
- Calibrating the impact coefficient from fills (paper trading simulates no impact).
- Corporate actions other than splits and cash dividends in the ledger (mergers, spin-offs, stock dividends, worthless removals) beyond the close-out of R-142. They are stored and reported by the data check only.
- Hard-to-borrow short sales and locates.
- Taxes, wash sales, regulatory fees and day-trading rules.
- Dividends in the paper-trading comparison (Alpaca paper trading pays none).
- A web interface or dashboard.
- Delivery of alerts to a person, and installation of a scheduler (RQ18).
- A concrete LLM provider adapter (RQ17).
- Changes to `.devteam/verify.sh`, `.github/workflows/`, `REVIEW.md` and `docs/reference/`. A human must make the changes of Arch 7.2.

## 7. Open questions

The thirteen questions of architecture section 8 are still open. Section 2 gives the answer assumed for each; confirming or correcting that table is the first thing asked of the reviewer. The questions below are new at this stage.

- **RQ1. Verify smoke step.** Arch 7.2 step 5 ends with `atp validate --family smoke`, which exits 3 for a one-trial family (R-257). Should the human-owned `verify.sh` accept exit 3 there, or should the smoke family be built up until it passes every gate? It also needs a fresh `ATP_HOME` on every run.
- **RQ2. DSR worked example.** Confirm from the PDF (p9-10) the inputs N = 100, variance of Sharpe ratios 1/2, skewness -3, kurtosis 10. They are illegible in the text extraction; computed, they reproduce the two legible results.
- **RQ3. Which runs count as trials.** R-193, R-196 and R-202 assume that only `candidate` runs enter the trial count and the CSCV matrix; that baseline, cost-stress, sensitivity, seed, ablation, placebo and shuffle runs are registered but not counted; and that an unclosed candidate run counts as one trial. Confirm.
- **RQ4. Small families.** R-219 assumes gate 4 is not applicable for a single registered candidate and fails as not evaluable for 2 to 19. Confirm, or name another rule.
- **RQ5. Gate 3 t-statistic.** R-218 defines it on the selected trial's per-session net excess returns and applies 3.0 to every signal origin by default. Confirm the series, and say whether strategies built only from published signals get a lower value (Arch Q6, C2).
- **RQ6. Cash rate.** No listed data set supplies a risk-free rate. R-198 uses a configured series or zero. Which series should it be?
- **RQ7. Spread.** No quote data is ingested, so R-102 charges a constant 5 bps half-spread. Accept, or add quote ingestion?
- **RQ8. Sector map for residual reversion.** No listed Alpaca data gives a sector. R-068 falls back to the most correlated configured ETF. Will a sector map be supplied, or should the paper's PCA variant be built instead?
- **RQ9. Characteristics.** Confirm or replace the five characteristics of R-069.
- **RQ10. Adjusted prices.** R-040 implies adjusting at view time from raw bars and the corporate actions visible at `as_of`, with Alpaca's adjusted bars at most a cross-check. Confirm, since Arch 5.3 stores Alpaca's adjusted bars.
- **RQ11. Delistings and other corporate actions.** R-142 closes a delisted position at its last close; mergers, spin-offs and stock dividends are out of scope for the ledger. Accept both?
- **RQ12. Risk gate.** Confirm the rejection rules of R-162 to R-174 and supply the limit values (Arch Q10) before any paper run.
- **RQ13. Paper trading only for candidates.** R-267 refuses `atp paper decide` for a configuration that has not passed gates 1 to 6. Confirm, or allow paper runs of baselines and unpromoted strategies.
- **RQ14. Proposed numbers.** Confirm or change the values in section 3, in particular the tracking tolerance (5 and 25 bps), the 2% buying-power buffer, the 95% data-coverage floor and the 20-trial floor for PBO.
- **RQ15. Performance targets.** Is the ten-year, 500-asset backtest in 10 minutes on one core (R-152, hence 0.15 seconds per optimization in R-123) a firm requirement or a goal? On which machine are the slow checks measured?
- **RQ16. Alpaca unknowns (Arch gap 10).** If a paper-only account cannot read SIP history, R-046 stops ingestion. What then: pay for the data plan, or redesign the liquidity filters for IEX data?
- **RQ17. LLM plugin.** Not ready to specify until Arch Q8 is answered: the concrete provider adapter and its call budget. Also confirm that scoring runs inside the decide job under a 300-second budget (R-242), the aggregation rule (R-239), and who supplies the alias dictionary of products and subsidiaries (R-233).
- **RQ18. Alerts and schedule.** Not ready to specify until Arch Q11 is answered: how an alert reaches a person, and which scheduler runs the jobs.
- **RQ19. Reference papers.** 19 of the 31 PDFs exceed 1 MB, so the hygiene test (R-281) fails if they are ever committed. Keep them out of the repository for good, or exempt `docs/reference/`?

This stage needs human approval before the plan stage starts.

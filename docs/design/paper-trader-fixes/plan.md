# paper-trader-fixes: plan

This plan builds `requirements.md` of this design on top of the landed
paper-trader code. It follows the shape of `docs/design/paper-trader/plan.md`:
each chain is one branch `chain/paper-trader-fixes/<chain>`, each step one
commit whose subject starts with `[<ID>]`, tests first from the tester and
code from the implementer.

Step IDs continue each component's numbering from the old plan (the old plan
ended at A11, C15, D27, E26, F13, G20, H14, I11, J06, K23, L04, M17, N12, O46,
P14, Q13, R27, V11 and W06), so no ID is used twice across the two plans.

Rules every step follows:

- A step stays inside its Files. A path ending in `/` means any file under
  that directory.
- Each step adds or extends one integration test file under
  `crates/<name>/tests/` and its Proof runs that file. A step whose Files hold
  only test files is a pin: the behaviour exists and must not change.
- A refactoring step cites the requirement it makes possible or the scan that
  later proves it, and its Proof runs an existing test that must stay green.
- Every step leaves CHECK passing. When a step changes something other tests
  rely on, those test files are in its Files.
- Tests use fake keys, `pt-mock-alpaca` and loopback only. No step reads a
  secrets file, and the live trading host appears nowhere.
- Tier `opus` marks migrations, changes that cross crates and anything on the
  order path; the rest is `sonnet`.

## Chains

| Chain | Story | After |
| --- | --- | --- |
| scaffold-workspace | Licences are judged as SPDX expressions and no crate declares a dependency it does not use. | - |
| scaffold-core | An order marked rejected may become submitted, so a held order can be adopted. | - |
| scaffold-store | The store is at schema version 5 and an older database still opens. | - |
| scaffold-alpaca | The client refuses a rate limit of 0, names what an order reject is, and lends out its sleeper. | - |
| scaffold-gate | The first-order approval carries the quantity to send and a stage knows its own name. | - |
| scaffold-live | The live crate has its three new modules and its new error, all still empty. | - |
| mock-hooks | The mock broker can refuse a route, refuse an order yet keep it, and report another margin multiplier. | - |
| config-backtest | The starting capital of a backtest is a config value outside the config hash, and sign-off minimums cannot be 0. | - |
| strategy-guards | The strategy refuses numbers it cannot use and exports its weights function and its ids. | - |
| stats-guards | Every statistic either refuses a bad input with a typed error or returns NaN, and never overflows. | - |
| portfolio-pins | The all-zero cases of sizing are pinned by a test. | - |
| risk-pins | The fail-closed rule for a batch that cannot be valued is pinned by a test. | - |
| vector-pins | The first-date rule of the vector backtest is pinned by a test. | - |
| ops-alerts | An alert leaks no secret through its code and follows no redirect. | - |
| ops-report | P&L from a zero base is absent, and the keys and timestamp of the status report are pinned. | - |
| core-view-doc | The comment on adv_shares says what the code does. | - |
| store-market | A bad batch of bars or a conflicting asset snapshot is refused whole with a typed error. | scaffold-store |
| store-trading | Fills, filled quantities, off-ledger orders and the two paired writes are strict and atomic. | scaffold-store |
| store-ops | Halts say when they are already cleared, and a stored backtest keeps its capital. | scaffold-store |
| alpaca-paths | Symbols and order ids are percent-encoded in request paths. | scaffold-alpaca, scaffold-workspace |
| alpaca-pins | Quantity parsing, order-list paging and the order retry rule are pinned by tests. | scaffold-alpaca, scaffold-workspace |
| gate-authorize | Cancelling is allowed in every stage, the kill switch has its own authorisation, and the first order is authorised for its exact quantity. | scaffold-gate |
| gate-checklist | The owner checklist has a thirteenth item for the first order. | scaffold-gate |
| live-adopt | An order marked rejected that the broker still holds is adopted. | scaffold-live, scaffold-core, mock-hooks |
| live-slots | The binary can tell whether a New York schedule slot has come due. | scaffold-live, scaffold-workspace |
| live-submit | Submission treats only 403 and 422 as a reject and fails when it cannot judge staleness. | scaffold-alpaca, mock-hooks |
| data-quality | A universe symbol with no stored bars is reported, and the bad-bar error has a pinned shape. | scaffold-workspace |
| data-pins | The raw refetch anchor, the margin boundary and the failed calendar import are pinned by tests. | scaffold-workspace, mock-hooks |
| engine-sim | A simulation refuses a reversed range, and its borrow and impact costs are pinned. | scaffold-workspace |
| engine-decide | A decision asks the strategy once, and its JSON reasons and missing-price cases are pinned. | scaffold-workspace |
| shadow-fidelity | Fidelity counts only planned rebalances and works on a store with no calendar. | scaffold-workspace |
| shadow-pins | The order states of the shadow day and the layout of the sign-off file are pinned. | scaffold-workspace |
| cli-shared | The command modules share one store opener, webhook lookup, strategy wrapper, stage name and config path, and the kill switch has its own module. | scaffold-gate, scaffold-alpaca, scaffold-workspace, strategy-guards |
| cli-fixtures-live | The command tests that drive a mock paper session take their fixture from tests/common. | scaffold-workspace |
| live-kill-switch | Flatten works in every stage: it cancels, settles, closes what the broker holds, adopts a held order and reports three counts. | gate-authorize, live-adopt, store-trading, scaffold-alpaca, cli-shared |
| live-first-order | A rejected first order is retried with back-off under new ids, adopted if held, and used up only when accepted. | gate-authorize, live-adopt, scaffold-alpaca |
| live-settle | Settlement writes atomically, abandons leg 2 after a short leg 1, counts releases as events and keeps off-ledger fills out of the book. | scaffold-live, store-trading, mock-hooks |
| live-rebalance | A rebalance writes its decision and orders together and warns about an asset with no snapshot. | store-trading |
| validate-capital | Starting capital is out of the trial hash and stored with the run. | store-ops, config-backtest |
| cli-modules | Each command owns its argument types and the report inputs are built in one place. | cli-shared |
| cli-data-check | The data check lists a universe symbol with no bars and fails. | cli-shared, data-quality |
| cli-fixtures-offline | The command tests that run offline backtests take their fixture from tests/common. | cli-fixtures-live |
| cli-validate-pins | The validation commands' family counting and output labels are pinned. | cli-fixtures-offline |
| live-cycle-pins | The first-order stage, failed runs and quiet rebalance outcomes are pinned at the level of a whole run. | live-first-order, live-rebalance, live-submit |
| live-alert-pins | Warning levels are pinned, and a flatten before the first reconciliation leaves a clean book. | live-settle, live-kill-switch, live-submit |
| validate-pins | Run comparisons use common dates, ignore the stored config hash and refuse non-finite returns. | validate-capital, stats-guards |
| cli-kill-switch | The kill-switch commands check --yes first, run in every stage and print what happened. | live-kill-switch, cli-modules, cli-fixtures-live |
| cli-run | The run command locks before it opens the store, always writes the report and can skip when no slot is due. | cli-modules, cli-fixtures-live, live-slots |
| cli-report | The status report shows the fidelity summary, also for an empty store. | cli-modules, cli-fixtures-live, shadow-fidelity, ops-report |
| cli-gates | The gates command shows whether the first order was accepted and sees the webhook variable. | cli-modules, gate-checklist, live-first-order |
| cli-backtest | Backtests take their capital from the flag, the account or the config, and say which. | cli-modules, cli-fixtures-offline, validate-capital, config-backtest |
| cli-deploy | The example schedules fire every fifteen minutes and pass --if-due. | cli-run |
| cli-e2e | Help offers only designed flags, and a flatten before the paper stage is proven end to end. | cli-kill-switch, cli-run, cli-backtest, cli-report, live-settle |
| hygiene-scan | A scan keeps the shared helpers single and the corrected comments correct. | cli-e2e, cli-gates, cli-data-check, cli-deploy, cli-validate-pins, live-first-order, core-view-doc, store-ops |

## Steps

| ID | Chain | Step | Reqs | Tier | Files | Proof |
| --- | --- | --- | --- | --- | --- | --- |
| W07 | scaffold-workspace | Judge dependency licences as SPDX expressions | R-003 | sonnet | crates/archtest/tests/licences.rs | cargo test -p pt-archtest --test licences |
| W08 | scaffold-workspace | Remove unused dependencies and add a scan that keeps them out | R-403 | opus | Cargo.lock, crates/alpaca/Cargo.toml, crates/cli/Cargo.toml, crates/core/Cargo.toml, crates/data/Cargo.toml, crates/engine/Cargo.toml, crates/gate/Cargo.toml, crates/shadow/Cargo.toml, crates/archtest/tests/unused_deps.rs | cargo test -p pt-archtest --test unused_deps |
| A12 | scaffold-core | Allow the transition from Rejected to Submitted | R-017 | sonnet | crates/core/src/order_state.rs, crates/core/tests/order_state.rs | cargo test -p pt-core --test order_state |
| D28 | scaffold-store | Add migration 0005 and the Invalid store error | R-415, R-416 | opus | crates/store/migrations/0005_fixes.sql, crates/store/src/db.rs, crates/store/src/error.rs, crates/store/tests/migrate_0005.rs | cargo test -p pt-store --test migrate_0005 |
| E27 | scaffold-alpaca | Refuse a rate limit of 0 with a typed error | R-432 | opus | crates/alpaca/src/, crates/alpaca/tests/, crates/cli/src/wiring.rs | cargo test -p pt-alpaca --test limiter |
| E28 | scaffold-alpaca | Add is_order_reject for 403 and 422 | R-433 | sonnet | crates/alpaca/src/error.rs, crates/alpaca/tests/order_reject.rs | cargo test -p pt-alpaca --test order_reject |
| E29 | scaffold-alpaca | Let callers sleep through the client's sleeper | R-434 | sonnet | crates/alpaca/src/http.rs, crates/alpaca/tests/http_sleep.rs | cargo test -p pt-alpaca --test http_sleep |
| N13 | scaffold-gate | Require qty in the first-order approval | R-467 | opus | crates/gate/src/owner.rs, crates/gate/src/status.rs, crates/gate/tests/first_order_qty.rs, crates/gate/tests/authorize_first_order.rs, crates/gate/tests/authorize_preconditions.rs, crates/gate/tests/checklist.rs, crates/gate/tests/first_order_approval.rs, crates/gate/tests/owner_example.rs, crates/gate/tests/owner_parse.rs, crates/live/tests/cycle_first_order.rs, crates/live/tests/cycle_first_order_repeat.rs, crates/live/tests/first_order_plan.rs, config/owner.example.toml | cargo test -p pt-gate --test first_order_qty |
| N14 | scaffold-gate | Add Stage::as_str | R-468 | sonnet | crates/gate/src/owner.rs, crates/gate/tests/stage_str.rs | cargo test -p pt-gate --test stage_str |
| O47 | scaffold-live | Add the empty adopt, halts and slots modules and the FirstOrderExhausted error | R-472, R-474, R-497, R-514 | sonnet | crates/live/src/lib.rs, crates/live/src/error.rs, crates/live/src/adopt.rs, crates/live/src/halts.rs, crates/live/src/slots.rs | cargo test -p pt-live --test fixture |
| F14 | mock-hooks | Add reject_next for one route | R-440 | sonnet | crates/mock-alpaca/src/server.rs, crates/mock-alpaca/src/routes.rs, crates/mock-alpaca/src/state.rs, crates/mock-alpaca/tests/reject_next.rs | cargo test -p pt-mock-alpaca --test reject_next |
| F15 | mock-hooks | Add reject_but_keep_next_order | R-441 | sonnet | crates/mock-alpaca/src/server.rs, crates/mock-alpaca/src/routes.rs, crates/mock-alpaca/src/state.rs, crates/mock-alpaca/tests/reject_keep.rs | cargo test -p pt-mock-alpaca --test reject_keep |
| F16 | mock-hooks | Add seed_account_multiplier | R-442 | sonnet | crates/mock-alpaca/src/server.rs, crates/mock-alpaca/src/routes.rs, crates/mock-alpaca/src/state.rs, crates/mock-alpaca/tests/account_multiplier.rs | cargo test -p pt-mock-alpaca --test account_multiplier |
| C16 | config-backtest | Add the backtest section with initial_capital outside the config hash | R-412, R-413 | sonnet | crates/config/src/config.rs, config/paper-trader.example.toml, crates/config/tests/backtest_capital.rs | cargo test -p pt-config --test backtest_capital |
| C17 | config-backtest | Require each sign-off minimum to be at least 1 | R-414 | sonnet | crates/config/src/config.rs, crates/config/tests/signoff_minimums.rs | cargo test -p pt-config --test signoff_minimums |
| H15 | strategy-guards | Make a symbol with non-finite volatility ineligible | R-448 | sonnet | crates/strategy/src/estimate.rs, crates/strategy/tests/estimate_non_finite.rs | cargo test -p pt-strategy --test estimate_non_finite |
| H16 | strategy-guards | Return an error for a momentum strategy with no lookback | R-449 | sonnet | crates/strategy/src/tsmom.rs, crates/strategy/tests/tsmom_no_lookback.rs | cargo test -p pt-strategy --test tsmom_no_lookback |
| H17 | strategy-guards | Take volatility and covariance weights from ewma_weights | R-450 | sonnet | crates/strategy/src/vol.rs, crates/strategy/tests/ewma_weights.rs | cargo test -p pt-strategy --test ewma_weights |
| H18 | strategy-guards | Export each strategy id as a constant | R-451 | sonnet | crates/strategy/src/tsmom.rs, crates/strategy/src/benchmark.rs, crates/strategy/tests/strategy_ids.rs | cargo test -p pt-strategy --test strategy_ids |
| M18 | stats-guards | Return an error from n_combinations for overflow and odd counts | R-244, R-462, R-463 | sonnet | crates/stats/src/pbo.rs, crates/stats/tests/pbo_combinations.rs, crates/stats/tests/pbo_speed.rs | cargo test -p pt-stats --test pbo_combinations |
| M19 | stats-guards | Refuse non-finite inputs in the statistics that report errors | R-464 | sonnet | crates/stats/src/ols.rs, crates/stats/src/dsr.rs, crates/stats/src/pbo.rs, crates/stats/src/normal.rs, crates/stats/tests/non_finite_inputs.rs | cargo test -p pt-stats --test non_finite_inputs |
| M20 | stats-guards | Return NaN from max_drawdown for a NaN input and pin the other plain statistics | R-465 | sonnet | crates/stats/src/perf.rs, crates/stats/src/descriptive.rs, crates/stats/tests/nan_propagates.rs | cargo test -p pt-stats --test nan_propagates |
| M21 | stats-guards | Return an error from ols when its result would not be finite | R-466 | sonnet | crates/stats/src/ols.rs, crates/stats/tests/ols_singular.rs | cargo test -p pt-stats --test ols_singular |
| I12 | portfolio-pins | Pin all-zero weights for zero signals and unmeasurable risk | R-452 | sonnet | crates/portfolio/tests/weights_all_zero.rs | cargo test -p pt-portfolio --test weights_all_zero |
| J07 | risk-pins | Pin the rejection of a whole batch that cannot be valued | R-453 | sonnet | crates/risk/tests/unpriced_batch.rs | cargo test -p pt-risk --test unpriced_batch |
| L05 | vector-pins | Pin that a first-date weights entry is not traded | R-461 | sonnet | crates/vector/tests/first_date.rs | cargo test -p pt-vector --test first_date |
| Q14 | ops-alerts | Scrub secrets from the alert code | R-522 | sonnet | crates/ops/src/alert.rs, crates/ops/tests/alert_scrub_code.rs | cargo test -p pt-ops --test alert_scrub_code |
| Q15 | ops-alerts | Follow no redirect when posting an alert | R-352 | sonnet | crates/ops/src/alert.rs, crates/ops/tests/alert_redirect.rs | cargo test -p pt-ops --test alert_redirect |
| Q16 | ops-report | Make each P&L value absent when its base equity is zero | R-356 | sonnet | crates/ops/src/pnl.rs, crates/ops/src/report.rs, crates/ops/tests/pnl.rs, crates/ops/tests/pnl_missing.rs, crates/ops/tests/pnl_zero_base.rs, crates/ops/tests/status_json_book.rs, crates/ops/tests/status_md.rs | cargo test -p pt-ops --test pnl_zero_base |
| Q17 | ops-report | Pin the keys of status.json | R-523 | sonnet | crates/ops/tests/status_schema.rs | cargo test -p pt-ops --test status_schema |
| Q18 | ops-report | Pin the timestamp and the book the status report shows | R-524 | sonnet | crates/ops/tests/status_time.rs | cargo test -p pt-ops --test status_time |
| A13 | core-view-doc | Correct the adv_shares doc comment to on or before | R-410 | sonnet | crates/core/src/view.rs | cargo test -p pt-core --test view_traits |
| D29 | store-market | Refuse a batch with two different bars for one key | R-417 | sonnet | crates/store/src/market.rs, crates/store/tests/bars_batch_duplicate.rs | cargo test -p pt-store --test bars_batch_duplicate |
| D30 | store-market | Refuse a bar version older than the stored one | R-418 | sonnet | crates/store/src/market.rs, crates/store/tests/bars_stale.rs, crates/store/tests/bars_versions.rs, crates/store/tests/bars_asof.rs | cargo test -p pt-store --test bars_stale |
| D31 | store-market | Keep the first asset snapshot for a symbol and instant | R-419 | sonnet | crates/store/src/market.rs, crates/store/tests/assets_repeat.rs | cargo test -p pt-store --test assets_repeat |
| D32 | store-trading | Ignore only a repeated activity id when recording a fill | R-420 | sonnet | crates/store/src/trading.rs, crates/store/tests/fills_strict.rs | cargo test -p pt-store --test fills_strict |
| D33 | store-trading | Refuse a filled quantity above the order quantity or below the stored one | R-421, R-422 | sonnet | crates/store/src/trading.rs, crates/store/tests/filled_qty.rs | cargo test -p pt-store --test filled_qty |
| D34 | store-trading | Add the off-ledger mark for orders | R-424 | sonnet | crates/store/src/trading.rs, crates/store/tests/off_ledger.rs | cargo test -p pt-store --test off_ledger |
| D35 | store-trading | Record a fill and its ledger entry in one transaction | R-425 | opus | crates/store/src/trading.rs, crates/store/tests/record_fill.rs | cargo test -p pt-store --test record_fill |
| D36 | store-trading | Insert a decision and its orders in one transaction | R-426 | opus | crates/store/src/trading.rs, crates/store/tests/decision_with_orders.rs | cargo test -p pt-store --test decision_with_orders |
| D37 | store-trading | Derive positions and cash at a date from one fold | R-427 | sonnet | crates/store/src/trading.rs, crates/store/tests/ledger_fold.rs | cargo test -p pt-store --test ledger_fold |
| D38 | store-trading | Pin the time of an order's creation event | R-423 | sonnet | crates/store/tests/order_event_time.rs | cargo test -p pt-store --test order_event_time |
| D39 | store-ops | Say already cleared when a cleared halt is cleared again | R-428 | sonnet | crates/store/src/ops.rs, crates/store/tests/halts_cleared_twice.rs | cargo test -p pt-store --test halts_cleared_twice |
| D40 | store-ops | Pin second precision and insertion order for stored instants | R-429 | sonnet | crates/store/tests/instant_precision.rs | cargo test -p pt-store --test instant_precision |
| D41 | store-ops | Store a backtest's starting capital and its source | R-430 | sonnet | crates/store/src/ops.rs, crates/store/tests/backtest_capital.rs | cargo test -p pt-store --test backtest_capital |
| D42 | store-ops | Pin backtest_family for a known and an unknown run | R-431 | sonnet | crates/store/tests/backtest_family.rs | cargo test -p pt-store --test backtest_family |
| D43 | store-ops | Name halts, alerts, trials and backtests in the module comment | R-411 | sonnet | crates/store/src/ops.rs | cargo test -p pt-store --test halts |
| E30 | alpaca-paths | Percent-encode the symbol in the asset path | R-435 | sonnet | crates/alpaca/src/reference.rs, crates/alpaca/tests/asset.rs | cargo test -p pt-alpaca --test asset |
| E31 | alpaca-paths | Percent-encode the order id in the cancel path | R-436 | sonnet | crates/alpaca/src/orders_write.rs, crates/alpaca/tests/cancel_order.rs | cargo test -p pt-alpaca --test cancel_order |
| E32 | alpaca-pins | Pin the parse error for an order without a whole-share quantity | R-437 | sonnet | crates/alpaca/tests/order_qty_parse.rs | cargo test -p pt-alpaca --test order_qty_parse |
| E33 | alpaca-pins | Pin exclusive paging of the order list across two pages | R-438 | sonnet | crates/alpaca/tests/orders_since_paging.rs | cargo test -p pt-alpaca --test orders_since_paging |
| E34 | alpaca-pins | Pin the order retry rule with several queued replies | R-439 | sonnet | crates/alpaca/tests/submit_retry.rs | cargo test -p pt-alpaca --test submit_retry |
| N15 | gate-authorize | Allow CancelOrder in every stage | R-263, R-321 | opus | crates/gate/src/gate.rs, crates/gate/tests/capabilities.rs, crates/live/tests/cancel_open.rs | cargo test -p pt-gate --test capabilities |
| N16 | gate-authorize | Add the kill-switch authorisation | R-265, R-269, R-469, R-470 | opus | crates/gate/src/gate.rs, crates/gate/src/error.rs, crates/gate/tests/kill_switch.rs | cargo test -p pt-gate --test kill_switch |
| N17 | gate-authorize | Authorise the first order only for the approved quantity | R-268 | opus | crates/gate/src/gate.rs, crates/gate/tests/authorize_first_order.rs | cargo test -p pt-gate --test authorize_first_order |
| N18 | gate-authorize | Pin the first_order getter | R-471 | sonnet | crates/gate/tests/first_order_getter.rs | cargo test -p pt-gate --test first_order_getter |
| N19 | gate-checklist | Add first_order_sent as the thirteenth checklist item | R-270, R-382 | sonnet | crates/gate/src/status.rs, crates/gate/tests/checklist.rs, crates/cli/tests/gates.rs | cargo test -p pt-gate --test checklist |
| O48 | live-adopt | Add adopt_if_held | R-472, R-473 | opus | crates/live/src/adopt.rs, crates/live/tests/adopt.rs | cargo test -p pt-live --test adopt |
| O49 | live-slots | Add chrono-tz and slot_due for the three New York slots | R-514, R-515, R-516 | opus | Cargo.toml, Cargo.lock, crates/live/Cargo.toml, crates/live/src/slots.rs, crates/live/tests/slots.rs | cargo test -p pt-live --test slots |
| O50 | live-submit | Treat only 403 and 422 as an order reject on submit | R-482, R-483 | opus | crates/live/src/submit.rs, crates/live/tests/submit_reject_status.rs | cargo test -p pt-live --test submit_reject_status |
| O51 | live-submit | Fail submission when no session precedes the trade session | R-484 | sonnet | crates/live/src/submit.rs, crates/live/tests/submit_no_prev_session.rs | cargo test -p pt-live --test submit_no_prev_session |
| G21 | data-quality | Report universe symbols with no stored bars | R-126, R-443 | sonnet | crates/data/src/quality.rs, crates/data/tests/no_bars.rs | cargo test -p pt-data --test no_bars |
| G22 | data-quality | Pin the symbol, date and message of the bad-bar error | R-444 | sonnet | crates/data/tests/check_bar.rs | cargo test -p pt-data --test check_bar |
| G23 | data-pins | Pin the raw refetch anchor at five sessions before the earliest latest bar | R-445 | sonnet | crates/data/tests/ingest_anchor.rs | cargo test -p pt-data --test ingest_anchor |
| G24 | data-pins | Pin the margin check at a multiplier of exactly 2 | R-446 | sonnet | crates/data/tests/doctor_margin.rs | cargo test -p pt-data --test doctor_margin |
| G25 | data-pins | Assert that a failed calendar import leaves no calendar | R-447 | sonnet | crates/data/tests/import_calendar.rs | cargo test -p pt-data --test import_calendar |
| K24 | engine-sim | Refuse a simulation whose start is after its end | R-454 | sonnet | crates/engine/src/simulate.rs, crates/engine/tests/simulate_reversed_range.rs | cargo test -p pt-engine --test simulate_reversed_range |
| K25 | engine-sim | Pin one day of borrow on the calendar's last session | R-455 | sonnet | crates/engine/tests/borrow_last_session.rs | cargo test -p pt-engine --test borrow_last_session |
| K26 | engine-sim | Tighten the impact cost comparison to 1e-9 | R-460 | sonnet | crates/engine/tests/sim_broker_impact.rs | cargo test -p pt-engine --test sim_broker_impact |
| K27 | engine-decide | Ask the strategy for its weights once per decision | R-459 | sonnet | crates/engine/src/decision.rs, crates/engine/tests/decide_once.rs | cargo test -p pt-engine --test decide_once |
| K28 | engine-decide | Pin the rejection reason names in decision JSON | R-456 | sonnet | crates/engine/tests/decision_reason_names.rs | cargo test -p pt-engine --test decision_reason_names |
| K29 | engine-decide | Pin the two missing-price cases of a decision | R-457, R-458 | sonnet | crates/engine/tests/decide_missing_price.rs | cargo test -p pt-engine --test decide_missing_price |
| P15 | shadow-fidelity | Count only planned decisions as rebalances | R-517 | sonnet | crates/shadow/src/fidelity.rs, crates/shadow/tests/fidelity_rebalances.rs, crates/shadow/tests/fidelity_late.rs | cargo test -p pt-shadow --test fidelity_rebalances |
| P16 | shadow-fidelity | Return all-zero fidelity for a store with no calendar | R-518 | sonnet | crates/shadow/src/fidelity.rs, crates/shadow/tests/fidelity_no_calendar.rs | cargo test -p pt-shadow --test fidelity_no_calendar |
| P17 | shadow-fidelity | Pin that the last session never counts as missed | R-519 | sonnet | crates/shadow/tests/fidelity_last_session.rs | cargo test -p pt-shadow --test fidelity_last_session |
| P18 | shadow-pins | Pin the order states the shadow day fills | R-520 | sonnet | crates/shadow/tests/shadow_states.rs | cargo test -p pt-shadow --test shadow_states |
| P19 | shadow-pins | Assert the exact layout of the sign-off file | R-521 | sonnet | crates/shadow/tests/signoff_file.rs | cargo test -p pt-shadow --test signoff_file |
| R28 | cli-shared | Open the store and look up the webhook in one place | R-405 | sonnet | crates/cli/src/wiring.rs, crates/cli/src/cmd_run.rs, crates/cli/src/cmd_ops.rs, crates/cli/src/cmd_data.rs | cargo test -p pt-cli --test run_ok |
| R29 | cli-shared | Share the stored strategy and take strategy ids from pt-strategy | R-405, R-406 | opus | crates/cli/src/main.rs, crates/cli/src/strategies.rs, crates/cli/src/cmd_run.rs, crates/cli/src/cmd_ops.rs, crates/cli/src/cmd_backtest.rs | cargo test -p pt-cli --test backtest_run |
| R30 | cli-shared | Use Stage::as_str in the command modules | R-404 | sonnet | crates/cli/src/wiring.rs, crates/cli/src/cmd_config.rs, crates/cli/src/cmd_run.rs, crates/cli/src/cmd_ops.rs | cargo test -p pt-cli --test gates |
| R31 | cli-shared | Resolve the config path once in dispatch | R-407 | sonnet | crates/cli/src/main.rs, crates/cli/src/wiring.rs | cargo test -p pt-cli --test config_check |
| R32 | cli-shared | Split the kill-switch commands into their own module | R-396 | opus | crates/cli/src/main.rs, crates/cli/src/cmd_kill.rs, crates/cli/src/cmd_ops.rs | cargo test -p pt-cli --test killswitch |
| R33 | cli-fixtures-live | Move the live fixture into tests/common and use it in run_ok | R-408 | sonnet | crates/cli/tests/common/mod.rs, crates/cli/tests/common/live.rs, crates/cli/tests/run_ok.rs | cargo test -p pt-cli --test run_ok |
| R34 | cli-fixtures-live | Use the shared live fixture in run_exit_codes | R-408 | sonnet | crates/cli/tests/common/live.rs, crates/cli/tests/run_exit_codes.rs | cargo test -p pt-cli --test run_exit_codes |
| R35 | cli-fixtures-live | Use the shared live fixture in run_report | R-408 | sonnet | crates/cli/tests/common/live.rs, crates/cli/tests/run_report.rs | cargo test -p pt-cli --test run_report |
| R36 | cli-fixtures-live | Use the shared live fixture in killswitch | R-408 | sonnet | crates/cli/tests/common/live.rs, crates/cli/tests/killswitch.rs | cargo test -p pt-cli --test killswitch |
| R37 | cli-fixtures-live | Use the shared live fixture in e2e_paper | R-408 | sonnet | crates/cli/tests/common/live.rs, crates/cli/tests/e2e_paper.rs | cargo test -p pt-cli --test e2e_paper |
| O52 | live-kill-switch | Return cancelled, accepted and rejected counts from flatten | R-322, R-487 | opus | crates/live/src/killswitch.rs, crates/live/tests/flatten.rs, crates/live/tests/flatten_counts.rs, crates/cli/src/cmd_kill.rs | cargo test -p pt-live --test flatten_counts |
| O53 | live-kill-switch | Authorise flatten orders through the kill-switch rule in every stage | R-485 | opus | crates/live/src/killswitch.rs, crates/live/tests/flatten_stages.rs | cargo test -p pt-live --test flatten_stages |
| O54 | live-kill-switch | Cancel open orders and settle before flatten closes positions | R-486 | opus | crates/live/src/killswitch.rs, crates/live/tests/flatten_sequence.rs | cargo test -p pt-live --test flatten_sequence |
| O55 | live-kill-switch | Adopt a held order before a flatten retry | R-488 | opus | crates/live/src/killswitch.rs, crates/live/tests/flatten_adopt.rs | cargo test -p pt-live --test flatten_adopt |
| O56 | live-kill-switch | Stop flatten on an error that is not a broker reject | R-489 | opus | crates/live/src/killswitch.rs, crates/live/tests/flatten.rs, crates/live/tests/flatten_not_reject.rs | cargo test -p pt-live --test flatten_not_reject |
| O57 | live-kill-switch | Mark flatten orders off-ledger while the paper ledger is empty | R-490 | opus | crates/live/src/killswitch.rs, crates/live/tests/flatten_off_ledger.rs | cargo test -p pt-live --test flatten_off_ledger |
| O58 | live-first-order | Plan the first order for the approved quantity | R-492 | sonnet | crates/live/src/first_order.rs, crates/live/tests/first_order_qty.rs | cargo test -p pt-live --test first_order_qty |
| O59 | live-first-order | Count the first order as used only once the broker accepted it | R-320, R-493 | opus | crates/live/src/first_order.rs, crates/live/src/cycle.rs, crates/live/tests/first_order_accepted.rs, crates/live/tests/cycle_first_order_repeat.rs | cargo test -p pt-live --test first_order_accepted |
| O60 | live-first-order | Retry a rejected first order under new ids with back-off | R-494, R-495 | opus | crates/live/src/first_order.rs, crates/live/src/cycle.rs, crates/live/src/ids.rs, crates/live/tests/first_order_retry.rs | cargo test -p pt-live --test first_order_retry |
| O61 | live-first-order | Adopt a held first order before retrying | R-496 | opus | crates/live/src/first_order.rs, crates/live/tests/first_order_adopt.rs | cargo test -p pt-live --test first_order_adopt |
| O62 | live-first-order | Fail the run with one critical alert when the retries are used up | R-497, R-498 | opus | crates/live/src/first_order.rs, crates/live/src/cycle.rs, crates/live/tests/first_order_exhausted.rs | cargo test -p pt-live --test first_order_exhausted |
| O63 | live-first-order | Give an edited approval its own attempts and ignore one after acceptance | R-499, R-500 | opus | crates/live/src/first_order.rs, crates/live/src/cycle.rs, crates/live/tests/first_order_edited.rs | cargo test -p pt-live --test first_order_edited |
| O64 | live-first-order | Use Stage::as_str in the cycle | R-404 | sonnet | crates/live/src/cycle.rs | cargo test -p pt-live --test cycle_paper |
| O65 | live-settle | Set every halt through one helper that raises halt_set | R-474 | sonnet | crates/live/src/halts.rs, crates/live/src/settle.rs, crates/live/src/reconcile.rs, crates/live/src/snapshot.rs, crates/live/tests/halt_alerts.rs | cargo test -p pt-live --test halt_alerts |
| O66 | live-settle | Record each fill with its ledger entry through record_fill | R-475 | opus | crates/live/src/settle.rs, crates/live/tests/settle_atomic.rs | cargo test -p pt-live --test settle_atomic |
| O67 | live-settle | Abandon leg 2 when leg 1 ends short after a partial fill | R-476, R-477 | opus | crates/live/src/settle.rs, crates/live/tests/legs_partial.rs | cargo test -p pt-live --test legs_partial |
| O68 | live-settle | Count the legs released by this settlement | R-478 | sonnet | crates/live/src/settle.rs, crates/live/tests/legs_event.rs | cargo test -p pt-live --test legs_event |
| O69 | live-settle | Keep fills of off-ledger orders out of the fills and the ledger | R-479 | opus | crates/live/src/settle.rs, crates/live/tests/settle_off_ledger.rs | cargo test -p pt-live --test settle_off_ledger |
| O70 | live-settle | Pin the single incomplete-fill alert and foreign orders beyond one page | R-480, R-481 | sonnet | crates/live/tests/settle_incomplete_once.rs, crates/live/tests/reconcile_paging.rs | cargo test -p pt-live --test reconcile_paging |
| O71 | live-rebalance | Write the decision and its orders in one transaction | R-506 | opus | crates/live/src/rebalance.rs, crates/live/tests/rebalance_atomic.rs | cargo test -p pt-live --test rebalance_atomic |
| O72 | live-rebalance | Warn when an asset has no stored snapshot | R-507 | sonnet | crates/live/src/rebalance.rs, crates/live/tests/rebalance_asset_missing.rs | cargo test -p pt-live --test rebalance_asset_missing |
| O73 | live-rebalance | Pin data_asof and the dry-run id prefix | R-509, R-510 | sonnet | crates/live/tests/rebalance_pins.rs | cargo test -p pt-live --test rebalance_pins |
| O74 | live-rebalance | Pin the bootstrap rule, late data and the rebalance warnings | R-511, R-512, R-513 | sonnet | crates/live/tests/rebalance_late_pins.rs | cargo test -p pt-live --test rebalance_late_pins |
| V12 | validate-capital | Take starting capital out of the trial hash | R-367 | opus | crates/validate/src/runner.rs, crates/validate/tests/trial_hash.rs | cargo test -p pt-validate --test trial_hash |
| V13 | validate-capital | Store the capital and its source with each recorded run | R-525, R-526 | sonnet | crates/validate/src/runner.rs, crates/validate/tests/run_capital.rs | cargo test -p pt-validate --test run_capital |
| R38 | cli-modules | Move the run and backtest argument types into their command modules | R-536, R-541 | opus | crates/cli/src/cli.rs, crates/cli/src/main.rs, crates/cli/src/cmd_run.rs, crates/cli/src/cmd_backtest.rs | cargo test -p pt-cli --test help |
| R39 | cli-modules | Assemble the report inputs in one shared module | R-538 | opus | crates/cli/src/main.rs, crates/cli/src/reporting.rs, crates/cli/src/cmd_run.rs, crates/cli/src/cmd_ops.rs | cargo test -p pt-cli --test report_status |
| R40 | cli-modules | Bring the command modules' comments up to date | R-409 | sonnet | crates/cli/src/main.rs, crates/cli/src/cmd_config.rs, crates/cli/src/cmd_ops.rs | cargo test -p pt-cli --test help |
| R41 | cli-data-check | List universe symbols with no bars in data check and exit 1 | R-383 | sonnet | crates/cli/src/cmd_data.rs, crates/cli/tests/data_check.rs | cargo test -p pt-cli --test data_check |
| R42 | cli-fixtures-offline | Move the offline fixture into tests/common and use it in backtest_run | R-408 | sonnet | crates/cli/tests/common/mod.rs, crates/cli/tests/common/offline.rs, crates/cli/tests/backtest_run.rs | cargo test -p pt-cli --test backtest_run |
| R43 | cli-fixtures-offline | Use the shared offline fixture in validate_dsr | R-408 | sonnet | crates/cli/tests/common/offline.rs, crates/cli/tests/validate_dsr.rs | cargo test -p pt-cli --test validate_dsr |
| R44 | cli-fixtures-offline | Use the shared offline fixture in validate_pbo | R-408 | sonnet | crates/cli/tests/common/offline.rs, crates/cli/tests/validate_pbo.rs | cargo test -p pt-cli --test validate_pbo |
| R45 | cli-fixtures-offline | Use the shared offline fixture in validate_benchmark | R-408 | sonnet | crates/cli/tests/common/offline.rs, crates/cli/tests/validate_benchmark.rs | cargo test -p pt-cli --test validate_benchmark |
| R46 | cli-fixtures-offline | Use the shared offline fixture in crosscheck | R-408 | sonnet | crates/cli/tests/common/offline.rs, crates/cli/tests/crosscheck.rs | cargo test -p pt-cli --test crosscheck |
| R47 | cli-validate-pins | Pin that validate dsr counts only its own family's trials | R-548 | sonnet | crates/cli/tests/validate_dsr.rs | cargo test -p pt-cli --test validate_dsr |
| R48 | cli-validate-pins | Pin the output labels of the validation commands | R-549 | sonnet | crates/cli/tests/validate_labels.rs | cargo test -p pt-cli --test validate_labels |
| O75 | live-cycle-pins | Pin that the first-order stage never rebalances and obeys a halt | R-501, R-502 | sonnet | crates/live/tests/cycle_first_order_pins.rs | cargo test -p pt-live --test cycle_first_order_pins |
| O76 | live-cycle-pins | Pin that a failed run sends nothing and raises one critical alert | R-503, R-504 | sonnet | crates/live/tests/cycle_failed_pins.rs | cargo test -p pt-live --test cycle_failed_pins |
| O77 | live-cycle-pins | Pin that deferred and skipped rebalances send nothing | R-508 | sonnet | crates/live/tests/cycle_no_submit.rs | cargo test -p pt-live --test cycle_no_submit |
| O78 | live-alert-pins | Pin the warning level of the order and snapshot alerts | R-505 | sonnet | crates/live/tests/alert_levels.rs | cargo test -p pt-live --test alert_levels |
| O79 | live-alert-pins | Prove a flatten before the first reconciliation leaves a clean book | R-491 | opus | crates/live/tests/flatten_then_open.rs | cargo test -p pt-live --test flatten_then_open |
| V14 | validate-pins | Share the runner's set-up with crosscheck and pin comparison on common dates | R-527 | sonnet | crates/validate/src/crosscheck.rs, crates/validate/src/runner.rs, crates/validate/tests/common_dates.rs | cargo test -p pt-validate --test common_dates |
| V15 | validate-pins | Pin that the statistics ignore the stored config hash | R-528 | sonnet | crates/validate/tests/config_mismatch.rs | cargo test -p pt-validate --test config_mismatch |
| V16 | validate-pins | Pin the error for an equity curve with a non-finite return | R-529 | sonnet | crates/validate/tests/non_finite_equity.rs | cargo test -p pt-validate --test non_finite_equity |
| R49 | cli-kill-switch | Check --yes first and run the kill switch in every stage | R-384, R-396 | opus | crates/cli/src/main.rs, crates/cli/src/cmd_kill.rs, crates/cli/tests/killswitch.rs, crates/cli/tests/stage_refusal.rs, crates/cli/tests/missing_keys.rs | cargo test -p pt-cli --test killswitch |
| R50 | cli-kill-switch | Print the three flatten counts and exit 1 on a rejection | R-532 | opus | crates/cli/src/cmd_kill.rs, crates/cli/tests/killswitch_flatten.rs | cargo test -p pt-cli --test killswitch_flatten |
| R51 | cli-kill-switch | Pin that the kill-switch commands print no secret | R-533 | sonnet | crates/cli/tests/killswitch_secrets.rs | cargo test -p pt-cli --test killswitch_secrets |
| R52 | cli-run | Take the run lock before opening the store | R-534 | opus | crates/cli/src/cmd_run.rs, crates/cli/tests/run_exit_codes.rs | cargo test -p pt-cli --test run_exit_codes |
| R53 | cli-run | Write the report whatever the cycle returned | R-535 | sonnet | crates/cli/src/cmd_run.rs, crates/cli/tests/run_report.rs | cargo test -p pt-cli --test run_report |
| R54 | cli-run | Add run --if-due | R-536, R-537 | opus | crates/cli/src/cmd_run.rs, crates/cli/tests/run_if_due.rs | cargo test -p pt-cli --test run_if_due |
| R55 | cli-report | Fill the fidelity section of the status report | R-538 | sonnet | crates/cli/src/reporting.rs, crates/cli/tests/report_fidelity.rs | cargo test -p pt-cli --test report_fidelity |
| R56 | cli-report | Report and sign off on an empty store without a special case | R-539 | sonnet | crates/cli/src/cmd_ops.rs, crates/cli/tests/report_status.rs, crates/cli/tests/signoff.rs | cargo test -p pt-cli --test report_status |
| R57 | cli-report | Pin the heartbeat check after a real run | R-540 | sonnet | crates/cli/tests/status_after_run.rs | cargo test -p pt-cli --test status_after_run |
| R58 | cli-gates | Show whether the first order was accepted in gates | R-530 | sonnet | crates/cli/src/cmd_config.rs, crates/cli/tests/gates_first_order.rs | cargo test -p pt-cli --test gates_first_order |
| R59 | cli-gates | Let the webhook checklist item see the environment variable | R-531 | sonnet | crates/cli/src/cmd_config.rs, crates/cli/tests/gates_webhook.rs | cargo test -p pt-cli --test gates_webhook |
| R60 | cli-backtest | Add backtest run --capital | R-541 | sonnet | crates/cli/src/cmd_backtest.rs, crates/cli/tests/backtest_capital.rs | cargo test -p pt-cli --test backtest_capital |
| R61 | cli-backtest | Use the configured capital and print the source | R-542, R-543 | sonnet | crates/cli/src/cmd_backtest.rs, crates/cli/tests/backtest_capital.rs | cargo test -p pt-cli --test backtest_capital |
| R62 | cli-backtest | Use the account's equity when keys are set and the stage allows it | R-544, R-545, R-546 | opus | crates/cli/src/cmd_backtest.rs, crates/cli/tests/backtest_capital_account.rs | cargo test -p pt-cli --test backtest_capital_account |
| R63 | cli-backtest | Add backtest crosscheck --capital without reading the account | R-547 | sonnet | crates/cli/src/cmd_backtest.rs, crates/cli/tests/crosscheck.rs | cargo test -p pt-cli --test crosscheck |
| R64 | cli-deploy | Fire the example schedules every fifteen minutes with --if-due | R-400 | sonnet | deploy/cron.example, deploy/launchd/com.paper-trader.run.plist.example, crates/cli/tests/deploy_files.rs | cargo test -p pt-cli --test deploy_files |
| R65 | cli-e2e | Check every help text against the list of designed flags | R-550 | sonnet | crates/cli/tests/help_flags.rs | cargo test -p pt-cli --test help_flags |
| R66 | cli-e2e | Prove a flatten in the ingest stage and a clean paper start end to end | R-551 | opus | crates/cli/tests/e2e_kill_switch.rs | cargo test -p pt-cli --test e2e_kill_switch |
| W09 | hygiene-scan | Scan that shared helpers have one definition | R-404, R-405, R-406, R-407 | sonnet | crates/archtest/tests/single_definitions.rs | cargo test -p pt-archtest --test single_definitions |
| W10 | hygiene-scan | Scan the shared test fixtures and the corrected comments | R-408, R-409, R-410, R-411 | sonnet | crates/archtest/tests/stale_text.rs | cargo test -p pt-archtest --test stale_text |

## Waves

Output of `chains.sh --check docs/design/paper-trader-fixes/plan.md`. The number after each chain is its step count.

Wave | Chains that can run together
--- | ---
1 | scaffold-workspace (2), scaffold-core (1), scaffold-store (1), scaffold-alpaca (3), scaffold-gate (2), scaffold-live (1), mock-hooks (3), config-backtest (2), strategy-guards (4), stats-guards (4), portfolio-pins (1), risk-pins (1), vector-pins (1), ops-alerts (2), ops-report (3), core-view-doc (1)
2 | store-market (3), store-trading (7), store-ops (5), alpaca-paths (2), alpaca-pins (3), gate-authorize (4), gate-checklist (1), live-adopt (1), live-slots (1), live-submit (2), data-quality (2), data-pins (3), engine-sim (3), engine-decide (3), shadow-fidelity (3), shadow-pins (2), cli-shared (5), cli-fixtures-live (5)
3 | live-kill-switch (6), live-first-order (7), live-settle (6), live-rebalance (4), validate-capital (2), cli-modules (3), cli-data-check (1), cli-fixtures-offline (5)
4 | cli-validate-pins (2), live-cycle-pins (3), live-alert-pins (2), validate-pins (3), cli-kill-switch (3), cli-run (3), cli-report (3), cli-gates (2), cli-backtest (4)
5 | cli-deploy (1), cli-e2e (2)
6 | hygiene-scan (2)

54 chains, 151 steps. Widest wave: 18 chains. Longest chain: 7 steps.

What the waves mean in practice:

- Wave 1 holds six scaffold chains, one per crate that has a change other crates see at compile time: a required field, a changed signature, new module lines, the migration, the manifests. Each is one to three steps. Behind them the chains touch disjoint files and run side by side.
- `scaffold-gate` makes `qty` required in `[first_order]`. From the moment it lands, an owner file that has a `[first_order]` table without `qty` fails to load. The owner adds `qty` before running a binary built after wave 1.
- Between `gate-authorize` (wave 2) and `live-first-order` (wave 3) the gate authorises the first order only for `qty` while the cycle still plans `max_qty`. If the two differ the order is refused, which fails closed. They are equal in every fixture.
- The first-order retry is complete at wave 3. The kill switch in every stage is complete at wave 4 (`cli-kill-switch`); before that the gate and the live crate allow it but the command still asks for the old stages.
- `trader run --if-due` exists from wave 4 and the example schedules change in wave 5. An installed schedule should be replaced only after `cli-deploy` has landed: the old three-entry schedule keeps working with every earlier wave.
- `live-slots` adds `chrono-tz`, which is not in this machine's cargo cache. That one step needs network access to crates.io.
- Wave 6 is the two scans. They come last because they fail while any duplicate this plan removes is still there.
- No chain waits for an owner step.

## Reading order

One line per chain: what to look for when reviewing it.

- `scaffold-workspace`: Check the licence function against the eleven cases of the requirement, and that each removed dependency really is unused: the scan, not the gap list, decides.
- `scaffold-core`: One new pair in the transition table; confirm every other pair from Rejected is still refused.
- `scaffold-store`: The migration must only add: two columns on backtest_runs, one on orders, two triggers. Check the test opens a real version-4 file.
- `scaffold-alpaca`: The first step changes a signature used by every alpaca test and by wiring.rs; look for a limit of 0 slipping through anywhere else. is_order_reject must be true for 403 and 422 only.
- `scaffold-gate`: The first step touches many fixtures: each should gain qty equal to its max_qty and nothing else. An owner file without qty must fail to parse.
- `scaffold-live`: Nothing but module lines, three empty files and one error variant.
- `mock-hooks`: A refused request must be recorded and must change no state; reject_but_keep must leave an order a lookup can find.
- `config-backtest`: The config hash test carries a literal from before the change; it must not move. Zero is refused for each of the three minimums by name.
- `strategy-guards`: No formula changes: the EWMA refactor must leave every existing volatility and covariance test untouched.
- `stats-guards`: Typed errors for the functions that return Result, NaN for the ones that do not; a constant x in ols keeps its old error.
- `portfolio-pins`: Test only. It should fail if a zero-risk portfolio ever gets a non-zero weight.
- `risk-pins`: Test only. The reducing order is rejected too: that is the decision, not a bug.
- `vector-pins`: Test only.
- `ops-alerts`: The code is scrubbed before it is stored, logged and posted. A redirect is one request and a failed delivery.
- `ops-report`: Pnl fields become options; check status.md and status.json both handle an absent value. The schema test should fail on any added or removed key.
- `core-view-doc`: A comment only.
- `store-market`: Each refusal must leave the whole batch unstored, the valid rows included.
- `store-trading`: The record_fill and decision steps are the two transactions: look for any path that commits half. The filled-quantity check must not refuse a repeat of the same quantity.
- `store-ops`: The already-cleared error keeps the NotFound variant. Runs saved the old way read back with no capital.
- `alpaca-paths`: The existing percent_encode helper is reused, not copied.
- `alpaca-pins`: Tests only, against a loopback server.
- `gate-authorize`: The heart of the kill-switch change. authorize_kill_switch must check the id prefix, the side and the size against the broker's position and nothing about stage or halts; authorize must be unchanged for every other order.
- `gate-checklist`: Thirteen items in the stated order; the command test for gates moves from twelve lines to thirteen in the same commit.
- `live-adopt`: It must send no order, ever: one lookup, then at most one state change.
- `live-slots`: A pure function of two instants. Check the fixed instants on both sides of both daylight-saving changes, and that chrono-tz is the only new dependency.
- `live-submit`: A 401 must leave the row as an intent and stop the batch; only 403 and 422 mark it rejected.
- `data-quality`: check_gaps is unchanged; the new function reports the symbols it skips.
- `data-pins`: Tests only.
- `engine-sim`: One explicit range check; the other two steps are tests.
- `engine-decide`: decide must give the same decision as before with one strategy call instead of two.
- `shadow-fidelity`: n_rebalances now excludes skipped decisions, which makes the sign-off minimum harder to reach, as intended.
- `shadow-pins`: Tests only.
- `cli-shared`: Refactoring with no behaviour change: every existing command test must pass untouched. The last step only moves the kill-switch code; its behaviour changes later, in cli-kill-switch.
- `cli-fixtures-live`: Test code only, mostly deletions: one copy of the fixture moves to tests/common and four more are deleted. No assertion should change.
- `live-kill-switch`: Read with gate-authorize beside it. Order of work inside flatten: halt, cancel, settle, read positions, authorise each closing order against the broker's position, send. Check the off-ledger mark is set only while the ledger is empty.
- `live-first-order`: The riskiest chain. Follow one approval through six attempts: ids, waits, the lookup before each retry, and what counts as used up. Then the edited approval before and after acceptance.
- `live-settle`: The re-add fallback goes once record_fill is used. Leg 2 is abandoned, never released, after a short leg 1, and no halt is set.
- `live-rebalance`: The backfill from outputs_json goes once the write is one transaction. Two steps are tests only.
- `validate-capital`: The hash test carries a literal computed before the change: it must hold at both capitals.
- `cli-modules`: Refactoring with no behaviour change, so that later command chains do not meet in main.rs or cli.rs.
- `cli-data-check`: One new output line, and exit 1.
- `cli-fixtures-offline`: The same for the offline backtest fixture. No assertion should change.
- `cli-validate-pins`: Tests only.
- `live-cycle-pins`: Tests only, at the level of run_cycle.
- `live-alert-pins`: Tests only. The last step is the proof that a flatten made before the paper book exists does not corrupt it.
- `validate-pins`: The first step refactors crosscheck's set-up; its results must not move.
- `cli-kill-switch`: No stage check and no key is loaded before --yes is seen. Exit 4 is gone for these two commands.
- `cli-run`: Lock, then store. The not-due path makes no request and writes no run row.
- `cli-report`: The same ten labels from run and from report.
- `cli-gates`: gates must stay offline and must not create a database.
- `cli-backtest`: Follow the source order: flag, account, config. In the backtest stage no request may be made, even with keys.
- `cli-deploy`: Two example files and their test.
- `cli-e2e`: Tests only: the whole kill-switch story through the binary.
- `hygiene-scan`: Two scans that fail if a duplicate or a stale comment comes back.

## Notes for the owner and the integrator

These are not steps. They are outside what a chain may edit, or outside the scope the owner set (architecture F-29).

- `check.sh` should log the name of a failing test, and should not share one `.devteam/state/last-check.log` between concurrent checks.
- Each worktree, reviewer and integrator should have its own `CARGO_TARGET_DIR`. A shared one left test binaries with another worktree's path baked in.
- The remote branch `integration/paper-trader` is stale. Updating it needs a force push, which the workflow forbids.
- The three flaky tests of the gap list have no known cause in test code and are not changed here.
- `crates/cli/src/wiring.rs` builds the client with a constant of 180 requests per minute and ignores `data.max_requests_per_minute`. Found while designing; not in the gap list.
- Two steps of the old plan ran over the size guideline because one test file carried a large fixture. This plan moves those fixtures to `tests/common` first (`cli-fixtures-live`, `cli-fixtures-offline`), so the command chains behind them add small test files.

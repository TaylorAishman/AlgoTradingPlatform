//! P08 (R-332): replay uses the bars as they were when the decision was made.

mod replay_fixture;

use pt_core::types::{Adjustment, PriceBasis};
use pt_data::view::StoreView;
use pt_engine::decision::{decide, decision_to_json, params_from_config};
use pt_shadow::replay::replay_decision;
use replay_fixture::{bar_rows, d, sym, t2, Fixture};
use rust_decimal::Decimal;

/// Restates SPY at t2: every close from 2024-02-15 on is halved.
fn restate_spy(f: &Fixture, adjustments: &[Adjustment]) {
    for adj in adjustments {
        f.store
            .upsert_bars(
                &bar_rows("SPY", *adj, d(2024, 2, 15), Decimal::new(5, 1)),
                t2(),
            )
            .unwrap();
    }
}

/// What the decision would be if it were recomputed over the restated bars.
fn decision_over_restated_bars(f: &Fixture) -> String {
    let symbols = vec![sym("SPY"), sym("TLT"), sym("BIL")];
    let view = StoreView::load(&f.store, &symbols, "sip", PriceBasis::Raw, Some(t2())).unwrap();
    let decision = decide(
        &f.record.inputs,
        &f.strategy,
        &view,
        &params_from_config(&f.cfg),
    )
    .unwrap();
    decision_to_json(&decision)
}

#[test]
fn replay_ignores_adjusted_bars_restated_after_the_decision() {
    // R-332
    let f = replay_fixture::build();
    restate_spy(&f, &[Adjustment::All]);
    let result = replay_decision(&f.store, &f.cfg, &f.strategy, f.decision_id).unwrap();
    assert!(result.matches, "diffs: {:?}", result.diffs);
    assert!(result.diffs.is_empty(), "diffs: {:?}", result.diffs);
}

#[test]
fn replay_ignores_raw_and_adjusted_bars_restated_after_the_decision() {
    // R-332
    let f = replay_fixture::build();
    restate_spy(&f, &[Adjustment::Raw, Adjustment::All]);
    let result = replay_decision(&f.store, &f.cfg, &f.strategy, f.decision_id).unwrap();
    assert!(result.matches, "diffs: {:?}", result.diffs);
    assert!(result.diffs.is_empty(), "diffs: {:?}", result.diffs);
}

#[test]
fn the_restatement_would_change_the_decision_if_it_were_seen() {
    // R-332: guards the test, so a match is not just a restatement that changes nothing.
    let f = replay_fixture::build();
    restate_spy(&f, &[Adjustment::Raw, Adjustment::All]);
    assert_ne!(
        decision_over_restated_bars(&f),
        f.row.outputs_json,
        "the restatement must flip the decision for this test to mean anything"
    );
}

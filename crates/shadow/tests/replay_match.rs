//! P06 (R-330): a stored decision is reproducible from its stored inputs.

mod replay_fixture;

use pt_shadow::replay::replay_decision;

#[test]
fn the_fixture_decision_replays_with_no_diffs() {
    // R-330
    let f = replay_fixture::build();
    let result = replay_decision(&f.store, &f.cfg, &f.strategy, f.decision_id).unwrap();
    assert!(result.matches, "diffs: {:?}", result.diffs);
    assert!(result.diffs.is_empty(), "diffs: {:?}", result.diffs);
}

#[test]
fn the_result_names_the_decision_that_was_replayed() {
    // R-330
    let f = replay_fixture::build();
    let result = replay_decision(&f.store, &f.cfg, &f.strategy, f.decision_id).unwrap();
    assert_eq!(result.decision_id, f.decision_id);
}

#[test]
fn the_fixture_decision_trades_both_symbols() {
    // R-330: guards the fixture, so a match is not an empty decision matching itself.
    let f = replay_fixture::build();
    assert!(f.row.outputs_json.contains("\"SPY\""));
    assert!(f.row.outputs_json.contains("\"TLT\""));
    assert!(
        f.row.outputs_json.contains("\"orders\":[{"),
        "{}",
        f.row.outputs_json
    );
}

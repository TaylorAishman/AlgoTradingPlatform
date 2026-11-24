//! R-264: in the paper stage an order is authorised only for an allow-listed
//! strategy, and the permit is bound to exactly that order.

use chrono::NaiveDate;
use pt_gate::error::GateError;
use pt_gate::gate::{AuthContext, Gate, OrderSummary};
use pt_gate::owner::{OwnerFile, Stage};

fn day(d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 12, d).unwrap()
}

fn paper_gate(allowlist: &[&str]) -> Gate {
    Gate::new(OwnerFile {
        stage: Stage::Paper,
        strategy_allowlist: allowlist.iter().map(|s| s.to_string()).collect(),
        account_flat_confirmed_on: Some(day(1)),
        ..OwnerFile::default()
    })
}

fn order() -> OrderSummary {
    OrderSummary {
        client_order_id: "fake-id-1".to_string(),
        symbol: "SPY".to_string(),
        side: "buy".to_string(),
        qty: 10,
    }
}

fn ctx(strategy: &str) -> AuthContext {
    AuthContext {
        strategy_id: strategy.to_string(),
        today: day(2),
        halted: false,
        risk_reducing: false,
        is_first_order: false,
        first_order_already_sent: false,
    }
}

#[test]
fn allow_listed_strategy_gets_a_permit_matching_the_same_order() {
    // R-264
    let gate = paper_gate(&["tsmom-v1"]);
    let permit = gate
        .authorize(&order(), &ctx("tsmom-v1"))
        .unwrap_or_else(|e| panic!("allow-listed strategy must be authorised: {e}"));
    assert!(permit.matches(&order()));
}

#[test]
fn permit_does_not_match_when_client_order_id_differs() {
    // R-264
    let permit = paper_gate(&["tsmom-v1"])
        .authorize(&order(), &ctx("tsmom-v1"))
        .ok()
        .unwrap();
    let mut other = order();
    other.client_order_id = "fake-id-2".to_string();
    assert!(!permit.matches(&other));
}

#[test]
fn permit_does_not_match_when_symbol_differs() {
    // R-264
    let permit = paper_gate(&["tsmom-v1"])
        .authorize(&order(), &ctx("tsmom-v1"))
        .ok()
        .unwrap();
    let mut other = order();
    other.symbol = "QQQ".to_string();
    assert!(!permit.matches(&other));
}

#[test]
fn permit_does_not_match_when_side_differs() {
    // R-264
    let permit = paper_gate(&["tsmom-v1"])
        .authorize(&order(), &ctx("tsmom-v1"))
        .ok()
        .unwrap();
    let mut other = order();
    other.side = "sell".to_string();
    assert!(!permit.matches(&other));
}

#[test]
fn permit_does_not_match_when_qty_differs() {
    // R-264
    let permit = paper_gate(&["tsmom-v1"])
        .authorize(&order(), &ctx("tsmom-v1"))
        .ok()
        .unwrap();
    let mut other = order();
    other.qty = 11;
    assert!(!permit.matches(&other));
    other.qty = 0;
    assert!(!permit.matches(&other));
}

#[test]
fn permit_is_bound_to_the_order_it_was_issued_for_not_the_last_one() {
    // R-264: two permits from the same gate each match only their own order.
    let gate = paper_gate(&["tsmom-v1"]);
    let mut second = order();
    second.client_order_id = "fake-id-2".to_string();
    let p1 = gate.authorize(&order(), &ctx("tsmom-v1")).ok().unwrap();
    let p2 = gate.authorize(&second, &ctx("tsmom-v1")).ok().unwrap();
    assert!(p1.matches(&order()) && !p1.matches(&second));
    assert!(p2.matches(&second) && !p2.matches(&order()));
}

#[test]
fn strategy_not_on_the_allowlist_is_refused() {
    // R-264
    let result = paper_gate(&["tsmom-v1"]).authorize(&order(), &ctx("other"));
    assert_eq!(result.err(), Some(GateError::StrategyNotAllowed));
}

#[test]
fn empty_allowlist_refuses_every_strategy() {
    // R-264: fail closed, nothing approved means nothing allowed.
    let gate = paper_gate(&[]);
    for strategy in ["tsmom-v1", "other", ""] {
        assert_eq!(
            gate.authorize(&order(), &ctx(strategy)).err(),
            Some(GateError::StrategyNotAllowed),
            "strategy {strategy:?}"
        );
    }
}

#[test]
fn strategy_match_is_exact_not_fuzzy() {
    // R-264: case, whitespace, prefix, suffix and empty id are all refused.
    let gate = paper_gate(&["tsmom-v1"]);
    for strategy in [
        "TSMOM-V1",
        "Tsmom-v1",
        " tsmom-v1",
        "tsmom-v1 ",
        "tsmom",
        "tsmom-v1-extra",
        "tsmom-v",
        "",
    ] {
        assert_eq!(
            gate.authorize(&order(), &ctx(strategy)).err(),
            Some(GateError::StrategyNotAllowed),
            "strategy {strategy:?}"
        );
    }
}

#[test]
fn empty_allowlist_entry_does_not_approve_an_empty_strategy_id_by_accident() {
    // R-264: an allowlist of only other names refuses the empty id.
    let gate = paper_gate(&["tsmom-v1", "other-v2"]);
    assert_eq!(
        gate.authorize(&order(), &ctx("")).err(),
        Some(GateError::StrategyNotAllowed)
    );
}

#[test]
fn any_entry_of_a_multi_entry_allowlist_is_accepted() {
    // R-264
    let gate = paper_gate(&["tsmom-v1", "other-v2"]);
    for strategy in ["tsmom-v1", "other-v2"] {
        let permit = gate.authorize(&order(), &ctx(strategy)).ok().unwrap();
        assert!(permit.matches(&order()));
    }
}

#[test]
fn authorize_is_repeatable_in_paper_stage() {
    // R-264: paper stage is not a one-shot stage.
    let gate = paper_gate(&["tsmom-v1"]);
    assert!(gate.authorize(&order(), &ctx("tsmom-v1")).is_ok());
    assert!(gate.authorize(&order(), &ctx("tsmom-v1")).is_ok());
}

#[test]
fn stages_below_first_order_refuse_even_an_allow_listed_strategy() {
    // R-264 fail-closed, overlapping R-269: stage wins over the allowlist.
    for stage in [Stage::Backtest, Stage::Ingest, Stage::DryRun] {
        let gate = Gate::new(OwnerFile {
            stage,
            strategy_allowlist: vec!["tsmom-v1".to_string()],
            account_flat_confirmed_on: Some(day(1)),
            ..OwnerFile::default()
        });
        assert_eq!(
            gate.authorize(&order(), &ctx("tsmom-v1")).err(),
            Some(GateError::StageForbids),
            "{stage:?}"
        );
    }
}

#[test]
fn default_owner_file_authorises_nothing() {
    // R-264 fail-closed
    let gate = Gate::new(OwnerFile::default());
    assert!(gate.authorize(&order(), &ctx("tsmom-v1")).is_err());
}

//! R-266: a halt blocks every order except one that reduces risk.

use chrono::NaiveDate;
use pt_gate::error::GateError;
use pt_gate::gate::{AuthContext, Gate, OrderSummary};
use pt_gate::owner::{OwnerFile, Stage};

fn day(d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 12, d).unwrap()
}

fn paper_gate() -> Gate {
    Gate::new(OwnerFile {
        stage: Stage::Paper,
        strategy_allowlist: vec!["tsmom-v1".to_string()],
        account_flat_confirmed_on: Some(day(1)),
        ..OwnerFile::default()
    })
}

fn order() -> OrderSummary {
    OrderSummary {
        client_order_id: "fake-id-1".to_string(),
        symbol: "SPY".to_string(),
        side: "sell".to_string(),
        qty: 10,
    }
}

fn ctx(strategy: &str, halted: bool, risk_reducing: bool) -> AuthContext {
    AuthContext {
        strategy_id: strategy.to_string(),
        today: day(2),
        halted,
        risk_reducing,
        is_first_order: false,
        first_order_already_sent: false,
    }
}

#[test]
fn halted_blocks_an_order_that_does_not_reduce_risk() {
    // R-266
    let result = paper_gate().authorize(&order(), &ctx("tsmom-v1", true, false));
    assert_eq!(result.err(), Some(GateError::Halted));
}

#[test]
fn halted_still_allows_a_risk_reducing_order() {
    // R-266
    let permit = paper_gate()
        .authorize(&order(), &ctx("tsmom-v1", true, true))
        .unwrap_or_else(|e| panic!("risk-reducing order must pass a halt: {e}"));
    assert!(permit.matches(&order()));
}

#[test]
fn not_halted_ignores_the_risk_reducing_flag() {
    // R-266
    for risk_reducing in [false, true] {
        let result = paper_gate().authorize(&order(), &ctx("tsmom-v1", false, risk_reducing));
        assert!(result.is_ok(), "risk_reducing={risk_reducing}");
    }
}

#[test]
fn a_halt_does_not_waive_the_strategy_allowlist() {
    // R-266 fail closed: a risk-reducing order from an unknown strategy is refused.
    let result = paper_gate().authorize(&order(), &ctx("other", true, true));
    assert_eq!(result.err(), Some(GateError::StrategyNotAllowed));
}

#[test]
fn a_halt_does_not_waive_the_stage() {
    // R-266 fail closed: a risk-reducing order below the paper stage is refused.
    for stage in [Stage::Backtest, Stage::Ingest, Stage::DryRun] {
        let gate = Gate::new(OwnerFile {
            stage,
            strategy_allowlist: vec!["tsmom-v1".to_string()],
            account_flat_confirmed_on: Some(day(1)),
            ..OwnerFile::default()
        });
        assert_eq!(
            gate.authorize(&order(), &ctx("tsmom-v1", true, true)).err(),
            Some(GateError::StageForbids),
            "{stage:?}"
        );
    }
}

#[test]
fn halted_permit_is_bound_to_the_order_it_was_issued_for() {
    // R-266
    let permit = paper_gate()
        .authorize(&order(), &ctx("tsmom-v1", true, true))
        .ok()
        .unwrap();
    let mut other = order();
    other.qty = 11;
    assert!(!permit.matches(&other));
}

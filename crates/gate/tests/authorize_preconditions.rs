//! R-267: no order before the owner confirms the account is flat.
//! R-269: below the first-order stage no order is ever authorised.

use chrono::NaiveDate;
use pt_gate::error::GateError;
use pt_gate::gate::{AuthContext, Gate, OrderSummary};
use pt_gate::owner::{FirstOrderApproval, OwnerFile, Stage};

fn day(d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 12, d).unwrap()
}

fn gate(stage: Stage, flat: Option<NaiveDate>) -> Gate {
    Gate::new(OwnerFile {
        stage,
        strategy_allowlist: vec!["tsmom-v1".to_string()],
        account_flat_confirmed_on: flat,
        first_order: Some(FirstOrderApproval {
            symbol: "SPY".to_string(),
            side: "buy".to_string(),
            qty: 1,
            max_qty: 1,
            valid_on: day(2),
        }),
        ..OwnerFile::default()
    })
}

fn order() -> OrderSummary {
    OrderSummary {
        client_order_id: "fake-id-1".to_string(),
        symbol: "SPY".to_string(),
        side: "buy".to_string(),
        qty: 1,
    }
}

fn ctx(halted: bool, risk_reducing: bool, is_first_order: bool) -> AuthContext {
    AuthContext {
        strategy_id: "tsmom-v1".to_string(),
        today: day(2),
        halted,
        risk_reducing,
        is_first_order,
        first_order_already_sent: false,
    }
}

#[test]
fn paper_without_flat_confirmation_is_refused() {
    // R-267
    let result = gate(Stage::Paper, None).authorize(&order(), &ctx(false, false, false));
    assert_eq!(result.err(), Some(GateError::FlatNotConfirmed));
}

#[test]
fn paper_without_flat_confirmation_is_refused_even_when_risk_reducing_and_halted() {
    // R-267 fail closed: no flag waives the confirmation.
    for (halted, risk_reducing) in [(false, true), (true, true), (true, false)] {
        let result =
            gate(Stage::Paper, None).authorize(&order(), &ctx(halted, risk_reducing, false));
        assert_eq!(
            result.err(),
            Some(GateError::FlatNotConfirmed),
            "halted={halted} risk_reducing={risk_reducing}"
        );
    }
}

#[test]
fn first_order_stage_without_flat_confirmation_is_refused() {
    // R-267
    for is_first in [false, true] {
        let result =
            gate(Stage::FirstOrder, None).authorize(&order(), &ctx(false, false, is_first));
        assert_eq!(
            result.err(),
            Some(GateError::FlatNotConfirmed),
            "is_first_order={is_first}"
        );
    }
}

#[test]
fn paper_with_flat_confirmation_is_authorised() {
    // R-267 positive control
    let result = gate(Stage::Paper, Some(day(1))).authorize(&order(), &ctx(false, false, false));
    assert!(result.is_ok());
}

#[test]
fn below_first_order_every_order_and_context_is_refused() {
    // R-269, with and without the flat confirmation, any flags.
    for stage in [Stage::Backtest, Stage::Ingest, Stage::DryRun] {
        for flat in [None, Some(day(1))] {
            for halted in [false, true] {
                for risk_reducing in [false, true] {
                    for is_first in [false, true] {
                        let result = gate(stage, flat)
                            .authorize(&order(), &ctx(halted, risk_reducing, is_first));
                        assert_eq!(
                            result.err(),
                            Some(GateError::StageForbids),
                            "{stage:?} flat={flat:?} halted={halted} rr={risk_reducing} first={is_first}"
                        );
                    }
                }
            }
        }
    }
}

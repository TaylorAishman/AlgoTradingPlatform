//! The gate reports the first order the owner approved. Reading it grants
//! nothing: `authorize` stays the only way to get a permit. (Enables R-319.)

use chrono::NaiveDate;
use pt_gate::error::GateError;
use pt_gate::gate::{AuthContext, Gate, OrderSummary};
use pt_gate::owner::{FirstOrderApproval, OwnerFile, Stage};

fn day(d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 12, d).unwrap()
}

fn approval() -> FirstOrderApproval {
    FirstOrderApproval {
        symbol: "SPY".to_string(),
        side: "buy".to_string(),
        max_qty: 1,
        valid_on: day(2),
    }
}

fn gate() -> Gate {
    Gate::new(OwnerFile {
        stage: Stage::FirstOrder,
        account_flat_confirmed_on: Some(day(1)),
        first_order: Some(approval()),
        ..OwnerFile::default()
    })
}

#[test]
fn first_order_returns_the_approval_as_the_owner_file_holds_it() {
    let g = gate();
    assert_eq!(g.first_order(), Some(&approval()));
}

#[test]
fn first_order_is_none_without_an_approval() {
    let g = Gate::new(OwnerFile {
        stage: Stage::FirstOrder,
        account_flat_confirmed_on: Some(day(1)),
        first_order: None,
        ..OwnerFile::default()
    });
    assert_eq!(g.first_order(), None);
    assert_eq!(Gate::new(OwnerFile::default()).first_order(), None);
}

#[test]
fn first_order_reports_the_file_in_every_stage_without_judging_it() {
    for stage in [
        Stage::Backtest,
        Stage::Ingest,
        Stage::DryRun,
        Stage::FirstOrder,
        Stage::Paper,
    ] {
        let g = Gate::new(OwnerFile {
            stage,
            first_order: Some(approval()),
            ..OwnerFile::default()
        });
        assert_eq!(g.first_order(), Some(&approval()), "stage {stage:?}");
    }
}

#[test]
fn an_order_built_from_the_approval_is_authorised_and_reading_grants_nothing() {
    let g = gate();
    let a = g.first_order().expect("approval").clone();
    let order = |symbol: &str| OrderSummary {
        client_order_id: "fake-id-1".to_string(),
        symbol: symbol.to_string(),
        side: a.side.clone(),
        qty: a.max_qty,
    };
    let ctx = AuthContext {
        strategy_id: "tsmom-v1".to_string(),
        today: a.valid_on,
        halted: false,
        risk_reducing: false,
        is_first_order: true,
        first_order_already_sent: false,
    };
    let own = order(&a.symbol);
    let permit = g.authorize(&own, &ctx).expect("permit");
    assert!(permit.matches(&own));
    assert_eq!(
        g.authorize(&order("QQQ"), &ctx).err(),
        Some(GateError::FirstOrderMismatch)
    );
}

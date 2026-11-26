//! R-268: in the first-order stage only the approved order may be authorised,
//! once.

use chrono::NaiveDate;
use pt_gate::error::GateError;
use pt_gate::gate::{AuthContext, Gate, OrderSummary};
use pt_gate::owner::{FirstOrderApproval, OwnerFile, Stage};

fn day(d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 12, d).unwrap()
}

fn approval(max_qty: u64) -> FirstOrderApproval {
    FirstOrderApproval {
        symbol: "SPY".to_string(),
        side: "buy".to_string(),
        qty: max_qty,
        max_qty,
        valid_on: day(2),
    }
}

fn gate_with(first_order: Option<FirstOrderApproval>, flat: Option<NaiveDate>) -> Gate {
    Gate::new(OwnerFile {
        stage: Stage::FirstOrder,
        strategy_allowlist: vec!["tsmom-v1".to_string()],
        account_flat_confirmed_on: flat,
        first_order,
        ..OwnerFile::default()
    })
}

fn gate() -> Gate {
    gate_with(Some(approval(1)), Some(day(1)))
}

fn order(symbol: &str, side: &str, qty: u64) -> OrderSummary {
    OrderSummary {
        client_order_id: "fake-id-1".to_string(),
        symbol: symbol.to_string(),
        side: side.to_string(),
        qty,
    }
}

fn spy() -> OrderSummary {
    order("SPY", "buy", 1)
}

fn ctx() -> AuthContext {
    AuthContext {
        strategy_id: "tsmom-v1".to_string(),
        today: day(2),
        halted: false,
        risk_reducing: false,
        is_first_order: true,
        first_order_already_sent: false,
    }
}

#[test]
fn approved_first_order_gets_a_permit_bound_to_it() {
    // R-268 positive control
    let permit = gate().authorize(&spy(), &ctx()).expect("permit");
    assert!(permit.matches(&spy()));
    assert!(!permit.matches(&order("SPY", "buy", 2)));
}

#[test]
fn quantity_below_max_is_authorised() {
    // R-268: max_qty is a ceiling.
    let g = gate_with(Some(approval(2)), Some(day(1)));
    assert!(g.authorize(&order("SPY", "buy", 2), &ctx()).is_ok());
    assert!(g.authorize(&order("SPY", "buy", 1), &ctx()).is_ok());
    assert_eq!(
        g.authorize(&order("SPY", "buy", 3), &ctx()).err(),
        Some(GateError::FirstOrderMismatch)
    );
}

#[test]
fn wrong_symbol_is_a_mismatch() {
    // R-268
    for symbol in ["QQQ", "spy", "SPY ", ""] {
        assert_eq!(
            gate().authorize(&order(symbol, "buy", 1), &ctx()).err(),
            Some(GateError::FirstOrderMismatch),
            "symbol {symbol:?}"
        );
    }
}

#[test]
fn wrong_side_is_a_mismatch() {
    // R-268
    for side in ["sell", "BUY", "buy ", ""] {
        assert_eq!(
            gate().authorize(&order("SPY", side, 1), &ctx()).err(),
            Some(GateError::FirstOrderMismatch),
            "side {side:?}"
        );
    }
}

#[test]
fn quantity_over_max_is_a_mismatch() {
    // R-268
    for qty in [2, 100, u64::MAX] {
        assert_eq!(
            gate().authorize(&order("SPY", "buy", qty), &ctx()).err(),
            Some(GateError::FirstOrderMismatch),
            "qty {qty}"
        );
    }
}

#[test]
fn a_day_other_than_valid_on_is_a_mismatch() {
    // R-268: the day before and the day after are both refused.
    for d in [1, 3, 30] {
        let mut c = ctx();
        c.today = day(d);
        assert_eq!(
            gate().authorize(&spy(), &c).err(),
            Some(GateError::FirstOrderMismatch),
            "today 2026-12-{d:02}"
        );
    }
}

#[test]
fn missing_first_order_table_is_a_mismatch() {
    // R-268
    let g = gate_with(None, Some(day(1)));
    assert_eq!(
        g.authorize(&spy(), &ctx()).err(),
        Some(GateError::FirstOrderMismatch)
    );
}

#[test]
fn second_first_order_is_already_sent() {
    // R-268
    let mut c = ctx();
    c.first_order_already_sent = true;
    assert_eq!(
        gate().authorize(&spy(), &c).err(),
        Some(GateError::FirstOrderAlreadySent)
    );
}

#[test]
fn order_not_marked_first_is_refused_by_the_stage() {
    // R-268: only the first order may go in this stage.
    let mut c = ctx();
    c.is_first_order = false;
    assert_eq!(
        gate().authorize(&spy(), &c).err(),
        Some(GateError::StageForbids)
    );
}

#[test]
fn not_first_order_is_refused_even_when_risk_reducing_or_halted() {
    // R-268 fail closed: no flag turns a non-first order into an allowed one.
    for (halted, risk_reducing) in [(false, true), (true, true), (true, false)] {
        let mut c = ctx();
        c.is_first_order = false;
        c.halted = halted;
        c.risk_reducing = risk_reducing;
        assert!(
            gate().authorize(&spy(), &c).is_err(),
            "halted={halted} risk_reducing={risk_reducing}"
        );
    }
}

#[test]
fn flat_confirmation_is_still_required_for_the_approved_order() {
    // R-267 with R-268
    let g = gate_with(Some(approval(1)), None);
    assert_eq!(
        g.authorize(&spy(), &ctx()).err(),
        Some(GateError::FlatNotConfirmed)
    );
}

#[test]
fn literal_r268_scenario_with_empty_allowlist_gets_a_permit() {
    // R-268 acceptance: stage FirstOrder, the approval and the flat
    // confirmation set, no allowlist; order SPY buy 1 gets a permit.
    let g = Gate::new(OwnerFile {
        stage: Stage::FirstOrder,
        account_flat_confirmed_on: Some(day(1)),
        first_order: Some(approval(1)),
        ..OwnerFile::default()
    });
    let permit = g.authorize(&spy(), &ctx()).expect("permit");
    assert!(permit.matches(&spy()));
}

#[test]
fn allowlist_does_not_block_the_approved_first_order() {
    // R-268: the approval is the authorisation in this stage; the allowlist
    // (filled only when moving to Paper) neither blocks nor widens it.
    let empty = Gate::new(OwnerFile {
        stage: Stage::FirstOrder,
        strategy_allowlist: vec![],
        account_flat_confirmed_on: Some(day(1)),
        first_order: Some(approval(1)),
        ..OwnerFile::default()
    });
    let mut other = ctx();
    other.strategy_id = "other".to_string();
    for (g, c) in [(&empty, ctx()), (&empty, other.clone()), (&gate(), other)] {
        assert!(g.authorize(&spy(), &c).is_ok(), "{c:?}");
        // A non-approved order is still refused as a mismatch.
        assert_eq!(
            g.authorize(&order("QQQ", "buy", 1), &c).err(),
            Some(GateError::FirstOrderMismatch),
            "{c:?}"
        );
    }
}

#[test]
fn zero_quantity_is_a_mismatch() {
    // R-268 fail closed: a zero-quantity order is not the approved order.
    for max_qty in [1, 2] {
        let g = gate_with(Some(approval(max_qty)), Some(day(1)));
        assert_eq!(
            g.authorize(&order("SPY", "buy", 0), &ctx()).err(),
            Some(GateError::FirstOrderMismatch),
            "max_qty {max_qty}"
        );
    }
}

#[test]
fn halt_refuses_the_approved_first_order() {
    // R-266 with R-268: a halt allows only risk-reducing orders.
    let mut c = ctx();
    c.halted = true;
    assert_eq!(gate().authorize(&spy(), &c).err(), Some(GateError::Halted));
}

#[test]
fn every_single_deviation_is_refused_and_never_permitted() {
    // R-268 fail closed: nothing but the exact approval produces a permit.
    let mut sent = ctx();
    sent.first_order_already_sent = true;
    let mut other_day = ctx();
    other_day.today = day(3);
    let cases = [
        (order("QQQ", "buy", 1), ctx()),
        (order("SPY", "sell", 1), ctx()),
        (order("SPY", "buy", 2), ctx()),
        (spy(), other_day),
        (spy(), sent),
    ];
    for (o, c) in cases {
        assert!(gate().authorize(&o, &c).is_err(), "{o:?} {c:?}");
    }
}

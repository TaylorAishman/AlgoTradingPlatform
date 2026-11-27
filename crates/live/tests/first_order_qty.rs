//! R-492: the first order is planned and sent for the approved quantity, not `max_qty`.

mod common;

use common::{d, sym, LiveFixture};
use pt_core::order_state::OrderState;
use pt_core::types::Side;
use pt_gate::gate::Gate;
use pt_gate::owner::{FirstOrderApproval, OwnerFile, Stage};
use pt_live::cycle::{run_cycle, RunStatus};

fn approved_fixture() -> LiveFixture {
    let mut fx = LiveFixture::new(Stage::FirstOrder);
    fx.gate = Gate::new(OwnerFile {
        stage: Stage::FirstOrder,
        account_flat_confirmed_on: Some(d(2024, 2, 29)),
        first_order: Some(FirstOrderApproval {
            symbol: "SPY".to_string(),
            side: "buy".to_string(),
            qty: 2,
            max_qty: 5,
            valid_on: d(2024, 3, 1),
        }),
        ..OwnerFile::default()
    });
    fx
}

// R-492
#[test]
fn the_first_order_is_stored_for_the_approved_quantity() {
    let fx = approved_fixture();

    let outcome = run_cycle(&fx.ctx()).expect("run");

    assert_eq!(outcome.status, RunStatus::Ok);
    let orders = fx
        .store
        .orders_in_states(&[
            OrderState::Intent,
            OrderState::Submitted,
            OrderState::Rejected,
        ])
        .expect("orders");
    assert_eq!(orders.len(), 1, "{orders:?}");
    let order = &orders[0];
    assert_eq!(order.client_order_id, "pt-first-20240301-SPY");
    assert_eq!(order.symbol, sym("SPY"));
    assert_eq!(order.side, Side::Buy);
    assert_eq!(order.qty, 2);
    assert_eq!(order.state, OrderState::Submitted);
}

// R-492
#[test]
fn the_first_order_is_posted_once_for_the_approved_quantity() {
    let fx = approved_fixture();

    run_cycle(&fx.ctx()).expect("run");

    let posts: Vec<_> = fx
        .mock
        .requests()
        .into_iter()
        .filter(|r| r.method == "POST")
        .collect();
    assert_eq!(posts.len(), 1);
    let body: serde_json::Value = serde_json::from_str(&posts[0].body).expect("json body");
    let qty = match &body["qty"] {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    assert_eq!(qty, "2", "{body}");
}

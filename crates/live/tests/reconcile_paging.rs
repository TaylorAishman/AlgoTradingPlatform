//! R-481: a foreign order is found beyond the first page of the order list.

mod common;

use common::{d, sym, LiveFixture};
use pt_core::order_state::OrderState;
use pt_core::types::Side;
use pt_gate::owner::Stage;
use pt_live::reconcile::reconcile;
use pt_store::trading::OrderRow;
use rust_decimal::Decimal;

// R-481
#[test]
fn a_foreign_order_after_501_known_orders_is_found_on_a_later_page() {
    let fx = LiveFixture::new(Stage::Paper);
    assert!(reconcile(&fx.ctx()).expect("opening").clean);

    for i in 0..501 {
        let id = format!("known-{i:04}");
        fx.mock.seed_foreign_order(&id, "SPY", "buy", 1);
        fx.store
            .insert_order(&OrderRow {
                client_order_id: id,
                decision_id: None,
                symbol: sym("SPY"),
                side: Side::Buy,
                qty: 1,
                leg: 1,
                state: OrderState::Submitted,
                broker_order_id: None,
                filled_qty: 0,
                reference_price: Decimal::from(100),
                trade_date: d(2024, 3, 1),
            })
            .expect("insert");
    }
    fx.mock.seed_foreign_order("manual-late", "SPY", "buy", 1);

    let report = reconcile(&fx.ctx()).expect("reconcile");

    assert_eq!(report.halt_reason.as_deref(), Some("foreign_order"));
    let halts = fx.store.active_halts().expect("halts");
    assert_eq!(halts.len(), 1);
    assert!(
        halts[0].detail.contains("manual-late"),
        "detail was {:?}",
        halts[0].detail
    );
    let list_requests = fx
        .mock
        .requests()
        .iter()
        .filter(|r| r.method == "GET" && r.path.starts_with("/v2/orders"))
        .count();
    assert!(list_requests >= 2, "order list requests: {list_requests}");
}

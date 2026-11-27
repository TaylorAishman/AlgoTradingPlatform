//! R-486: `flatten` cancels open orders, settles, and only then closes what
//! the broker holds.

mod common;

use common::{d, sym, LiveFixture};
use pt_core::order_state::OrderState;
use pt_core::types::Side;
use pt_gate::gate::AuthContext;
use pt_gate::owner::Stage;
use pt_live::killswitch::flatten;
use pt_live::reconcile::reconcile;
use pt_live::submit::submit_planned;
use pt_store::trading::{Book, OrderRow};
use rust_decimal::Decimal;

const BUY_ID: &str = "pt-tsmom-v1-20240229-SPY-1";

fn auth() -> AuthContext {
    AuthContext {
        strategy_id: "tsmom-v1".to_string(),
        today: d(2024, 3, 1),
        halted: false,
        risk_reducing: false,
        is_first_order: false,
        first_order_already_sent: false,
    }
}

// R-486
#[test]
fn flatten_cancels_the_open_buy_settles_its_partial_fill_then_closes_the_filled_part() {
    let fx = LiveFixture::new(Stage::Paper);
    assert!(reconcile(&fx.ctx()).expect("reconcile").clean);
    fx.store
        .insert_order(&OrderRow {
            client_order_id: BUY_ID.to_string(),
            decision_id: None,
            symbol: sym("SPY"),
            side: Side::Buy,
            qty: 10,
            leg: 1,
            state: OrderState::Intent,
            broker_order_id: None,
            filled_qty: 0,
            reference_price: Decimal::from(100),
            trade_date: d(2024, 3, 1),
        })
        .expect("insert");
    submit_planned(&fx.ctx(), &auth()).expect("submit");
    // Filled 4 at the broker, not yet settled in the store.
    fx.mock.fill(BUY_ID, 4, "100");
    assert_eq!(
        fx.store.order(BUY_ID).expect("read").expect("row").state,
        OrderState::Submitted
    );

    let report = flatten(&fx.ctx()).expect("flatten");

    assert_eq!(report.cancelled, 1);
    assert_eq!(report.accepted, 1);
    assert_eq!(report.rejected, 0);

    let requests = fx.mock.requests();
    let delete_at = requests
        .iter()
        .position(|r| r.method == "DELETE")
        .expect("a DELETE was recorded");
    let post_at = requests
        .iter()
        .rposition(|r| r.method == "POST" && r.path == "/v2/orders")
        .expect("a closing POST was recorded");
    assert!(
        delete_at < post_at,
        "the DELETE came before the closing POST"
    );

    let buy = fx.store.order(BUY_ID).expect("read").expect("row");
    assert_eq!(buy.state, OrderState::Canceled);
    assert_eq!(buy.filled_qty, 4);
    assert_eq!(
        fx.store
            .positions(Book::Paper)
            .expect("positions")
            .get(&sym("SPY")),
        Some(&4)
    );

    let closing: Vec<serde_json::Value> = requests
        .iter()
        .filter(|r| r.method == "POST" && r.path == "/v2/orders")
        .skip(1) // the buy submitted above
        .map(|r| serde_json::from_str(&r.body).expect("json body"))
        .collect();
    assert_eq!(closing.len(), 1, "one closing POST");
    assert_eq!(closing[0]["side"], "sell");
    assert_eq!(closing[0]["symbol"], "SPY");
    assert_eq!(closing[0]["qty"].as_str(), Some("4"));
}

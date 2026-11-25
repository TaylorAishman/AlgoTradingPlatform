mod common;

use common::*;
use pt_core::cost::CostSpec;
use pt_core::types::Side;
use pt_engine::broker::SimBroker;
use pt_engine::error::EngineError;

// R-198
#[test]
fn fills_at_the_open_of_the_trade_date_in_the_order_given() {
    let broker = SimBroker::new(CostSpec::zero());
    let orders = [
        order("B", Side::Sell, 500, 1, 50),
        order("A", Side::Buy, 500, 2, 100),
    ];
    let fills = broker
        .execute(d(2024, 1, 3), &orders, &worked_view())
        .unwrap();
    assert_eq!(fills.len(), 2);
    assert_eq!(fills[0].symbol, sym("B"));
    assert_eq!(fills[0].side, Side::Sell);
    assert_eq!(fills[0].qty, 500);
    assert_eq!(fills[0].price, price(49));
    assert_eq!(fills[0].cost.0, dec(0));
    assert_eq!(fills[1].symbol, sym("A"));
    assert_eq!(fills[1].side, Side::Buy);
    assert_eq!(fills[1].qty, 500);
    assert_eq!(fills[1].price, price(101));
    assert_eq!(fills[1].cost.0, dec(0));
}

// R-198
#[test]
fn a_missing_bar_for_an_ordered_symbol_is_missing_bar() {
    let broker = SimBroker::new(CostSpec::zero());
    let orders = [
        order("B", Side::Sell, 500, 1, 50),
        order("A", Side::Buy, 500, 2, 100),
    ];
    let view = worked_view_without(&[("B", 3)]);
    let err = broker.execute(d(2024, 1, 3), &orders, &view).unwrap_err();
    assert_eq!(
        err,
        EngineError::MissingBar {
            symbol: "B".to_string(),
            date: d(2024, 1, 3)
        }
    );
}

// R-198
#[test]
fn no_orders_give_no_fills() {
    let broker = SimBroker::new(CostSpec::zero());
    let fills = broker.execute(d(2024, 1, 3), &[], &worked_view()).unwrap();
    assert!(fills.is_empty());
}

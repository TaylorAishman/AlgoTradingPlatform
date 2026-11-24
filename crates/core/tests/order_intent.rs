use pt_core::error::CoreError;
use pt_core::money::Price;
use pt_core::types::{OrderIntent, Side, Symbol};
use rust_decimal::Decimal;

fn sym() -> Symbol {
    Symbol::new("SPY").unwrap()
}

fn px() -> Price {
    Price::new(Decimal::new(10125, 2)).unwrap()
}

fn is_invalid_order(r: Result<OrderIntent, CoreError>) -> bool {
    matches!(r, Err(CoreError::InvalidOrder(_)))
}

// R-015
#[test]
fn zero_quantity_is_rejected() {
    assert!(is_invalid_order(OrderIntent::new(
        sym(),
        Side::Buy,
        0,
        1,
        px()
    )));
}

// R-015
#[test]
fn leg_zero_is_rejected() {
    assert!(is_invalid_order(OrderIntent::new(
        sym(),
        Side::Buy,
        5,
        0,
        px()
    )));
}

// R-015
#[test]
fn leg_three_is_rejected() {
    assert!(is_invalid_order(OrderIntent::new(
        sym(),
        Side::Sell,
        5,
        3,
        px()
    )));
}

// R-015
#[test]
fn quantity_five_leg_two_succeeds() {
    let o = OrderIntent::new(sym(), Side::Sell, 5, 2, px()).unwrap();
    assert_eq!(o.symbol, sym());
    assert_eq!(o.side, Side::Sell);
    assert_eq!(o.qty, 5);
    assert_eq!(o.leg, 2);
    assert_eq!(o.reference_price, px());
}

// R-015
#[test]
fn leg_one_succeeds() {
    assert!(OrderIntent::new(sym(), Side::Buy, 1, 1, px()).is_ok());
}

// R-015
#[test]
fn side_sign_is_plus_one_for_buy_and_minus_one_for_sell() {
    assert_eq!(Side::Buy.sign(), 1);
    assert_eq!(Side::Sell.sign(), -1);
}

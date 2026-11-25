//! R-337: slippage is recorded per filled order against the raw open of its
//! trade date, using the quantity-weighted fill price.

mod common;

use common::*;
use pt_core::order_state::OrderState;
use pt_core::types::Side;
use pt_shadow::slippage::record_slippage;
use pt_store::shadow::SlippageRow;

fn row<'a>(rows: &'a [SlippageRow], id: &str) -> &'a SlippageRow {
    rows.iter()
        .find(|r| r.client_order_id == id)
        .unwrap_or_else(|| panic!("no slippage row for {id}"))
}

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-6, "{a} != {b}");
}

// R-337
#[test]
fn fixture_orders_are_recorded_against_the_raw_open() {
    let store = shadow_store();
    let cfg = config();

    assert_eq!(record_slippage(&store, &cfg).unwrap(), 2);

    let rows = store.slippage_rows().unwrap();
    assert_eq!(rows.len(), 2);
    let a = row(&rows, "o-A");
    assert_eq!(a.reference_price, dec("101"));
    assert_eq!(a.avg_fill_price, dec("101.05"));
    assert_eq!(u64::from(a.qty), 500);
    close(a.bps, 4.950495);
    let b = row(&rows, "o-B");
    assert_eq!(b.reference_price, dec("49"));
    assert_eq!(b.avg_fill_price, dec("48.97"));
    assert_eq!(u64::from(b.qty), 500);
    close(b.bps, 6.122449);
}

// R-337
#[test]
fn average_fill_price_is_weighted_by_quantity() {
    let store = shadow_store();
    store
        .insert_order(&order(
            "o-C",
            "A",
            Side::Buy,
            10,
            OrderState::Filled,
            Some("b-C"),
            "100",
        ))
        .unwrap();
    store
        .insert_fill(&fill("f-C1", "b-C", "A", Side::Buy, 4, "100"))
        .unwrap();
    store
        .insert_fill(&fill("f-C2", "b-C", "A", Side::Buy, 6, "101"))
        .unwrap();

    assert_eq!(record_slippage(&store, &config()).unwrap(), 3);

    let rows = store.slippage_rows().unwrap();
    let c = row(&rows, "o-C");
    assert_eq!(c.avg_fill_price, dec("100.6"));
    assert_eq!(c.reference_price, dec("101"));
    assert_eq!(u64::from(c.qty), 10);
    // A buy filled below the open of 101 is better than the reference.
    close(c.bps, (100.6 - 101.0) / 101.0 * 10_000.0);
}

// R-337
#[test]
fn dry_run_orders_and_orders_without_fills_are_skipped() {
    let store = shadow_store();
    // A dry-run order, even with a fill row under its broker id, is not recorded.
    store
        .insert_order(&order(
            "o-D",
            "A",
            Side::Buy,
            5,
            OrderState::DryRun,
            Some("b-D"),
            "100",
        ))
        .unwrap();
    store
        .insert_fill(&fill("f-D", "b-D", "A", Side::Buy, 5, "102"))
        .unwrap();
    // A submitted order with no fill is not recorded.
    store
        .insert_order(&order(
            "o-E",
            "B",
            Side::Sell,
            5,
            OrderState::Submitted,
            Some("b-E"),
            "50",
        ))
        .unwrap();

    assert_eq!(record_slippage(&store, &config()).unwrap(), 2);

    let rows = store.slippage_rows().unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows
        .iter()
        .all(|r| r.client_order_id == "o-A" || r.client_order_id == "o-B"));
}

// R-337
#[test]
fn a_second_call_records_nothing() {
    let store = shadow_store();
    let cfg = config();
    assert_eq!(record_slippage(&store, &cfg).unwrap(), 2);
    assert_eq!(record_slippage(&store, &cfg).unwrap(), 0);
    assert_eq!(store.slippage_rows().unwrap().len(), 2);
}

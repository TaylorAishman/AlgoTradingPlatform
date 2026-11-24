mod common;

use chrono::{Duration, NaiveDate};
use common::*;
use pt_core::cost::CostSpec;
use pt_core::money::Money;
use pt_core::types::Side;
use pt_core::view::MemoryView;
use pt_engine::broker::SimBroker;
use pt_engine::cost::{fill_cost, ImpactInputs};
use pt_engine::error::EngineError;

fn impact_spec() -> CostSpec {
    CostSpec {
        impact_coeff: dec(1),
        ..CostSpec::zero()
    }
}

fn trade_date() -> NaiveDate {
    d(2024, 2, 15)
}

/// The 21 sessions before the trade date, oldest first.
fn sessions() -> Vec<NaiveDate> {
    (1..=21)
        .rev()
        .map(|n| trade_date() - Duration::days(n))
        .collect()
}

/// Adjusted closes alternating +1% and -2% returns, one per session.
fn closes() -> Vec<f64> {
    let mut c = vec![100.0_f64];
    for i in 1..21 {
        let r = if i % 2 == 1 { 0.01 } else { -0.02 };
        let prev = c[i - 1];
        c.push(prev * (1.0 + r));
    }
    c
}

/// A view with execution bars (volume 1000000) for the 21 sessions and the
/// trade date (open 100), and the last `n_closes` adjusted closes.
fn view_with(n_closes: usize) -> MemoryView {
    let mut view = MemoryView::new();
    for day in sessions() {
        view.push_exec(bar("A", day, 100, 100));
    }
    view.push_exec(bar("A", trade_date(), 100, 100));
    let days = sessions();
    let cs = closes();
    for i in (21 - n_closes)..21 {
        view.push_adj_close(&sym("A"), days[i], cs[i]);
    }
    view
}

fn sample_std(rets: &[f64]) -> f64 {
    let n = rets.len() as f64;
    let mean = rets.iter().sum::<f64>() / n;
    (rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt()
}

// R-211
#[test]
fn impact_uses_sample_volatility_and_adv_from_the_view() {
    let cs = closes();
    let rets: Vec<f64> = cs.windows(2).map(|w| w[1] / w[0] - 1.0).collect();
    assert_eq!(rets.len(), 20);
    let expected = fill_cost(
        &impact_spec(),
        10_000,
        price(100),
        Some(ImpactInputs {
            daily_vol: sample_std(&rets),
            adv_shares: 1_000_000.0,
        }),
    )
    .unwrap();
    assert!(expected.0 > dec(0));

    let broker = SimBroker::new(impact_spec());
    let orders = [order("A", Side::Buy, 10_000, 1, 100)];
    let fills = broker
        .execute(trade_date(), &orders, &view_with(21))
        .unwrap();
    assert_eq!(fills.len(), 1);
    let cost = fills[0].cost;
    assert!(cost.0 > dec(0));
    let tolerance = rust_decimal::Decimal::new(1, 4);
    assert!(
        (cost.0 - expected.0).abs() <= tolerance,
        "cost {cost:?} differs from expected {expected:?}"
    );
}

// R-211
#[test]
fn fifteen_adjusted_closes_are_missing_impact_inputs() {
    let broker = SimBroker::new(impact_spec());
    let orders = [order("A", Side::Buy, 10_000, 1, 100)];
    let err = broker
        .execute(trade_date(), &orders, &view_with(15))
        .unwrap_err();
    assert!(
        matches!(err, EngineError::MissingImpactInputs(_)),
        "got {err:?}"
    );
}

// R-211
#[test]
fn zero_impact_coefficient_needs_no_adjusted_closes() {
    let broker = SimBroker::new(CostSpec::zero());
    let orders = [order("A", Side::Buy, 10_000, 1, 100)];
    let mut view = MemoryView::new();
    view.push_exec(bar("A", trade_date(), 100, 100));
    let fills = broker.execute(trade_date(), &orders, &view).unwrap();
    assert_eq!(fills.len(), 1);
    assert_eq!(fills[0].cost, Money(dec(0)));
}

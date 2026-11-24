//! Pre-trade order checks.

use std::collections::BTreeMap;

use pt_core::money::{Money, Price};
use pt_core::types::{OrderIntent, Positions, Side, Symbol};
use rust_decimal::Decimal;

/// Configured risk limits.
#[derive(Debug, Clone, PartialEq)]
pub struct RiskLimits {
    pub max_order_notional: Decimal,
    pub max_weight: Decimal,
    pub max_gross: Decimal,
    pub daily_loss_limit: Decimal,
    pub max_drawdown: Decimal,
}

/// Why an order was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rejection {
    NoEquity,
    OrderNotional,
    WeightCap,
    Gross,
}

/// Orders split into approved and rejected (with the reason).
#[derive(Debug, Clone, PartialEq)]
pub struct GateOutcome {
    pub approved: Vec<OrderIntent>,
    pub rejected: Vec<(OrderIntent, Rejection)>,
}

/// Check each order against the limits, in order: NoEquity, OrderNotional, WeightCap, Gross.
/// Each order sees the position left by the orders approved before it. Then the
/// projected gross exposure over the whole batch is checked: if it exceeds
/// `max_gross * equity`, every order that passed the earlier checks is rejected
/// with `Gross`. Orders that only reduce a position (smaller absolute size, same
/// side, no flip through zero) are exempt from the WeightCap and Gross checks. An ordered symbol missing from
/// `prices` is valued at the order's reference price; a held symbol with no price
/// at all cannot be valued, so the whole batch is rejected with `Gross` (fail
/// closed), reducing orders included.
pub fn check_orders(
    orders: &[OrderIntent],
    positions: &Positions,
    equity: Money,
    prices: &BTreeMap<Symbol, Price>,
    limits: &RiskLimits,
) -> GateOutcome {
    let mut out = GateOutcome {
        approved: Vec::new(),
        rejected: Vec::new(),
    };
    let mut running: BTreeMap<Symbol, i64> =
        positions.iter().map(|(k, v)| (k.clone(), *v)).collect();
    let mut passed: Vec<(OrderIntent, bool)> = Vec::new();
    for o in orders {
        if equity.0 <= Decimal::ZERO {
            out.rejected.push((o.clone(), Rejection::NoEquity));
            continue;
        }
        let price = prices.get(&o.symbol).copied().unwrap_or(o.reference_price);
        let notional = price.value() * Decimal::from(o.qty);
        if notional > limits.max_order_notional {
            out.rejected.push((o.clone(), Rejection::OrderNotional));
            continue;
        }
        let delta = match o.side {
            Side::Buy => o.qty as i64,
            Side::Sell => -(o.qty as i64),
        };
        let before = running.get(&o.symbol).copied().unwrap_or(0);
        let after = before + delta;
        let reducing = after.abs() < before.abs() && after.signum() * before.signum() >= 0;
        if !reducing && (price.value() * Decimal::from(after)).abs() > limits.max_weight * equity.0
        {
            out.rejected.push((o.clone(), Rejection::WeightCap));
            continue;
        }
        running.insert(o.symbol.clone(), after);
        passed.push((o.clone(), reducing));
    }
    let mut ref_prices: BTreeMap<&Symbol, Price> = BTreeMap::new();
    for o in orders {
        ref_prices.entry(&o.symbol).or_insert(o.reference_price);
    }
    let mut gross = Decimal::ZERO;
    let mut unpriced = false;
    for (sym, qty) in &running {
        if *qty == 0 {
            continue;
        }
        match prices.get(sym).or_else(|| ref_prices.get(sym)) {
            Some(p) => gross += (p.value() * Decimal::from(*qty)).abs(),
            None => unpriced = true,
        }
    }
    let over_gross = !passed.is_empty() && (unpriced || gross > limits.max_gross * equity.0);
    for (o, reducing) in passed {
        // A reducing order is exempt from the gross cap, but not when the
        // batch cannot be valued at all.
        if over_gross && (unpriced || !reducing) {
            out.rejected.push((o, Rejection::Gross));
        } else {
            out.approved.push(o);
        }
    }
    out
}

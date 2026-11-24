//! Pre-trade order checks.

use std::collections::BTreeMap;

use pt_core::money::{Money, Price};
use pt_core::types::{OrderIntent, Positions, Symbol};
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
}

/// Orders split into approved and rejected (with the reason).
#[derive(Debug, Clone, PartialEq)]
pub struct GateOutcome {
    pub approved: Vec<OrderIntent>,
    pub rejected: Vec<(OrderIntent, Rejection)>,
}

/// Check each order against the limits. NoEquity is checked first.
pub fn check_orders(
    orders: &[OrderIntent],
    _positions: &Positions,
    equity: Money,
    prices: &BTreeMap<Symbol, Price>,
    limits: &RiskLimits,
) -> GateOutcome {
    let mut out = GateOutcome {
        approved: Vec::new(),
        rejected: Vec::new(),
    };
    for o in orders {
        if equity.0 <= Decimal::ZERO {
            out.rejected.push((o.clone(), Rejection::NoEquity));
            continue;
        }
        let price = prices.get(&o.symbol).copied().unwrap_or(o.reference_price);
        let notional = price.value() * Decimal::from(o.qty);
        if notional > limits.max_order_notional {
            out.rejected.push((o.clone(), Rejection::OrderNotional));
        } else {
            out.approved.push(o.clone());
        }
    }
    out
}

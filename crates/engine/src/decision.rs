//! The shared decision function and its parameters.

use crate::error::EngineError;
use chrono::{DateTime, NaiveDate, Utc};
use pt_config::config::Config;
use pt_core::money::{Money, Price};
use pt_core::signal::{RiskEstimates, SignalSet};
use pt_core::types::{OrderIntent, Positions, Symbol, Weights};
use pt_core::view::MarketView;
use pt_portfolio::orders::{plan_orders, target_shares};
use pt_portfolio::weights::{target_weights, SizingParams};
use pt_risk::pretrade::{check_orders, Rejection, RiskLimits};
use pt_strategy::estimate::{estimate_risk, VolParams};
use pt_strategy::strategy::Strategy;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

/// What a decision is made from: the dates, the account and the universe.
#[derive(Debug, Clone, PartialEq)]
pub struct DecisionInputs {
    pub signal_date: NaiveDate,
    pub trade_date: NaiveDate,
    pub ref_date: NaiveDate,
    pub equity: Money,
    pub positions: Positions,
    pub universe: Vec<Symbol>,
    pub shortable: BTreeSet<Symbol>,
}

/// A decision: what it was given and the weights, targets and orders it produced.
#[derive(Debug, Clone, PartialEq)]
pub struct Decision {
    pub signals: SignalSet,
    pub risk: RiskEstimates,
    pub weights: Weights,
    pub target: Positions,
    pub orders: Vec<OrderIntent>,
    pub rejected: Vec<(OrderIntent, Rejection)>,
}

/// Everything the decision function reads from configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct DecisionParams {
    pub vol: VolParams,
    pub sizing: SizingParams,
    pub limits: RiskLimits,
}

fn float(d: Decimal) -> f64 {
    d.to_f64().unwrap_or(f64::NAN)
}

/// The one place config becomes decision parameters.
pub fn params_from_config(cfg: &Config) -> DecisionParams {
    DecisionParams {
        vol: VolParams {
            com_days: f64::from(cfg.strategy.vol_com_days),
            annualization: f64::from(cfg.strategy.vol_annualization),
            min_obs: cfg.strategy.vol_min_obs as usize,
            max_obs: cfg.strategy.vol_max_obs as usize,
        },
        sizing: SizingParams {
            target_vol: float(cfg.sizing.target_vol),
            max_weight: float(cfg.sizing.max_weight),
            max_gross: float(cfg.sizing.max_gross),
        },
        limits: RiskLimits {
            max_order_notional: cfg.risk.max_order_notional,
            max_weight: cfg.sizing.max_weight,
            max_gross: cfg.sizing.max_gross,
            daily_loss_limit: cfg.risk.daily_loss_limit,
            max_drawdown: cfg.risk.max_drawdown,
        },
    }
}

/// Turns signals and risk estimates into weights (Arch 5.10.3), whole-share
/// targets and orders (Arch 5.10.4) at the given reference prices.
pub fn decide_with(
    signals: SignalSet,
    risk: RiskEstimates,
    inputs: &DecisionInputs,
    ref_prices: &BTreeMap<Symbol, Price>,
    p: &DecisionParams,
) -> Result<Decision, EngineError> {
    let weights = target_weights(&signals.signals, &risk, &inputs.shortable, &p.sizing);
    let target = target_shares(&weights, inputs.equity, ref_prices)?;
    let planned = plan_orders(&inputs.positions, &target, ref_prices)?;
    let gate = check_orders(
        &planned,
        &inputs.positions,
        inputs.equity,
        ref_prices,
        &p.limits,
    );
    Ok(Decision {
        signals,
        risk,
        weights,
        target,
        orders: gate.approved,
        rejected: gate.rejected,
    })
}

/// Reads signals, risk and reference closes from the view, then decides.
/// A symbol with a nonzero weight or a current position must have a bar on
/// the reference date; otherwise the data is not ready.
pub fn decide(
    inputs: &DecisionInputs,
    strategy: &dyn Strategy,
    view: &dyn MarketView,
    p: &DecisionParams,
) -> Result<Decision, EngineError> {
    let signals = strategy.signals(&inputs.universe, inputs.signal_date, view)?;
    let risk = estimate_risk(&inputs.universe, inputs.signal_date, view, &p.vol);
    let weights = target_weights(&signals.signals, &risk, &inputs.shortable, &p.sizing);
    let mut ref_prices = BTreeMap::new();
    for symbol in &inputs.universe {
        let weighted = weights.get(symbol).is_some_and(|w| !w.is_zero());
        let held = inputs.positions.get(symbol).is_some_and(|q| *q != 0);
        if !weighted && !held {
            continue;
        }
        let bar =
            view.exec_bar(symbol, inputs.ref_date)
                .ok_or_else(|| EngineError::DataNotReady {
                    symbol: symbol.to_string(),
                    date: inputs.ref_date,
                })?;
        ref_prices.insert(symbol.clone(), bar.close);
    }
    decide_with(signals, risk, inputs, &ref_prices, p)
}

/// Non-finite values (NaN, infinities) become `null`.
fn f64_json(x: f64) -> serde_json::Value {
    serde_json::Number::from_f64(x).map_or(serde_json::Value::Null, serde_json::Value::Number)
}

fn float_map_json(m: &BTreeMap<Symbol, f64>) -> serde_json::Value {
    m.iter()
        .map(|(k, v)| (k.to_string(), f64_json(*v)))
        .collect::<serde_json::Map<_, _>>()
        .into()
}

fn reasons_json(m: &BTreeMap<Symbol, String>) -> serde_json::Value {
    m.iter()
        .map(|(k, v)| (k.to_string(), serde_json::Value::from(v.as_str())))
        .collect::<serde_json::Map<_, _>>()
        .into()
}

fn order_json(o: &OrderIntent) -> serde_json::Value {
    serde_json::json!({
        "symbol": o.symbol.to_string(),
        "side": match o.side {
            pt_core::types::Side::Buy => "buy",
            pt_core::types::Side::Sell => "sell",
        },
        "qty": o.qty,
        "leg": o.leg,
        "reference_price": o.reference_price.value().to_string(),
    })
}

/// The decision as canonical JSON: sorted keys, no extra whitespace and
/// decimals written as strings, so equal decisions give equal text.
pub fn decision_to_json(d: &Decision) -> String {
    let cov: Vec<serde_json::Value> = d
        .risk
        .cov
        .iter()
        .map(|((a, b), v)| serde_json::json!([a.to_string(), b.to_string(), f64_json(*v)]))
        .collect();
    let weights: serde_json::Map<_, _> = d
        .weights
        .iter()
        .map(|(k, v)| (k.to_string(), serde_json::Value::from(format!("{v:.6}"))))
        .collect();
    let target: serde_json::Map<_, _> = d
        .target
        .iter()
        .map(|(k, v)| (k.to_string(), serde_json::Value::from(*v)))
        .collect();
    let rejected: Vec<serde_json::Value> = d
        .rejected
        .iter()
        .map(|(o, r)| serde_json::json!({ "order": order_json(o), "reason": format!("{r:?}") }))
        .collect();
    let value = serde_json::json!({
        "signals": {
            "asof": d.signals.asof.to_string(),
            "signals": float_map_json(&d.signals.signals),
            "ineligible": reasons_json(&d.signals.ineligible),
        },
        "risk": {
            "asof": d.risk.asof.to_string(),
            "vols": float_map_json(&d.risk.vols),
            "cov": cov,
            "ineligible": reasons_json(&d.risk.ineligible),
        },
        "weights": weights,
        "target": target,
        "orders": d.orders.iter().map(order_json).collect::<Vec<_>>(),
        "rejected": rejected,
    });
    value.to_string()
}

/// A decision's inputs plus where they came from, kept so a run can be replayed.
#[derive(Debug, Clone, PartialEq)]
pub struct DecisionRecord {
    pub inputs: DecisionInputs,
    pub data_asof: DateTime<Utc>,
    pub feed: String,
    pub config_hash: String,
    pub code_version: String,
    pub bootstrap: bool,
}

/// The record as canonical JSON: sorted keys, decimals as strings and the
/// universe in its given order.
pub fn record_to_json(r: &DecisionRecord) -> String {
    let i = &r.inputs;
    let positions: serde_json::Map<_, _> = i
        .positions
        .iter()
        .map(|(k, v)| (k.to_string(), serde_json::Value::from(*v)))
        .collect();
    let value = serde_json::json!({
        "signal_date": i.signal_date.to_string(),
        "trade_date": i.trade_date.to_string(),
        "ref_date": i.ref_date.to_string(),
        "equity": i.equity.0.to_string(),
        "positions": positions,
        "universe": i.universe.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        "shortable": i.shortable.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        "data_asof": r.data_asof.to_rfc3339(),
        "feed": r.feed,
        "config_hash": r.config_hash,
        "code_version": r.code_version,
        "bootstrap": r.bootstrap,
    });
    value.to_string()
}

fn bad(what: &str) -> EngineError {
    EngineError::Invalid(format!("decision record: bad or missing {what}"))
}

fn text<'a>(v: &'a serde_json::Value, key: &str) -> Result<&'a str, EngineError> {
    v.get(key).and_then(|x| x.as_str()).ok_or_else(|| bad(key))
}

fn date(v: &serde_json::Value, key: &str) -> Result<NaiveDate, EngineError> {
    text(v, key)?.parse().map_err(|_| bad(key))
}

fn symbols(v: &serde_json::Value, key: &str) -> Result<Vec<Symbol>, EngineError> {
    v.get(key)
        .and_then(|x| x.as_array())
        .ok_or_else(|| bad(key))?
        .iter()
        .map(|s| {
            let s = s.as_str().ok_or_else(|| bad(key))?;
            Symbol::new(s).map_err(EngineError::from)
        })
        .collect()
}

/// Reads a record written by `record_to_json`.
pub fn record_from_json(s: &str) -> Result<DecisionRecord, EngineError> {
    let v: serde_json::Value = serde_json::from_str(s)
        .map_err(|e| EngineError::Invalid(format!("decision record: {e}")))?;
    let mut positions = Positions::new();
    for (k, q) in v
        .get("positions")
        .and_then(|x| x.as_object())
        .ok_or_else(|| bad("positions"))?
    {
        positions.insert(Symbol::new(k)?, q.as_i64().ok_or_else(|| bad("positions"))?);
    }
    let equity = Decimal::from_str(text(&v, "equity")?).map_err(|_| bad("equity"))?;
    let data_asof = DateTime::parse_from_rfc3339(text(&v, "data_asof")?)
        .map_err(|_| bad("data_asof"))?
        .with_timezone(&Utc);
    Ok(DecisionRecord {
        inputs: DecisionInputs {
            signal_date: date(&v, "signal_date")?,
            trade_date: date(&v, "trade_date")?,
            ref_date: date(&v, "ref_date")?,
            equity: Money(equity),
            positions,
            universe: symbols(&v, "universe")?,
            shortable: symbols(&v, "shortable")?.into_iter().collect(),
        },
        data_asof,
        feed: text(&v, "feed")?.to_string(),
        config_hash: text(&v, "config_hash")?.to_string(),
        code_version: text(&v, "code_version")?.to_string(),
        bootstrap: v
            .get("bootstrap")
            .and_then(|x| x.as_bool())
            .ok_or_else(|| bad("bootstrap"))?,
    })
}

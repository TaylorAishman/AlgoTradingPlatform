//! Backtest runner: the request and the trial hash that identifies a trial.

use std::fmt::Write as _;

use chrono::NaiveDate;
use pt_config::config::Config;
use rust_decimal::Decimal;
use sha2::{Digest, Sha256};

/// What to backtest: one strategy over an inclusive date range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BacktestRequest {
    pub strategy: String,
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub initial_capital: Decimal,
}

fn line(out: &mut String, key: &str, value: impl std::fmt::Display) {
    // Writing to a String cannot fail.
    let _ = writeln!(out, "{key}={value}");
}

fn text(out: &mut String, key: &str, value: &str) {
    // Debug quoting escapes newlines and quotes, so values cannot forge keys.
    line(out, key, format!("{value:?}"));
}

fn dec(out: &mut String, key: &str, value: Decimal) {
    line(out, key, value.normalize());
}

/// The lowercase hex SHA-256 of everything that changes a backtest result: the request,
/// the strategy parameters, the universe, the research costs, sizing, risk, and the data
/// feed. Paths and signoff thresholds are left out because they do not change the result.
pub fn trial_hash(cfg: &Config, req: &BacktestRequest, params: &str) -> String {
    let mut o = String::new();
    text(&mut o, "strategy", &req.strategy);
    text(&mut o, "params", params);
    line(&mut o, "from", req.from);
    line(&mut o, "to", req.to);
    dec(&mut o, "initial_capital", req.initial_capital);

    text(&mut o, "data.feed", &cfg.data.feed);

    let c = &cfg.cost.research;
    dec(&mut o, "cost.spread_bps", c.spread_bps);
    dec(&mut o, "cost.fee_bps", c.fee_bps);
    dec(&mut o, "cost.impact_coeff", c.impact_coeff);
    dec(&mut o, "cost.borrow_bps_annual", c.borrow_bps_annual);

    let s = &cfg.strategy;
    text(&mut o, "strategy.risk_free_symbol", &s.risk_free_symbol);
    line(&mut o, "strategy.vol_com_days", s.vol_com_days);
    line(&mut o, "strategy.vol_annualization", s.vol_annualization);
    line(&mut o, "strategy.vol_min_obs", s.vol_min_obs);
    line(&mut o, "strategy.vol_max_obs", s.vol_max_obs);

    dec(&mut o, "sizing.target_vol", cfg.sizing.target_vol);
    dec(&mut o, "sizing.max_weight", cfg.sizing.max_weight);
    dec(&mut o, "sizing.max_gross", cfg.sizing.max_gross);

    let r = &cfg.risk;
    dec(&mut o, "risk.max_order_notional", r.max_order_notional);
    dec(&mut o, "risk.daily_loss_limit", r.daily_loss_limit);
    dec(&mut o, "risk.max_drawdown", r.max_drawdown);
    dec(&mut o, "risk.cash_tolerance", r.cash_tolerance);

    line(&mut o, "universe.len", cfg.universe.len());
    for (i, inst) in cfg.universe.iter().enumerate() {
        text(&mut o, &format!("universe.{i}.symbol"), &inst.symbol);
        line(
            &mut o,
            &format!("universe.{i}.asset_class"),
            format!("{:?}", inst.asset_class),
        );
        line(&mut o, &format!("universe.{i}.shortable"), inst.shortable);
    }

    let digest = Sha256::digest(o.as_bytes());
    let mut hex = String::with_capacity(64);
    for b in digest {
        let _ = write!(hex, "{b:02x}");
    }
    hex
}

//! A stable hash of the parsed configuration. It depends on values only.

use std::fmt::Write as _;

use pt_core::cost::CostSpec;
use rust_decimal::Decimal;
use sha2::{Digest, Sha256};

use crate::config::{AssetClass, Config};

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

fn cost(out: &mut String, prefix: &str, spec: &CostSpec) {
    dec(out, &format!("{prefix}.spread_bps"), spec.spread_bps);
    dec(out, &format!("{prefix}.fee_bps"), spec.fee_bps);
    dec(out, &format!("{prefix}.impact_coeff"), spec.impact_coeff);
    dec(
        out,
        &format!("{prefix}.borrow_bps_annual"),
        spec.borrow_bps_annual,
    );
}

fn class_name(class: AssetClass) -> &'static str {
    match class {
        AssetClass::Equity => "equity",
        AssetClass::Treasury => "treasury",
        AssetClass::Credit => "credit",
        AssetClass::Commodity => "commodity",
        AssetClass::Currency => "currency",
        AssetClass::RealEstate => "real_estate",
    }
}

/// The canonical text: fixed field order, one `key=value` per line, decimals normalised.
fn canonical(cfg: &Config) -> String {
    let mut o = String::new();
    text(&mut o, "run.db_path", &cfg.run.db_path);
    text(&mut o, "run.output_dir", &cfg.run.output_dir);

    text(&mut o, "data.feed", &cfg.data.feed);
    line(&mut o, "data.start", cfg.data.start);
    line(
        &mut o,
        "data.max_requests_per_minute",
        cfg.data.max_requests_per_minute,
    );

    let s = &cfg.strategy;
    text(&mut o, "strategy.id", &s.id);
    let lookbacks: Vec<String> = s.lookback_months.iter().map(u32::to_string).collect();
    line(
        &mut o,
        "strategy.lookback_months",
        format!("[{}]", lookbacks.join(",")),
    );
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
    line(
        &mut o,
        "risk.max_rebalance_delay_sessions",
        r.max_rebalance_delay_sessions,
    );
    line(
        &mut o,
        "risk.min_minutes_before_close",
        r.min_minutes_before_close,
    );
    dec(&mut o, "risk.cash_tolerance", r.cash_tolerance);

    line(&mut o, "stats.periods_per_year", cfg.stats.periods_per_year);
    line(&mut o, "stats.pbo_partitions", cfg.stats.pbo_partitions);

    cost(&mut o, "cost.research", &cfg.cost.research);
    cost(&mut o, "cost.shadow", &cfg.cost.shadow);

    let g = &cfg.signoff;
    line(&mut o, "signoff.min_sessions", g.min_sessions);
    line(&mut o, "signoff.min_rebalances", g.min_rebalances);
    line(&mut o, "signoff.min_fills", g.min_fills);
    line(
        &mut o,
        "signoff.max_replay_mismatches",
        g.max_replay_mismatches,
    );
    line(
        &mut o,
        "signoff.max_reconciliation_breaks",
        g.max_reconciliation_breaks,
    );
    line(&mut o, "signoff.max_missed_sessions", g.max_missed_sessions);
    line(&mut o, "signoff.max_late_rebalances", g.max_late_rebalances);
    dec(
        &mut o,
        "signoff.slippage_tolerance_bps",
        g.slippage_tolerance_bps,
    );
    dec(&mut o, "signoff.max_mean_diff_bps", g.max_mean_diff_bps);
    dec(&mut o, "signoff.max_abs_diff_bps", g.max_abs_diff_bps);

    line(&mut o, "universe.len", cfg.universe.len());
    for (i, inst) in cfg.universe.iter().enumerate() {
        text(&mut o, &format!("universe.{i}.symbol"), &inst.symbol);
        line(
            &mut o,
            &format!("universe.{i}.asset_class"),
            class_name(inst.asset_class),
        );
        line(&mut o, &format!("universe.{i}.shortable"), inst.shortable);
    }
    o
}

/// The lowercase hex SHA-256 of the canonical form of `cfg`.
pub fn config_hash(cfg: &Config) -> String {
    let digest = Sha256::digest(canonical(cfg).as_bytes());
    let mut hex = String::with_capacity(64);
    for b in digest {
        let _ = write!(hex, "{b:02x}");
    }
    hex
}

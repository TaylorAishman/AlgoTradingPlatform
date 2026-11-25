//! Backtest runner: the request, the trial hash that identifies a trial, and the
//! research run that is stored.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::sync::Arc;

use chrono::NaiveDate;
use pt_calendar::schedule::monthly_schedule;
use pt_config::config::Config;
use pt_config::hash::config_hash;
use pt_core::clock::Clock;
use pt_core::types::{Adjustment, PriceBasis, Symbol};
use pt_data::view::StoreView;
use pt_engine::decision::params_from_config;
use pt_engine::schedule::run_backtest;
use pt_engine::simulate::{SimConfig, SimResult};
use pt_stats::descriptive::returns_from_equity;
use pt_stats::perf::{annualized_sharpe, annualized_vol, cagr, max_drawdown};
use pt_store::db::Store;
use pt_strategy::benchmark::LongOnly;
use pt_strategy::strategy::Strategy;
use pt_strategy::tsmom::{Tsmom, TsmomParams};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use sha2::{Digest, Sha256};

use crate::error::ValidateError;

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

/// Summary statistics of a backtest.
#[derive(Debug, Clone, PartialEq)]
pub struct Metrics {
    pub sharpe: f64,
    pub cagr: f64,
    pub vol: f64,
    pub max_drawdown: f64,
    pub turnover: f64,
    pub cost_total: Decimal,
    pub borrow_total: Decimal,
    pub n_obs: usize,
}

const PERIODS_PER_YEAR: f64 = 252.0;

fn to_f64(d: Decimal) -> Result<f64, ValidateError> {
    d.to_f64()
        .ok_or_else(|| ValidateError::Invalid(format!("{d} does not fit in f64")))
}

impl Metrics {
    /// The metrics of `result`. `rf_closes` are the closes of the risk-free instrument
    /// `rf_symbol` by date; they are aligned to the equity dates, and a missing close is an
    /// error. Sharpe is taken on the excess return.
    fn from_result(
        result: &SimResult,
        rf_symbol: &str,
        rf_closes: &BTreeMap<NaiveDate, f64>,
    ) -> Result<Metrics, ValidateError> {
        let equity = result
            .equity
            .iter()
            .map(|(_, e)| to_f64(*e))
            .collect::<Result<Vec<_>, _>>()?;
        let rets = returns_from_equity(&equity);
        let rf = result
            .equity
            .iter()
            .map(|(d, _)| {
                rf_closes
                    .get(d)
                    .copied()
                    .ok_or_else(|| ValidateError::Invalid(format!("no {rf_symbol} close on {d}")))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let excess: Vec<f64> = rets
            .iter()
            .zip(returns_from_equity(&rf))
            .map(|(r, b)| r - b)
            .collect();
        let turnover = if equity.is_empty() || rets.is_empty() {
            0.0
        } else {
            let mean_eq = equity.iter().sum::<f64>() / equity.len() as f64;
            let years = rets.len() as f64 / PERIODS_PER_YEAR;
            to_f64(result.traded_notional)? / mean_eq / years
        };
        Ok(Metrics {
            sharpe: annualized_sharpe(&excess, PERIODS_PER_YEAR),
            cagr: cagr(&equity, PERIODS_PER_YEAR),
            vol: annualized_vol(&rets, PERIODS_PER_YEAR),
            max_drawdown: max_drawdown(&equity),
            turnover,
            cost_total: result.cost_total,
            borrow_total: result.borrow_total,
            n_obs: rets.len(),
        })
    }
}

/// A stored research backtest.
#[derive(Debug, Clone, PartialEq)]
pub struct BacktestOutcome {
    pub run_id: i64,
    pub trial_hash: String,
    pub metrics: Metrics,
    pub result: SimResult,
}

fn symbol(s: &str) -> Result<Symbol, ValidateError> {
    Symbol::new(s).map_err(|e| ValidateError::Invalid(format!("symbol {s:?}: {e}")))
}

/// Run `req` on adjusted prices with the research cost profile and store its equity.
pub fn run_and_record(
    store: &Store,
    cfg: &Config,
    req: &BacktestRequest,
    clock: &dyn Clock,
) -> Result<BacktestOutcome, ValidateError> {
    let cal = Arc::new(store.calendar()?);
    let universe = cfg
        .universe
        .iter()
        .map(|i| symbol(&i.symbol))
        .collect::<Result<Vec<_>, _>>()?;
    let shortable: BTreeSet<Symbol> = cfg
        .universe
        .iter()
        .zip(&universe)
        .filter(|(i, _)| i.shortable)
        .map(|(_, s)| s.clone())
        .collect();
    let risk_free = symbol(&cfg.strategy.risk_free_symbol)?;
    let rf_sym = risk_free.clone();

    let strategy: Box<dyn Strategy> = match req.strategy.as_str() {
        "tsmom" => Box::new(Tsmom::new(
            TsmomParams {
                lookback_months: cfg.strategy.lookback_months.clone(),
                risk_free_symbol: risk_free.clone(),
            },
            Arc::clone(&cal),
        )),
        "benchmark" => Box::new(LongOnly::new(Arc::clone(&cal))),
        other => {
            return Err(ValidateError::Invalid(format!(
                "unknown strategy {other:?}"
            )))
        }
    };

    let mut symbols = universe.clone();
    if !symbols.contains(&risk_free) {
        symbols.push(risk_free);
    }
    let view = StoreView::load(store, &symbols, &cfg.data.feed, PriceBasis::Adjusted, None)?;
    let params = params_from_config(cfg);
    let sim = SimConfig {
        start: req.from,
        end: req.to,
        initial_capital: req.initial_capital,
        cost: cfg.cost.research,
        limits: params.limits.clone(),
        universe,
    };
    let schedule = monthly_schedule(&cal, req.from, req.to);
    let result = run_backtest(
        &sim,
        &schedule,
        &shortable,
        strategy.as_ref(),
        &view,
        &cal,
        &params,
    )?;

    let rf_closes = store
        .bars_asof(
            &rf_sym,
            Adjustment::All,
            &cfg.data.feed,
            req.from,
            req.to,
            None,
        )?
        .iter()
        .map(|b| Ok((b.date, to_f64(b.close.value())?)))
        .collect::<Result<BTreeMap<_, _>, ValidateError>>()?;
    let metrics = Metrics::from_result(&result, rf_sym.as_str(), &rf_closes)?;

    let hash = trial_hash(cfg, req, &strategy.params_canonical());
    let run_id = store.save_backtest(
        strategy.id(),
        &hash,
        &config_hash(cfg),
        &result.equity,
        clock.now_utc(),
    )?;
    Ok(BacktestOutcome {
        run_id,
        trial_hash: hash,
        metrics,
        result,
    })
}

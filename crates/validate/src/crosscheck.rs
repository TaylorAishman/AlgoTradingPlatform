//! Cross-check: the engine and the vectorised calculator run the same weight schedule
//! and their equity series are compared session by session.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use pt_calendar::calendar::TradingCalendar;
use pt_calendar::schedule::monthly_schedule;
use pt_config::config::Config;
use pt_core::cost::CostSpec;
use pt_core::types::{PriceBasis, Symbol};
use pt_core::view::MarketView;
use pt_data::view::StoreView;
use pt_engine::decision::params_from_config;
use pt_engine::error::EngineError;
use pt_engine::schedule::weights_schedule;
use pt_engine::simulate::{simulate, SimConfig};
use pt_store::db::Store;
use pt_strategy::benchmark::LongOnly;
use pt_strategy::strategy::Strategy;
use pt_strategy::tsmom::{Tsmom, TsmomParams};
use pt_vector::vector::{compare, vector_equity, CrossCheck, VectorInputs};
use rust_decimal::Decimal;

use crate::error::ValidateError;
use crate::runner::BacktestRequest;

fn symbol(s: &str) -> Result<Symbol, ValidateError> {
    Symbol::new(s).map_err(|e| ValidateError::Invalid(format!("symbol {s:?}: {e}")))
}

/// The cost profile both calculators run with: all zero, or the research profile
/// without the impact term, which the vector calculator does not implement.
fn cost_spec(cfg: &Config, zero_cost: bool) -> CostSpec {
    if zero_cost {
        CostSpec::zero()
    } else {
        CostSpec {
            impact_coeff: Decimal::ZERO,
            ..cfg.cost.research
        }
    }
}

/// Run `req` through the engine and through the vector calculator on adjusted prices
/// with the same weight schedule, and compare their equity on every session of the
/// range within `tolerance`. With `zero_cost` both run without costs; otherwise both
/// run with the research costs and `impact_coeff` set to 0.
pub fn crosscheck(
    store: &Store,
    cfg: &Config,
    req: &BacktestRequest,
    zero_cost: bool,
    tolerance: Decimal,
) -> Result<CrossCheck, ValidateError> {
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
    let cost = cost_spec(cfg, zero_cost);
    let sim = SimConfig {
        start: req.from,
        end: req.to,
        initial_capital: req.initial_capital,
        cost,
        limits: params.limits.clone(),
        universe: universe.clone(),
    };
    let schedule = monthly_schedule(&cal, req.from, req.to);
    let weights = weights_schedule(
        &schedule,
        &universe,
        &shortable,
        strategy.as_ref(),
        &view,
        &params,
    )?;
    let engine = simulate(&sim, &weights, &view, &cal)?;

    let inputs = vector_inputs(req, &universe, &weights, &view, &cal, cost)?;
    let vector = vector_equity(&inputs)?;
    // The vector series starts one session early, before anything is held.
    let vector = vector.get(1..).unwrap_or_default();

    let dates: Vec<_> = engine.equity.iter().map(|(d, _)| *d).collect();
    let engine_equity: Vec<Decimal> = engine.equity.iter().map(|(_, e)| *e).collect();
    Ok(compare(&dates, &engine_equity, vector, tolerance)?)
}

/// The vector calculator's inputs. The dates start at the session before `req.from`
/// so that a trade on the first session has a previous close to size from, as in the
/// engine; that extra session holds nothing and is dropped from the comparison.
fn vector_inputs(
    req: &BacktestRequest,
    universe: &[Symbol],
    weights: &BTreeMap<chrono::NaiveDate, pt_core::types::Weights>,
    view: &dyn MarketView,
    cal: &TradingCalendar,
    cost: CostSpec,
) -> Result<VectorInputs, ValidateError> {
    let mut dates = Vec::new();
    if let Some(prev) = cal.prev_session(req.from) {
        dates.push(prev);
    }
    dates.extend_from_slice(cal.sessions_between(req.from, req.to));

    let mut open = Vec::with_capacity(dates.len());
    let mut close = Vec::with_capacity(dates.len());
    let mut days_to_next = Vec::with_capacity(dates.len());
    for &date in &dates {
        let mut o = Vec::with_capacity(universe.len());
        let mut c = Vec::with_capacity(universe.len());
        for s in universe {
            let bar = view
                .exec_bar(s, date)
                .ok_or_else(|| EngineError::MissingBar {
                    symbol: s.to_string(),
                    date,
                })?;
            o.push(bar.open.value());
            c.push(bar.close.value());
        }
        open.push(o);
        close.push(c);
        // Borrow accrues to the next session, as in the engine; 1 on the last one.
        days_to_next.push(
            cal.next_session(date)
                .map_or(1, |next| (next - date).num_days()),
        );
    }

    let mut rows = BTreeMap::new();
    for (date, w) in weights {
        if let Some(s) = w.keys().find(|s| !universe.contains(s)) {
            return Err(ValidateError::Invalid(format!(
                "weight for {s} on {date} is outside the universe"
            )));
        }
        let row = universe
            .iter()
            .map(|s| w.get(s).copied().unwrap_or(Decimal::ZERO))
            .collect();
        rows.insert(*date, row);
    }

    Ok(VectorInputs {
        dates,
        symbols: universe.to_vec(),
        open,
        close,
        weights: rows,
        days_to_next,
        initial_capital: req.initial_capital,
        cost,
    })
}

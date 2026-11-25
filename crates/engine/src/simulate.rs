//! The backtest loop: weights by trade date become orders, fills, marks and one
//! equity point per session (architecture 5.10.5).

use std::collections::BTreeMap;

use chrono::NaiveDate;
use pt_calendar::calendar::TradingCalendar;
use pt_core::cost::CostSpec;
use pt_core::money::{Money, Price};
use pt_core::types::{Fill, OrderIntent, Symbol, Weights};
use pt_core::view::MarketView;
use pt_portfolio::orders::{plan_orders, target_shares};
use pt_risk::halts::HaltReason;
use pt_risk::pretrade::{check_orders, Rejection, RiskLimits};
use rust_decimal::Decimal;

use crate::book::Book;
use crate::broker::SimBroker;
use crate::error::EngineError;

/// What a simulation runs over and with.
#[derive(Debug, Clone, PartialEq)]
pub struct SimConfig {
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub initial_capital: Decimal,
    pub cost: CostSpec,
    pub limits: RiskLimits,
    pub universe: Vec<Symbol>,
}

/// The outcome of a simulation.
#[derive(Debug, Clone, PartialEq)]
pub struct SimResult {
    /// One point per session in the range, at the close.
    pub equity: Vec<(NaiveDate, Decimal)>,
    /// Every fill in execution order, with its trade date.
    pub fills: Vec<(NaiveDate, Fill)>,
    pub rejected: Vec<(NaiveDate, OrderIntent, Rejection)>,
    pub would_halt: Vec<(NaiveDate, HaltReason)>,
    pub cost_total: Decimal,
    pub borrow_total: Decimal,
    /// Sum of `qty * price` over every fill.
    pub traded_notional: Decimal,
}

impl SimResult {
    /// True when no order was rejected.
    pub fn clean(&self) -> bool {
        self.rejected.is_empty()
    }
}

/// Run the sessions from `cfg.start` to `cfg.end`. `weights` is keyed by trade date.
///
/// On a trade date the equity at the previous close (or the initial capital on the
/// first session) and the previous session's execution closes size the targets;
/// the orders pass the pre-trade checks, fill at the open, and the book is marked
/// at the close.
pub fn simulate(
    cfg: &SimConfig,
    weights: &BTreeMap<NaiveDate, Weights>,
    view: &dyn MarketView,
    cal: &TradingCalendar,
) -> Result<SimResult, EngineError> {
    let broker = SimBroker::new(cfg.cost);
    let mut book = Book::new(cfg.initial_capital);
    let mut result = SimResult {
        equity: Vec::new(),
        fills: Vec::new(),
        rejected: Vec::new(),
        would_halt: Vec::new(),
        cost_total: Decimal::ZERO,
        borrow_total: Decimal::ZERO,
        traded_notional: Decimal::ZERO,
    };
    for &date in cal.sessions_between(cfg.start, cfg.end) {
        if let Some(w) = weights.get(&date) {
            let equity = result
                .equity
                .last()
                .map_or(cfg.initial_capital, |&(_, e)| e);
            trade(
                cfg,
                &broker,
                &mut book,
                &mut result,
                date,
                w,
                equity,
                view,
                cal,
            )?;
        }
        let closes = closes_on(book.positions.keys(), date, view)?;
        let equity = book.equity(&closes)?;
        result.equity.push((date, equity));
    }
    Ok(result)
}

/// Size, check and fill the orders for one trade date.
#[allow(clippy::too_many_arguments)]
fn trade(
    cfg: &SimConfig,
    broker: &SimBroker,
    book: &mut Book,
    result: &mut SimResult,
    date: NaiveDate,
    weights: &Weights,
    equity: Decimal,
    view: &dyn MarketView,
    cal: &TradingCalendar,
) -> Result<(), EngineError> {
    let ref_date = cal.prev_session(date).ok_or_else(|| {
        EngineError::Invalid(format!("no session before trade date {date} for prices"))
    })?;
    let priced = weights
        .iter()
        .filter(|(_, w)| !w.is_zero())
        .map(|(s, _)| s)
        .chain(book.positions.keys());
    let ref_prices = closes_on(priced, ref_date, view)?;
    let target = target_shares(weights, Money(equity), &ref_prices)?;
    let orders = plan_orders(&book.positions, &target, &ref_prices)?;
    let gate = check_orders(
        &orders,
        &book.positions,
        Money(equity),
        &ref_prices,
        &cfg.limits,
    );
    result
        .rejected
        .extend(gate.rejected.into_iter().map(|(o, r)| (date, o, r)));
    for fill in broker.execute(date, &gate.approved, view)? {
        book.apply_fill(&fill);
        result.cost_total += fill.cost.0;
        result.traded_notional += fill.price.value() * Decimal::from(fill.qty);
        result.fills.push((date, fill));
    }
    Ok(())
}

/// Execution-basis closes of `symbols` on `date`. A missing bar is `MissingBar`.
fn closes_on<'a>(
    symbols: impl Iterator<Item = &'a Symbol>,
    date: NaiveDate,
    view: &dyn MarketView,
) -> Result<BTreeMap<Symbol, Price>, EngineError> {
    let mut out = BTreeMap::new();
    for symbol in symbols {
        let bar = view
            .exec_bar(symbol, date)
            .ok_or_else(|| EngineError::MissingBar {
                symbol: symbol.to_string(),
                date,
            })?;
        out.insert(symbol.clone(), bar.close);
    }
    Ok(out)
}

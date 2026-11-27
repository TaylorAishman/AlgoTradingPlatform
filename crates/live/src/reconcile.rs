//! Reconciliation of the local paper book against the broker account.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use pt_alpaca::{account, orders_read, reference};
use pt_store::trading::{Book, LedgerEntry};

use crate::context::LiveContext;
use crate::error::LiveError;
use crate::halts::set_halt;
use crate::snapshot::expected_last_session;

/// The outcome of one reconciliation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileReport {
    pub clean: bool,
    pub halt_reason: Option<String>,
}

/// Set a halt and record a critical `halt_set` alert, unless a halt with this
/// reason is already active (a halted run still reconciles). Returns the
/// unclean report.
fn halt(ctx: &LiveContext<'_>, reason: &str, detail: &str) -> Result<ReconcileReport, LiveError> {
    set_halt(ctx, reason, detail)?;
    Ok(ReconcileReport {
        clean: false,
        halt_reason: Some(reason.to_string()),
    })
}

/// Reconcile the paper book with the broker. On the first run, with an empty
/// ledger and a flat account, record the account cash as the opening balance.
pub fn reconcile(ctx: &LiveContext<'_>) -> Result<ReconcileReport, LiveError> {
    if ctx.store.ledger(Book::Paper)?.is_empty() {
        let held = account::positions(ctx.http)?;
        if !held.is_empty() {
            let detail = held
                .iter()
                .map(|p| format!("{} {}", p.symbol, p.qty))
                .collect::<Vec<_>>()
                .join(", ");
            return halt(ctx, "account_not_flat", &detail);
        }
        let acct = account::account(ctx.http)?;
        let clock = reference::clock(ctx.http)?;
        let calendar = ctx.store.calendar()?;
        let date = expected_last_session(&clock, &calendar)
            .ok_or_else(|| LiveError::Invalid("no session before the trade session".into()))?;
        ctx.store.append_ledger(&LedgerEntry {
            book: Book::Paper,
            date,
            kind: "open_balance".to_string(),
            symbol: None,
            qty: 0,
            price: None,
            cash_delta: acct.cash,
            ref_id: None,
        })?;
    }
    let mut foreign = Vec::new();
    for o in orders_read::orders_since(ctx.http, DateTime::<Utc>::UNIX_EPOCH)? {
        if ctx.store.order(&o.client_order_id)?.is_none() {
            foreign.push(o.client_order_id);
        }
    }
    if !foreign.is_empty() {
        let detail = foreign.join(", ");
        return halt(ctx, "foreign_order", &detail);
    }
    let mut local: BTreeMap<String, i64> = ctx
        .store
        .positions(Book::Paper)?
        .into_iter()
        .map(|(s, q)| (s.as_str().to_string(), q))
        .collect();
    let mut broker: BTreeMap<String, i64> = BTreeMap::new();
    for p in account::positions(ctx.http)? {
        *broker.entry(p.symbol).or_insert(0) += p.qty;
    }
    let mut symbols: Vec<String> = local.keys().chain(broker.keys()).cloned().collect();
    symbols.sort();
    symbols.dedup();
    let breaks: Vec<String> = symbols
        .iter()
        .filter_map(|s| {
            let l = local.remove(s).unwrap_or(0);
            let b = broker.get(s).copied().unwrap_or(0);
            (l != b).then(|| format!("{s}: broker {b}, ledger {l}"))
        })
        .collect();
    if !breaks.is_empty() {
        let detail = breaks.join("; ");
        return halt(ctx, "reconciliation_position", &detail);
    }
    let broker_cash = account::account(ctx.http)?.cash;
    let ledger_cash = ctx.store.cash(Book::Paper)?;
    if (broker_cash - ledger_cash).abs() > ctx.cfg.risk.cash_tolerance {
        let detail = format!("broker {broker_cash}, ledger {ledger_cash}");
        return halt(ctx, "reconciliation_cash", &detail);
    }
    Ok(ReconcileReport {
        clean: true,
        halt_reason: None,
    })
}

//! Reconciliation of the local paper book against the broker account.

use chrono::{DateTime, Utc};
use pt_alpaca::{account, orders_read, reference};
use pt_ops::alert::Level;
use pt_store::trading::{Book, LedgerEntry};

use crate::context::LiveContext;
use crate::error::LiveError;
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
    let active = ctx.store.active_halts()?;
    if !active.iter().any(|h| h.reason == reason) {
        let now = ctx.clock.now_utc();
        ctx.store.set_halt(reason, detail, now)?;
        ctx.alerter.alert(
            ctx.store,
            Level::Critical,
            "halt_set",
            &format!("halt {reason}: {detail}"),
            None,
            now,
        );
    }
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
    Ok(ReconcileReport {
        clean: true,
        halt_reason: None,
    })
}

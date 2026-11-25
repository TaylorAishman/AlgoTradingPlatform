//! Reconciliation of the local paper book against the broker account.

use pt_alpaca::{account, reference};
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

/// Reconcile the paper book with the broker. On the first run, with an empty
/// ledger and a flat account, record the account cash as the opening balance.
pub fn reconcile(ctx: &LiveContext<'_>) -> Result<ReconcileReport, LiveError> {
    if ctx.store.ledger(Book::Paper)?.is_empty() {
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
    Ok(ReconcileReport {
        clean: true,
        halt_reason: None,
    })
}

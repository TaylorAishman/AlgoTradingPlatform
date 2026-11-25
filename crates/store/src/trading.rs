//! Repositories for decisions, orders, fills and the ledgers.

use chrono::{DateTime, NaiveDate, NaiveTime, SecondsFormat, Utc};
use pt_core::order_state::{transition_allowed, OrderState};
use pt_core::types::{Positions, Side, Symbol};
use rusqlite::{params, params_from_iter, Row, Transaction, TransactionBehavior};
use rust_decimal::Decimal;

use crate::codec::{dec_to_text, text_to_dec};
use crate::db::Store;
use crate::error::StoreError;

/// Whether a decision was only computed (`DryRun`) or sent to the broker (`Paper`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    DryRun,
    Paper,
}

impl Mode {
    fn as_text(self) -> &'static str {
        match self {
            Mode::DryRun => "dry_run",
            Mode::Paper => "paper",
        }
    }

    fn from_text(s: &str) -> Result<Mode, StoreError> {
        match s {
            "dry_run" => Ok(Mode::DryRun),
            "paper" => Ok(Mode::Paper),
            other => Err(StoreError::Codec(format!("invalid mode {other:?}"))),
        }
    }
}

/// One decision: the inputs and outputs of a strategy on a signal date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionRow {
    pub id: i64,
    pub strategy_id: String,
    pub signal_date: NaiveDate,
    pub trade_date: Option<NaiveDate>,
    pub mode: Mode,
    pub status: String,
    pub inputs_json: String,
    pub outputs_json: String,
    pub created_at: DateTime<Utc>,
}

const DECISION_COLUMNS: &str =
    "id, strategy_id, signal_date, trade_date, mode, status, inputs_json, outputs_json, created_at";

/// One order as stored.
#[derive(Debug, Clone, PartialEq)]
pub struct OrderRow {
    pub client_order_id: String,
    pub decision_id: Option<i64>,
    pub symbol: Symbol,
    pub side: Side,
    pub qty: u64,
    pub leg: u8,
    pub state: OrderState,
    pub broker_order_id: Option<String>,
    pub filled_qty: u64,
    pub reference_price: Decimal,
    pub trade_date: NaiveDate,
}

const ORDER_COLUMNS: &str = "client_order_id, decision_id, symbol, side, qty, leg, state, \
     broker_order_id, filled_qty, reference_price, trade_date";

/// One broker fill, keyed by its activity id.
#[derive(Debug, Clone, PartialEq)]
pub struct FillRow {
    pub activity_id: String,
    pub broker_order_id: String,
    pub symbol: Symbol,
    pub side: Side,
    pub qty: u64,
    pub price: Decimal,
    pub at: DateTime<Utc>,
}

fn side_to_text(s: Side) -> &'static str {
    match s {
        Side::Buy => "buy",
        Side::Sell => "sell",
    }
}

fn text_to_side(s: &str) -> Result<Side, StoreError> {
    match s {
        "buy" => Ok(Side::Buy),
        "sell" => Ok(Side::Sell),
        other => Err(StoreError::Codec(format!("invalid side {other:?}"))),
    }
}

fn state_to_text(s: OrderState) -> &'static str {
    match s {
        OrderState::Intent => "intent",
        OrderState::DryRun => "dry_run",
        OrderState::Submitted => "submitted",
        OrderState::PartiallyFilled => "partially_filled",
        OrderState::Filled => "filled",
        OrderState::Canceled => "canceled",
        OrderState::Expired => "expired",
        OrderState::Rejected => "rejected",
        OrderState::Abandoned => "abandoned",
    }
}

fn text_to_state(s: &str) -> Result<OrderState, StoreError> {
    Ok(match s {
        "intent" => OrderState::Intent,
        "dry_run" => OrderState::DryRun,
        "submitted" => OrderState::Submitted,
        "partially_filled" => OrderState::PartiallyFilled,
        "filled" => OrderState::Filled,
        "canceled" => OrderState::Canceled,
        "expired" => OrderState::Expired,
        "rejected" => OrderState::Rejected,
        "abandoned" => OrderState::Abandoned,
        other => return Err(StoreError::Codec(format!("invalid order state {other:?}"))),
    })
}

fn to_i64(v: u64, what: &str) -> Result<i64, StoreError> {
    i64::try_from(v).map_err(|_| StoreError::Codec(format!("{what} {v} out of range")))
}

fn order_from_row(r: &Row<'_>) -> Result<OrderRow, StoreError> {
    let qty: i64 = r.get(4)?;
    let leg: i64 = r.get(5)?;
    let filled: i64 = r.get(8)?;
    Ok(OrderRow {
        client_order_id: r.get(0)?,
        decision_id: r.get(1)?,
        symbol: Symbol::new(&r.get::<_, String>(2)?)
            .map_err(|e| StoreError::Codec(format!("invalid symbol: {e}")))?,
        side: text_to_side(&r.get::<_, String>(3)?)?,
        qty: u64::try_from(qty).map_err(|_| StoreError::Codec(format!("invalid qty {qty}")))?,
        leg: u8::try_from(leg).map_err(|_| StoreError::Codec(format!("invalid leg {leg}")))?,
        state: text_to_state(&r.get::<_, String>(6)?)?,
        broker_order_id: r.get(7)?,
        filled_qty: u64::try_from(filled)
            .map_err(|_| StoreError::Codec(format!("invalid filled_qty {filled}")))?,
        reference_price: text_to_dec(&r.get::<_, String>(9)?)?,
        trade_date: text_to_date(&r.get::<_, String>(10)?)?,
    })
}

fn date_to_text(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

fn text_to_date(s: &str) -> Result<NaiveDate, StoreError> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| StoreError::Codec(format!("invalid date {s:?}: {e}")))
}

fn time_to_text(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::AutoSi, true)
}

fn text_to_time(s: &str) -> Result<DateTime<Utc>, StoreError> {
    DateTime::parse_from_rfc3339(s)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|e| StoreError::Codec(format!("invalid timestamp {s:?}: {e}")))
}

fn decision_from_row(r: &Row<'_>) -> Result<DecisionRow, StoreError> {
    let trade_date: Option<String> = r.get(3)?;
    Ok(DecisionRow {
        id: r.get(0)?,
        strategy_id: r.get(1)?,
        signal_date: text_to_date(&r.get::<_, String>(2)?)?,
        trade_date: trade_date.as_deref().map(text_to_date).transpose()?,
        mode: Mode::from_text(&r.get::<_, String>(4)?)?,
        status: r.get(5)?,
        inputs_json: r.get(6)?,
        outputs_json: r.get(7)?,
        created_at: text_to_time(&r.get::<_, String>(8)?)?,
    })
}

fn is_unique_violation(e: &rusqlite::Error) -> bool {
    matches!(
        e,
        rusqlite::Error::SqliteFailure(f, _)
            if f.code == rusqlite::ErrorCode::ConstraintViolation
                && f.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE
    )
}

impl Store {
    /// Inserts a decision and returns its id. The `id` field of `d` is ignored.
    /// A second decision for the same strategy, signal date and mode is
    /// `StoreError::Duplicate`.
    pub fn insert_decision(&self, d: &DecisionRow) -> Result<i64, StoreError> {
        let res = self.conn.execute(
            "INSERT INTO decisions (strategy_id, signal_date, trade_date, mode, status, \
             inputs_json, outputs_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                d.strategy_id,
                date_to_text(d.signal_date),
                d.trade_date.map(date_to_text),
                d.mode.as_text(),
                d.status,
                d.inputs_json,
                d.outputs_json,
                time_to_text(d.created_at),
            ],
        );
        match res {
            Ok(_) => Ok(self.conn.last_insert_rowid()),
            Err(e) if is_unique_violation(&e) => Err(StoreError::Duplicate(format!(
                "decision for {} on {} in mode {}",
                d.strategy_id,
                date_to_text(d.signal_date),
                d.mode.as_text()
            ))),
            Err(e) => Err(e.into()),
        }
    }

    /// The decision for a strategy, signal date and mode, if one is stored.
    pub fn decision_for(
        &self,
        strategy_id: &str,
        signal_date: NaiveDate,
        mode: Mode,
    ) -> Result<Option<DecisionRow>, StoreError> {
        let sql = format!(
            "SELECT {DECISION_COLUMNS} FROM decisions \
             WHERE strategy_id = ?1 AND signal_date = ?2 AND mode = ?3"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query(params![
            strategy_id,
            date_to_text(signal_date),
            mode.as_text()
        ])?;
        match rows.next()? {
            Some(r) => Ok(Some(decision_from_row(r)?)),
            None => Ok(None),
        }
    }

    /// All decisions of one mode, in insertion order.
    pub fn decisions(&self, mode: Mode) -> Result<Vec<DecisionRow>, StoreError> {
        let sql = format!("SELECT {DECISION_COLUMNS} FROM decisions WHERE mode = ?1 ORDER BY id");
        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query(params![mode.as_text()])?;
        let mut out = Vec::new();
        while let Some(r) = rows.next()? {
            out.push(decision_from_row(r)?);
        }
        Ok(out)
    }

    /// Inserts an order and its creation event (`from_state` NULL) in one
    /// transaction. A repeated `client_order_id` is `StoreError::Duplicate`.
    pub fn insert_order(&self, o: &OrderRow) -> Result<(), StoreError> {
        let tx = Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate)?;
        // The creation event is stamped with the trade date at 00:00:00 UTC: library
        // crates read no wall clock and this signature carries no time.
        let created_at = o.trade_date.and_time(NaiveTime::MIN).and_utc();
        let res = tx.execute(
            "INSERT INTO orders (client_order_id, decision_id, symbol, side, qty, leg, state, \
             broker_order_id, filled_qty, reference_price, trade_date) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                o.client_order_id,
                o.decision_id,
                o.symbol.as_str(),
                side_to_text(o.side),
                to_i64(o.qty, "qty")?,
                i64::from(o.leg),
                state_to_text(o.state),
                o.broker_order_id,
                to_i64(o.filled_qty, "filled_qty")?,
                dec_to_text(o.reference_price),
                date_to_text(o.trade_date),
            ],
        );
        match res {
            Ok(_) => {}
            Err(rusqlite::Error::SqliteFailure(f, _))
                if f.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY =>
            {
                return Err(StoreError::Duplicate(format!(
                    "order {}",
                    o.client_order_id
                )));
            }
            Err(e) => return Err(e.into()),
        }
        tx.execute(
            "INSERT INTO order_events (client_order_id, from_state, to_state, filled_qty, \
             broker_order_id, at) VALUES (?1, NULL, ?2, ?3, ?4, ?5)",
            params![
                o.client_order_id,
                state_to_text(o.state),
                to_i64(o.filled_qty, "filled_qty")?,
                o.broker_order_id,
                time_to_text(created_at),
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Moves an order to `to`, checked against the transition table, and
    /// appends an `order_events` row, both in one transaction. A `None`
    /// `broker_order_id` keeps the stored one. An illegal transition is
    /// `StoreError::IllegalTransition` and writes nothing; an unknown id is
    /// `StoreError::NotFound`.
    pub fn set_order_state(
        &self,
        client_order_id: &str,
        to: OrderState,
        filled_qty: u64,
        broker_order_id: Option<&str>,
        at: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        let tx = Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate)?;
        let current: Option<(String, Option<String>)> = {
            let mut stmt =
                tx.prepare("SELECT state, broker_order_id FROM orders WHERE client_order_id = ?1")?;
            let mut rows = stmt.query(params![client_order_id])?;
            match rows.next()? {
                Some(r) => Some((r.get(0)?, r.get(1)?)),
                None => None,
            }
        };
        let (from_text, stored_broker) =
            current.ok_or_else(|| StoreError::NotFound(format!("order {client_order_id}")))?;
        let from = text_to_state(&from_text)?;
        if !transition_allowed(from, to) {
            return Err(StoreError::IllegalTransition {
                from: state_to_text(from).to_string(),
                to: state_to_text(to).to_string(),
            });
        }
        let broker = broker_order_id.map(str::to_string).or(stored_broker);
        let filled = to_i64(filled_qty, "filled_qty")?;
        tx.execute(
            "UPDATE orders SET state = ?1, filled_qty = ?2, broker_order_id = ?3 \
             WHERE client_order_id = ?4",
            params![state_to_text(to), filled, broker, client_order_id],
        )?;
        tx.execute(
            "INSERT INTO order_events (client_order_id, from_state, to_state, filled_qty, \
             broker_order_id, at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                client_order_id,
                state_to_text(from),
                state_to_text(to),
                filled,
                broker,
                time_to_text(at),
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// The order with this client order id, if stored.
    pub fn order(&self, client_order_id: &str) -> Result<Option<OrderRow>, StoreError> {
        let sql = format!("SELECT {ORDER_COLUMNS} FROM orders WHERE client_order_id = ?1");
        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query(params![client_order_id])?;
        match rows.next()? {
            Some(r) => Ok(Some(order_from_row(r)?)),
            None => Ok(None),
        }
    }

    /// Orders whose state is one of `states`, in insertion order. Empty input gives empty output.
    pub fn orders_in_states(&self, states: &[OrderState]) -> Result<Vec<OrderRow>, StoreError> {
        if states.is_empty() {
            return Ok(Vec::new());
        }
        let marks = vec!["?"; states.len()].join(", ");
        let sql =
            format!("SELECT {ORDER_COLUMNS} FROM orders WHERE state IN ({marks}) ORDER BY rowid");
        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query(params_from_iter(states.iter().map(|s| state_to_text(*s))))?;
        let mut out = Vec::new();
        while let Some(r) = rows.next()? {
            out.push(order_from_row(r)?);
        }
        Ok(out)
    }

    /// Records a fill. Returns true when a row was inserted, false when the
    /// activity id is already stored (the first row is kept).
    pub fn insert_fill(&self, f: &FillRow) -> Result<bool, StoreError> {
        let n = self.conn.execute(
            "INSERT OR IGNORE INTO fills (activity_id, broker_order_id, symbol, side, qty, \
             price, at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                f.activity_id,
                f.broker_order_id,
                f.symbol.as_str(),
                side_to_text(f.side),
                to_i64(f.qty, "qty")?,
                dec_to_text(f.price),
                time_to_text(f.at),
            ],
        )?;
        Ok(n == 1)
    }

    /// All fills, in insertion order.
    pub fn fills(&self) -> Result<Vec<FillRow>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT activity_id, broker_order_id, symbol, side, qty, price, at \
             FROM fills ORDER BY rowid",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(r) = rows.next()? {
            let qty: i64 = r.get(4)?;
            out.push(FillRow {
                activity_id: r.get(0)?,
                broker_order_id: r.get(1)?,
                symbol: Symbol::new(&r.get::<_, String>(2)?)
                    .map_err(|e| StoreError::Codec(format!("invalid symbol: {e}")))?,
                side: text_to_side(&r.get::<_, String>(3)?)?,
                qty: u64::try_from(qty)
                    .map_err(|_| StoreError::Codec(format!("invalid qty {qty}")))?,
                price: text_to_dec(&r.get::<_, String>(5)?)?,
                at: text_to_time(&r.get::<_, String>(6)?)?,
            });
        }
        Ok(out)
    }
}

/// Which ledger an entry belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Book {
    Paper,
    Shadow,
}

impl Book {
    fn as_text(self) -> &'static str {
        match self {
            Book::Paper => "paper",
            Book::Shadow => "shadow",
        }
    }
}

/// One cash and position movement in a book.
#[derive(Debug, Clone, PartialEq)]
pub struct LedgerEntry {
    pub book: Book,
    pub date: NaiveDate,
    pub kind: String,
    pub symbol: Option<Symbol>,
    pub qty: i64,
    pub price: Option<Decimal>,
    pub cash_delta: Decimal,
    pub ref_id: Option<String>,
}

impl Store {
    /// Appends a ledger entry and returns its id. Ids increase with insertion.
    pub fn append_ledger(&self, e: &LedgerEntry) -> Result<i64, StoreError> {
        self.conn.execute(
            "INSERT INTO ledger_entries (book, date, kind, symbol, qty, price, cash_delta, \
             ref_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                e.book.as_text(),
                date_to_text(e.date),
                e.kind,
                e.symbol.as_ref().map(Symbol::as_str),
                e.qty,
                e.price.map(dec_to_text),
                dec_to_text(e.cash_delta),
                e.ref_id,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// The entries of one book, in insertion order.
    pub fn ledger(&self, book: Book) -> Result<Vec<LedgerEntry>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT date, kind, symbol, qty, price, cash_delta, ref_id \
             FROM ledger_entries WHERE book = ?1 ORDER BY id",
        )?;
        let mut rows = stmt.query(params![book.as_text()])?;
        let mut out = Vec::new();
        while let Some(r) = rows.next()? {
            let symbol: Option<String> = r.get(2)?;
            let price: Option<String> = r.get(4)?;
            out.push(LedgerEntry {
                book,
                date: text_to_date(&r.get::<_, String>(0)?)?,
                kind: r.get(1)?,
                symbol: symbol
                    .map(|s| Symbol::new(&s))
                    .transpose()
                    .map_err(|e| StoreError::Codec(format!("invalid symbol: {e}")))?,
                qty: r.get(3)?,
                price: price.map(|p| text_to_dec(&p)).transpose()?,
                cash_delta: text_to_dec(&r.get::<_, String>(5)?)?,
                ref_id: r.get(6)?,
            });
        }
        Ok(out)
    }

    /// Net quantity per symbol in a book; symbols with a net of zero are absent.
    pub fn positions(&self, book: Book) -> Result<Positions, StoreError> {
        let mut out = Positions::new();
        for e in self.ledger(book)? {
            if let Some(s) = e.symbol {
                *out.entry(s).or_insert(0) += e.qty;
            }
        }
        out.retain(|_, q| *q != 0);
        Ok(out)
    }

    /// The sum of the cash deltas in a book; zero when it has no entries.
    pub fn cash(&self, book: Book) -> Result<Decimal, StoreError> {
        Ok(self
            .ledger(book)?
            .iter()
            .fold(Decimal::ZERO, |acc, e| acc + e.cash_delta))
    }
}

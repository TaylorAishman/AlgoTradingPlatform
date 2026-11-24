//! Shared mock state: the request log and the seeded account and clock.

/// One request the mock received.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recorded {
    pub method: String,
    pub path: String,
    pub query: String,
    pub body: String,
}

/// The seeded market clock; all fields are served verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clock {
    pub timestamp: String,
    pub is_open: bool,
    pub next_open: String,
    pub next_close: String,
}

/// A seeded asset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub symbol: String,
    pub tradable: bool,
    pub shortable: bool,
    pub easy_to_borrow: bool,
}

/// A seeded daily bar.
#[derive(Debug, Clone, PartialEq)]
pub struct Bar {
    pub adjustment: String,
    pub feed: String,
    pub symbol: String,
    /// Session date, YYYY-MM-DD.
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: u64,
}

use chrono::{DateTime, Duration, SecondsFormat, TimeZone, Utc};

/// An order the mock accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order {
    pub id: String,
    pub client_order_id: String,
    pub symbol: String,
    pub qty: String,
    pub side: String,
    pub order_type: String,
    pub time_in_force: String,
    pub status: String,
    /// When the order was accepted, RFC 3339 UTC.
    pub submitted_at: String,
    pub filled_qty: String,
    /// Total cost of the fills so far, at `SCALE`.
    pub filled_notional: i128,
    /// Weighted average fill price; `None` until the first fill.
    pub filled_avg_price: Option<String>,
}

/// One fill, served by the FILL activities route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Activity {
    pub id: String,
    pub order_id: String,
    pub symbol: String,
    pub side: String,
    pub qty: String,
    pub price: String,
    /// When the fill happened, RFC 3339 UTC; strictly increasing across fills.
    pub transaction_time: String,
    /// `fill` for a completing fill, `partial_fill` otherwise.
    pub kind: String,
}

/// Fixed-point scale for exact money arithmetic: nine decimal places.
pub const SCALE: i128 = 1_000_000_000;

/// Parse a plain decimal string (optional leading `-`, at most nine decimals)
/// into an integer at `SCALE`. Anything else is `None`.
pub fn parse_dec(s: &str) -> Option<i128> {
    let (neg, rest) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s),
    };
    let (int, frac) = rest.split_once('.').unwrap_or((rest, ""));
    let plain = |p: &str| p.bytes().all(|b| b.is_ascii_digit());
    if int.is_empty() || !plain(int) || !plain(frac) || frac.len() > 9 || int.len() > 20 {
        return None;
    }
    let frac_scaled = format!("{frac:0<9}").parse::<i128>().ok()?;
    let n = int
        .parse::<i128>()
        .ok()?
        .checked_mul(SCALE)?
        .checked_add(frac_scaled)?;
    Some(if neg { -n } else { n })
}

/// Format an integer at `SCALE` as a decimal string with trailing zeros trimmed.
pub fn fmt_dec(n: i128) -> String {
    let sign = if n < 0 { "-" } else { "" };
    let a = n.unsigned_abs();
    let scale = SCALE.unsigned_abs();
    let (int, frac) = (a / scale, a % scale);
    if frac == 0 {
        format!("{sign}{int}")
    } else {
        let f = format!("{frac:09}");
        format!("{sign}{int}.{}", f.trim_end_matches('0'))
    }
}

/// Mutable state behind the server.
#[derive(Debug, Default)]
pub struct State {
    pub requests: Vec<Recorded>,
    /// Seeded cash, served as both cash and equity.
    pub account_cash: Option<String>,
    pub clock: Option<Clock>,
    /// Seeded session dates (YYYY-MM-DD), served by `/v2/calendar`.
    pub sessions: Vec<String>,
    pub assets: Vec<Asset>,
    pub bars: Vec<Bar>,
    /// Accepted orders, oldest first.
    pub orders: Vec<Order>,
    /// Signed position quantity per symbol, at `SCALE`; flat symbols are removed.
    pub positions: Vec<(String, i128)>,
    /// Fills in the order they happened.
    pub activities: Vec<Activity>,
    /// Feeds whose bars are answered 403.
    pub denied_feeds: Vec<String>,
    /// Injected failures: (requests left, status).
    pub fail_next: Option<(u32, u16)>,
}

impl State {
    /// A deterministic RFC 3339 UTC timestamp `offset` seconds after the seeded
    /// clock (or a fixed instant when no clock is seeded), so that a larger
    /// offset is always a later timestamp.
    pub fn stamp(&self, offset: usize) -> String {
        let fixed = Utc.with_ymd_and_hms(2024, 1, 2, 14, 30, 0).unwrap();
        let base = self
            .clock
            .as_ref()
            .and_then(|c| DateTime::parse_from_rfc3339(&c.timestamp).ok())
            .map_or(fixed, |d| d.with_timezone(&Utc));
        let secs = i64::try_from(offset).unwrap_or(i64::MAX / 2);
        (base + Duration::seconds(secs)).to_rfc3339_opts(SecondsFormat::Secs, true)
    }
}

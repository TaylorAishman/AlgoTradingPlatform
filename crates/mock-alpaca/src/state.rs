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
    /// Feeds whose bars are answered 403.
    pub denied_feeds: Vec<String>,
    /// Injected failures: (requests left, status).
    pub fail_next: Option<(u32, u16)>,
}

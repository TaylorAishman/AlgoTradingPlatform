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
}

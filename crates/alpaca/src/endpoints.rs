//! The hosts the Alpaca client talks to.

/// The Alpaca paper trading host.
pub const PAPER_TRADING_URL: &str = "https://paper-api.alpaca.markets";

/// The Alpaca market data host.
pub const MARKET_DATA_URL: &str = "https://data.alpaca.markets";

/// The trading and market data base URLs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoints {
    trading: String,
    data: String,
}

impl Endpoints {
    /// The default endpoints: the paper trading host and the market data host.
    pub fn production() -> Self {
        Self {
            trading: PAPER_TRADING_URL.to_string(),
            data: MARKET_DATA_URL.to_string(),
        }
    }

    /// The trading base URL.
    pub fn trading(&self) -> &str {
        &self.trading
    }

    /// The market data base URL.
    pub fn data(&self) -> &str {
        &self.data
    }
}

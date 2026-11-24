//! The hosts the Alpaca client talks to.

use crate::error::AlpacaError;

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

    /// Endpoints with optional overrides. Only loopback hosts (`127.0.0.1`,
    /// `localhost`) are accepted; if either override is refused, the pair is.
    pub fn with_overrides(trading: Option<&str>, data: Option<&str>) -> Result<Self, AlpacaError> {
        for url in [trading, data].into_iter().flatten() {
            if !is_loopback(url) {
                return Err(AlpacaError::EndpointNotAllowed(url.to_string()));
            }
        }
        let mut e = Self::production();
        if let Some(u) = trading {
            e.trading = u.to_string();
        }
        if let Some(u) = data {
            e.data = u.to_string();
        }
        Ok(e)
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

/// True when `url` is `http(s)://` followed by exactly a loopback host.
fn is_loopback(url: &str) -> bool {
    let rest = match url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
    {
        Some(r) => r,
        None => return false,
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if authority.contains('@') {
        return false;
    }
    let host = match authority.split_once(':') {
        Some((h, port)) => {
            if port.is_empty() || !port.bytes().all(|b| b.is_ascii_digit()) {
                return false;
            }
            h
        }
        None => authority,
    };
    host == "127.0.0.1" || host == "localhost"
}

//! Account endpoint: cash, equity and trading flags.

use std::str::FromStr;

use rust_decimal::Decimal;

use crate::error::AlpacaError;
use crate::http::HttpClient;

/// The account fields the system reads. Money is parsed exactly from strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireAccount {
    pub status: String,
    pub cash: Decimal,
    pub equity: Decimal,
    pub last_equity: Decimal,
    pub buying_power: Decimal,
    pub multiplier: Decimal,
    pub shorting_enabled: bool,
    pub trading_blocked: bool,
    pub account_blocked: bool,
}

/// GET /v2/account on the trading base URL.
pub fn account(http: &HttpClient) -> Result<WireAccount, AlpacaError> {
    let body = http.get_trading("/v2/account")?;
    let v: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| AlpacaError::Parse(e.to_string()))?;
    let missing = |k: &str| AlpacaError::Parse(format!("account field {k} missing"));
    let text = |k: &str| -> Result<String, AlpacaError> {
        v.get(k)
            .and_then(|x| x.as_str())
            .map(str::to_string)
            .ok_or_else(|| missing(k))
    };
    let money = |k: &str| -> Result<Decimal, AlpacaError> {
        Decimal::from_str(&text(k)?).map_err(|e| AlpacaError::Parse(format!("{k}: {e}")))
    };
    let flag = |k: &str| -> Result<bool, AlpacaError> {
        v.get(k).and_then(|x| x.as_bool()).ok_or_else(|| missing(k))
    };
    Ok(WireAccount {
        status: text("status")?,
        cash: money("cash")?,
        equity: money("equity")?,
        last_equity: money("last_equity")?,
        buying_power: money("buying_power")?,
        multiplier: money("multiplier")?,
        shorting_enabled: flag("shorting_enabled")?,
        trading_blocked: flag("trading_blocked")?,
        account_blocked: flag("account_blocked")?,
    })
}

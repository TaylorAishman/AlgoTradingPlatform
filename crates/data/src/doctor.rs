//! The smoke test: each check is reported, and a failure does not stop the rest.

use chrono::Duration;
use pt_alpaca::account::account;
use pt_alpaca::data::daily_bars;
use pt_alpaca::http::HttpClient;
use pt_alpaca::reference::{asset, clock};
use pt_config::config::Config;
use rust_decimal::Decimal;

/// One smoke-test result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorLine {
    pub check: &'static str,
    pub pass: bool,
    pub detail: String,
}

fn line(check: &'static str, pass: bool, detail: impl Into<String>) -> DoctorLine {
    DoctorLine {
        check,
        pass,
        detail: detail.into(),
    }
}

/// Runs the account, margin, clock, feed and asset checks and returns one line for each.
pub fn doctor(http: &HttpClient, cfg: &Config) -> Vec<DoctorLine> {
    let mut lines = Vec::new();
    match account(http) {
        Ok(a) => {
            let active = a.status == "ACTIVE" && !a.trading_blocked && !a.account_blocked;
            lines.push(line(
                "account_active",
                active,
                format!(
                    "status {}, trading_blocked {}, account_blocked {}",
                    a.status, a.trading_blocked, a.account_blocked
                ),
            ));
            lines.push(line(
                "shorting_enabled",
                a.shorting_enabled,
                format!("shorting_enabled {}", a.shorting_enabled),
            ));
            lines.push(line(
                "margin_multiplier",
                a.multiplier >= Decimal::from(2),
                format!("multiplier {}", a.multiplier),
            ));
        }
        Err(e) => {
            let detail = format!("account request failed: {e}");
            for check in ["account_active", "shorting_enabled", "margin_multiplier"] {
                lines.push(line(check, false, detail.clone()));
            }
        }
    }
    let now = match clock(http) {
        Ok(c) => {
            lines.push(line(
                "clock",
                true,
                format!("is_open {}, next_open {}", c.is_open, c.next_open),
            ));
            Some(c.timestamp.date_naive())
        }
        Err(e) => {
            lines.push(line("clock", false, format!("clock request failed: {e}")));
            None
        }
    };
    let symbol = cfg.universe.first().map(|i| i.symbol.clone());
    lines.push(feed_line(http, cfg, symbol.as_deref(), now));
    lines.push(asset_line(http, symbol.as_deref()));
    lines
}

fn feed_line(
    http: &HttpClient,
    cfg: &Config,
    symbol: Option<&str>,
    now: Option<chrono::NaiveDate>,
) -> DoctorLine {
    let feed = cfg.data.feed.as_str();
    let (Some(symbol), Some(end)) = (symbol, now) else {
        return line("feed", false, "no universe symbol or clock to probe with");
    };
    let start = end - Duration::days(14);
    match daily_bars(http, &[symbol.to_string()], start, end, "raw", feed) {
        Ok(bars) if bars.is_empty() => line(
            "feed",
            false,
            format!("feed {feed} returned no bars for {symbol} from {start} to {end}"),
        ),
        Ok(bars) => line(
            "feed",
            true,
            format!("feed {feed} returned {} bars for {symbol}", bars.len()),
        ),
        Err(e) => line(
            "feed",
            false,
            format!("feed {feed} request failed: {e}; consider data.feed = \"iex\""),
        ),
    }
}

fn asset_line(http: &HttpClient, symbol: Option<&str>) -> DoctorLine {
    let Some(symbol) = symbol else {
        return line("asset_fields", false, "no universe symbol to probe with");
    };
    match asset(http, symbol) {
        Ok(Some(a)) => line(
            "asset_fields",
            true,
            format!(
                "{} tradable {}, shortable {}, easy_to_borrow {}",
                a.symbol, a.tradable, a.shortable, a.easy_to_borrow
            ),
        ),
        Ok(None) => line("asset_fields", false, format!("asset {symbol} not found")),
        Err(e) => line("asset_fields", false, format!("asset request failed: {e}")),
    }
}

//! The smoke test: each check is reported, and a failure does not stop the rest.

use pt_alpaca::account::account;
use pt_alpaca::http::HttpClient;
use pt_alpaca::reference::clock;
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

/// Runs the account, margin and clock checks and returns one line for each.
pub fn doctor(http: &HttpClient, _cfg: &Config) -> Vec<DoctorLine> {
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
    match clock(http) {
        Ok(c) => lines.push(line(
            "clock",
            true,
            format!("is_open {}, next_open {}", c.is_open, c.next_open),
        )),
        Err(e) => lines.push(line("clock", false, format!("clock request failed: {e}"))),
    }
    lines
}

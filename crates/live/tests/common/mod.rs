//! The live fixture: a seeded mock broker, a store, a config, a gate, a strategy and a clock.
//! Later live tests build on this file and do not change it.
#![allow(dead_code)]

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use pt_alpaca::http::{loopback_client, HttpClient};
use pt_calendar::calendar::TradingCalendar;
use pt_config::config::{parse, Config};
use pt_core::clock::FixedClock;
use pt_core::types::Symbol;
use pt_gate::gate::Gate;
use pt_gate::owner::{OwnerFile, Stage};
use pt_live::context::LiveContext;
use pt_mock_alpaca::server::MockAlpaca;
use pt_ops::alert::Alerter;
use pt_store::db::Store;
use pt_strategy::tsmom::{Tsmom, TsmomParams};
use std::sync::Arc;

pub fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).expect("valid date")
}

pub fn sym(s: &str) -> Symbol {
    Symbol::new(s).expect("valid symbol")
}

/// Weekdays of 2024 minus the ten NYSE full-day holidays (252 sessions).
pub fn sessions_2024() -> Vec<NaiveDate> {
    use chrono::{Datelike, Weekday};
    let holidays = [
        d(2024, 1, 1),
        d(2024, 1, 15),
        d(2024, 2, 19),
        d(2024, 3, 29),
        d(2024, 5, 27),
        d(2024, 6, 19),
        d(2024, 7, 4),
        d(2024, 9, 2),
        d(2024, 11, 28),
        d(2024, 12, 25),
    ];
    let mut out = Vec::new();
    let mut day = d(2024, 1, 1);
    while day.year() == 2024 {
        if !matches!(day.weekday(), Weekday::Sat | Weekday::Sun) && !holidays.contains(&day) {
            out.push(day);
        }
        day = day.succ_opt().expect("next day");
    }
    out
}

const CONFIG_TEXT: &str = r#"
[data]
start = "2024-01-02"

[strategy]
lookback_months = [1]
risk_free_symbol = "BIL"
vol_min_obs = 5
vol_max_obs = 20

[sizing]
max_weight = 0.5

[risk]
max_order_notional = 60000

[[universe]]
symbol = "SPY"
asset_class = "equity"
shortable = true

[[universe]]
symbol = "TLT"
asset_class = "equity"
shortable = true
"#;

/// Everything a live test needs, owning what `LiveContext` borrows.
pub struct LiveFixture {
    pub mock: MockAlpaca,
    pub store: Store,
    pub cfg: Config,
    pub gate: Gate,
    pub http: HttpClient,
    pub clock: FixedClock,
    pub alerter: Alerter,
    pub strategy: Tsmom,
    pub calendar: Arc<TradingCalendar>,
    _dir: tempfile::TempDir,
}

impl LiveFixture {
    /// Build the fixture with the owner's stage set to `stage`.
    pub fn new(stage: Stage) -> LiveFixture {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = Store::open(&dir.path().join("live.sqlite")).expect("store");
        let sessions = sessions_2024();
        store.put_sessions(&sessions).expect("calendar");
        let calendar = Arc::new(TradingCalendar::new(sessions.clone()).expect("calendar"));

        let cfg = parse(CONFIG_TEXT).expect("fixture config parses and validates");

        let mock = MockAlpaca::start();
        mock.seed_account("100000");
        let dates: Vec<String> = sessions.iter().map(|s| s.to_string()).collect();
        let date_refs: Vec<&str> = dates.iter().map(String::as_str).collect();
        mock.seed_sessions(&date_refs);
        for s in ["SPY", "TLT", "BIL"] {
            mock.seed_asset(s, true, true, true);
        }
        mock.set_clock(
            "2024-03-01T08:00:00-05:00",
            false,
            "2024-03-01T09:30:00-05:00",
            "2024-03-01T16:00:00-05:00",
        );
        let mut raw = Vec::new();
        let mut prev: [Option<f64>; 3] = [None; 3];
        for (i, date) in sessions.iter().take(41).enumerate() {
            let i = i as i64;
            let closes = [
                (100 + i + (i % 2)) as f64,
                (1000 - 5 * i + 2 * (i % 3)) as f64 / 10.0,
                (10000 + i) as f64 / 100.0,
            ];
            for (k, name) in ["SPY", "TLT", "BIL"].iter().enumerate() {
                let close = closes[k];
                let open = prev[k].unwrap_or(close);
                prev[k] = Some(close);
                raw.push((
                    name.to_string(),
                    date.to_string(),
                    open,
                    open.max(close),
                    open.min(close),
                    close,
                    1_000_000u64,
                ));
            }
        }
        mock.seed_bars("raw", "sip", raw.clone());
        mock.seed_bars("all", "sip", raw);

        let gate = Gate::new(OwnerFile {
            stage,
            strategy_allowlist: vec!["tsmom-v1".to_string()],
            account_flat_confirmed_on: Some(d(2024, 2, 29)),
            ..OwnerFile::default()
        });
        let http = loopback_client(&mock.url()).expect("loopback client");
        let strategy = Tsmom::new(
            TsmomParams {
                lookback_months: cfg.strategy.lookback_months.clone(),
                risk_free_symbol: sym(&cfg.strategy.risk_free_symbol),
            },
            calendar.clone(),
        );
        // 08:00 at UTC-5 on 2024-03-01.
        let clock = FixedClock(Utc.with_ymd_and_hms(2024, 3, 1, 13, 0, 0).unwrap());
        LiveFixture {
            mock,
            store,
            cfg,
            gate,
            http,
            clock,
            alerter: Alerter::new(None, Vec::new()),
            strategy,
            calendar,
            _dir: dir,
        }
    }

    /// The instant the fixture's clock reports.
    pub fn now(&self) -> DateTime<Utc> {
        self.clock.0
    }

    /// A context borrowing everything in the fixture.
    pub fn ctx(&self) -> LiveContext<'_> {
        LiveContext {
            store: &self.store,
            cfg: &self.cfg,
            gate: &self.gate,
            http: &self.http,
            clock: &self.clock,
            alerter: &self.alerter,
            strategy: &self.strategy,
            code_version: "test-version",
        }
    }
}

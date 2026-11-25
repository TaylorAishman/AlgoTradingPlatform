//! `data` commands and the offline `calendar import`.

use std::path::Path;

use anyhow::{anyhow, Context};
use pt_core::types::Adjustment;
use pt_data::csv_import::{import_bars_csv, import_calendar_csv};
use pt_data::ingest::ingest;
use pt_data::quality::check_gaps;
use pt_data::reference::sync_calendar;
use pt_store::db::Store;

use crate::cli::DataCmd;
use crate::wiring;

fn open_store() -> anyhow::Result<Store> {
    let cfg = wiring::load_config()?;
    let path = Path::new(&cfg.run.db_path);
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    Store::open(path).with_context(|| format!("opening store {}", path.display()))
}

fn read_csv(csv: &Path) -> anyhow::Result<String> {
    std::fs::read_to_string(csv).with_context(|| format!("reading {}", csv.display()))
}

/// Import trading sessions from a CSV file. Needs no keys and no network.
pub fn calendar_import(csv: &Path) -> anyhow::Result<i32> {
    let text = read_csv(csv)?;
    let store = open_store()?;
    let n = import_calendar_csv(&store, &text)?;
    println!("imported {n} sessions");
    Ok(0)
}

pub fn data(action: &DataCmd) -> anyhow::Result<i32> {
    match action {
        DataCmd::ImportCsv {
            csv,
            adjustment,
            source,
        } => {
            let adjustment = match adjustment.as_str() {
                "raw" => Adjustment::Raw,
                "all" => Adjustment::All,
                other => return Err(anyhow!("unknown adjustment {other:?}: use raw or all")),
            };
            let text = read_csv(csv)?;
            let store = open_store()?;
            let n = import_bars_csv(&store, &text, adjustment, source, chrono::Utc::now())?;
            println!("imported {n} bars");
            Ok(0)
        }
        DataCmd::Ingest => data_ingest(),
        DataCmd::Check => check(),
    }
}

/// List every session missing from the stored raw bars of each universe
/// symbol and the risk-free symbol, up to the last stored session on or before
/// today, as `SYMBOL DATE` lines. Exits 1 when there is any (R-126, R-383).
fn check() -> anyhow::Result<i32> {
    let cfg = wiring::load_config()?;
    let store = open_store()?;
    let calendar = store.calendar().context("reading the calendar")?;
    let today = chrono::Utc::now().date_naive();
    let Some(through) = calendar
        .sessions_between(chrono::NaiveDate::MIN, today)
        .last()
        .copied()
    else {
        println!("no gaps");
        return Ok(0);
    };
    let gaps = check_gaps(&store, &cfg, through)?;
    if gaps.is_empty() {
        println!("no gaps");
        return Ok(0);
    }
    for gap in &gaps {
        println!("{} {}", gap.symbol, gap.date.format("%Y-%m-%d"));
    }
    Ok(1)
}

/// The market date at the broker now, and whether today's session has closed
/// (R-385). `next_close` falls on today's date only while today's close is
/// still ahead.
fn broker_today(http: &pt_alpaca::http::HttpClient) -> anyhow::Result<(chrono::NaiveDate, bool)> {
    let clock = pt_alpaca::reference::clock(http).context("reading the broker clock")?;
    let today = clock.timestamp.date_naive();
    let closed = clock.next_close.date_naive() > today;
    Ok((today, closed))
}

/// Fetch the broker calendar from the configured data start to the broker's
/// date and store it.
pub fn calendar_sync() -> anyhow::Result<i32> {
    let cfg = wiring::load_config()?;
    let store = open_store()?;
    let http = wiring::build_client()?;
    let (today, _) = broker_today(&http)?;
    let n = sync_calendar(&store, &http, cfg.data.start, today)?;
    println!("synced {n} sessions");
    Ok(0)
}

/// Fetch raw and adjusted bars through the expected last session: the latest
/// synced session that has closed at the broker's clock.
/// The expected last session: the latest synced session that has closed at
/// the broker's clock.
fn through_date(
    store: &Store,
    http: &pt_alpaca::http::HttpClient,
) -> anyhow::Result<chrono::NaiveDate> {
    let (today, closed) = broker_today(http)?;
    let calendar = store.calendar().context("reading the calendar")?;
    let last_day = if closed {
        today
    } else {
        today.pred_opt().context("date out of range")?
    };
    calendar
        .sessions_between(chrono::NaiveDate::MIN, last_day)
        .last()
        .copied()
        .ok_or_else(|| {
            anyhow!("no synced session on or before {last_day}: run `trader calendar sync`")
        })
}

/// Check every universe symbol against the broker and the stored bars, one
/// `SYMBOL PASS|FAIL reasons` line each. Exits 1 if any symbol fails.
pub fn universe_verify() -> anyhow::Result<i32> {
    let cfg = wiring::load_config()?;
    let store = open_store()?;
    let http = wiring::build_client()?;
    let through = through_date(&store, &http)?;
    let checks = pt_data::universe::verify_universe(&store, &http, &cfg, through)?;
    let mut failed = false;
    for c in &checks {
        if c.pass {
            println!("{} PASS", c.symbol);
        } else {
            failed = true;
            println!("{} FAIL {}", c.symbol, c.reasons.join(", "));
        }
    }
    Ok(i32::from(failed))
}

/// Fetch raw and adjusted bars through the expected last session.
fn data_ingest() -> anyhow::Result<i32> {
    let cfg = wiring::load_config()?;
    let store = open_store()?;
    let http = wiring::build_client()?;
    let through = through_date(&store, &http)?;
    let report = ingest(&store, &http, &cfg, through, chrono::Utc::now())?;
    for (name, stats) in [("raw", report.raw), ("adjusted", report.adjusted)] {
        println!(
            "{name}: {} inserted, {} unchanged, {} restated",
            stats.inserted, stats.unchanged, stats.restated
        );
    }
    if !report.complete {
        wiring::log("warn", &format!("some symbols lack a bar for {through}"));
    }
    println!("through {through}");
    Ok(0)
}

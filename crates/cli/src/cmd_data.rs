//! `data` commands and the offline `calendar import`.

use std::path::Path;

use anyhow::{anyhow, Context};
use pt_core::types::Adjustment;
use pt_data::csv_import::{import_bars_csv, import_calendar_csv};
use pt_data::quality::check_gaps;
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
        DataCmd::Check => check(),
        _ => anyhow::bail!("`data` is not implemented yet"),
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

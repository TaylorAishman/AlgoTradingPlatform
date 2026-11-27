//! `backtest` commands.

use std::path::Path;

use anyhow::Context;
use chrono::NaiveDate;
use pt_core::clock::SystemClock;
use pt_store::db::Store;
use pt_validate::crosscheck::crosscheck as run_crosscheck;
use pt_validate::error::ValidateError;
use pt_validate::overfit::dsr_for_run;
use pt_validate::runner::{run_and_record, BacktestRequest};
use rust_decimal::Decimal;

use crate::cli::BacktestCmd;
use crate::wiring;

/// Starting capital of a research backtest.
const INITIAL_CAPITAL: i64 = 100_000;

pub fn backtest(action: &BacktestCmd) -> anyhow::Result<i32> {
    match action {
        BacktestCmd::Run { from, to, strategy } => run(from, to, strategy.as_deref()),
        BacktestCmd::Crosscheck {
            from,
            to,
            with_costs,
        } => crosscheck(from, to, *with_costs),
    }
}

fn date(label: &str, text: &str) -> anyhow::Result<NaiveDate> {
    NaiveDate::parse_from_str(text, "%Y-%m-%d")
        .with_context(|| format!("{label} {text:?} is not a date (YYYY-MM-DD)"))
}

/// The trial-registry family a strategy name records under.
fn family(strategy: &str) -> &str {
    match strategy {
        "tsmom" => pt_strategy::tsmom::ID,
        "benchmark" => pt_strategy::benchmark::ID,
        other => other,
    }
}

fn run(from: &str, to: &str, strategy: Option<&str>) -> anyhow::Result<i32> {
    let cfg = wiring::load_config()?;
    let req = BacktestRequest {
        strategy: strategy.unwrap_or("tsmom").to_string(),
        from: date("--from", from)?,
        to: date("--to", to)?,
        initial_capital: Decimal::new(INITIAL_CAPITAL, 0),
    };
    let path = Path::new(&cfg.run.db_path);
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let store = Store::open(path).with_context(|| format!("opening store {}", path.display()))?;
    let out = run_and_record(&store, &cfg, &req, &SystemClock)?;
    let dsr = dsr_for_run(&store, &cfg, out.run_id, family(&req.strategy))?;
    let m = &out.metrics;
    println!("run {}", out.run_id);
    println!("trial {}", out.trial_hash);
    println!("sharpe {}", m.sharpe);
    println!("cagr {}", m.cagr);
    println!("vol {}", m.vol);
    println!("drawdown {}", m.max_drawdown);
    println!("turnover {}", m.turnover);
    println!("cost {}", m.cost_total);
    println!("borrow {}", m.borrow_total);
    println!("dsr {}", dsr.dsr);
    println!("trials {}", dsr.n_trials);
    Ok(0)
}

/// Run the engine and vector calculators on the same schedule and compare equity.
/// Exits 1 when they differ or the run cannot be compared.
fn crosscheck(from: &str, to: &str, with_costs: bool) -> anyhow::Result<i32> {
    let cfg = wiring::load_config()?;
    let req = BacktestRequest {
        strategy: "tsmom".to_string(),
        from: date("--from", from)?,
        to: date("--to", to)?,
        initial_capital: Decimal::new(INITIAL_CAPITAL, 0),
    };
    let path = Path::new(&cfg.run.db_path);
    let store = Store::open(path).with_context(|| format!("opening store {}", path.display()))?;
    match run_crosscheck(&store, &cfg, &req, !with_costs, Decimal::ZERO) {
        Ok(c) => {
            println!("max_abs_diff {}", c.max_abs_diff.normalize());
            if c.equal {
                println!("PASS");
                Ok(0)
            } else {
                println!("FAIL");
                if let Some(d) = c.first_diff_date {
                    println!("first_diff_date {d}");
                }
                Ok(1)
            }
        }
        Err(e @ ValidateError::NotComparable(_)) => {
            println!("{e}");
            Ok(1)
        }
        Err(e) => Err(e.into()),
    }
}

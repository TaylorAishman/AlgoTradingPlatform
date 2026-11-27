//! `backtest` commands.

use std::path::Path;

use anyhow::Context;
use chrono::NaiveDate;
use clap::Subcommand;
use pt_config::config::Config;
use pt_core::clock::SystemClock;
use pt_gate::owner::Stage;
use pt_store::db::Store;
use pt_validate::crosscheck::crosscheck as run_crosscheck;
use pt_validate::error::ValidateError;
use pt_validate::overfit::dsr_for_run;
use pt_validate::runner::{run_and_record_with, BacktestRequest, CapitalSource};
use rust_decimal::Decimal;

use crate::wiring;

#[derive(Debug, Subcommand)]
pub enum BacktestCmd {
    /// Run a backtest
    Run {
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        /// Strategy name (default from the config)
        #[arg(long)]
        strategy: Option<String>,
        /// Starting capital (default from the config)
        #[arg(long)]
        capital: Option<Decimal>,
    },
    /// Compare the event and vector engines
    Crosscheck {
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        with_costs: bool,
        /// Starting capital (default from the config; the account is never read)
        #[arg(long)]
        capital: Option<Decimal>,
    },
}

pub fn backtest(action: &BacktestCmd) -> anyhow::Result<i32> {
    match action {
        BacktestCmd::Run {
            from,
            to,
            strategy,
            capital,
        } => run(from, to, strategy.as_deref(), *capital),
        BacktestCmd::Crosscheck {
            from,
            to,
            with_costs,
            capital,
        } => crosscheck(from, to, *with_costs, *capital),
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

/// The capital when no flag is given: the account's equity when keys are set
/// and the stage is `ingest` or later, else the configured capital. A failed
/// account read warns and falls back; the backtest stage makes no request.
fn config_or_account_capital(cfg: &Config) -> anyhow::Result<(Decimal, CapitalSource)> {
    let fallback = (cfg.backtest.initial_capital, CapitalSource::Config);
    if wiring::load_keys().is_err() {
        return Ok(fallback);
    }
    if wiring::load_owner()?.stage < Stage::Ingest {
        wiring::log(
            "info",
            "stage is backtest: using the configured capital, no account request",
        );
        return Ok(fallback);
    }
    let read = wiring::build_client().and_then(|http| Ok(pt_alpaca::account::account(&http)?));
    match read {
        Ok(a) => Ok((a.equity, CapitalSource::Account)),
        Err(e) => {
            wiring::log(
                "warn",
                &format!("account read failed, using the configured capital: {e}"),
            );
            Ok(fallback)
        }
    }
}

fn run(
    from: &str,
    to: &str,
    strategy: Option<&str>,
    capital: Option<Decimal>,
) -> anyhow::Result<i32> {
    let cfg = wiring::load_config()?;
    let (initial_capital, source) = match capital {
        Some(c) if c <= Decimal::ZERO => {
            eprintln!("--capital must be greater than zero");
            return Ok(1);
        }
        Some(c) => (c, CapitalSource::Flag),
        None => config_or_account_capital(&cfg)?,
    };
    let req = BacktestRequest {
        strategy: strategy.unwrap_or("tsmom").to_string(),
        from: date("--from", from)?,
        to: date("--to", to)?,
        initial_capital,
    };
    let path = Path::new(&cfg.run.db_path);
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let store = Store::open(path).with_context(|| format!("opening store {}", path.display()))?;
    let out = run_and_record_with(&store, &cfg, &req, source, &SystemClock)?;
    let dsr = dsr_for_run(&store, &cfg, out.run_id, family(&req.strategy))?;
    let m = &out.metrics;
    println!("run {}", out.run_id);
    println!("capital {}", initial_capital.normalize());
    println!("source {}", source.as_str());
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
fn crosscheck(
    from: &str,
    to: &str,
    with_costs: bool,
    capital: Option<Decimal>,
) -> anyhow::Result<i32> {
    let cfg = wiring::load_config()?;
    let initial_capital = match capital {
        Some(c) if c <= Decimal::ZERO => {
            eprintln!("--capital must be greater than zero");
            return Ok(1);
        }
        Some(c) => c,
        None => cfg.backtest.initial_capital,
    };
    let req = BacktestRequest {
        strategy: "tsmom".to_string(),
        from: date("--from", from)?,
        to: date("--to", to)?,
        initial_capital,
    };
    let path = Path::new(&cfg.run.db_path);
    let store = Store::open(path).with_context(|| format!("opening store {}", path.display()))?;
    println!("capital {}", initial_capital.normalize());
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

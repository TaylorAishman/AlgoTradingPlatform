//! `validate` commands.

use std::path::Path;

use anyhow::Context;
use pt_store::db::Store;
use pt_validate::error::ValidateError;
use pt_validate::overfit::{dsr_for_run, pbo_for_runs};

use crate::cli::ValidateCmd;
use crate::wiring;

pub fn validate(action: &ValidateCmd) -> anyhow::Result<i32> {
    match action {
        ValidateCmd::Dsr { run } => dsr(*run),
        ValidateCmd::Pbo { runs } => pbo(runs),
        ValidateCmd::Benchmark { .. } => {
            anyhow::bail!("this `validate` command is not implemented yet")
        }
    }
}

/// Print the Deflated Sharpe Ratio of a stored run, one `label value` per line.
fn dsr(run: i64) -> anyhow::Result<i32> {
    let cfg = wiring::load_config()?;
    let path = Path::new(&cfg.run.db_path);
    let store = Store::open(path).with_context(|| format!("opening store {}", path.display()))?;
    let family = store.backtest_family(run)?;
    let r = dsr_for_run(&store, &cfg, run, &family)?;
    println!("dsr {}", r.dsr);
    println!("sr0 {}", r.sr0);
    println!("skew {}", r.skew);
    println!("kurt {}", r.kurt);
    println!("trials {}", r.n_trials);
    Ok(0)
}

/// Print the Probability of Backtest Overfitting of stored runs, one `label value` per line.
fn pbo(runs: &[i64]) -> anyhow::Result<i32> {
    let cfg = wiring::load_config()?;
    let path = Path::new(&cfg.run.db_path);
    let store = Store::open(path).with_context(|| format!("opening store {}", path.display()))?;
    let s = usize::try_from(cfg.stats.pbo_partitions)?;
    match pbo_for_runs(&store, runs, s) {
        Ok(r) => {
            println!("pbo {}", r.pbo);
            println!("combinations {}", r.n_combinations);
            println!("rows {}", r.rows_used);
            Ok(0)
        }
        Err(ValidateError::TooFewTrials) => {
            eprintln!("PBO is undefined for fewer than 2 trials");
            Ok(1)
        }
        Err(e) => Err(e.into()),
    }
}

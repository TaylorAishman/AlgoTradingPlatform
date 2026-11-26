//! `validate` commands.

use std::path::Path;

use anyhow::Context;
use pt_store::db::Store;
use pt_validate::overfit::dsr_for_run;

use crate::cli::ValidateCmd;
use crate::wiring;

pub fn validate(action: &ValidateCmd) -> anyhow::Result<i32> {
    match action {
        ValidateCmd::Dsr { run } => dsr(*run),
        ValidateCmd::Pbo { .. } | ValidateCmd::Benchmark { .. } => {
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

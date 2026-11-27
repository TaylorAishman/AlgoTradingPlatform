//! The command tree. Parsing only; no command reads or writes anything here.
//!
//! Fields are read by the command modules as they are implemented.
#![allow(dead_code)]

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::cmd_backtest::BacktestCmd;
use crate::cmd_run::RunArgs;

/// Paper trading with an auditable store: backtest, validate, run daily.
#[derive(Debug, Parser)]
#[command(name = "trader", version, about)]
pub struct Cli {
    /// Path to the config file (falls back to PT_CONFIG)
    #[arg(long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Check the config file
    Config {
        #[command(subcommand)]
        action: ConfigCmd,
    },
    /// Show the stage and the owner checklist
    Gates,
    /// Trading calendar commands
    Calendar {
        #[command(subcommand)]
        action: CalendarCmd,
    },
    /// Market data commands
    Data {
        #[command(subcommand)]
        action: DataCmd,
    },
    /// Universe commands
    Universe {
        #[command(subcommand)]
        action: UniverseCmd,
    },
    /// Smoke-test the account and data plan
    Doctor,
    /// Backtest commands
    Backtest {
        #[command(subcommand)]
        action: BacktestCmd,
    },
    /// Overfitting and benchmark statistics
    Validate {
        #[command(subcommand)]
        action: ValidateCmd,
    },
    /// Run every phase that is due; safe to repeat
    Run(RunArgs),
    /// Inspect or change halts
    Halt {
        #[command(subcommand)]
        action: HaltCmd,
    },
    /// Cancel all open orders
    CancelOpen(ConfirmArgs),
    /// Close all positions
    Flatten(ConfirmArgs),
    /// Write the status report
    Report,
    /// Check the heartbeat of the last successful run
    Status {
        /// Fail when the last successful run is older than this
        #[arg(long, value_name = "HOURS")]
        max_age_hours: Option<u64>,
    },
    /// Sign-off report
    Signoff {
        #[command(subcommand)]
        action: SignoffCmd,
    },
}

#[derive(Debug, Args)]
pub struct ConfirmArgs {
    /// Confirm the action
    #[arg(long)]
    pub yes: bool,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCmd {
    /// Validate the config and print its hash
    Check,
}

#[derive(Debug, Subcommand)]
pub enum CalendarCmd {
    /// Import sessions from a CSV file
    Import {
        #[arg(long)]
        csv: PathBuf,
    },
    /// Fetch the calendar from the broker
    Sync,
}

#[derive(Debug, Subcommand)]
pub enum DataCmd {
    /// Import bars from a CSV file
    ImportCsv {
        #[arg(long)]
        csv: PathBuf,
        #[arg(long)]
        adjustment: String,
        #[arg(long)]
        source: String,
    },
    /// Fetch bars from the broker
    Ingest,
    /// Report gaps in the stored bars
    Check,
}

#[derive(Debug, Subcommand)]
pub enum UniverseCmd {
    /// Verify every universe symbol
    Verify,
}

#[derive(Debug, Subcommand)]
pub enum ValidateCmd {
    /// Deflated Sharpe Ratio of a run
    Dsr {
        #[arg(long)]
        run: i64,
    },
    /// Probability of backtest overfitting across runs
    Pbo {
        #[arg(long, value_delimiter = ',')]
        runs: Vec<i64>,
    },
    /// Compare a run with the benchmark run
    Benchmark {
        #[arg(long)]
        run: i64,
        #[arg(long)]
        benchmark_run: i64,
    },
}

#[derive(Debug, Subcommand)]
pub enum HaltCmd {
    /// List active halts
    Status,
    /// Set a manual halt
    Set {
        #[arg(long)]
        reason: String,
    },
    /// Clear a halt
    Clear {
        #[arg(long)]
        id: i64,
        #[arg(long)]
        note: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum SignoffCmd {
    /// Write the sign-off report
    Report,
}

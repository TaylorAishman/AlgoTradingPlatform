//! The `trader` binary: argument parsing and wiring only.

mod cli;
mod cmd_backtest;
mod cmd_config;
mod cmd_data;
mod cmd_ops;
mod cmd_run;
mod cmd_validate;
mod wiring;

use std::process::ExitCode;

use clap::Parser;

use cli::{Cli, Command};

fn dispatch(cli: &Cli) -> anyhow::Result<i32> {
    match &cli.command {
        Command::Config { action } => cmd_config::config(action),
        Command::Gates => cmd_config::gates(),
        Command::Calendar { action } => cmd_config::calendar(action),
        Command::Data { action } => cmd_data::data(action),
        Command::Universe { action } => cmd_config::universe(action),
        Command::Doctor => cmd_run::doctor(),
        Command::Backtest { action } => cmd_backtest::backtest(action),
        Command::Validate { action } => cmd_validate::validate(action),
        Command::Run => cmd_run::run(),
        Command::Halt { action } => cmd_ops::halt(action),
        Command::CancelOpen(args) => cmd_ops::cancel_open(args),
        Command::Flatten(args) => cmd_ops::flatten(args),
        Command::Report => cmd_ops::report(),
        Command::Status { max_age_hours } => cmd_ops::status(*max_age_hours),
        Command::Signoff { action } => cmd_ops::signoff(action),
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match dispatch(&cli) {
        Ok(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::from(1)
        }
    }
}

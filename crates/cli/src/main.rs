//! The `trader` binary: argument parsing and wiring only.

mod cli;
mod cmd_backtest;
mod cmd_config;
mod cmd_data;
mod cmd_kill;
mod cmd_ops;
mod cmd_run;
mod cmd_validate;
mod strategies;
mod wiring;

use std::process::ExitCode;

use clap::Parser;

use cli::{CalendarCmd, Cli, Command, DataCmd, UniverseCmd};
use pt_gate::owner::Stage;

/// Exit code for a command the operating stage forbids.
const EXIT_STAGE_FORBIDS: i32 = 4;

/// The minimum stage of each command, with the name it is reported under.
/// `None` means any stage may run it.
fn minimum_stage(command: &Command) -> Option<(&'static str, Stage)> {
    match command {
        Command::Calendar {
            action: CalendarCmd::Sync,
        } => Some(("calendar sync", Stage::Ingest)),
        Command::Data {
            action: DataCmd::Ingest,
        } => Some(("data ingest", Stage::Ingest)),
        Command::Universe {
            action: UniverseCmd::Verify,
        } => Some(("universe verify", Stage::Ingest)),
        Command::Doctor => Some(("doctor", Stage::Ingest)),
        Command::Run => Some(("run", Stage::Ingest)),
        Command::CancelOpen(_) => Some(("cancel-open", Stage::FirstOrder)),
        Command::Flatten(_) => Some(("flatten", Stage::Paper)),
        _ => None,
    }
}

/// Check the minimum stage, then run the command. The check comes before any
/// command module runs, so a refused command loads no key and makes no request.
fn dispatch(cli: &Cli) -> anyhow::Result<i32> {
    if let Some((name, required)) = minimum_stage(&cli.command) {
        if let Some(message) = wiring::stage_refusal(name, required)? {
            wiring::log("error", &message);
            return Ok(EXIT_STAGE_FORBIDS);
        }
        // Every command with a minimum stage talks to the network. The client is
        // not used by the stub command modules yet.
        let _client = wiring::build_client()?;
    }
    match &cli.command {
        Command::Config { action } => cmd_config::config(action),
        Command::Gates => cmd_config::gates(),
        Command::Calendar { action } => cmd_config::calendar(action),
        Command::Data { action } => cmd_data::data(action),
        Command::Universe { action } => cmd_config::universe(action),
        Command::Doctor => cmd_data::doctor(),
        Command::Backtest { action } => cmd_backtest::backtest(action),
        Command::Validate { action } => cmd_validate::validate(action),
        Command::Run => cmd_run::run(),
        Command::Halt { action } => cmd_ops::halt(action),
        Command::CancelOpen(args) => cmd_kill::cancel_open(args),
        Command::Flatten(args) => cmd_kill::flatten(args),
        Command::Report => cmd_ops::report(),
        Command::Status { max_age_hours } => cmd_ops::status(*max_age_hours),
        Command::Signoff { action } => cmd_ops::signoff(action),
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    wiring::set_config_flag(cli.config.clone());
    match dispatch(&cli) {
        Ok(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
        Err(err) => {
            wiring::log("error", &format!("{err:#}"));
            ExitCode::from(1)
        }
    }
}

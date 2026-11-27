//! `cancel-open` and `flatten` commands.

use anyhow::Context;
use pt_core::clock::SystemClock;
use pt_core::types::Symbol;
use pt_gate::gate::Gate;
use pt_live::context::LiveContext;
use pt_live::error::LiveError;
use pt_ops::alert::Alerter;
use pt_strategy::tsmom::TsmomParams;

use crate::cli::ConfirmArgs;
use crate::strategies::StoredTsmom;
use crate::wiring;

/// Cancel every open order at the broker and print how many cancels were sent.
/// Exits 1 without `--yes`, before any client is built.
pub fn cancel_open(args: &ConfirmArgs) -> anyhow::Result<i32> {
    kill_switch(args, "cancel-open", pt_live::killswitch::cancel_open)
}

/// Close every position and halt, printing how many orders were sent.
/// Exits 1 without `--yes`, before any client is built. Works while halted.
pub fn flatten(args: &ConfirmArgs) -> anyhow::Result<i32> {
    kill_switch(args, "flatten", |ctx| {
        pt_live::killswitch::flatten(ctx).map(|r| r.accepted)
    })
}

/// Require `--yes`, build the live context as `run` does, call `action` and
/// print its count.
fn kill_switch(
    args: &ConfirmArgs,
    name: &str,
    action: fn(&LiveContext) -> Result<usize, LiveError>,
) -> anyhow::Result<i32> {
    if !args.yes {
        wiring::log("error", &format!("`{name}` needs --yes to confirm"));
        return Ok(1);
    }
    let cfg = wiring::load_config()?;
    let owner = wiring::load_owner()?;
    let keys = wiring::load_keys()?;
    let store = wiring::open_store(&cfg.run.db_path)?;
    let http = wiring::build_client()?;
    let alerter = Alerter::new(
        wiring::alert_webhook(owner.alert_webhook_url.clone()),
        vec![
            keys.key_id.expose().to_string(),
            keys.secret_key.expose().to_string(),
        ],
    );
    let gate = Gate::new(owner);
    // The kill switch only needs the strategy's id; the calendar is unused.
    let params = TsmomParams {
        lookback_months: cfg.strategy.lookback_months.clone(),
        risk_free_symbol: Symbol::new(&cfg.strategy.risk_free_symbol)
            .context("strategy.risk_free_symbol")?,
    };
    let strategy = StoredTsmom::new(&store, params, cfg.data.start)?;
    let clock = SystemClock;
    let ctx = LiveContext {
        store: &store,
        cfg: &cfg,
        gate: &gate,
        http: &http,
        clock: &clock,
        alerter: &alerter,
        strategy: &strategy,
        code_version: env!("CARGO_PKG_VERSION"),
    };
    let n = action(&ctx)?;
    println!("{n}");
    Ok(0)
}

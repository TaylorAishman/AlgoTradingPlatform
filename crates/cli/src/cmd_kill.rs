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

/// Lines to print and the exit code.
type KillOutcome = Result<(Vec<String>, i32), LiveError>;

/// Cancel every open order at the broker and print how many cancels were sent.
/// Exits 1 without `--yes`, before any client is built.
pub fn cancel_open(args: &ConfirmArgs) -> anyhow::Result<i32> {
    kill_switch(args, "cancel-open", |ctx| {
        let n = pt_live::killswitch::cancel_open(ctx)?;
        Ok((vec![n.to_string()], 0))
    })
}

/// Close every position and halt, printing the cancelled, accepted and rejected counts.
/// Exits 1 without `--yes`, before any client is built, and when the broker rejected
/// any order. Works while halted.
pub fn flatten(args: &ConfirmArgs) -> anyhow::Result<i32> {
    kill_switch(args, "flatten", |ctx| {
        let r = pt_live::killswitch::flatten(ctx)?;
        let lines = vec![
            format!("cancelled {}", r.cancelled),
            format!("accepted {}", r.accepted),
            format!("rejected {}", r.rejected),
        ];
        Ok((lines, i32::from(r.rejected > 0)))
    })
}

/// Require `--yes`, then the keys, build the live context as `run` does, call `action` and
/// print its lines and return its exit code.
fn kill_switch(
    args: &ConfirmArgs,
    name: &str,
    action: fn(&LiveContext) -> KillOutcome,
) -> anyhow::Result<i32> {
    if !args.yes {
        wiring::log("error", &format!("`{name}` needs --yes to confirm"));
        return Ok(1);
    }
    // Keys come first: a missing key is reported before any config problem.
    let keys = wiring::load_keys()?;
    let cfg = wiring::load_config()?;
    let owner = wiring::load_owner()?;
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
    let (lines, code) = action(&ctx)?;
    for line in lines {
        println!("{line}");
    }
    Ok(code)
}

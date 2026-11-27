//! The one place a halt is set: it records the halt and raises a critical `halt_set` alert.

use pt_ops::alert::Level;

use crate::context::LiveContext;
use crate::error::LiveError;

/// Sets a halt with `reason` and records a critical `halt_set` alert, unless a
/// halt with this reason is already active. Returns whether a halt was set.
pub fn set_halt(ctx: &LiveContext<'_>, reason: &str, detail: &str) -> Result<bool, LiveError> {
    if ctx.store.active_halts()?.iter().any(|h| h.reason == reason) {
        return Ok(false);
    }
    let now = ctx.clock.now_utc();
    ctx.store.set_halt(reason, detail, now)?;
    ctx.alerter.alert(
        ctx.store,
        Level::Critical,
        "halt_set",
        &format!("halt {reason}: {detail}"),
        None,
        now,
    );
    Ok(true)
}

//! Sign-off evaluation: the seven fidelity criteria of R-341.

use pt_config::config::Config;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

use crate::fidelity::Fidelity;

/// One judged criterion: the measured value against its limit.
#[derive(Debug, Clone, PartialEq)]
pub struct Criterion {
    pub id: &'static str,
    pub value: f64,
    pub limit: f64,
    pub pass: bool,
}

/// The sign-off outcome: not ready until the sample is large enough.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Fail,
    NotReady,
}

/// The criteria judged for one fidelity summary, and the verdict.
#[derive(Debug, Clone, PartialEq)]
pub struct SignoffReport {
    pub verdict: Verdict,
    pub criteria: Vec<Criterion>,
}

fn bps(d: Decimal) -> f64 {
    d.to_f64().unwrap_or(f64::NAN)
}

fn count(id: &'static str, value: usize, limit: u32) -> Criterion {
    Criterion {
        id,
        value: value as f64,
        limit: f64::from(limit),
        pass: value <= limit as usize,
    }
}

fn measure(id: &'static str, value: f64, limit: f64, judged: f64) -> Criterion {
    Criterion {
        id,
        value,
        limit,
        pass: judged <= limit,
    }
}

/// Judge `f` against the thresholds in `cfg`. A sample below any configured
/// minimum is `NotReady`, whatever the criteria say.
pub fn evaluate(f: &Fidelity, cfg: &Config) -> SignoffReport {
    let s = &cfg.signoff;
    let slippage_limit = bps(cfg.cost.shadow.spread_bps + s.slippage_tolerance_bps);
    let criteria = vec![
        count(
            "replay_mismatches",
            f.replay_mismatches,
            s.max_replay_mismatches,
        ),
        count(
            "reconciliation_breaks",
            f.reconciliation_breaks,
            s.max_reconciliation_breaks,
        ),
        count("missed_sessions", f.missed_sessions, s.max_missed_sessions),
        count("late_rebalances", f.late_rebalances, s.max_late_rebalances),
        measure(
            "slippage_vs_model",
            f.mean_slippage_bps,
            slippage_limit,
            f.mean_slippage_bps,
        ),
        measure(
            "mean_diff",
            f.mean_diff_bps,
            bps(s.max_mean_diff_bps),
            f.mean_diff_bps.abs(),
        ),
        measure(
            "max_abs_diff",
            f.max_abs_diff_bps,
            bps(s.max_abs_diff_bps),
            f.max_abs_diff_bps,
        ),
    ];
    let small = f.n_sessions < s.min_sessions as usize
        || f.n_rebalances < s.min_rebalances as usize
        || f.n_fills < s.min_fills as usize;
    let verdict = if small {
        Verdict::NotReady
    } else if criteria.iter().all(|c| c.pass) {
        Verdict::Pass
    } else {
        Verdict::Fail
    };
    SignoffReport { verdict, criteria }
}

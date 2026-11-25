//! R-341: sign-off is judged on the seven fidelity criteria only.

mod common;

use pt_config::config::{parse, Config};
use pt_shadow::fidelity::Fidelity;
use pt_shadow::signoff::evaluate;

const IDS: [&str; 7] = [
    "replay_mismatches",
    "reconciliation_breaks",
    "missed_sessions",
    "late_rebalances",
    "slippage_vs_model",
    "mean_diff",
    "max_abs_diff",
];

/// The smallest config `parse` accepts: validation requires a non-empty
/// universe. Every other setting keeps its default.
const UNIVERSE: &str =
    "[[universe]]\nsymbol = \"A\"\nasset_class = \"equity\"\nshortable = true\n\n";

fn default_cfg() -> Config {
    parse(UNIVERSE).expect("default config")
}

/// A fidelity that passes every criterion under the default thresholds.
fn good() -> Fidelity {
    Fidelity {
        n_sessions: 63,
        n_rebalances: 3,
        n_fills: 30,
        replay_mismatches: 0,
        reconciliation_breaks: 0,
        missed_sessions: 2,
        late_rebalances: 0,
        mean_slippage_bps: 9.0,
        mean_diff_bps: 0.8,
        max_abs_diff_bps: 20.0,
    }
}

/// Pass flag of the criterion `id` after evaluating `f` under `cfg`.
fn passes(f: &Fidelity, cfg: &Config, id: &str) -> bool {
    evaluate(f, cfg)
        .criteria
        .iter()
        .find(|c| c.id == id)
        .unwrap_or_else(|| panic!("criterion {id} missing"))
        .pass
}

#[test]
fn criteria_are_exactly_the_seven_fidelity_ids_in_order() {
    // R-341
    let report = evaluate(&good(), &default_cfg());
    let ids: Vec<&str> = report.criteria.iter().map(|c| c.id).collect();
    assert_eq!(ids, IDS);
}

#[test]
fn no_criterion_id_mentions_return_profit_or_sharpe() {
    // R-341
    for c in evaluate(&good(), &default_cfg()).criteria {
        let id = c.id.to_lowercase();
        for word in ["return", "profit", "sharpe"] {
            assert!(!id.contains(word), "{id} mentions {word}");
        }
    }
}

#[test]
fn a_clean_fidelity_passes_every_criterion() {
    // R-341
    let report = evaluate(&good(), &default_cfg());
    for c in &report.criteria {
        assert!(c.pass, "{} should pass", c.id);
    }
}

#[test]
fn criteria_do_not_depend_on_sample_size() {
    // R-341: sample size is the verdict's concern, not a criterion.
    let mut f = good();
    f.n_sessions = 1;
    f.n_rebalances = 0;
    f.n_fills = 0;
    let report = evaluate(&f, &default_cfg());
    assert_eq!(report.criteria.len(), 7);
    assert!(report.criteria.iter().all(|c| c.pass));
}

#[test]
fn replay_mismatches_fail_above_zero() {
    // R-341
    let cfg = default_cfg();
    assert!(passes(&good(), &cfg, "replay_mismatches"));
    let mut f = good();
    f.replay_mismatches = 1;
    assert!(!passes(&f, &cfg, "replay_mismatches"));
}

#[test]
fn reconciliation_breaks_fail_above_zero() {
    // R-341
    let cfg = default_cfg();
    assert!(passes(&good(), &cfg, "reconciliation_breaks"));
    let mut f = good();
    f.reconciliation_breaks = 1;
    assert!(!passes(&f, &cfg, "reconciliation_breaks"));
}

#[test]
fn missed_sessions_pass_at_two_and_fail_at_three() {
    // R-341
    let cfg = default_cfg();
    let mut f = good();
    f.missed_sessions = 2;
    assert!(passes(&f, &cfg, "missed_sessions"));
    f.missed_sessions = 3;
    assert!(!passes(&f, &cfg, "missed_sessions"));
}

#[test]
fn late_rebalances_fail_above_zero() {
    // R-341
    let cfg = default_cfg();
    assert!(passes(&good(), &cfg, "late_rebalances"));
    let mut f = good();
    f.late_rebalances = 1;
    assert!(!passes(&f, &cfg, "late_rebalances"));
}

#[test]
fn slippage_is_judged_against_model_plus_tolerance() {
    // R-341: model 5 bps, tolerance 5 bps: 9 passes, 10.5 fails.
    let cfg = default_cfg();
    let mut f = good();
    f.mean_slippage_bps = 9.0;
    assert!(passes(&f, &cfg, "slippage_vs_model"));
    f.mean_slippage_bps = 10.5;
    assert!(!passes(&f, &cfg, "slippage_vs_model"));
}

#[test]
fn slippage_limit_follows_the_configured_model() {
    // R-341: the shadow fixture models a 10 bps spread, so the limit is 15.
    let cfg = common::config();
    let mut f = good();
    f.mean_slippage_bps = 14.0;
    assert!(passes(&f, &cfg, "slippage_vs_model"));
    f.mean_slippage_bps = 16.0;
    assert!(!passes(&f, &cfg, "slippage_vs_model"));
}

#[test]
fn mean_diff_fails_beyond_one_bp_in_either_direction() {
    // R-341
    let cfg = default_cfg();
    let mut f = good();
    f.mean_diff_bps = 0.8;
    assert!(passes(&f, &cfg, "mean_diff"));
    f.mean_diff_bps = -0.8;
    assert!(passes(&f, &cfg, "mean_diff"));
    f.mean_diff_bps = -1.2;
    assert!(!passes(&f, &cfg, "mean_diff"));
    f.mean_diff_bps = 1.2;
    assert!(!passes(&f, &cfg, "mean_diff"));
}

#[test]
fn max_abs_diff_passes_at_twenty_and_fails_at_twenty_six() {
    // R-341
    let cfg = default_cfg();
    let mut f = good();
    f.max_abs_diff_bps = 20.0;
    assert!(passes(&f, &cfg, "max_abs_diff"));
    f.max_abs_diff_bps = 26.0;
    assert!(!passes(&f, &cfg, "max_abs_diff"));
}

#[test]
fn one_failing_value_fails_only_its_own_criterion() {
    // R-341
    let cfg = default_cfg();
    let mut f = good();
    f.max_abs_diff_bps = 26.0;
    let report = evaluate(&f, &cfg);
    for c in &report.criteria {
        assert_eq!(c.pass, c.id != "max_abs_diff", "{}", c.id);
    }
}

#[test]
fn criteria_report_value_and_limit() {
    // R-341
    let report = evaluate(&good(), &default_cfg());
    let find = |id: &str| report.criteria.iter().find(|c| c.id == id).unwrap();
    assert_eq!(find("missed_sessions").value, 2.0);
    assert_eq!(find("missed_sessions").limit, 2.0);
    assert_eq!(find("replay_mismatches").limit, 0.0);
    assert_eq!(find("max_abs_diff").value, 20.0);
    assert_eq!(find("max_abs_diff").limit, 25.0);
    assert_eq!(find("mean_diff").limit, 1.0);
    assert_eq!(find("slippage_vs_model").value, 9.0);
    assert_eq!(find("slippage_vs_model").limit, 10.0);
}

#[test]
fn thresholds_come_from_the_configuration() {
    // R-341
    let cfg = parse(&format!(
        "{UNIVERSE}[signoff]\nmax_missed_sessions = 5\nmax_abs_diff_bps = 40\n"
    ))
    .unwrap();
    let mut f = good();
    f.missed_sessions = 5;
    f.max_abs_diff_bps = 30.0;
    assert!(passes(&f, &cfg, "missed_sessions"));
    assert!(passes(&f, &cfg, "max_abs_diff"));
    f.missed_sessions = 6;
    f.max_abs_diff_bps = 41.0;
    assert!(!passes(&f, &cfg, "missed_sessions"));
    assert!(!passes(&f, &cfg, "max_abs_diff"));
}

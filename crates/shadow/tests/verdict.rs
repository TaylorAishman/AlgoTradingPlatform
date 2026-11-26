//! R-342: the verdict is not-ready until the sample is large enough, then
//! pass only if every criterion passes.

use pt_config::config::{parse, Config};
use pt_shadow::fidelity::Fidelity;
use pt_shadow::signoff::{evaluate, Verdict};

/// The smallest config `parse` accepts: validation requires a non-empty
/// universe. Every other setting keeps its default.
const UNIVERSE: &str =
    "[[universe]]\nsymbol = \"A\"\nasset_class = \"equity\"\nshortable = true\n\n";

fn default_cfg() -> Config {
    parse(UNIVERSE).expect("default config")
}

/// A fidelity that is large enough and passes every criterion.
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

/// Assert the verdict is `Fail` and only criterion `id` has `pass == false`.
fn assert_fails_only(f: &Fidelity, id: &str) {
    let report = evaluate(f, &default_cfg());
    assert_eq!(report.verdict, Verdict::Fail, "{id}");
    for c in &report.criteria {
        assert_eq!(c.pass, c.id != id, "{} (changed {id})", c.id);
    }
}

#[test]
fn a_large_clean_sample_passes() {
    // R-342
    let report = evaluate(&good(), &default_cfg());
    assert_eq!(report.verdict, Verdict::Pass);
    assert!(report.criteria.iter().all(|c| c.pass));
}

#[test]
fn sixty_two_sessions_is_not_ready() {
    // R-342
    let mut f = good();
    f.n_sessions = 62;
    assert_eq!(evaluate(&f, &default_cfg()).verdict, Verdict::NotReady);
}

#[test]
fn two_rebalances_is_not_ready() {
    // R-342
    let mut f = good();
    f.n_rebalances = 2;
    assert_eq!(evaluate(&f, &default_cfg()).verdict, Verdict::NotReady);
}

#[test]
fn twenty_nine_fills_is_not_ready() {
    // R-342
    let mut f = good();
    f.n_fills = 29;
    assert_eq!(evaluate(&f, &default_cfg()).verdict, Verdict::NotReady);
}

#[test]
fn a_small_sample_is_not_ready_even_when_a_criterion_fails() {
    // R-342: not-ready comes before pass or fail.
    let mut f = good();
    f.n_sessions = 10;
    f.replay_mismatches = 1;
    assert_eq!(evaluate(&f, &default_cfg()).verdict, Verdict::NotReady);
}

#[test]
fn sample_minimums_come_from_the_configuration() {
    // R-342
    let cfg = parse(&format!("{UNIVERSE}[signoff]\nmin_sessions = 70\n")).unwrap();
    assert_eq!(evaluate(&good(), &cfg).verdict, Verdict::NotReady);
    let mut f = good();
    f.n_sessions = 70;
    assert_eq!(evaluate(&f, &cfg).verdict, Verdict::Pass);
}

#[test]
fn one_replay_mismatch_fails() {
    // R-342
    let mut f = good();
    f.replay_mismatches = 1;
    assert_fails_only(&f, "replay_mismatches");
}

#[test]
fn one_reconciliation_break_fails() {
    // R-342
    let mut f = good();
    f.reconciliation_breaks = 1;
    assert_fails_only(&f, "reconciliation_breaks");
}

#[test]
fn three_missed_sessions_fail() {
    // R-342
    let mut f = good();
    f.missed_sessions = 3;
    assert_fails_only(&f, "missed_sessions");
}

#[test]
fn one_late_rebalance_fails() {
    // R-342
    let mut f = good();
    f.late_rebalances = 1;
    assert_fails_only(&f, "late_rebalances");
}

#[test]
fn slippage_of_ten_and_a_half_bps_fails() {
    // R-342
    let mut f = good();
    f.mean_slippage_bps = 10.5;
    assert_fails_only(&f, "slippage_vs_model");
}

#[test]
fn a_mean_difference_of_minus_one_point_two_bps_fails() {
    // R-342
    let mut f = good();
    f.mean_diff_bps = -1.2;
    assert_fails_only(&f, "mean_diff");
}

#[test]
fn a_maximum_difference_of_twenty_six_bps_fails() {
    // R-342
    let mut f = good();
    f.max_abs_diff_bps = 26.0;
    assert_fails_only(&f, "max_abs_diff");
}

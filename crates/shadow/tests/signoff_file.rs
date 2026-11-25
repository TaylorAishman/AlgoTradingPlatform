//! R-343: the sign-off report is a local file stating the verdict, each
//! criterion and the data feed.

use std::fs;

use pt_shadow::signoff::{write_signoff, Criterion, SignoffReport, Verdict};

const SENTENCE: &str = "Paper trading validates the system, not the strategy.";

fn criterion(id: &'static str, value: f64, limit: f64, pass: bool) -> Criterion {
    Criterion {
        id,
        value,
        limit,
        pass,
    }
}

fn report(verdict: Verdict, feed: &str) -> SignoffReport {
    SignoffReport {
        verdict,
        criteria: vec![
            criterion("replay_mismatches", 0.0, 0.0, true),
            criterion("slippage_vs_model", 12.5, 20.5, true),
            criterion("max_abs_diff", 55.5, 40.5, false),
        ],
        feed: feed.to_string(),
    }
}

fn written(r: &SignoffReport) -> String {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = write_signoff(r, dir.path()).expect("write_signoff");
    fs::read_to_string(path).expect("read signoff")
}

#[test]
fn creates_signoff_md_in_the_given_directory_and_returns_its_path() {
    // R-343
    let dir = tempfile::tempdir().expect("temp dir");
    let path = write_signoff(&report(Verdict::Fail, "sip"), dir.path()).expect("write_signoff");
    assert_eq!(path, dir.path().join("signoff.md"));
    assert!(path.is_file());
}

#[test]
fn states_the_verdict_word() {
    // R-343
    let pass = SignoffReport {
        verdict: Verdict::Pass,
        criteria: vec![criterion("replay_mismatches", 0.0, 0.0, true)],
        feed: "sip".into(),
    };
    let text = written(&pass);
    assert!(text.contains("PASS"), "{text}");
    assert!(!text.contains("FAIL"), "{text}");

    let text = written(&report(Verdict::Fail, "sip"));
    assert!(text.contains("FAIL"), "{text}");

    let text = written(&report(Verdict::NotReady, "sip")).to_lowercase();
    assert!(
        text.contains("not ready") || text.contains("not_ready"),
        "{text}"
    );
}

#[test]
fn has_one_row_per_criterion_with_value_limit_and_result() {
    // R-343
    let text = written(&report(Verdict::Fail, "sip"));
    let row = |id: &str| -> String {
        let rows: Vec<&str> = text.lines().filter(|l| l.contains(id)).collect();
        assert_eq!(rows.len(), 1, "one row for {id}: {text}");
        rows[0].to_string()
    };

    let slip = row("slippage_vs_model");
    assert!(slip.contains("12.5") && slip.contains("20.5"), "{slip}");
    assert!(slip.contains("PASS") && !slip.contains("FAIL"), "{slip}");

    let diff = row("max_abs_diff");
    assert!(diff.contains("55.5") && diff.contains("40.5"), "{diff}");
    assert!(diff.contains("FAIL") && !diff.contains("PASS"), "{diff}");

    let replay = row("replay_mismatches");
    assert!(replay.contains("PASS"), "{replay}");
}

#[test]
fn names_the_data_feed() {
    // R-343
    assert!(written(&report(Verdict::Pass, "iex")).contains("iex"));
    assert!(written(&report(Verdict::Pass, "csv:vendor")).contains("csv:vendor"));
}

#[test]
fn carries_the_system_not_strategy_sentence() {
    // R-343
    assert!(written(&report(Verdict::Pass, "sip")).contains(SENTENCE));
}

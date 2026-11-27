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

fn cells(line: &str) -> Vec<String> {
    line.trim()
        .trim_start_matches('|')
        .trim_end_matches('|')
        .split('|')
        .map(|c| c.trim().to_string())
        .collect()
}

#[test]
fn non_empty_lines_have_the_fixed_layout_in_order() {
    // R-521
    let r = SignoffReport {
        verdict: Verdict::NotReady,
        criteria: vec![
            criterion("replay_mismatches", 0.0, 0.0, true),
            criterion("mean_diff", 1.2, 1.0, false),
        ],
        feed: "iex".to_string(),
    };
    let text = written(&r);
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 8, "{text}");
    assert_eq!(lines[0], "# Paper-trading sign-off");
    assert_eq!(lines[1], "Verdict: NOT READY");
    assert_eq!(lines[2], "Data feed: iex");
    assert_eq!(
        cells(lines[3]),
        ["Criterion", "Value", "Limit", "Result"],
        "{text}"
    );
    let sep = cells(lines[4]);
    assert_eq!(sep.len(), 4, "{text}");
    assert!(
        sep.iter()
            .all(|c| !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':')),
        "{text}"
    );
    assert_eq!(cells(lines[5]), ["replay_mismatches", "0", "0", "PASS"]);
    assert_eq!(cells(lines[6]), ["mean_diff", "1.2", "1", "FAIL"]);
    assert_eq!(lines[7], SENTENCE);
}

#[test]
fn verdict_line_uses_pass_and_fail_words() {
    // R-521
    let text = written(&report(Verdict::Pass, "sip"));
    assert!(text.lines().any(|l| l == "Verdict: PASS"), "{text}");
    let text = written(&report(Verdict::Fail, "sip"));
    assert!(text.lines().any(|l| l == "Verdict: FAIL"), "{text}");
}

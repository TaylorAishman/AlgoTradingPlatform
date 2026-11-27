//! R-532: `flatten` prints how many orders it cancelled, how many the broker
//! accepted and how many it rejected, and fails when any was rejected.

mod common;

use common::live::{env, posts, run, Env};
use common::{stderr, stdout};

fn ingest_stage(e: &Env) {
    std::fs::write(&e.owner, "stage = \"ingest\"\n").unwrap();
}

fn lines(out: &std::process::Output) -> Vec<String> {
    stdout(out).lines().map(|l| l.trim().to_string()).collect()
}

#[test]
fn flatten_prints_three_counts_and_halts_as_flattened() {
    // R-532
    let e = env();
    ingest_stage(&e);
    e.mock.seed_position("SPY", 10);
    e.mock.seed_position("TLT", -5);
    let out = run(&e, &["flatten", "--yes"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stdout: {} stderr: {}",
        stdout(&out),
        stderr(&out)
    );
    assert_eq!(lines(&out), ["cancelled 0", "accepted 2", "rejected 0"]);
    assert_eq!(posts(&e).len(), 2, "POSTs: {:?}", posts(&e));
    let halt = run(&e, &["halt", "status"]);
    assert_eq!(halt.status.code(), Some(2), "{}", stdout(&halt));
    assert!(stdout(&halt).contains("flattened"), "{}", stdout(&halt));
}

#[test]
fn flatten_with_one_rejected_order_prints_both_counts_and_exits_one() {
    // R-532
    let e = env();
    ingest_stage(&e);
    e.mock.seed_position("SPY", 10);
    e.mock.seed_position("TLT", -5);
    e.mock.reject_next("POST", "/v2/orders", 1, 422);
    let out = run(&e, &["flatten", "--yes"]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "stdout: {} stderr: {}",
        stdout(&out),
        stderr(&out)
    );
    let l = lines(&out);
    assert!(l.contains(&"accepted 1".to_string()), "{l:?}");
    assert!(l.contains(&"rejected 1".to_string()), "{l:?}");
}

//! R-536 and R-537: `trader run --if-due` does nothing and succeeds when the
//! run is not due, and runs when there has been no earlier run.

mod common;

use common::live::{env, posts, run};
use common::{stderr, stdout};

#[test]
fn if_due_straight_after_a_run_does_nothing_and_succeeds() {
    // R-536
    let e = env();
    let first = run(&e, &["run"]);
    assert_eq!(first.status.code(), Some(0), "stderr: {}", stderr(&first));
    let status_path = e.dir.path().join("reports").join("status.json");
    let status_before = std::fs::read(&status_path).expect("status.json after the first run");
    let requests_before = e.mock.requests().len();

    let out = run(&e, &["run", "--if-due"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stdout: {} stderr: {}",
        stdout(&out),
        stderr(&out)
    );
    assert_eq!(
        e.mock.requests().len(),
        requests_before,
        "the mock recorded a further request"
    );
    let store = pt_store::db::Store::open(&e.dir.path().join("test.sqlite")).unwrap();
    assert_eq!(store.runs().unwrap().len(), 1);
    assert_eq!(
        std::fs::read(&status_path).unwrap(),
        status_before,
        "status.json changed"
    );

    let err = stderr(&out);
    let mut saw_not_due = false;
    for line in err.lines().filter(|l| !l.trim().is_empty()) {
        let v: serde_json::Value =
            serde_json::from_str(line).unwrap_or_else(|e| panic!("not JSON ({e}): {line}"));
        for key in ["level", "message", "ts"] {
            assert!(
                v.get(key).and_then(|x| x.as_str()).is_some(),
                "missing string `{key}` in: {line}"
            );
        }
        if v["message"].as_str().unwrap().contains("not due") {
            saw_not_due = true;
        }
    }
    assert!(saw_not_due, "no log message contains `not due`: {err}");
}

#[test]
fn if_due_with_no_earlier_run_runs() {
    // R-537
    let e = env();
    let out = run(&e, &["run", "--if-due"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stdout: {} stderr: {}",
        stdout(&out),
        stderr(&out)
    );
    assert_eq!(posts(&e).len(), 2, "POSTs: {:?}", posts(&e));
    let store = pt_store::db::Store::open(&e.dir.path().join("test.sqlite")).unwrap();
    let runs = store.runs().unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, "ok");
}

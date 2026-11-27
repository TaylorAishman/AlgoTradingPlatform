//! R-394 (report) and R-401 (logging): after the cycle `trader run` writes
//! `status.json` and `status.md`, whether the cycle ended ok or failed, and
//! every stderr line of the run is a JSON log line free of the secret.

mod common;

use common::live::{env, run, Env};
use common::{stderr, stdout};

fn status_json(e: &Env) -> serde_json::Value {
    let path = e.dir.path().join("reports").join("status.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("reading {}: {err}", path.display()));
    serde_json::from_str(&text).expect("status.json is JSON")
}

#[test]
fn run_writes_status_json_with_stage_paper_and_status_md() {
    // R-394
    let e = env();
    let out = run(&e, &["run"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert_eq!(status_json(&e)["stage"], "paper");
    assert!(e.dir.path().join("reports").join("status.md").exists());
}

#[test]
fn the_report_still_exists_when_the_broker_fails_every_request() {
    // R-394: exit 1 and the report is still written.
    let e = env();
    e.mock.fail_next(1000, 401);
    let out = run(&e, &["run"]);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    assert_eq!(status_json(&e)["stage"], "paper");
}

#[test]
fn every_stderr_line_of_a_run_is_json_and_hides_the_secret() {
    // R-401
    let e = env();
    let out = run(&e, &["run"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let err = stderr(&out);
    for line in err.lines().filter(|l| !l.trim().is_empty()) {
        let v: serde_json::Value =
            serde_json::from_str(line).unwrap_or_else(|e| panic!("not JSON ({e}): {line}"));
        for key in ["level", "message", "ts"] {
            assert!(
                v.get(key).and_then(|x| x.as_str()).is_some(),
                "missing string `{key}` in: {line}"
            );
        }
    }
    assert!(!err.contains("fake-secret"));
    assert!(!stdout(&out).contains("fake-secret"));
}

#[test]
fn an_active_halt_exits_two_and_the_report_lists_it() {
    // R-394: with an active halt it exits 2 and the report lists the halt.
    let e = env();
    {
        let store = pt_store::db::Store::open(&e.dir.path().join("test.sqlite")).unwrap();
        store
            .set_halt("manual", "holiday", chrono::Utc::now())
            .unwrap();
    }
    let out = run(&e, &["run"]);
    assert_eq!(
        out.status.code(),
        Some(2),
        "stdout: {} stderr: {}",
        stdout(&out),
        stderr(&out)
    );
    let status = status_json(&e);
    let halts = status["active_halts"]
        .as_array()
        .expect("active_halts array");
    assert_eq!(halts.len(), 1, "status.json: {status}");
    assert_eq!(halts[0]["reason"], "manual");
    assert_eq!(halts[0]["detail"], "holiday");
}

fn generated_at(e: &Env) -> chrono::DateTime<chrono::Utc> {
    status_json(e)["generated_at"]
        .as_str()
        .expect("generated_at is a string")
        .parse()
        .expect("generated_at is a timestamp")
}

#[test]
fn the_report_is_rewritten_when_the_run_log_cannot_be_written() {
    // R-535: the database is read-only, so the cycle fails; the report is
    // still written, and the run exits 1.
    let e = env();
    let first = run(&e, &["run"]);
    assert_eq!(first.status.code(), Some(0), "stderr: {}", stderr(&first));
    let before = generated_at(&e);

    let db = e.dir.path().join("test.sqlite");
    let mut perms = std::fs::metadata(&db).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&db, perms).unwrap();

    let out = run(&e, &["run"]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "stdout: {} stderr: {}",
        stdout(&out),
        stderr(&out)
    );
    let after = generated_at(&e);
    assert!(
        after > before,
        "generated_at did not advance: {before} -> {after}"
    );
}

//! Tests that a report replaces the previous one atomically (R-359).

use chrono::{DateTime, Duration, TimeZone, Utc};
use pt_ops::report::{write_status, ReportInputs};
use pt_store::db::Store;
use serde_json::Value;

fn first() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, 2, 22, 0, 0).unwrap()
}

fn config(dir: &std::path::Path) -> pt_config::config::Config {
    pt_config::config::parse(&format!(
        "[run]\noutput_dir = {:?}\n\n[[universe]]\nsymbol = \"SPY\"\nasset_class = \"equity\"\nshortable = true\n",
        dir.to_str().unwrap()
    ))
    .unwrap()
}

fn write(store: &Store, cfg: &pt_config::config::Config, now: DateTime<Utc>) {
    let inp = ReportInputs {
        store,
        cfg,
        stage: "paper",
        fidelity: &[],
        validation_signoff: None,
        now,
    };
    write_status(&inp).unwrap();
}

fn generated_at(path: &std::path::Path) -> DateTime<Utc> {
    let v: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    v["generated_at"].as_str().unwrap().parse().unwrap()
}

// R-359: after two calls the directory holds exactly status.md and status.json,
// and both hold the second call's generated_at.
#[test]
fn two_writes_leave_exactly_two_files_with_the_second_time() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let store = Store::open_in_memory().unwrap();
    let second = first() + Duration::hours(1);
    write(&store, &cfg, first());
    write(&store, &cfg, second);

    let mut names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, vec!["status.json", "status.md"]);

    assert_eq!(generated_at(&dir.path().join("status.json")), second);
    let md = std::fs::read_to_string(dir.path().join("status.md")).unwrap();
    assert!(
        md.contains(&second.to_rfc3339()),
        "md lacks second time: {md}"
    );
    assert!(
        !md.contains(&first().to_rfc3339()),
        "md has first time: {md}"
    );
}

// R-359: a reader that already has the old file open still sees the complete old
// report, because the new one is written aside and renamed over it.
#[cfg(unix)]
#[test]
fn replacing_does_not_modify_the_previous_file_in_place() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let store = Store::open_in_memory().unwrap();
    write(&store, &cfg, first());

    for name in ["status.json", "status.md"] {
        let live = dir.path().join(name);
        let held = dir.path().join(format!("held-{name}"));
        std::fs::hard_link(&live, &held).unwrap();
    }
    let old_json = std::fs::read_to_string(dir.path().join("held-status.json")).unwrap();
    let old_md = std::fs::read_to_string(dir.path().join("held-status.md")).unwrap();

    write(&store, &cfg, first() + Duration::hours(1));

    assert_eq!(
        std::fs::read_to_string(dir.path().join("held-status.json")).unwrap(),
        old_json,
        "status.json was rewritten in place"
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("held-status.md")).unwrap(),
        old_md,
        "status.md was rewritten in place"
    );
    assert_ne!(
        std::fs::read_to_string(dir.path().join("status.json")).unwrap(),
        old_json
    );
}

//! R-551: the kill switch works end to end in the ingest stage and leaves the
//! paper book able to start. The real `trader` binary against the mock broker,
//! with fake keys only.

mod common;

use common::live::{env, posts, run};
use common::{stderr, stdout};
use pt_store::db::Store;
use pt_store::trading::Book;
use std::process::Output;

fn code0(out: &Output, what: &str) {
    assert_eq!(
        out.status.code(),
        Some(0),
        "{what}\nstdout: {}\nstderr: {}",
        stdout(out),
        stderr(out)
    );
}

#[test]
fn flatten_in_ingest_then_paper_start_opens_the_book_flat() {
    // R-551
    let e = env();
    std::fs::write(&e.owner, "stage = \"ingest\"\n").unwrap();
    e.mock.seed_position("SPY", 10);

    let out = run(&e, &["flatten", "--yes"]);
    code0(&out, "flatten --yes");
    assert!(
        stdout(&out).lines().any(|l| l.trim() == "accepted 1"),
        "stdout: {}",
        stdout(&out)
    );
    let sent = posts(&e);
    assert_eq!(sent.len(), 1, "POSTs: {sent:?}");
    let body: serde_json::Value = serde_json::from_str(&sent[0].body).unwrap();
    assert_eq!(body["symbol"].as_str(), Some("SPY"), "{body}");
    assert_eq!(body["side"].as_str(), Some("sell"), "{body}");
    let qty: u64 = body["qty"].as_str().unwrap().parse().unwrap();
    assert_eq!(qty, 10);
    e.mock
        .fill(body["client_order_id"].as_str().unwrap(), qty, "140.00");

    // The owner moves to the paper stage with the allowlist and flat confirmation.
    std::fs::write(
        &e.owner,
        "stage = \"paper\"\nstrategy_allowlist = [\"tsmom-v1\"]\n\
         account_flat_confirmed_on = 2024-02-29\n",
    )
    .unwrap();

    let status = run(&e, &["halt", "status"]);
    assert_eq!(status.status.code(), Some(2), "{}", stdout(&status));
    assert!(stdout(&status).contains("flattened"), "{}", stdout(&status));

    let clear = run(&e, &["halt", "clear", "--id", "1", "--note", "flat"]);
    code0(&clear, "halt clear");

    code0(&run(&e, &["run"]), "run");

    let store = Store::open(&e.dir.path().join("test.sqlite")).unwrap();
    let ledger = store.ledger(Book::Paper).unwrap();
    assert_eq!(
        ledger.first().map(|l| l.kind.as_str()),
        Some("open_balance"),
        "ledger: {ledger:?}"
    );
    drop(store);

    let path = e.dir.path().join("reports").join("status.json");
    let status: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("status.json exists")).unwrap();
    let positions = status["positions"].as_array().expect("positions array");
    assert!(positions.is_empty(), "positions: {positions:?}");
}

//! R-399 (a mock paper session works end to end) and R-402 (the binary never
//! writes the owner file): the real `trader` binary against the mock broker,
//! with fake keys only.

mod common;

use common::{stderr, stdout};
use pt_mock_alpaca::server::MockAlpaca;
use pt_store::db::Store;
use pt_store::trading::Book;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Command, Output};

use chrono::{Datelike, NaiveDate, Weekday};

fn d(m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2024, m, day).unwrap()
}

/// Weekdays from 2024-01-02 to 2024-02-29 less the two holidays: 41 sessions.
fn sessions() -> Vec<NaiveDate> {
    let holidays = [d(1, 15), d(2, 19)];
    let mut out = Vec::new();
    let mut day = d(1, 2);
    while day <= d(2, 29) {
        if !matches!(day.weekday(), Weekday::Sat | Weekday::Sun) && !holidays.contains(&day) {
            out.push(day);
        }
        day = day.succ_opt().unwrap();
    }
    out
}

/// The broker calendar: the bar sessions plus March 2024 through the 28th,
/// so 2024-02-29 is a month end and 2024-03-01 is the trade session. Bars
/// stay on the 41 sessions above, as in the live fixture.
fn calendar_sessions() -> Vec<NaiveDate> {
    let mut out = sessions();
    let mut day = d(3, 1);
    while day <= d(3, 28) {
        if !matches!(day.weekday(), Weekday::Sat | Weekday::Sun) {
            out.push(day);
        }
        day = day.succ_opt().unwrap();
    }
    out
}

const CONFIG: &str = r#"
[data]
start = "2024-01-02"

[strategy]
lookback_months = [1]
risk_free_symbol = "BIL"
vol_min_obs = 5
vol_max_obs = 20

[sizing]
max_weight = 0.5

[risk]
max_order_notional = 60000

[[universe]]
symbol = "SPY"
asset_class = "equity"
shortable = true

[[universe]]
symbol = "TLT"
asset_class = "equity"
shortable = true
"#;

struct Env {
    dir: tempfile::TempDir,
    home: tempfile::TempDir,
    cfg: PathBuf,
    owner: PathBuf,
    mock: MockAlpaca,
}

/// The live fixture: stage `paper`, the mock pre-open on 2024-03-01.
fn env() -> Env {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let db = dir.path().join("test.sqlite");
    let cfg = dir.path().join("cfg.toml");
    std::fs::write(
        &cfg,
        format!(
            "{CONFIG}\n[run]\ndb_path = \"{}\"\noutput_dir = \"{}\"\n",
            db.display(),
            dir.path().join("reports").display()
        ),
    )
    .unwrap();
    let owner = home.path().join("owner.toml");
    std::fs::write(
        &owner,
        "stage = \"paper\"\nstrategy_allowlist = [\"tsmom-v1\"]\n\
         account_flat_confirmed_on = 2024-02-29\n",
    )
    .unwrap();

    let mock = MockAlpaca::start();
    mock.seed_account("100000");
    let days = sessions();
    let dates: Vec<String> = calendar_sessions().iter().map(|s| s.to_string()).collect();
    let refs: Vec<&str> = dates.iter().map(String::as_str).collect();
    mock.seed_sessions(&refs);
    for s in ["SPY", "TLT", "BIL"] {
        mock.seed_asset(s, true, true, true);
    }
    mock.set_clock(
        "2024-03-01T08:00:00-05:00",
        false,
        "2024-03-01T09:30:00-05:00",
        "2024-03-01T16:00:00-05:00",
    );
    let close_of = |name: &str, i: i64| -> f64 {
        match name {
            "SPY" => (100 + i + (i % 2)) as f64,
            "TLT" => (1000 - 5 * i + 2 * (i % 3)) as f64 / 10.0,
            _ => (10000 + i) as f64 / 100.0,
        }
    };
    let mut bars = Vec::new();
    for (i, date) in days.iter().enumerate() {
        let i = i as i64;
        for name in ["SPY", "TLT", "BIL"] {
            let close = close_of(name, i);
            let open = close_of(name, (i - 1).max(0));
            bars.push((
                name.to_string(),
                date.to_string(),
                open,
                open.max(close),
                open.min(close),
                close,
                1_000_000u64,
            ));
        }
    }
    mock.seed_bars("raw", "sip", bars.clone());
    mock.seed_bars("all", "sip", bars);
    Env {
        dir,
        home,
        cfg,
        owner,
        mock,
    }
}

fn run(e: &Env, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_trader"))
        .arg("--config")
        .arg(&e.cfg)
        .args(args)
        .current_dir(e.dir.path())
        .env("HOME", e.home.path())
        .env("PT_OWNER_FILE", &e.owner)
        .env("APCA_API_KEY_ID", "fake-key-id")
        .env("APCA_API_SECRET_KEY", "fake-secret")
        .env("PT_ALPACA_TRADING_URL", e.mock.url())
        .env("PT_ALPACA_DATA_URL", e.mock.url())
        .env_remove("PT_CONFIG")
        .env_remove("PT_SECRETS_FILE")
        .output()
        .unwrap()
}

fn posts(e: &Env) -> Vec<pt_mock_alpaca::state::Recorded> {
    e.mock
        .requests()
        .into_iter()
        .filter(|r| r.method == "POST")
        .collect()
}

/// The positions the mock itself serves at `GET /v2/positions`.
fn mock_served_positions(e: &Env) -> BTreeMap<String, i64> {
    let addr = e.mock.url().trim_start_matches("http://").to_string();
    let mut conn = TcpStream::connect(&addr).unwrap();
    write!(
        conn,
        "GET /v2/positions HTTP/1.1\r\nHost: {addr}\r\nAPCA-API-KEY-ID: fake-key-id\r\n\
         APCA-API-SECRET-KEY: fake-secret\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut raw = String::new();
    conn.read_to_string(&mut raw).unwrap();
    let body = raw.split("\r\n\r\n").nth(1).expect("http body");
    let body = body
        .trim_start_matches(|c: char| c != '[')
        .trim_end_matches(|c: char| c != ']');
    let list: serde_json::Value = serde_json::from_str(body).expect("positions JSON");
    list.as_array()
        .expect("positions array")
        .iter()
        .map(|p| {
            (
                p["symbol"].as_str().unwrap().to_string(),
                p["qty"]
                    .as_str()
                    .unwrap()
                    .parse::<i64>()
                    .expect("integer qty"),
            )
        })
        .collect()
}

fn status_json(e: &Env) -> serde_json::Value {
    let path = e.dir.path().join("reports").join("status.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("status.json exists"))
        .expect("status.json is JSON")
}

/// Row counts of the tables the session writes, to compare between runs.
fn counts(e: &Env) -> Vec<(&'static str, usize)> {
    let store = Store::open(&e.dir.path().join("test.sqlite")).unwrap();
    vec![
        ("fills", store.fills().unwrap().len()),
        ("slippage", store.slippage_rows().unwrap().len()),
        ("shadow_daily", store.shadow_days().unwrap().len()),
        (
            "paper_equity",
            store.equity_series(Book::Paper).unwrap().len(),
        ),
        ("paper_ledger", store.ledger(Book::Paper).unwrap().len()),
        ("shadow_ledger", store.ledger(Book::Shadow).unwrap().len()),
    ]
}

fn ok(e: &Env, args: &[&str]) -> Output {
    let out = run(e, args);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{args:?}\nstdout: {}\nstderr: {}",
        stdout(&out),
        stderr(&out)
    );
    out
}

/// Plays the whole session and checks R-399. Returns the mock's positions
/// (symbol to signed quantity), derived from the orders it accepted and filled.
fn paper_session(e: &Env) -> BTreeMap<String, i64> {
    // (1) pre-open run: exits 0 and submits two orders.
    ok(e, &["run"]);
    let sent = posts(e);
    assert_eq!(sent.len(), 2, "POSTs: {sent:?}");

    // (2) fill both orders at the mock, seed 2024-03-01 bars, set a post-close clock.
    let mut mock_positions = BTreeMap::new();
    for p in &sent {
        let body: serde_json::Value = serde_json::from_str(&p.body).unwrap();
        let text = |k: &str| body[k].as_str().unwrap_or_default().to_string();
        let qty: u64 = text("qty").parse().expect("integer qty");
        assert!(qty > 0, "{body}");
        let symbol = text("symbol");
        let price = if symbol == "SPY" { "140.00" } else { "80.20" };
        e.mock.fill(&text("client_order_id"), qty, price);
        let signed = if text("side") == "sell" {
            -(qty as i64)
        } else {
            qty as i64
        };
        mock_positions.insert(symbol, signed);
    }
    e.mock.seed_bars(
        "raw",
        "sip",
        vec![
            (
                "SPY".into(),
                "2024-03-01".into(),
                141.0,
                142.0,
                141.0,
                142.0,
                1_000_000u64,
            ),
            (
                "TLT".into(),
                "2024-03-01".into(),
                80.0,
                80.0,
                79.9,
                79.9,
                1_000_000,
            ),
            (
                "BIL".into(),
                "2024-03-01".into(),
                100.4,
                100.41,
                100.4,
                100.41,
                1_000_000,
            ),
        ],
    );
    e.mock.seed_bars(
        "all",
        "sip",
        vec![
            (
                "SPY".into(),
                "2024-03-01".into(),
                141.0,
                142.0,
                141.0,
                142.0,
                1_000_000u64,
            ),
            (
                "TLT".into(),
                "2024-03-01".into(),
                80.0,
                80.0,
                79.9,
                79.9,
                1_000_000,
            ),
            (
                "BIL".into(),
                "2024-03-01".into(),
                100.4,
                100.41,
                100.4,
                100.41,
                1_000_000,
            ),
        ],
    );
    e.mock.set_clock(
        "2024-03-01T17:30:00-05:00",
        false,
        "2024-03-04T09:30:00-05:00",
        "2024-03-04T16:00:00-05:00",
    );

    // (3) post-close run: fills, positions, snapshots, shadow row, slippage, status.json.
    ok(e, &["run"]);
    let store = Store::open(&e.dir.path().join("test.sqlite")).unwrap();
    assert_eq!(store.fills().unwrap().len(), 2, "two fills");
    let day = |m, dd| NaiveDate::from_ymd_opt(2024, m, dd).unwrap();
    let stored: BTreeMap<String, i64> = store
        .positions_at(Book::Paper, day(3, 1))
        .unwrap()
        .iter()
        .map(|(s, q)| (s.as_str().to_string(), *q))
        .collect();
    assert_eq!(stored, mock_positions, "store positions equal the mock's");
    assert_eq!(
        stored,
        mock_served_positions(e),
        "store positions equal what the mock serves"
    );
    let snaps: Vec<NaiveDate> = store
        .equity_series(Book::Paper)
        .unwrap()
        .iter()
        .map(|s| s.date)
        .collect();
    for want in [day(2, 29), day(3, 1)] {
        assert!(snaps.contains(&want), "paper snapshot {want} in {snaps:?}");
    }
    let shadow: Vec<NaiveDate> = store
        .shadow_days()
        .unwrap()
        .iter()
        .map(|s| s.date)
        .collect();
    assert!(shadow.contains(&day(3, 1)), "shadow_daily row: {shadow:?}");
    assert_eq!(store.slippage_rows().unwrap().len(), 2, "two slippage rows");
    drop(store);

    let status = status_json(e);
    let listed: BTreeMap<String, i64> = status["positions"]
        .as_array()
        .expect("positions array")
        .iter()
        .map(|p| {
            (
                p["symbol"].as_str().unwrap().to_string(),
                p["qty"].as_i64().unwrap(),
            )
        })
        .collect();
    assert_eq!(listed, mock_positions, "status.json lists both positions");
    assert_eq!(listed.len(), 2);

    // (4) the sign-off verdict.
    let out = ok(e, &["signoff", "report"]);
    assert!(stdout(&out).contains("NOT_READY"), "{}", stdout(&out));

    // (5) a third run changes no row count and sends no order.
    let before = counts(e);
    ok(e, &["run"]);
    assert_eq!(counts(e), before, "a third run changes no row count");
    assert_eq!(posts(e).len(), 2, "no further order");
    mock_positions
}

#[test]
fn mock_paper_session_runs_end_to_end() {
    // R-399
    let e = env();
    paper_session(&e);
}

#[cfg(unix)]
#[test]
fn a_read_only_owner_file_is_read_never_written() {
    // R-402: mode 0400, and every command still exits as in R-399.
    use std::os::unix::fs::PermissionsExt;
    let e = env();
    std::fs::set_permissions(&e.owner, std::fs::Permissions::from_mode(0o400)).unwrap();
    let bytes = std::fs::read(&e.owner).unwrap();
    let mtime = std::fs::metadata(&e.owner).unwrap().modified().unwrap();

    paper_session(&e);

    assert_eq!(std::fs::read(&e.owner).unwrap(), bytes, "owner bytes");
    let meta = std::fs::metadata(&e.owner).unwrap();
    assert_eq!(meta.modified().unwrap(), mtime, "owner mtime");
    assert_eq!(meta.permissions().mode() & 0o777, 0o400, "owner mode");
    // The run did read it: it is in the paper stage, so orders went out.
    assert_eq!(posts(&e).len(), 2);
}

/// Names under the `Commands:` heading of a clap help text.
fn subcommands(help: &str) -> Vec<String> {
    help.lines()
        .skip_while(|l| !l.starts_with("Commands:"))
        .skip(1)
        .take_while(|l| l.starts_with("  "))
        .filter_map(|l| l.split_whitespace().next())
        .filter(|n| *n != "help")
        .map(String::from)
        .collect()
}

fn collect_help(e: &Env, path: &[String], out: &mut Vec<(String, String)>) {
    let mut args: Vec<&str> = path.iter().map(String::as_str).collect();
    args.push("--help");
    let help = stdout(&ok(e, &args));
    for sub in subcommands(&help) {
        let mut next = path.to_vec();
        next.push(sub);
        collect_help(e, &next, out);
    }
    out.push((path.join(" "), help));
}

#[test]
fn no_help_text_offers_a_way_to_set_stage_allowlist_flat_or_approval() {
    // R-402
    let e = env();
    let mut all = Vec::new();
    collect_help(&e, &[], &mut all);
    assert!(all.len() > 5, "walked the subcommand tree: {}", all.len());
    for (name, help) in &all {
        let lower = help.to_lowercase();
        for word in [
            "--stage",
            "allowlist",
            "--flat",
            "flat-confirm",
            "approv",
            "--confirm",
            "--set-",
            "--first-order",
            "--allow",
        ] {
            assert!(
                !lower.contains(word),
                "`trader {name} --help` offers {word}:\n{help}"
            );
        }
    }
}

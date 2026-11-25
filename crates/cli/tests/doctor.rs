//! R-387: `trader doctor` prints one line per check with PASS or FAIL, exits 0
//! when all pass and 1 otherwise; with the `sip` feed denied the `feed` line
//! fails and mentions `iex`.

mod common;

use common::{stderr, stdout};
use pt_mock_alpaca::server::MockAlpaca;
use std::path::PathBuf;
use std::process::{Command, Output};

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
    std::fs::write(&owner, "stage = \"ingest\"\n").unwrap();

    let mock = MockAlpaca::start();
    mock.seed_account("100000");
    mock.seed_sessions(&["2024-02-26", "2024-02-27", "2024-02-28", "2024-02-29"]);
    for s in ["SPY", "TLT", "BIL"] {
        mock.seed_asset(s, true, true, true);
    }
    mock.set_clock(
        "2024-03-01T08:00:00-05:00",
        false,
        "2024-03-01T09:30:00-05:00",
        "2024-03-01T16:00:00-05:00",
    );
    let mut bars = Vec::new();
    for date in [
        "2024-02-26",
        "2024-02-27",
        "2024-02-28",
        "2024-02-29",
        "2024-03-01",
    ] {
        for name in ["SPY", "TLT", "BIL"] {
            bars.push((
                name.to_string(),
                date.to_string(),
                100.0,
                100.0,
                100.0,
                100.0,
                1_000_000u64,
            ));
        }
    }
    mock.seed_bars("raw", "sip", bars.clone());
    mock.seed_bars("all", "sip", bars.clone());
    mock.seed_bars("raw", "iex", bars.clone());
    mock.seed_bars("all", "iex", bars);
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

fn lines_for<'a>(text: &'a str, check: &str) -> Vec<&'a str> {
    text.lines()
        .filter(|l| {
            l.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .any(|t| t == check)
        })
        .collect()
}

#[test]
fn doctor_prints_one_pass_line_per_check_and_exits_zero_when_all_pass() {
    // R-387
    let e = env();
    let out = run(&e, &["doctor"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stdout: {} stderr: {}",
        stdout(&out),
        stderr(&out)
    );
    let text = stdout(&out);
    for check in [
        "account_active",
        "shorting_enabled",
        "margin_multiplier",
        "clock",
        "feed",
        "asset_fields",
    ] {
        let lines = lines_for(&text, check);
        assert_eq!(lines.len(), 1, "one line for {check} in: {text}");
        assert!(lines[0].contains("PASS"), "{}", lines[0]);
        assert!(!lines[0].contains("FAIL"), "{}", lines[0]);
    }
    assert!(!text.contains("FAIL"), "{text}");
}

#[test]
fn doctor_fails_the_feed_line_and_mentions_iex_when_sip_is_denied() {
    // R-387
    let e = env();
    e.mock.deny_feed("sip");
    let out = run(&e, &["doctor"]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "stdout: {} stderr: {}",
        stdout(&out),
        stderr(&out)
    );
    let text = stdout(&out);
    let feed = lines_for(&text, "feed");
    assert_eq!(feed.len(), 1, "one feed line in: {text}");
    assert!(feed[0].contains("FAIL"), "{}", feed[0]);
    assert!(feed[0].contains("iex"), "{}", feed[0]);
    // A failure does not stop the other checks.
    let account = lines_for(&text, "account_active");
    assert_eq!(account.len(), 1, "{text}");
    assert!(account[0].contains("PASS"), "{}", account[0]);
}

//! R-132: the calendar syncs from Alpaca into the store.

use chrono::NaiveDate;
use pt_alpaca::http::loopback_client;
use pt_data::reference::sync_calendar;
use pt_mock_alpaca::server::MockAlpaca;
use pt_store::db::Store;

fn d(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").expect("date")
}

// R-132: five seeded sessions in range are returned and stored.
#[test]
fn sync_calendar_stores_the_five_seeded_sessions() {
    let sessions = [
        "2024-01-02",
        "2024-01-03",
        "2024-01-04",
        "2024-01-05",
        "2024-01-08",
    ];
    let mock = MockAlpaca::start();
    mock.seed_sessions(&sessions);
    let http = loopback_client(&mock.url()).expect("loopback");
    let store = Store::open_in_memory().expect("store");

    let n = sync_calendar(&store, &http, d("2024-01-01"), d("2024-01-31")).expect("sync");
    assert_eq!(n, 5);

    let cal = store.calendar().expect("calendar");
    let stored = cal.sessions_between(d("2024-01-01"), d("2024-01-31"));
    let expected: Vec<NaiveDate> = sessions.iter().map(|s| d(s)).collect();
    assert_eq!(stored, expected.as_slice());
}

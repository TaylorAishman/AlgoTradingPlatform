//! R-429: operations records keep instants to the second and list equal
//! instants in the order they were written.

use chrono::{DateTime, TimeZone, Utc};
use pt_store::db::Store;

fn at(ms: i64) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2024, 3, 1, 10, 0, 0).unwrap() + chrono::Duration::milliseconds(ms)
}

#[test]
fn halts_set_in_the_same_second_keep_write_order_and_whole_second_instants() {
    // R-429
    let store = Store::open_in_memory().unwrap();
    let first = store.set_halt("first", "a", at(900)).unwrap();
    let second = store.set_halt("second", "b", at(100)).unwrap();
    let rows = store.halts().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].id, first);
    assert_eq!(rows[1].id, second);
    assert_eq!(rows[0].reason, "first");
    assert_eq!(rows[1].reason, "second");
    let whole = Utc.with_ymd_and_hms(2024, 3, 1, 10, 0, 0).unwrap();
    assert_eq!(rows[0].set_at, whole);
    assert_eq!(rows[1].set_at, whole);
}

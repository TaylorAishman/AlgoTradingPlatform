//! Run slots: weekday times at 08:00, 09:45 and 17:30 New York time.

use chrono::{DateTime, Datelike, NaiveTime, TimeZone, Utc, Weekday};
use chrono_tz::America::New_York;

/// The slot times of a weekday, in New York local time.
const SLOTS: [(u32, u32); 3] = [(8, 0), (9, 45), (17, 30)];

/// True when a weekday slot lies after `last` and at or before `now`.
/// A first run (`last` is `None`) is always due.
pub fn slot_due(last: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool {
    let Some(last) = last else {
        return true;
    };
    if last >= now {
        return false;
    }
    let end = now.with_timezone(&New_York).date_naive();
    let mut day = last.with_timezone(&New_York).date_naive();
    while day <= end {
        if !matches!(day.weekday(), Weekday::Sat | Weekday::Sun) {
            for (h, m) in SLOTS {
                let Some(time) = NaiveTime::from_hms_opt(h, m, 0) else {
                    continue;
                };
                let Some(slot) = New_York.from_local_datetime(&day.and_time(time)).earliest()
                else {
                    continue;
                };
                let slot = slot.with_timezone(&Utc);
                if slot > last && slot <= now {
                    return true;
                }
            }
        }
        let Some(next) = day.succ_opt() else {
            break;
        };
        day = next;
    }
    false
}

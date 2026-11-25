//! R-334: on a day without orders the paper and shadow returns are identical.

mod common;

use common::*;
use pt_shadow::daily::shadow_day;

// R-334
#[test]
fn quiet_day_has_identical_paper_and_shadow_returns() {
    let store = shadow_store();
    let cfg = config();

    let day = shadow_day(&store, &cfg, d(2024, 1, 4)).unwrap();

    let expected = 102_960.0_f64 / 100_960.0 - 1.0;
    assert_eq!(day.date, d(2024, 1, 4));
    assert_eq!(day.diff, 0.0);
    assert_eq!(day.paper_return, day.shadow_return);
    assert!((day.paper_return - expected).abs() < 1e-12);
    assert!((day.shadow_return - expected).abs() < 1e-12);
}

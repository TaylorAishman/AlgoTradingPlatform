//! Tests for mapping Alpaca order statuses to internal order states.

use pt_core::order_state::OrderState;
use pt_live::error::LiveError;
use pt_live::status::map_status;

const OPEN_STATUSES: [&str; 10] = [
    "new",
    "accepted",
    "pending_new",
    "accepted_for_bidding",
    "held",
    "done_for_day",
    "calculated",
    "stopped",
    "suspended",
    "pending_cancel",
];

// R-281
#[test]
fn open_statuses_with_no_fill_map_to_submitted() {
    for status in OPEN_STATUSES {
        assert_eq!(
            map_status(status, 0),
            Ok(OrderState::Submitted),
            "status {status}"
        );
    }
}

// R-281
#[test]
fn open_statuses_with_a_fill_map_to_partially_filled() {
    for status in OPEN_STATUSES {
        assert_eq!(
            map_status(status, 3),
            Ok(OrderState::PartiallyFilled),
            "status {status}"
        );
    }
}

// R-281
#[test]
fn partially_filled_maps_to_partially_filled() {
    assert_eq!(
        map_status("partially_filled", 4),
        Ok(OrderState::PartiallyFilled)
    );
}

// R-281
#[test]
fn filled_maps_to_filled() {
    assert_eq!(map_status("filled", 10), Ok(OrderState::Filled));
}

// R-281
#[test]
fn terminal_statuses_map_to_their_states() {
    assert_eq!(map_status("canceled", 0), Ok(OrderState::Canceled));
    assert_eq!(map_status("expired", 0), Ok(OrderState::Expired));
    assert_eq!(map_status("rejected", 0), Ok(OrderState::Rejected));
}

// R-281
#[test]
fn replaced_statuses_are_unexpected() {
    assert_eq!(
        map_status("replaced", 0),
        Err(LiveError::UnexpectedStatus("replaced".to_string()))
    );
    assert_eq!(
        map_status("pending_replace", 0),
        Err(LiveError::UnexpectedStatus("pending_replace".to_string()))
    );
}

// R-281
#[test]
fn undocumented_status_is_unknown() {
    assert_eq!(
        map_status("bogus", 0),
        Err(LiveError::UnknownStatus("bogus".to_string()))
    );
}

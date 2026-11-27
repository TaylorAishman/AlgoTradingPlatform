use pt_core::order_state::{transition_allowed, OrderState};
use OrderState::*;

const ALL: [OrderState; 9] = [
    Intent,
    DryRun,
    Submitted,
    PartiallyFilled,
    Filled,
    Canceled,
    Expired,
    Rejected,
    Abandoned,
];

const ALLOWED: [(OrderState, OrderState); 13] = [
    (Intent, Submitted),
    (Intent, Rejected),
    (Intent, Abandoned),
    (Submitted, PartiallyFilled),
    (Submitted, Filled),
    (Submitted, Canceled),
    (Submitted, Expired),
    (Submitted, Rejected),
    (PartiallyFilled, PartiallyFilled),
    (PartiallyFilled, Filled),
    (PartiallyFilled, Canceled),
    (PartiallyFilled, Expired),
    (Rejected, Submitted),
];

// R-017: every listed pair is allowed.
#[test]
fn listed_transitions_are_allowed() {
    for (from, to) in ALLOWED {
        assert!(transition_allowed(from, to), "{from:?} -> {to:?}");
    }
}

// R-017: exactly the listed pairs are allowed, among all 81 pairs.
#[test]
fn every_other_pair_is_rejected() {
    let mut allowed_count = 0;
    for from in ALL {
        for to in ALL {
            let expected = ALLOWED.contains(&(from, to));
            assert_eq!(transition_allowed(from, to), expected, "{from:?} -> {to:?}");
            if expected {
                allowed_count += 1;
            }
        }
    }
    assert_eq!(allowed_count, 13);
}

// R-017: a rejected order may be resubmitted.
#[test]
fn rejected_may_move_to_submitted() {
    assert!(transition_allowed(Rejected, Submitted));
}

// R-017: from Rejected, only Submitted is reachable.
#[test]
fn rejected_reaches_nothing_but_submitted() {
    for to in ALL {
        assert_eq!(transition_allowed(Rejected, to), to == Submitted, "{to:?}");
    }
}

// R-017: DryRun and the other terminal states have no outgoing transitions.
#[test]
fn terminal_and_dry_run_states_have_no_exits() {
    for from in [DryRun, Filled, Canceled, Expired, Abandoned] {
        for to in ALL {
            assert!(!transition_allowed(from, to), "{from:?} -> {to:?}");
        }
    }
}

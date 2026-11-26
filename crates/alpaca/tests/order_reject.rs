use pt_alpaca::error::AlpacaError;

fn rejected(status: u16) -> AlpacaError {
    AlpacaError::Rejected {
        status,
        message: "m".to_string(),
    }
}

/// R-433: 403 and 422 are order rejections.
#[test]
fn rejected_403_and_422_are_order_rejects() {
    assert!(rejected(403).is_order_reject());
    assert!(rejected(422).is_order_reject());
}

/// R-433: other client statuses are not.
#[test]
fn rejected_400_401_404_409_are_not_order_rejects() {
    for status in [400, 401, 404, 409] {
        assert!(!rejected(status).is_order_reject(), "status {status}");
    }
}

/// R-433: other error kinds are not.
#[test]
fn other_errors_are_not_order_rejects() {
    let others = [
        AlpacaError::Server { status: 500 },
        AlpacaError::RateLimited,
        AlpacaError::Transport("t".to_string()),
        AlpacaError::Parse("p".to_string()),
    ];
    for e in others {
        assert!(!e.is_order_reject(), "{e:?}");
    }
}

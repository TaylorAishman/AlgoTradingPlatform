//! Order lifecycle states and the allowed transitions between them.

/// The lifecycle state of an order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderState {
    Intent,
    DryRun,
    Submitted,
    PartiallyFilled,
    Filled,
    Canceled,
    Expired,
    Rejected,
    Abandoned,
}

/// Whether an order may move from `from` to `to`.
pub fn transition_allowed(from: OrderState, to: OrderState) -> bool {
    use OrderState::*;
    matches!(
        (from, to),
        (Intent, Submitted | Rejected | Abandoned)
            | (
                Submitted,
                PartiallyFilled | Filled | Canceled | Expired | Rejected
            )
            | (
                PartiallyFilled,
                PartiallyFilled | Filled | Canceled | Expired
            )
            | (Rejected, Submitted)
    )
}

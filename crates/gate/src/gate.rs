//! Capability checks by operating stage and order authorisation. Anything not
//! explicitly allowed is denied.

use chrono::NaiveDate;

use crate::error::GateError;
use crate::owner::{OwnerFile, Stage};

/// Something the program may ask to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    MarketDataRead,
    TradingRead,
    RecordDryRun,
    SubmitOrder,
    CancelOrder,
}

/// The fields of an order that a permit is bound to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderSummary {
    pub client_order_id: String,
    pub symbol: String,
    pub side: String,
    pub qty: u64,
}

/// What the caller knows at the moment it asks to send an order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthContext {
    pub strategy_id: String,
    pub today: NaiveDate,
    pub halted: bool,
    pub risk_reducing: bool,
    pub is_first_order: bool,
    pub first_order_already_sent: bool,
}

/// Proof that the gate authorised exactly one order. Only `Gate::authorize`
/// can create one: the field is private and there is no public constructor.
///
/// R-265: a struct literal written outside this crate does not compile, because
/// the field is private.
///
/// ```compile_fail,E0451
/// use pt_gate::gate::{OrderPermit, OrderSummary};
///
/// let _forged = OrderPermit {
///     order: OrderSummary {
///         client_order_id: "x".to_string(),
///         symbol: "SPY".to_string(),
///         side: "buy".to_string(),
///         qty: 1,
///     },
/// };
/// ```
#[derive(Debug)]
pub struct OrderPermit {
    order: OrderSummary,
}

impl OrderPermit {
    /// Whether `o` is the order this permit was issued for: client order id,
    /// symbol, side and quantity must all be equal.
    pub fn matches(&self, o: &OrderSummary) -> bool {
        self.order.client_order_id == o.client_order_id
            && self.order.symbol == o.symbol
            && self.order.side == o.side
            && self.order.qty == o.qty
    }
}

/// The gate: holds what the owner has approved.
#[derive(Debug, Clone)]
pub struct Gate {
    owner: OwnerFile,
}

impl Gate {
    pub fn new(owner: OwnerFile) -> Gate {
        Gate { owner }
    }

    /// The operating stage the owner file grants.
    pub fn stage(&self) -> Stage {
        self.owner.stage
    }

    /// Whether the current stage allows `cap`. Fails closed: only entries in the
    /// stage table are allowed.
    pub fn allows(&self, cap: Capability) -> Result<(), GateError> {
        let allowed = match (self.owner.stage, cap) {
            (Stage::Backtest, _) => false,
            (
                Stage::Ingest | Stage::DryRun | Stage::FirstOrder | Stage::Paper,
                Capability::MarketDataRead | Capability::TradingRead,
            ) => true,
            (Stage::DryRun | Stage::FirstOrder | Stage::Paper, Capability::RecordDryRun) => true,
            (
                Stage::FirstOrder | Stage::Paper,
                Capability::SubmitOrder | Capability::CancelOrder,
            ) => true,
            _ => false,
        };
        if allowed {
            Ok(())
        } else {
            Err(GateError::StageForbids)
        }
    }

    /// Authorise one order. Fails closed: below `FirstOrder` the stage forbids
    /// it, and in `Paper` the strategy must be on the allowlist exactly.
    /// The `FirstOrder` stage refuses every order until its rules exist.
    pub fn authorize(
        &self,
        order: &OrderSummary,
        ctx: &AuthContext,
    ) -> Result<OrderPermit, GateError> {
        match self.owner.stage {
            Stage::Paper | Stage::FirstOrder => {}
            Stage::Backtest | Stage::Ingest | Stage::DryRun => {
                return Err(GateError::StageForbids);
            }
        }
        // R-267: no order, of any kind, before the owner confirms the account
        // is flat. Checked before every other rule, so no flag waives it.
        if self.owner.account_flat_confirmed_on.is_none() {
            return Err(GateError::FlatNotConfirmed);
        }
        // The first-order rules are not built yet: refuse until they are.
        if self.owner.stage == Stage::FirstOrder {
            return Err(GateError::StageForbids);
        }
        if !self.owner.strategy_allowlist.contains(&ctx.strategy_id) {
            return Err(GateError::StrategyNotAllowed);
        }
        // R-266: a halt refuses everything except an order the caller marks as
        // risk-reducing. The flag is the only proof the gate accepts.
        if ctx.halted && !ctx.risk_reducing {
            return Err(GateError::Halted);
        }
        Ok(OrderPermit {
            order: order.clone(),
        })
    }
}

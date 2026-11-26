//! Capability checks by operating stage and order authorisation. Anything not
//! explicitly allowed is denied.

use chrono::NaiveDate;

use crate::error::GateError;
use crate::owner::{FirstOrderApproval, OwnerFile, Stage};

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
/// and `Gate::authorize_kill_switch` can create one: the field is private and there is no public constructor.
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

    /// The first order the owner approved in `[first_order]`, if any. A plain
    /// reader, the same in every stage; it issues no permit. `authorize` is
    /// still the only way to send an order.
    pub fn first_order(&self) -> Option<&FirstOrderApproval> {
        self.owner.first_order.as_ref()
    }

    /// Whether the current stage allows `cap`. Fails closed: only entries in the
    /// stage table are allowed. Cancelling is the kill switch, so every stage
    /// allows it (R-263, R-321).
    pub fn allows(&self, cap: Capability) -> Result<(), GateError> {
        let allowed = match (self.owner.stage, cap) {
            (_, Capability::CancelOrder) => true,
            (Stage::Backtest, _) => false,
            (
                Stage::Ingest | Stage::DryRun | Stage::FirstOrder | Stage::Paper,
                Capability::MarketDataRead | Capability::TradingRead,
            ) => true,
            (Stage::DryRun | Stage::FirstOrder | Stage::Paper, Capability::RecordDryRun) => true,
            (Stage::FirstOrder | Stage::Paper, Capability::SubmitOrder) => true,
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
    /// In `FirstOrder` only the single approved order may go, once.
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
        if self.owner.stage == Stage::FirstOrder {
            return self.authorize_first_order(order, ctx);
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

    /// R-469, R-470 (decision G-53): authorise one kill-switch order. Allowed in
    /// every stage, with no flat confirmation, allowlist or halt check, because
    /// it can only reduce risk. Fails closed: the order must be a `pt-flatten-`
    /// order whose side is opposite to `broker_position` and whose quantity is
    /// between 1 and the absolute position.
    pub fn authorize_kill_switch(
        &self,
        order: &OrderSummary,
        broker_position: i64,
    ) -> Result<OrderPermit, GateError> {
        let reducing = order.client_order_id.starts_with("pt-flatten-")
            && order.qty >= 1
            && order.qty <= broker_position.unsigned_abs()
            && match order.side.as_str() {
                "sell" => broker_position > 0,
                "buy" => broker_position < 0,
                _ => false,
            };
        if !reducing {
            return Err(GateError::NotRiskReducing);
        }
        Ok(OrderPermit {
            order: order.clone(),
        })
    }

    /// R-268: the `FirstOrder` stage. Fails closed: a permit is issued only for
    /// the order the owner approved in `[first_order]`, on its day, once. The
    /// caller has already checked the flat confirmation. The allowlist does not
    /// apply: the approval itself authorises this one order.
    fn authorize_first_order(
        &self,
        order: &OrderSummary,
        ctx: &AuthContext,
    ) -> Result<OrderPermit, GateError> {
        // Only the first order may go in this stage; no other flag changes that.
        if !ctx.is_first_order {
            return Err(GateError::StageForbids);
        }
        // The strategy allowlist is not consulted here: the owner adds the
        // strategy to it only when moving to `Paper`, after this approval. The
        // exact match against `[first_order]` below is the authorisation.
        // The first order opens a position in a flat account, so it can never
        // reduce risk: a halt refuses it whatever the caller's flag says.
        if ctx.halted {
            return Err(GateError::Halted);
        }
        if ctx.first_order_already_sent {
            return Err(GateError::FirstOrderAlreadySent);
        }
        let Some(approved) = &self.owner.first_order else {
            return Err(GateError::FirstOrderMismatch);
        };
        // Exact comparison: no case or whitespace folding.
        // Only the approved quantity is authorised: `max_qty` is not a ceiling
        // for the order. A zero quantity is never approved.
        let matches = order.symbol == approved.symbol
            && order.side == approved.side
            && order.qty >= 1
            && order.qty == approved.qty
            && ctx.today == approved.valid_on;
        if !matches {
            return Err(GateError::FirstOrderMismatch);
        }
        Ok(OrderPermit {
            order: order.clone(),
        })
    }
}

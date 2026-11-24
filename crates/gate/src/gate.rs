//! Capability checks by operating stage. Anything not in the table is denied.

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
}

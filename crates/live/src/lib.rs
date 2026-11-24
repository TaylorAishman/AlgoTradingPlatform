//! Order identifiers and state mapping, idempotent submission, settlement, reconciliation, equity snapshots, rebalance decisions, the run cycle, the first-order flow and the kill switch.

pub mod context;
pub mod cycle;
pub mod error;
pub mod first_order;
pub mod ids;
pub mod killswitch;
pub mod lock;
pub mod rebalance;
pub mod reconcile;
pub mod settle;
pub mod snapshot;
pub mod status;
pub mod submit;

//! The strategy interface.

use crate::error::StrategyError;
use chrono::NaiveDate;
use pt_core::signal::SignalSet;
use pt_core::types::Symbol;
use pt_core::view::HistoryView;

/// Turns market history into one signal per symbol for a decision date.
pub trait Strategy {
    /// A stable identifier for the strategy and its version.
    fn id(&self) -> &str;
    /// The parameters in a canonical text form.
    fn params_canonical(&self) -> String;
    /// Signals for `universe` using only data on or before `asof`.
    fn signals(
        &self,
        universe: &[Symbol],
        asof: NaiveDate,
        view: &dyn HistoryView,
    ) -> Result<SignalSet, StrategyError>;
}

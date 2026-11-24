//! The simulated broker: fills at the open of the trade date.

use crate::cost::fill_cost;
use crate::error::EngineError;
use chrono::NaiveDate;
use pt_core::cost::CostSpec;
use pt_core::types::{Fill, OrderIntent};
use pt_core::view::MarketView;

/// A broker that fills every order at the open of the trade date.
#[derive(Debug, Clone, Copy)]
pub struct SimBroker {
    spec: CostSpec,
}

impl SimBroker {
    pub fn new(spec: CostSpec) -> Self {
        SimBroker { spec }
    }

    /// Fill `orders` in the order given at the open of `date`, charging `fill_cost`.
    /// An ordered symbol with no bar on `date` is `MissingBar`.
    pub fn execute(
        &self,
        date: NaiveDate,
        orders: &[OrderIntent],
        view: &dyn MarketView,
    ) -> Result<Vec<Fill>, EngineError> {
        orders
            .iter()
            .map(|o| {
                let bar =
                    view.exec_bar(&o.symbol, date)
                        .ok_or_else(|| EngineError::MissingBar {
                            symbol: o.symbol.to_string(),
                            date,
                        })?;
                let cost = fill_cost(&self.spec, o.qty, bar.open, None)?;
                Ok(Fill {
                    symbol: o.symbol.clone(),
                    side: o.side,
                    qty: o.qty,
                    price: bar.open,
                    cost,
                })
            })
            .collect()
    }
}

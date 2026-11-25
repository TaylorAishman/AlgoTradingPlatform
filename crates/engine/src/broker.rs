//! The simulated broker: fills at the open of the trade date.

use crate::cost::{fill_cost, ImpactInputs};
use crate::error::EngineError;
use chrono::NaiveDate;
use pt_core::cost::CostSpec;
use pt_core::types::{Fill, OrderIntent, Symbol};
use pt_core::view::MarketView;

/// Sessions in the volatility and volume windows.
const WINDOW: usize = 20;

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
                let impact = self.impact_inputs(&o.symbol, date, view)?;
                let cost = fill_cost(&self.spec, o.qty, bar.open, impact)?;
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

    /// Impact inputs for `symbol` on `date`, or `None` when the coefficient is zero.
    /// Volatility is the sample standard deviation of the last 20 simple returns of
    /// adjusted closes; ADV is the mean volume over 20 sessions.
    fn impact_inputs(
        &self,
        symbol: &Symbol,
        date: NaiveDate,
        view: &dyn MarketView,
    ) -> Result<Option<ImpactInputs>, EngineError> {
        if self.spec.impact_coeff.is_zero() {
            return Ok(None);
        }
        let closes = view.adj_closes(symbol, date, WINDOW + 1);
        let adv = view.adv_shares(symbol, date, WINDOW);
        let (true, Some(adv_shares)) = (closes.len() > WINDOW, adv) else {
            return Err(EngineError::MissingImpactInputs(format!(
                "{symbol}: need {} adjusted closes and {WINDOW} sessions of volume",
                WINDOW + 1
            )));
        };
        let rets: Vec<f64> = closes.windows(2).map(|w| w[1].1 / w[0].1 - 1.0).collect();
        let n = rets.len() as f64;
        let mean = rets.iter().sum::<f64>() / n;
        let var = rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1.0);
        Ok(Some(ImpactInputs {
            daily_vol: var.sqrt(),
            adv_shares,
        }))
    }
}

//! The borrowed context every live operation runs against.

use pt_alpaca::http::HttpClient;
use pt_config::config::Config;
use pt_core::clock::Clock;
use pt_gate::gate::Gate;
use pt_ops::alert::Alerter;
use pt_store::db::Store;
use pt_strategy::strategy::Strategy;

/// Everything a live operation needs, borrowed from its owner.
pub struct LiveContext<'a> {
    pub store: &'a Store,
    pub cfg: &'a Config,
    pub gate: &'a Gate,
    pub http: &'a HttpClient,
    pub clock: &'a dyn Clock,
    pub alerter: &'a Alerter,
    pub strategy: &'a dyn Strategy,
    pub code_version: &'a str,
}

//! C13 (R-031): the run, data, strategy and stats sections read their
//! architecture 5.3 defaults. Values are compared through their printed text
//! so no type is pinned beyond strings, unsigned integers and one date.

use pt_config::config::{parse, Config};

const UNIVERSE: &str =
    "[[universe]]\nsymbol = \"SPY\"\nasset_class = \"equity\"\nshortable = true\n";

fn with(extra: &str) -> Config {
    parse(&format!("{UNIVERSE}\n{extra}")).expect("config parses")
}

#[test]
fn run_section_defaults_when_absent() {
    // R-031
    let cfg = with("");
    assert_eq!(cfg.run.db_path.to_string(), "var/paper-trader.sqlite");
    assert_eq!(cfg.run.output_dir.to_string(), "var/reports");
}

#[test]
fn data_section_defaults_when_absent() {
    // R-031
    let cfg = with("");
    assert_eq!(cfg.data.feed.to_string(), "sip");
    assert_eq!(format!("{}", cfg.data.start), "2016-01-04");
    assert_eq!(format!("{}", cfg.data.max_requests_per_minute), "180");
}

#[test]
fn strategy_section_defaults_when_absent() {
    // R-031
    let s = with("").strategy;
    assert_eq!(s.id.to_string(), "tsmom-v1");
    assert_eq!(format!("{:?}", s.lookback_months), "[12]");
    assert_eq!(s.risk_free_symbol.to_string(), "BIL");
    assert_eq!(format!("{}", s.vol_com_days), "60");
    assert_eq!(format!("{}", s.vol_annualization), "261");
    assert_eq!(format!("{}", s.vol_min_obs), "126");
    assert_eq!(format!("{}", s.vol_max_obs), "504");
}

#[test]
fn stats_section_defaults_when_absent() {
    // R-031
    let s = with("").stats;
    assert_eq!(format!("{}", s.periods_per_year), "252");
    assert_eq!(format!("{}", s.pbo_partitions), "16");
}

#[test]
fn given_values_override_defaults() {
    // R-031
    let cfg = with(
        "[run]\ndb_path = \"a.db\"\noutput_dir = \"out\"\n\
         [data]\nfeed = \"csv:mydata\"\nstart = \"2020-02-03\"\nmax_requests_per_minute = 100\n\
         [strategy]\nid = \"x\"\nlookback_months = [1, 3, 12]\nrisk_free_symbol = \"SHY\"\n\
         vol_com_days = 30\nvol_annualization = 252\nvol_min_obs = 10\nvol_max_obs = 20\n\
         [stats]\nperiods_per_year = 260\npbo_partitions = 8\n",
    );
    assert_eq!(cfg.run.db_path.to_string(), "a.db");
    assert_eq!(cfg.run.output_dir.to_string(), "out");
    assert_eq!(cfg.data.feed.to_string(), "csv:mydata");
    assert_eq!(format!("{}", cfg.data.start), "2020-02-03");
    assert_eq!(format!("{}", cfg.data.max_requests_per_minute), "100");
    assert_eq!(cfg.strategy.id.to_string(), "x");
    assert_eq!(format!("{:?}", cfg.strategy.lookback_months), "[1, 3, 12]");
    assert_eq!(cfg.strategy.risk_free_symbol.to_string(), "SHY");
    assert_eq!(format!("{}", cfg.strategy.vol_com_days), "30");
    assert_eq!(format!("{}", cfg.strategy.vol_annualization), "252");
    assert_eq!(format!("{}", cfg.strategy.vol_min_obs), "10");
    assert_eq!(format!("{}", cfg.strategy.vol_max_obs), "20");
    assert_eq!(format!("{}", cfg.stats.periods_per_year), "260");
    assert_eq!(format!("{}", cfg.stats.pbo_partitions), "8");
}

#[test]
fn one_key_set_keeps_the_rest_of_its_section_default() {
    // R-031
    let cfg = with("[data]\nfeed = \"csv:mydata\"\n");
    assert_eq!(cfg.data.feed.to_string(), "csv:mydata");
    assert_eq!(format!("{}", cfg.data.start), "2016-01-04");
    assert_eq!(format!("{}", cfg.data.max_requests_per_minute), "180");
}

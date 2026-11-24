use pt_alpaca::endpoints::{Endpoints, MARKET_DATA_URL, PAPER_TRADING_URL};

// R-080: the default endpoints are the paper trading host and the market data host.
#[test]
fn production_trading_is_the_paper_host() {
    assert_eq!(
        Endpoints::production().trading(),
        "https://paper-api.alpaca.markets"
    );
}

// R-080
#[test]
fn production_data_is_the_market_data_host() {
    assert_eq!(
        Endpoints::production().data(),
        "https://data.alpaca.markets"
    );
}

// R-080: the two host constants carry the same values.
#[test]
fn host_constants_match_the_production_endpoints() {
    assert_eq!(PAPER_TRADING_URL, "https://paper-api.alpaca.markets");
    assert_eq!(MARKET_DATA_URL, "https://data.alpaca.markets");
}

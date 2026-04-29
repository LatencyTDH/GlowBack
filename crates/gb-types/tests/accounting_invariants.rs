use std::collections::HashMap;

use chrono::{Duration, Utc};
use gb_types::{Fill, Portfolio, Side, Symbol};
use rand::{rngs::StdRng, Rng, SeedableRng};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use uuid::Uuid;

fn test_fill(
    symbol: &Symbol,
    side: Side,
    quantity: Decimal,
    price: Decimal,
    commission: Decimal,
    offset_seconds: i64,
) -> Fill {
    Fill {
        id: Uuid::new_v4(),
        order_id: Uuid::new_v4(),
        symbol: symbol.clone(),
        side,
        quantity,
        price,
        commission,
        executed_at: Utc::now() + Duration::seconds(offset_seconds),
        strategy_id: "accounting-test".to_string(),
    }
}

fn assert_decimal_eq(left: Decimal, right: Decimal) {
    let tolerance = Decimal::new(1, 18);
    assert!(
        (left - right).abs() <= tolerance,
        "left={left} right={right} diff={}",
        left - right
    );
}

fn assert_accounting_identity(portfolio: &Portfolio) {
    let signed_market_value: Decimal = portfolio
        .positions
        .values()
        .map(|position| position.market_value)
        .sum();
    assert_decimal_eq(portfolio.total_equity, portfolio.cash + signed_market_value);
    assert_decimal_eq(
        portfolio.total_pnl,
        portfolio.total_realized_pnl + portfolio.total_unrealized_pnl,
    );
    assert_decimal_eq(
        portfolio.total_equity,
        portfolio.initial_capital + portfolio.total_pnl - portfolio.total_commissions,
    );

    for position in portfolio.positions.values() {
        assert_eq!(
            position.total_pnl(),
            position.realized_pnl + position.unrealized_pnl
        );
        if position.quantity > Decimal::ZERO {
            assert!(position.market_value >= Decimal::ZERO);
        } else if position.quantity < Decimal::ZERO {
            assert!(position.market_value <= Decimal::ZERO);
        }
    }
}

#[test]
fn long_only_positions_hold_equity_until_mark_changes() {
    let symbol = Symbol::equity("AAPL");
    let mut portfolio = Portfolio::new("acct-long".into(), dec!(1000));

    portfolio.apply_fill(&test_fill(
        &symbol,
        Side::Buy,
        dec!(10),
        dec!(100),
        Decimal::ZERO,
        0,
    ));

    let position = portfolio
        .get_position(&symbol)
        .expect("position should exist");
    assert_eq!(portfolio.cash, dec!(0));
    assert_eq!(position.market_value, dec!(1000));
    assert_eq!(portfolio.total_equity, dec!(1000));
    assert_eq!(portfolio.total_pnl, Decimal::ZERO);

    let mut prices = HashMap::new();
    prices.insert(symbol.clone(), dec!(110));
    portfolio.update_market_prices(&prices);

    let position = portfolio
        .get_position(&symbol)
        .expect("position should still exist");
    assert_eq!(position.market_value, dec!(1100));
    assert_eq!(position.unrealized_pnl, dec!(100));
    assert_eq!(portfolio.total_equity, dec!(1100));
    assert_accounting_identity(&portfolio);
}

#[test]
fn short_positions_carry_negative_market_value_and_positive_pnl_on_price_drop() {
    let symbol = Symbol::equity("TSLA");
    let mut portfolio = Portfolio::new("acct-short".into(), dec!(1000));

    portfolio.apply_fill(&test_fill(
        &symbol,
        Side::Sell,
        dec!(5),
        dec!(100),
        Decimal::ZERO,
        0,
    ));

    let position = portfolio
        .get_position(&symbol)
        .expect("short position should exist");
    assert_eq!(portfolio.cash, dec!(1500));
    assert_eq!(position.market_value, dec!(-500));
    assert_eq!(portfolio.total_equity, dec!(1000));
    assert_eq!(portfolio.total_pnl, Decimal::ZERO);

    let mut prices = HashMap::new();
    prices.insert(symbol.clone(), dec!(90));
    portfolio.update_market_prices(&prices);

    let position = portfolio
        .get_position(&symbol)
        .expect("short position should still exist");
    assert_eq!(position.market_value, dec!(-450));
    assert_eq!(position.unrealized_pnl, dec!(50));
    assert_eq!(portfolio.total_equity, dec!(1050));
    assert_accounting_identity(&portfolio);
}

#[test]
fn fractional_multi_asset_positions_and_commissions_balance() {
    let btc = Symbol::crypto("BTC-USD");
    let eth = Symbol::crypto("ETH-USD");
    let mut portfolio = Portfolio::new("acct-fractional".into(), dec!(10000));

    portfolio.apply_fill(&test_fill(
        &btc,
        Side::Buy,
        dec!(0.5),
        dec!(20000),
        dec!(25),
        0,
    ));
    portfolio.apply_fill(&test_fill(
        &eth,
        Side::Buy,
        dec!(2.25),
        dec!(2000),
        dec!(5),
        1,
    ));

    assert_eq!(portfolio.total_commissions, dec!(30));
    assert_eq!(portfolio.cash, dec!(-4530));
    assert_eq!(portfolio.total_equity, dec!(9970));
    assert_accounting_identity(&portfolio);

    let mut prices = HashMap::new();
    prices.insert(btc.clone(), dec!(21000));
    prices.insert(eth.clone(), dec!(1900));
    portfolio.update_market_prices(&prices);

    assert_eq!(portfolio.total_realized_pnl, Decimal::ZERO);
    assert_eq!(portfolio.total_unrealized_pnl, dec!(275));
    assert_eq!(portfolio.total_equity, dec!(10245));
    assert_accounting_identity(&portfolio);
}

#[test]
fn deterministic_random_fill_stream_preserves_accounting_invariants() {
    let symbols = [
        Symbol::equity("AAPL"),
        Symbol::equity("MSFT"),
        Symbol::crypto("BTC-USD"),
    ];
    let mut portfolio = Portfolio::new("acct-random".into(), dec!(50_000));
    let mut rng = StdRng::seed_from_u64(105);
    let mut prices: HashMap<Symbol, Decimal> = HashMap::new();

    for step in 0..200 {
        let symbol = symbols[rng.random_range(0..symbols.len())].clone();
        let side = if rng.random_bool(0.5) {
            Side::Buy
        } else {
            Side::Sell
        };
        let whole_qty: i64 = rng.random_range(1..=4);
        let half_step: i64 = rng.random_range(0..=1);
        let quantity = Decimal::from(whole_qty) + Decimal::new(half_step, 1);
        let price = Decimal::from(rng.random_range(25..=250));
        let commission = Decimal::new(rng.random_range(0..=20), 1);

        portfolio.apply_fill(&test_fill(
            &symbol,
            side,
            quantity,
            price,
            commission,
            step as i64,
        ));
        prices.insert(symbol, price);
        portfolio.update_market_prices(&prices);

        assert_accounting_identity(&portfolio);
    }
}

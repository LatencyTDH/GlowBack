use chrono::{DateTime, Duration, TimeZone, Utc};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use gb_types::{
    Bar, MarketDataBuffer, MarketEvent, Portfolio, Resolution, StrategyContext, Symbol,
};
use rust_decimal::Decimal;
use std::collections::HashMap;

const STRATEGY_MARKET_DATA_WINDOW: usize = 100;

fn timestamp(day_offset: i64) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap() + Duration::days(day_offset)
}

fn sample_market_data(
    symbol_count: usize,
    day_count: usize,
) -> (Vec<DateTime<Utc>>, HashMap<Symbol, Vec<Bar>>) {
    let dates = (0..day_count)
        .map(|day| timestamp(day as i64))
        .collect::<Vec<_>>();

    let market_data = (0..symbol_count)
        .map(|index| {
            let symbol = Symbol::equity(&format!("SYM{index:03}"));
            let bars = dates
                .iter()
                .enumerate()
                .map(|(day, current_time)| {
                    let price = Decimal::from(100 + index as i64 + day as i64);
                    Bar::new(
                        symbol.clone(),
                        *current_time,
                        price,
                        price,
                        price,
                        price,
                        Decimal::from(1_000),
                        Resolution::Day,
                    )
                })
                .collect::<Vec<_>>();
            (symbol, bars)
        })
        .collect::<HashMap<_, _>>();

    (dates, market_data)
}

fn legacy_rebuild_context(
    current_time: DateTime<Utc>,
    market_data: &HashMap<Symbol, Vec<Bar>>,
    portfolio: &Portfolio,
) -> StrategyContext {
    let mut context = StrategyContext::new("bench".to_string(), portfolio.initial_capital);
    context.current_time = current_time;
    context.portfolio = portfolio.clone();

    for (symbol, bars) in market_data {
        let mut buffer = MarketDataBuffer::new(symbol.clone(), STRATEGY_MARKET_DATA_WINDOW);
        for bar in bars {
            if bar.timestamp <= current_time {
                buffer.add_event(MarketEvent::Bar(bar.clone()));
            }
        }
        context.market_data.insert(symbol.clone(), buffer);
    }

    context
}

fn run_legacy_context_updates(
    dates: &[DateTime<Utc>],
    market_data: &HashMap<Symbol, Vec<Bar>>,
    portfolio: &Portfolio,
) -> StrategyContext {
    let mut latest = StrategyContext::new("bench".to_string(), portfolio.initial_capital);
    for &current_time in dates {
        latest = legacy_rebuild_context(current_time, market_data, portfolio);
    }
    latest
}

fn run_incremental_context_updates(
    dates: &[DateTime<Utc>],
    market_data: &HashMap<Symbol, Vec<Bar>>,
    portfolio: &Portfolio,
) -> StrategyContext {
    let mut context = StrategyContext::new("bench".to_string(), portfolio.initial_capital);
    context.portfolio = portfolio.clone();

    for symbol in market_data.keys() {
        context.market_data.insert(
            symbol.clone(),
            MarketDataBuffer::new(symbol.clone(), STRATEGY_MARKET_DATA_WINDOW),
        );
    }

    let mut next_indices = market_data
        .keys()
        .cloned()
        .map(|symbol| (symbol, 0usize))
        .collect::<HashMap<_, _>>();

    for &current_time in dates {
        context.current_time = current_time;
        let current_date = current_time.date_naive();

        for (symbol, bars) in market_data {
            let next_index = next_indices
                .get_mut(symbol)
                .expect("symbol index should exist");
            while let Some(bar) = bars.get(*next_index) {
                let bar_date = bar.timestamp.date_naive();
                if bar_date < current_date {
                    *next_index += 1;
                    continue;
                }
                if bar_date > current_date {
                    break;
                }

                context
                    .market_data
                    .get_mut(symbol)
                    .expect("symbol buffer should exist")
                    .add_event(MarketEvent::Bar(bar.clone()));
                *next_index += 1;
            }
        }
    }

    context
}

fn benchmark_strategy_context_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("strategy_context_scaling");

    for (symbol_count, day_count) in [(10usize, 126usize), (50usize, 252usize)] {
        let scenario_name = format!("{symbol_count}symbols_{day_count}days");
        let (dates, market_data) = sample_market_data(symbol_count, day_count);
        let portfolio = Portfolio::new("bench".to_string(), Decimal::from(100_000));

        group.throughput(Throughput::Elements((symbol_count * day_count) as u64));
        group.bench_with_input(
            BenchmarkId::new("legacy_full_rebuild", &scenario_name),
            &scenario_name,
            |b, _| {
                b.iter(|| {
                    black_box(run_legacy_context_updates(
                        black_box(&dates),
                        black_box(&market_data),
                        black_box(&portfolio),
                    ))
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("incremental_buffers", &scenario_name),
            &scenario_name,
            |b, _| {
                b.iter(|| {
                    black_box(run_incremental_context_updates(
                        black_box(&dates),
                        black_box(&market_data),
                        black_box(&portfolio),
                    ))
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, benchmark_strategy_context_scaling);
criterion_main!(benches);

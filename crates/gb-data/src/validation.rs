use std::collections::HashSet;

use chrono::{DateTime, Datelike, Utc, Weekday};
use gb_types::{Bar, DataValidationSummary, DatasetKind, PriceAdjustmentMode, Resolution, Symbol};
use rust_decimal::Decimal;

pub fn summarize_bars(
    bars: &[Bar],
    symbol: &Symbol,
    resolution: Resolution,
    dataset_kind: DatasetKind,
    price_adjustment: PriceAdjustmentMode,
) -> DataValidationSummary {
    let mut duplicate_timestamps = 0u64;
    let mut invalid_ohlcv_rows = 0u64;
    let mut negative_price_rows = 0u64;
    let mut negative_volume_rows = 0u64;
    let mut seen_timestamps = HashSet::new();
    let mut ordered_unique_timestamps = Vec::new();

    for bar in bars {
        if !seen_timestamps.insert(bar.timestamp) {
            duplicate_timestamps += 1;
        } else {
            ordered_unique_timestamps.push(bar.timestamp);
        }

        if bar.open < Decimal::ZERO
            || bar.high < Decimal::ZERO
            || bar.low < Decimal::ZERO
            || bar.close < Decimal::ZERO
        {
            negative_price_rows += 1;
        }

        if bar.volume < Decimal::ZERO {
            negative_volume_rows += 1;
        }

        if bar.high < bar.low
            || bar.high < bar.open
            || bar.high < bar.close
            || bar.low > bar.open
            || bar.low > bar.close
        {
            invalid_ohlcv_rows += 1;
        }
    }

    ordered_unique_timestamps.sort_unstable();
    let missing_intervals = count_missing_intervals(&ordered_unique_timestamps, symbol, resolution);
    let total_bars = ordered_unique_timestamps.len() as u64;
    let sample_data = dataset_kind == DatasetKind::Sample;
    let critical_issue_count =
        duplicate_timestamps + invalid_ohlcv_rows + negative_price_rows + negative_volume_rows;
    let warning_issue_count = missing_intervals + u64::from(sample_data);

    let mut critical_issues = Vec::new();
    let mut warnings = Vec::new();

    if duplicate_timestamps > 0 {
        critical_issues.push(format!(
            "Detected {} duplicate timestamp rows for {}.",
            duplicate_timestamps, symbol
        ));
    }
    if invalid_ohlcv_rows > 0 {
        critical_issues.push(format!(
            "Detected {} rows with invalid OHLC relationships for {}.",
            invalid_ohlcv_rows, symbol
        ));
    }
    if negative_price_rows > 0 {
        critical_issues.push(format!(
            "Detected {} rows with negative prices for {}.",
            negative_price_rows, symbol
        ));
    }
    if negative_volume_rows > 0 {
        critical_issues.push(format!(
            "Detected {} rows with negative volume for {}.",
            negative_volume_rows, symbol
        ));
    }
    if missing_intervals > 0 {
        warnings.push(format!(
            "Detected {} missing expected {} intervals for {}.",
            missing_intervals,
            resolution_label(resolution),
            symbol
        ));
    }
    if sample_data {
        warnings.push(
            "Synthetic sample/demo data loaded — use these results for product smoke tests, not research conclusions."
                .to_string(),
        );
    }

    DataValidationSummary {
        total_rows_seen: bars.len() as u64,
        total_bars,
        duplicate_timestamps,
        missing_intervals,
        invalid_ohlcv_rows,
        negative_price_rows,
        negative_volume_rows,
        has_critical_issues: critical_issue_count > 0,
        critical_issue_count,
        warning_issue_count,
        timezone: "UTC".to_string(),
        resolution: resolution.to_string(),
        dataset_kind,
        price_adjustment,
        sample_data,
        critical_issues,
        warnings,
    }
}

fn count_missing_intervals(
    timestamps: &[DateTime<Utc>],
    symbol: &Symbol,
    resolution: Resolution,
) -> u64 {
    if timestamps.len() < 2 {
        return 0;
    }

    match resolution {
        Resolution::Day if !symbol.asset_class.is_24_7() => {
            count_missing_weekdays(timestamps) as u64
        }
        _ if symbol.asset_class.is_24_7() => {
            let Some(seconds_per_bar) = resolution.to_seconds() else {
                return 0;
            };
            count_missing_fixed_intervals(timestamps, seconds_per_bar as i64)
        }
        _ => 0,
    }
}

fn count_missing_weekdays(timestamps: &[DateTime<Utc>]) -> usize {
    timestamps
        .windows(2)
        .map(|pair| {
            let mut current = pair[0].date_naive();
            let next = pair[1].date_naive();
            let mut missing = 0usize;

            while let Some(candidate) = current.succ_opt() {
                if candidate >= next {
                    break;
                }
                if !matches!(candidate.weekday(), Weekday::Sat | Weekday::Sun) {
                    missing += 1;
                }
                current = candidate;
            }

            missing
        })
        .sum()
}

fn count_missing_fixed_intervals(timestamps: &[DateTime<Utc>], interval_seconds: i64) -> u64 {
    timestamps
        .windows(2)
        .map(|pair| {
            let diff_seconds = (pair[1] - pair[0]).num_seconds();
            if diff_seconds <= interval_seconds {
                0
            } else {
                (diff_seconds / interval_seconds).saturating_sub(1) as u64
            }
        })
        .sum()
}

fn resolution_label(resolution: Resolution) -> &'static str {
    match resolution {
        Resolution::Tick => "tick",
        Resolution::Second => "second",
        Resolution::Minute => "minute",
        Resolution::FiveMinute => "5-minute",
        Resolution::FifteenMinute => "15-minute",
        Resolution::Hour => "hour",
        Resolution::FourHour => "4-hour",
        Resolution::Day => "daily",
        Resolution::Week => "weekly",
        Resolution::Month => "monthly",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use gb_types::{AssetClass, PriceAdjustmentMode};

    fn equity_symbol() -> Symbol {
        Symbol::new("AAPL", "NASDAQ", AssetClass::Equity)
    }

    fn sample_bar(day: u32) -> Bar {
        let symbol = equity_symbol();
        Bar::new(
            symbol,
            Utc.with_ymd_and_hms(2026, 4, day, 0, 0, 0).unwrap(),
            Decimal::from(100),
            Decimal::from(101),
            Decimal::from(99),
            Decimal::from(100),
            Decimal::from(10_000),
            Resolution::Day,
        )
    }

    #[test]
    fn summary_counts_missing_weekdays_for_daily_equity_data() {
        let bars = vec![sample_bar(6), sample_bar(8)]; // Monday + Wednesday
        let summary = summarize_bars(
            &bars,
            &equity_symbol(),
            Resolution::Day,
            DatasetKind::UserProvided,
            PriceAdjustmentMode::Raw,
        );

        assert_eq!(summary.missing_intervals, 1);
        assert!(!summary.has_critical_issues);
        assert_eq!(summary.warning_issue_count, 1);
    }

    #[test]
    fn summary_flags_duplicate_timestamps_as_critical() {
        let bars = vec![sample_bar(6), sample_bar(6)];
        let summary = summarize_bars(
            &bars,
            &equity_symbol(),
            Resolution::Day,
            DatasetKind::UserProvided,
            PriceAdjustmentMode::Raw,
        );

        assert_eq!(summary.duplicate_timestamps, 1);
        assert!(summary.has_critical_issues);
        assert!(summary.critical_issue_count >= 1);
    }

    #[test]
    fn summary_marks_sample_data_and_preserves_adjustment_metadata() {
        let bars = vec![sample_bar(6), sample_bar(7)];
        let summary = summarize_bars(
            &bars,
            &equity_symbol(),
            Resolution::Day,
            DatasetKind::Sample,
            PriceAdjustmentMode::Synthetic,
        );

        assert!(summary.sample_data);
        assert_eq!(summary.dataset_kind, DatasetKind::Sample);
        assert_eq!(summary.price_adjustment, PriceAdjustmentMode::Synthetic);
        assert!(summary
            .warnings
            .iter()
            .any(|warning| warning.contains("Synthetic sample/demo data")));
    }
}

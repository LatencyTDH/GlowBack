# Results & Metrics

Backtest results include:

- **Equity curve** and drawdowns
- **Performance metrics**: Sharpe, Sortino, Calmar, CAGR
- **Risk metrics**: VaR, CVaR, skewness, kurtosis
- **Trade analytics**: win rate, profit factor, average win/loss

## Annualized return (CAGR)

`annualized_return` is reported as a compounded annual growth rate (CAGR), not a simple linear scaling:

- `total_return` is the cumulative return at the end of the backtest
- `years = N / 252` where `N` is the number of daily return observations (252 trading days/year)
- `annualized_return = (1 + total_return)^(1/years) - 1`

Results are persisted for later analysis and reporting.

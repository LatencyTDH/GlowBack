# UI Workflow

## Steps

1. **Load data** in the Data Loader page.
2. **Create a strategy** in the Strategy Editor.
3. **Run backtest** in the Backtest Runner.
4. **Review results** in Results Dashboard and Portfolio Analyzer.
5. **Deep-dive analytics** in the Advanced Analytics page.

## Advanced Analytics (New)

The **🔬 Advanced Analytics** page provides:

### Heatmaps
- **Monthly returns heatmap** — calendar grid (month × year) colour-coded by return.
- **Correlation matrix** — pairwise return correlations for multi-symbol backtests.
- **Drawdown heatmap** — worst monthly drawdown in calendar form.

### Rolling Statistics
- Rolling Sharpe ratio with configurable window (30/60/90/252 days).
- Annualised rolling volatility with percentile bands.
- Rolling beta against an actual benchmark series loaded into the run.
- Rolling maximum drawdown (trailing window).

### Compare Runs
Save multiple backtest results and compare them side-by-side with an equity-curve overlay and metrics table.

### Parameter Sensitivity
Sweep two parameters across a grid and visualise the impact on return, Sharpe, drawdown, or win rate as a surface heatmap.

### Export
- Download equity curve and trades as CSV.
- Download summary metrics as JSON.
- Download an institutional-style tearsheet in JSON or Markdown.
- Use browser print (Ctrl+P) for a quick PDF snapshot.

## Dark Mode

Toggle **🌙 Dark Mode** in the sidebar for a dark colour scheme.

## Tips

- Start with sample data to validate logic quickly.
- Save configurations for reproducibility.
- Include benchmark bars in the loaded dataset to unlock real beta/alpha/information-ratio analytics.
- Export results or tearsheets for offline analysis.
- Use the *Compare Runs* tab to evaluate strategies against each other.

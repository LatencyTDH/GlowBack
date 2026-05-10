class LifecycleTemplateStrategy:
    """Example custom strategy showing the optional UI lifecycle hooks."""

    def __init__(self):
        self.name = "Lifecycle Template"
        self.primary_symbol = None
        self.entered = False

    def on_start(self, portfolio, metadata):
        self.primary_symbol = metadata["symbols"][0]
        return [
            f"Prepared {self.primary_symbol} strategy for {metadata['bars']} bars "
            f"from {metadata['start'].date().isoformat()} to {metadata['end'].date().isoformat()}"
        ]

    def on_bar(self, bar, portfolio):
        if bar.symbol != self.primary_symbol:
            return []

        if not self.entered and portfolio.get_position(bar.symbol) == 0:
            shares = int(portfolio.cash * 0.50 / bar.close)
            if shares > 0:
                portfolio.buy(bar.symbol, shares, bar.close, bar.timestamp)
                self.entered = True
                return [f"Entered {shares} shares of {bar.symbol} at ${bar.close:.2f}"]
        return []

    def on_day_end(self, trading_day, portfolio):
        return [f"Marked day end for {trading_day.date().isoformat()} with value ${portfolio.value:.2f}"]

    def on_finish(self, portfolio, summary):
        return [
            f"Finished with {summary['total_trades']} trades, cash ${summary['final_cash']:.2f}, "
            f"value ${summary['final_value']:.2f}"
        ]

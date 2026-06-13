# CSV Data

GlowBack's CSV provider expects files to live in a directory and follow the pattern
`{SYMBOL}_{resolution}.csv` (for example `AAPL_1d.csv`). This tutorial ships a
checked-in fixture so the docs stay executable instead of relying on a local file you
have to guess into the right shape.

## Run the checked-in tutorial

From the repository root:

```bash
./scripts/csv_data_tutorial.sh
```

That script builds `gb-python` in an isolated virtualenv, loads
`examples/data/AAPL_1d.csv`, verifies the expected January 2025 bars, and runs a
CSV-backed buy-and-hold backtest.

Expected success marker:

```text
✅ CSV data tutorial completed successfully
```

## Prepare your own CSV directory

Include columns for:

- `timestamp`
- `open`
- `high`
- `low`
- `close`
- `volume`

Use one file per symbol/resolution pair, for example:

```text
/path/to/csv-fixtures/
  AAPL_1d.csv
  MSFT_1d.csv
```

## Load via UI

1. Open the **Data Loader** page.
2. Select **CSV Upload**.
3. Map columns and choose a symbol.
4. Load and validate the dataset.

## Load via Python

```python
from pathlib import Path

import glowback

csv_dir = Path("examples/data")
manager = glowback.DataManager()
manager.add_csv_provider(str(csv_dir))

symbol = glowback.Symbol("AAPL", "NASDAQ", "equity")
bars = manager.load_data(
    symbol,
    "2025-01-02T00:00:00Z",
    "2025-01-31T23:59:59Z",
    "day",
)

engine = glowback.BacktestEngine(
    symbols=["AAPL"],
    start_date="2025-01-02T00:00:00Z",
    end_date="2025-01-31T23:59:59Z",
    data_source="csv",
    csv_data_path=str(csv_dir),
)
result = engine.run_buy_and_hold()
```

The full executable companion lives at `examples/csv_data_tutorial.py`.

# CSV Data

## Prepare a CSV

Include columns for timestamp, open, high, low, close, volume.

## Load via UI

1. Open the **Data Loader** page.
2. Select **CSV Upload**.
3. Map columns and choose a symbol.
4. Load and validate the dataset.

## Load via Python

```python
import glowback

manager = glowback.PyDataManager()
manager.add_csv_provider("/path/to/data")
```

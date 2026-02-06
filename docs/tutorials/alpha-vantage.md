# Alpha Vantage

## Setup

1. Get an API key from https://www.alphavantage.co/
2. Add the provider in Python or UI.

## Load via Python

```python
import glowback

manager = glowback.PyDataManager()
manager.add_alpha_vantage_provider("YOUR_API_KEY")
```

## Notes

Alpha Vantage has rate limits on the free tier. Use caching for repeated runs.

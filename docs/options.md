# Options Module (`gb-options`)

The `gb-options` crate adds options instrument support to GlowBack, including
pricing, greeks computation, chain generation, and trade execution simulation.

## Features

### Contract Modelling (`contract`)
- `OptionContract` — call/put, strike, expiration, exercise style, multiplier
- European and American exercise styles
- Intrinsic value, ITM/ATM/OTM classification, time-to-expiry helpers

### Black-Scholes Pricing (`pricing`)
- `black_scholes_price()` — theoretical price for European options
- Full greeks: delta, gamma, theta (daily), vega (per 1%), rho (per 1%)
- Dividend yield support via continuous-yield model
- `implied_volatility()` — Newton-Raphson solver to back out IV from market price

### Greeks (`greeks`)
- `Greeks` struct with delta, gamma, theta, vega, rho
- Computed analytically from the Black-Scholes closed-form solution

### Execution Simulation (`execution`)
- `simulate_open()` — open a long or short options position with theoretical premium
- `simulate_exercise()` — exercise at expiration (auto-exercise if ITM)
- `options_pnl()` — round-trip P&L calculation
- Commission handling per contract

### Option Chain (`chain`)
- `build_chain()` — generate a full option chain (calls + puts at evenly spaced strikes)
- `OptionChain` — ATM strike lookup, strike-level access, put-call parity

## Quick Start

```rust
use gb_options::*;
use gb_types::market::Symbol;
use rust_decimal_macros::dec;
use chrono::{Utc, TimeZone};

// Define a contract
let underlying = Symbol::equity("AAPL");
let expiration = Utc.with_ymd_and_hms(2026, 6, 20, 20, 0, 0).unwrap();
let contract = OptionContract::equity(
    underlying, OptionKind::Call, dec!(150), expiration,
);

// Price it
let input = PricingInput {
    spot: 155.0,
    risk_free_rate: 0.05,
    volatility: 0.25,
    dividend_yield: 0.0,
    time_to_expiry: 0.25,
};
let result = black_scholes_price(&contract, &input);
println!("Price: {}, Delta: {}", result.price, result.greeks.delta);

// Compute implied vol from a market price
let iv = implied_volatility(&contract, 8.50, 155.0, 0.05, 0.0, 0.25);
println!("IV: {:?}", iv);
```

## Tests

```bash
cargo test -p gb-options
```

34 unit tests covering:
- Contract intrinsic value, ITM/OTM, time-to-expiry
- Black-Scholes pricing sanity (call & put)
- Put-call parity verification
- Greeks sign correctness (call & put)
- Implied volatility round-trip convergence
- Exercise simulation (ITM call, ITM put, OTM rejection)
- Trade P&L round-trip
- Option chain generation and structure

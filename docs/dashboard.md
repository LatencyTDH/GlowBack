# React Dashboard

GlowBack ships a modern React dashboard for managing and reviewing backtest runs through
your browser.

## Overview

The dashboard connects to the [FastAPI gateway](../api/) and provides:

- **Run list** — table of all backtests with state badges, progress bars, and timing
- **Run launcher** — form to configure symbols, dates, strategy, execution parameters, and capital
- **Results view** — equity curve chart, drawdown chart, performance metric cards (Sharpe, Sortino, Calmar, VaR, …), trade logs, and a live WebSocket event stream

## Tech Stack

| Layer | Choice |
|-------|--------|
| Framework | React 19 + TypeScript |
| Bundler | Vite 7 |
| Styling | Tailwind CSS 4 |
| Charts | Recharts |
| Routing | React Router 7 |

## Quick Start

```bash
# Terminal 1 – API
cd api && pip install -r requirements.txt && uvicorn app.main:app --reload

# Terminal 2 – Dashboard
cd dashboard && npm install && npm run dev
```

Open http://localhost:5173.

## Development

The Vite dev server proxies `/api/*` to the FastAPI backend on port 8000, so no
CORS configuration is needed during development.

```bash
npm run dev      # hot-reload dev server
npm run build    # type-check + production build
npm run lint     # ESLint
npm run preview  # preview production bundle
```

## Architecture

```
Browser ──► Vite Dev Server (:5173) ──proxy──► FastAPI (:8000)
                                                  │
                                           Mock / Rust Engine
```

In production, the dashboard `dist/` folder can be served by any static file host
or embedded behind a reverse proxy alongside the API.

# GlowBack React Dashboard

Modern web dashboard for the GlowBack backtesting platform, built with React 19, TypeScript, Vite, and Tailwind CSS.

## Features

- **Backtest List** — view all runs with state, progress, and duration
- **New Backtest** — form to configure and launch runs (symbols, dates, strategy, execution params)
- **Backtest Detail** — real-time progress via WebSocket, equity curve chart, drawdown chart, full metrics grid, event stream, and logs
- **Responsive** — works on desktop and mobile

## Tech Stack

- [React 19](https://react.dev/) + [TypeScript](https://www.typescriptlang.org/)
- [Vite 7](https://vite.dev/) (build + dev server with API proxy)
- [Tailwind CSS 4](https://tailwindcss.com/)
- [Recharts](https://recharts.org/) (equity curve + drawdown charts)
- [React Router 7](https://reactrouter.com/)

## Quick Start

```bash
cd dashboard
npm install
npm run dev
```

The dev server starts on http://localhost:5173 and proxies `/api/*` requests to the FastAPI backend on port 8000.

### Start the API server

In a separate terminal:

```bash
cd api
pip install -r requirements.txt
uvicorn app.main:app --reload
```

## Scripts

| Command | Description |
|---------|-------------|
| `npm run dev` | Start dev server (hot reload) |
| `npm run build` | Type-check + production build |
| `npm run lint` | ESLint check |
| `npm run preview` | Preview production build |

## Project Structure

```
dashboard/
├── src/
│   ├── api.ts                  # API client (fetch + WebSocket)
│   ├── types.ts                # TypeScript types (mirrors API models)
│   ├── App.tsx                 # Router setup
│   ├── main.tsx                # Entry point
│   ├── index.css               # Tailwind imports
│   ├── components/
│   │   ├── Layout.tsx          # App shell (header + nav)
│   │   ├── StateBadge.tsx      # Run state badge
│   │   ├── EquityCurveChart.tsx # Equity curve line chart
│   │   ├── DrawdownChart.tsx   # Drawdown area chart
│   │   └── MetricsGrid.tsx     # Performance metrics cards
│   └── pages/
│       ├── BacktestList.tsx    # List all backtest runs
│       ├── NewBacktest.tsx     # Create new backtest form
│       └── BacktestDetail.tsx  # Run detail + results
├── vite.config.ts              # Vite config (proxy, Tailwind)
├── eslint.config.js            # ESLint config
└── package.json
```

## API Proxy

In development, Vite proxies `/api/*` to `http://127.0.0.1:8000` (stripping the `/api` prefix). This avoids CORS issues during development. The API also includes CORS middleware for production deployments where the dashboard and API may be on different origins.

## License

MIT — see the main repository for details.

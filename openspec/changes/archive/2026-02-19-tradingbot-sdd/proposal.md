## Why

ClawBot is a new automated cryptocurrency trading engine built in Rust. The goal is to create a production-ready, self-hosted trading system that eliminates the need for third-party bots, gives full control over strategy logic and risk management, and enables safe deployment on a personal cloud droplet — with real money on the line only after rigorous testing through a paper trading mode.

## What Changes

- **New project** — no existing codebase; this is a greenfield Rust workspace.
- A core async trading engine that streams live Binance price data via WebSocket and executes orders.
- A pluggable, trait-based strategy system configured via YAML/JSON files (supports RSI, MACD, and custom indicators).
- A Risk Manager gatekeeper that enforces stop-loss, take-profit, max drawdown, and position-sizing rules before any order reaches the exchange.
- A Telegram bot interface for remote C2: `/start`, `/stop`, `/status`, and emergency overrides.
- A Web Dashboard (Axum backend + React/Vue frontend) with real-time WebSocket log streaming, portfolio overview, trade history, and performance charts.
- Paper Trading mode for strategy validation without real capital exposure.
- A fully automated GitHub Actions CI/CD pipeline (lint → test → cross-compile → deploy) targeting a DigitalOcean Droplet.
- Secrets management via `.env` locally and GitHub Secrets + DigitalOcean environment variables in production.

## Capabilities

### New Capabilities

- `trading-engine`: Core async loop — Binance WebSocket price streaming, order execution, and lifecycle management (start/stop/pause).
- `strategy-layer`: Pluggable trait system for defining trading strategies and indicators; loaded from YAML/JSON config at runtime.
- `risk-manager`: Gatekeeper module that validates every order against configurable risk rules (stop-loss, take-profit, max exposure, max drawdown) before exchange submission.
- `telegram-controller`: Async Telegram bot (teloxide) providing C2 commands and emergency manual overrides.
- `dashboard-api`: Internal REST + WebSocket API (Axum) that exposes portfolio state, trade history, live logs, and config to the dashboard frontend.
- `web-dashboard`: Browser-based dashboard with four tabs — Overview, Operations (live logs + trade history), Strategy & Config (live edits), and Performance (equity curves, win/loss).
- `paper-trading`: Simulation mode that mirrors live trading logic against real market data without executing actual orders.
- `cicd-pipeline`: GitHub Actions workflows for CI (fmt, clippy, test on every push/PR) and CD (cross-compile, scp deploy, systemd restart on merge to main).

### Modified Capabilities

<!-- None — this is a new project with no existing specs. -->

## Impact

- **New Rust workspace** at repo root; no existing code is modified.
- **External dependencies**: Binance REST + WebSocket API, Telegram Bot API, DigitalOcean Droplet (Ubuntu), GitHub Actions runners.
- **Runtime dependencies**: tokio, reqwest/tungstenite (or binance-rs), teloxide, axum, serde, dotenv, SQLite (via sqlx) or PostgreSQL.
- **Security surface**: Binance API keys, Binance secret, Telegram bot token — all must be kept out of source control; enforced via `.gitignore` and CI secret injection.
- **Financial risk**: The Risk Manager capability is the highest-criticality module in the system; failures here can result in real monetary loss.

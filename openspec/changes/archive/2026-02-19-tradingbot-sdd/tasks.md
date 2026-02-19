## 1. Workspace & Shared Infrastructure

- [x] 1.1 Initialize Cargo workspace root `Cargo.toml` listing all member crates: `crates/common`, `crates/engine`, `crates/strategy`, `crates/risk`, `crates/telegram`, `crates/api`, `crates/paper`, `bin/clawbot`
- [x] 1.2 Create stub `Cargo.toml` and `src/lib.rs` (or `main.rs`) for each member crate
- [x] 1.3 Add workspace-level `.gitignore` covering: `.env`, `target/`, `*.db`, `*.db-wal`, `*.db-shm`, `frontend/dist/`, `frontend/node_modules/`
- [x] 1.4 Add `dotenvy` to `common`; implement `Config` struct that loads all env vars at startup and panics with a clear message on missing required values
- [x] 1.5 Define shared types in `common`: `MarketEvent`, `Order`, `OrderSide`, `Signal`, `Position`, `TradingMode` (Live/Paper), `RejectionReason`, `EngineState` (Stopped/Running/Paused/Halted)
- [x] 1.6 Define shared error type in `common` using `thiserror`

## 2. Trading Engine — WebSocket Skeleton

- [x] 2.1 Add `tokio-tungstenite`, `reqwest` (with TLS), and `serde_json` to `crates/engine`
- [x] 2.2 Define `ExchangeClient` trait in `engine` with `pub(crate)` visibility: `async fn submit_order(&self, order: &Order) -> Result<Fill>`; `async fn open_positions(&self) -> Result<Vec<Position>>`
- [x] 2.3 Implement `BinanceClient` struct connecting to Binance WebSocket stream endpoint via `tokio-tungstenite`
- [x] 2.4 Parse Binance aggTrade/kline stream JSON payloads into `MarketEvent` values
- [x] 2.5 Set up `tokio::sync::broadcast` channel for `MarketEvent` distribution to downstream crates
- [x] 2.6 Implement exponential-backoff reconnection loop around the WebSocket connection (start 1s, cap 60s)
- [x] 2.7 After each reconnect, trigger a position audit before resuming market event emission
- [x] 2.8 Implement engine lifecycle state machine driven by `tokio::sync::mpsc` command channel: accept `Start`, `Stop`, `Pause`, `Resume` commands

## 3. Telegram C2 — Hello World

- [x] 3.1 Add `teloxide` (with `dispatching` and macros features) to `crates/telegram`
- [x] 3.2 Define `BotCommands` enum with `/start`, `/stop`, `/status`, `/reset-drawdown` using `#[derive(BotCommands)]`
- [x] 3.3 Implement `TELEGRAM_ALLOWED_USER_IDS` whitelist: parse comma-separated IDs from env, silently ignore all other senders
- [x] 3.4 Implement `/start` handler: send `Start` to engine command channel; reply "Engine started." or "Engine is already running."
- [x] 3.5 Implement `/stop` handler: send `Stop` to engine command channel; reply "Closing open positions and stopping…" then "Engine stopped."
- [x] 3.6 Implement `/status` handler: read engine state snapshot and reply with mode, state, open position count, unrealized PnL, 24h realized PnL
- [x] 3.7 Wire `telegram` crate into `bin/clawbot` `main()` alongside engine; spawn both as concurrent tokio tasks
- [x] 3.8 Smoke-test: confirm bot responds to `/status` in both Stopped and Running states

## 4. Strategy Layer

- [x] 4.1 Define `Strategy` trait in `crates/strategy`: `fn name(&self) -> &str` and `fn evaluate(&self, events: &[MarketEvent]) -> Option<Signal>`
- [x] 4.2 Define `Signal` type: `Buy { pair, quantity }` / `Sell { pair, quantity }`
- [x] 4.3 Implement `RsiIndicator` struct: configurable `period`, `overbought`, `oversold` thresholds; return `None` if fewer than `period` data points
- [x] 4.4 Implement `MacdIndicator` struct: configurable `fast`, `slow`, `signal` periods; emit `Bullish`/`Bearish` crossover events
- [x] 4.5 Define TOML strategy config schema: top-level array of `[[strategy]]` entries with `type`, `pair`, `indicator_params`, optional risk overrides
- [x] 4.6 Implement config file loader using `serde` + `toml`; exit process with error on missing file or unknown `type` value
- [x] 4.7 Implement strategy registry: map config `type` strings to strategy constructors; instantiate all configured strategies at startup
- [x] 4.8 Implement per-pair market event fan-out: each strategy's `evaluate` is called only with events for its configured pair
- [x] 4.9 Unit test: RSI returns correct values on a known price series (verified against reference calculation)
- [x] 4.10 Unit test: RSI returns `None` when data length < period
- [x] 4.11 Unit test: MACD correctly detects bullish and bearish crossovers
- [x] 4.12 Integration test: two strategies on different pairs receive independent, non-overlapping events

## 5. Risk Manager

- [x] 5.1 Define `RiskConfig` struct in `crates/risk`: `stop_loss_pct`, `take_profit_pct`, `max_exposure_per_trade_usd`, `max_drawdown_pct`
- [x] 5.2 Implement `RiskManager`: holds `mpsc::Receiver<Signal>` from strategy layer, `mpsc::Sender<Order>` to executor; signals never bypass this step
- [x] 5.3 Implement `MAX_OPEN_ORDERS` as a compiled-in `const`; reject signals with `HardCeilingReached` when limit is met
- [x] 5.4 Implement max exposure check: reject if `quantity × entry_price > max_exposure_per_trade_usd`, reason `ExposureLimitExceeded`
- [x] 5.5 Implement stop-loss price monitor: for each open position, watch incoming `MarketEvent`; emit market close order when unrealized loss ≥ `stop_loss_pct`
- [x] 5.6 Implement take-profit price monitor: emit market close order when unrealized gain ≥ `take_profit_pct`
- [x] 5.7 Implement max drawdown circuit breaker: track portfolio peak value; enter `HaltedState` and block all orders when drawdown ≥ `max_drawdown_pct`
- [x] 5.8 Implement `RejectionEvent` emission with typed reason; forward events to Telegram alert channel
- [x] 5.9 Implement `/reset-drawdown` handler in `telegram` crate: send reset command to Risk Manager; exit `HaltedState`
- [x] 5.10 Implement proactive Telegram alerts for: `StopLossTriggered`, `TakeProfitTriggered`, `OrderFailed`, drawdown halt entered
- [x] 5.11 Unit test: stop-loss fires at exactly the configured threshold price
- [x] 5.12 Unit test: take-profit fires at exactly the configured threshold price
- [x] 5.13 Unit test: order with notional above limit is rejected with correct reason
- [x] 5.14 Unit test: drawdown halt engages after sufficient loss and blocks subsequent orders
- [x] 5.15 Unit test: hard ceiling rejects the (N+1)th order correctly
- [x] 5.16 Property-based test (proptest): all risk rule evaluations on randomized `f64` price inputs complete without panic or overflow

## 6. Paper Trading

- [x] 6.1 Implement `PaperClient` struct in `crates/paper` satisfying the `ExchangeClient` trait
- [x] 6.2 Implement simulated buy fill: `ask_price × (1 + PAPER_SLIPPAGE_BPS / 10_000)` with `PAPER_SLIPPAGE_BPS` from env (default 10)
- [x] 6.3 Implement simulated sell fill: `bid_price × (1 - PAPER_SLIPPAGE_BPS / 10_000)`
- [x] 6.4 Implement in-memory paper position ledger: track simulated open positions and balance
- [x] 6.5 Implement `TRADING_MODE` env var check in `bin/clawbot`: inject `PaperClient` for `paper`, `BinanceClient` for `live`; exit on invalid value
- [x] 6.6 Integration test: full paper trade flow — market event → strategy signal → risk approval → `PaperClient` fill → position recorded
- [x] 6.7 Integration test: risk rejection in paper mode produces identical events and alerts as it would in live mode

## 7. Order Executor & Persistence

- [x] 7.1 Add `sqlx` (sqlite + runtime-tokio + macros features) to `common`; define `DATABASE_URL` env var
- [x] 7.2 Write migration `0001_initial.sql`: `trades` table — `id`, `pair`, `side`, `entry_price`, `exit_price`, `quantity`, `pnl_usd`, `mode`, `opened_at`, `closed_at`
- [x] 7.3 Write migration `0001_initial.sql`: `positions` table — `id`, `pair`, `side`, `entry_price`, `quantity`, `mode`, `opened_at`
- [x] 7.4 Implement `OrderExecutor`: consumes approved `Order` from Risk Manager mpsc channel, calls `ExchangeClient::submit_order`
- [x] 7.5 On successful fill: write trade record to `trades` table with correct `mode` column value
- [x] 7.6 On buy fill: upsert row in `positions`; on sell fill: remove position row and write closed trade
- [x] 7.7 Implement startup position audit: query `positions` table + `ExchangeClient::open_positions`, log any discrepancies as warnings before entering main loop

## 8. Dashboard API

- [x] 8.1 Add `axum`, `tower-http` (CORS, compression), and `rust-embed` to `crates/api`
- [x] 8.2 Implement bearer token auth extractor: read `DASHBOARD_TOKEN` from env; return HTTP 401 for missing/invalid token on all `/api/*` and `/ws/*` routes
- [x] 8.3 Implement `GET /api/portfolio`: query `positions` table + latest price for each pair; return array of positions with unrealized PnL and totals
- [x] 8.4 Implement `GET /api/trades`: paginated query (`page`, `limit`) with optional `pair` filter, ordered by `closed_at DESC`
- [x] 8.5 Implement `GET /api/performance`: compute equity curve data points, win rate, average win/loss ratio, total PnL, and max drawdown from `trades` table
- [x] 8.6 Implement `GET /api/config`: return current strategy config as JSON
- [x] 8.7 Implement `POST /api/config`: parse and validate the body; on success hot-reload strategy registry; on error return HTTP 422 with field-level details; running config unchanged on error
- [x] 8.8 Implement `GET /ws/logs`: WebSocket upgrade; subscribe to internal log broadcast channel; stream newline-delimited log lines; clean up subscription on disconnect
- [x] 8.9 Implement `GET /healthz`: return `{"status": "ok", "engine": "<state>"}` — no auth required; used by systemd and ops scripts
- [x] 8.10 Embed `frontend/dist/` at compile time via `rust-embed`; serve `/` → `index.html` and static assets via Axum fallback handler

## 9. Web Dashboard Frontend

- [x] 9.1 Scaffold Vue 3 + Vite project under `frontend/`; configure dev proxy to forward `/api` and `/ws` to Axum
- [x] 9.2 Implement login screen: text input for bearer token, store in `sessionStorage`, inject in `Authorization` header on all requests; block all routes until authenticated
- [x] 9.3 Implement Overview tab: fetch `/api/portfolio` every 5 seconds; display total value (USD + BTC), positions table (pair, entry, current, qty, PnL USD, PnL %)
- [x] 9.4 Implement Operations tab — log terminal: connect to `/ws/logs`, display last 500 lines in monospace pre; auto-scroll to bottom; pause scroll when user scrolls up
- [x] 9.5 Implement Operations tab — trade history: paginated table from `/api/trades`; columns: Time, Pair, Side, Entry, Exit, Qty, PnL USD, PnL %; pair filter input
- [x] 9.6 Implement Strategy & Config tab: GET `/api/config` rendered as read-only code block; "Edit" toggles to text area; "Apply" POSTs and shows success toast or inline 422 error
- [x] 9.7 Implement Performance tab: equity curve line chart (Chart.js or ECharts); win/loss ratio chart; stats card (total PnL, max drawdown, win rate, trade count)
- [x] 9.8 Configure Vite `build.outDir` to `../frontend/dist/` relative to frontend root; ensure build succeeds with `vite build`
- [x] 9.9 Verify that `cargo build --release` embeds `frontend/dist/` via `rust-embed` and that `/` serves the dashboard correctly from the compiled binary

## 10. CI/CD Pipeline & Infrastructure

- [x] 10.1 Create `.github/workflows/ci.yml`: trigger on push to any branch and on pull_request targeting `main`; jobs: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`
- [x] 10.2 Add Cargo build cache to CI workflow: `actions/cache` covering `~/.cargo/registry`, `~/.cargo/git`, and `target/`; cache key based on `Cargo.lock` hash
- [x] 10.3 Create `.github/workflows/cd.yml`: trigger on push to `main` only
- [x] 10.4 Add cross-compilation step in CD: install `cross`; run `cross build --release --target x86_64-unknown-linux-musl`
- [x] 10.5 Add Vite frontend build step in CD before cross-compile: `npm ci && npm run build` in `frontend/`
- [x] 10.6 Add all required GitHub Secrets: `DO_SSH_PRIVATE_KEY`, `DROPLET_IP`, `DROPLET_USER`, `BINANCE_API_KEY`, `BINANCE_SECRET`, `TELEGRAM_TOKEN`, `TELEGRAM_ALLOWED_USER_IDS`, `DASHBOARD_TOKEN`, `TRADING_MODE`
- [x] 10.7 Implement deploy step in CD: SSH to Droplet, rename current binary to `clawbot.prev`, `scp` new binary to `/usr/local/bin/clawbot`, set executable bit
- [x] 10.8 Write systemd environment file from GitHub Secrets in deploy step (written to `/etc/clawbot/env`); ensure file is `chmod 600`
- [x] 10.9 Implement post-deploy health check in CD: `systemctl restart clawbot`, poll `GET /healthz` every 2s for up to 30s; fail workflow if service does not reach `active (running)`
- [x] 10.10 Write `deploy/clawbot.service` systemd unit file: `Restart=on-failure`, `RestartSec=10`, `EnvironmentFile=/etc/clawbot/env`, `ExecStart=/usr/local/bin/clawbot`
- [x] 10.11 Document DigitalOcean Droplet bootstrap in `deploy/README.md`: OS version, firewall rules (allow 22, 443; deny all else), systemd unit install steps, initial `.env` setup

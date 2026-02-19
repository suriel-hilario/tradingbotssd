## Context

ClawBot is a greenfield automated crypto trading engine. There is no prior codebase to migrate. The system needs to be production-safe from day one: it will run on real capital after validation in paper-trading mode, and failures in risk management have direct financial consequences. The architecture must therefore prioritize correctness and safety over feature completeness.

The system runs as a single long-lived process on a DigitalOcean Droplet, managed by systemd. All subsystems (engine, bot, API server, strategy runner) execute concurrently within one tokio runtime.

## Goals / Non-Goals

**Goals:**
- A single deployable binary with all subsystems co-located (engine, Telegram bot, Axum API, strategy runner).
- A Cargo workspace (monorepo) with clearly separated crates per capability domain.
- Explicit, typed boundaries between modules — no shared global mutable state outside `Arc<RwLock<...>>` channels.
- The Risk Manager sits on the critical path of every order; it must never be bypassable.
- Paper Trading mode is a first-class runtime switch, not a compile-time flag.
- All secrets loaded exclusively from environment variables; zero hardcoded credentials.
- GitHub Actions CI/CD pipeline that produces a cross-compiled `x86_64-unknown-linux-musl` static binary.

**Non-Goals:**
- Multi-exchange support at this stage (Binance only).
- High-frequency trading (sub-millisecond latency) — this is a swing/day-trade bot.
- A native desktop or mobile app; dashboard is browser-only.
- Horizontal scaling / distributed deployment.
- Backtesting against historical data (future phase).

## Decisions

### D1: Cargo Workspace (Monorepo) over a single crate
**Decision**: Organize as a Cargo workspace with one crate per capability:
```
clawbot/
  Cargo.toml          # workspace root
  crates/
    engine/           # trading-engine
    strategy/         # strategy-layer
    risk/             # risk-manager
    telegram/         # telegram-controller
    api/              # dashboard-api
    paper/            # paper-trading
    common/           # shared types, errors, config
  bin/
    clawbot/          # binary entrypoint, wires all crates
  frontend/           # Vue/React dashboard SPA
```
**Rationale**: Clear compile-time boundaries prevent accidental coupling. Each crate can be tested in isolation. `common` enforces a single source of truth for shared types (e.g., `Order`, `Position`, `MarketEvent`).
**Alternative considered**: Single flat crate with modules — rejected because it makes enforcing the Risk Manager as a mandatory gateway harder; any module could call exchange directly.

---

### D2: tokio broadcast/mpsc channels as the internal message bus
**Decision**: All inter-module communication uses typed `tokio::sync` channels:
- `mpsc` for command flows (Telegram → Engine, e.g., start/stop signals).
- `broadcast` for market events (Engine → Strategy → Risk Manager → Order Executor).
- `mpsc` for order submission (Strategy → Risk Manager → Executor).

**Rationale**: Explicit, async, non-blocking. Avoids shared mutable state. Makes the Risk Manager a mandatory in-line step on every order — strategies send to the risk channel, not directly to the executor.
**Alternative considered**: `Arc<Mutex<...>>` shared state — rejected because it inverts control; modules pull state instead of reacting to events, making the system harder to reason about.

---

### D3: Axum over Actix-Web for the dashboard API
**Decision**: Use `axum` for the REST + WebSocket telemetry API.
**Rationale**: Axum is `tower`-native, integrates cleanly with tokio, and has first-class WebSocket support via `axum::extract::ws`. Its extractor-based design is composable without needing actor-model boilerplate. Aligns with the rest of the async stack.
**Alternative considered**: Actix-Web — mature but uses its own actor runtime, which creates friction when the rest of the system is tokio-native.

---

### D4: SQLite (via sqlx) as the default database
**Decision**: Use SQLite via `sqlx` with compile-time query checking. Provide a config switch to point at a PostgreSQL URL for production if desired.
**Rationale**: SQLite requires zero infrastructure, is embedded, and is more than fast enough for trade history and telemetry at this scale. `sqlx` with `DATABASE_URL` makes migrating to PostgreSQL trivial — the same queries work.
**Alternative considered**: In-memory-only (no persistence) — rejected because trade history and PnL data must survive restarts. PostgreSQL from the start — adds operational overhead (managed DB cost, connection pooling) before it's needed.

---

### D5: Paper Trading as a runtime enum flag, not separate binary
**Decision**: A `TradingMode` enum (`Live | Paper`) is loaded from config at startup. The `engine` crate dispatches through a `dyn ExchangeClient` trait — `BinanceClient` in live mode, `PaperClient` (simulates fills based on live price data) in paper mode.
**Rationale**: Identical code paths for strategy, risk, telemetry, and logging in both modes. Paper mode exercises the full system including risk checks. Switching modes requires only a config change and restart.
**Alternative considered**: Compile-time feature flag — rejected because it makes it impossible to accidentally switch to paper mode without a recompile, but also makes the binary non-deployable as a single artifact.

---

### D6: Binance WebSocket via custom tungstenite/reqwest, not binance-rs
**Decision**: Implement Binance WebSocket connectivity directly using `tokio-tungstenite` for streams and `reqwest` for REST (order placement, account info). Wrap behind the `ExchangeClient` trait.
**Rationale**: `binance-rs` is not async-native and has irregular maintenance. Direct implementation against the Binance API docs gives full control over reconnect logic, stream multiplexing, and error handling — all critical for a bot running 24/7.
**Alternative considered**: `binance-rs` — blocked on async support; wrapping sync code in `tokio::task::spawn_blocking` introduces latency and complexity.

---

### D7: teloxide for Telegram C2
**Decision**: Use `teloxide` with the `dispatching` module for command routing.
**Rationale**: `teloxide` is the de-facto async Telegram framework for Rust. Its command derive macro (`#[derive(BotCommands)]`) maps `/start`, `/stop`, `/status` to handler functions cleanly. Polling mode is sufficient for a single-operator bot.
**Alternative considered**: Raw Telegram API via `reqwest` — too much boilerplate for no gain.

---

### D8: Static musl binary for deployment
**Decision**: CI builds a `x86_64-unknown-linux-musl` static binary via `cross`. No Docker, no runtime dependencies on the Droplet beyond the binary and a `.env` file.
**Rationale**: Musl static linking eliminates glibc version drift between CI runner and Droplet. The binary is self-contained; deployment is a single `scp` + `systemctl restart`.
**Alternative considered**: Docker container — adds operational overhead (image registry, daemon, compose) that isn't justified for a single-process deployment. Cross-compilation without `cross` (using cargo directly) — `cross` handles MUSL toolchain and OpenSSL cross-compilation reliably.

---

### D9: Dashboard frontend as Vue SPA served by Axum
**Decision**: Vue 3 + Vite SPA, served as static files by the same Axum process from an embedded asset path. WebSocket endpoint (`/ws/logs`) streams log lines to the Operations tab in real time.
**Rationale**: Avoids running a separate frontend server or nginx on the Droplet. The Axum server serves `/` → `index.html` and handles `/api/*` + `/ws/*` routes. Build step runs in CI; the compiled `dist/` folder is embedded into the binary via `rust-embed`.
**Alternative considered**: Leptos/Dioxus full-stack Rust — higher complexity, smaller ecosystem for charts/UI libraries. Separate nginx + frontend — added ops overhead.

## Risks / Trade-offs

- **Risk Manager bypass**: If a new module calls the exchange client directly, risk rules are skipped. → Mitigation: The `ExchangeClient` trait is private to the `engine` crate; only the `OrderExecutor` (which is downstream of `RiskManager`) holds a reference. Enforced by Rust's visibility rules.

- **WebSocket reconnection**: Binance disconnects streams every 24h; network blips happen. → Mitigation: The engine's WebSocket loop wraps connection in an exponential-backoff reconnect loop. Missed candles during reconnect trigger a position audit before resuming strategy execution.

- **Telegram as sole C2**: If Telegram is down, there's no way to stop the bot remotely. → Mitigation: A `/healthz` HTTP endpoint allows direct SSH-based `systemctl stop clawbot` as fallback. A `MAX_OPEN_ORDERS` hard ceiling in the Risk Manager limits damage during an outage.

- **SQLite under write contention**: Multiple async tasks writing trade events simultaneously. → Mitigation: `sqlx` uses a connection pool with write serialization. At this event rate (trades per minute, not per second), SQLite is not a bottleneck.

- **Paper mode ≠ live mode slippage**: Paper fills at mid-price underestimate real slippage. → Mitigation: Paper `PaperClient` applies a configurable `slippage_bps` to all simulated fills.

- **Single-process crash = total outage**: All subsystems go down together. → Mitigation: systemd `Restart=on-failure` with a 10s backoff. The bot audits open positions on startup and closes any orphaned ones before resuming.

## Migration Plan

1. **Phase 1 (Skeleton)**: Scaffold workspace, implement `trading-engine` WebSocket loop and `.env` loading. Telegram "hello world". No orders placed.
2. **Phase 2 (Brain)**: Implement `strategy-layer` trait + first strategy. Implement `risk-manager` with full test coverage. Wire paper-trading mode. Run for ≥7 days in paper mode.
3. **Phase 3 (UI & Ops)**: Build `dashboard-api` + Vue frontend. Author GitHub Actions CI/CD. Deploy to Droplet. Enable live trading with minimal exposure.

**Rollback**: `git revert` the deploy commit → CI rebuilds previous binary → scp + systemctl restart. The previous binary is kept on the Droplet as `clawbot.prev` until the next deploy succeeds.

## Open Questions

- **Database choice for production**: Start with SQLite; revisit if DigitalOcean Managed Postgres is warranted after Phase 3.
- **Strategy config format**: YAML vs TOML — TOML is native to Rust tooling; YAML is more readable for non-Rustaceans. Decision deferred to `strategy-layer` spec.
- **Dashboard auth**: The Axum server will be publicly accessible on the Droplet. At minimum, a static bearer token in `.env` should gate `/api/*` and `/ws/*`. Full OAuth is out of scope.
- **Binance Testnet**: Should Phase 1/2 use Binance Testnet instead of paper mode? Testnet requires a separate API key and has lower liquidity simulation. Decision: use paper mode for speed; Testnet for final pre-live validation.

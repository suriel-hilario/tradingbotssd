## ADDED Requirements

### Requirement: Bearer token authentication
All API and WebSocket endpoints SHALL require a `Authorization: Bearer <token>` header. The token SHALL be loaded from the `DASHBOARD_TOKEN` environment variable. Requests without a valid token SHALL receive HTTP 401.

#### Scenario: Valid token provided
- **WHEN** a request includes the correct bearer token
- **THEN** the server processes the request normally

#### Scenario: Missing or invalid token
- **WHEN** a request is missing the `Authorization` header or provides an incorrect token
- **THEN** the server returns HTTP 401 with body `{"error": "unauthorized"}`

---

### Requirement: Portfolio state endpoint
`GET /api/portfolio` SHALL return current portfolio value, open positions, and 24h PnL.

#### Scenario: Positions exist
- **WHEN** the engine has one or more open positions
- **THEN** the response includes an array of positions each with: pair, entry_price, current_price, quantity, unrealized_pnl_usd, unrealized_pnl_pct

#### Scenario: No open positions
- **WHEN** there are no open positions
- **THEN** the response returns an empty `positions` array and correct portfolio totals

---

### Requirement: Trade history endpoint
`GET /api/trades` SHALL return a paginated list of completed trades stored in the database, ordered by close time descending.

#### Scenario: Paginated trade history
- **WHEN** `GET /api/trades?page=1&limit=50` is called
- **THEN** the response includes up to 50 trades and a `total` count for pagination

#### Scenario: Filter by pair
- **WHEN** `GET /api/trades?pair=BTC/USDT` is called
- **THEN** only trades for BTC/USDT are returned

---

### Requirement: Live log WebSocket stream
`GET /ws/logs` SHALL upgrade to a WebSocket connection and push newline-delimited log lines to the client in real time.

#### Scenario: Client connects and receives logs
- **WHEN** an authenticated client connects to `/ws/logs`
- **THEN** it receives new log lines as they are emitted by the engine, strategy, and risk modules

#### Scenario: Client disconnects
- **WHEN** the WebSocket client disconnects
- **THEN** the server cleans up the subscription without error and other connected clients are unaffected

---

### Requirement: Performance metrics endpoint
`GET /api/performance` SHALL return aggregated performance statistics: equity curve data points, win rate, average win/loss ratio, total realized PnL, and max drawdown reached.

#### Scenario: Performance data returned
- **WHEN** at least one completed trade exists in the database
- **THEN** the response includes equity curve as an array of `{timestamp, value}` objects and all statistical fields

#### Scenario: No trades yet
- **WHEN** no trades have been completed
- **THEN** the response returns zeroed statistics and an empty equity curve

---

### Requirement: Config read endpoint
`GET /api/config` SHALL return the current strategy configuration as parsed JSON/TOML. `POST /api/config` SHALL accept an updated configuration, validate it, and hot-reload the strategy layer without restarting the process.

#### Scenario: Successful config update
- **WHEN** a valid config JSON is POSTed to `/api/config`
- **THEN** the server returns HTTP 200 and the strategy layer reloads the new config within 1 second

#### Scenario: Invalid config rejected
- **WHEN** a config with a missing required field or unknown strategy type is POSTed
- **THEN** the server returns HTTP 422 with a descriptive validation error and the running config is unchanged

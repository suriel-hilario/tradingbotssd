## ADDED Requirements

### Requirement: WebSocket market data streaming
The engine SHALL maintain a persistent WebSocket connection to the Binance stream endpoint for each configured trading pair, receiving real-time price and trade events.

#### Scenario: Successful stream connection
- **WHEN** the engine starts with a valid trading pair configured
- **THEN** it connects to the Binance WebSocket stream within 5 seconds and begins emitting `MarketEvent` values to the internal broadcast channel

#### Scenario: Automatic reconnection on disconnect
- **WHEN** the WebSocket connection drops unexpectedly
- **THEN** the engine attempts reconnection using exponential backoff (starting at 1s, capping at 60s) and resumes streaming without operator intervention

#### Scenario: Stream reconnects every 24 hours
- **WHEN** the Binance server closes the stream after its 24-hour session limit
- **THEN** the engine reconnects immediately and performs a position audit before resuming strategy execution

---

### Requirement: Order execution
The engine SHALL execute market and limit orders on Binance via the REST API, exclusively through an `ExchangeClient` trait implementation. No module other than the `OrderExecutor` SHALL hold a reference to `ExchangeClient`.

#### Scenario: Successful buy order
- **WHEN** the `OrderExecutor` receives an approved `Order` from the Risk Manager
- **THEN** it submits the order to Binance, records the fill in the database, and emits a `PositionUpdated` event

#### Scenario: Exchange API error on order submission
- **WHEN** the Binance API returns a non-2xx response for an order
- **THEN** the engine logs the error with full response body, does not retry automatically, and emits an `OrderFailed` event for the Telegram controller to surface

---

### Requirement: Lifecycle management
The engine SHALL support `Start`, `Stop`, and `Pause` commands received via the internal command channel.

#### Scenario: Stop command received while position is open
- **WHEN** a `Stop` command is received and there is an open position
- **THEN** the engine closes the position at market price before halting the main loop

#### Scenario: Pause command received
- **WHEN** a `Pause` command is received
- **THEN** the engine continues streaming market data but suppresses new order signals until a `Resume` command is received

---

### Requirement: Position audit on startup
The engine SHALL query open positions from both the local database and the Binance account on startup, and reconcile any discrepancies before beginning normal operation.

#### Scenario: Orphaned position detected on startup
- **WHEN** the engine starts and finds an open position in the Binance account that is not recorded locally
- **THEN** it records the position in the database and emits a warning log before resuming

#### Scenario: No discrepancies on startup
- **WHEN** the engine starts and local and exchange positions match
- **THEN** it proceeds to the main loop without intervention

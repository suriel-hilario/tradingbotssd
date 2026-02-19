## ADDED Requirements

### Requirement: Runtime mode switch
The system SHALL read `TRADING_MODE` from the environment at startup. Accepted values are `paper` and `live`. Any other value SHALL cause the process to exit with a descriptive error. The mode SHALL NOT be changeable at runtime without a restart.

#### Scenario: Paper mode activated
- **WHEN** `TRADING_MODE=paper` is set and the process starts
- **THEN** the `PaperClient` is injected as the `ExchangeClient` implementation and all order submissions are simulated

#### Scenario: Invalid mode value
- **WHEN** `TRADING_MODE=sandbox` (or any unrecognized value) is set
- **THEN** the process exits with: `ERROR: TRADING_MODE must be 'paper' or 'live'`

---

### Requirement: Simulated order fills
The `PaperClient` SHALL simulate order fills based on current market price. It SHALL apply a configurable `PAPER_SLIPPAGE_BPS` (default: 10 basis points) to all simulated fill prices to model real-world spread.

#### Scenario: Simulated buy fill
- **WHEN** a buy order is submitted to the `PaperClient`
- **THEN** it is filled at `current_ask_price × (1 + slippage_bps / 10_000)` and a synthetic fill confirmation is returned

#### Scenario: Simulated sell fill
- **WHEN** a sell order is submitted to the `PaperClient`
- **THEN** it is filled at `current_bid_price × (1 - slippage_bps / 10_000)` and a synthetic fill confirmation is returned

---

### Requirement: Paper trading state persistence
Simulated positions and trade history in paper mode SHALL be stored in the same database schema as live mode, distinguished by a `mode` column set to `'paper'`.

#### Scenario: Paper trade recorded
- **WHEN** a paper order fill occurs
- **THEN** the trade is written to the database with `mode = 'paper'` and is visible in the dashboard's trade history table

#### Scenario: Live and paper trades coexist without collision
- **WHEN** the system is switched between modes across restarts
- **THEN** live and paper trade records do not overwrite each other and can be queried independently

---

### Requirement: Identical code path for risk and strategy
Paper mode SHALL exercise the Risk Manager, Strategy Layer, Telegram controller alerts, and Dashboard API in exactly the same way as live mode. The only component that differs SHALL be the `ExchangeClient` implementation.

#### Scenario: Risk Manager rejects a paper order
- **WHEN** a simulated order violates the stop-loss rule in paper mode
- **THEN** the rejection is logged, a Telegram alert is sent, and the `PaperClient` is never called — identical behavior to live mode

#### Scenario: Drawdown halt triggers in paper mode
- **WHEN** simulated losses cause max drawdown to be exceeded
- **THEN** the Risk Manager halts new orders and the Telegram controller sends an alert, the same as in live mode

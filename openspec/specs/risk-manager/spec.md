## ADDED Requirements

### Requirement: Mandatory order gateway
Every order signal produced by the strategy layer MUST pass through the Risk Manager before reaching the `OrderExecutor`. The Risk Manager SHALL be the sole entity that forwards approved orders downstream. No strategy or other module SHALL submit orders directly to the exchange client.

#### Scenario: Order approved and forwarded
- **WHEN** the Risk Manager receives a valid signal that passes all configured rules
- **THEN** it forwards the order to the `OrderExecutor` channel with an `Approved` status

#### Scenario: Order rejected
- **WHEN** any risk rule is violated
- **THEN** the order is dropped, a `RejectionEvent` is emitted (with reason), and nothing is sent to the `OrderExecutor`

---

### Requirement: Stop-loss enforcement
The Risk Manager SHALL reject any order or close a position when the unrealized loss on that position reaches or exceeds the configured `stop_loss` percentage.

#### Scenario: Stop-loss triggered on open position
- **WHEN** the current market price causes unrealized loss ≥ `stop_loss` on an open position
- **THEN** the Risk Manager emits a market sell order for the full position size and logs a `StopLossTriggered` event

#### Scenario: New order rejected due to stop-loss proximity
- **WHEN** the entry price of a proposed new order would immediately trigger the stop-loss given current spread
- **THEN** the order is rejected with reason `StopLossProximity`

---

### Requirement: Take-profit enforcement
The Risk Manager SHALL emit a closing order when the unrealized gain on an open position reaches or exceeds the configured `take_profit` percentage.

#### Scenario: Take-profit triggered
- **WHEN** the current market price causes unrealized gain ≥ `take_profit` on an open position
- **THEN** the Risk Manager emits a market sell order for the full position size and logs a `TakeProfitTriggered` event

---

### Requirement: Maximum exposure per trade
The Risk Manager SHALL reject any order where the notional value exceeds `max_exposure_per_trade` (configured as a percentage of total portfolio value or an absolute USD amount).

#### Scenario: Order exceeds max exposure
- **WHEN** the quantity × entry price of a proposed order exceeds `max_exposure_per_trade`
- **THEN** the order is rejected with reason `ExposureLimitExceeded`

#### Scenario: Order within exposure limit
- **WHEN** the notional value is within the configured limit
- **THEN** this rule does not block the order

---

### Requirement: Maximum drawdown circuit breaker
The Risk Manager SHALL halt all new order signals when the portfolio's total drawdown from its peak value reaches or exceeds `max_drawdown` percentage.

#### Scenario: Max drawdown breached
- **WHEN** the portfolio value drops to (peak × (1 - max_drawdown))
- **THEN** the Risk Manager enters `HaltedState`, rejects all new orders, and sends a Telegram alert

#### Scenario: Manual reset after drawdown halt
- **WHEN** an operator sends a `/reset-drawdown` Telegram command after reviewing the situation
- **THEN** the Risk Manager exits `HaltedState` and resumes normal operation

---

### Requirement: Hard order ceiling
The Risk Manager SHALL enforce a `MAX_OPEN_ORDERS` hard ceiling regardless of other config. This value SHALL be compiled-in as a constant (not user-configurable at runtime) to act as a last-resort safeguard.

#### Scenario: Hard ceiling reached
- **WHEN** the number of open orders equals `MAX_OPEN_ORDERS`
- **THEN** all new order signals are rejected with reason `HardCeilingReached` until an order closes

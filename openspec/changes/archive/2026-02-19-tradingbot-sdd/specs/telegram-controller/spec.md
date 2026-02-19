## ADDED Requirements

### Requirement: Operator authentication
The Telegram controller SHALL only respond to commands from a whitelist of authorized Telegram user IDs configured via environment variable (`TELEGRAM_ALLOWED_USER_IDS`). All other messages SHALL be silently ignored.

#### Scenario: Command from authorized user
- **WHEN** an authorized user ID sends a command
- **THEN** the bot processes the command and responds in the same chat

#### Scenario: Command from unauthorized user
- **WHEN** an unknown user ID sends any message or command
- **THEN** the bot does not respond and logs the unauthorized attempt

---

### Requirement: /start command
The controller SHALL send a `Start` command to the engine when an authorized user sends `/start`.

#### Scenario: Engine successfully started
- **WHEN** `/start` is received and the engine is in `Stopped` or `Paused` state
- **THEN** the bot replies "Engine started." and the engine transitions to `Running`

#### Scenario: Engine already running
- **WHEN** `/start` is received and the engine is already in `Running` state
- **THEN** the bot replies "Engine is already running."

---

### Requirement: /stop command
The controller SHALL send a `Stop` command to the engine when an authorized user sends `/stop`. Open positions are closed by the engine before halting (per the trading-engine spec).

#### Scenario: Engine stopped with open positions
- **WHEN** `/stop` is received and there are open positions
- **THEN** the bot replies "Closing open positions and stopping…" and confirms "Engine stopped." after positions are closed

#### Scenario: Engine stopped with no open positions
- **WHEN** `/stop` is received and there are no open positions
- **THEN** the bot replies "Engine stopped." immediately

---

### Requirement: /status command
The controller SHALL reply with a human-readable status summary when `/status` is received.

#### Scenario: Status report
- **WHEN** `/status` is received
- **THEN** the bot replies with: engine state (Running/Stopped/Paused/Halted), trading mode (Live/Paper), number of open positions, total unrealized PnL, and 24h realized PnL

---

### Requirement: /reset-drawdown command
The controller SHALL forward a drawdown reset command to the Risk Manager when `/reset-drawdown` is received.

#### Scenario: Drawdown reset while halted
- **WHEN** `/reset-drawdown` is received and the Risk Manager is in `HaltedState`
- **THEN** the bot replies "Drawdown reset. Engine resuming." and the Risk Manager exits `HaltedState`

#### Scenario: Drawdown reset while not halted
- **WHEN** `/reset-drawdown` is received and the Risk Manager is not halted
- **THEN** the bot replies "No active drawdown halt."

---

### Requirement: Proactive alerts
The controller SHALL send unprompted Telegram messages to all authorized users for the following events: `StopLossTriggered`, `TakeProfitTriggered`, `OrderFailed`, drawdown halt entered, and engine crash/restart detected.

#### Scenario: Stop-loss triggered alert
- **WHEN** the Risk Manager emits `StopLossTriggered`
- **THEN** the bot sends a message to all authorized users: "⚠️ Stop-loss triggered on [pair]. Position closed at [price]."

#### Scenario: Engine crash detected
- **WHEN** the engine task exits with an error
- **THEN** the bot sends "🚨 Engine crashed. Check logs." to all authorized users before the systemd restart takes effect

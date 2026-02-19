## ADDED Requirements

### Requirement: Pluggable Strategy trait
The system SHALL define a `Strategy` trait that all strategy implementations MUST satisfy. The trait SHALL expose at minimum: `fn name(&self) -> &str` and `fn evaluate(&self, events: &[MarketEvent]) -> Option<Signal>`.

#### Scenario: Strategy produces a buy signal
- **WHEN** `evaluate` is called with market events that satisfy the strategy's entry condition
- **THEN** it returns `Some(Signal::Buy { pair, quantity })` without side effects

#### Scenario: Strategy produces no signal
- **WHEN** market conditions do not meet entry or exit criteria
- **THEN** `evaluate` returns `None`

---

### Requirement: Config-driven strategy loading
The system SHALL load strategy configurations from a YAML or TOML file at startup. Each entry SHALL specify the strategy type, trading pair, indicator parameters, and risk overrides.

#### Scenario: Valid config file loaded
- **WHEN** the process starts with a valid strategy config file at the path specified in the environment
- **THEN** all defined strategies are instantiated and registered with the engine before the first market event is processed

#### Scenario: Config file missing
- **WHEN** the config file path is set in the environment but the file does not exist
- **THEN** the process exits with a non-zero status code and logs a descriptive error message

#### Scenario: Unknown strategy type in config
- **WHEN** a config entry specifies a strategy type not registered in the system
- **THEN** the process exits at startup with an error identifying the unrecognized type

---

### Requirement: RSI indicator
The system SHALL provide a built-in RSI (Relative Strength Index) indicator with a configurable period. It SHALL be usable as a component within any strategy.

#### Scenario: RSI crosses above overbought threshold
- **WHEN** the RSI value crosses above the configured `overbought` level (default 70)
- **THEN** the indicator returns an `Overbought` signal

#### Scenario: RSI crosses below oversold threshold
- **WHEN** the RSI value crosses below the configured `oversold` level (default 30)
- **THEN** the indicator returns an `Oversold` signal

#### Scenario: Insufficient data for RSI calculation
- **WHEN** fewer price points than the configured period are available
- **THEN** the indicator returns `None` rather than an incorrect value

---

### Requirement: MACD indicator
The system SHALL provide a built-in MACD indicator with configurable fast, slow, and signal periods.

#### Scenario: MACD line crosses above signal line
- **WHEN** the MACD line crosses above the signal line
- **THEN** the indicator returns a `Bullish` crossover event

#### Scenario: MACD line crosses below signal line
- **WHEN** the MACD line crosses below the signal line
- **THEN** the indicator returns a `Bearish` crossover event

---

### Requirement: Multiple concurrent strategies
The system SHALL support running multiple strategies simultaneously on different trading pairs or with different parameters on the same pair.

#### Scenario: Two strategies active on different pairs
- **WHEN** two strategies are configured for different pairs (e.g., BTC/USDT and ETH/USDT)
- **THEN** each strategy receives only market events for its configured pair and their signals are evaluated independently

## ADDED Requirements

### Requirement: Dashboard authentication gate
The dashboard SHALL display a login screen before any data is shown. The user SHALL enter the bearer token. The token SHALL be stored in `sessionStorage` and sent with every API and WebSocket request.

#### Scenario: Correct token entered
- **WHEN** the user enters the correct bearer token and submits
- **THEN** the dashboard unlocks and navigates to the Overview tab

#### Scenario: Incorrect token entered
- **WHEN** the user enters an incorrect token
- **THEN** an error message is shown and the login screen remains

---

### Requirement: Overview tab
The Overview tab SHALL display current portfolio value in USD and BTC, a list of active positions, and the 24h realized and unrealized PnL. Data SHALL refresh automatically every 5 seconds.

#### Scenario: Active position displayed
- **WHEN** the engine has an open position
- **THEN** the tab shows pair, entry price, current price, quantity, and PnL in both USD and percentage

#### Scenario: Auto-refresh updates values
- **WHEN** 5 seconds elapse since the last fetch
- **THEN** portfolio values update without a manual page reload

---

### Requirement: Operations tab — live log stream
The Operations tab SHALL display a scrolling log terminal that streams real-time log lines via the `/ws/logs` WebSocket. The terminal SHALL show the last 500 lines and auto-scroll to the bottom unless the user has manually scrolled up.

#### Scenario: Log lines stream in real time
- **WHEN** the Operations tab is active and the WebSocket is connected
- **THEN** new log lines appear within 500ms of being emitted by the backend

#### Scenario: User scrolls up to review logs
- **WHEN** the user scrolls up in the log terminal
- **THEN** auto-scroll is paused until the user scrolls back to the bottom

---

### Requirement: Operations tab — trade history table
The Operations tab SHALL display a paginated table of completed trades below the log terminal, with columns: Time, Pair, Side, Entry Price, Exit Price, Quantity, PnL (USD), PnL (%).

#### Scenario: Trade history loads on tab open
- **WHEN** the Operations tab is opened
- **THEN** the first page of trade history is fetched and rendered within 1 second

#### Scenario: Next page navigation
- **WHEN** the user clicks "Next" on the trade history table
- **THEN** the next page of trades is fetched and displayed

---

### Requirement: Strategy & Config tab
The Strategy & Config tab SHALL display the current strategy configuration in a read-only code viewer. An "Edit" button SHALL switch to an editable JSON/TOML text area. Submitting the edited config SHALL call `POST /api/config`.

#### Scenario: Config edit submitted successfully
- **WHEN** the user edits the config and clicks "Apply"
- **THEN** the dashboard shows a success toast and reverts to the read-only view showing the updated config

#### Scenario: Config validation error shown
- **WHEN** the backend returns HTTP 422
- **THEN** the error message is displayed inline below the editor without closing the edit view

---

### Requirement: Performance tab
The Performance tab SHALL display an equity curve line chart, a win/loss ratio bar or pie chart, and a statistics summary card (total PnL, max drawdown, win rate, trade count).

#### Scenario: Equity curve rendered
- **WHEN** trade history contains at least one completed trade
- **THEN** the equity curve is plotted with time on the x-axis and portfolio value on the y-axis

#### Scenario: No trade data available
- **WHEN** no trades have been completed
- **THEN** the tab shows an empty-state message: "No completed trades yet."

-- ClawBot initial schema

CREATE TABLE IF NOT EXISTS positions (
    id          TEXT    PRIMARY KEY,
    pair        TEXT    NOT NULL,
    side        TEXT    NOT NULL CHECK (side IN ('BUY', 'SELL')),
    entry_price REAL    NOT NULL,
    quantity    REAL    NOT NULL,
    mode        TEXT    NOT NULL CHECK (mode IN ('live', 'paper')),
    opened_at   TEXT    NOT NULL   -- ISO-8601 datetime
);

CREATE TABLE IF NOT EXISTS trades (
    id          TEXT    PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    pair        TEXT    NOT NULL,
    side        TEXT    NOT NULL CHECK (side IN ('BUY', 'SELL')),
    entry_price REAL    NOT NULL,
    exit_price  REAL    NOT NULL,
    quantity    REAL    NOT NULL,
    pnl_usd     REAL    NOT NULL,
    mode        TEXT    NOT NULL CHECK (mode IN ('live', 'paper')),
    opened_at   TEXT    NOT NULL,
    closed_at   TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_trades_pair       ON trades (pair);
CREATE INDEX IF NOT EXISTS idx_trades_closed_at  ON trades (closed_at DESC);
CREATE INDEX IF NOT EXISTS idx_trades_mode       ON trades (mode);

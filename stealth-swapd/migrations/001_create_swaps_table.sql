-- Create initial database schema for swap tracking
CREATE TABLE IF NOT EXISTS swaps (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    swap_id BLOB NOT NULL UNIQUE,
    quote_id TEXT NOT NULL,
    direction TEXT NOT NULL CHECK (direction IN ('usdc_to_xmr', 'xmr_to_usdc')),
    usdc_amount INTEGER NOT NULL,
    xmr_amount INTEGER NOT NULL,
    secret_hash BLOB NOT NULL,
    monero_sub_address TEXT NOT NULL,
    alice_solana TEXT,
    state TEXT NOT NULL 
        CHECK (state IN ('quoted', 'locked_usdc', 'locked_xmr', 'redeemed', 'refunded', 'failed')),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    expires_at DATETIME NOT NULL,
    monero_txid TEXT,
    solana_signature TEXT,
    failure_reason TEXT,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Index for lookup by swap_id and state
CREATE INDEX IF NOT EXISTS idx_swap_id ON swaps(swap_id);
CREATE INDEX IF NOT EXISTS idx_state ON swaps(state);
CREATE INDEX IF NOT EXISTS idx_created_at ON swaps(created_at);
CREATE INDEX IF NOT EXISTS idx_expires_at ON swaps(expires_at);

-- Create table for encrypted adaptor secrets
CREATE TABLE IF NOT EXISTS adaptor_secrets (
    swap_id BLOB PRIMARY KEY,
    encrypted_secret BLOB NOT NULL,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (swap_id) REFERENCES swaps(swap_id) ON DELETE CASCADE
);

-- Create table for operational metrics
CREATE TABLE IF NOT EXISTS metrics (
    key TEXT PRIMARY KEY,
    value REAL NOT NULL,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Some initial metrics
INSERT OR IGNORE INTO metrics (key, value) VALUES 
    ('monero_balance_xmr', 0.0),
    ('solana_balance_usdc', 0.0),
    ('relayer_fees_earned_usdc', 0.0);
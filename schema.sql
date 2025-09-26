CREATE TABLE IF NOT EXISTS raw_transfers (
    tx_hash TEXT PRIMARY KEY,
    block_number INTEGER,
    from_address TEXT,
    to_address TEXT,
    value TEXT,
    timestamp INTEGER
);

CREATE TABLE IF NOT EXISTS net_flows (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    cumulative_in TEXT DEFAULT '0',
    cumulative_out TEXT DEFAULT '0',
    net_flow TEXT DEFAULT '0',
    last_updated INTEGER
);

INSERT OR IGNORE INTO net_flows (id, cumulative_in, cumulative_out, net_flow, last_updated)
VALUES (1, '0', '0', '0', 0);

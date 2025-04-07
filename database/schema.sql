-- schema
CREATE SCHEMA IF NOT EXISTS  apestrong;

-- Keeps track of last seen event signature for each pool
-- This is used to avoid reprocessing events that have already been processed
CREATE TABLE IF NOT EXISTS apestrong.last_signatures (
    pool_address TEXT PRIMARY KEY,
    signature TEXT NOT NULL,
    dex_type TEXT NOT NULL,
    last_updated TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);


-- Base table for common fields
CREATE TABLE IF NOT EXISTS apestrong.orca_whirlpool_events (
    id SERIAL PRIMARY KEY,
    signature VARCHAR(88) NOT NULL UNIQUE,
    whirlpool VARCHAR(44) NOT NULL,
    event_type VARCHAR(32) NOT NULL,
    version INT NOT NULL DEFAULT 1,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_orca_whirlpool_events_whirlpool_timestamp ON apestrong.orca_whirlpool_events (whirlpool, timestamp);

-- Traded events
CREATE TABLE IF NOT EXISTS apestrong.orca_traded_events (
    event_id INT PRIMARY KEY REFERENCES apestrong.orca_whirlpool_events(id) ON DELETE CASCADE,
    a_to_b BOOLEAN NOT NULL,
    pre_sqrt_price BIGINT NOT NULL,
    post_sqrt_price BIGINT NOT NULL,
    input_amount BIGINT NOT NULL,
    output_amount BIGINT NOT NULL,
    input_transfer_fee BIGINT NOT NULL,
    output_transfer_fee BIGINT NOT NULL,
    lp_fee BIGINT NOT NULL,
    protocol_fee BIGINT NOT NULL
);

-- Liquidity Increased events
CREATE TABLE IF NOT EXISTS apestrong.orca_liquidity_increased_events (
    event_id INT PRIMARY KEY REFERENCES apestrong.orca_whirlpool_events(id) ON DELETE CASCADE,
    position VARCHAR(44) NOT NULL,
    tick_lower_index INT NOT NULL,
    tick_upper_index INT NOT NULL,
    liquidity BIGINT NOT NULL,
    token_a_amount BIGINT NOT NULL,
    token_b_amount BIGINT NOT NULL,
    token_a_transfer_fee BIGINT NOT NULL,
    token_b_transfer_fee BIGINT NOT NULL
);

-- Liquidity Decreased events
CREATE TABLE IF NOT EXISTS apestrong.orca_liquidity_decreased_events (
    event_id INT PRIMARY KEY REFERENCES apestrong.orca_whirlpool_events(id) ON DELETE CASCADE,
    position VARCHAR(44) NOT NULL,
    tick_lower_index INT NOT NULL,
    tick_upper_index INT NOT NULL,
    liquidity BIGINT NOT NULL,
    token_a_amount BIGINT NOT NULL,
    token_b_amount BIGINT NOT NULL,
    token_a_transfer_fee BIGINT NOT NULL,
    token_b_transfer_fee BIGINT NOT NULL
);

-- Create views for convenience (optional)
CREATE OR REPLACE VIEW apestrong.v_orca_whirlpool_traded AS
SELECT
    e.id, e.signature, e.whirlpool, e.timestamp,
    t.a_to_b, t.pre_sqrt_price, t.post_sqrt_price, t.input_amount, t.output_amount,
    t.input_transfer_fee, t.output_transfer_fee, t.lp_fee, t.protocol_fee
FROM
    apestrong.orca_whirlpool_events e
JOIN
    apestrong.orca_traded_events t ON e.id = t.event_id
WHERE
    e.event_type = 'Traded';

CREATE OR REPLACE VIEW apestrong.v_orca_whirlpool_liquidity_increased AS
SELECT 
    e.id, e.signature, e.whirlpool, e.timestamp, 
    l.position, l.tick_lower_index, l.tick_upper_index, 
    l.liquidity, l.token_a_amount, l.token_b_amount,
    l.token_a_transfer_fee, l.token_b_transfer_fee
FROM 
    apestrong.orca_whirlpool_events e
JOIN 
    apestrong.orca_liquidity_increased_events l ON e.id = l.event_id
WHERE 
    e.event_type = 'liquidity_increased';

CREATE OR REPLACE VIEW apestrong.v_orca_whirlpool_liquidity_decreased AS
SELECT 
    e.id, e.signature, e.whirlpool, e.timestamp, 
    l.position, l.tick_lower_index, l.tick_upper_index, 
    l.liquidity, l.token_a_amount, l.token_b_amount,
    l.token_a_transfer_fee, l.token_b_transfer_fee
FROM 
   apestrong.orca_whirlpool_events e
JOIN 
    apestrong.orca_liquidity_decreased_events l ON e.id = l.event_id
WHERE 
    e.event_type = 'liquidity_decreased';
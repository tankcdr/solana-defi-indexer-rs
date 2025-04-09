-- Ensure the schema exists (assuming apestrong schema is already created from previous script)
-- If not, uncomment the following line:
-- CREATE SCHEMA IF NOT EXISTS apestrong;

-- Base table for common fields of Orca Whirlpool events
CREATE TABLE IF NOT EXISTS apestrong.orca_whirlpool_events (
    id SERIAL PRIMARY KEY,
    signature VARCHAR(88) NOT NULL UNIQUE,
    whirlpool VARCHAR(44) NOT NULL,
    event_type VARCHAR(32) NOT NULL,
    version INT NOT NULL DEFAULT 1,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for performance on whirlpool and timestamp
CREATE INDEX IF NOT EXISTS idx_orca_whirlpool_events_whirlpool_timestamp 
    ON apestrong.orca_whirlpool_events (whirlpool, timestamp);

-- Table for Traded events, inheriting from base events
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

-- Table for Liquidity Increased events, inheriting from base events
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

-- Table for Liquidity Decreased events, inheriting from base events
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

-- View for Traded events
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

-- View for Liquidity Increased events
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

-- View for Liquidity Decreased events
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
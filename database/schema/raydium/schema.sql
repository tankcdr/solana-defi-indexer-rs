

-- Base table for common fields of Raydium CLMM events
CREATE TABLE IF NOT EXISTS apestrong.raydium_clmm_events (
    id SERIAL PRIMARY KEY,
    signature VARCHAR(88) NOT NULL UNIQUE,
    pool VARCHAR(44) NOT NULL,
    event_type VARCHAR(32) NOT NULL,
    version INT NOT NULL DEFAULT 1,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for performance on whirlpool and timestamp
CREATE INDEX IF NOT EXISTS idx_raydium_clmm_events_pool_timestamp 
    ON apestrong.raydium_clmm_events (pool, timestamp);

-- Table for CreatePersonalPosition events, inheriting from base events
CREATE TABLE IF NOT EXISTS apestrong.raydium_clmm_create_position_events (
    event_id INT PRIMARY KEY REFERENCES apestrong.raydium_clmm_events(id) ON DELETE CASCADE,
    minter VARCHAR(44) NOT NULL,
    nft_owner VARCHAR(44) NOT NULL,
    output_amount BIGINT NOT NULL,
    tick_lower_index INTEGER NOT NULL,
    tick_upper_index INTEGER NOT NULL,
    liquidity NUMERIC(39, 0) NOT NULL,
    deposit_amount_0 NUMERIC NOT NULL,
    deposit_amount_1 NUMERIC NOT NULL,
    deposit_amount_0_transfer_fee NUMERIC NOT NULL,
    deposit_amount_1_transfer_fee NUMERIC NOT NULL
);

-- Table for Liquidity Increased events, inheriting from base events
CREATE TABLE IF NOT EXISTS apestrong.raydium_clmm_liquidity_increased_events (
    event_id INT PRIMARY KEY REFERENCES apestrong.raydium_clmm_events(id) ON DELETE CASCADE,
    position_nft_mint VARCHAR(44) NOT NULL,
    liquidity NUMERIC(39, 0) NOT NULL,
    amount_0 NUMERIC NOT NULL,
    amount_1 NUMERIC NOT NULL,
    amount_0_transfer_fee NUMERIC NOT NULL,
    amount_1_transfer_fee NUMERIC NOT NULL,
);

-- Table for Liquidity Decreased events, inheriting from base events
CREATE TABLE IF NOT EXISTS apestrong.raydium_clmm_liquidity_decreased_events (
    event_id INT PRIMARY KEY REFERENCES apestrong.raydium_clmm_events(id) ON DELETE CASCADE,
    position_nft_mint VARCHAR(44) NOT NULL,
    liquidity NUMERIC(39, 0) NOT NULL,
    decrease_amount_0 NUMERIC NOT NULL,
    decrease_amount_1 NUMERIC NOT NULL,
    fee_amount_0 NUMERIC NOT NULL,
    fee_amount_1 NUMERIC NOT NULL,
    transfer_fee_0 NUMERIC NOT NULL,
    transfer_fee_1 NUMERIC NOT NULL,
);

-- View for Traded events
CREATE OR REPLACE VIEW apestrong.v_raydium_clmm_create_position AS
SELECT
    e.id, e.signature, e.whirlpool, e.timestamp,
    t.a_to_b, t.pre_sqrt_price, t.post_sqrt_price, t.input_amount, t.output_amount,
    t.input_transfer_fee, t.output_transfer_fee, t.lp_fee, t.protocol_fee
FROM
    apestrong.raydium_clmm_events e
JOIN
    apestrong.raydium_clmm_create_position_events t ON e.id = t.event_id
WHERE
    e.event_type = 'CreatePersonalPosition';

-- View for Liquidity Increased events
CREATE OR REPLACE VIEW apestrong.v_raydium_clmm_liquidity_increased AS
SELECT 
    e.id, e.signature, e.whirlpool, e.timestamp, 
    l.position, l.tick_lower_index, l.tick_upper_index, 
    l.liquidity, l.token_a_amount, l.token_b_amount,
    l.token_a_transfer_fee, l.token_b_transfer_fee
FROM 
    apestrong.raydium_clmm_events e
JOIN 
    apestrong.orca_liquidity_increased_events l ON e.id = l.event_id
WHERE 
    e.event_type = 'liquidity_increased';

-- View for Liquidity Decreased events
CREATE OR REPLACE VIEW apestrong.v_raydium_clmm_liquidity_decreased AS
SELECT 
    e.id, e.signature, e.whirlpool, e.timestamp, 
    l.position, l.tick_lower_index, l.tick_upper_index, 
    l.liquidity, l.token_a_amount, l.token_b_amount,
    l.token_a_transfer_fee, l.token_b_transfer_fee
FROM 
    apestrong.raydium_clmm_events e
JOIN 
    apestrong.orca_liquidity_decreased_events l ON e.id = l.event_id
WHERE 
    e.event_type = 'liquidity_decreased';
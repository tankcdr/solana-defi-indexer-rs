-- common schema entities

CREATE SCHEMA IF NOT EXISTS  apestrong;

-- Creating dex type to limit allowed values 
CREATE TYPE dex_type AS ENUM ('orca', 'raydium');

-- Keeps track of last seen event signature for each pool
-- This is used to avoid reprocessing events that have already been processed
CREATE TABLE IF NOT EXISTS apestrong.last_signatures (
    pool_address VARCHAR(44) PRIMARY KEY,
    signature VARCHAR(88) NOT NULL,
    dex dex_type NOT NULL,
    last_updated TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Add index for performance
CREATE INDEX IF NOT EXISTS idx_last_signatures_dex ON apestrong.last_signatures(dex);
CREATE INDEX IF NOT EXISTS idx_last_signatures_last_updated ON apestrong.last_signatures(last_updated);

-- Registered Orca Whirlpool pools that we are interested in monitoring
CREATE TABLE IF NOT EXISTS apestrong.subscribed_pools (
    pool_mint VARCHAR(44) PRIMARY KEY,
    pool_name VARCHAR(128),  
    dex dex_type NOT NULL,
    token_a_mint VARCHAR(44) REFERENCES apestrong.token_metadata(mint),
    token_b_mint VARCHAR(44) REFERENCES apestrong.token_metadata(mint),
    last_updated TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_subscribed_pools_token_a ON apestrong.subscribed_pools(token_a_mint);
CREATE INDEX IF NOT EXISTS idx_subscribed_pools_token_b ON apestrong.subscribed_pools(token_b_mint);

-- Token Metadata related to the pools we are subscribing too
CREATE TABLE IF NOT EXISTS apestrong.token_metadata (
    mint VARCHAR(44) PRIMARY KEY,
    token_name VARCHAR(20),
    symbol VARCHAR(10), 
    decimals INT NOT NULL,
    last_updated TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Add timestamp index for time-based queries
CREATE INDEX IF NOT EXISTS idx_subscribed_pools_last_updated ON apestrong.subscribed_pools(last_updated);
CREATE INDEX IF NOT EXISTS idx_token_metadata_last_updated ON apestrong.token_metadata(last_updated);
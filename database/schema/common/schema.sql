-- Create the schema if it doesn’t exist
CREATE SCHEMA IF NOT EXISTS apestrong;

-- Create the dex_type ENUM conditionally
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_type t
        JOIN pg_namespace n ON n.oid = t.typnamespace
        WHERE t.typname = 'dex_type'
        AND n.nspname = 'apestrong'
    ) THEN
        CREATE TYPE apestrong.dex_type AS ENUM ('orca', 'raydium');
    END IF;
END;
$$;

-- Create the last_signatures table to track processed event signatures
CREATE TABLE IF NOT EXISTS apestrong.last_signatures (
    pool_address VARCHAR(44) PRIMARY KEY,
    signature VARCHAR(88) NOT NULL,
    dex apestrong.dex_type NOT NULL,
    last_updated TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Add indexes for performance on last_signatures
CREATE INDEX IF NOT EXISTS idx_last_signatures_dex ON apestrong.last_signatures(dex);
CREATE INDEX IF NOT EXISTS idx_last_signatures_last_updated ON apestrong.last_signatures(last_updated);

-- Create the token_metadata table for pool-related token info
CREATE TABLE IF NOT EXISTS apestrong.token_metadata (
    mint VARCHAR(44) PRIMARY KEY,
    token_name VARCHAR(20),
    symbol VARCHAR(10),
    decimals INT NOT NULL,
    last_updated TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Add index for time-based queries on token_metadata
CREATE INDEX IF NOT EXISTS idx_token_metadata_last_updated ON apestrong.token_metadata(last_updated);

-- Create the subscribed_pools table for monitoring Orca Whirlpool pools
CREATE TABLE IF NOT EXISTS apestrong.subscribed_pools (
    pool_mint VARCHAR(44) PRIMARY KEY,
    pool_name VARCHAR(128),
    dex apestrong.dex_type NOT NULL,
    token_a_mint VARCHAR(44) REFERENCES apestrong.token_metadata(mint),
    token_b_mint VARCHAR(44) REFERENCES apestrong.token_metadata(mint),
    last_updated TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Add indexes for performance on subscribed_pools
CREATE INDEX IF NOT EXISTS idx_subscribed_pools_token_a ON apestrong.subscribed_pools(token_a_mint);
CREATE INDEX IF NOT EXISTS idx_subscribed_pools_token_b ON apestrong.subscribed_pools(token_b_mint);
CREATE INDEX IF NOT EXISTS idx_subscribed_pools_last_updated ON apestrong.subscribed_pools(last_updated);
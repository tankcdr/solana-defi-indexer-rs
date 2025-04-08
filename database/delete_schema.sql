-- Drop statements for all tables and views
DROP VIEW IF EXISTS apestrong.v_orca_whirlpool_traded;
DROP VIEW IF EXISTS apestrong.v_orca_whirlpool_liquidity_increased;
DROP VIEW IF EXISTS apestrong.v_orca_whirlpool_liquidity_decreased;
DROP TABLE IF EXISTS apestrong.orca_traded_events;
DROP TABLE IF EXISTS apestrong.orca_liquidity_increased_events;
DROP TABLE IF EXISTS apestrong.orca_liquidity_decreased_events;
DROP TABLE IF EXISTS apestrong.orca_whirlpool_events;
DROP TABLE IF EXISTS apestrong.last_signatures;
DROP TABLE IF EXISTS apestrong.orca_whirlpool_pools;

-- Add statements for future DEXes here
-- DROP TABLE IF EXISTS apestrong.raydium_events;
-- etc.
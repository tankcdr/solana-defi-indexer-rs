DROP INDEX IF EXISTS idx_subscribed_pools_token_a;
DROP INDEX IF EXISTS idx_subscribed_pools_token_b;
DROP TABLE IF EXISTS apestrong.subscribed_pools;
DROP INDEX IF EXISTS idx_token_metadata_last_updated;
DROP INDEX IF EXISTS idx_subscribed_pools_last_updated;
DROP TABLE IF EXISTS apestrong.token_metadata;
DROP INDEX IF EXISTS idx_last_signatures_dex;
DROP INDEX IF EXISTS idx_last_signatures_last_updated;
DROP TABLE IF EXISTS apestrong.last_signatures;
DROP TYPE IF EXISTS apestrong.dex_type;
DROP SCHEMA IF EXISTS apestrong;
use anyhow::{ Context, Result };
use sqlx::{ PgPool, Postgres, Transaction, Row };
use std::collections::HashSet;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use crate::db::common::Repository;
use crate::models::orca::whirlpool::{
    OrcaWhirlpoolEvent,
    OrcaWhirlpoolTradedEventRecord,
    OrcaWhirlpoolLiquidityIncreasedEventRecord,
    OrcaWhirlpoolLiquidityDecreasedEventRecord,
    OrcaWhirlpoolPool,
};

/// Repository for Orca Whirlpool event database operations
pub struct OrcaWhirlpoolRepository {
    pool: PgPool,
}

impl OrcaWhirlpoolRepository {
    /// Create a new repository instance
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert a base Orca Whirlpool event
    async fn insert_base_event<'a>(
        &self,
        tx: &mut Transaction<'a, Postgres>,
        event: &OrcaWhirlpoolEvent
    ) -> Result<i32> {
        let row = sqlx
            ::query(
                "INSERT INTO apestrong.orca_whirlpool_events (signature, whirlpool, event_type, version) VALUES ($1, $2, $3, $4) RETURNING id"
            )
            .bind(&event.signature)
            .bind(&event.whirlpool)
            .bind(&event.event_type)
            .bind(event.version)
            .fetch_one(&mut **tx).await
            .context("Failed to insert base Orca Whirlpool event")?;

        let id: i32 = row.get("id");
        Ok(id)
    }

    /// Insert a traded event into the database
    pub async fn insert_traded_event(&self, event: OrcaWhirlpoolTradedEventRecord) -> Result<i32> {
        let mut tx = self.pool.begin().await?;

        // Insert the base event
        let event_id = self.insert_base_event(&mut tx, &event.base).await?;

        // Insert the traded-specific data
        sqlx
            ::query(
                "INSERT INTO apestrong.orca_traded_events (event_id, a_to_b, pre_sqrt_price, post_sqrt_price, input_amount, output_amount, input_transfer_fee, output_transfer_fee, lp_fee, protocol_fee) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
            )
            .bind(event_id)
            .bind(event.data.a_to_b)
            .bind(event.data.pre_sqrt_price)
            .bind(event.data.post_sqrt_price)
            .bind(event.data.input_amount)
            .bind(event.data.output_amount)
            .bind(event.data.input_transfer_fee)
            .bind(event.data.output_transfer_fee)
            .bind(event.data.lp_fee)
            .bind(event.data.protocol_fee)
            .execute(&mut *tx).await
            .context("Failed to insert Orca Whirlpool traded event")?;

        tx.commit().await?;
        Ok(event_id)
    }

    /// Insert a liquidity increased event into the database
    pub async fn insert_liquidity_increased_event(
        &self,
        event: OrcaWhirlpoolLiquidityIncreasedEventRecord
    ) -> Result<i32> {
        let mut tx = self.pool.begin().await?;

        // Insert the base event
        let event_id = self.insert_base_event(&mut tx, &event.base).await?;

        // Insert the liquidity data
        sqlx
            ::query(
                "INSERT INTO apestrong.orca_liquidity_increased_events (event_id, position, tick_lower_index, tick_upper_index, liquidity, token_a_amount, token_b_amount, token_a_transfer_fee, token_b_transfer_fee) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
            )
            .bind(event_id)
            .bind(&event.data.position)
            .bind(event.data.tick_lower_index)
            .bind(event.data.tick_upper_index)
            .bind(event.data.liquidity)
            .bind(event.data.token_a_amount)
            .bind(event.data.token_b_amount)
            .bind(event.data.token_a_transfer_fee)
            .bind(event.data.token_b_transfer_fee)
            .execute(&mut *tx).await
            .context("Failed to insert Orca Whirlpool liquidity increased event")?;

        tx.commit().await?;
        Ok(event_id)
    }

    /// Insert a liquidity decreased event into the database
    pub async fn insert_liquidity_decreased_event(
        &self,
        event: OrcaWhirlpoolLiquidityDecreasedEventRecord
    ) -> Result<i32> {
        let mut tx = self.pool.begin().await?;

        // Insert the base event
        let event_id = self.insert_base_event(&mut tx, &event.base).await?;

        // Insert the liquidity data
        sqlx
            ::query(
                "INSERT INTO apestrong.orca_liquidity_decreased_events (event_id, position, tick_lower_index, tick_upper_index, liquidity, token_a_amount, token_b_amount, token_a_transfer_fee, token_b_transfer_fee) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
            )
            .bind(event_id)
            .bind(&event.data.position)
            .bind(event.data.tick_lower_index)
            .bind(event.data.tick_upper_index)
            .bind(event.data.liquidity)
            .bind(event.data.token_a_amount)
            .bind(event.data.token_b_amount)
            .bind(event.data.token_a_transfer_fee)
            .bind(event.data.token_b_transfer_fee)
            .execute(&mut *tx).await
            .context("Failed to insert Orca Whirlpool liquidity decreased event")?;

        tx.commit().await?;
        Ok(event_id)
    }

    /// Get recent trade volume for a specific pool
    pub async fn get_recent_trade_volume(&self, pool_address: &str, hours: i64) -> Result<i64> {
        let row = sqlx
            ::query(
                "SELECT COALESCE(SUM(t.input_amount), 0) as volume FROM apestrong.orca_whirlpool_events e JOIN apestrong.orca_traded_events t ON e.id = t.event_id WHERE e.whirlpool = $1 AND e.event_type = 'traded' AND e.timestamp > NOW() - INTERVAL '1 hour' * $2"
            )
            .bind(pool_address)
            .bind(hours)
            .fetch_one(&self.pool).await
            .context("Failed to get recent trade volume")?;

        let volume: Option<i64> = row.get("volume");
        Ok(volume.unwrap_or(0))
    }

    //
    // Pool Management Methods (from orca_pools.rs)
    //

    /// Get all pools from the database
    pub async fn get_all_pools(&self) -> Result<Vec<OrcaWhirlpoolPool>> {
        let rows = sqlx
            ::query(
                "SELECT p.pool_mint as whirlpool, 
                        p.token_a_mint as token_mint_a, 
                        p.token_b_mint as token_mint_b, 
                        p.pool_name,
                        ta.token_name as token_name_a, 
                        tb.token_name as token_name_b,
                        ta.decimals as decimals_a, 
                        tb.decimals as decimals_b
                 FROM apestrong.subscribed_pools p
                 LEFT JOIN apestrong.token_metadata ta ON p.token_a_mint = ta.mint
                 LEFT JOIN apestrong.token_metadata tb ON p.token_b_mint = tb.mint
                 WHERE p.dex = 'orca'"
            )
            .fetch_all(&self.pool).await
            .context("Failed to fetch Orca Whirlpool pools")?;

        let pools = rows
            .into_iter()
            .map(|row| OrcaWhirlpoolPool {
                whirlpool: row.get("whirlpool"),
                token_mint_a: row.get("token_mint_a"),
                token_mint_b: row.get("token_mint_b"),
                token_name_a: row.get("token_name_a"),
                token_name_b: row.get("token_name_b"),
                pool_name: row.get("pool_name"),
                decimals_a: row.get("decimals_a"),
                decimals_b: row.get("decimals_b"),
            })
            .collect();

        Ok(pools)
    }

    /// Get a specific pool by address
    pub async fn get_pool(&self, whirlpool_address: &str) -> Result<Option<OrcaWhirlpoolPool>> {
        let row = sqlx
            ::query(
                "SELECT p.pool_mint as whirlpool, 
                        p.token_a_mint as token_mint_a, 
                        p.token_b_mint as token_mint_b, 
                        p.pool_name,
                        ta.token_name as token_name_a, 
                        tb.token_name as token_name_b,
                        ta.decimals as decimals_a, 
                        tb.decimals as decimals_b
                 FROM apestrong.subscribed_pools p
                 LEFT JOIN apestrong.token_metadata ta ON p.token_a_mint = ta.mint
                 LEFT JOIN apestrong.token_metadata tb ON p.token_b_mint = tb.mint
                 WHERE p.pool_mint = $1 AND p.dex = 'orca'"
            )
            .bind(whirlpool_address)
            .fetch_optional(&self.pool).await
            .context("Failed to fetch Orca Whirlpool pool")?;

        match row {
            Some(row) =>
                Ok(
                    Some(OrcaWhirlpoolPool {
                        whirlpool: row.get("whirlpool"),
                        token_mint_a: row.get("token_mint_a"),
                        token_mint_b: row.get("token_mint_b"),
                        token_name_a: row.get("token_name_a"),
                        token_name_b: row.get("token_name_b"),
                        pool_name: row.get("pool_name"),
                        decimals_a: row.get("decimals_a"),
                        decimals_b: row.get("decimals_b"),
                    })
                ),
            None => Ok(None),
        }
    }

    /// Add or update a pool
    pub async fn upsert_pool(&self, pool: &OrcaWhirlpoolPool) -> Result<()> {
        // Start a transaction
        let mut tx = self.pool.begin().await?;

        // First, ensure token metadata exists for both tokens
        for (mint, name, decimals, is_a) in [
            (&pool.token_mint_a, &pool.token_name_a, pool.decimals_a, true),
            (&pool.token_mint_b, &pool.token_name_b, pool.decimals_b, false),
        ] {
            sqlx
                ::query(
                    "INSERT INTO apestrong.token_metadata (mint, token_name, decimals, last_updated)
                 VALUES ($1, $2, $3, NOW())
                 ON CONFLICT (mint) DO UPDATE SET
                 token_name = EXCLUDED.token_name,
                 decimals = EXCLUDED.decimals,
                 last_updated = NOW()"
                )
                .bind(mint)
                .bind(name)
                .bind(decimals)
                .execute(&mut *tx).await
                .context(
                    format!("Failed to insert token metadata for token_{}", if is_a {
                        "a"
                    } else {
                        "b"
                    })
                )?;
        }

        // Then insert or update the pool
        sqlx
            ::query(
                "INSERT INTO apestrong.subscribed_pools
             (pool_mint, pool_name, dex, token_a_mint, token_b_mint, last_updated)
             VALUES ($1, $2, 'orca', $3, $4, NOW())
             ON CONFLICT (pool_mint) DO UPDATE SET
             pool_name = EXCLUDED.pool_name,
             dex = EXCLUDED.dex,
             token_a_mint = EXCLUDED.token_a_mint,
             token_b_mint = EXCLUDED.token_b_mint,
             last_updated = NOW()"
            )
            .bind(&pool.whirlpool)
            .bind(&pool.pool_name)
            .bind(&pool.token_mint_a)
            .bind(&pool.token_mint_b)
            .execute(&mut *tx).await
            .context("Failed to insert or update pool")?;

        // Commit the transaction
        tx.commit().await?;

        Ok(())
    }

    /// Check if a pool exists
    pub async fn pool_exists(&self, whirlpool_address: &str) -> Result<bool> {
        let exists: (bool,) = sqlx
            ::query_as(
                "SELECT EXISTS(SELECT 1 FROM apestrong.subscribed_pools WHERE pool_mint = $1 AND dex = 'orca')"
            )
            .bind(whirlpool_address)
            .fetch_one(&self.pool).await
            .context("Failed to check if pool exists")?;

        Ok(exists.0)
    }

    /// Get all pool pubkeys as a HashSet
    pub async fn get_pool_pubkeys(&self) -> Result<HashSet<Pubkey>> {
        let rows = sqlx
            ::query(
                "SELECT pool_mint as whirlpool FROM apestrong.subscribed_pools WHERE dex = 'orca'"
            )
            .fetch_all(&self.pool).await
            .context("Failed to fetch pool addresses")?;

        let mut pool_set = HashSet::new();
        for row in rows {
            let address: String = row.get("whirlpool");
            if let Ok(pubkey) = Pubkey::from_str(&address) {
                pool_set.insert(pubkey);
            }
        }

        Ok(pool_set)
    }

    /// Get pool addresses with priority fallback: Provided list > Database > Default
    ///
    /// This function fetches pool addresses based on the following priority:
    /// 1. The provided list of addresses (if any)
    /// 2. Pool addresses stored in the database
    /// 3. A default pool address as a fallback
    pub async fn get_pools_with_fallback(
        &self,
        provided_pools: Option<&Vec<String>>,
        default_pool: &str
    ) -> Result<HashSet<Pubkey>> {
        // 1. If provided addresses exist and are not empty, use them
        if let Some(addresses) = provided_pools {
            if !addresses.is_empty() {
                let mut pubkeys = HashSet::new();
                for addr in addresses {
                    let pubkey = Pubkey::from_str(addr).context(
                        format!("Invalid Solana address: {}", addr)
                    )?;
                    pubkeys.insert(pubkey);
                }
                return Ok(pubkeys);
            }
        }

        // 2. Try to get pools from the database
        let db_pools = self.get_pool_pubkeys().await?;
        if !db_pools.is_empty() {
            return Ok(db_pools);
        }

        // 3. Use the default pool as fallback
        let mut pubkeys = HashSet::new();
        pubkeys.insert(
            Pubkey::from_str(default_pool).context("Failed to parse default Orca pool address")?
        );

        Ok(pubkeys)
    }
}

impl Repository for OrcaWhirlpoolRepository {
    fn pool(&self) -> &PgPool {
        &self.pool
    }
}

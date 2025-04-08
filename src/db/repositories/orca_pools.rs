use anyhow::{ Context, Result };
use sqlx::{ PgPool, Row };
use std::collections::HashSet;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Orca Whirlpool Pool record
#[derive(Debug, Clone)]
pub struct OrcaWhirlpoolPool {
    pub whirlpool: String,
    pub token_mint_a: String,
    pub token_mint_b: String,
    pub token_name_a: Option<String>,
    pub token_name_b: Option<String>,
    pub pool_name: Option<String>,
    pub decimals_a: i32,
    pub decimals_b: i32,
}

/// Repository for Orca Whirlpool pool database operations
pub struct OrcaWhirlpoolPoolRepository {
    pool: PgPool,
}

impl OrcaWhirlpoolPoolRepository {
    /// Create a new repository instance
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get all pools from the database
    pub async fn get_all_pools(&self) -> Result<Vec<OrcaWhirlpoolPool>> {
        let rows = sqlx
            ::query(
                "SELECT whirlpool, token_mint_a, token_mint_b, token_name_a, token_name_b,
                 pool_name, decimals_a, decimals_b
                 FROM apestrong.orca_whirlpool_pools"
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
                "SELECT whirlpool, token_mint_a, token_mint_b, token_name_a, token_name_b,
                 pool_name, decimals_a, decimals_b
                 FROM apestrong.orca_whirlpool_pools
                 WHERE whirlpool = $1"
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
        sqlx
            ::query(
                "INSERT INTO apestrong.orca_whirlpool_pools
             (whirlpool, token_mint_a, token_mint_b, token_name_a, token_name_b, pool_name,
              decimals_a, decimals_b, last_updated)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
             ON CONFLICT (whirlpool) DO UPDATE SET
               token_mint_a = EXCLUDED.token_mint_a,
               token_mint_b = EXCLUDED.token_mint_b,
               token_name_a = EXCLUDED.token_name_a,
               token_name_b = EXCLUDED.token_name_b,
               pool_name = EXCLUDED.pool_name,
               decimals_a = EXCLUDED.decimals_a,
               decimals_b = EXCLUDED.decimals_b,
               last_updated = NOW()"
            )
            .bind(&pool.whirlpool)
            .bind(&pool.token_mint_a)
            .bind(&pool.token_mint_b)
            .bind(&pool.token_name_a)
            .bind(&pool.token_name_b)
            .bind(&pool.pool_name)
            .bind(pool.decimals_a)
            .bind(pool.decimals_b)
            .execute(&self.pool).await
            .context("Failed to insert or update pool")?;

        Ok(())
    }

    /// Check if a pool exists
    pub async fn pool_exists(&self, whirlpool_address: &str) -> Result<bool> {
        let exists: (bool,) = sqlx
            ::query_as(
                "SELECT EXISTS(SELECT 1 FROM apestrong.orca_whirlpool_pools WHERE whirlpool = $1)"
            )
            .bind(whirlpool_address)
            .fetch_one(&self.pool).await
            .context("Failed to check if pool exists")?;

        Ok(exists.0)
    }

    /// Get all pool pubkeys as a HashSet
    pub async fn get_pool_pubkeys(&self) -> Result<HashSet<Pubkey>> {
        let rows = sqlx
            ::query("SELECT whirlpool FROM apestrong.orca_whirlpool_pools")
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

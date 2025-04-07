use anyhow::{ Context, Result };
use solana_sdk::pubkey::Pubkey;
use sqlx::{ PgPool, Row };
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{ Arc, Mutex };

/// Enum-based store to contain both memory and database implementations
pub enum SignatureStore {
    InMemory(InMemorySignatureStore),
    Database(DbSignatureStore),
}

impl SignatureStore {
    /// Store the last processed signature for a pool
    pub fn update_signature(&self, pool: &Pubkey, signature: String, dex_type: &str) -> Result<()> {
        match self {
            Self::InMemory(store) => store.update_signature(pool, signature, dex_type),
            Self::Database(store) => store.update_signature(pool, signature, dex_type),
        }
    }

    /// Retrieve the last processed signature for a pool
    pub fn get_signature(&self, pool: &Pubkey, dex_type: &str) -> Result<Option<String>> {
        match self {
            Self::InMemory(store) => store.get_signature(pool, dex_type),
            Self::Database(store) => store.get_signature(pool, dex_type),
        }
    }

    /// Check if we have a stored signature for this pool
    pub fn has_signature(&self, pool: &Pubkey, dex_type: &str) -> Result<bool> {
        match self {
            Self::InMemory(store) => store.has_signature(pool, dex_type),
            Self::Database(store) => store.has_signature(pool, dex_type),
        }
    }

    /// Get all tracked pools for a specific DEX
    pub fn get_tracked_pools(&self, dex_type: &str) -> Result<Vec<Pubkey>> {
        match self {
            Self::InMemory(store) => store.get_tracked_pools(dex_type),
            Self::Database(store) => store.get_tracked_pools(dex_type),
        }
    }
}

/// In-memory implementation of signature storage
pub struct InMemorySignatureStore {
    // Key: (pool_pubkey, dex_type)
    signatures: Arc<Mutex<HashMap<(Pubkey, String), String>>>,
}

impl InMemorySignatureStore {
    pub fn new() -> Self {
        Self {
            signatures: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn update_signature(&self, pool: &Pubkey, signature: String, dex_type: &str) -> Result<()> {
        let mut store = self.signatures
            .lock()
            .map_err(|_| anyhow::anyhow!("Failed to acquire lock"))?;
        store.insert((*pool, dex_type.to_string()), signature);
        Ok(())
    }

    pub fn get_signature(&self, pool: &Pubkey, dex_type: &str) -> Result<Option<String>> {
        let store = self.signatures.lock().map_err(|_| anyhow::anyhow!("Failed to acquire lock"))?;
        Ok(store.get(&(*pool, dex_type.to_string())).cloned())
    }

    pub fn has_signature(&self, pool: &Pubkey, dex_type: &str) -> Result<bool> {
        let store = self.signatures.lock().map_err(|_| anyhow::anyhow!("Failed to acquire lock"))?;
        Ok(store.contains_key(&(*pool, dex_type.to_string())))
    }

    pub fn get_tracked_pools(&self, dex_type: &str) -> Result<Vec<Pubkey>> {
        let store = self.signatures.lock().map_err(|_| anyhow::anyhow!("Failed to acquire lock"))?;

        let mut pools = Vec::new();
        for ((pool, stored_dex), _) in store.iter() {
            if stored_dex == dex_type {
                pools.push(*pool);
            }
        }

        Ok(pools)
    }
}

/// Database-backed implementation of signature storage
pub struct DbSignatureStore {
    db_pool: PgPool,
}

impl DbSignatureStore {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    /// Asynchronous wrapper to update a signature in the database
    pub async fn update_signature_async(
        &self,
        pool: &Pubkey,
        signature: String,
        dex_type: &str
    ) -> Result<()> {
        sqlx
            ::query(
                r#"
            INSERT INTO apestrong.last_signatures (pool_address, signature, dex_type, last_updated)
            VALUES ($1, $2, $3, NOW())
            ON CONFLICT (pool_address) 
            DO UPDATE SET 
                signature = $2,
                dex_type = $3,
                last_updated = NOW()
            "#
            )
            .bind(pool.to_string())
            .bind(&signature)
            .bind(dex_type)
            .execute(&self.db_pool).await
            .with_context(|| format!("Failed to update signature in database for pool {}", pool))?;

        Ok(())
    }

    /// Asynchronous wrapper to get a signature from the database
    pub async fn get_signature_async(
        &self,
        pool: &Pubkey,
        dex_type: &str
    ) -> Result<Option<String>> {
        let result = sqlx
            ::query(
                r#"
            SELECT signature 
            FROM apestrong.last_signatures 
            WHERE pool_address = $1 AND dex_type = $2
            "#
            )
            .bind(pool.to_string())
            .bind(dex_type)
            .fetch_optional(&self.db_pool).await
            .with_context(|| format!("Failed to query signature from database for pool {}", pool))?;

        // Extract the signature value
        match result {
            Some(row) =>
                Ok(
                    Some(
                        row
                            .try_get("signature")
                            .with_context(||
                                format!("Failed to extract signature field from result for pool {}", pool)
                            )?
                    )
                ),
            None => Ok(None),
        }
    }

    /// Asynchronous wrapper to check if a signature exists in the database
    pub async fn has_signature_async(&self, pool: &Pubkey, dex_type: &str) -> Result<bool> {
        let result = sqlx
            ::query(
                r#"
            SELECT 1 
            FROM apestrong.last_signatures 
            WHERE pool_address = $1 AND dex_type = $2
            "#
            )
            .bind(pool.to_string())
            .bind(dex_type)
            .fetch_optional(&self.db_pool).await
            .with_context(||
                format!("Failed to check signature existence in database for pool {}", pool)
            )?;

        Ok(result.is_some())
    }

    /// Asynchronous wrapper to get all tracked pools for a specific DEX
    pub async fn get_tracked_pools_async(&self, dex_type: &str) -> Result<Vec<Pubkey>> {
        let rows = sqlx
            ::query(
                r#"
            SELECT pool_address 
            FROM apestrong.last_signatures 
            WHERE dex_type = $1
            "#
            )
            .bind(dex_type)
            .fetch_all(&self.db_pool).await
            .with_context(||
                format!("Failed to query tracked pools from database for DEX type {}", dex_type)
            )?;

        let mut pools = Vec::with_capacity(rows.len());
        for row in rows {
            let address: String = row
                .try_get("pool_address")
                .with_context(|| "Failed to extract pool_address field from result")?;

            match Pubkey::from_str(&address) {
                Ok(pubkey) => pools.push(pubkey),
                Err(e) => {
                    eprintln!("Failed to parse pool address {}: {}", address, e);
                    // Continue with next row
                }
            }
        }

        Ok(pools)
    }

    pub fn update_signature(&self, pool: &Pubkey, signature: String, dex_type: &str) -> Result<()> {
        // Create a runtime and block on the async function
        let rt = tokio::runtime::Runtime::new().context("Failed to create Tokio runtime")?;

        rt.block_on(self.update_signature_async(pool, signature, dex_type))
    }

    pub fn get_signature(&self, pool: &Pubkey, dex_type: &str) -> Result<Option<String>> {
        let rt = tokio::runtime::Runtime::new().context("Failed to create Tokio runtime")?;

        rt.block_on(self.get_signature_async(pool, dex_type))
    }

    pub fn has_signature(&self, pool: &Pubkey, dex_type: &str) -> Result<bool> {
        let rt = tokio::runtime::Runtime::new().context("Failed to create Tokio runtime")?;

        rt.block_on(self.has_signature_async(pool, dex_type))
    }

    pub fn get_tracked_pools(&self, dex_type: &str) -> Result<Vec<Pubkey>> {
        let rt = tokio::runtime::Runtime::new().context("Failed to create Tokio runtime")?;

        rt.block_on(self.get_tracked_pools_async(dex_type))
    }
}

/// Type of signature store to create
pub enum SignatureStoreType {
    InMemory,
    Database,
}

/// Create a signature store of the specified type
pub fn create_signature_store(
    store_type: SignatureStoreType,
    db_pool: Option<PgPool>
) -> Result<SignatureStore> {
    match store_type {
        SignatureStoreType::InMemory => {
            Ok(SignatureStore::InMemory(InMemorySignatureStore::new()))
        }
        SignatureStoreType::Database => {
            let pool = db_pool.ok_or_else(||
                anyhow::anyhow!("Database pool required for DB signature store")
            )?;
            Ok(SignatureStore::Database(DbSignatureStore::new(pool)))
        }
    }
}

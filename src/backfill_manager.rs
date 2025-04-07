use anyhow::{ Context, Result };
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_client::GetConfirmedSignaturesForAddress2Config,
    rpc_config::RpcTransactionConfig,
};
use solana_sdk::{ commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signature };
use solana_transaction_status::UiTransactionEncoding;
use std::str::FromStr;

use crate::db::signature_store::SignatureStore;

/// Configuration for backfill operations
pub struct BackfillConfig {
    /// Solana RPC URL
    pub rpc_url: String,
    /// Maximum number of signatures to fetch per request
    pub max_signatures_per_request: usize,
    /// How far back to look for transactions on initial backfill
    pub initial_backfill_slots: u64,
    /// DEX type identifier (e.g., "orca", "raydium")
    pub dex_type: String,
}

impl Default for BackfillConfig {
    fn default() -> Self {
        Self {
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            max_signatures_per_request: 100,
            initial_backfill_slots: 10_000, // Approx 4 hours of slots
            dex_type: "orca".to_string(),
        }
    }
}

/// Manages backfilling missed transactions
pub struct BackfillManager {
    config: BackfillConfig,
    signature_store: SignatureStore,
    rpc_client: RpcClient,
}

impl BackfillManager {
    /// Create a new BackfillManager
    pub fn new(config: BackfillConfig, signature_store: SignatureStore) -> Self {
        let rpc_client = RpcClient::new_with_commitment(
            config.rpc_url.clone(),
            CommitmentConfig::confirmed()
        );

        Self {
            config,
            signature_store,
            rpc_client,
        }
    }

    /// Perform initial backfill for a pool to establish baseline data
    pub async fn initial_backfill_for_pool(&self, pool: &Pubkey) -> Result<Vec<Signature>> {
        println!("Performing initial backfill for pool {}", pool);

        let signatures = self.rpc_client.get_signatures_for_address_with_config(
            pool,
            GetConfirmedSignaturesForAddress2Config {
                limit: Some(self.config.max_signatures_per_request),
                before: None,
                until: None,
                commitment: Some(CommitmentConfig::confirmed()),
            }
        ).await?;

        let mut result = Vec::new();

        if let Some(last_info) = signatures.last() {
            // Store the oldest signature as our start point
            self.signature_store.update_signature(
                pool,
                last_info.signature.clone(),
                &self.config.dex_type
            ).await?;
        }

        if let Some(first_info) = signatures.first() {
            // Process from newest to oldest
            for info in &signatures {
                let signature = Signature::from_str(&info.signature)?;
                result.push(signature);
            }

            // Store the newest signature for future backfills
            self.signature_store.update_signature(
                pool,
                first_info.signature.clone(),
                &self.config.dex_type
            ).await?;
        }

        println!(
            "Initial backfill complete for pool {}, fetched {} signatures",
            pool,
            result.len()
        );
        Ok(result)
    }

    /// Backfill missed transactions for a pool since the last processed signature
    pub async fn backfill_since_last_signature(&self, pool: &Pubkey) -> Result<Vec<Signature>> {
        let last_signature = match
            self.signature_store.get_signature(pool, &self.config.dex_type).await?
        {
            Some(sig) => sig,
            None => {
                println!("No last signature for pool {}, performing initial backfill", pool);
                return self.initial_backfill_for_pool(pool).await;
            }
        };

        println!("Backfilling pool {} since signature {}", pool, last_signature);

        // Convert the last_signature string to a Signature
        let until_signature = Signature::from_str(&last_signature)?;

        let signatures = self.rpc_client.get_signatures_for_address_with_config(
            pool,
            GetConfirmedSignaturesForAddress2Config {
                limit: Some(self.config.max_signatures_per_request),
                before: None,
                until: Some(until_signature),
                commitment: Some(CommitmentConfig::confirmed()),
            }
        ).await?;

        let mut result = Vec::new();

        if signatures.is_empty() {
            println!("No new transactions since last signature");
            return Ok(result);
        }

        println!("Found {} new transactions since last signature", signatures.len());

        // Process from newest to oldest
        for info in &signatures {
            let signature = Signature::from_str(&info.signature)?;
            result.push(signature);
        }

        // Update the newest signature
        if let Some(first_info) = signatures.first() {
            self.signature_store.update_signature(
                pool,
                first_info.signature.clone(),
                &self.config.dex_type
            ).await?;
        }

        Ok(result)
    }

    /// Get all pools this DEX is tracking
    pub async fn get_tracked_pools(&self) -> Result<Vec<Pubkey>> {
        self.signature_store.get_tracked_pools(&self.config.dex_type).await
    }

    /// Check if we have a signature for this pool
    pub async fn has_signature_for_pool(&self, pool: &Pubkey) -> Result<bool> {
        self.signature_store.has_signature(pool, &self.config.dex_type).await
    }

    /// Fetch transaction details for a signature
    pub async fn fetch_transaction(
        &self,
        signature: &Signature
    ) -> Result<solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta> {
        self.rpc_client
            .get_transaction_with_config(signature, RpcTransactionConfig {
                encoding: Some(UiTransactionEncoding::JsonParsed),
                commitment: Some(CommitmentConfig::confirmed()),
                max_supported_transaction_version: Some(0),
            }).await
            .with_context(|| format!("Failed to fetch transaction for signature {}", signature))
    }
}

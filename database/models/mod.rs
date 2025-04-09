// Re-export DEX-specific modules
pub mod orca;

// Common traits and structures
use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use futures::future::BoxFuture;

// Common token information structure
pub struct TokenInfo {
    pub mint: Pubkey,
    pub decimals: u8,
    pub symbol: String,
    pub name: String,
}

// Common pool record structure
pub struct PoolRecord {
    pub pool_address: Pubkey,
    pub pool_name: String,
    pub dex: String,
    pub token_a: TokenInfo,
    pub token_b: TokenInfo,
}

// DEX processor trait - to be implemented by each DEX model
pub trait DexProcessor {
    // Use BoxFuture instead of async fn for trait objects
    fn process_pool<'a>(
        &'a self,
        rpc_client: &'a RpcClient,
        pool_pubkey: &'a Pubkey,
        metadata_program_id: &'a Pubkey,
        token_cache: &'a mut HashMap<Pubkey, TokenInfo>,
        verbose: bool
    ) -> BoxFuture<'a, Result<PoolRecord>>;
}

// Helper method for token info cloning
impl Clone for TokenInfo {
    fn clone(&self) -> Self {
        TokenInfo {
            mint: self.mint,
            decimals: self.decimals,
            symbol: self.symbol.clone(),
            name: self.name.clone(),
        }
    }
}

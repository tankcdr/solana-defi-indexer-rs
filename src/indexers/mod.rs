pub mod dex_indexer;
pub mod orca;

pub use dex_indexer::*;
pub use orca::*;

// Future protocol indexers will be added here
// pub mod raydium;
// pub use raydium::*;

use anyhow::Result;

/// Public helper function to start any DEX indexer
///
/// This provides a clean public API for starting indexers without having to
/// create public wrapper methods for each implementation
pub async fn start_indexer<T: DexIndexer + Send + Sync>(
    indexer: &T,
    rpc_url: &str,
    ws_url: &str
) -> Result<()> {
    // Call the trait method
    indexer.start(rpc_url, ws_url).await
}

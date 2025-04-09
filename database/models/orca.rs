use anyhow::{ Context, Result };
use borsh::BorshDeserialize;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use mpl_token_metadata::accounts::Metadata as MplMetadata;
use futures::future::BoxFuture;

use super::{ DexProcessor, PoolRecord, TokenInfo };

// Orca Whirlpool account data layout
#[derive(BorshDeserialize, Debug)]
#[allow(dead_code)]
pub struct WhirlpoolData {
    pub whirlpools_config: Pubkey, // 32 bytes
    pub whirlpool_bump: [u8; 1], // 1 byte
    pub tick_spacing: u16, // 2 bytes
    pub tick_spacing_seed: [u8; 2], // 2 bytes
    pub fee_rate: u16, // 2 bytes
    pub protocol_fee_rate: u16, // 2 bytes
    pub liquidity: u128, // 16 bytes
    pub sqrt_price: u128, // 16 bytes
    pub tick_current_index: i32, // 4 bytes
    pub protocol_fee_owed_a: u64, // 8 bytes
    pub protocol_fee_owed_b: u64, // 8 bytes
    pub token_mint_a: Pubkey, // 32 bytes
    pub token_vault_a: Pubkey, // 32 bytes
    pub fee_growth_global_a: u128, // 16 bytes
    pub token_mint_b: Pubkey, // 32 bytes
    pub token_vault_b: Pubkey, // 32 bytes
    pub fee_growth_global_b: u128, // 16 bytes
    pub reward_last_updated_timestamp: u64, // 8 bytes
    pub reward_infos: [RewardInfo; 3], // 3 * 128 = 384 bytes
}

#[derive(BorshDeserialize, Debug)]
#[allow(dead_code)]
pub struct RewardInfo {
    pub mint: Pubkey, // 32 bytes
    pub vault: Pubkey, // 32 bytes
    pub authority: Pubkey, // 32 bytes
    pub emissions_per_second_x64: u128, // 16 bytes
    pub growth_global_x64: u128, // 16 bytes
}

pub struct OrcaProcessor;

impl DexProcessor for OrcaProcessor {
    fn process_pool<'a>(
        &'a self,
        rpc_client: &'a RpcClient,
        pool_pubkey: &'a Pubkey,
        metadata_program_id: &'a Pubkey,
        token_cache: &'a mut HashMap<Pubkey, TokenInfo>,
        verbose: bool
    ) -> BoxFuture<'a, Result<PoolRecord>> {
        Box::pin(async move {
            // Fetch the pool account data
            let pool_account = rpc_client
                .get_account_with_commitment(pool_pubkey, CommitmentConfig::confirmed()).await?
                .value.context("Pool account not found")?;

            // Debug information
            if verbose {
                println!("Account data length: {} bytes", pool_account.data.len());
            }

            // Try to deserialize the pool data (skip first 8 bytes which is the anchor discriminator)
            let pool_data = WhirlpoolData::try_from_slice(&pool_account.data[8..]).context(
                "Failed to deserialize pool data"
            )?;

            if verbose {
                println!("Found pool with the following data:");
                println!("  Token Mint A: {}", pool_data.token_mint_a);
                println!("  Token Mint B: {}", pool_data.token_mint_b);
                println!("  Tick Spacing: {}", pool_data.tick_spacing);
                println!("  Fee Rate: {}", pool_data.fee_rate);
            }

            // Fetch token information, using cache if available
            let token_a_info = if let Some(info) = token_cache.get(&pool_data.token_mint_a) {
                info.clone()
            } else {
                let info = fetch_token_info(
                    rpc_client,
                    &pool_data.token_mint_a,
                    metadata_program_id
                ).await.context("Failed to fetch Token A information")?;
                token_cache.insert(pool_data.token_mint_a, info.clone());
                info
            };

            let token_b_info = if let Some(info) = token_cache.get(&pool_data.token_mint_b) {
                info.clone()
            } else {
                let info = fetch_token_info(
                    rpc_client,
                    &pool_data.token_mint_b,
                    metadata_program_id
                ).await.context("Failed to fetch Token B information")?;
                token_cache.insert(pool_data.token_mint_b, info.clone());
                info
            };

            // Create pool record
            let pool_record = PoolRecord {
                pool_address: *pool_pubkey,
                pool_name: format!("{} / {}", token_a_info.symbol, token_b_info.symbol),
                dex: String::from("orca"),
                token_a: token_a_info,
                token_b: token_b_info,
            };

            // Display token information
            if verbose {
                println!("  Token A Symbol: {}", pool_record.token_a.symbol);
                println!("  Token A Decimals: {}", pool_record.token_a.decimals);
                println!("  Token B Symbol: {}", pool_record.token_b.symbol);
                println!("  Token B Decimals: {}", pool_record.token_b.decimals);
            }

            Ok(pool_record)
        })
    }
}

// Helper function to deserialize metadata
fn deserialize_metadata(data: &[u8]) -> Result<MplMetadata, anyhow::Error> {
    let data_owned = data.to_vec();
    let mut slice = data_owned.as_slice();
    let metadata: MplMetadata = MplMetadata::deserialize(&mut slice).map_err(|e|
        anyhow::anyhow!("Failed to deserialize metadata: {}", e)
    )?;

    Ok(metadata)
}

// Fetch token information (mint details, metadata, decimals)
pub async fn fetch_token_info(
    rpc_client: &RpcClient,
    token_mint: &Pubkey,
    metadata_program_id: &Pubkey
) -> Result<TokenInfo> {
    // Fetch token mint account
    let token_account = rpc_client
        .get_account_with_commitment(token_mint, CommitmentConfig::confirmed()).await?
        .value.context(format!("Token mint account not found for {}", token_mint))?;

    // Extract decimals
    let decimals = if token_account.data.len() >= 45 {
        token_account.data[44] // Offset for decimals in token mint data
    } else {
        println!("WARNING: Cannot extract decimals for token {}, using default value of 6", token_mint);
        6 // Default value for most tokens
    };

    // Try to fetch metadata
    let mut symbol = String::new();
    let mut name = String::new();
    let (metadata_pda, _bump) = Pubkey::find_program_address(
        &[b"metadata", &metadata_program_id.to_bytes(), &token_mint.to_bytes()],
        metadata_program_id
    );

    match
        rpc_client.get_account_with_commitment(&metadata_pda, CommitmentConfig::confirmed()).await
    {
        Ok(account_result) => {
            if let Some(metadata_account) = account_result.value {
                match deserialize_metadata(&metadata_account.data) {
                    Ok(metadata) => {
                        symbol = metadata.symbol.trim_end_matches('\0').to_string();
                        name = metadata.name.trim_end_matches('\0').to_string();
                    }
                    Err(e) => {
                        println!(
                            "Warning: Failed to deserialize metadata for {}: {}",
                            token_mint,
                            e
                        );
                    }
                }
            }
        }
        Err(e) => {
            println!("Warning: Failed to fetch metadata account for {}: {}", token_mint, e);
        }
    }

    Ok(TokenInfo {
        mint: *token_mint,
        decimals,
        symbol,
        name,
    })
}

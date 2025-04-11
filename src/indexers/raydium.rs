use anyhow::{ Context, Result };
use borsh::BorshDeserialize;
use solana_client::rpc_response::RpcLogsResponse;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashSet;
use std::str::FromStr;
use sqlx::PgPool;
use chrono::Utc;
use async_trait::async_trait;

use crate::db::repositories::raydium::RaydiumRepository;
use crate::db::signature_store::SignatureStore;
use crate::backfill_manager::BackfillManager;
use crate::models::raydium::amm::{
    TRADED_EVENT_DISCRIMINATOR as AMM_TRADED_DISCRIMINATOR,
    // Add other AMM discriminators as needed
};
use crate::models::raydium::clmm::{
    CLMM_CREATE_PERSONAL_POSITION_DISCRIMINATOR,
    CLMM_LIQUIDITY_INCREASED_DISCRIMINATOR,
    CLMM_LIQUIDITY_DECREASED_DISCRIMINATOR,
    RaydiumCLMMEventType,
    RaydiumCLMMCreatePositionEvent,
    RaydiumCLMMIncreaseLiquidityEvent,
    RaydiumCLMMDecreaseLiquidityEvent,
    RaydiumCLMMEvent,
    RaydiumCLMMCreatePositionRecord,
    RaydiumCLMMIncreaseLiquidityRecord,
    RaydiumCLMMDecreaseLiquidityRecord,
    RaydiumCLMMCreatePostionEventRecord,
    RaydiumCLMMIncreaseLiquidityEventRecord,
    RaydiumCLMMDecreaseLiquidityEventRecord,
};
use crate::utils::logging;
use crate::indexers::dex_indexer::{ DexIndexer, ConnectionConfig };

// Default pools for fallback
const DEFAULT_RAYDIUM_AMM_POOL: &str = ""; // Replace with an appropriate default AMM pool
const DEFAULT_RAYDIUM_CLMM_POOL: &str = ""; // Replace with an appropriate default CLMM pool
const DEX_NAME: &str = "raydium";

/// The pool type for distinguishing between AMM and CLMM pools
#[derive(Debug, Clone, PartialEq)]
pub enum RaydiumPoolType {
    AMM,
    CLMM,
}

/// Represents a parsed event from Raydium logs
#[derive(Debug)]
pub enum RaydiumParsedEvent {
    // AMM Events
    AmmTraded(String), // Just signature for now, will expand with proper struct
    // Additional AMM events as needed

    // CLMM Events
    ClmmCreatePosition(RaydiumCLMMCreatePositionEvent, String, Pubkey), // Event, signature, and pool
    ClmmIncreaseLiquidity(RaydiumCLMMIncreaseLiquidityEvent, String, Pubkey),
    ClmmDecreaseLiquidity(RaydiumCLMMDecreaseLiquidityEvent, String, Pubkey),
}

/// Raydium combined indexer for both AMM and CLMM
pub struct RaydiumIndexer {
    repository: RaydiumRepository,
    amm_pool_pubkeys: HashSet<Pubkey>,
    clmm_pool_pubkeys: HashSet<Pubkey>,
    signature_store: SignatureStore,
    backfill_manager: BackfillManager,
    connection_config: ConnectionConfig,
}

impl RaydiumIndexer {
    // Helper methods for event logging

    /// Log details about a CLMM create position event
    fn log_create_position_event(&self, event: &RaydiumCLMMCreatePositionEvent, pool: &Pubkey) {
        self.log_event_processed(
            "CreatePosition",
            &pool.to_string(),
            &format!(
                "Minter: {}, NFT Owner: {}, Liquidity: {}",
                event.minter,
                event.nft_owner,
                event.liquidity
            )
        );
    }

    /// Log details about a CLMM increase liquidity event
    fn log_increase_liquidity_event(
        &self,
        event: &RaydiumCLMMIncreaseLiquidityEvent,
        pool: &Pubkey
    ) {
        self.log_event_processed(
            "IncreaseLiquidity",
            &pool.to_string(),
            &format!(
                "Position: {}, Amount0: {}, Amount1: {}",
                event.position_nft_mint,
                event.amount_0,
                event.amount_1
            )
        );
    }

    /// Log details about a CLMM decrease liquidity event
    fn log_decrease_liquidity_event(
        &self,
        event: &RaydiumCLMMDecreaseLiquidityEvent,
        pool: &Pubkey
    ) {
        self.log_event_processed(
            "DecreaseLiquidity",
            &pool.to_string(),
            &format!(
                "Position: {}, Amount0: {}, Amount1: {}",
                event.position_nft_mint,
                event.decrease_amount_0,
                event.decrease_amount_1
            )
        );
    }

    /// Create a base CLMM event record
    fn create_clmm_base_event(
        &self,
        signature: &str,
        pool: &Pubkey,
        event_type: RaydiumCLMMEventType
    ) -> RaydiumCLMMEvent {
        RaydiumCLMMEvent {
            id: 0, // Will be set by database
            signature: signature.to_string(),
            pool: pool.to_string(),
            event_type: event_type.to_string(),
            version: 1,
            timestamp: Utc::now(),
        }
    }

    /// Get all monitored pools (both AMM and CLMM)
    // Helper method that combines both AMM and CLMM pools
    fn all_pool_pubkeys(&self) -> HashSet<Pubkey> {
        let mut all_pools = HashSet::new();
        all_pools.extend(self.amm_pool_pubkeys.iter().cloned());
        all_pools.extend(self.clmm_pool_pubkeys.iter().cloned());
        all_pools
    }

    /// Determine if a pool is an AMM pool
    fn is_amm_pool(&self, pool: &Pubkey) -> bool {
        self.amm_pool_pubkeys.contains(pool)
    }

    /// Determine if a pool is a CLMM pool
    fn is_clmm_pool(&self, pool: &Pubkey) -> bool {
        self.clmm_pool_pubkeys.contains(pool)
    }

    /// Parse logs for CLMM events
    async fn parse_clmm_events(&self, log: &RpcLogsResponse) -> Result<Vec<RaydiumParsedEvent>> {
        // Check if the log contains relevant CLMM event keywords
        let contains_relevant_events = log.logs
            .iter()
            .any(|line| {
                line.contains("CreatePosition") ||
                    line.contains("IncreaseLiquidity") ||
                    line.contains("DecreaseLiquidity")
            });

        if !contains_relevant_events {
            return Ok(Vec::new());
        }

        let mut events = Vec::new();

        // Process each log line
        for line in &log.logs {
            if line.contains("Program data:") {
                if let Some(data) = self.extract_event_data(line) {
                    if data.len() < 8 {
                        continue;
                    }

                    // Check the discriminator
                    let discriminator = &data[0..8];

                    // Parse create position events
                    if discriminator == &CLMM_CREATE_PERSONAL_POSITION_DISCRIMINATOR[..] {
                        if
                            let Ok(event) = RaydiumCLMMCreatePositionEvent::try_from_slice(
                                &data[8..]
                            )
                        {
                            // Check if this pool is monitored
                            if self.is_clmm_pool(&event.pool_state) {
                                self.log_create_position_event(&event, &event.pool_state);
                                events.push(
                                    RaydiumParsedEvent::ClmmCreatePosition(
                                        event,
                                        log.signature.clone(),
                                        event.pool_state
                                    )
                                );
                            }
                        }
                    } else if
                        // Parse increase liquidity events
                        discriminator == &CLMM_LIQUIDITY_INCREASED_DISCRIMINATOR[..]
                    {
                        if
                            let Ok(event) = RaydiumCLMMIncreaseLiquidityEvent::try_from_slice(
                                &data[8..]
                            )
                        {
                            // We need to determine the pool address for increase liquidity events
                            // This might require looking up the position in the logs or database
                            // For now, we'll log a placeholder and implement the lookup later
                            let pool = self.lookup_pool_for_position(
                                &event.position_nft_mint,
                                log
                            )?;

                            if let Some(pool_pubkey) = pool {
                                if self.is_clmm_pool(&pool_pubkey) {
                                    self.log_increase_liquidity_event(&event, &pool_pubkey);
                                    events.push(
                                        RaydiumParsedEvent::ClmmIncreaseLiquidity(
                                            event,
                                            log.signature.clone(),
                                            pool_pubkey
                                        )
                                    );
                                }
                            }
                        }
                    } else if
                        // Parse decrease liquidity events
                        discriminator == &CLMM_LIQUIDITY_DECREASED_DISCRIMINATOR[..]
                    {
                        if
                            let Ok(event) = RaydiumCLMMDecreaseLiquidityEvent::try_from_slice(
                                &data[8..]
                            )
                        {
                            // We need to determine the pool address for decrease liquidity events
                            // This might require looking up the position in the logs or database
                            let pool = self.lookup_pool_for_position(
                                &event.position_nft_mint,
                                log
                            )?;

                            if let Some(pool_pubkey) = pool {
                                if self.is_clmm_pool(&pool_pubkey) {
                                    self.log_decrease_liquidity_event(&event, &pool_pubkey);
                                    events.push(
                                        RaydiumParsedEvent::ClmmDecreaseLiquidity(
                                            event,
                                            log.signature.clone(),
                                            pool_pubkey
                                        )
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(events)
    }

    /// Parse logs for AMM events
    async fn parse_amm_events(&self, log: &RpcLogsResponse) -> Result<Vec<RaydiumParsedEvent>> {
        // Implementation to parse AMM events similar to CLMM parsing
        // For now return empty vector as placeholder
        Ok(Vec::new())
    }

    /// Helper method to look up the pool address from a position NFT
    fn lookup_pool_for_position(
        &self,
        position_nft_mint: &Pubkey,
        log: &RpcLogsResponse
    ) -> Result<Option<Pubkey>> {
        // This is a placeholder implementation
        // In a real implementation, you would:
        // 1. Try to find the pool address in the log
        // 2. If not found, query the database for the position -> pool mapping
        // 3. If still not found, return None or error depending on requirements

        // For now, return None as a placeholder
        Ok(None)
    }
}

#[async_trait]
impl DexIndexer for RaydiumIndexer {
    type Repository = RaydiumRepository;
    type ParsedEvent = RaydiumParsedEvent;

    async fn new(
        db_pool: PgPool,
        provided_pools: Option<&Vec<String>>,
        connection_config: ConnectionConfig
    ) -> Result<Self> {
        // Create the repository for database access
        let repository = RaydiumRepository::new(db_pool.clone());

        // Resolve pool addresses with priority: CLI args > DB > Default
        // This needs to separate pools into AMM and CLMM types
        let (amm_pool_pubkeys, clmm_pool_pubkeys) = repository.get_pools_with_fallback(
            provided_pools,
            DEFAULT_RAYDIUM_AMM_POOL,
            DEFAULT_RAYDIUM_CLMM_POOL
        ).await?;

        // Log the source of pool addresses
        if provided_pools.is_some() && !provided_pools.unwrap().is_empty() {
            logging::log_activity(DEX_NAME, "Pool source", Some("from command line arguments"));
        } else if !amm_pool_pubkeys.is_empty() || !clmm_pool_pubkeys.is_empty() {
            logging::log_activity(DEX_NAME, "Pool source", Some("from database"));
        } else {
            logging::log_activity(
                DEX_NAME,
                "Pool source",
                Some("using default pools (no pools in CLI or database)")
            );
        }

        // Create the signature store
        let signature_store = Self::create_signature_store()?;

        // Create the backfill manager
        let backfill_config = crate::backfill_manager::BackfillConfig {
            rpc_url: connection_config.rpc_url.clone(),
            max_signatures_per_request: 100,
            initial_backfill_slots: 10_000,
            dex_type: DEX_NAME.to_string(),
        };
        let backfill_manager = BackfillManager::new(backfill_config, signature_store.clone());

        Ok(Self {
            repository,
            amm_pool_pubkeys,
            clmm_pool_pubkeys,
            signature_store,
            backfill_manager,
            connection_config,
        })
    }

    fn program_ids(&self) -> Vec<&str> {
        vec![
            // AMM program ID
            "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
            // CLMM program ID
            "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK"
        ]
    }

    fn pool_pubkeys(&self) -> &HashSet<Pubkey> {
        // Return all pools (both AMM and CLMM)
        // This is a limitation of the current trait design
        // We maintain separate pool sets internally but expose a combined view
        &self.all_pool_pubkeys()
    }

    fn repository(&self) -> &Self::Repository {
        &self.repository
    }

    fn dex_name(&self) -> &str {
        DEX_NAME
    }

    fn signature_store(&self) -> &SignatureStore {
        &self.signature_store
    }

    fn backfill_manager(&self) -> &BackfillManager {
        &self.backfill_manager
    }

    fn connection_config(&self) -> &ConnectionConfig {
        &self.connection_config
    }

    /// Parse events from a log, returning any found events without persisting them
    async fn parse_log_events(&self, log: &RpcLogsResponse) -> Result<Vec<Self::ParsedEvent>> {
        // Quick check if the log contains any of our program IDs
        if !self.contains_program_mentions(log) {
            return Ok(Vec::new());
        }

        // Parse both AMM and CLMM events
        let mut events = Vec::new();

        // Add AMM events
        let amm_events = self.parse_amm_events(log).await?;
        events.extend(amm_events);

        // Add CLMM events
        let clmm_events = self.parse_clmm_events(log).await?;
        events.extend(clmm_events);

        Ok(events)
    }

    /// Handle a single event (for both real-time and backfill processing)
    async fn handle_event(&self, event: Self::ParsedEvent) -> Result<()> {
        match event {
            // Handle AMM events
            RaydiumParsedEvent::AmmTraded(signature) => {
                // Handle AMM traded event (placeholder)
                log::info!("Processed AMM traded event for transaction: {}", signature);
                Ok(())
            }

            // Handle CLMM events
            RaydiumParsedEvent::ClmmCreatePosition(event_data, signature, pool) => {
                // Create the base event
                let base_event = self.create_clmm_base_event(
                    &signature,
                    &pool,
                    RaydiumCLMMEventType::CreatePosition
                );

                // Create the data record
                let data = RaydiumCLMMCreatePositionRecord {
                    event_id: 0, // Will be set after base event is inserted
                    minter: event_data.minter.to_string(),
                    nft_owner: event_data.nft_owner.to_string(),
                    output_amount: 0, // This field is not in the event data
                    tick_lower_index: event_data.tick_lower_index,
                    tick_upper_index: event_data.tick_upper_index,
                    liquidity: event_data.liquidity,
                    deposit_amount_0: event_data.deposit_amount_0,
                    deposit_amount_1: event_data.deposit_amount_1,
                    deposit_amount_0_transfer_fee: event_data.deposit_amount_0_transfer_fee,
                    deposit_amount_1_transfer_fee: event_data.deposit_amount_1_transfer_fee,
                };

                let event_record = RaydiumCLMMCreatePostionEventRecord {
                    base: base_event,
                    data,
                };

                self.repository.insert_clmm_create_position_event(event_record).await?;
                Ok(())
            }

            RaydiumParsedEvent::ClmmIncreaseLiquidity(event_data, signature, pool) => {
                // Create the base event
                let base_event = self.create_clmm_base_event(
                    &signature,
                    &pool,
                    RaydiumCLMMEventType::IncreaseLiquidity
                );

                // Create the data record
                let data = RaydiumCLMMIncreaseLiquidityRecord {
                    event_id: 0, // Will be set after base event is inserted
                    position_nft_mint: event_data.position_nft_mint,
                    liquidity: event_data.liquidity,
                    amount_0: event_data.amount_0,
                    amount_1: event_data.amount_1,
                    amount_0_transfer_fee: event_data.amount_0_transfer_fee,
                    amount_1_transfer_fee: event_data.amount_1_transfer_fee,
                };

                let event_record = RaydiumCLMMIncreaseLiquidityEventRecord {
                    base: base_event,
                    data,
                };

                self.repository.insert_clmm_increase_liquidity_event(event_record).await?;
                Ok(())
            }

            RaydiumParsedEvent::ClmmDecreaseLiquidity(event_data, signature, pool) => {
                // Create the base event
                let base_event = self.create_clmm_base_event(
                    &signature,
                    &pool,
                    RaydiumCLMMEventType::DecreaseLiquidity
                );

                // Create the data record
                let data = RaydiumCLMMDecreaseLiquidityRecord {
                    event_id: 0, // Will be set after base event is inserted
                    position_nft_mint: event_data.position_nft_mint,
                    liquidity: event_data.liquidity,
                    decrease_amount_0: event_data.decrease_amount_0,
                    decrease_amount_1: event_data.decrease_amount_1,
                    fee_amount_0: event_data.fee_amount_0,
                    fee_amount_1: event_data.fee_amount_1,
                    reward_amounts: event_data.reward_amounts,
                    transfer_fee_0: event_data.transfer_fee_0,
                    transfer_fee_1: event_data.transfer_fee_1,
                };

                let event_record = RaydiumCLMMDecreaseLiquidityEventRecord {
                    base: base_event,
                    data,
                };

                self.repository.insert_clmm_decrease_liquidity_event(event_record).await?;
                Ok(())
            }
        }
    }
}

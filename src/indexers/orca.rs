use anyhow::Result;
use borsh::BorshDeserialize;
use solana_client::rpc_response::RpcLogsResponse;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashSet;
use sqlx::PgPool;

use crate::db::repositories::OrcaWhirlpoolRepository;
use crate::db::DbSignatureStore;
use crate::indexers::dex_indexer::DexIndexer;
use crate::models::orca::whirlpool::{
    TRADED_EVENT_DISCRIMINATOR,
    LIQUIDITY_INCREASED_DISCRIMINATOR,
    LIQUIDITY_DECREASED_DISCRIMINATOR,
    OrcaWhirlpoolEventType,
    OrcaWhirlpoolEvent,
    OrcaWhirlpoolTradedEvent,
    OrcaWhirlpoolLiquidityIncreasedEvent,
    OrcaWhirlpoolLiquidityDecreasedEvent,
    OrcaWhirlpoolTradedRecord,
    OrcaWhirlpoolLiquidityRecord,
    OrcaWhirlpoolTradedEventRecord,
    OrcaWhirlpoolLiquidityIncreasedEventRecord,
    OrcaWhirlpoolLiquidityDecreasedEventRecord,
};
use crate::{ BackfillConfig, BackfillManager, SignatureStore };

use super::ConnectionConfig;

// Default Orca pool (SOL/USDC)
const DEFAULT_ORCA_POOL: &str = "Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE";
const DEX: &str = "orca";

/// Represents a parsed event from Orca Whirlpool logs
#[derive(Debug)]
pub enum OrcaWhirlpoolParsedEvent {
    Traded(OrcaWhirlpoolTradedEvent, String), // Event and signature
    LiquidityIncreased(OrcaWhirlpoolLiquidityIncreasedEvent, String),
    LiquidityDecreased(OrcaWhirlpoolLiquidityDecreasedEvent, String),
}

/// Orca Whirlpool event indexer
pub struct OrcaWhirlpoolIndexer {
    repository: OrcaWhirlpoolRepository,
    pool_pubkeys: HashSet<Pubkey>,
    signature_store: SignatureStore,
    backfill_manager: BackfillManager,
    connection_config: ConnectionConfig,
}

impl OrcaWhirlpoolIndexer {
    // Utility methods that are not part of the trait
    /// Log details about a traded event
    fn log_traded_event(&self, event: &OrcaWhirlpoolTradedEvent) {
        self.log_event_processed(
            "Traded",
            &event.whirlpool.to_string(),
            &format!(
                "A→B: {}, Amount in: {}, Amount out: {}",
                event.a_to_b,
                event.input_amount,
                event.output_amount
            )
        );
    }

    /// Log details about a liquidity increased event
    fn log_liquidity_increased_event(&self, event: &OrcaWhirlpoolLiquidityIncreasedEvent) {
        self.log_event_processed(
            "LiquidityIncreased",
            &event.whirlpool.to_string(),
            &format!(
                "Position: {}, TokenA: {}, TokenB: {}",
                event.position.to_string(),
                event.token_a_amount,
                event.token_b_amount
            )
        );
    }

    /// Log details about a liquidity decreased event
    fn log_liquidity_decreased_event(&self, event: &OrcaWhirlpoolLiquidityDecreasedEvent) {
        self.log_event_processed(
            "LiquidityDecreased",
            &event.whirlpool.to_string(),
            &format!(
                "Position: {}, TokenA: {}, TokenB: {}",
                event.position.to_string(),
                event.token_a_amount,
                event.token_b_amount
            )
        );
    }

    /// Create a base event record
    fn create_base_event(
        &self,
        signature: &str,
        whirlpool: &Pubkey,
        event_type: OrcaWhirlpoolEventType
    ) -> OrcaWhirlpoolEvent {
        OrcaWhirlpoolEvent {
            id: 0, // Will be set by database
            signature: signature.to_string(),
            whirlpool: whirlpool.to_string(),
            event_type: event_type.to_string(),
            version: 1,
            timestamp: chrono::Utc::now(),
        }
    }
}

#[async_trait::async_trait]
impl DexIndexer for OrcaWhirlpoolIndexer {
    type Repository = OrcaWhirlpoolRepository;
    type ParsedEvent = OrcaWhirlpoolParsedEvent;

    async fn new(
        db_pool: PgPool,
        provided_pools: Option<&Vec<String>>,
        connection_config: ConnectionConfig
    ) -> Result<Self> {
        // Create the repository for database access
        let repository = OrcaWhirlpoolRepository::new(db_pool.clone());

        // Resolve pool addresses with priority: CLI args > DB > Default
        let pool_pubkeys = repository.get_pools_with_fallback(
            provided_pools,
            DEFAULT_ORCA_POOL
        ).await?;

        // Log the source of pool addresses
        if provided_pools.is_some() && !provided_pools.unwrap().is_empty() {
            crate::utils::logging::log_activity(
                DEX,
                "Pool source",
                Some("from command line arguments")
            );
        } else if pool_pubkeys.len() > 1 {
            crate::utils::logging::log_activity(DEX, "Pool source", Some("from database"));
        } else {
            crate::utils::logging::log_activity(
                DEX,
                "Pool source",
                Some("using default pool (no pools in CLI or database)")
            );
        }

        // Create the signature store
        let signature_store = SignatureStore::Database(DbSignatureStore::new(db_pool.clone()));

        // Create the backfill manager
        let backfill_config = BackfillConfig {
            rpc_url: connection_config.rpc_url.clone(),
            max_signatures_per_request: 100,
            initial_backfill_slots: 10_000,
            dex_type: DEX.to_string(),
        };
        let backfill_manager = BackfillManager::new(backfill_config, signature_store.clone());

        Ok(Self {
            repository,
            pool_pubkeys,
            signature_store,
            backfill_manager,
            connection_config,
        })
    }

    fn program_ids(&self) -> Vec<&str> {
        vec!["whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"]
    }

    fn pool_pubkeys(&self) -> &HashSet<Pubkey> {
        &self.pool_pubkeys
    }

    fn repository(&self) -> &Self::Repository {
        &self.repository
    }

    fn dex_name(&self) -> &str {
        DEX
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
        // Quick initial check for relevant event keywords
        let contains_relevant_events = log.logs
            .iter()
            .any(|line| {
                line.contains("Swap") ||
                    line.contains("IncreaseLiquidity") ||
                    line.contains("DecreaseLiquidity")
            });

        if !contains_relevant_events {
            return Ok(Vec::new());
        }

        let mut events = Vec::new();

        // Extract and process events
        let log_lines: Vec<&str> = log.logs
            .iter()
            .map(|s| s.as_str())
            .collect();

        // Find a mention of a whirlpool address that matches our active pools
        for line in &log_lines {
            if line.contains("Program data:") {
                // Extract the binary data part
                if let Some(data) = self.extract_event_data(line) {
                    if data.len() >= 8 {
                        // Get the discriminator (first 8 bytes)
                        let discriminator = &data[0..8];

                        // Using if-else statements with slice comparisons instead of match
                        if discriminator == &TRADED_EVENT_DISCRIMINATOR[..] {
                            if let Ok(event) = OrcaWhirlpoolTradedEvent::try_from_slice(&data[8..]) {
                                // Check if this pool is in our watch list
                                if self.is_monitored_pool(&event.whirlpool, self.pool_pubkeys()) {
                                    self.log_traded_event(&event);
                                    events.push(
                                        OrcaWhirlpoolParsedEvent::Traded(
                                            event,
                                            log.signature.clone()
                                        )
                                    );
                                }
                            }
                        } else if discriminator == &LIQUIDITY_INCREASED_DISCRIMINATOR[..] {
                            if
                                let Ok(event) =
                                    OrcaWhirlpoolLiquidityIncreasedEvent::try_from_slice(&data[8..])
                            {
                                // Check if this pool is in our watch list
                                if self.is_monitored_pool(&event.whirlpool, self.pool_pubkeys()) {
                                    self.log_liquidity_increased_event(&event);
                                    events.push(
                                        OrcaWhirlpoolParsedEvent::LiquidityIncreased(
                                            event,
                                            log.signature.clone()
                                        )
                                    );
                                }
                            }
                        } else if discriminator == &LIQUIDITY_DECREASED_DISCRIMINATOR[..] {
                            if
                                let Ok(event) =
                                    OrcaWhirlpoolLiquidityDecreasedEvent::try_from_slice(&data[8..])
                            {
                                // Check if this pool is in our watch list
                                if self.is_monitored_pool(&event.whirlpool, self.pool_pubkeys()) {
                                    self.log_liquidity_decreased_event(&event);
                                    events.push(
                                        OrcaWhirlpoolParsedEvent::LiquidityDecreased(
                                            event,
                                            log.signature.clone()
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

    /// Handle a single event (for both real-time and backfill processing)
    async fn handle_event(&self, event: Self::ParsedEvent) -> Result<()> {
        match event {
            OrcaWhirlpoolParsedEvent::Traded(event_data, signature) => {
                // Create the base event
                let base_event = self.create_base_event(
                    &signature,
                    &event_data.whirlpool,
                    OrcaWhirlpoolEventType::Traded
                );

                // Create the data record
                let data = OrcaWhirlpoolTradedRecord {
                    event_id: 0, // Will be set after base event is inserted
                    a_to_b: event_data.a_to_b,
                    pre_sqrt_price: event_data.pre_sqrt_price as i64,
                    post_sqrt_price: event_data.post_sqrt_price as i64,
                    input_amount: event_data.input_amount as i64,
                    output_amount: event_data.output_amount as i64,
                    input_transfer_fee: event_data.input_transfer_fee as i64,
                    output_transfer_fee: event_data.output_transfer_fee as i64,
                    lp_fee: event_data.lp_fee as i64,
                    protocol_fee: event_data.protocol_fee as i64,
                };

                let event_record = OrcaWhirlpoolTradedEventRecord {
                    base: base_event,
                    data,
                };

                self.repository.insert_traded_event(event_record).await?;
            }
            OrcaWhirlpoolParsedEvent::LiquidityIncreased(event_data, signature) => {
                // Create the base event
                let base_event = self.create_base_event(
                    &signature,
                    &event_data.whirlpool,
                    OrcaWhirlpoolEventType::LiquidityIncreased
                );

                // Create the data record
                let data = OrcaWhirlpoolLiquidityRecord {
                    event_id: 0, // Will be set after base event is inserted
                    position: event_data.position.to_string(),
                    tick_lower_index: event_data.tick_lower_index,
                    tick_upper_index: event_data.tick_upper_index,
                    liquidity: event_data.liquidity as i64,
                    token_a_amount: event_data.token_a_amount as i64,
                    token_b_amount: event_data.token_b_amount as i64,
                    token_a_transfer_fee: event_data.token_a_transfer_fee as i64,
                    token_b_transfer_fee: event_data.token_b_transfer_fee as i64,
                };

                let event_record = OrcaWhirlpoolLiquidityIncreasedEventRecord {
                    base: base_event,
                    data,
                };

                self.repository.insert_liquidity_increased_event(event_record).await?;
            }
            OrcaWhirlpoolParsedEvent::LiquidityDecreased(event_data, signature) => {
                // Create the base event
                let base_event = self.create_base_event(
                    &signature,
                    &event_data.whirlpool,
                    OrcaWhirlpoolEventType::LiquidityDecreased
                );

                // Create the data record
                let data = OrcaWhirlpoolLiquidityRecord {
                    event_id: 0, // Will be set after base event is inserted
                    position: event_data.position.to_string(),
                    tick_lower_index: event_data.tick_lower_index,
                    tick_upper_index: event_data.tick_upper_index,
                    liquidity: event_data.liquidity as i64,
                    token_a_amount: event_data.token_a_amount as i64,
                    token_b_amount: event_data.token_b_amount as i64,
                    token_a_transfer_fee: event_data.token_a_transfer_fee as i64,
                    token_b_transfer_fee: event_data.token_b_transfer_fee as i64,
                };

                let event_record = OrcaWhirlpoolLiquidityDecreasedEventRecord {
                    base: base_event,
                    data,
                };

                self.repository.insert_liquidity_decreased_event(event_record).await?;
            }
        }

        Ok(())
    }
}

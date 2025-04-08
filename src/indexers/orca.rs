use anyhow::{ Context, Result };
use solana_client::{ rpc_config::RpcTransactionLogsFilter, rpc_response::RpcLogsResponse };
use solana_sdk::{ commitment_config::CommitmentConfig, pubkey::Pubkey };
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::{ Arc, RwLock };
use std::sync::atomic::{ AtomicBool, Ordering };
use tokio::sync::Mutex;
use crate::db::common::Repository;
use std::time::Duration;
use tokio::time::interval;
use tokio::select;
use base64::engine::general_purpose;
use base64::Engine as _;
use borsh::BorshDeserialize;

// Update imports to use new signature store
use crate::backfill_manager::{ BackfillConfig, BackfillManager };
use crate::db::repositories::{
    OrcaWhirlpoolRepository,
    OrcaWhirlpoolPoolRepository,
    OrcaWhirlpoolBatchRepository, // Keep the batch repository trait for DB operations
};
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
use crate::websocket_manager::{ WebSocketConfig, WebSocketManager };

// Default Orca pool (SOL/USDC)
const DEFAULT_ORCA_POOL: &str = "Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE";

/// Orca Whirlpool event indexer
pub struct OrcaWhirlpoolIndexer {
    repository: OrcaWhirlpoolRepository,
    pool_pubkeys: HashSet<Pubkey>,
}

impl OrcaWhirlpoolIndexer {
    /// Create a new indexer with the given repository and pool set
    pub fn new(repository: OrcaWhirlpoolRepository, pool_pubkeys: HashSet<Pubkey>) -> Self {
        Self { repository, pool_pubkeys }
    }

    /// Create an indexer instance with a freshly initialized repository and default pool
    pub fn create(db_pool: sqlx::PgPool) -> Result<Self> {
        // Create a singleton pool set with the default pool
        let mut pool_pubkeys = HashSet::new();
        pool_pubkeys.insert(
            Pubkey::from_str(DEFAULT_ORCA_POOL).context(
                "Failed to parse default Orca pool address"
            )?
        );

        let repository = OrcaWhirlpoolRepository::new(db_pool);
        Ok(Self::new(repository, pool_pubkeys))
    }

    /// Create an indexer and resolve pool addresses in one operation
    ///
    /// This method:
    /// 1. Creates the required repositories
    /// 2. Resolves pool addresses based on priority (CLI > DB > Default)
    /// 3. Logs the source of pool addresses
    /// 4. Returns the fully configured indexer
    pub async fn create_with_pools(
        db_pool: sqlx::PgPool,
        provided_pools: Option<&Vec<String>>
    ) -> Result<Self> {
        // Create the pool repository for address resolution
        let pool_repo = OrcaWhirlpoolPoolRepository::new(db_pool.clone());

        // Resolve pool addresses
        let pool_pubkeys = pool_repo.get_pools_with_fallback(
            provided_pools,
            DEFAULT_ORCA_POOL
        ).await?;

        // Log information about the pool source
        if provided_pools.is_some() && !provided_pools.unwrap().is_empty() {
            println!("Using pool addresses from command line arguments");
        } else if pool_pubkeys.len() > 1 {
            println!("Using pool addresses from database");
        } else {
            println!("No pools specified via CLI or found in database. Using default pool");
        }

        // Create the indexer with the resolved pools
        let repository = OrcaWhirlpoolRepository::new(db_pool);
        Ok(Self::new(repository, pool_pubkeys))
    }

    /// Start indexing events using the pools configured in this indexer
    pub async fn start(&self, rpc_url: &str, ws_url: &str) -> Result<()> {
        // Create a shared pool set for filtering events
        let active_pools = Arc::new(RwLock::new(self.pool_pubkeys.clone()));
        let pool_addresses: Vec<String> = self.pool_pubkeys
            .iter()
            .map(|p| p.to_string())
            .collect();

        println!("Monitoring the following pools:");
        for pool in &pool_addresses {
            println!("  - {}", pool);
        }

        // Create a signature store for tracking last signatures
        // Use the database implementation to persist signatures between runs
        // Access the pool via the Repository trait
        let db_pool = self.repository.pool().clone();
        let signature_store = crate::db::signature_store::create_signature_store(
            crate::db::signature_store::SignatureStoreType::Database,
            Some(db_pool)
        )?;

        // Initialize backfill manager with config and store
        let backfill_config = BackfillConfig {
            rpc_url: rpc_url.to_string(),
            max_signatures_per_request: 100,
            initial_backfill_slots: 10_000,
            dex_type: "orca".to_string(), // Specify this is for Orca DEX
        };
        let backfill_manager = BackfillManager::new(backfill_config, signature_store);

        // Setup WebSocket manager for live events first to ensure we don't miss events
        let ws_config = WebSocketConfig {
            ws_url: ws_url.to_string(),
            filter: RpcTransactionLogsFilter::Mentions(
                vec![
                    // Orca Whirlpool program
                    "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc".to_string()
                ]
            ),
            max_reconnect_attempts: 0, // Unlimited reconnection attempts
            reconnect_base_delay_ms: 500,
            reconnect_max_delay_ms: 30_000,
            commitment: CommitmentConfig::confirmed(),
        };

        println!("Starting WebSocket subscription to capture real-time events...");
        let ws_manager = WebSocketManager::new(ws_config);
        let mut rx_buffer = ws_manager.start_subscription().await?;

        // Create a buffer to store events that arrive during backfill
        let event_buffer = Arc::new(Mutex::new(Vec::<RpcLogsResponse>::new()));
        let is_backfilling = Arc::new(AtomicBool::new(true));

        // Create a clone for the buffer collection task
        let buffer_clone = event_buffer.clone();
        let is_backfilling_clone = is_backfilling.clone();

        // Start a task to collect events during backfill
        let buffer_task = tokio::spawn(async move {
            while is_backfilling_clone.load(Ordering::Relaxed) {
                match tokio::time::timeout(Duration::from_millis(100), rx_buffer.recv()).await {
                    Ok(Some(log_response)) => {
                        // Store the event in our buffer
                        let mut guard = buffer_clone.lock().await;
                        guard.push(log_response.clone());
                    }
                    _ => {} // Either timeout or None result, just continue
                }
            }
        });

        println!("Performing initial backfill for all pools while buffering live events...");

        // Track pools with successful processing
        let mut successful_pools = HashSet::new();

        // Wrap the backfill manager in an Arc for sharing across tasks
        let backfill_manager = Arc::new(backfill_manager);

        println!("Using sequential transaction fetching for RPC calls to avoid rate limiting");

        // Perform initial backfill for all pools
        for pool in &self.pool_pubkeys {
            let pool_pubkey = *pool;
            // Using backfill_manager's built-in logging instead of duplicating it here
            let signatures = match backfill_manager.initial_backfill_for_pool(pool).await {
                Ok(sigs) => sigs,
                Err(e) => {
                    eprintln!("Error during initial backfill for pool {}: {:#}", pool, e);
                    // Continue with next pool instead of stopping the indexer
                    continue;
                }
            };

            if signatures.is_empty() {
                println!("No signatures found for pool {}", pool);
                continue;
            }
            println!("Fetching {} transactions sequentially for pool {}", signatures.len(), pool);

            // Process the transactions sequentially
            let mut _success_count = 0;
            let total_fetched = signatures.len();

            // Create collections for batching different event types
            let mut traded_events: Vec<OrcaWhirlpoolTradedEventRecord> = Vec::new();
            let mut liquidity_increased_events: Vec<OrcaWhirlpoolLiquidityIncreasedEventRecord> =
                Vec::new();
            let mut liquidity_decreased_events: Vec<OrcaWhirlpoolLiquidityDecreasedEventRecord> =
                Vec::new();

            // Process each signature sequentially
            for sig in signatures {
                let result = backfill_manager.fetch_transaction(&sig).await;
                match result {
                    Ok(tx) => {
                        if let Some(meta) = tx.transaction.meta {
                            if
                                let Some(log_messages) = Into::<Option<Vec<String>>>::into(
                                    meta.log_messages
                                )
                            {
                                let log_response = RpcLogsResponse {
                                    signature: sig.to_string(),
                                    err: meta.err,
                                    logs: log_messages,
                                };

                                // Instead of direct processing, use our batch collector
                                match
                                    self.collect_events_from_log(
                                        &log_response,
                                        &active_pools,
                                        &mut traded_events,
                                        &mut liquidity_increased_events,
                                        &mut liquidity_decreased_events
                                    ).await
                                {
                                    Ok(true) => {
                                        // Only count successfully processed transactions
                                        _success_count += 1;
                                        // Mark this pool as having successful processing
                                        successful_pools.insert(pool_pubkey);
                                    }
                                    Ok(false) => {
                                        // No events found, but not an error
                                    }
                                    Err(e) => {
                                        eprintln!(
                                            "Error processing transaction during initial backfill: {:#}",
                                            e
                                        );
                                        // Continue processing instead of returning the error
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error fetching transaction {}: {:#}", sig, e);
                        // Continue with the next transaction
                    }
                }
            }

            // Now batch insert all collected events
            if _success_count > 0 {
                println!("Batch inserting {} traded events", traded_events.len());
                if !traded_events.is_empty() {
                    if let Err(e) = self.repository.batch_insert_traded_events(traded_events).await {
                        eprintln!("Error batch inserting traded events: {:#}", e);
                    }
                }

                println!(
                    "Batch inserting {} liquidity increased events",
                    liquidity_increased_events.len()
                );
                if !liquidity_increased_events.is_empty() {
                    if
                        let Err(e) = self.repository.batch_insert_liquidity_increased_events(
                            liquidity_increased_events
                        ).await
                    {
                        eprintln!("Error batch inserting liquidity increased events: {:#}", e);
                    }
                }

                println!(
                    "Batch inserting {} liquidity decreased events",
                    liquidity_decreased_events.len()
                );
                if !liquidity_decreased_events.is_empty() {
                    if
                        let Err(e) = self.repository.batch_insert_liquidity_decreased_events(
                            liquidity_decreased_events
                        ).await
                    {
                        eprintln!("Error batch inserting liquidity decreased events: {:#}", e);
                    }
                }
            }

            println!(
                "Successfully processed {} out of {} transactions for pool {}",
                _success_count,
                total_fetched,
                pool
            );
        }

        // Signal that backfill is complete
        is_backfilling.store(false, Ordering::Relaxed);

        // Wait for the buffer task to complete
        if let Err(e) = buffer_task.await {
            eprintln!("Error in event buffer task: {:#}", e);
        }

        // Process any events that were buffered during backfill
        let buffered_events = event_buffer.lock().await;
        println!("Processing {} events that arrived during backfill", buffered_events.len());

        for event in buffered_events.iter() {
            if let Err(e) = self.process_log(event, &active_pools).await {
                eprintln!("Error processing buffered event: {:#}", e);
                // Continue processing instead of returning the error
            }
        }

        // We need a new WebSocket subscription for the main processing loop
        // since the previous one was moved into the buffer task
        println!("Starting new WebSocket subscription for main processing loop...");
        let mut rx_main = ws_manager.start_subscription().await?;

        // Setup backfill interval (every 5 minutes)
        let mut backfill_interval = interval(Duration::from_secs(300));

        // Track the last time we detected a connection issue
        let mut last_backfill = std::time::Instant::now();

        println!("Monitoring Orca Whirlpool logs for {} pools...", self.pool_pubkeys.len());

        loop {
            select! {
                // Process incoming WebSocket messages
                Some(log_response) = rx_main.recv() => {
                    if let Err(e) = self.process_log(&log_response, &active_pools).await {
                        eprintln!("Error processing WebSocket log: {:#}", e);
                        // Continue processing instead of stopping the indexer
                    }
                }
                
                // Periodically check for missed transactions
                _ = backfill_interval.tick() => {
                    println!("Running scheduled backfill check");
                    
                    // Check WebSocket health
                    if let Some(elapsed) = ws_manager.time_since_last_received() {
                        if elapsed > Duration::from_secs(60) {
                            println!("WebSocket connection seems stale (no messages for {}s), running backfill", elapsed.as_secs());
                            
                            // If it's been more than 2 minutes since our last backfill, do another one
                            if last_backfill.elapsed() > Duration::from_secs(120) {
                                for pool in &self.pool_pubkeys {
                                    let _pool_pubkey = *pool; // Add underscore to unused variable
                                    let signatures = match backfill_manager.backfill_since_last_signature(pool).await {
                                        Ok(sigs) => sigs,
                                        Err(e) => {
                                            eprintln!("Error during backfill for pool {}: {:#}", pool, e);
                                            // Continue with next pool
                                            continue;
                                        }
                                    };
                                    
                                    if signatures.is_empty() {
                                        println!("No new signatures found for pool {} during scheduled backfill", pool);
                                        continue;
                                    }
                                    
                                    println!(
                                        "Scheduled backfill: Fetching {} transactions sequentially for pool {}",
                                        signatures.len(),
                                        pool
                                    );
                                    
                                    // Store count before processing
                                    let scheduled_total = signatures.len();
                                    let mut scheduled_success_count = 0;
                                    
                                    // Create collections for batching different event types
                                    let mut traded_events: Vec<OrcaWhirlpoolTradedEventRecord> = Vec::new();
                                    let mut liquidity_increased_events: Vec<OrcaWhirlpoolLiquidityIncreasedEventRecord> = Vec::new();
                                    let mut liquidity_decreased_events: Vec<OrcaWhirlpoolLiquidityDecreasedEventRecord> = Vec::new();
                                    
                                    // Process each signature sequentially
                                    for sig in signatures {
                                        let result = backfill_manager.fetch_transaction(&sig).await;
                                        match result {
                                            Ok(tx) => {
                                                if let Some(meta) = tx.transaction.meta {
                                                    if let Some(log_messages) = Into::<Option<Vec<String>>>::into(meta.log_messages) {
                                                        let log_response = RpcLogsResponse {
                                                            signature: sig.to_string(),
                                                            err: meta.err,
                                                            logs: log_messages,
                                                        };
                                                        
                                                        // Use batch collection instead of direct processing
                                                        match self.collect_events_from_log(
                                                            &log_response,
                                                            &active_pools,
                                                            &mut traded_events,
                                                            &mut liquidity_increased_events,
                                                            &mut liquidity_decreased_events
                                                        ).await {
                                                            Ok(true) => {
                                                                scheduled_success_count += 1;
                                                            }
                                                            Ok(false) => {
                                                                // No events found, but not an error
                                                            }
                                                            Err(e) => {
                                                                eprintln!("Error processing transaction during scheduled backfill: {:#}", e);
                                                                // Continue processing instead of returning the error
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                // Log fetch error but continue with next signature
                                                eprintln!("Error fetching transaction {} during scheduled backfill: {:#}", sig, e);
                                            }
                                        }
                                    }
                                    
                                    // Now batch insert all collected events
                                    if scheduled_success_count > 0 {
                                        println!("Scheduled backfill: Batch inserting {} traded events", traded_events.len());
                                        if !traded_events.is_empty() {
                                            if let Err(e) = self.repository.batch_insert_traded_events(traded_events).await {
                                                eprintln!("Error batch inserting traded events during scheduled backfill: {:#}", e);
                                            }
                                        }
                                        
                                        println!("Scheduled backfill: Batch inserting {} liquidity increased events", liquidity_increased_events.len());
                                        if !liquidity_increased_events.is_empty() {
                                            if let Err(e) = self.repository.batch_insert_liquidity_increased_events(liquidity_increased_events).await {
                                                eprintln!("Error batch inserting liquidity increased events during scheduled backfill: {:#}", e);
                                            }
                                        }
                                        
                                        println!("Scheduled backfill: Batch inserting {} liquidity decreased events", liquidity_decreased_events.len());
                                        if !liquidity_decreased_events.is_empty() {
                                            if let Err(e) = self.repository.batch_insert_liquidity_decreased_events(liquidity_decreased_events).await {
                                                eprintln!("Error batch inserting liquidity decreased events during scheduled backfill: {:#}", e);
                                            }
                                        }
                                    }
                                    
                                    println!("Scheduled backfill: Successfully processed {} out of {} transactions for pool {}",
                                             scheduled_success_count, scheduled_total, pool);
                                }
                                
                                last_backfill = std::time::Instant::now();
                            }
                        }
                    }
                }
            }
        }
    }

    /// Process a log response
    async fn process_log(
        &self,
        log: &RpcLogsResponse,
        active_pools: &Arc<RwLock<HashSet<Pubkey>>>
    ) -> Result<()> {
        // Quick initial check for relevant event keywords
        let contains_relevant_events = log.logs
            .iter()
            .any(|line| {
                line.contains("Swap") ||
                    line.contains("IncreaseLiquidity") ||
                    line.contains("DecreaseLiquidity")
            });

        if !contains_relevant_events {
            return Ok(());
        }

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
                                let active_pools_guard = active_pools.read().unwrap();
                                if active_pools_guard.contains(&event.whirlpool) {
                                    // Release the read lock before processing
                                    drop(active_pools_guard);
                                    self.log_traded_event(&event);
                                    return self.handle_traded_event(
                                        event,
                                        log.signature.clone()
                                    ).await;
                                }
                            }
                        } else if discriminator == &LIQUIDITY_INCREASED_DISCRIMINATOR[..] {
                            if
                                let Ok(event) =
                                    OrcaWhirlpoolLiquidityIncreasedEvent::try_from_slice(&data[8..])
                            {
                                // Check if this pool is in our watch list
                                let active_pools_guard = active_pools.read().unwrap();
                                if active_pools_guard.contains(&event.whirlpool) {
                                    // Release the read lock before processing
                                    drop(active_pools_guard);
                                    self.log_liquidity_increased_event(&event);
                                    return self.handle_liquidity_increased_event(
                                        event,
                                        log.signature.clone()
                                    ).await;
                                }
                            }
                        } else if discriminator == &LIQUIDITY_DECREASED_DISCRIMINATOR[..] {
                            if
                                let Ok(event) =
                                    OrcaWhirlpoolLiquidityDecreasedEvent::try_from_slice(&data[8..])
                            {
                                // Check if this pool is in our watch list
                                let active_pools_guard = active_pools.read().unwrap();
                                if active_pools_guard.contains(&event.whirlpool) {
                                    // Release the read lock before processing
                                    drop(active_pools_guard);
                                    self.log_liquidity_decreased_event(&event);
                                    return self.handle_liquidity_decreased_event(
                                        event,
                                        log.signature.clone()
                                    ).await;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_traded_event(
        &self,
        event: OrcaWhirlpoolTradedEvent,
        signature: String
    ) -> Result<()> {
        // Create a new OrcaWhirlpoolEvent without ID and timestamp
        // These will be set by the database when inserting
        let base_event = OrcaWhirlpoolEvent {
            id: 0, // Will be set by database
            signature,
            whirlpool: event.whirlpool.to_string(),
            event_type: OrcaWhirlpoolEventType::Traded.to_string(),
            version: 1,
            timestamp: chrono::Utc::now(), // Will be overwritten by database
        };

        // Create the data record without event_id
        // This will be set by the repository after the base event is inserted
        let data = OrcaWhirlpoolTradedRecord {
            event_id: 0, // Will be set after base event is inserted
            a_to_b: event.a_to_b,
            pre_sqrt_price: event.pre_sqrt_price as i64,
            post_sqrt_price: event.post_sqrt_price as i64,
            input_amount: event.input_amount as i64,
            output_amount: event.output_amount as i64,
            input_transfer_fee: event.input_transfer_fee as i64,
            output_transfer_fee: event.output_transfer_fee as i64,
            lp_fee: event.lp_fee as i64,
            protocol_fee: event.protocol_fee as i64,
        };

        let event_record = OrcaWhirlpoolTradedEventRecord {
            base: base_event,
            data,
        };

        self.repository.insert_traded_event(event_record).await?;
        Ok(())
    }

    async fn handle_liquidity_increased_event(
        &self,
        event: OrcaWhirlpoolLiquidityIncreasedEvent,
        signature: String
    ) -> Result<()> {
        // Create a new OrcaWhirlpoolEvent without ID and timestamp
        // These will be set by the database when inserting
        let base_event = OrcaWhirlpoolEvent {
            id: 0, // Will be set by database
            signature,
            whirlpool: event.whirlpool.to_string(),
            event_type: OrcaWhirlpoolEventType::LiquidityIncreased.to_string(),
            version: 1,
            timestamp: chrono::Utc::now(), // Will be overwritten by database
        };

        // Create the data record without event_id
        // This will be set by the repository after the base event is inserted
        let data = OrcaWhirlpoolLiquidityRecord {
            event_id: 0, // Will be set after base event is inserted
            position: event.position.to_string(),
            tick_lower_index: event.tick_lower_index,
            tick_upper_index: event.tick_upper_index,
            liquidity: event.liquidity as i64,
            token_a_amount: event.token_a_amount as i64,
            token_b_amount: event.token_b_amount as i64,
            token_a_transfer_fee: event.token_a_transfer_fee as i64,
            token_b_transfer_fee: event.token_b_transfer_fee as i64,
        };

        let event_record = OrcaWhirlpoolLiquidityIncreasedEventRecord {
            base: base_event,
            data,
        };

        self.repository.insert_liquidity_increased_event(event_record).await?;
        Ok(())
    }

    async fn handle_liquidity_decreased_event(
        &self,
        event: OrcaWhirlpoolLiquidityDecreasedEvent,
        signature: String
    ) -> Result<()> {
        // Create a new OrcaWhirlpoolEvent without ID and timestamp
        // These will be set by the database when inserting
        let base_event = OrcaWhirlpoolEvent {
            id: 0, // Will be set by database
            signature,
            whirlpool: event.whirlpool.to_string(),
            event_type: OrcaWhirlpoolEventType::LiquidityDecreased.to_string(),
            version: 1,
            timestamp: chrono::Utc::now(), // Will be overwritten by database
        };

        // Create the data record without event_id
        // This will be set by the repository after the base event is inserted
        let data = OrcaWhirlpoolLiquidityRecord {
            event_id: 0, // Will be set after base event is inserted
            position: event.position.to_string(),
            tick_lower_index: event.tick_lower_index,
            tick_upper_index: event.tick_upper_index,
            liquidity: event.liquidity as i64,
            token_a_amount: event.token_a_amount as i64,
            token_b_amount: event.token_b_amount as i64,
            token_a_transfer_fee: event.token_a_transfer_fee as i64,
            token_b_transfer_fee: event.token_b_transfer_fee as i64,
        };

        let event_record = OrcaWhirlpoolLiquidityDecreasedEventRecord {
            base: base_event,
            data,
        };

        self.repository.insert_liquidity_decreased_event(event_record).await?;
        Ok(())
    }

    fn extract_event_data(&self, log_message: &str) -> Option<Vec<u8>> {
        let parts: Vec<&str> = log_message.split("Program data: ").collect();
        if parts.len() >= 2 {
            if let Ok(decoded) = general_purpose::STANDARD.decode(parts[1]) {
                return Some(decoded);
            }
        }
        None
    }

    fn log_traded_event(&self, event: &OrcaWhirlpoolTradedEvent) {
        println!(
            "Traded event: Pool {}, A→B: {}, Amount in: {}, Amount out: {}",
            event.whirlpool.to_string(),
            event.a_to_b,
            event.input_amount,
            event.output_amount
        );
    }

    fn log_liquidity_increased_event(&self, event: &OrcaWhirlpoolLiquidityIncreasedEvent) {
        println!(
            "Liquidity Increased: Pool {}, Position {}, TokenA: {}, TokenB: {}",
            event.whirlpool.to_string(),
            event.position.to_string(),
            event.token_a_amount,
            event.token_b_amount
        );
    }

    fn log_liquidity_decreased_event(&self, event: &OrcaWhirlpoolLiquidityDecreasedEvent) {
        println!(
            "Liquidity Decreased: Pool {}, Position {}, TokenA: {}, TokenB: {}",
            event.whirlpool.to_string(),
            event.position.to_string(),
            event.token_a_amount,
            event.token_b_amount
        );
    }

    // Collect events for batch insertion
    async fn collect_events_from_log(
        &self,
        log: &RpcLogsResponse,
        active_pools: &Arc<RwLock<HashSet<Pubkey>>>,
        traded_events: &mut Vec<OrcaWhirlpoolTradedEventRecord>,
        liquidity_increased_events: &mut Vec<OrcaWhirlpoolLiquidityIncreasedEventRecord>,
        liquidity_decreased_events: &mut Vec<OrcaWhirlpoolLiquidityDecreasedEventRecord>
    ) -> Result<bool> {
        // Quick initial check for relevant event keywords
        let contains_relevant_events = log.logs
            .iter()
            .any(|line| {
                line.contains("Swap") ||
                    line.contains("IncreaseLiquidity") ||
                    line.contains("DecreaseLiquidity")
            });

        if !contains_relevant_events {
            return Ok(false);
        }

        let mut found_events = false;

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
                                let active_pools_guard = active_pools.read().unwrap();
                                if active_pools_guard.contains(&event.whirlpool) {
                                    // Release the read lock before processing
                                    drop(active_pools_guard);

                                    // Create the event record and add to batch
                                    let base_event = OrcaWhirlpoolEvent {
                                        id: 0, // Will be set by database
                                        signature: log.signature.clone(),
                                        whirlpool: event.whirlpool.to_string(),
                                        event_type: OrcaWhirlpoolEventType::Traded.to_string(),
                                        version: 1,
                                        timestamp: chrono::Utc::now(), // Will be overwritten by database
                                    };

                                    let data = OrcaWhirlpoolTradedRecord {
                                        event_id: 0, // Will be set after base event is inserted
                                        a_to_b: event.a_to_b,
                                        pre_sqrt_price: event.pre_sqrt_price as i64,
                                        post_sqrt_price: event.post_sqrt_price as i64,
                                        input_amount: event.input_amount as i64,
                                        output_amount: event.output_amount as i64,
                                        input_transfer_fee: event.input_transfer_fee as i64,
                                        output_transfer_fee: event.output_transfer_fee as i64,
                                        lp_fee: event.lp_fee as i64,
                                        protocol_fee: event.protocol_fee as i64,
                                    };

                                    let event_record = OrcaWhirlpoolTradedEventRecord {
                                        base: base_event,
                                        data,
                                    };

                                    self.log_traded_event(&event);
                                    traded_events.push(event_record);
                                    found_events = true;
                                }
                            }
                        } else if discriminator == &LIQUIDITY_INCREASED_DISCRIMINATOR[..] {
                            if
                                let Ok(event) =
                                    OrcaWhirlpoolLiquidityIncreasedEvent::try_from_slice(&data[8..])
                            {
                                // Check if this pool is in our watch list
                                let active_pools_guard = active_pools.read().unwrap();
                                if active_pools_guard.contains(&event.whirlpool) {
                                    // Release the read lock before processing
                                    drop(active_pools_guard);

                                    // Create the event record and add to batch
                                    let base_event = OrcaWhirlpoolEvent {
                                        id: 0, // Will be set by database
                                        signature: log.signature.clone(),
                                        whirlpool: event.whirlpool.to_string(),
                                        event_type: OrcaWhirlpoolEventType::LiquidityIncreased.to_string(),
                                        version: 1,
                                        timestamp: chrono::Utc::now(), // Will be overwritten by database
                                    };

                                    let data = OrcaWhirlpoolLiquidityRecord {
                                        event_id: 0, // Will be set after base event is inserted
                                        position: event.position.to_string(),
                                        tick_lower_index: event.tick_lower_index,
                                        tick_upper_index: event.tick_upper_index,
                                        liquidity: event.liquidity as i64,
                                        token_a_amount: event.token_a_amount as i64,
                                        token_b_amount: event.token_b_amount as i64,
                                        token_a_transfer_fee: event.token_a_transfer_fee as i64,
                                        token_b_transfer_fee: event.token_b_transfer_fee as i64,
                                    };

                                    let event_record = OrcaWhirlpoolLiquidityIncreasedEventRecord {
                                        base: base_event,
                                        data,
                                    };

                                    self.log_liquidity_increased_event(&event);
                                    liquidity_increased_events.push(event_record);
                                    found_events = true;
                                }
                            }
                        } else if discriminator == &LIQUIDITY_DECREASED_DISCRIMINATOR[..] {
                            if
                                let Ok(event) =
                                    OrcaWhirlpoolLiquidityDecreasedEvent::try_from_slice(&data[8..])
                            {
                                // Check if this pool is in our watch list
                                let active_pools_guard = active_pools.read().unwrap();
                                if active_pools_guard.contains(&event.whirlpool) {
                                    // Release the read lock before processing
                                    drop(active_pools_guard);

                                    // Create the event record and add to batch
                                    let base_event = OrcaWhirlpoolEvent {
                                        id: 0, // Will be set by database
                                        signature: log.signature.clone(),
                                        whirlpool: event.whirlpool.to_string(),
                                        event_type: OrcaWhirlpoolEventType::LiquidityDecreased.to_string(),
                                        version: 1,
                                        timestamp: chrono::Utc::now(), // Will be overwritten by database
                                    };

                                    let data = OrcaWhirlpoolLiquidityRecord {
                                        event_id: 0, // Will be set after base event is inserted
                                        position: event.position.to_string(),
                                        tick_lower_index: event.tick_lower_index,
                                        tick_upper_index: event.tick_upper_index,
                                        liquidity: event.liquidity as i64,
                                        token_a_amount: event.token_a_amount as i64,
                                        token_b_amount: event.token_b_amount as i64,
                                        token_a_transfer_fee: event.token_a_transfer_fee as i64,
                                        token_b_transfer_fee: event.token_b_transfer_fee as i64,
                                    };

                                    let event_record = OrcaWhirlpoolLiquidityDecreasedEventRecord {
                                        base: base_event,
                                        data,
                                    };

                                    self.log_liquidity_decreased_event(&event);
                                    liquidity_decreased_events.push(event_record);
                                    found_events = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(found_events)
    }
}

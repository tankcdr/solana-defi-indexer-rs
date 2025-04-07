use anyhow::Result;
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{ RpcTransactionConfig, RpcTransactionLogsFilter },
    rpc_client::GetConfirmedSignaturesForAddress2Config,
    rpc_response::RpcLogsResponse,
};
use solana_sdk::{ commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signature };
use solana_transaction_status::UiTransactionEncoding;
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
use crate::db::repositories::OrcaWhirlpoolRepository;
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

/// Orca Whirlpool event indexer
pub struct OrcaWhirlpoolIndexer {
    repository: OrcaWhirlpoolRepository,
}

impl OrcaWhirlpoolIndexer {
    /// Create a new indexer with the given repository
    pub fn new(repository: OrcaWhirlpoolRepository) -> Self {
        Self { repository }
    }

    /// Start indexing events for the given pools
    pub async fn start(&self, rpc_url: &str, ws_url: &str, pools: &HashSet<Pubkey>) -> Result<()> {
        // Create a shared pool set for filtering events
        let active_pools = Arc::new(RwLock::new(pools.clone()));
        let pool_addresses: Vec<String> = pools
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

        // Perform initial backfill for all pools
        for pool in pools {
            // Using backfill_manager's built-in logging instead of duplicating it here
            let signatures = match backfill_manager.initial_backfill_for_pool(pool).await {
                Ok(sigs) => sigs,
                Err(e) => {
                    eprintln!("Error during initial backfill for pool {}: {:#}", pool, e);
                    // Continue with next pool instead of stopping the indexer
                    continue;
                }
            };

            // Process the transactions
            let mut _success_count = 0;

            for sig in signatures {
                match backfill_manager.fetch_transaction(&sig).await {
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

                                // Use process_log but don't propagate errors
                                match self.process_log(&log_response, &active_pools).await {
                                    Ok(_) => {
                                        // Only count successfully processed transactions
                                        _success_count += 1;
                                        // Mark this pool as having successful processing
                                        successful_pools.insert(*pool);
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

            // For manual backfill, directly use the backfill_manager to update
            // signatures, as it handles both initial and newest signature tracking
            // We're just providing an enhanced processing loop with better error handling
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

        println!("Monitoring Orca Whirlpool logs for {} pools...", pools.len());

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
                                for pool in pools {
                                    let signatures = match backfill_manager.backfill_since_last_signature(pool).await {
                                        Ok(sigs) => sigs,
                                        Err(e) => {
                                            eprintln!("Error during backfill for pool {}: {:#}", pool, e);
                                            // Continue with next pool
                                            continue;
                                        }
                                    };
                                    for sig in signatures {
                                        if let Ok(tx) = backfill_manager.fetch_transaction(&sig).await {
                                            if let Some(meta) = tx.transaction.meta {
                                                if let Some(log_messages) = Into::<Option<Vec<String>>>::into(meta.log_messages) {
                                                    let log_response = RpcLogsResponse {
                                                        signature: sig.to_string(),
                                                        err: meta.err,
                                                        logs: log_messages,
                                                    };
                                                    
                                                    if let Err(e) = self.process_log(&log_response, &active_pools).await {
                                                        eprintln!("Error processing transaction during scheduled backfill: {:#}", e);
                                                        // Continue processing instead of returning the error
                                                    }
                                                }
                                            }
                                        } else {
                                            // Log fetch error but continue with next signature
                                            eprintln!("Error fetching transaction {} during backfill", sig);
                                        }
                                    }
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

        // Process each log line
        for log_line in &log.logs {
            if log_line.starts_with("Program data: ") {
                if let Some(event_bytes) = self.extract_event_data(log_line) {
                    if event_bytes.len() < 8 {
                        continue;
                    }

                    let discriminator = &event_bytes[..8];
                    let active_pools_guard = active_pools.read().unwrap();

                    match discriminator {
                        d if d == TRADED_EVENT_DISCRIMINATOR => {
                            if
                                let Ok(event) = OrcaWhirlpoolTradedEvent::try_from_slice(
                                    &event_bytes[8..]
                                )
                            {
                                if active_pools_guard.contains(&event.whirlpool) {
                                    drop(active_pools_guard); // Release lock before async operation
                                    if let Err(e) = self.handle_traded_event(log, event).await {
                                        eprintln!("Error processing traded event: {:#}", e);
                                        // Continue processing instead of returning the error
                                    }
                                }
                            }
                        }
                        d if d == LIQUIDITY_INCREASED_DISCRIMINATOR => {
                            if
                                let Ok(event) =
                                    OrcaWhirlpoolLiquidityIncreasedEvent::try_from_slice(
                                        &event_bytes[8..]
                                    )
                            {
                                if active_pools_guard.contains(&event.whirlpool) {
                                    drop(active_pools_guard); // Release lock before async operation
                                    if
                                        let Err(e) = self.handle_liquidity_increased_event(
                                            log,
                                            event
                                        ).await
                                    {
                                        eprintln!(
                                            "Error processing liquidity increased event: {:#}",
                                            e
                                        );
                                        // Continue processing instead of returning the error
                                    }
                                }
                            }
                        }
                        d if d == LIQUIDITY_DECREASED_DISCRIMINATOR => {
                            if
                                let Ok(event) =
                                    OrcaWhirlpoolLiquidityDecreasedEvent::try_from_slice(
                                        &event_bytes[8..]
                                    )
                            {
                                if active_pools_guard.contains(&event.whirlpool) {
                                    drop(active_pools_guard); // Release lock before async operation
                                    if
                                        let Err(e) = self.handle_liquidity_decreased_event(
                                            log,
                                            event
                                        ).await
                                    {
                                        eprintln!(
                                            "Error processing liquidity decreased event: {:#}",
                                            e
                                        );
                                        // Continue processing instead of returning the error
                                    }
                                }
                            }
                        }
                        _ => {} // Not a relevant discriminator
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle a traded event
    async fn handle_traded_event(
        &self,
        log: &RpcLogsResponse,
        event: OrcaWhirlpoolTradedEvent
    ) -> Result<()> {
        println!(
            "Swap detected for pool {}! A->B: {}, Input: {}, Output: {}, LP Fee: {}",
            event.whirlpool,
            event.a_to_b,
            event.input_amount,
            event.output_amount,
            event.lp_fee
        );

        // Create base event record
        let base_event = OrcaWhirlpoolEvent::new(
            log.signature.clone(),
            event.whirlpool,
            OrcaWhirlpoolEventType::Traded
        );

        // Create traded event record
        let traded_event = OrcaWhirlpoolTradedRecord {
            event_id: 0, // Will be set by the database
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

        // Create composite record
        let event_record = OrcaWhirlpoolTradedEventRecord {
            base: base_event,
            data: traded_event,
        };

        // Store in database
        self.repository.insert_traded_event(event_record).await?;

        Ok(())
    }

    /// Handle a liquidity increased event
    async fn handle_liquidity_increased_event(
        &self,
        log: &RpcLogsResponse,
        event: OrcaWhirlpoolLiquidityIncreasedEvent
    ) -> Result<()> {
        println!(
            "Liquidity increased for pool {}: token_a={}, token_b={}",
            event.whirlpool,
            event.token_a_amount,
            event.token_b_amount
        );

        // Create base event record
        let base_event = OrcaWhirlpoolEvent::new(
            log.signature.clone(),
            event.whirlpool,
            OrcaWhirlpoolEventType::LiquidityIncreased
        );

        // Create liquidity event record
        let liquidity_event = OrcaWhirlpoolLiquidityRecord {
            event_id: 0, // Will be set by the database
            position: event.position.to_string(),
            tick_lower_index: event.tick_lower_index,
            tick_upper_index: event.tick_upper_index,
            liquidity: event.liquidity as i64,
            token_a_amount: event.token_a_amount as i64,
            token_b_amount: event.token_b_amount as i64,
            token_a_transfer_fee: event.token_a_transfer_fee as i64,
            token_b_transfer_fee: event.token_b_transfer_fee as i64,
        };

        // Create composite record
        let event_record = OrcaWhirlpoolLiquidityIncreasedEventRecord {
            base: base_event,
            data: liquidity_event,
        };

        // Store in database
        self.repository.insert_liquidity_increased_event(event_record).await?;

        Ok(())
    }

    /// Handle a liquidity decreased event
    async fn handle_liquidity_decreased_event(
        &self,
        log: &RpcLogsResponse,
        event: OrcaWhirlpoolLiquidityDecreasedEvent
    ) -> Result<()> {
        println!(
            "Liquidity decreased for pool {}: token_a={}, token_b={}",
            event.whirlpool,
            event.token_a_amount,
            event.token_b_amount
        );

        // Create base event record
        let base_event = OrcaWhirlpoolEvent::new(
            log.signature.clone(),
            event.whirlpool,
            OrcaWhirlpoolEventType::LiquidityDecreased
        );

        // Create liquidity event record
        let liquidity_event = OrcaWhirlpoolLiquidityRecord {
            event_id: 0, // Will be set by the database
            position: event.position.to_string(),
            tick_lower_index: event.tick_lower_index,
            tick_upper_index: event.tick_upper_index,
            liquidity: event.liquidity as i64,
            token_a_amount: event.token_a_amount as i64,
            token_b_amount: event.token_b_amount as i64,
            token_a_transfer_fee: event.token_a_transfer_fee as i64,
            token_b_transfer_fee: event.token_b_transfer_fee as i64,
        };

        // Create composite record
        let event_record = OrcaWhirlpoolLiquidityDecreasedEventRecord {
            base: base_event,
            data: liquidity_event,
        };

        // Store in database
        self.repository.insert_liquidity_decreased_event(event_record).await?;

        Ok(())
    }

    /// Backfill events for a single pool
    #[allow(dead_code)]
    async fn backfill_pool(&self, rpc_client: &RpcClient, pool_pubkey: &Pubkey) -> Result<()> {
        println!("Backfilling events for pool {}...", pool_pubkey);

        let signatures = match
            rpc_client.get_signatures_for_address_with_config(
                pool_pubkey,
                GetConfirmedSignaturesForAddress2Config {
                    limit: Some(50), // Get more historical data
                    ..Default::default()
                }
            ).await
        {
            Ok(sigs) => sigs,
            Err(e) => {
                eprintln!("Error fetching signatures for pool {}: {:#}", pool_pubkey, e);
                return Ok(()); // Continue without failing the entire process
            }
        };

        println!("Fetched {} signatures", signatures.len());

        for sig_info in signatures {
            let tx = match Signature::from_str(&sig_info.signature) {
                Ok(sig) => {
                    match
                        rpc_client.get_transaction_with_config(&sig, RpcTransactionConfig {
                            encoding: Some(UiTransactionEncoding::JsonParsed),
                            commitment: Some(CommitmentConfig::confirmed()),
                            max_supported_transaction_version: Some(0),
                        }).await
                    {
                        Ok(tx) => tx,
                        Err(e) => {
                            eprintln!("Error fetching transaction {}: {:#}", sig_info.signature, e);
                            continue; // Skip this transaction but continue with others
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error parsing signature {}: {:#}", sig_info.signature, e);
                    continue; // Skip this transaction but continue with others
                }
            };

            if let Some(meta) = tx.transaction.meta {
                if let Some(log_messages) = Into::<Option<Vec<String>>>::into(meta.log_messages) {
                    for log in log_messages {
                        if log.starts_with("Program data: ") {
                            if let Some(event_bytes) = self.extract_event_data(&log) {
                                if event_bytes.len() < 8 {
                                    continue;
                                }

                                let discriminator = &event_bytes[..8];
                                let log_response = RpcLogsResponse {
                                    signature: sig_info.signature.clone(),
                                    err: None,
                                    logs: vec![log.clone()],
                                };

                                if discriminator == TRADED_EVENT_DISCRIMINATOR {
                                    if
                                        let Ok(event) = OrcaWhirlpoolTradedEvent::try_from_slice(
                                            &event_bytes[8..]
                                        )
                                    {
                                        if event.whirlpool == *pool_pubkey {
                                            if
                                                let Err(e) = self.handle_traded_event(
                                                    &log_response,
                                                    event
                                                ).await
                                            {
                                                eprintln!(
                                                    "Error processing traded event during backfill for pool {}: {:#}",
                                                    pool_pubkey,
                                                    e
                                                );
                                                // Continue processing instead of returning the error
                                            }
                                        }
                                    }
                                } else if discriminator == LIQUIDITY_INCREASED_DISCRIMINATOR {
                                    if
                                        let Ok(event) =
                                            OrcaWhirlpoolLiquidityIncreasedEvent::try_from_slice(
                                                &event_bytes[8..]
                                            )
                                    {
                                        if event.whirlpool == *pool_pubkey {
                                            if
                                                let Err(e) = self.handle_liquidity_increased_event(
                                                    &log_response,
                                                    event
                                                ).await
                                            {
                                                eprintln!(
                                                    "Error processing liquidity increased event during backfill for pool {}: {:#}",
                                                    pool_pubkey,
                                                    e
                                                );
                                                // Continue processing instead of returning the error
                                            }
                                        }
                                    }
                                } else if discriminator == LIQUIDITY_DECREASED_DISCRIMINATOR {
                                    if
                                        let Ok(event) =
                                            OrcaWhirlpoolLiquidityDecreasedEvent::try_from_slice(
                                                &event_bytes[8..]
                                            )
                                    {
                                        if event.whirlpool == *pool_pubkey {
                                            if
                                                let Err(e) = self.handle_liquidity_decreased_event(
                                                    &log_response,
                                                    event
                                                ).await
                                            {
                                                eprintln!(
                                                    "Error processing liquidity decreased event during backfill for pool {}: {:#}",
                                                    pool_pubkey,
                                                    e
                                                );
                                                // Continue processing instead of returning the error
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Extract event data from a log message
    fn extract_event_data(&self, log_message: &str) -> Option<Vec<u8>> {
        if let Some(data_start) = log_message.find("Program data: ") {
            let data_str = &log_message[data_start + 14..].trim();
            match general_purpose::STANDARD.decode(data_str) {
                Ok(decoded) => Some(decoded),
                Err(e) => {
                    println!("Failed to decode base64: {}", e);
                    None
                }
            }
        } else {
            None
        }
    }
}

use anyhow::Result;
use solana_client::rpc_config::RpcTransactionLogsFilter;
use solana_client::rpc_response::RpcLogsResponse;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{ AtomicBool, Ordering };
use std::time::Duration;
use tokio::sync::{ Mutex, mpsc::Receiver };
use tokio::task::JoinHandle;
use tokio::time::interval;
use tokio::select;
use base64::engine::general_purpose;
use base64::Engine;
use async_trait::async_trait;

use crate::backfill_manager::{ BackfillManager, BackfillConfig };
use crate::db::signature_store::{ SignatureStore, SignatureStoreType };
use crate::db::Repository;
use crate::websocket_manager::{ WebSocketManager, WebSocketConfig };

/// Core trait for all DEX indexers
#[async_trait]
pub trait DexIndexer {
    // Associated types for DEX-specific structures
    type Repository: crate::db::common::Repository;
    type ParsedEvent: Send;

    //
    // REQUIRED METHODS (must be implemented by each DEX)
    //

    /// Return program IDs to monitor
    fn program_ids(&self) -> Vec<&str>;

    /// Return pools to monitor
    fn pool_pubkeys(&self) -> &HashSet<Pubkey>;

    /// Access to repository
    fn repository(&self) -> &Self::Repository;

    /// Name of the DEX (for logs and config)
    fn dex_name(&self) -> &str;

    /// Parse events from a log, returning any found events without persisting them
    async fn parse_log_events(&self, log: &RpcLogsResponse) -> Result<Vec<Self::ParsedEvent>>;

    /// Handle a single event (for both real-time and backfill processing)
    async fn handle_event(&self, event: Self::ParsedEvent) -> Result<()>;

    //
    // CORE PROCESSING METHODS (default implementations)
    //

    /// Process a single log (for real-time events)
    async fn process_log(&self, log: &RpcLogsResponse) -> Result<()> {
        // Check if log contains relevant program IDs
        if !self.contains_program_mentions(log) {
            return Ok(());
        }

        // Parse and process events
        let start = std::time::Instant::now();
        let events = self.parse_log_events(log).await?;

        for event in events {
            if let Err(e) = self.handle_event(event).await {
                self.log_error("Failed to handle event", &e);
                // Continue processing other events
            }
        }

        self.record_processing_time("process_log", start.elapsed().as_millis() as u64);
        Ok(())
    }

    /// Start the indexer
    async fn start(&self, rpc_url: &str, ws_url: &str) -> Result<()> {
        // Log startup information
        self.log_activity(&format!("Starting {} indexer", self.dex_name()), None);

        // Log all pools being monitored
        self.log_monitored_pools();

        // Create signature store
        let signature_store = self.create_signature_store()?;

        // Initialize backfill manager
        let backfill_manager = self.create_backfill_manager(rpc_url, signature_store);
        let backfill_manager = Arc::new(backfill_manager);

        // Setup WebSocket manager
        let (ws_manager, rx_buffer) = self.setup_websocket_manager(ws_url).await?;

        // Setup event buffering during backfill
        let (event_buffer, is_backfilling, buffer_task) =
            self.setup_event_buffering(rx_buffer).await;

        // Perform initial backfill
        self.perform_backfill(&backfill_manager).await?;

        // Signal backfill completion and process buffered events
        self.process_buffered_events(event_buffer, is_backfilling, buffer_task).await?;

        // Main event processing loop with periodic backfill
        self.run_main_event_loop(ws_manager, backfill_manager).await
    }

    //
    // EVENT DETECTION HELPERS
    //

    /// Extract binary data from log lines
    fn extract_event_data(&self, log_line: &str) -> Option<Vec<u8>> {
        let parts: Vec<&str> = log_line.split("Program data: ").collect();
        if parts.len() >= 2 {
            if let Ok(decoded) = general_purpose::STANDARD.decode(parts[1]) {
                return Some(decoded);
            }
        }
        None
    }

    /// Check if a discriminator matches
    fn matches_discriminator(data: &[u8], discriminator: &[u8; 8]) -> bool {
        data.len() >= 8 && &data[0..8] == discriminator
    }

    /// Check if a pubkey is in the monitored pool set
    fn is_monitored_pool(&self, pool: &Pubkey, pool_set: &HashSet<Pubkey>) -> bool {
        pool_set.contains(pool)
    }

    /// Check if a log contains events from any of the monitored programs
    fn contains_program_mentions(&self, log: &RpcLogsResponse) -> bool {
        let program_ids = self.program_ids();
        log.logs
            .iter()
            .any(|line| { program_ids.iter().any(|&program_id| line.contains(program_id)) })
    }

    /// Check if log contains keywords suggesting relevant events
    fn contains_event_keywords(&self, log: &RpcLogsResponse, keywords: &[&str]) -> bool {
        log.logs.iter().any(|line| { keywords.iter().any(|&keyword| line.contains(keyword)) })
    }

    /// Helper to convert transaction & metadata into RpcLogsResponse for processing
    fn tx_to_logs_response(&self, signature: &str, logs: &[String]) -> RpcLogsResponse {
        RpcLogsResponse {
            signature: signature.to_string(),
            err: None,
            logs: logs
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }

    //
    // ERROR HANDLING METHODS
    //

    /// Handle common error cases with standardized recovery strategies
    fn handle_rpc_error(&self, err: &anyhow::Error, context: &str) -> Result<()> {
        // Log the error with context
        self.log_error(context, err);

        // Check if it's a rate limit error
        if err.to_string().contains("429") || err.to_string().contains("rate limit") {
            // Implement exponential backoff
            self.log_activity("Rate limit hit, implementing backoff...", None);
            // Return special error type that signals backoff needed
            return Err(anyhow::anyhow!("RateLimit"));
        }

        // For other errors, log and continue
        Err(anyhow::anyhow!("Non-recoverable error: {}", err))
    }

    /// Helper to handle transaction parsing errors
    fn handle_tx_parse_error(&self, signature: &str, err: &anyhow::Error) -> Result<()> {
        self.log_error(&format!("Error parsing transaction {}", signature), err);

        // Decide if we should retry or skip based on error type
        if err.to_string().contains("not found") {
            self.log_activity("Transaction not found, skipping", None);
            Ok(()) // Return Ok to continue processing
        } else {
            // For other errors, propagate
            Err(anyhow::anyhow!("Transaction parse error: {}", err))
        }
    }

    /// Helper to categorize errors as transient or permanent
    fn is_transient_error(&self, err: &anyhow::Error) -> bool {
        let err_str = err.to_string();
        err_str.contains("429") ||
            err_str.contains("rate limit") ||
            err_str.contains("timeout") ||
            err_str.contains("connection")
    }

    //
    // LOGGING METHODS
    //

    /// Log all pools being monitored
    fn log_monitored_pools(&self) {
        let pool_addresses: Vec<String> = self
            .pool_pubkeys()
            .iter()
            .map(|p| p.to_string())
            .collect();
        // Log using the standard format
        self.log_activity("Monitoring pools", Some(&format!("{} pools", pool_addresses.len())));
        for pool in &pool_addresses {
            self.log_activity("Pool", Some(pool));
        }
    }

    /// Log event processing with standard format
    fn log_event_processed<T: std::fmt::Debug + std::fmt::Display>(
        &self,
        event_type: &str,
        entity: &str,
        details: &T
    ) {
        // Log with the DEX name and include the event details in the standard output
        crate::utils::logging::log_dex_activity(
            "event",
            self.dex_name(),
            &format!("{} event", event_type),
            Some(&format!("{}: {}", entity, details))
        );

        // Also log to debug for more detailed information
        if log::log_enabled!(log::Level::Debug) {
            log::debug!(
                "[{}] {} ({}) - Details: {:?}",
                "event",
                self.dex_name(),
                format!("{} event", event_type),
                details
            );
        }
    }

    /// Log processing statistics
    fn log_processing_stats(&self, context: &str, total: usize, success: usize) {
        let percent = if total > 0 { ((success as f64) / (total as f64)) * 100.0 } else { 0.0 };

        let stats = format!(
            "Successfully processed {} out of {} transactions ({:.1}%)",
            success,
            total,
            percent
        );

        crate::utils::logging::log_stats(self.dex_name(), context, &stats);
    }

    /// Structured error logging with context
    fn log_error(&self, context: &str, err: &anyhow::Error) {
        crate::utils::logging::log_error(self.dex_name(), context, err);

        // Log full error with backtrace in debug mode
        if log::log_enabled!(log::Level::Debug) {
            log::debug!("Full error: {:#}", err);
        }
    }

    /// Activity logging for major operations
    fn log_activity(&self, activity: &str, details: Option<&str>) {
        crate::utils::logging::log_activity(self.dex_name(), activity, details);
    }

    /// Helper for tracking performance metrics
    fn record_processing_time(&self, operation: &str, duration_ms: u64) {
        self.log_activity(operation, Some(&format!("completed in {} ms", duration_ms)));
    }

    //
    // INFRASTRUCTURE SETUP METHODS
    //

    /// Create signature store
    fn create_signature_store(&self) -> Result<SignatureStore> {
        let db_pool = self.repository().pool().clone();
        crate::db::signature_store::create_signature_store(
            SignatureStoreType::Database,
            Some(db_pool)
        )
    }

    /// Create backfill manager
    fn create_backfill_manager(
        &self,
        rpc_url: &str,
        signature_store: SignatureStore
    ) -> BackfillManager {
        let backfill_config = BackfillConfig {
            rpc_url: rpc_url.to_string(),
            max_signatures_per_request: 100,
            initial_backfill_slots: 10_000,
            dex_type: self.dex_name().to_string(),
        };

        BackfillManager::new(backfill_config, signature_store)
    }

    /// Setup WebSocket manager
    async fn setup_websocket_manager(
        &self,
        ws_url: &str
    ) -> Result<(WebSocketManager, Receiver<RpcLogsResponse>)> {
        let ws_config = WebSocketConfig {
            ws_url: ws_url.to_string(),
            filter: RpcTransactionLogsFilter::Mentions(
                self
                    .program_ids()
                    .iter()
                    .map(|&s| s.to_string())
                    .collect()
            ),
            max_reconnect_attempts: 0, // Unlimited reconnection attempts
            reconnect_base_delay_ms: 500,
            reconnect_max_delay_ms: 30_000,
            commitment: CommitmentConfig::confirmed(),
        };

        self.log_activity("Starting WebSocket subscription for real-time events", None);
        let ws_manager = WebSocketManager::new(ws_config);
        let rx_buffer = ws_manager.start_subscription().await?;

        Ok((ws_manager, rx_buffer))
    }

    /// Setup event buffering during backfill
    async fn setup_event_buffering(
        &self,
        rx_buffer: Receiver<RpcLogsResponse>
    ) -> (Arc<Mutex<Vec<RpcLogsResponse>>>, Arc<AtomicBool>, JoinHandle<()>) {
        let event_buffer = Arc::new(Mutex::new(Vec::<RpcLogsResponse>::new()));
        let is_backfilling = Arc::new(AtomicBool::new(true));

        // Create clones for the buffer collection task
        let buffer_clone = event_buffer.clone();
        let is_backfilling_clone = is_backfilling.clone();
        let mut rx_clone = rx_buffer;

        // Start a task to collect events during backfill
        let buffer_task = tokio::spawn(async move {
            while is_backfilling_clone.load(Ordering::Relaxed) {
                match tokio::time::timeout(Duration::from_millis(100), rx_clone.recv()).await {
                    Ok(Some(log_response)) => {
                        // Store the event in our buffer
                        let mut guard = buffer_clone.lock().await;
                        guard.push(log_response.clone());
                    }
                    _ => {} // Either timeout or None result, just continue
                }
            }
        });

        (event_buffer, is_backfilling, buffer_task)
    }

    //
    // BACKFILL OPERATIONS
    //

    /// Main backfill coordinator - orchestrates the entire backfill process
    async fn perform_backfill(&self, backfill_manager: &Arc<BackfillManager>) -> Result<()> {
        self.log_activity("Starting initial backfill", None);

        // Track overall statistics
        let mut total_processed = 0;
        let mut total_success = 0;

        for pool in self.pool_pubkeys() {
            let result = self.backfill_pool(backfill_manager, pool).await;

            match result {
                Ok((processed, success)) => {
                    total_processed += processed;
                    total_success += success;
                }
                Err(e) => {
                    self.log_error(&format!("Backfill for pool {}", pool), &e);
                    // Continue with next pool
                }
            }
        }

        self.log_processing_stats("Initial backfill complete", total_processed, total_success);
        Ok(())
    }

    /// Process backfill for a single pool
    async fn backfill_pool(
        &self,
        backfill_manager: &Arc<BackfillManager>,
        pool: &Pubkey
    ) -> Result<(usize, usize)> {
        self.log_activity("Backfilling pool", Some(&pool.to_string()));

        // Get signatures for this pool
        let signatures = backfill_manager.initial_backfill_for_pool(pool).await.map_err(|e| {
            self.log_error(&format!("Failed to get signatures for pool {}", pool), &e);
            e
        })?;

        if signatures.is_empty() {
            self.log_activity("Backfill", Some(&format!("No signatures found for pool {}", pool)));
            return Ok((0, 0));
        }
        self.log_activity(
            "Transaction fetch",
            Some(&format!("Fetching {} transactions for pool {}", signatures.len(), pool))
        );

        // Process the transactions and return stats
        self.process_backfill_signatures(backfill_manager, &signatures).await
    }

    /// Process a batch of signatures during backfill
    async fn process_backfill_signatures(
        &self,
        backfill_manager: &Arc<BackfillManager>,
        signatures: &Vec<Signature>
    ) -> Result<(usize, usize)> {
        let total = signatures.len();
        let mut success_count = 0;
        let mut event_batch = Vec::new();

        for sig in signatures {
            match backfill_manager.fetch_transaction(sig).await {
                Ok(tx) => {
                    if let Some(meta) = tx.transaction.meta {
                        if
                            let Some(log_messages) = Into::<Option<Vec<String>>>::into(
                                meta.log_messages
                            )
                        {
                            let logs_response = self.tx_to_logs_response(
                                &sig.to_string(),
                                &log_messages
                            );

                            // Parse events from this transaction
                            let events = self.parse_log_events(&logs_response).await?;

                            if !events.is_empty() {
                                success_count += 1;
                                event_batch.extend(events);
                            }
                        }
                    }
                }
                Err(e) => {
                    self.handle_tx_parse_error(&sig.to_string(), &e)?;
                    // Continue with next signature
                }
            }
        }

        // Process each event individually
        if !event_batch.is_empty() {
            // Log that we're processing events
            self.log_activity(
                "Processing backfill events",
                Some(&format!("{} events", event_batch.len()))
            );

            // Process each event individually
            for event in event_batch {
                if let Err(e) = self.handle_event(event).await {
                    self.log_error("Failed to process backfill event", &e);
                    // Continue with next event
                }
            }
        }

        Ok((total, success_count))
    }

    /// Handle periodic/scheduled backfill operations
    async fn perform_scheduled_backfill(
        &self,
        backfill_manager: &Arc<BackfillManager>
    ) -> Result<()> {
        self.log_activity("Running scheduled backfill", None);

        let mut total_processed = 0;
        let mut total_success = 0;

        for pool in self.pool_pubkeys() {
            // Get signatures since last processed
            let signatures = match backfill_manager.backfill_since_last_signature(pool).await {
                Ok(sigs) => sigs,
                Err(e) => {
                    self.log_error(
                        &format!("Failed to get recent signatures for pool {}", pool),
                        &e
                    );
                    continue;
                }
            };

            if signatures.is_empty() {
                continue;
            }

            // Process these signatures
            match self.process_backfill_signatures(backfill_manager, &signatures).await {
                Ok((processed, success)) => {
                    total_processed += processed;
                    total_success += success;
                }
                Err(e) => {
                    self.log_error(
                        &format!("Error processing scheduled backfill for pool {}", pool),
                        &e
                    );
                    // Continue with next pool
                }
            }
        }

        if total_processed > 0 {
            self.log_processing_stats("Scheduled backfill", total_processed, total_success);
        }

        Ok(())
    }

    /// Process events that were buffered during backfill
    async fn process_buffered_events(
        &self,
        event_buffer: Arc<Mutex<Vec<RpcLogsResponse>>>,
        is_backfilling: Arc<AtomicBool>,
        buffer_task: JoinHandle<()>
    ) -> Result<()> {
        // Signal that backfill is complete
        is_backfilling.store(false, Ordering::Relaxed);

        // Wait for the buffer task to complete
        if let Err(e) = buffer_task.await {
            self.log_error("Error in event buffer task", &anyhow::anyhow!("{}", e));
        }

        // Process any events that were buffered during backfill
        let buffered_events = event_buffer.lock().await;
        let count = buffered_events.len();

        self.log_activity(&format!("Processing {} buffered events", count), None);

        for event in buffered_events.iter() {
            if let Err(e) = self.process_log(event).await {
                self.log_error("Error processing buffered event", &e);
                // Continue processing instead of returning the error
            }
        }

        Ok(())
    }

    /// Main event processing loop with periodic backfill
    async fn run_main_event_loop(
        &self,
        ws_manager: WebSocketManager,
        backfill_manager: Arc<BackfillManager>
    ) -> Result<()> {
        // We need a new WebSocket subscription for the main processing loop
        self.log_activity("Starting main event processing loop", None);
        let mut rx_main = ws_manager.start_subscription().await?;

        // Setup backfill interval (every 5 minutes)
        let mut backfill_interval = interval(Duration::from_secs(300));

        // Track the last time we detected a connection issue
        let mut last_backfill = std::time::Instant::now();

        loop {
            select! {
                // Process incoming WebSocket messages
                Some(log_response) = rx_main.recv() => {
                    if let Err(e) = self.process_log(&log_response).await {
                        self.log_error("Error processing WebSocket log", &e);
                        // Continue processing instead of stopping the indexer
                    }
                }
                
                // Periodically check for missed transactions
                _ = backfill_interval.tick() => {
                    if let Some(elapsed) = ws_manager.time_since_last_received() {
                        if elapsed > Duration::from_secs(60) {
                            self.log_activity("WebSocket connection seems stale, running backfill", 
                                            Some(&format!("No messages for {}s", elapsed.as_secs())));
                            
                            // If it's been more than 2 minutes since our last backfill, do another one
                            if last_backfill.elapsed() > Duration::from_secs(120) {
                                if let Err(e) = self.perform_scheduled_backfill(&backfill_manager).await {
                                    self.log_error("Error during scheduled backfill", &e);
                                }
                                
                                last_backfill = std::time::Instant::now();
                            }
                        }
                    }
                }
            }
        }
    }
}

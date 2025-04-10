use anyhow::Result;
use futures::stream::StreamExt;
use solana_client::{
    nonblocking::pubsub_client::PubsubClient,
    rpc_config::RpcTransactionLogsConfig,
    rpc_config::RpcTransactionLogsFilter,
    rpc_response::RpcLogsResponse,
};
use solana_sdk::commitment_config::CommitmentConfig;
use std::sync::{ Arc, atomic::{ AtomicBool, Ordering } };
use std::time::{ Duration, Instant };
use tokio::sync::mpsc;
use tokio::time::sleep;

use crate::utils::logging;

/// Configuration for the WebSocket manager
pub struct WebSocketConfig {
    /// WebSocket URL
    pub ws_url: String,
    /// Custom filter for logs
    pub filter: RpcTransactionLogsFilter,
    /// Maximum number of reconnection attempts
    pub max_reconnect_attempts: u32,
    /// Initial reconnection delay in milliseconds
    pub reconnect_base_delay_ms: u64,
    /// Maximum reconnection delay in milliseconds
    pub reconnect_max_delay_ms: u64,
    /// Log subscription commitment level
    pub commitment: CommitmentConfig,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            ws_url: "wss://api.mainnet-beta.solana.com".to_string(),
            filter: RpcTransactionLogsFilter::All,
            max_reconnect_attempts: 0, // 0 means unlimited
            reconnect_base_delay_ms: 500,
            reconnect_max_delay_ms: 30000, // 30 seconds
            commitment: CommitmentConfig::confirmed(),
        }
    }
}

/// WebSocket connection manager for Solana
pub struct WebSocketManager {
    config: WebSocketConfig,
    running: Arc<AtomicBool>,
    last_received: Arc<std::sync::Mutex<Option<Instant>>>,
}

impl WebSocketManager {
    /// Create a new WebSocket manager
    pub fn new(config: WebSocketConfig) -> Self {
        Self {
            config,
            running: Arc::new(AtomicBool::new(true)),
            last_received: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    /// Start the WebSocket subscription with reconnection logic
    pub async fn start_subscription(&self) -> Result<mpsc::Receiver<RpcLogsResponse>> {
        // Create a channel for passing log responses
        let (tx, rx) = mpsc::channel::<RpcLogsResponse>(1000);

        // Clone values for the subscription task
        let running = self.running.clone();
        let config = self.config.clone();
        let last_received = self.last_received.clone();

        // Start the subscription task
        tokio::spawn(async move {
            let mut reconnect_attempts = 0;
            let mut reconnect_delay = config.reconnect_base_delay_ms;

            // Continuously try to maintain the connection
            while running.load(Ordering::SeqCst) {
                let pubsub_client_result = PubsubClient::new(&config.ws_url).await;

                if let Ok(pubsub_client) = pubsub_client_result {
                    // Subscribe to logs
                    let subscription_result = pubsub_client.logs_subscribe(
                        config.filter.clone(),
                        RpcTransactionLogsConfig {
                            commitment: Some(config.commitment),
                        }
                    ).await;

                    match subscription_result {
                        Ok((mut log_stream, _subscription_id)) => {
                            logging::log_activity(
                                "websocket",
                                "Connection",
                                Some("established successfully")
                            );

                            // Reset reconnection counters upon successful connection
                            reconnect_attempts = 0;
                            reconnect_delay = config.reconnect_base_delay_ms;

                            // Process incoming logs until disconnection
                            while let Some(response) = log_stream.next().await {
                                // Update last received timestamp
                                {
                                    let mut guard = last_received.lock().unwrap();
                                    *guard = Some(Instant::now());
                                }

                                // Send to channel, break if channel is closed
                                if tx.send(response.value).await.is_err() {
                                    logging::log_activity(
                                        "websocket",
                                        "Channel closed",
                                        Some("stopping WebSocket subscription")
                                    );
                                    return;
                                }
                            }

                            logging::log_activity(
                                "websocket",
                                "Connection dropped",
                                Some("will reconnect...")
                            );
                        }
                        Err(e) => {
                            logging::log_error(
                                "websocket",
                                "Subscription failure",
                                &anyhow::anyhow!("{}", e)
                            );
                        }
                    }
                } else if let Err(e) = pubsub_client_result {
                    // Log connection error
                    logging::log_error(
                        "websocket",
                        "Connection failure",
                        &anyhow::anyhow!("{}", e)
                    );
                }

                // Check if we've hit the maximum reconnection attempts
                if
                    config.max_reconnect_attempts > 0 &&
                    reconnect_attempts >= config.max_reconnect_attempts
                {
                    let msg = format!(
                        "Maximum reconnection attempts reached ({}), stopping reconnection",
                        config.max_reconnect_attempts
                    );
                    logging::log_error(
                        "websocket",
                        "Reconnection limit reached",
                        &anyhow::anyhow!("{}", msg)
                    );
                    break;
                }

                // Implement exponential backoff for reconnection
                reconnect_attempts += 1;
                logging::log_activity(
                    "websocket",
                    "Reconnection",
                    Some(&format!("attempt {} in {} ms", reconnect_attempts, reconnect_delay))
                );
                sleep(Duration::from_millis(reconnect_delay)).await;

                // Increase delay for next attempt with exponential backoff
                reconnect_delay = std::cmp::min(reconnect_delay * 2, config.reconnect_max_delay_ms);
            }

            logging::log_activity("websocket", "Manager stopped", None);
        });

        Ok(rx)
    }

    /// Get the time since the last received message
    pub fn time_since_last_received(&self) -> Option<Duration> {
        let guard = self.last_received.lock().unwrap();
        guard.map(|instant| instant.elapsed())
    }

    /// Check if the connection is likely dead
    pub fn is_connection_dead(&self, timeout: Duration) -> bool {
        match self.time_since_last_received() {
            Some(elapsed) => elapsed > timeout,
            None => false, // Never received anything yet
        }
    }

    /// Stop the WebSocket subscription
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

impl Clone for WebSocketConfig {
    fn clone(&self) -> Self {
        WebSocketConfig {
            ws_url: self.ws_url.clone(),
            filter: self.filter.clone(),
            max_reconnect_attempts: self.max_reconnect_attempts,
            reconnect_base_delay_ms: self.reconnect_base_delay_ms,
            reconnect_max_delay_ms: self.reconnect_max_delay_ms,
            commitment: self.commitment,
        }
    }
}

# Adding a New DEX

This guide walks through the process of adding support for a new decentralized exchange (DEX) to the event indexer.

## Overview

Adding a new DEX involves several steps:

1. Create database tables for the new DEX's events
2. Define model structures for the on-chain events
3. Implement a repository for database operations
4. Build an indexer for processing the DEX's events
5. Update the main application with a new CLI command

Let's go through each step in detail using a hypothetical "Raydium" DEX as an example.

## 1. Database Schema

First, create database tables for the new DEX's events in `database/schema.sql`:

```sql
-- Base table for Raydium events
CREATE TABLE apestrong.raydium_concentrated_events (
    id SERIAL PRIMARY KEY,
    signature VARCHAR(88) NOT NULL UNIQUE,
    pool VARCHAR(44) NOT NULL,
    event_type VARCHAR(32) NOT NULL,
    version INT NOT NULL DEFAULT 1,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_raydium_concentrated_events_pool_timestamp
ON apestrong.raydium_concentrated_events (pool, timestamp);

-- Swap events
CREATE TABLE apestrong.raydium_swap_events (
    event_id INT PRIMARY KEY REFERENCES apestrong.raydium_concentrated_events(id) ON DELETE CASCADE,
    in_token_amount BIGINT NOT NULL,
    out_token_amount BIGINT NOT NULL,
    fee_amount BIGINT NOT NULL,
    price NUMERIC NOT NULL
);

-- Add more event-specific tables as needed

-- Create a view for convenience
CREATE OR REPLACE VIEW apestrong.v_raydium_concentrated_swap AS
SELECT
    e.id, e.signature, e.pool, e.timestamp,
    s.in_token_amount, s.out_token_amount, s.fee_amount, s.price
FROM
    apestrong.raydium_concentrated_events e
JOIN
    apestrong.raydium_swap_events s ON e.id = s.event_id
WHERE
    e.event_type = 'Swap';
```

Run the setup script to apply these schema changes:

```bash
cargo run --bin setup_db
```

## 2. Define Models

### Create Model Directory Structure

```
src/models/raydium/
src/models/raydium/mod.rs
src/models/raydium/concentrated.rs
```

### Update `mod.rs` Files

First, update `src/models/raydium/mod.rs`:

```rust
pub mod concentrated;
```

Then, update `src/models/mod.rs` to include the new module:

```rust
pub mod common;
pub mod orca;
pub mod raydium; // Add this line
```

### Update Protocol Enum

Add the new DEX to the Protocol enum in `src/models/common.rs`:

```rust
pub enum Protocol {
    OrcaWhirlpool,
    RaydiumConcentrated,  // Add this line
    // Future protocols...
}

impl ToString for Protocol {
    fn to_string(&self) -> String {
        match self {
            Protocol::OrcaWhirlpool => "orca_whirlpool".to_string(),
            Protocol::RaydiumConcentrated => "raydium_concentrated".to_string(),  // Add this line
        }
    }
}

impl FromStr for Protocol {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "orca_whirlpool" => Ok(Protocol::OrcaWhirlpool),
            "raydium_concentrated" => Ok(Protocol::RaydiumConcentrated),  // Add this line
            _ => Err(format!("Unknown protocol: {}", s)),
        }
    }
}
```

### Create Event Models

Create model structures in `src/models/raydium/concentrated.rs`:

```rust
/******************************************************************************
 * IMPORTANT: DO NOT MODIFY THIS FILE WITHOUT EXPLICIT APPROVAL
 *
 * This file is protected and should not be modified without explicit approval.
 * Any changes could break the indexer functionality.
 *
 * See .nooverwrite.json for more information on protected files.
 ******************************************************************************/

use chrono::{DateTime, Utc};
use borsh::BorshDeserialize;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

// Raydium Concentrated event discriminators
pub const SWAP_EVENT_DISCRIMINATOR: [u8; 8] = [0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80]; // Example - replace with actual discriminator

// Event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RaydiumEventType {
    Swap,
    PositionCreated,
    PositionClosed,
    // Add more as needed
}

impl ToString for RaydiumEventType {
    fn to_string(&self) -> String {
        match self {
            RaydiumEventType::Swap => "Swap".to_string(),
            RaydiumEventType::PositionCreated => "PositionCreated".to_string(),
            RaydiumEventType::PositionClosed => "PositionClosed".to_string(),
        }
    }
}

impl FromStr for RaydiumEventType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Swap" => Ok(RaydiumEventType::Swap),
            "PositionCreated" => Ok(RaydiumEventType::PositionCreated),
            "PositionClosed" => Ok(RaydiumEventType::PositionClosed),
            _ => Err(format!("Unknown Raydium event type: {}", s)),
        }
    }
}

// On-chain event structures (as deserialized from Solana transactions)
#[derive(BorshDeserialize, Debug)]
pub struct RaydiumSwapEvent {
    pub pool: Pubkey,
    pub in_token_amount: u64,
    pub out_token_amount: u64,
    pub fee_amount: u64,
    pub price: u64,
}

impl RaydiumSwapEvent {
    pub fn try_from_bytes(data: &[u8]) -> Result<Self, borsh::maybestd::io::Error> {
        Self::try_from_slice(data)
    }
}

// Database models
#[derive(Debug, Clone, FromRow)]
pub struct RaydiumConcentratedEvent {
    pub id: i32,
    pub signature: String,
    pub pool: String,
    pub event_type: String,
    pub version: i32,
    pub timestamp: DateTime<Utc>,
}

impl RaydiumConcentratedEvent {
    pub fn new(signature: String, pool: Pubkey, event_type: RaydiumEventType) -> Self {
        Self {
            id: 0, // Will be set by the database
            signature,
            pool: pool.to_string(),
            event_type: event_type.to_string(),
            version: 1,
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct RaydiumSwapRecord {
    pub event_id: i32,
    pub in_token_amount: i64,
    pub out_token_amount: i64,
    pub fee_amount: i64,
    pub price: f64,
}

// Composite record
#[derive(Debug)]
pub struct RaydiumSwapEventRecord {
    pub base: RaydiumConcentratedEvent,
    pub data: RaydiumSwapRecord,
}
```

## 3. Create Repository

Create a new repository in `src/db/repositories/raydium.rs`:

```rust
/******************************************************************************
 * IMPORTANT: DO NOT MODIFY THIS FILE WITHOUT EXPLICIT APPROVAL
 *
 * This file is protected and should not be modified without explicit approval.
 * Any changes could break the indexer functionality.
 *
 * See .nooverwrite.json for more information on protected files.
 ******************************************************************************/

use anyhow::{Context, Result};
use sqlx::{PgPool, Postgres, Transaction, Row};

use crate::db::common::Repository;
use crate::models::raydium::concentrated::{
    RaydiumConcentratedEvent,
    RaydiumSwapEventRecord,
};

pub struct RaydiumRepository {
    pool: PgPool,
}

impl RaydiumRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn insert_base_event<'a>(
        &self,
        tx: &mut Transaction<'a, Postgres>,
        event: &RaydiumConcentratedEvent
    ) -> Result<i32> {
        let row = sqlx
            ::query(
                "INSERT INTO apestrong.raydium_concentrated_events (signature, pool, event_type, version) VALUES ($1, $2, $3, $4) RETURNING id"
            )
            .bind(&event.signature)
            .bind(&event.pool)
            .bind(&event.event_type)
            .bind(event.version)
            .fetch_one(&mut **tx).await
            .context("Failed to insert base Raydium event")?;

        let id: i32 = row.get("id");
        Ok(id)
    }

    pub async fn insert_swap_event(&self, event: RaydiumSwapEventRecord) -> Result<i32> {
        let mut tx = self.pool.begin().await?;

        // Insert the base event
        let event_id = self.insert_base_event(&mut tx, &event.base).await?;

        // Insert the swap-specific data
        sqlx
            ::query(
                "INSERT INTO apestrong.raydium_swap_events (event_id, in_token_amount, out_token_amount, fee_amount, price) VALUES ($1, $2, $3, $4, $5)"
            )
            .bind(event_id)
            .bind(event.data.in_token_amount)
            .bind(event.data.out_token_amount)
            .bind(event.data.fee_amount)
            .bind(event.data.price)
            .execute(&mut *tx).await
            .context("Failed to insert Raydium swap event")?;

        tx.commit().await?;
        Ok(event_id)
    }

    // Add methods for other event types as needed

    // Query method example
    pub async fn get_recent_trade_volume(&self, pool_address: &str, hours: i64) -> Result<i64> {
        let row = sqlx
            ::query(
                "SELECT COALESCE(SUM(s.in_token_amount), 0) as volume FROM apestrong.raydium_concentrated_events e JOIN apestrong.raydium_swap_events s ON e.id = s.event_id WHERE e.pool = $1 AND e.event_type = 'Swap' AND e.timestamp > NOW() - INTERVAL '1 hour' * $2"
            )
            .bind(pool_address)
            .bind(hours)
            .fetch_one(&self.pool).await
            .context("Failed to get recent trade volume")?;

        let volume: Option<i64> = row.get("volume");
        Ok(volume.unwrap_or(0))
    }
}

impl Repository for RaydiumRepository {
    fn pool(&self) -> &PgPool {
        &self.pool
    }
}
```

Update `src/db/repositories/mod.rs` to include the new repository:

```rust
pub mod orca;
pub mod raydium;  // Add this line

pub use orca::*;
pub use raydium::*;  // Add this line
```

## 4. Create an Indexer

Create an indexer in `src/indexers/raydium.rs`:

```rust
/******************************************************************************
 * IMPORTANT: DO NOT MODIFY THIS FILE WITHOUT EXPLICIT APPROVAL
 *
 * This file is protected and should not be modified without explicit approval.
 * Any changes could break the indexer functionality.
 *
 * See .nooverwrite.json for more information on protected files.
 ******************************************************************************/

use anyhow::{Context, Result};
use futures::stream::StreamExt;
use solana_client::{
    nonblocking::{pubsub_client::PubsubClient, rpc_client::RpcClient},
    rpc_config::{RpcTransactionConfig, RpcTransactionLogsConfig, RpcTransactionLogsFilter},
    rpc_client::GetConfirmedSignaturesForAddress2Config,
    rpc_response::RpcLogsResponse,
};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signature};
use solana_transaction_status::UiTransactionEncoding;
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use base64::engine::general_purpose;
use base64::Engine as _;

use crate::db::repositories::RaydiumRepository;
use crate::models::raydium::concentrated::{
    SWAP_EVENT_DISCRIMINATOR,
    RaydiumEventType,
    RaydiumSwapEvent,
    RaydiumConcentratedEvent,
    RaydiumSwapRecord,
    RaydiumSwapEventRecord,
};

/// Raydium concentrated liquidity indexer
pub struct RaydiumIndexer {
    repository: RaydiumRepository,
}

impl RaydiumIndexer {
    /// Create a new indexer with the given repository
    pub fn new(repository: RaydiumRepository) -> Self {
        Self { repository }
    }

    /// Start indexing events for the given pools
    pub async fn start(&self, rpc_url: &str, ws_url: &str, pools: &[Pubkey]) -> Result<()> {
        // Create a shared pool set for filtering events
        let active_pools: Arc<RwLock<HashSet<Pubkey>>> = Arc::new(RwLock::new(HashSet::new()));
        {
            let mut pool_set = active_pools.write().unwrap();
            for pool in pools {
                pool_set.insert(*pool);
            }
        }

        // Initialize RPC client for backfilling
        let rpc_client = RpcClient::new_with_commitment(
            rpc_url.to_string(),
            CommitmentConfig::confirmed()
        );

        // Backfill recent events
        for pool in pools {
            println!("Backfilling data for pool {}", pool);
            self.backfill_pool(&rpc_client, pool).await?;
        }

        // Start live event subscription
        let pubsub_client = PubsubClient::new(ws_url).await?;

        println!("Starting live event subscription...");
        let (mut log_stream, _) = pubsub_client.logs_subscribe(
            RpcTransactionLogsFilter::All,
            RpcTransactionLogsConfig {
                commitment: Some(CommitmentConfig::confirmed()),
            }
        ).await?;

        println!("Monitoring Raydium logs for {} pools...", pools.len());
        while let Some(response) = log_stream.next().await {
            self.process_log(&response.value, &active_pools).await?;
        }

        Ok(())
    }

    /// Process a log response
    async fn process_log(
        &self,
        log: &RpcLogsResponse,
        active_pools: &Arc<RwLock<HashSet<Pubkey>>>
    ) -> Result<()> {
        // Implementation similar to OrcaWhirlpoolIndexer
        // Process Raydium events based on their discriminators

        // This is a simplified example
        Ok(())
    }

    /// Backfill events for a single pool
    async fn backfill_pool(&self, rpc_client: &RpcClient, pool_pubkey: &Pubkey) -> Result<()> {
        // Implementation similar to OrcaWhirlpoolIndexer
        // Fetch and process historical transactions

        // This is a simplified example
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
```

Update `src/indexers/mod.rs`:

```rust
pub mod orca;
pub mod raydium;  // Add this line

pub use orca::*;
pub use raydium::*;  // Add this line
```

## 5. Update the Main Application

Finally, update `src/main.rs` to add a new command for the Raydium indexer:

```rust
use anyhow::{ Context, Result };
use clap::{ Parser, Subcommand };
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use indexer::{
    db::{ Database, DbConfig },
    db::repositories::{ OrcaWhirlpoolRepository, RaydiumRepository }, // Add RaydiumRepository
    indexers::{ OrcaWhirlpoolIndexer, RaydiumIndexer }, // Add RaydiumIndexer
};

// Default values
const DEFAULT_RPC_URL: &str = "https://api.mainnet-beta.solana.com";
const DEFAULT_WS_URL: &str = "wss://api.mainnet-beta.solana.com";
const DEFAULT_ORCA_POOL: &str = "Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE"; // SOL/USDC pool
const DEFAULT_RAYDIUM_POOL: &str = "RaydiumPoolAddressHere"; // Replace with an actual Raydium pool

/// Solana DEX indexer CLI
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Solana RPC URL
    #[arg(long, default_value = DEFAULT_RPC_URL)]
    rpc_url: String,

    /// Solana WebSocket URL
    #[arg(long, default_value = DEFAULT_WS_URL)]
    ws_url: String,

    /// Indexer command to run
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run the Orca Whirlpool indexer
    Orca {
        /// Comma-separated list of pool addresses to index
        #[arg(long, use_value_delimiter = true, value_delimiter = ',')]
        pools: Option<Vec<String>>,
    },
    /// Run the Raydium indexer
    Raydium {
        /// Comma-separated list of pool addresses to index
        #[arg(long, use_value_delimiter = true, value_delimiter = ',')]
        pools: Option<Vec<String>>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file if present
    dotenv::dotenv().ok();

    // Parse command line arguments
    let cli = Cli::parse();

    // Get database configuration
    let db_config = DbConfig::from_env().context("Failed to get database configuration")?;

    // Connect to the database
    let db = Database::connect(db_config).await.context("Failed to connect to database")?;

    match &cli.command {
        Command::Orca { pools } => {
            println!("Starting Orca Whirlpool indexer...");

            // Parse pool addresses
            let pool_pubkeys = match pools {
                Some(addresses) => {
                    // Convert string addresses to Pubkeys
                    let mut pubkeys = Vec::new();
                    for addr in addresses {
                        let pubkey = Pubkey::from_str(addr).context(
                            format!("Invalid Solana address: {}", addr)
                        )?;
                        pubkeys.push(pubkey);
                    }
                    pubkeys
                }
                None => {
                    // Use default pool if none specified
                    vec![
                        Pubkey::from_str(DEFAULT_ORCA_POOL).context(
                            "Failed to parse default Orca pool address"
                        )?
                    ]
                }
            };

            // Create repository and indexer
            let repository = OrcaWhirlpoolRepository::new(db.pool().clone());
            let indexer = OrcaWhirlpoolIndexer::new(repository);

            // Start the indexer
            indexer
                .start(&cli.rpc_url, &cli.ws_url, &pool_pubkeys).await
                .context("Orca indexer failed")?;
        }
        Command::Raydium { pools } => {
            println!("Starting Raydium indexer...");

            // Parse pool addresses
            let pool_pubkeys = match pools {
                Some(addresses) => {
                    // Convert string addresses to Pubkeys
                    let mut pubkeys = Vec::new();
                    for addr in addresses {
                        let pubkey = Pubkey::from_str(addr).context(
                            format!("Invalid Solana address: {}", addr)
                        )?;
                        pubkeys.push(pubkey);
                    }
                    pubkeys
                }
                None => {
                    // Use default pool if none specified
                    vec![
                        Pubkey::from_str(DEFAULT_RAYDIUM_POOL).context(
                            "Failed to parse default Raydium pool address"
                        )?
                    ]
                }
            };

            // Create repository and indexer
            let repository = RaydiumRepository::new(db.pool().clone());
            let indexer = RaydiumIndexer::new(repository);

            // Start the indexer
            indexer
                .start(&cli.rpc_url, &cli.ws_url, &pool_pubkeys).await
                .context("Raydium indexer failed")?;
        }
    }

    Ok(())
}
```

## 6. Update Library Exports

Update `src/lib.rs` to export the new Raydium components:

```rust
// Re-export core modules
pub mod models;
pub mod db;
pub mod indexers;

// Re-export common types and traits
pub use models::common::Protocol;
pub use db::{ Database, DbConfig };

// Re-export protocol-specific components
pub use models::orca::whirlpool::{
    TRADED_EVENT_DISCRIMINATOR,
    LIQUIDITY_INCREASED_DISCRIMINATOR,
    LIQUIDITY_DECREASED_DISCRIMINATOR,
    OrcaWhirlpoolEventType,
    OrcaWhirlpoolTradedEvent,
    OrcaWhirlpoolLiquidityIncreasedEvent,
    OrcaWhirlpoolLiquidityDecreasedEvent,
};

// Raydium exports
pub use models::raydium::concentrated::{
    SWAP_EVENT_DISCRIMINATOR,
    RaydiumEventType,
    RaydiumSwapEvent,
};

pub use db::repositories::OrcaWhirlpoolRepository;
pub use db::repositories::RaydiumRepository;
pub use indexers::OrcaWhirlpoolIndexer;
pub use indexers::RaydiumIndexer;
```

## 7. Testing

After implementing these changes, test that your new DEX integration works:

1. Update the database schema: `cargo run --bin setup_db`
2. Run the Raydium indexer: `cargo run -- raydium`
3. Verify that events are being captured in the database:
   ```sql
   SELECT * FROM apestrong.raydium_concentrated_events LIMIT 10;
   ```

## 8. Update Protection Settings

Add the new protected files to `.nooverwrite.json`:

```json
{
  "protectedFiles": [
    {
      "path": "src/models/raydium/concentrated.rs",
      "reason": "Contains critical Raydium model definitions.",
      "lastUpdated": "2025-04-04",
      "owner": "core-team"
    },
    {
      "path": "src/db/repositories/raydium.rs",
      "reason": "Contains critical Raydium database operations.",
      "lastUpdated": "2025-04-04",
      "owner": "core-team"
    },
    {
      "path": "src/indexers/raydium.rs",
      "reason": "Contains critical Raydium indexer implementation.",
      "lastUpdated": "2025-04-04",
      "owner": "core-team"
    }
  ]
}
```

## Conclusion

By following this pattern, you can add support for any DEX protocol to the indexer. The modular architecture ensures that each protocol's specific logic is isolated, making maintenance and updates easier. The CLI-based approach allows users to easily select which DEX and pools to index.

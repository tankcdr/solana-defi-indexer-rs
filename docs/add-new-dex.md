# Adding a New DEX

This guide walks through the process of adding support for a new decentralized exchange (DEX) to the event indexer.

## Overview

Adding a new DEX involves several steps:

1. Create database tables for the new DEX's events
2. Define model structures for the on-chain events
3. Implement a repository for database operations
4. Build an indexer that implements the `DexIndexer` trait
5. Update the main application with a new CLI command

The core of our architecture is the `DexIndexer` trait, which provides:

- A standardized interface for all DEX implementations
- Default implementations for common operations (processing logs, backfilling, etc.)
- Protocol-specific hooks for customizing event parsing and handling
- Robust error handling and recovery strategies

Let's go through each step in detail using a hypothetical "Raydium" DEX as an example.

## 1. Database Schema

First, create a directory for the new DEX's schema files:

```bash
mkdir -p database/schema/raydium
```

Then create the schema files for the new DEX:

1. `database/schema/raydium/schema.sql` - Schema creation SQL:

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

2. `database/schema/raydium/delete.sql` - Schema deletion SQL:

```sql
-- Drop views first
DROP VIEW IF EXISTS apestrong.v_raydium_concentrated_swap;

-- Drop event-specific tables
DROP TABLE IF EXISTS apestrong.raydium_swap_events;

-- Drop base table
DROP TABLE IF EXISTS apestrong.raydium_concentrated_events;
```

3. `database/schema/raydium/subscribed_pools.txt` - Pools to monitor:

```
# Example Raydium Concentrated Liquidity Pools
# SOL-USDC Pool
RaydiumPoolAddress1
# Another pool
RaydiumPoolAddress2
```

Apply the schema changes using the database utilities:

```bash
# Create just the Raydium schema
./database/dbutil.sh create raydium

# Or update all schemas
./database/dbutil.sh create all
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

## 4. Create an Indexer that Implements the DexIndexer Trait

Create an indexer in `src/indexers/raydium.rs` that implements the `DexIndexer` trait:

```rust
use anyhow::{Context, Result};
use solana_client::rpc_response::RpcLogsResponse;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashSet;
use std::str::FromStr;
use sqlx::PgPool;
use async_trait::async_trait;

use crate::db::repositories::RaydiumRepository;
use crate::indexers::dex_indexer::DexIndexer;
use crate::models::raydium::concentrated::{
    SWAP_EVENT_DISCRIMINATOR,
    RaydiumEventType,
    RaydiumSwapEvent,
    RaydiumConcentratedEvent,
    RaydiumSwapRecord,
    RaydiumSwapEventRecord,
};

// Default Raydium pool
const DEFAULT_RAYDIUM_POOL: &str = "RaydiumPoolAddressHere";
const DEX: &str = "raydium";

/// Represents a parsed event from Raydium logs
#[derive(Debug)]
pub enum RaydiumParsedEvent {
    Swap(RaydiumSwapEvent, String), // Event and signature
    // Add more event types as needed
}

/// Raydium indexer
pub struct RaydiumIndexer {
    repository: RaydiumRepository,
    pool_pubkeys: HashSet<Pubkey>,
}

impl RaydiumIndexer {
    /// Create a new indexer with the given repository and pool set
    pub fn new(repository: RaydiumRepository, pool_pubkeys: HashSet<Pubkey>) -> Self {
        Self { repository, pool_pubkeys }
    }

    /// Create an indexer instance with a freshly initialized repository and default pool
    pub fn create(db_pool: PgPool) -> Result<Self> {
        // Create a singleton pool set with the default pool
        let mut pool_pubkeys = HashSet::new();
        pool_pubkeys.insert(
            Pubkey::from_str(DEFAULT_RAYDIUM_POOL).context(
                "Failed to parse default Raydium pool address"
            )?
        );

        let repository = RaydiumRepository::new(db_pool);
        Ok(Self::new(repository, pool_pubkeys))
    }

    /// Create an indexer and resolve pool addresses in one operation
    pub async fn create_with_pools(
        db_pool: PgPool,
        provided_pools: Option<&Vec<String>>
    ) -> Result<Self> {
        // Create the repository for database access
        let repository = RaydiumRepository::new(db_pool.clone());

        // Resolve pool addresses
        let pool_pubkeys = repository.get_pools_with_fallback(
            provided_pools,
            DEFAULT_RAYDIUM_POOL
        ).await?;

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

        Ok(Self::new(repository, pool_pubkeys))
    }

    /// Create a base event record
    fn create_base_event(
        &self,
        signature: &str,
        pool: &Pubkey,
        event_type: RaydiumEventType
    ) -> RaydiumConcentratedEvent {
        RaydiumConcentratedEvent {
            id: 0, // Will be set by database
            signature: signature.to_string(),
            pool: pool.to_string(),
            event_type: event_type.to_string(),
            version: 1,
            timestamp: chrono::Utc::now(),
        }
    }
}
#[async_trait::async_trait]
impl DexIndexer for RaydiumIndexer {
type Repository = RaydiumRepository;
type ParsedEvent = RaydiumParsedEvent;

fn program_ids(&self) -> Vec<&str> {
    vec!["675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"] // Raydium program ID
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

/// Parse events from a log, returning any found events without persisting them
async fn parse_log_events(&self, log: &RpcLogsResponse) -> Result<Vec<Self::ParsedEvent>> {
    // Quick initial check for relevant event keywords
    let contains_relevant_events = log.logs
        .iter()
        .any(|line| {
            line.contains("Swap") ||
                line.contains("CreatePosition") ||
                line.contains("ClosePosition")
        });

    if !contains_relevant_events {
        return Ok(Vec::new());
    }

    let mut events = Vec::new();

    // Extract and process events
    // Implementation similar to OrcaWhirlpoolIndexer
    // Process Raydium events based on their discriminators

    // This is a simplified example - you would add actual event parsing logic here

    Ok(events)
}
    }

    /// Handle a single event (for both real-time and backfill processing)
    async fn handle_event(&self, event: Self::ParsedEvent) -> Result<()> {
        match event {
            RaydiumParsedEvent::Swap(event_data, signature) => {
                // Create the base event
                let base_event = self.create_base_event(
                    &signature,
                    &event_data.pool,
                    RaydiumEventType::Swap
                );

                // Create the data record
                let data = RaydiumSwapRecord {
                    event_id: 0, // Will be set after base event is inserted
                    in_token_amount: event_data.in_token_amount as i64,
                    out_token_amount: event_data.out_token_amount as i64,
                    fee_amount: event_data.fee_amount as i64,
                    price: event_data.price as f64,
                };

                let event_record = RaydiumSwapEventRecord {
                    base: base_event,
                    data,
                };

                self.repository.insert_swap_event(event_record).await?;
            }
            // Add handling for other event types
        }

        Ok(())
    }
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

### Local Testing

After implementing these changes, test that your new DEX integration works:

1. Update the database schema:

   ```bash
   ./database/dbutil.sh create raydium
   ```

2. Load pool addresses:

   ```bash
   ./database/load_pools.sh raydium
   ```

3. Run the Raydium indexer:

   ```bash
   cargo run -- raydium
   ```

4. Verify that events are being captured in the database:
   ```sql
   SELECT * FROM apestrong.raydium_concentrated_events LIMIT 10;
   ```

### Docker Testing

You can also test your implementation using Docker:

1. Build the Docker images with your new DEX implementation:

   ```bash
   docker compose build
   ```

2. Run the stack with your new DEX type:

   ```bash
   DEX_TYPE=raydium docker compose up -d
   ```

3. Check the logs to verify operation:

   ```bash
   docker compose logs -f
   ```

4. Connect to the database to verify event capture:
   ```bash
   docker exec -it prediction-market-indexer_db_1 psql -U postgres -c "SELECT * FROM apestrong.raydium_concentrated_events LIMIT 10;"
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

## 9. Docker Integration

If using Docker, ensure your new DEX is properly integrated:

1. Update environment handling in `Dockerfile.indexer` if needed
2. Test the DEX_TYPE environment variable recognition
3. Verify that pool loading works correctly in the Docker environment

You can add a dedicated section to the `docs/docker-setup.md` documentation explaining how to use your new DEX with Docker:

````markdown
### Running Raydium Indexer with Docker

To run the Raydium indexer:

```bash
DEX_TYPE=raydium docker compose up -d
```
````

This will automatically:

1. Create the Raydium schema
2. Load the pools from `database/schema/raydium/subscribed_pools.txt`
3. Start the indexer for Raydium events

```
## Conclusion

By following this pattern, you can add support for any DEX protocol to the indexer. The modular architecture ensures that each protocol's specific logic is isolated, making maintenance and updates easier.

The system provides multiple ways to interact with your implementation:

1. **CLI-based approach** allows users to easily select which DEX and pools to index
2. **Docker-based deployment** provides containerized setup with minimal configuration
3. **Database utilities** make schema management and pool tracking consistent across DEXes

This flexibility ensures that your DEX implementation can be used in various deployment scenarios, from development to production.
By following this pattern, you can add support for any DEX protocol to the indexer. The modular architecture ensures that each protocol's specific logic is isolated, making maintenance and updates easier. The CLI-based approach allows users to easily select which DEX and pools to index.
```

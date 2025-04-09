# DEX Event Indexer Architecture

This document describes the architecture of the DEX Event Indexer, explaining its components and their interactions.

## Overview

The DEX Event Indexer follows a modular, protocol-oriented architecture that provides a clear separation of concerns and makes it easy to add support for new DEXs.

## Directory Structure

```
indexer/
├── src/
│   ├── models/                  # Data models
│   │   ├── common.rs            # Protocol enum and shared types
│   │   ├── orca/                # Orca-specific models
│   │   │   ├── mod.rs           # Module exports
│   │   │   └── whirlpool.rs     # Whirlpool specific types
│   │   └── raydium/             # Raydium-specific models (in progress)
│   │       ├── mod.rs           # Module exports
│   │       └── concentrated.rs  # Concentrated liquidity types
│   ├── db/                      # Database layer
│   │   ├── common.rs            # Repository trait
│   │   ├── pool.rs              # Database connection management
│   │   ├── signature_store.rs   # Tracks processed signatures
│   │   └── repositories/        # Protocol-specific repositories
│   │       ├── mod.rs           # Repository exports
│   │       ├── orca.rs          # Orca database operations
│   │       ├── orca_pools.rs    # Orca pool management
│   │       ├── orca_batch.rs    # Batch operations for Orca
│   │       └── raydium.rs       # Raydium database operations
│   ├── indexers/                # Event processing logic
│   │   ├── mod.rs               # Indexer exports
│   │   ├── orca.rs              # Orca event handling
│   │   └── raydium.rs           # Raydium event handling
│   ├── websocket_manager.rs     # WebSocket connection management
│   ├── backfill_manager.rs      # Historical data backfilling
│   ├── lib.rs                   # Library exports
│   └── main.rs                  # CLI entry point with command parsing
├── database/                    # Database setup and utilities
│   ├── schema/                  # SQL schema definitions
│   │   ├── common/              # Shared tables across DEXes
│   │   │   ├── schema.sql       # Common table definitions
│   │   │   └── delete.sql       # SQL to delete common tables
│   │   ├── orca/                # Orca-specific schema
│   │   │   ├── schema.sql       # Orca table definitions
│   │   │   ├── delete.sql       # SQL to delete Orca tables
│   │   │   └── subscribed_pools.txt # Orca pools to monitor
│   │   └── raydium/             # Raydium-specific schema
│   │       ├── schema.sql       # Raydium table definitions
│   │       ├── delete.sql       # SQL to delete Raydium tables
│   │       └── subscribed_pools.txt # Raydium pools to monitor
│   ├── models/                  # Rust models for DB utilities
│   │   ├── mod.rs               # Common interfaces
│   │   └── orca.rs              # Orca-specific implementations
│   ├── dbutil.rs                # Database schema utility
│   ├── dbutil.sh                # Shell wrapper for dbutil
│   ├── load_pools.rs            # Pool loading utility
│   ├── load_pools.sh            # Shell wrapper for load_pools
│   ├── README.md                # Database utilities documentation
│   └── init_database.sh         # Docker initialization script
├── docker-compose.yml           # Docker Compose configuration
├── Dockerfile.indexer           # Dockerfile for the indexer
├── Dockerfile.init              # Dockerfile for database initialization
├── Dockerfile.postgres          # Dockerfile for PostgreSQL setup
└── docs/                        # Documentation
```

## Key Components

### Models

The `models` directory contains the data structures for both on-chain events and database records. These are organized by protocol:

- `models/common.rs`: Defines the `Protocol` enum and shared types used across all DEXes
- `models/orca/whirlpool.rs`: Defines Orca Whirlpool-specific structures:
  - Event discriminators (identifying bytes for each event type)
  - On-chain event structures (deserialized from transaction logs)
  - Database records (matching the database schema)
  - Composite record types (combining base and specific event data)
- `models/raydium/concentrated.rs`: Defines Raydium-specific structures (similar organization)

This file includes three main event types for Orca Whirlpool:

- `Traded` (swap events)
- `LiquidityIncreased` (liquidity addition events)
- `LiquidityDecreased` (liquidity removal events)

### Database Layer

The `db` directory handles all database interactions:

- `db/pool.rs`: Manages database connection pooling using SQLx
- `db/common.rs`: Defines the `Repository` trait that all repositories implement
- `db/signature_store.rs`: Tracks processed transaction signatures to avoid duplicates
- `db/repositories/`:
  - `orca.rs`: Implements database operations for Orca Whirlpool events
  - `orca_pools.rs`: Manages Orca pool data
  - `orca_batch.rs`: Provides optimized batch operations for high-volume processing
  - `raydium.rs`: Implements database operations for Raydium events

The repository implements methods for:

- Inserting base events
- Inserting traded (swap) events
- Inserting liquidity increased events
- Inserting liquidity decreased events
- Querying recent trade volume

All database tables are created under the 'apestrong' schema to keep them organized.

### Indexers

The `indexers` directory contains the event processing logic for each protocol:

- `indexers/orca.rs`: Handles Orca Whirlpool event indexing by:
  - Subscribing to Solana transaction logs via WebSocket
  - Parsing event data using Borsh deserialization
  - Filtering events by monitored pool addresses
  - Storing events in the database via the repository
  - Backfilling historical events for complete data
- `indexers/raydium.rs`: Handles Raydium event indexing with a similar approach

The separation allows each indexer to handle the unique aspects of its protocol while sharing common patterns for WebSocket management and backfilling.

### Additional Components

- `websocket_manager.rs`: Provides WebSocket connection management and reconnection logic
- `backfill_manager.rs`: Implements historical event recovery and processing

### Main Application

`main.rs` is the entry point that:

1. Parses command-line arguments using Clap
2. Loads configuration from .env file via dotenv
3. Establishes database connections
4. Creates repositories and indexers based on the specified command
5. Starts the indexing processes for the selected DEX

### Database Utilities

The `database` directory contains utilities for schema management and pool tracking:

- `dbutil.rs`/`dbutil.sh`: Tools for creating and deleting database schemas
- `load_pools.rs`/`load_pools.sh`: Tools for loading pool addresses into the database
- Schema files are organized by DEX in the `schema` directory
- `subscribed_pools.txt` files contain addresses of the pools to monitor

### Docker Infrastructure

The project includes Docker support for containerized deployment:

- `docker-compose.yml`: Defines a multi-container setup with:
  - PostgreSQL database
  - Database initialization service
  - DEX indexer service
- Environment variables like `DEX_TYPE` control which DEX is indexed

## Data Flow

1. **Event Source**: The indexer subscribes to transaction logs from the Solana blockchain via WebSocket
2. **Event Filtering**: For each transaction log, the indexer checks if it contains events from monitored DEX pools
3. **Event Parsing**: The indexer parses the event data using protocol-specific discriminators and Borsh deserialization
4. **Database Storage**: Events are stored in the appropriate database tables with transactions ensuring data integrity
5. **Backfilling**: The indexer can also retrieve historical events for monitored pools to ensure data completeness

## Protocol Abstraction

Each DEX protocol is treated as a self-contained module with its own:

1. **Data Models**: Defining the structure of its events
2. **Repository**: Managing its database operations
3. **Indexer**: Processing its events

This allows each protocol to be implemented independently without affecting others.

## Adding New Protocols

When adding a new DEX, each component is added in isolation:

1. Add protocol-specific models in `models/[protocol]/`
2. Create database schema files in `database/schema/[protocol]/`
3. Implement a repository in `db/repositories/[protocol].rs`
4. Create an indexer in `indexers/[protocol].rs`
5. Add a new command in `main.rs` to enable the new indexer
6. Update the Docker files if deploying with containers

This modular approach allows for consistent extension without disrupting existing functionality. For detailed instructions, see [Adding a New DEX](./add-new-dex.md).
For detailed instructions, see [Adding a New DEX](./add-new-dex.md).

## Design Decisions

### Protocol-Specific Database Tables

Each protocol has its own set of database tables rather than a general-purpose table structure. This approach:

- Accommodates the unique event structures of each DEX
- Allows for protocol-specific optimizations
- Simplifies queries by avoiding complex joins or polymorphic data

### Event Type Separation

Event types (Traded, LiquidityIncreased, LiquidityDecreased) are kept in separate tables even when their structures are similar, ensuring:

- Semantic clarity about the nature of each event
- The ability to add event-specific fields in the future
- Efficient queries for specific event types

### CLI-Based Configuration

The application uses a command-line interface to:

- Select which DEX to index
- Specify which pools to monitor
- Configure RPC and WebSocket endpoints

This makes it easy to deploy multiple instances monitoring different DEXs or pools.

### Docker-Based Deployment

The application can also be deployed using Docker:

- Environment variables control which DEX to index
- Pool configurations are loaded during container initialization
- Multiple container instances can run concurrently
- Data persistence is managed through Docker volumes

### Async Processing

The indexer uses Tokio for asynchronous processing, allowing it to:

- Handle high volumes of transactions efficiently
- Process blockchain events and database operations concurrently
- Maintain WebSocket connections while performing other tasks

### Database Organization

The database schema follows these principles:

- Common tables shared across all DEXes are in the `common` schema
- Each DEX has its own set of tables and views
- The `apestrong` schema contains all tables
- Indexes are created for frequently queried columns
- Pool addresses are tracked in a central table
- Last processed signatures are stored for recovery

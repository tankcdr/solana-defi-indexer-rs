# DEX Event Indexer Architecture

This document describes the architecture of the DEX Event Indexer, explaining its components and their interactions.

## Overview

The DEX Event Indexer follows a modular, protocol-oriented architecture that provides a clear separation of concerns and makes it easy to add support for new DEXs.

## Directory Structure

```
indexer/
├── src/
│   ├── models/               # Data models
│   │   ├── common.rs         # Protocol enum and shared types
│   │   └── orca/             # Orca-specific models
│   │       └── whirlpool.rs  # Whirlpool specific types
│   ├── db/                   # Database layer
│   │   ├── common.rs         # Repository trait
│   │   ├── pool.rs           # Database connection management
│   │   └── repositories/     # Protocol-specific repositories
│   │       └── orca.rs       # Orca database operations
│   ├── indexers/             # Event processing logic
│   │   └── orca.rs           # Orca event handling
│   ├── lib.rs                # Library exports
│   └── main.rs               # CLI entry point with command parsing
├── database/                 # Database setup
│   ├── schema.sql            # Schema definition
│   └── setup_db.rs           # Database initialization script
└── docs/                     # Documentation
```

## Key Components

### Models

The `models` directory contains the data structures for both on-chain events and database records. These are organized by protocol:

- `models/common.rs`: Defines the `Protocol` enum and any shared types
- `models/orca/whirlpool.rs`: Defines Orca Whirlpool-specific structures:
  - Event discriminators (identifying bytes for each event type)
  - On-chain event structures (deserialized from transaction logs)
  - Database records (matching the database schema)
  - Composite record types (combining base and specific event data)

This file includes three main event types for Orca Whirlpool:

- `Traded` (swap events)
- `LiquidityIncreased` (liquidity addition events)
- `LiquidityDecreased` (liquidity removal events)

### Database Layer

The `db` directory handles all database interactions:

- `db/pool.rs`: Manages database connection pooling using SQLx
- `db/common.rs`: Defines the `Repository` trait that all repositories implement
- `db/repositories/orca.rs`: Implements database operations for Orca Whirlpool events

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

### Main Application

`main.rs` is the entry point that:

1. Parses command-line arguments using Clap
2. Loads configuration from .env file via dotenv
3. Establishes database connections
4. Creates repositories and indexers based on the specified command
5. Starts the indexing processes for the selected DEX

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
2. Create database tables in `database/schema.sql`
3. Implement a repository in `db/repositories/[protocol].rs`
4. Create an indexer in `indexers/[protocol].rs`
5. Add a new command in `main.rs` to enable the new indexer

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

### Async Processing

The indexer uses Tokio for asynchronous processing, allowing it to:

- Handle high volumes of transactions efficiently
- Process blockchain events and database operations concurrently
- Maintain WebSocket connections while performing other tasks

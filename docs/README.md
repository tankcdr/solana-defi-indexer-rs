# DEX Event Indexer Documentation

Welcome to the DEX Event Indexer documentation. This document serves as an entry point to understand, use, and extend the DEX Event Indexer.

## Table of Contents

- [Overview](#overview)
- [Getting Started](#getting-started)
- [Architecture](#architecture)
- [Database Schema](#database-schema)
- [CLI Usage](#cli-usage)
- [Adding New DEXs](#adding-new-dexs)

## Overview

The DEX Event Indexer is a Rust application that indexes events from decentralized exchanges (DEXs) on the Solana blockchain. It captures various events such as swaps, liquidity additions, and liquidity removals, storing them in a PostgreSQL database for later analysis.

The indexer monitors Solana transactions in real-time by subscribing to WebSocket logs and also provides backfilling capabilities for historical events.

Currently supported DEXs:

- **Orca Whirlpool** (Concentrated Liquidity Pools) - Fully implemented
- **Raydium** (Concentrated Liquidity Pools) - Implementation in progress

The system is designed to be modular and extensible through the `DexIndexer` trait, allowing new DEXes to be added with minimal changes to the existing codebase.

## Getting Started

- [Setup Instructions](./setup.md)
- [Docker Setup](./docker-setup.md) (Recommended)
- [Running the Indexer](./running.md)
- [CLI Usage](./cli-usage.md)
- [Database Utilities](../database/README.md)

## Architecture

The indexer follows a modular, protocol-oriented architecture built around the `DexIndexer` trait:

- **Models**: Define the structures for blockchain events and database tables
  - Organized by protocol (e.g., `models/orca/whirlpool.rs`, `models/raydium/concentrated.rs`)
  - Common models shared across protocols (`models/common.rs`)
- **DB**: Handle database connections and operations
  - Connection pool management
  - Protocol-specific repositories
  - Signature tracking to prevent duplicate processing
- **Indexers**: Process blockchain events and store them in the database
  - Core `DexIndexer` trait with default implementations for common operations
  - Protocol-specific indexers implementing the trait
  - WebSocket-based real-time monitoring
  - Event buffering during backfill operations
  - Scheduled backfilling for missed events

All database tables are created under the 'apestrong' schema for organization. The schema is structured into:

- **Common tables**: Shared across all DEXes (token metadata, subscribed pools, signature tracking)
- **Protocol-specific tables**: Each DEX has its own set of tables for events
- **Views**: For convenient querying of complex data relationships

Each protocol (DEX) has its own set of models, database tables, and indexing logic, allowing for easy extension. The database schema can be created and managed using the utilities in the `database` directory.

Read more in the [Architecture Documentation](./architecture.md) and [Database README](../database/README.md).

## Database Schema

The indexer stores event data in a structured PostgreSQL database. The schema is designed to efficiently store and query various types of DEX events:

- **Common Schema** (`database/schema/common/`)

  - `apestrong.token_metadata`: Stores token information
  - `apestrong.subscribed_pools`: Tracks monitored pools
  - `apestrong.last_signatures`: Tracks the last processed event for each pool

- **Orca Schema** (`database/schema/orca/`)

  - Base table for all events: `apestrong.orca_whirlpool_events`
  - Event-specific tables: `orca_traded_events`, `orca_liquidity_increased_events`, etc.
  - Views for easier querying: `v_orca_whirlpool_traded`, etc.

- **Raydium Schema** (`database/schema/raydium/`)
  - Similar structure to Orca, with Raydium-specific tables

For detailed information about the database tables, columns, relationships, and sample queries, see the [Database Schema Documentation](./database-schema.md).

## CLI Usage

The indexer supports a command-line interface for specifying which DEX to monitor and which pools to index:

```
indexer [global options] <command> [command options]
```

For example, to monitor a specific Orca Whirlpool pool:

```
cargo run -- orca --pools Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE
```

Alternatively, you can use Docker Compose for a simpler setup:

```bash
DEX_TYPE=orca docker compose up -d
```

See the [CLI Usage Guide](./cli-usage.md) and [Docker Setup](./docker-setup.md) for more details.

## Adding New DEXs

We've designed the system to be easily extendable with new DEXs through the `DexIndexer` trait:

1. Create database schema files in `database/schema/<dex_name>/`
2. Create model structures in `src/models/<dex_name>/`
3. Implement a repository in `src/db/repositories/<dex_name>.rs`
4. Create an indexer in `src/indexers/<dex_name>.rs` that implements the `DexIndexer` trait
5. Update the main application to support the new DEX

The `DexIndexer` trait provides:

- A standardized interface for all DEX implementations
- Default implementations for common operations (log processing, backfilling, etc.)
- Protocol-specific hooks for customizing event parsing and handling
- Robust error handling and recovery strategies
- Event buffering during backfill operations

For complete step-by-step instructions with code examples, see [How to Add a New DEX](./add-new-dex.md).

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

The DEX Event Indexer is a Rust application that indexes events from decentralized exchanges (DEXs) on the Solana blockchain. It captures various events such as swaps, liquidity additions, and liquidity removals, storing them in a PostgreSQL database (via Supabase) for later analysis.

The indexer monitors Solana transactions in real-time by subscribing to WebSocket logs and also provides backfilling capabilities for historical events.

Currently supported DEXs:

- Orca Whirlpool (Concentrated Liquidity Pools)

Future implementations planned:

- Raydium Concentrated Liquidity Pools

## Getting Started

- [Setup Instructions](./setup.md)
- [Running the Indexer](./running.md)
- [CLI Usage](./cli-usage.md)

## Architecture

The indexer follows a modular, protocol-oriented architecture:

- **Models**: Define the structures for blockchain events and database tables
- **DB**: Handle database connections and operations
- **Indexers**: Process blockchain events and store them in the database

All database tables are created under the 'apestrong' schema for organization. Each protocol (DEX) has its own set of models, database tables, and indexing logic, allowing for easy extension.

Read more in the [Architecture Documentation](./architecture.md).

## Database Schema

The indexer stores event data in a structured PostgreSQL database. The schema is designed to efficiently store and query various types of DEX events.

- Base tables store common event information
- Specialized tables store event-specific details
- Views combine related data for easier querying

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

See the [CLI Usage Guide](./cli-usage.md) for more details.

## Adding New DEXs

We've designed the system to be easily extendable with new DEXs:

- [How to Add a New DEX](./add-new-dex.md)

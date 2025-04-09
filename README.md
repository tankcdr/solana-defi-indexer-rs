# DEX Event Indexer

A modular, extensible system for indexing events from decentralized exchanges (DEXs) on the Solana blockchain.

## Features

- Captures swap and liquidity events from Orca Whirlpool pools
- Stores events in a PostgreSQL database (using "apestrong" schema)
- Easily extensible to support additional DEXs
- Real-time monitoring via WebSocket connections
- Historical event backfilling with automatic recovery
- Command-line interface for flexible configuration
- Signature tracking to avoid duplicate event processing

## Quick Start

### Using Docker (Recommended)

The easiest way to get started is using Docker Compose:

```bash
# Start the indexer with Docker
docker compose up -d

# View logs
docker compose logs -f
```

By default, this will start the Orca indexer. You can customize which DEX to index using the `DEX_TYPE` environment variable:

```bash
DEX_TYPE=raydium docker compose up -d
```

For more details, see the [Docker Setup Guide](./docs/docker-setup.md).

### Manual Setup

1. Clone the repository
2. Configure your environment variables in `.env`
3. Set up the database using the database utilities: `./database/dbutil.sh create all`
4. Load default pools: `./database/load_pools.sh all`
5. Start the indexer: `cargo run -- orca --pools <POOL_ADDRESSES>` or just `cargo run` to use default pools

For detailed instructions, see the [Setup Guide](./docs/setup.md) and [Database Utilities](./database/README.md).

## Documentation

Comprehensive documentation is available in the [docs](./docs) directory:

- [Main Documentation](./docs/README.md)
- [Setup Instructions](./docs/setup.md)
- [Running the Indexer](./docs/running.md)
- [Architecture Overview](./docs/architecture.md)
- [Adding New DEXs](./docs/add-new-dex.md)
- [Database Schema](./docs/database-schema.md)

## Supported DEXs

Currently supported DEXs:

- [Orca Whirlpool](https://www.orca.so/) - Concentrated liquidity pools
- [Raydium](https://raydium.io/) - Concentrated liquidity pools (in progress)

## Architecture

The indexer follows a modular, protocol-oriented architecture:

```
indexer/
├── src/
│   ├── models/               # Data models organized by protocol
│   │   ├── common.rs         # Shared types and enums
│   │   └── orca/             # Protocol-specific models
│   ├── db/                   # Database operations and repositories
│   │   └── repositories/     # Protocol-specific database operations
│   ├── indexers/             # Event processing logic
│   ├── websocket_manager.rs  # WebSocket connection management
│   └── backfill_manager.rs   # Historical data backfilling
├── database/                 # Database setup scripts and configuration
│   ├── schema/               # SQL schema definitions by protocol
│   │   ├── common/           # Shared tables across DEXes
│   │   ├── orca/             # Orca-specific tables
│   │   └── raydium/          # Raydium-specific tables
│   └── models/               # Rust models for database operations
└── docs/                     # Documentation
```

This architecture makes it easy to add support for additional DEXs without modifying existing code. The separation of concerns allows for straightforward extension and maintenance.

## Requirements

- Rust (latest stable)
- PostgreSQL database (self-hosted or Docker)
- Solana RPC endpoint with WebSocket support
- Environment configuration via `.env` file or environment variables

### Docker Requirements (Alternative)

- Docker and Docker Compose
- No need for local PostgreSQL or Rust installation

## Contributing

Contributions are welcome! To add support for a new DEX, please follow the [guide](./docs/add-new-dex.md) and submit a pull request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

# DEX Event Indexer

A modular, extensible system for indexing events from decentralized exchanges (DEXs) on the Solana blockchain.

## Features

- Captures swap and liquidity events from Orca Whirlpool pools
- Stores events in a PostgreSQL database (via Supabase)
- Easily extensible to support additional DEXs
- Real-time monitoring via WebSocket connections
- Historical event backfilling

## Quick Start

1. Clone the repository
2. Configure your environment variables in `.env`
3. Set up the database: `cargo run --bin setup_db`
4. Start the indexer: `cargo run`

For detailed instructions, see the [Setup Guide](./docs/setup.md).

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

- [Orca Whirlpool](https://www.orca.so/)

## Architecture

The indexer follows a modular, protocol-oriented architecture:

```
indexer/
├── src/
│   ├── models/               # Data models organized by protocol
│   ├── db/                   # Database operations
│   └── indexers/             # Event processing logic
└── database/                 # Database setup scripts
```

This architecture makes it easy to add support for additional DEXs without modifying existing code.

## Requirements

- Rust (latest stable)
- PostgreSQL database (via Supabase or self-hosted)
- Solana RPC endpoint with WebSocket support

## Contributing

Contributions are welcome! To add support for a new DEX, please follow the [guide](./docs/add-new-dex.md) and submit a pull request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

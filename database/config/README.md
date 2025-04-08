# Orca Whirlpool Pool Configuration

This directory contains configuration files for Orca Whirlpool pools that the indexer will monitor.

## Pool List

The `orca_pools.txt` file contains a list of Orca Whirlpool pools to monitor. Each line should contain a single pool address in base58 format. Lines beginning with `#` are treated as comments.

Example:

```
# SOL-USDC Pool
Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE
```

## How Pool Loading Works

There are several ways to load and manage pools:

1. **Database Initialization**: When the PostgreSQL container starts, it automatically loads the pools from `orca_pools.txt` into the database.

2. **Manual Pool Loading**: You can manually load a pool using either:

   - The `load_orca_pool` utility
   - The `load_orca_pools.sh` script
   - The indexer's `LoadPool` command

3. **Indexer Operation**: The indexer can be configured to use pools from the database by passing the `--use-db-pools` flag.

## Pool Loader Utility

The `load_orca_pool` utility connects to Solana to fetch complete pool details:

```bash
# Load a single pool
cargo run --bin load_orca_pool Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE

# With custom RPC URL
cargo run --bin load_orca_pool --solana-rpc-url https://api.mainnet-beta.solana.com Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE
```

## Batch Loading Script

The `load_orca_pools.sh` script allows loading multiple pools at once:

```bash
# Load all pools from config file
./database/load_orca_pools.sh

# Load specific pools
./database/load_orca_pools.sh Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE HoGQJncRH2qP1yW8EcXWKP5UEK2xaJ8HEVwEVuZHKhDg
```

The script automatically reads the `.env` file from the project root directory and uses those environment variables (like DATABASE_URL and SOLANA_RPC_URL) when loading pools, ensuring consistent configuration.

## Using the Indexer's LoadPool Command

```bash
# Load a pool using the indexer
cargo run -- LoadPool Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE
```

## Running the Indexer with Database Pools

To use pools from the database when running the indexer:

```bash
cargo run -- Orca --use-db-pools
```

This will fetch all pools from the `apestrong.orca_whirlpool_pools` table and subscribe to their events.

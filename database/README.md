# Database Utilities and Schema

This directory contains tools for setting up and managing the indexer's database across multiple DEXes.

## Directory Structure

- `schema/`: Contains SQL schema definitions organized by DEX
  - `common/`: Common tables shared across all DEXes
  - `orca/`: Orca-specific tables and views
  - `raydium/`: Raydium-specific tables and views
- `models/`: Contains Rust models for DEX-specific data processing
  - `mod.rs`: Common interfaces and types
  - `orca.rs`: Orca-specific implementations
- Utility scripts and tools for managing the database

## Schema Overview

The database is organized into a modular schema structure:

### Common Schema (`schema/common/schema.sql`)

- `apestrong.token_metadata`: Stores token information (name, symbol, decimals)
- `apestrong.subscribed_pools`: Tracks pools from all DEXes that the indexer monitors
- `apestrong.last_signatures`: Tracks the last seen event signature for each pool

### Orca Schema (`schema/orca/schema.sql`)

- `apestrong.orca_whirlpool_events`: Base table for all Orca Whirlpool events
- `apestrong.orca_traded_events`: Stores details for trading events
- `apestrong.orca_liquidity_increased_events`: Stores details for liquidity increase events
- `apestrong.orca_liquidity_decreased_events`: Stores details for liquidity decrease events
- Views for easier querying:
  - `apestrong.v_orca_whirlpool_traded`
  - `apestrong.v_orca_whirlpool_liquidity_increased`
  - `apestrong.v_orca_whirlpool_liquidity_decreased`

### Raydium Schema (`schema/raydium/schema.sql`)

- Contains Raydium-specific tables and views (implementation in progress)

## Database Utilities

### Database Schema Management (dbutil)

The database schema utility (`dbutil`) provides commands to create and delete schemas for different DEXes.

```bash
# Create schemas
./database/dbutil.sh create all         # Create all schemas
./database/dbutil.sh create orca        # Create Orca schema (includes common)
./database/dbutil.sh create raydium     # Create Raydium schema (includes common)

# Delete schemas (will prompt for confirmation)
./database/dbutil.sh delete orca        # Delete Orca schema
./database/dbutil.sh delete all --yes   # Delete all schemas without confirmation

# Options
./database/dbutil.sh create orca --verbose         # Show SQL statements
./database/dbutil.sh delete raydium --database-url "postgres://..." # Custom connection
```

For Docker environments, use `dbutil_docker.sh` with the same arguments.

### Pool Loading (load_pools)

The pool loading utility (`load_pools`) populates the database with pool data from different DEXes. It reads pool addresses from `subscribed_pools.txt` files located in each DEX's schema directory:

- `schema/orca/subscribed_pools.txt`: Contains Orca pool addresses to load
- `schema/raydium/subscribed_pools.txt`: Contains Raydium pool addresses to load

Each `subscribed_pools.txt` file should contain one pool address per line, with comments starting with `#`:

```
# Example subscribed_pools.txt
# SOL-USDC Pool
Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE

# Another pool
C9U2Ksk6KKWvLEeo5yUQ7Xu46X7NzeBJtd9PBfuXaUSM
```

To load pools:

```bash
# Load pools
./database/load_pools.sh all        # Load pools from all DEXes
./database/load_pools.sh orca       # Load only Orca pools
./database/load_pools.sh raydium    # Load only Raydium pools

# Options
./database/load_pools.sh orca --verbose  # Show detailed processing
```

For Docker environments, use `load_pools_docker.sh` with the same arguments. The Docker version also includes additional checks to avoid reloading pools that are already in the database.

## Prerequisites

- Rust toolchain installed
- PostgreSQL database
- Environment configuration (`.env` file or environment variables)

## Configuration

Required environment variables:

- `DATABASE_URL`: PostgreSQL connection string
- `SOLANA_RPC_URL`: Solana RPC endpoint for fetching blockchain data

Optional configuration:

- `DATABASE_MAX_CONNECTIONS`: Maximum number of database connections (default: 5)

## Example `.env` File

```
# Database connection
DATABASE_URL=postgres://username:password@localhost:5432/indexer

# Solana RPC settings
SOLANA_RPC_URL=https://api.mainnet-beta.solana.com
SOLANA_WS_URL=wss://api.mainnet-beta.solana.com
```

## Technical Details

### Modular Design

The database utilities follow a modular, extensible design:

1. **Schema Structure**:

   - Common tables (like `token_metadata`) are in the common schema
   - DEX-specific tables are in their respective schemas
   - Each DEX's schema can be created/deleted independently

2. **Code Architecture**:

   - `dbutil` handles schema operations with proper dependency ordering
   - `load_pools` uses traits and dynamic dispatch for DEX-specific processing
   - Token caching reduces redundant database operations

3. **Docker Integration**:
   - All utilities have Docker-optimized versions
   - Automatic pool counting and comparison to avoid redundant work

### Adding a New DEX

To add support for a new DEX:

1. Create schema files in `schema/<dex_name>/`
2. Create a processor implementation in `models/<dex_name>.rs`
3. Update the DEX type enums in `dbutil.rs` and `load_pools.rs`

## Troubleshooting

### Common Issues

1. **Tables Not Created**:

   - Run with `--verbose` to see SQL execution details
   - Check PostgreSQL logs for errors
   - Ensure the DEX name is correct (e.g., "orca", not "Orca")

2. **Connection Error**:

   - Verify the `DATABASE_URL` is correct
   - Check if PostgreSQL is running
   - Ensure network access is allowed

3. **Permission Error**:
   - Ensure your database user has CREATE permission
   - For schema operations, verify permissions on the schema

### Getting Help

Run any utility with the `--help` flag for usage information:

```bash
./database/dbutil.sh --help
./database/load_pools.sh --help
```

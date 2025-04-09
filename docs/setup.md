# Setup Instructions

This guide will walk you through setting up the DEX Event Indexer.

## Prerequisites

### Docker Setup (Recommended)

- Docker and Docker Compose
- Git (for cloning the repository)
- No need for local PostgreSQL or Rust installation

### Manual Setup

- Rust (latest stable version)
- PostgreSQL database (self-hosted)
- Solana RPC endpoint with WebSocket support
- Git (for cloning the repository)

## Installation

1. Clone the repository:

   ```bash
   git clone https://github.com/yourusername/prediction-market-indexer.git
   cd prediction-market-indexer
   ```

2. Choose your setup method:

   **Option A: Docker Setup (Recommended)**

   ```bash
   # Create a .env file (optional, for custom settings)
   cp .env.example .env

   # Start services
   docker compose up -d
   ```

   This will start the complete stack including PostgreSQL database and the indexer. For more details, see the [Docker Setup Guide](./docker-setup.md).

   **Option B: Manual Setup**

   Install Rust dependencies:

   ```bash
   cargo build
   ```

## Database Setup

The indexer uses PostgreSQL to store the captured DEX events under the 'apestrong' schema. The database schema is modular, with common tables shared across all DEXes and DEX-specific tables for each supported protocol.

### Database Structure

The database is organized into:

- **Common schema**: Tables for token metadata, subscribed pools, and signature tracking
- **DEX-specific schemas**: Tables for event data from each supported DEX (Orca, Raydium, etc.)

### Environment Configuration

1. Copy the example environment file:

   ```bash
   cp .env.example .env
   ```

2. Edit `.env` with your database details:

   ```
   # Supabase connection settings
   DATABASE_URL=postgres://postgres:[YOUR-PASSWORD]@db.[YOUR-PROJECT-REF].supabase.co:5432/postgres
   DATABASE_MAX_CONNECTIONS=5
   DATABASE_CONNECT_TIMEOUT=30

   # Solana RPC settings
   SOLANA_RPC_URL=https://api.mainnet-beta.solana.com
   SOLANA_WS_URL=wss://api.mainnet-beta.solana.com
   ```

   For Supabase, you can find your database connection details in the Supabase dashboard under Project Settings > Database.

### Create Database Schema

The project includes utilities to create and manage the database schema:

```bash
# Create all database schemas
./database/dbutil.sh create all

# Or create schema for a specific DEX
./database/dbutil.sh create orca
```

This will create the following schemas:

**Common Schema:**

- `apestrong.token_metadata` - Information about tokens
- `apestrong.subscribed_pools` - Pools being monitored by the indexer
- `apestrong.last_signatures` - Last processed signature for each pool

**Orca Schema:**

- `apestrong.orca_whirlpool_events` - Base table for all Orca events
- `apestrong.orca_traded_events` - For swap events
- `apestrong.orca_liquidity_increased_events` - For liquidity additions
- `apestrong.orca_liquidity_decreased_events` - For liquidity removals
- Views for easier querying

**Raydium Schema** (if enabled):

- Similar structure to Orca, with Raydium-specific tables

### Load Pool Addresses

After creating the database schema, you need to load the pool addresses that the indexer will monitor:

```bash
# Load pools for all DEXes
./database/load_pools.sh all

# Or load pools for a specific DEX
./database/load_pools.sh orca
```

This loads the pool addresses from the respective `subscribed_pools.txt` files in the `database/schema/` directories.

## Verifying Setup

To verify that your setup is working correctly:

1. If using Docker, check container status:

   ```bash
   docker compose ps
   ```

2. Check that the database tables were created:

   ```sql
   SELECT * FROM apestrong.orca_whirlpool_events LIMIT 5;
   ```

3. Run a test query to ensure you can connect to the Solana RPC:

   ```bash
   curl -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' $SOLANA_RPC_URL
   ```

4. Verify WebSocket connectivity (this will fail if WebSockets are not supported):
   ```bash
   wscat -c $SOLANA_WS_URL
   ```

## RPC Provider Recommendations

For production use, we recommend using a dedicated RPC provider with WebSocket support:

- [QuickNode](https://www.quicknode.com/)
- [Alchemy](https://www.alchemy.com/)
- [Helius](https://helius.xyz/)

Public endpoints have rate limits that may affect the indexer's performance for high-volume monitoring.

## Troubleshooting

### Docker Issues

- If using Docker and encountering issues:
  - Check container status: `docker compose ps`
  - View logs: `docker compose logs`
  - Check network connectivity: `docker network inspect prediction-market-indexer_default`
  - Verify persistent volumes: `docker volume ls`

### Database Connection Issues

- Verify your database credentials and URL
- Ensure PostgreSQL port 5432 is accessible
- Check that your database user has permissions to create schemas and tables
- If using a hosted solution like Supabase, check if your IP is allowed in network restrictions

### Solana RPC Issues

- Public RPC endpoints may have rate limits; consider using a private RPC provider
- Ensure your RPC provider supports WebSocket connections
- WebSocket connections require proper proxy configuration if behind a firewall
- Check that your RPC provider allows subscription to logs

## Next Steps

- If using Docker: Monitor logs with `docker compose logs -f`
- If using manual setup: Proceed to [Running the Indexer](./running.md) for instructions on using the command-line interface to start the indexer
- To understand the database structure: Check the [Database README](../database/README.md)
- To explore more features: Read the [CLI Usage Guide](./cli-usage.md)

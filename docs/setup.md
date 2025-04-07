# Setup Instructions

This guide will walk you through setting up the DEX Event Indexer.

## Prerequisites

- Rust (latest stable version)
- PostgreSQL database (via Supabase or self-hosted)
- Solana RPC endpoint with WebSocket support
- Git (for cloning the repository)

## Installation

1. Clone the repository:

   ```bash
   git clone https://github.com/yourusername/dex-event-indexer.git
   cd dex-event-indexer
   ```

2. Install Rust dependencies:
   ```bash
   cargo build
   ```

## Database Setup

The indexer uses PostgreSQL (via Supabase) to store the captured DEX events under the 'apestrong' schema.

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

Run the database setup script to create all necessary tables and views:

```bash
cargo run --bin setup_db
```

This will execute the SQL in `database/schema.sql` to create the following tables under the 'apestrong' schema:

- `apestrong.orca_whirlpool_events` (base table)
- `apestrong.orca_traded_events` (for swap events)
- `apestrong.orca_liquidity_increased_events` (for liquidity additions)
- `apestrong.orca_liquidity_decreased_events` (for liquidity removals)

And these convenience views:

- `apestrong.v_orca_whirlpool_traded`
- `apestrong.v_orca_whirlpool_liquidity_increased`
- `apestrong.v_orca_whirlpool_liquidity_decreased`

## Verifying Setup

To verify that your setup is working correctly:

1. Check that the database tables were created:

   ```sql
   SELECT * FROM apestrong.orca_whirlpool_events LIMIT 5;
   ```

2. Run a test query to ensure you can connect to the Solana RPC:

   ```bash
   curl -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' $SOLANA_RPC_URL
   ```

3. Verify WebSocket connectivity (this will fail if WebSockets are not supported):
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

### Database Connection Issues

- Verify your Supabase credentials and URL
- Check if your IP is allowed in Supabase's network restrictions
- Ensure PostgreSQL port 5432 is accessible
- Check that your database user has permissions to create schemas and tables

### Solana RPC Issues

- Public RPC endpoints may have rate limits; consider using a private RPC provider
- Ensure your RPC provider supports WebSocket connections
- WebSocket connections require proper proxy configuration if behind a firewall
- Check that your RPC provider allows subscription to logs

## Next Steps

Once setup is complete, proceed to [Running the Indexer](./running.md) for instructions on using the command-line interface to start the indexer.

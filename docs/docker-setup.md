# Docker Setup

This guide explains how to run the DEX Event Indexer using Docker.

## Docker Artifacts

The following Docker artifacts are provided:

1. `Dockerfile.indexer` - Dockerfile for building and running the DEX indexer
2. `Dockerfile.init` - Dockerfile for database initialization and pool loading
3. `Dockerfile.postgres` - Dockerfile for PostgreSQL database with schema initialization
4. `docker-compose.yml` - Configuration for running the complete stack

## Running with Docker Compose

The simplest way to run the indexer is using Docker Compose:

```bash
# Build and start containers
docker compose up -d

# View logs
docker compose logs -f
```

This will start a complete stack with three services:

1. `db` - PostgreSQL database
2. `init-db` - One-time initialization service that sets up the database schema and loads pool addresses
3. `dex-indexer` - The main indexer service that processes events from the blockchain

> **Note:** The Dockerfiles use Rust 1.84+ and copy the project's Cargo.lock to ensure consistent dependencies. The container environment is optimized for production use.

## How It Works

The Docker Compose setup follows this workflow:

1. The `db` service starts a PostgreSQL database
2. The `init-db` service:
   - Waits for the database to be healthy
   - Creates the common database schema
   - Creates the DEX-specific schema (orca, raydium, or both based on `DEX_TYPE`)
   - Loads pool addresses from the configuration files
3. The `dex-indexer` service:
   - Waits for both the database and initialization to complete
   - Starts monitoring blockchain events for the specified DEX type
   - Processes and stores events in the database

By default, the setup uses the Orca DEX, but you can customize this with the `DEX_TYPE` environment variable. 2. Start the DEX indexer connected to the database (defaults to Orca)

## Customizing Environment Variables

You can customize the environment by either:

1. Setting environment variables before running `docker-compose up`:

   ```bash
   export SOLANA_RPC_URL=https://your-rpc-endpoint.com
   export SOLANA_WS_URL=wss://your-ws-endpoint.com
   export DEX_TYPE=orca  # Can be 'orca' (default), 'raydium', or 'all'
   docker compose up -d
   ```

2. Creating a `.env` file in the same directory as the `docker-compose.yml`:
   ```
   SOLANA_RPC_URL=https://your-rpc-endpoint.com
   SOLANA_WS_URL=wss://your-ws-endpoint.com
   DEX_TYPE=orca  # Can be 'orca' (default), 'raydium', or 'all'
   ```

## Specifying Custom DEX Pools

## Customizing Pool Addresses

There are two ways to specify which pools to index:

### 1. Using the Pool Configuration Files

The easiest way is to modify the pool configuration files before building the containers:

- `database/schema/orca/subscribed_pools.txt` - For Orca pools
- `database/schema/raydium/subscribed_pools.txt` - For Raydium pools

Each file should contain one pool address per line:

```
# SOL-USDC Pool
Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE

# Another pool
7qbRF6YsyGuLUVs6Y1q64bdVrfe4ZcUUz1JRdoVNUJnm
```

### 2. Passing Custom Pools to the Indexer

For more dynamic control, you can modify the `dex-indexer` service command in the `docker-compose.yml` file:

```yaml
command: ["--pools", "pool1,pool2,pool3"]
```

Replace `pool1,pool2,pool3` with comma-separated Solana addresses of the pools you want to index.

Note that the DEX type is determined by the `DEX_TYPE` environment variable which defaults to `orca`.

## Accessing the Database

The database is exposed on port 5432. You can connect to it using any PostgreSQL client using:

- Host: `localhost`
- Port: `5432`
- User: `postgres`
- Password: `postgres`
- Database: `postgres`

To check the indexed data:

```sql
SELECT * FROM apestrong.orca_whirlpool_events LIMIT 10;
SELECT * FROM apestrong.v_orca_whirlpool_traded LIMIT 10;
```

## Running Individual Containers

If you prefer to run containers individually:

### Database

```bash
docker build -t dex-indexer-db -f Dockerfile.postgres .
docker run -d --name dex-indexer-db \
  -e DEX_TYPE=all \
  -p 5432:5432 dex-indexer-db
```

````

### DEX Indexer

```bash
docker build -t dex-indexer -f Dockerfile.indexer .
docker run -d --name dex-indexer \
  --link dex-indexer-db:db \
  -e DATABASE_URL=postgres://postgres:postgres@db:5432/postgres \
  -e SOLANA_RPC_URL=https://api.mainnet-beta.solana.com \
  -e SOLANA_WS_URL=wss://api.mainnet-beta.solana.com \
  -e DEX_TYPE=orca \
  dex-indexer
````

To use a different DEX type (when supported):

```bash
docker run -d --name dex-indexer \
  --link dex-indexer-db:db \
  -e DATABASE_URL=postgres://postgres:postgres@db:5432/postgres \
  -e SOLANA_RPC_URL=https://api.mainnet-beta.solana.com \
  -e SOLANA_WS_URL=wss://api.mainnet-beta.solana.com \
  -e DEX_TYPE=raydium \
  dex-indexer
```

````

## Database Persistence

The database data is stored in a named volume `postgres_data` which persists across container restarts. To completely reset the data:

```bash
docker compose down -v
docker compose up -d
````

```

## Troubleshooting

### Connection Issues

If the indexer can't connect to the database:
- Check that the database is running: `docker compose ps`
- Check database logs: `docker compose logs db`
- Check if initialization succeeded: `docker compose logs init-db`
- Ensure the `DATABASE_URL` is correctly configured
- Ensure the `DATABASE_URL` is correctly configured

### RPC Issues

If there are RPC connection errors:

- Verify your RPC provider is working
- Check that the WebSocket URL is correct and the provider supports WebSockets
- Consider using a dedicated RPC provider for production use

## Managing Multiple DEX Types

The Docker setup supports three modes via the `DEX_TYPE` environment variable:

1. Single DEX mode: `DEX_TYPE=orca` or `DEX_TYPE=raydium`
2. All DEXes mode: `DEX_TYPE=all`

When running in `all` mode, the initialization service will set up both DEX schemas, but the indexer still needs to be configured to index events from a specific DEX.

If you need to run indexers for multiple DEXes simultaneously, consider:

1. Creating separate Docker Compose stacks with different service names
2. Using a single database but multiple indexer containers
3. Setting up proper resource allocation for each container

## Adding Custom DEXes

To add a new DEX to the Docker setup:

1. Create schema files in `database/schema/<dex_name>/`
2. Update the initialization script to include the new DEX
3. Create a pool configuration file in the new schema directory
4. Rebuild the Docker images

For more details on implementing a new DEX, see the [Adding New DEXs](./add-new-dex.md) guide.
```

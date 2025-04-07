# Docker Setup

This guide explains how to run the DEX Event Indexer using Docker.

## Docker Artifacts

The following Docker artifacts are provided:

1. `Dockerfile` - Base Dockerfile for the indexer
2. `Dockerfile.orca` - Dockerfile specifically for the Orca indexer
3. `Dockerfile.postgres` - Dockerfile for the PostgreSQL database with schema initialization
4. `docker-compose.yml` - Configuration to run both the database and Orca indexer

## Running with Docker Compose

The simplest way to run the indexer is using Docker Compose:

```bash
# Build and start containers
docker-compose up -d

# View logs
docker-compose logs -f
```

> **Note:** The Dockerfiles use Rust 1.70 and a dependency caching strategy that avoids Cargo.lock compatibility issues. The Docker build creates a new project with manually specified dependencies, which ensures build stability across different environments. The exact dependency versions in the container may differ slightly from your local development environment, but all functionality will remain the same.

This will:

1. Start a PostgreSQL database with the schema initialized
2. Start the Orca indexer connected to the database

## Customizing Environment Variables

You can customize the environment by either:

1. Setting environment variables before running `docker-compose up`:

   ```bash
   export SOLANA_RPC_URL=https://your-rpc-endpoint.com
   export SOLANA_WS_URL=wss://your-ws-endpoint.com
   docker-compose up -d
   ```

2. Creating a `.env` file in the same directory as the `docker-compose.yml`:
   ```
   SOLANA_RPC_URL=https://your-rpc-endpoint.com
   SOLANA_WS_URL=wss://your-ws-endpoint.com
   ```

## Specifying Custom Orca Pools

To specify custom Orca pools to index, modify the `command` section of the `orca-indexer` service in `docker-compose.yml`:

```yaml
command:
  [
    "--rpc-url",
    "${SOLANA_RPC_URL}",
    "--ws-url",
    "${SOLANA_WS_URL}",
    "orca",
    "--pools",
    "pool1,pool2,pool3",
  ]
```

Replace `pool1,pool2,pool3` with comma-separated Solana addresses of the Orca pools you want to index.

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
docker run -d --name dex-indexer-db -p 5432:5432 dex-indexer-db
```

### Orca Indexer

```bash
docker build -t orca-indexer -f Dockerfile.orca .
docker run -d --name orca-indexer \
  --link dex-indexer-db:db \
  -e DATABASE_URL=postgres://postgres:postgres@db:5432/postgres \
  -e SOLANA_RPC_URL=https://api.mainnet-beta.solana.com \
  -e SOLANA_WS_URL=wss://api.mainnet-beta.solana.com \
  orca-indexer
```

## Database Persistence

The database data is stored in a named volume `postgres_data` which persists across container restarts. To completely reset the data:

```bash
docker-compose down -v
docker-compose up -d
```

## Troubleshooting

### Connection Issues

If the indexer can't connect to the database:

- Check that the database is running: `docker-compose ps`
- Check database logs: `docker-compose logs db`
- Ensure the `DATABASE_URL` is correctly configured

### RPC Issues

If there are RPC connection errors:

- Verify your RPC provider is working
- Check that the WebSocket URL is correct and the provider supports WebSockets
- Consider using a dedicated RPC provider for production use

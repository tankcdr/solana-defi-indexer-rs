# Running the DEX Event Indexer

This guide explains how to run and manage the DEX Event Indexer using its command-line interface.

## Running Options

You can run the indexer using either Docker or direct command-line execution.

### Option 1: Docker (Recommended)

The simplest way to run the indexer is with Docker Compose:

```bash
# Start the entire stack with default settings (Orca indexer)
docker compose up -d

# View logs
docker compose logs -f

# Run with a specific DEX type
DEX_TYPE=raydium docker compose up -d
```

### Option 2: Command Line

After completing the [setup process](./setup.md), you can run the indexer directly with:

```bash
cargo run --bin indexer [global options] <command> [command options]
```

Where:

- `[global options]` affects all indexers (RPC settings)
- `<command>` selects which DEX to index (e.g., `orca`, `raydium`)
- `[command options]` configures the specific indexer

## Examples

### Running Specific DEX Indexers

#### Orca Whirlpool Indexer

Run the Orca indexer with default settings (monitors the SOL/USDC pool):

```bash
# Docker
DEX_TYPE=orca docker compose up -d

# Command line
cargo run --bin indexer orca
```

#### Raydium Indexer

Run the Raydium indexer with default settings:

```bash
# Docker
DEX_TYPE=raydium docker compose up -d

# Command line
cargo run --bin indexer raydium
```

### Specifying Custom Pools

To monitor a specific Orca Whirlpool pool:

```bash
# Command line
cargo run --bin indexer orca --pools Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE
```

### Multiple Pools

To monitor multiple pools at once:

```bash
# Command line
cargo run --bin indexer orca --pools Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE,7qbRF6YsyGuLUVs6Y1q64bdVrfe4ZcUUz1JRdoVNUJnm
```

### Using Custom RPC Endpoints

To use specific RPC and WebSocket endpoints:

```bash
# Command line
cargo run --bin indexer --rpc-url https://your-rpc-provider.com --ws-url wss://your-rpc-provider.com orca
```

When using Docker, you can set these in the environment:

```bash
SOLANA_RPC_URL=https://your-rpc-provider.com SOLANA_WS_URL=wss://your-rpc-provider.com docker compose up -d
```

## Configuration Sources

The indexer combines configuration from multiple sources in this priority order:

1. Command-line arguments (highest priority)
2. Environment variables set in the shell
3. .env file variables (lowest priority)

This allows for flexible deployment scenarios.

## Production Deployment

For production environments, we recommend:

### Docker Deployment (Recommended)

The easiest way to deploy in production is using Docker Compose:

```bash
# Clone the repository
git clone https://github.com/yourusername/prediction-market-indexer.git
cd prediction-market-indexer

# Create a production .env file
cp .env.example .env
# Edit .env with production settings

# Start in detached mode
docker compose up -d
```

Consider setting up a reverse proxy (like Nginx) if you need to expose metrics or an API.

### Manual Deployment

#### 1. Building a Release Binary

```bash
cargo build --release
```

The compiled binary will be located at `target/release/indexer`.

#### 2. Using a Process Manager

Use a process manager like systemd to ensure the indexer stays running:

Create a systemd service file at `/etc/systemd/system/dex-indexer.service`:

```ini
[Unit]
Description=DEX Event Indexer
After=network.target postgresql.service

[Service]
Type=simple
User=indexer
WorkingDirectory=/path/to/indexer
ExecStart=/path/to/indexer/target/release/indexer orca --pools Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE
Restart=always
RestartSec=10
# Maximum number of restarts in a time window
StartLimitBurst=5
StartLimitIntervalSec=60
# Environment variables
Environment="DATABASE_URL=postgres://username:password@localhost:5432/postgres"
Environment="DATABASE_MAX_CONNECTIONS=5"
Environment="DATABASE_CONNECT_TIMEOUT=30"
Environment="SOLANA_RPC_URL=https://your-rpc-provider.com"
Environment="SOLANA_WS_URL=wss://your-rpc-provider.com"

[Install]
WantedBy=multi-user.target
```

For Raydium, create a similar file but with the `raydium` command instead of `orca`.

Enable and start the service:

```bash
sudo systemctl enable dex-indexer
sudo systemctl start dex-indexer
```

### 3. Monitoring Logs

View indexer logs with:

```bash
sudo journalctl -u dex-indexer -f
```

## Running Multiple Instances

### Using Docker

To run multiple indexers using Docker, you can create separate Docker Compose files or use service scaling:

#### Method 1: Multiple Compose Files

Create separate compose files for each DEX type:

```bash
# orca.docker-compose.yml for Orca
# raydium.docker-compose.yml for Raydium

# Run them separately
docker compose -f orca.docker-compose.yml up -d
docker compose -f raydium.docker-compose.yml up -d
```

#### Method 2: Multiple Container Instances

Modify the `docker-compose.yml` to include multiple indexer services:

```yaml
services:
  db:
    # Database config...

  init-db:
    # Init config...

  orca-indexer:
    build:
      context: .
      dockerfile: Dockerfile.indexer
    environment:
      - DEX_TYPE=orca
    # other settings...

  raydium-indexer:
    build:
      context: .
      dockerfile: Dockerfile.indexer
    environment:
      - DEX_TYPE=raydium
    # other settings...
```

### Using Manual Setup

To monitor different DEXs or pool sets without Docker:

1. Create separate systemd service files for each instance
2. Use different DEX types or pools for each instance
3. Optionally use different database connection settings if needed

For example:

```
dex-indexer-orca.service → Runs the Orca indexer for high-volume pools
dex-indexer-raydium.service → Runs the Raydium indexer
```

## Resource Considerations

### RPC Node Requirements

The indexer maintains a WebSocket connection to a Solana RPC node. Consider:

- **Rate Limits**: Public RPC nodes often have rate limits that might affect indexing performance
- **WebSocket Support**: Ensure your RPC provider properly supports WebSocket connections
- **Historical Data**: Backfilling requires access to historical transaction data, which some RPC providers limit
- **Connection Stability**: For production, choose a provider with good uptime and low latency

We recommend:

- For testing: Public endpoints like Solana's official RPC
- For production: Dedicated RPC nodes from providers like QuickNode, Helius, or Alchemy

### Database Scaling

For high-volume indexing:

- Set `DATABASE_MAX_CONNECTIONS` appropriately (default: 5)
- Use connection pooling for better performance
- Index frequently queried columns
- Monitor database performance using PostgreSQL monitoring tools
- Consider database read replicas for analytics workloads

### Docker Resource Allocation

When using Docker in production:

- Allocate sufficient memory (at least 1GB for the indexer)
- Set appropriate CPU limits
- Use persistent volumes for the database
- Monitor container health and logs

## Troubleshooting

### WebSocket Connection Issues

If you encounter WebSocket connection errors:

1. Verify your RPC endpoint supports WebSockets (use `wscat -c YOUR_WS_URL` to test)
2. Check network connectivity and firewall settings
3. Try using a different RPC provider

### Database Connection Issues

If you encounter database connection errors:

1. Verify your database credentials and connection string in the `.env` file
2. Check that the database server is accessible from your network
3. Ensure your IP address is allowed in Supabase's network restrictions

### Event Processing Issues

If events are not being processed correctly:

1. Check the logs for error messages, particularly deserialization errors
2. Verify that the pools you're monitoring are valid pools for the selected DEX
3. Ensure your RPC provider has adequate transaction history for backfilling
4. Check the format of the pool addresses in the configuration files
5. For Docker, make sure the initialization service completed successfully

### Handling Crashes and Restarts

The system is designed to recover from crashes:

1. It tracks the last processed signature for each pool
2. On restart, it automatically continues from where it left off
3. The Docker setup includes restart policies to handle container failures

## Next Steps

After your indexer is running, you might want to:

- [Add support for additional DEXs](./add-new-dex.md)
- Build analytics dashboards on top of the indexed data
- Set up monitoring and alerting for the indexer process
- Integrate with other systems via the collected data
- Scale the system to handle more DEXes or higher transaction volumes

### Monitoring Tools

For production deployments, consider setting up:

1. Prometheus for metrics collection
2. Grafana for visualizing metrics and setting up alerts
3. Log aggregation using tools like ELK stack or Loki
4. Database monitoring using tools like pgHero or pgAdmin

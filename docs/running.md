# Running the DEX Event Indexer

This guide explains how to run and manage the DEX Event Indexer using its command-line interface.

## Basic Usage

After completing the [setup process](./setup.md), you can run the indexer with the following command structure:

```bash
cargo run -- [global options] <command> [command options]
```

Where:

- `[global options]` affects all indexers (RPC settings)
- `<command>` selects which DEX to index (currently only `orca`)
- `[command options]` configures the specific indexer

## Examples

### Running with Default Settings

This will monitor the default SOL/USDC Orca Whirlpool pool:

```bash
cargo run
```

### Specifying a Custom Pool

To monitor a specific Orca Whirlpool pool:

```bash
cargo run -- orca --pools Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE
```

### Multiple Pools

To monitor multiple pools at once:

```bash
cargo run -- orca --pools Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE,7qbRF6YsyGuLUVs6Y1q64bdVrfe4ZcUUz1JRdoVNUJnm
```

### Using Custom RPC Endpoints

To use specific RPC and WebSocket endpoints:

```bash
cargo run -- --rpc-url https://your-rpc-provider.com --ws-url wss://your-rpc-provider.com orca
```

## Configuration Sources

The indexer combines configuration from multiple sources in this priority order:

1. Command-line arguments (highest priority)
2. Environment variables set in the shell
3. .env file variables (lowest priority)

This allows for flexible deployment scenarios.

## Production Deployment

For production environments, we recommend:

### 1. Building a Release Binary

```bash
cargo build --release
```

The compiled binary will be located at `target/release/indexer`.

### 2. Using a Process Manager

Use a process manager like systemd to ensure the indexer stays running:

Create a systemd service file at `/etc/systemd/system/dex-indexer.service`:

```ini
[Unit]
Description=DEX Event Indexer
After=network.target

[Service]
Type=simple
User=indexer
WorkingDirectory=/path/to/indexer
ExecStart=/path/to/indexer/target/release/indexer orca --pools Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE
Restart=always
Environment="DATABASE_URL=postgres://username:password@localhost:5432/postgres"
Environment="SOLANA_RPC_URL=https://your-rpc-provider.com"
Environment="SOLANA_WS_URL=wss://your-rpc-provider.com"

[Install]
WantedBy=multi-user.target
```

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

To monitor different DEXs or pool sets, you can run multiple instances of the indexer with different configurations:

1. Create separate service files for each instance
2. Use different pools or different DEXs for each instance
3. Optionally use different database connection settings if needed

For example, one service might index high-volume pools while another indexes less active pools.

## Resource Considerations

### RPC Node Requirements

The indexer maintains a WebSocket connection to a Solana RPC node. Consider:

- **Rate Limits**: Public RPC nodes often have rate limits that might affect indexing performance
- **WebSocket Support**: Ensure your RPC provider properly supports WebSocket connections
- **Historical Data**: Backfilling requires access to historical transaction data, which some RPC providers limit

### Database Scaling

For high-volume indexing:

- Set `DATABASE_MAX_CONNECTIONS` appropriately (default: 5)
- Monitor database performance using Supabase or PostgreSQL monitoring tools
- Consider database read replicas for analytics workloads

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
2. Verify that the pools you're monitoring are valid Orca Whirlpool pools
3. Ensure your RPC provider has adequate transaction history for backfilling

## Next Steps

After your indexer is running, you might want to:

- [Add support for additional DEXs](./add-new-dex.md)
- Build analytics dashboards on top of the indexed data
- Set up monitoring and alerting for the indexer process

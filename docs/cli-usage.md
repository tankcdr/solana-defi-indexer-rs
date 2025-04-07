# Indexer CLI Usage Guide

The DEX Event Indexer includes a command-line interface (CLI) that allows you to specify which DEX to monitor, which pools to track, and configure RPC endpoints. This document explains how to use the CLI.

## Basic Usage

The basic command structure is:

```
indexer [global options] <command> [command options]
```

Where:

- `[global options]` are options that apply to all indexers
- `<command>` is the specific indexer to run (e.g., `orca`)
- `[command options]` are options specific to the chosen indexer

## Global Options

The following global options are available:

- `--rpc-url <URL>`: Specify the Solana RPC URL (default: https://api.mainnet-beta.solana.com)
- `--ws-url <URL>`: Specify the Solana WebSocket URL (default: wss://api.mainnet-beta.solana.com)

## Available Commands

### Orca Whirlpool Indexer

Run the Orca Whirlpool indexer:

```
indexer orca [options]
```

Options:

- `--pools <ADDRESSES>`: Comma-separated list of pool addresses to index (default: Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE, which is the SOL/USDC pool)

## Examples

### Run the Orca indexer with default settings

```bash
cargo run -- orca
```

### Run the Orca indexer with a custom pool

```bash
cargo run -- orca --pools Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE
```

### Run the Orca indexer with custom RPC and WebSocket URLs

```bash
cargo run -- --rpc-url https://solana-api.projectserum.com --ws-url wss://solana-api.projectserum.com orca
```

### Run the Orca indexer with multiple pools

```bash
cargo run -- orca --pools Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE,7qbRF6YsyGuLUVs6Y1q64bdVrfe4ZcUUz1JRdoVNUJnm
```

## Environment Variables vs Command Line

The indexer uses configuration from multiple sources in this priority order:

1. Command-line arguments (highest priority)
2. Environment variables in the shell
3. .env file variables (lowest priority)

For example, if you set `SOLANA_RPC_URL` in your .env file but also use `--rpc-url` on the command line, the command-line value will be used.

## Protected Files System

The indexer codebase includes a protection mechanism for critical files:

1. **Protected File Headers**: Core implementation files include warning headers.
2. **Configuration File**: A `.nooverwrite.json` file at the project root lists protected files.

### Protected File Header Example

```rust
/******************************************************************************
 * IMPORTANT: DO NOT MODIFY THIS FILE WITHOUT EXPLICIT APPROVAL
 *
 * This file is protected and should not be modified without explicit approval.
 * Any changes could break the indexer functionality.
 *
 * See .nooverwrite.json for more information on protected files.
 ******************************************************************************/
```

### .nooverwrite.json Format

```json
{
  "protectedFiles": [
    {
      "path": "src/indexers/orca.rs",
      "reason": "Contains critical Orca indexer implementation.",
      "lastUpdated": "2025-04-01",
      "owner": "core-team"
    }
  ],
  "instructions": "Files listed here are protected and should not be modified without explicit approval."
}
```

## Adding New Indexers

To add support for a new DEX indexer:

1. Follow the instructions in [Adding a New DEX](./add-new-dex.md)
2. Update `main.rs` to add a new subcommand for the indexer
3. Add command-line argument processing for the new indexer's options

After implementing a new indexer, users can run it with:

```bash
cargo run -- new-dex-name [options]
```

The command-line structure is designed to easily accommodate adding new indexers in the future.

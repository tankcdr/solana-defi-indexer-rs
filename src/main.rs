/******************************************************************************
 * INDEXER MAIN ENTRY POINT
 *
 * This file coordinates the launching of different DEX indexers based on
 * command line arguments. It currently supports Orca with Raydium planned.
 ******************************************************************************/

use anyhow::{ Context, Result };
use clap::{ Parser, Subcommand };
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use indexer::{
    db::{ Database, DbConfig },
    db::repositories::{ OrcaWhirlpoolRepository, OrcaWhirlpoolPoolRepository },
    indexers::OrcaWhirlpoolIndexer,
};

// Default values
const DEFAULT_RPC_URL: &str = "https://api.mainnet-beta.solana.com";
const DEFAULT_WS_URL: &str = "wss://api.mainnet-beta.solana.com";
const DEFAULT_ORCA_POOL: &str = "Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE"; // SOL/USDC pool

/// Solana DEX indexer CLI
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Solana RPC URL
    #[arg(long, default_value = DEFAULT_RPC_URL)]
    rpc_url: String,

    /// Solana WebSocket URL
    #[arg(long, default_value = DEFAULT_WS_URL)]
    ws_url: String,

    /// Indexer command to run
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run the Orca Whirlpool indexer
    Orca {
        /// Comma-separated list of pool addresses to index
        #[arg(long, use_value_delimiter = true, value_delimiter = ',')]
        pools: Option<Vec<String>>,
    },
    // Future support for additional DEXes
    /*
    /// Run the Raydium indexer (future implementation)
    Raydium {
        /// Comma-separated list of pool addresses to index
        #[arg(long, use_value_delimiter = true, value_delimiter = ',')]
        pools: Option<Vec<String>>,
    },
    */
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file if present
    dotenv::dotenv().ok();

    // Parse command line arguments
    let cli = Cli::parse();

    // Get database configuration
    let db_config = DbConfig::from_env().context("Failed to get database configuration")?;

    // Connect to the database
    let db = Database::connect(db_config).await.context("Failed to connect to database")?;

    match &cli.command {
        Command::Orca { pools } => {
            println!("Starting Orca Whirlpool indexer...");

            // Get pool addresses in priority order: CLI > Database > Default
            let pool_pubkeys = get_pool_addresses(pools, &db).await?;

            // Create repository and indexer
            let repository = OrcaWhirlpoolRepository::new(db.pool().clone());
            let indexer = OrcaWhirlpoolIndexer::new(repository);

            // Start the indexer with the HashSet of pool pubkeys
            indexer
                .start(&cli.rpc_url, &cli.ws_url, &pool_pubkeys).await
                .context("Orca indexer failed")?;
        }
        // For future implementation
        /*
        Command::Raydium { pools } => {
            println!("Raydium indexer not yet implemented");
            // TODO: Implement Raydium indexer
        }
        */
    }

    Ok(())
}

/// Get pool addresses in priority order: CLI > Database > Default
///
/// This function fetches pool addresses based on the following priority:
/// 1. Command line arguments (if provided)
/// 2. Database entries (if available)
/// 3. Default hardcoded pool address
async fn get_pool_addresses(
    cli_pools: &Option<Vec<String>>,
    db: &Database
) -> Result<std::collections::HashSet<Pubkey>> {
    // 1. If CLI arguments are provided, use them
    if let Some(addresses) = cli_pools {
        if !addresses.is_empty() {
            println!("Using pool addresses from command line arguments");
            let mut pubkeys = std::collections::HashSet::new();
            for addr in addresses {
                let pubkey = Pubkey::from_str(addr).context(
                    format!("Invalid Solana address: {}", addr)
                )?;
                pubkeys.insert(pubkey);
            }
            return Ok(pubkeys);
        }
    }

    // 2. If no CLI arguments, try to get pools from the database
    let pool_repo = OrcaWhirlpoolPoolRepository::new(db.pool().clone());
    let db_pools = pool_repo.get_pool_pubkeys().await?;

    if !db_pools.is_empty() {
        println!("Using pool addresses from database");
        return Ok(db_pools);
    }

    // 3. If no pools in database, use default
    println!("No pools specified via CLI or found in database. Using default pool");
    let mut pubkeys = std::collections::HashSet::new();
    pubkeys.insert(
        Pubkey::from_str(DEFAULT_ORCA_POOL).context("Failed to parse default Orca pool address")?
    );

    Ok(pubkeys)
}

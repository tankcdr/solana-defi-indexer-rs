/******************************************************************************
 * INDEXER MAIN ENTRY POINT
 *
 * This file coordinates the launching of different DEX indexers based on
 * command line arguments. It currently supports Orca with Raydium planned.
 ******************************************************************************/

use anyhow::{ Context, Result };
use clap::{ Parser, Subcommand };

use indexer::{
    db::{ Database, DbConfig },
    indexers::{ OrcaWhirlpoolIndexer, start_indexer },
    utils::logging,
};

// Default values
const DEFAULT_RPC_URL: &str = "https://api.mainnet-beta.solana.com";
const DEFAULT_WS_URL: &str = "wss://api.mainnet-beta.solana.com";

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
    logging::log_activity("system", "Database connection", Some("Successfully connected"));

    match &cli.command {
        Command::Orca { pools } => {
            logging::log_activity(
                "system",
                "Indexer initialization",
                Some("Starting Orca Whirlpool indexer")
            );

            // Create indexer with resolved pool addresses in one operation
            let indexer = OrcaWhirlpoolIndexer::create_with_pools(
                db.pool().clone(),
                pools.as_ref()
            ).await?;

            // Start the indexer (pools are contained within the indexer)
            start_indexer(&indexer, &cli.rpc_url, &cli.ws_url).await.context(
                "Orca indexer failed"
            )?;
        }
        // For future implementation
        /*
        Command::Raydium { pools } => {
            logging::log_activity("system", "Raydium indexer", Some("not yet implemented"));
            // TODO: Implement Raydium indexer
        }
        */
    }

    Ok(())
}

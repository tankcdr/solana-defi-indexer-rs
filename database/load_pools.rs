use anyhow::{ Context, Result };
use clap::Parser;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use std::str::FromStr;
use std::env;
use std::fs;
use std::path::Path;
use std::collections::{ HashMap, HashSet };
use dotenv::dotenv;

// Import our models
mod models;
use models::{ TokenInfo, PoolRecord, DexProcessor };
use models::orca::OrcaProcessor;

// Command line arguments
#[derive(Parser, Debug)]
#[command(version, about = "Load DEX pool data into the database")]
struct Args {
    /// DEX type (orca, raydium, or all)
    #[arg(long, default_value = "all")]
    dex: String,

    /// Database URL
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,

    /// Solana RPC URL
    #[arg(long, env = "SOLANA_RPC_URL")]
    solana_rpc_url: Option<String>,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv().ok();

    // Parse command line arguments
    let args = Args::parse();

    // Get database URL from arguments or environment
    let database_url = match args.database_url {
        Some(url) => url,
        None => env::var("DATABASE_URL").context("DATABASE_URL not set")?,
    };

    // Get Solana RPC URL from arguments or environment
    let solana_rpc_url = match args.solana_rpc_url {
        Some(url) => url,
        None =>
            env
                ::var("SOLANA_RPC_URL")
                .context(
                    "SOLANA_RPC_URL not set (use --solana-rpc-url or set SOLANA_RPC_URL env var)"
                )?,
    };

    if args.verbose {
        println!("Database URL: {}", database_url);
        println!("Solana RPC URL: {}", solana_rpc_url);
        println!("DEX: {}", args.dex);
    }

    // Determine which DEXes to process
    let dexes = if args.dex == "all" {
        vec!["orca", "raydium"]
    } else if args.dex == "orca" || args.dex == "raydium" {
        vec![args.dex.as_str()]
    } else {
        return Err(
            anyhow::anyhow!("Invalid DEX type: {}. Valid options are: orca, raydium, all", args.dex)
        );
    };

    // Connect to Solana RPC
    let rpc_client = RpcClient::new_with_commitment(solana_rpc_url, CommitmentConfig::confirmed());

    // Connect to the database
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url).await
        .context("Failed to connect to the database")?;

    // Preload token cache from database
    let mut token_cache: HashMap<Pubkey, TokenInfo> = load_token_cache_from_db(&db_pool).await?;

    // Keep track of which tokens are already in the database to avoid redundant writes
    let mut saved_tokens: HashSet<Pubkey> = token_cache.keys().cloned().collect();

    if args.verbose {
        println!("Preloaded {} tokens from database", token_cache.len());
    }

    // Get Metaplex program ID
    let metadata_program_id = Pubkey::from_str("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s")?;

    // Process each DEX
    for dex in dexes {
        println!("Processing {} pools...", dex);

        // Define the path to the subscribed pools file
        let pools_file_path = format!("database/schema/{}/subscribed_pools.txt", dex);
        let path = Path::new(&pools_file_path);

        if !path.exists() {
            println!("Warning: Pools file not found at {}", pools_file_path);
            continue;
        }

        // Read pool addresses from file
        let content = fs
            ::read_to_string(path)
            .context(format!("Failed to read {}", pools_file_path))?;

        // Get the appropriate processor for this DEX
        let processor: Box<dyn DexProcessor> = match dex {
            "orca" => Box::new(OrcaProcessor {}),
            "raydium" => {
                println!("Raydium processing not yet implemented, skipping...");
                continue;
                // TODO: When implemented, return Box::new(RaydiumProcessor {})
            }
            _ => unreachable!(),
        };

        // Process each non-empty, non-comment line as a pool address
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Parse pool address
            let pool_pubkey = match Pubkey::from_str(trimmed) {
                Ok(pubkey) => pubkey,
                Err(e) => {
                    println!("Warning: Invalid pool address format '{}': {}", trimmed, e);
                    continue;
                }
            };

            println!("Fetching data for {} pool: {}", dex, pool_pubkey);

            // Process the pool using the appropriate processor
            match
                processor.process_pool(
                    &rpc_client,
                    &pool_pubkey,
                    &metadata_program_id,
                    &mut token_cache,
                    args.verbose
                ).await
            {
                Ok(pool_record) => {
                    // Save the pool data to the database
                    save_pool_to_database(&db_pool, &pool_record, &mut saved_tokens).await.context(
                        format!("Failed to save {} pool data to database", dex)
                    )?;
                    println!("Successfully processed {} pool: {}", dex, pool_pubkey);
                }
                Err(e) => {
                    println!("Error processing {} pool {}: {}", dex, pool_pubkey, e);
                }
            }
        }
    }

    println!("Successfully loaded pools data!");
    Ok(())
}

// Load token metadata from database into cache
async fn load_token_cache_from_db(db_pool: &sqlx::PgPool) -> Result<HashMap<Pubkey, TokenInfo>> {
    let mut token_cache = HashMap::new();

    // Query all tokens from the database
    let rows = sqlx
        ::query("SELECT mint, token_name, symbol, decimals FROM apestrong.token_metadata")
        .fetch_all(db_pool).await
        .context("Failed to load tokens from database")?;

    // Convert database records to TokenInfo and add to cache
    for row in rows {
        let mint: String = row.get(0);
        match Pubkey::from_str(&mint) {
            Ok(pubkey) => {
                let token_info = TokenInfo {
                    mint: pubkey,
                    name: row.get::<Option<String>, _>(1).unwrap_or_default(),
                    symbol: row.get::<Option<String>, _>(2).unwrap_or_default(),
                    decimals: row.get::<i32, _>(3) as u8,
                };
                token_cache.insert(pubkey, token_info);
            }
            Err(e) => {
                println!("Warning: Invalid pubkey in database: {}: {}", mint, e);
                continue;
            }
        }
    }

    Ok(token_cache)
}

// Save token metadata and pool to database
async fn save_pool_to_database(
    db_pool: &sqlx::PgPool,
    pool_record: &PoolRecord,
    saved_tokens: &mut HashSet<Pubkey>
) -> Result<()> {
    // Start a transaction
    let mut transaction = db_pool.begin().await?;

    // First, save token metadata if it's not already in the database
    for token in [&pool_record.token_a, &pool_record.token_b].iter() {
        // Only insert tokens that we haven't already saved or loaded from the database
        if !saved_tokens.contains(&token.mint) {
            sqlx
                ::query(
                    "INSERT INTO apestrong.token_metadata (mint, token_name, symbol, decimals, last_updated)
                 VALUES ($1, $2, $3, $4, NOW())
                 ON CONFLICT (mint) DO UPDATE 
                 SET token_name = $2, symbol = $3, decimals = $4, last_updated = NOW()"
                )
                .bind(token.mint.to_string())
                .bind(&token.name)
                .bind(&token.symbol)
                .bind(token.decimals as i32)
                .execute(&mut *transaction).await
                .context("Failed to save token metadata")?;

            // Add to saved tokens set to avoid duplicate inserts in future
            saved_tokens.insert(token.mint);
        }
    }

    // Then, save the pool record
    sqlx
        ::query(
            "INSERT INTO apestrong.subscribed_pools 
         (pool_mint, pool_name, dex, token_a_mint, token_b_mint, last_updated)
         VALUES ($1, $2, $3::apestrong.dex_type, $4, $5, NOW())
         ON CONFLICT (pool_mint) DO UPDATE
         SET pool_name = $2, dex = $3::apestrong.dex_type, token_a_mint = $4, token_b_mint = $5, last_updated = NOW()"
        )
        .bind(pool_record.pool_address.to_string())
        .bind(&pool_record.pool_name)
        .bind(&pool_record.dex)
        .bind(pool_record.token_a.mint.to_string())
        .bind(pool_record.token_b.mint.to_string())
        .execute(&mut *transaction).await
        .context("Failed to save pool record")?;

    // Commit the transaction
    transaction.commit().await?;

    println!("Saved/updated pool: {}", pool_record.pool_name);
    Ok(())
}

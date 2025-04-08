use anyhow::{ Context, Result };
use clap::Parser;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use sqlx::postgres::PgPoolOptions;
use std::str::FromStr;
use std::env;
use dotenv::dotenv;
use borsh::BorshDeserialize;
use mpl_token_metadata::accounts::Metadata as MplMetadata;

// Structure to hold token information
struct TokenInfo {
    mint: Pubkey,
    decimals: u8,
    symbol: String,
    name: String,
}

// Structure to represent a pool record for the database
struct WhirlpoolRecord {
    pool_address: Pubkey,
    pool_name: String,
    token_a: TokenInfo,
    token_b: TokenInfo,
    #[allow(dead_code)]
    tick_spacing: u16,
    #[allow(dead_code)]
    fee_rate: u16,
}

// Command line arguments
#[derive(Parser, Debug)]
#[command(version, about = "Load Orca Whirlpool pool data into the database")]
struct Args {
    /// Pool address (Pubkey in base58 encoding)
    #[arg(required = true)]
    pool_address: String,

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

// Whirlpool account data layout (simplified, only extracting the fields we need)
#[derive(BorshDeserialize, Debug)]
#[allow(dead_code)]
struct WhirlpoolData {
    pub whirlpools_config: Pubkey, // 32 bytes
    pub whirlpool_bump: [u8; 1], // 1 byte
    pub tick_spacing: u16, // 2 bytes
    pub tick_spacing_seed: [u8; 2], // 2 bytes
    pub fee_rate: u16, // 2 bytes
    pub protocol_fee_rate: u16, // 2 bytes
    pub liquidity: u128, // 16 bytes
    pub sqrt_price: u128, // 16 bytes
    pub tick_current_index: i32, // 4 bytes
    pub protocol_fee_owed_a: u64, // 8 bytes
    pub protocol_fee_owed_b: u64, // 8 bytes
    pub token_mint_a: Pubkey, // 32 bytes
    pub token_vault_a: Pubkey, // 32 bytes
    pub fee_growth_global_a: u128, // 16 bytes
    pub token_mint_b: Pubkey, // 32 bytes
    pub token_vault_b: Pubkey, // 32 bytes
    pub fee_growth_global_b: u128, // 16 bytes
    pub reward_last_updated_timestamp: u64, // 8 bytes
    pub reward_infos: [RewardInfo; 3], // 3 * 128 = 384 bytes
}

// Token Mint account data layout (simplified, only extracting decimals)
#[derive(BorshDeserialize, Debug)]
pub struct RewardInfo {
    pub mint: Pubkey, // 32 bytes
    pub vault: Pubkey, // 32 bytes
    pub authority: Pubkey, // 32 bytes
    pub emissions_per_second_x64: u128, // 16 bytes
    pub growth_global_x64: u128, // 16 bytes
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
    }

    let pool_pubkey = Pubkey::from_str(&args.pool_address).context(
        "Invalid pool address format - must be a valid base58-encoded Solana public key"
    )?;

    // Connect to Solana RPC
    let rpc_client = RpcClient::new_with_commitment(solana_rpc_url, CommitmentConfig::confirmed());

    println!("Fetching data for Orca Whirlpool: {}", pool_pubkey);

    // Fetch the pool account data
    let pool_account = rpc_client
        .get_account_with_commitment(&pool_pubkey, CommitmentConfig::confirmed()).await?
        .value.context("Pool account not found")?;

    // Debug information
    println!("Account data length: {} bytes", pool_account.data.len());

    // Try to deserialize the pool data
    let pool_data = WhirlpoolData::try_from_slice(&pool_account.data[8..]).context(
        "Failed to deserialize pool data"
    )?;

    println!("Found pool with the following data:");
    println!("  Token Mint A: {}", pool_data.token_mint_a);
    println!("  Token Mint B: {}", pool_data.token_mint_b);
    println!("  Tick Spacing: {}", pool_data.tick_spacing);
    println!("  Fee Rate: {}", pool_data.fee_rate);

    // Get Metaplex program ID
    let metadata_program_id = Pubkey::from_str("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s")?;

    // Fetch token information
    let token_a_info = fetch_token_info(
        &rpc_client,
        &pool_data.token_mint_a,
        &metadata_program_id
    ).await.context("Failed to fetch Token A information")?;

    let token_b_info = fetch_token_info(
        &rpc_client,
        &pool_data.token_mint_b,
        &metadata_program_id
    ).await.context("Failed to fetch Token B information")?;

    // Create pool record
    let pool_record = WhirlpoolRecord {
        pool_address: pool_pubkey,
        pool_name: format!("{} / {}", token_a_info.symbol, token_b_info.symbol),
        token_a: token_a_info,
        token_b: token_b_info,
        tick_spacing: pool_data.tick_spacing,
        fee_rate: pool_data.fee_rate,
    };

    // Display token information
    println!("  Token A Symbol: {}", pool_record.token_a.symbol);
    println!("  Token A Decimals: {}", pool_record.token_a.decimals);
    println!("  Token B Symbol: {}", pool_record.token_b.symbol);
    println!("  Token B Decimals: {}", pool_record.token_b.decimals);

    // Connect to the database
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url).await
        .context("Failed to connect to the database")?;

    // Save pool data to database
    save_pool_to_database(&db_pool, &pool_record).await.context(
        "Failed to save pool data to database"
    )?;

    println!("Successfully loaded pool data for {}", pool_pubkey);
    Ok(())
}

fn deserialize_metadata(data: &[u8]) -> Result<MplMetadata, anyhow::Error> {
    let data_owned = data.to_vec();
    let mut slice = data_owned.as_slice();
    let metadata: MplMetadata = MplMetadata::deserialize(&mut slice).map_err(|e|
        anyhow::anyhow!("Failed to deserialize metadata: {}", e)
    )?;

    Ok(metadata)
}

// Fetch token information (mint details, metadata, decimals)
async fn fetch_token_info(
    rpc_client: &RpcClient,
    token_mint: &Pubkey,
    metadata_program_id: &Pubkey
) -> Result<TokenInfo> {
    // Fetch token mint account
    let token_account = rpc_client
        .get_account_with_commitment(token_mint, CommitmentConfig::confirmed()).await?
        .value.context(format!("Token mint account not found for {}", token_mint))?;

    // Extract decimals
    let decimals = if token_account.data.len() >= 45 {
        token_account.data[44] // Offset for decimals in token mint data
    } else {
        println!("WARNING: Cannot extract decimals for token {}, using default value of 6", token_mint);
        6 // Default value for most tokens
    };

    // Try to fetch metadata
    let mut symbol = String::new();
    let mut name = String::new();
    let (metadata_pda, _bump) = Pubkey::find_program_address(
        &[b"metadata", &metadata_program_id.to_bytes(), &token_mint.to_bytes()],
        metadata_program_id
    );

    match
        rpc_client.get_account_with_commitment(&metadata_pda, CommitmentConfig::confirmed()).await
    {
        Ok(account_result) => {
            if let Some(metadata_account) = account_result.value {
                match deserialize_metadata(&metadata_account.data) {
                    Ok(metadata) => {
                        symbol = metadata.symbol.trim_end_matches('\0').to_string();
                        name = metadata.name.trim_end_matches('\0').to_string();
                    }
                    Err(e) => {
                        println!(
                            "Warning: Failed to deserialize metadata for {}: {}",
                            token_mint,
                            e
                        );
                    }
                }
            }
        }
        Err(e) => {
            println!("Warning: Failed to fetch metadata account for {}: {}", token_mint, e);
        }
    }

    Ok(TokenInfo {
        mint: *token_mint,
        decimals,
        symbol,
        name,
    })
}

// Save pool data to database
async fn save_pool_to_database(
    db_pool: &sqlx::PgPool,
    pool_record: &WhirlpoolRecord
) -> Result<()> {
    // Check if the pool already exists in the database
    let exists: (bool,) = sqlx
        ::query_as(
            "SELECT EXISTS(SELECT 1 FROM apestrong.orca_whirlpool_pools WHERE whirlpool = $1)"
        )
        .bind(pool_record.pool_address.to_string())
        .fetch_one(db_pool).await
        .context("Failed to check if pool exists")?;

    if exists.0 {
        println!("Pool already exists in the database. Updating...");

        // Update existing pool
        sqlx
            ::query(
                "UPDATE apestrong.orca_whirlpool_pools
                SET token_a_mint = $2, token_b_mint = $3, token_a_dec = $4, token_b_dec = $5, token_a_name = $6, token_b_name = $7, token_a_symbol = $8,  token_b_symbol = $9, pool_name = $10, last_updated = NOW()
                WHERE whirlpool = $1"
            )
            .bind(pool_record.pool_address.to_string())
            .bind(pool_record.token_a.mint.to_string())
            .bind(pool_record.token_b.mint.to_string())
            .bind(pool_record.token_a.decimals as i32)
            .bind(pool_record.token_b.decimals as i32)
            .bind(pool_record.token_a.name.to_string())
            .bind(pool_record.token_b.name.to_string())
            .bind(pool_record.token_a.symbol.to_string())
            .bind(pool_record.token_b.symbol.to_string())
            .bind(pool_record.pool_name.to_string())
            .execute(db_pool).await
            .context("Failed to update pool in database")?;
    } else {
        println!("Adding new pool to the database...");

        // Insert new pool
        sqlx
            ::query(
                "INSERT INTO apestrong.orca_whirlpool_pools
                (whirlpool, token_a_mint, token_b_mint, token_a_dec, token_b_dec, token_a_name, token_b_name, 
                token_a_symbol, token_b_symbol, pool_name, last_updated)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW())"
            )
            .bind(pool_record.pool_address.to_string())
            .bind(pool_record.token_a.mint.to_string())
            .bind(pool_record.token_b.mint.to_string())
            .bind(pool_record.token_a.decimals as i32)
            .bind(pool_record.token_b.decimals as i32)
            .bind(pool_record.token_a.name.to_string())
            .bind(pool_record.token_b.name.to_string())
            .bind(pool_record.token_a.symbol.to_string())
            .bind(pool_record.token_b.symbol.to_string())
            .bind(pool_record.pool_name.to_string())
            .execute(db_pool).await
            .context("Failed to insert pool into database")?;
    }

    Ok(())
}

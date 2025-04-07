use std::{ env, fs };
use std::path::Path;
use std::collections::HashMap;
use anyhow::{ Context, Result };
use clap::Parser;
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use serde::{ Deserialize, Serialize };

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexConfig {
    pub table_prefix: String,
    pub description: String,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub schema: String,
    pub dexes: HashMap<String, DexConfig>,
}

impl DatabaseConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path).context("Failed to open database config file")?;
        let config = serde_json
            ::from_str(&content)
            .context("Failed to parse database config file")?;
        Ok(config)
    }

    pub fn enabled_dexes(&self) -> Vec<(&String, &DexConfig)> {
        self.dexes
            .iter()
            .filter(|(_, config)| config.enabled)
            .collect()
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Set up the indexer database schema")]
struct Args {
    /// Path to schema file
    #[arg(long, default_value = "database/schema.sql")]
    schema_file: String,

    /// Path to delete schema file
    #[arg(long, default_value = "database/delete_schema.sql")]
    delete_schema_file: String,

    /// Path to config file
    #[arg(long, default_value = "database/config/db_config.json")]
    config_file: String,

    /// Database URL (overrides .env)
    #[arg(long)]
    database_url: Option<String>,

    /// Drop existing tables before creation
    #[arg(long)]
    drop_existing: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if present
    dotenv::dotenv().ok();

    // Parse command line arguments
    let args = Args::parse();

    // Get database URL
    let database_url = match &args.database_url {
        Some(url) => url.clone(),
        None => env::var("DATABASE_URL").context("DATABASE_URL environment variable not set")?,
    };

    if args.verbose {
        println!("Database URL: {}", database_url);
    }

    println!("Setting up database schema...");

    // Get database URL
    let database_url = match &args.database_url {
        Some(url) => url.clone(),
        None => env::var("DATABASE_URL").context("DATABASE_URL environment variable not set")?,
    };

    if args.verbose {
        println!("Database URL: {}", database_url);
    }

    // Load config if file exists
    let config = if Path::new(&args.config_file).exists() {
        match DatabaseConfig::load_from_file(&args.config_file) {
            Ok(config) => {
                println!("Loaded database configuration from {}", args.config_file);
                Some(config)
            }
            Err(e) => {
                println!("Warning: Failed to load config: {}", e);
                None
            }
        }
    } else {
        println!("Config file not found: {}", args.config_file);
        None
    };

    println!("Setting up database schema...");

    // Read the schema SQL file
    let schema_sql = fs
        ::read_to_string(&args.schema_file)
        .context(format!("Failed to read schema file: {}", args.schema_file))?;

    // Connect to the database
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url).await
        .context("Failed to connect to database")?;

    println!("Connected to database.");

    // Drop existing tables if requested
    if args.drop_existing {
        println!("Dropping existing tables...");

        // Read the delete schema SQL file
        let delete_sql = fs
            ::read_to_string(&args.delete_schema_file)
            .context(format!("Failed to read delete schema file: {}", args.delete_schema_file))?;

        for statement in delete_sql.split(';') {
            let stmt = statement.trim();
            if !stmt.is_empty() {
                if args.verbose {
                    println!("Executing: {}", stmt);
                }

                sqlx
                    ::query(stmt)
                    .execute(&pool).await
                    .with_context(|| format!("Failed to execute drop statement: {}", stmt))?;
            }
        }
    }

    // Execute schema SQL statements
    println!("Applying schema...");
    for statement in schema_sql.split(';') {
        let stmt = statement.trim();
        if !stmt.is_empty() {
            if args.verbose {
                println!("Executing: {}", stmt);
            }

            sqlx
                ::query(stmt)
                .execute(&pool).await
                .with_context(|| format!("Failed to execute SQL: {}", stmt))?;
        }
    }

    // Verify tables were created
    println!("Verifying schema setup...");

    // If we have a config, use it to check tables with proper prefixes
    if let Some(config) = config {
        for (dex_name, dex_config) in config.enabled_dexes() {
            let table_name = format!("{}{}_whirlpool_events", dex_config.table_prefix, dex_name);
            let check_sql = format!(
                "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = '{}' AND table_name = '{}')",
                config.schema,
                table_name
            );

            match sqlx::query(&check_sql).fetch_one(&pool).await {
                Ok(row) => {
                    let exists: bool = row.try_get(0)?;
                    if exists {
                        println!("Successfully created tables for DEX: {}", dex_name);
                    } else {
                        println!("Warning: tables for DEX '{}' may not have been created correctly.", dex_name);
                    }
                }
                Err(e) => {
                    println!(
                        "Warning: Unable to verify table creation for DEX '{}': {}",
                        dex_name,
                        e
                    );
                }
            }
        }
    } else {
        // Fallback to checking the main table
        let check_result = sqlx
            ::query(
                "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'orca_whirlpool_events')"
            )
            .fetch_one(&pool).await;

        match check_result {
            Ok(row) => {
                let exists: bool = row.try_get(0)?;
                if exists {
                    println!("Successfully created tables!");
                } else {
                    println!("Warning: tables may not have been created correctly.");
                }
            }
            Err(e) => {
                println!("Warning: Unable to verify table creation: {}", e);
            }
        }
    }

    println!("Database setup complete.");
    Ok(())
}

use anyhow::{ Context, Result };
use clap::{ Parser, ValueEnum };
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::fs;
use std::path::Path;

// Define valid operations
#[derive(Debug, Clone, ValueEnum)]
enum Operation {
    Create,
    Delete,
}

// Define valid DEX types
#[derive(Debug, Clone, ValueEnum)]
enum DexType {
    Common,
    Orca,
    Raydium,
    All,
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Database utility for schema management")]
struct Args {
    /// Operation to perform (create or delete)
    #[arg(value_enum)]
    operation: Operation,

    /// DEX type to target (common, orca, raydium, or all)
    #[arg(value_enum)]
    dex: DexType,

    /// Database URL (overrides .env)
    #[arg(long)]
    database_url: Option<String>,

    /// Skip confirmation prompts
    #[arg(long)]
    yes: bool,

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
        println!("Operation: {:?}", args.operation);
        println!("DEX: {:?}", args.dex);
    }

    // Determine which DEXes to process
    let mut dexes = match args.dex {
        DexType::All => vec!["common", "orca", "raydium"],
        DexType::Common => vec!["common"],
        DexType::Orca => vec!["common", "orca"], // Always include common for individual DEXes
        DexType::Raydium => vec!["common", "raydium"], // Always include common for individual DEXes
    };

    // For delete operations, reverse the order to handle common schema last
    // This ensures dependencies are deleted in the correct order
    if matches!(args.operation, Operation::Delete) {
        dexes.reverse();
        if args.verbose {
            println!("Delete operation: Processing schemas in reverse order");
        }
    }

    // Connect to the database
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url).await
        .context("Failed to connect to database")?;

    println!("Connected to database.");

    // Process each DEX
    for dex in dexes {
        let schema_path = format!("database/schema/{}/schema.sql", dex);
        let delete_path = format!("database/schema/{}/delete.sql", dex);

        match args.operation {
            Operation::Create => {
                println!("Creating schema for {}...", dex);

                if !Path::new(&schema_path).exists() {
                    println!("Warning: Schema file not found at {}", schema_path);
                    continue;
                }

                // Read and execute the schema SQL file
                let schema_sql = fs
                    ::read_to_string(&schema_path)
                    .context(format!("Failed to read schema file: {}", schema_path))?;

                execute_sql_statements(&pool, &schema_sql, args.verbose).await?;
                println!("Successfully created schema for {}", dex);
            }
            Operation::Delete => {
                if !args.yes {
                    println!("WARNING: You are about to delete the {} schema!", dex);
                    println!("This will delete all data for this DEX.");
                    println!("Please type 'yes' to continue or any other input to skip:");

                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input).context("Failed to read input")?;
                    if input.trim().to_lowercase() != "yes" {
                        println!("Skipping deletion of {} schema.", dex);
                        continue;
                    }
                }

                if !Path::new(&delete_path).exists() {
                    println!("Warning: Delete schema file not found at {}", delete_path);
                    continue;
                }

                println!("Deleting schema for {}...", dex);

                // Read and execute the delete SQL file
                let delete_sql = fs
                    ::read_to_string(&delete_path)
                    .context(format!("Failed to read delete schema file: {}", delete_path))?;

                execute_sql_statements(&pool, &delete_sql, args.verbose).await?;
                println!("Successfully deleted schema for {}", dex);
            }
        }
    }

    println!("Database operation completed successfully.");
    Ok(())
}

// Helper function to execute SQL statements, preserving dollar-quoted blocks
// DO NOT MODIFY
async fn execute_sql_statements(pool: &sqlx::PgPool, sql: &str, verbose: bool) -> Result<()> {
    let mut statements = Vec::new();
    let mut current_stmt = String::new();
    let mut in_dollar_quoted = false;
    let chars: Vec<char> = sql.chars().collect();

    for i in 0..chars.len() {
        current_stmt.push(chars[i]);

        // Check for dollar-quoted string start/end
        if i >= 1 && chars[i - 1] == '$' && chars[i] == '$' {
            in_dollar_quoted = !in_dollar_quoted;
        }

        // Split on semicolon only if not in a dollar-quoted block
        if chars[i] == ';' && !in_dollar_quoted {
            statements.push(current_stmt.trim().to_string());
            current_stmt.clear();
        }
    }

    // Add any remaining statement
    if !current_stmt.trim().is_empty() {
        statements.push(current_stmt.trim().to_string());
    }

    // Execute each statement
    for stmt in statements {
        if verbose {
            println!("Executing: {}", stmt);
        }
        sqlx
            ::query(&stmt)
            .execute(pool).await
            .with_context(|| format!("Failed to execute SQL: {}", stmt))?;
    }

    Ok(())
}

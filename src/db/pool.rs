use anyhow::{ Context, Result };
use sqlx::postgres::{ PgPool, PgPoolOptions };
use std::env;
use std::sync::Arc;
use std::time::Duration;

/// Database configuration for connecting to Supabase
#[derive(Debug, Clone)]
pub struct DbConfig {
    pub connection_string: String,
    pub max_connections: u32,
    pub connect_timeout: Duration,
}

impl DbConfig {
    /// Create a new DbConfig from environment variables
    pub fn from_env() -> Result<Self> {
        // Get database URL from environment variables
        let db_url = env::var("DATABASE_URL").context("DATABASE_URL environment variable not set")?;

        // Default to 5 connections, but allow configuration
        let max_connections = env
            ::var("DATABASE_MAX_CONNECTIONS")
            .unwrap_or_else(|_| "5".to_string())
            .parse::<u32>()
            .context("Invalid DATABASE_MAX_CONNECTIONS value")?;

        // Default timeout of 30 seconds
        let connect_timeout_secs = env
            ::var("DATABASE_CONNECT_TIMEOUT")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<u64>()
            .context("Invalid DATABASE_CONNECT_TIMEOUT value")?;

        Ok(Self {
            connection_string: db_url,
            max_connections,
            connect_timeout: Duration::from_secs(connect_timeout_secs),
        })
    }
}

/// Database connection pool for Supabase PostgreSQL
#[derive(Clone)]
pub struct Database {
    pool: Arc<PgPool>,
}

impl Database {
    /// Create a new database connection pool
    pub async fn connect(config: DbConfig) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .acquire_timeout(config.connect_timeout)
            .connect(&config.connection_string).await
            .context("Failed to connect to database")?;

        // Verify connection by running a simple query
        sqlx::query("SELECT 1").execute(&pool).await.context("Failed to execute test query")?;

        println!("Successfully connected to database");

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// Get a reference to the inner connection pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

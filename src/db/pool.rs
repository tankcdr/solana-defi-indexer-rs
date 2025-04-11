use anyhow::{ Context, Result };
use sqlx::postgres::{ PgPool, PgPoolOptions };
use std::env;
use std::sync::Arc;
use std::time::Duration;

use crate::utils::logging;

/// Database configuration for connecting to Supabase
#[derive(Debug, Clone)]
pub struct DbConfig {
    pub connection_string: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub max_lifetime: Duration,
    pub idle_timeout: Duration,
    pub connect_timeout: Duration,
}

impl DbConfig {
    /// Create a configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let connection_string = env
            ::var("DATABASE_URL")
            .context("DATABASE_URL environment variable not set")?;

        Ok(Self {
            connection_string,
            max_connections: 10,
            min_connections: 1,
            max_lifetime: Duration::from_secs(30 * 60), // 30 minutes
            idle_timeout: Duration::from_secs(10 * 60), // 10 minutes
            connect_timeout: Duration::from_secs(30), // 30 seconds
        })
    }
}

/// Database connection abstraction
pub struct Database {
    pool: Arc<PgPool>,
}

impl Database {
    /// Connect to the database
    pub async fn connect(config: DbConfig) -> Result<Self> {
        // Initialize connection pool
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .max_lifetime(config.max_lifetime)
            .idle_timeout(config.idle_timeout)
            .acquire_timeout(config.connect_timeout)
            .connect(&config.connection_string).await
            .context("Failed to connect to database")?;

        // Verify connection by running a simple query
        sqlx::query("SELECT 1").execute(&pool).await.context("Failed to execute test query")?;

        logging::log_activity("database", "Successfully connected to database", None);

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// Get a reference to the inner connection pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

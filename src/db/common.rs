use sqlx::PgPool;

// Protocol-agnostic repository trait that all specific repositories can implement
pub trait Repository {
    /// Get the connection pool
    fn pool(&self) -> &PgPool;
}

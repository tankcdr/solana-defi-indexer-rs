pub struct RaydiumIndexer {
    amm_repository: RaydiumAmmRepository,
    clmm_repository: RaydiumClmmRepository,
    pool_repository: RaydiumPoolRepository,
    // Maintain separate pool sets for each program
    amm_pool_pubkeys: HashSet<Pubkey>,
    clmm_pool_pubkeys: HashSet<Pubkey>,
}

const DEFAULT_RAYDIUM_CLMM_POOL: &str = &*"";

pub struct RaydiumIndexer {
    repository: RaydiumRepository,
    pool_pubkeys: HashSet<Pubkey>,
}

impl RaydiumIndexer {
    /// Create a new indexer with the given repository and pool set
    pub fn new(repository: RaydiumRepository, pool_pubkeys: HashSet<Pubkey>) -> Self {
        Self { repository, pool_pubkeys }
    }

    /// Create an indexer instance with a freshly initialized repository and default pool
    pub fn create(db_pool: sqlx::PgPool) -> Result<Self> {
        // Create a singleton pool set with the default pool
        let mut pool_pubkeys = HashSet::new();
        pool_pubkeys.insert(
            Pubkey::from_str(DEFAULT_RAYDIUM_CLMM_POOL).context(
                "Failed to parse default Raydium pool address"
            )?
        );

        let repository = RaydiumRepository::new(db_pool);
        Ok(Self::new(repository, pool_pubkeys))
    }

    ///TODO: evaluate the dual new on raydium repository
    pub async fn create_with_pools(
        db_pool: sqlx::PgPool,
        provided_pools: Option<&Vec<String>>
    ) -> Result<Self> {
        // Create the pool repository for address resolution
        let pool_repo = RaydiumRepository::new(db_pool.clone());

        // Resolve pool addresses
        let pool_pubkeys = pool_repo.get_pools_with_fallback(
            provided_pools,
            DEFAULT_RAYDIUM_CLMM_POOL
        ).await?;

        // Log information about the pool source
        let component = "raydium";

        if provided_pools.is_some() && !provided_pools.unwrap().is_empty() {
            logging::log_activity(component, "Pool source", Some("from command line arguments"));
        } else if pool_pubkeys.len() > 1 {
            logging::log_activity(component, "Pool source", Some("from database"));
        } else {
            logging::log_activity(
                component,
                "Pool source",
                Some("using default pool (no pools in CLI or database)")
            );
        }

        // Create the indexer with the resolved pools
        let repository = RaydiumRepository::new(db_pool);
        Ok(Self::new(repository, pool_pubkeys))
    }
}
